//! Matched-accuracy paired timing campaigns of re-audit R3 (HOM-06,
//! POLY-03).
//!
//! Every arm runs the whole attempt it claims to replace: an adaptive
//! integration over the same output grid and tolerances (HOM-06) or a joint
//! phi action on identical input vectors (POLY-03). Accuracy and work are
//! measured by `verify_arm`, in a process of its own and outside any timed
//! loop; wall time comes only from independent-session paired timing
//! receipts. Nothing here promotes anything by itself.

use std::{collections::BTreeMap, fs, path::Path, sync::Arc};

use anyhow::{Context, Result, bail};
use clap::ValueEnum;
use rodas5p_core::{
    CoreResult, DenseMatrix, DenseOperator, LinearMethod, LinearSolverConfig, PreconditionerKind,
    WorkCounters, dense_phi_combination,
    polynomial_action::{
        CoefficientCache, JointPhiInput, PolynomialBasis, SymmetricNonpositiveOperator,
        joint_phi_action_unbounded,
    },
    sha256_hex,
};
use rodas5p_fair_ab::{
    ArmIdentity, FairError, FairResult, PairedTimingEvidence, PairedTimingProtocol,
    PairedTimingReceipt, PairedWorkload, SessionFailure, SessionRecord, measure_paired_session,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, FusedPhiKrylovConfig, OdeProblem, OutputSchedule, Q2Admission,
    Q2CertificateSource, QuadraticStageProblem, TransactionalQ1Q2Config, fused_phi_action,
    integrate_sequential_matrix_free_adaptive_observed,
    integrate_transactional_q1_q2_adaptive_observed_with_admission, scalar_linear_problem,
};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Study {
    Hom06,
    Poly03,
}

impl Study {
    pub fn name(self) -> &'static str {
        match self {
            Self::Hom06 => "hom06",
            Self::Poly03 => "poly03",
        }
    }

    pub fn reference(self) -> &'static str {
        match self {
            Self::Hom06 => "sequential-jf-gmres",
            Self::Poly03 => "arnoldi-fused-phi",
        }
    }

    pub fn arms(self) -> &'static [&'static str] {
        match self {
            Self::Hom06 => &[
                "transactional-certified-p1",
                "transactional-certified-p2",
                "transactional-certified-p4",
                "transactional-certified-p8",
            ],
            Self::Poly03 => &[
                "chebyshev-cold",
                "chebyshev-warm",
                "laguerre-cold",
                "laguerre-warm",
            ],
        }
    }
}

// ---------------------------------------------------------------------------
// HOM-06: autonomous f(z) = A z + q z^2 problems
// ---------------------------------------------------------------------------

/// `f(z) = A z + q z^2` with its certificate data `J = A + 2 diag(q y)`.
struct QuadraticSystem {
    a: Vec<Vec<f64>>,
    q: Vec<f64>,
}

impl Q2CertificateSource for QuadraticSystem {
    fn stage_problem(&self, _t: f64, y: &[f64], h: f64) -> CoreResult<QuadraticStageProblem> {
        let jacobian = self
            .a
            .iter()
            .enumerate()
            .map(|(i, row)| {
                row.iter()
                    .enumerate()
                    .map(|(j, value)| {
                        if i == j {
                            value + 2.0 * self.q[i] * y[i]
                        } else {
                            *value
                        }
                    })
                    .collect()
            })
            .collect();
        Ok(QuadraticStageProblem {
            jacobian,
            y: y.to_vec(),
            h,
            q: self.q.clone(),
        })
    }
}

struct HomCase {
    id: &'static str,
    problem: OdeProblem,
    y0: Vec<f64>,
    source: Arc<QuadraticSystem>,
}

fn quadratic_ode(id: &str, system: &Arc<QuadraticSystem>) -> Result<OdeProblem> {
    let n = system.q.len();
    let rhs_system = system.clone();
    let jvp_system = system.clone();
    Ok(OdeProblem::new(
        id,
        n,
        Arc::new(move |_t, y: &[f64], out: &mut [f64]| {
            for (i, value) in out.iter_mut().enumerate() {
                let linear = rhs_system.a[i]
                    .iter()
                    .zip(y)
                    .map(|(a, x)| a * x)
                    .sum::<f64>();
                *value = linear + rhs_system.q[i] * y[i] * y[i];
            }
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(
            move |_t, y: &[f64], v: &[f64], out: &mut [f64]| {
                for (i, value) in out.iter_mut().enumerate() {
                    let linear = jvp_system.a[i]
                        .iter()
                        .zip(v)
                        .map(|(a, x)| a * x)
                        .sum::<f64>();
                    *value = linear + 2.0 * jvp_system.q[i] * y[i] * v[i];
                }
                Ok(())
            },
        )),
        None,
        true,
        None,
        None,
    )?)
}

fn hom06_cases() -> Result<Vec<HomCase>> {
    let mut cases = Vec::new();
    let mut push = |id: &'static str, a: Vec<Vec<f64>>, q: Vec<f64>, y0: Vec<f64>| -> Result<()> {
        let source = Arc::new(QuadraticSystem { a, q });
        cases.push(HomCase {
            id,
            problem: quadratic_ode(id, &source)?,
            y0,
            source,
        });
        Ok(())
    };
    push("scalar-linear", vec![vec![-20.0]], vec![0.0], vec![1.0])?;
    push("scalar-quadratic", vec![vec![-20.0]], vec![-2.0], vec![1.0])?;
    let diagonal = (0..8)
        .map(|i| {
            (0..8)
                .map(|j| {
                    if i == j {
                        -(10.0_f64).powf(i as f64 * 3.0 / 7.0)
                    } else {
                        0.0
                    }
                })
                .collect()
        })
        .collect();
    push(
        "diagonal-quadratic-8",
        diagonal,
        vec![-0.5; 8],
        vec![1.0; 8],
    )?;
    let tridiagonal = (0..6)
        .map(|i: usize| {
            (0..6)
                .map(|j: usize| match i.abs_diff(j) {
                    0 => -100.0,
                    1 => 50.0,
                    _ => 0.0,
                })
                .collect()
        })
        .collect();
    push(
        "coupled-linear-6",
        tridiagonal,
        vec![0.0; 6],
        (0..6).map(|i| 1.0 + 0.1 * i as f64).collect(),
    )?;
    push(
        "nonsymmetric-linear-2",
        vec![vec![-1000.0, 999.0], vec![0.0, -1.0]],
        vec![0.0; 2],
        vec![1.0, 1.0],
    )?;
    // The scalar linear case must equal the library's own problem.
    let (_, library_y0) = scalar_linear_problem(-20.0, 1.0);
    debug_assert_eq!(library_y0, cases[0].y0);
    Ok(cases)
}

const HOM06_SPAN: (f64, f64) = (0.0, 1.0);

fn hom06_adaptive() -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 1.0e-8,
        rtol: 1.0e-6,
        initial_step: 1.0e-3,
        ..AdaptiveStepConfig::default()
    }
}

fn hom06_output() -> Result<OutputSchedule> {
    Ok(OutputSchedule::uniform(HOM06_SPAN.0, HOM06_SPAN.1, 0.1)?)
}

fn hom06_linear() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        rtol: 1.0e-10,
        atol: 1.0e-12,
        restart: 32,
        maxiter: 256,
        preconditioner: PreconditionerKind::None,
        ..LinearSolverConfig::default()
    }
}

fn hom06_threads(arm: &str) -> Result<usize> {
    arm.strip_prefix("transactional-certified-p")
        .and_then(|p| p.parse().ok())
        .with_context(|| format!("unknown HOM-06 arm {arm}"))
}

/// One full attempt of an arm on a case: states on the output grid, work,
/// and the transactional diagnostics.
fn hom06_run(
    case: &HomCase,
    arm: &str,
) -> Result<(Vec<Vec<f64>>, WorkCounters, serde_json::Value)> {
    let output = hom06_output()?;
    if arm == Study::Hom06.reference() {
        let result = integrate_sequential_matrix_free_adaptive_observed(
            &case.problem,
            HOM06_SPAN,
            &case.y0,
            &hom06_linear(),
            &hom06_adaptive(),
            &output,
        )?;
        if !result.observed.success {
            bail!(
                "{}: sequential run failed: {}",
                case.id,
                result.observed.message
            );
        }
        let details = serde_json::json!({
            "internal_steps": result.observed.internal_steps,
            "attempts": result.diagnostics.attempts,
        });
        return Ok((result.observed.y, result.observed.counters, details));
    }
    let config = TransactionalQ1Q2Config {
        threads: hom06_threads(arm)?,
        ..TransactionalQ1Q2Config::default()
    };
    let result = integrate_transactional_q1_q2_adaptive_observed_with_admission(
        &case.problem,
        HOM06_SPAN,
        &case.y0,
        &config,
        &hom06_adaptive(),
        &output,
        Q2Admission::NativeTargetCertificate(case.source.as_ref()),
    )?;
    if !result.observed.success {
        bail!(
            "{}: transactional run failed: {}",
            case.id,
            result.observed.message
        );
    }
    let d = &result.transactional;
    let attempts = d.q1_path_attempts.max(1) as f64;
    let details = serde_json::json!({
        "internal_steps": result.observed.internal_steps,
        "attempts": d.q1_path_attempts,
        "p1_accepted_q1_fraction": d.accepted_q1_fast_steps as f64 / attempts,
        "q2_escalation_fraction": d.q2_path_attempts as f64 / attempts,
        "pf_fallback_fraction": d.selected_sequential_fallback_attempts as f64 / attempts,
        "certificate_attempts": d.certificate_attempts,
        "certificate_admissions": d.certificate_admissions,
        "certificate_fraction": d.certificate_admissions as f64 / d.certificate_attempts.max(1) as f64,
        "certificate_operations": d.certificate_operations,
        "accepted_q2_escalated_steps": d.accepted_q2_escalated_steps,
        "admission_mismatches": d.accepted_q2_escalated_steps.saturating_sub(d.certificate_admissions),
        "total_w_solve_batches": d.total_w_solve_batches,
        "total_critical_path_depth": d.total_critical_path_depth,
    });
    Ok((result.observed.y, result.observed.counters, details))
}

/// Reference states: the sequential stepper at tolerances 1e-13 / 1e-11.
fn hom06_reference_states(case: &HomCase) -> Result<Vec<Vec<f64>>> {
    let tight = AdaptiveStepConfig {
        atol: 1.0e-13,
        rtol: 1.0e-11,
        initial_step: 1.0e-5,
        ..AdaptiveStepConfig::default()
    };
    let result = integrate_sequential_matrix_free_adaptive_observed(
        &case.problem,
        HOM06_SPAN,
        &case.y0,
        &LinearSolverConfig {
            rtol: 1.0e-13,
            atol: 1.0e-15,
            ..hom06_linear()
        },
        &tight,
        &hom06_output()?,
    )?;
    if !result.observed.success {
        bail!("{}: reference run failed", case.id);
    }
    Ok(result.observed.y)
}

/// Largest `|y - y_ref| / (atol + rtol |y_ref|)` over the grid.
fn tolerance_ratio(states: &[Vec<f64>], reference: &[Vec<f64>]) -> f64 {
    let adaptive = hom06_adaptive();
    states
        .iter()
        .zip(reference)
        .flat_map(|(row, reference_row)| row.iter().zip(reference_row))
        .map(|(y, r)| (y - r).abs() / (adaptive.atol + adaptive.rtol * r.abs()))
        .fold(0.0, f64::max)
}

// ---------------------------------------------------------------------------
// POLY-03: symmetric nonpositive operators, h = 1 so w_k = b_k exactly
// ---------------------------------------------------------------------------

struct PolyCase {
    id: &'static str,
    matrix: DenseMatrix,
    operator: SymmetricNonpositiveOperator,
    vectors: [Vec<f64>; 5],
}

const POLY03_BUDGET: f64 = 1.0e-12;

fn poly_vectors(n: usize) -> [Vec<f64>; 5] {
    let weights = [1.0, -1.0, 0.5, -0.25, 0.125];
    std::array::from_fn(|k| {
        let raw = (0..n)
            .map(|i| (i as f64 * (0.23 + 0.11 * k as f64)).cos() + 0.2 * (k + 1) as f64)
            .collect::<Vec<_>>();
        let norm = raw.iter().map(|x| x * x).sum::<f64>().sqrt();
        raw.iter().map(|x| x / norm * weights[k]).collect()
    })
}

fn poly03_cases() -> Result<Vec<PolyCase>> {
    let mut cases = Vec::new();
    let mut push =
        |id: &'static str, rows: Vec<Vec<f64>>, declared: Option<(f64, f64)>| -> Result<()> {
            let refs = rows.iter().map(Vec::as_slice).collect::<Vec<_>>();
            let matrix = DenseMatrix::from_rows(&refs)?;
            let operator = match declared {
                None => SymmetricNonpositiveOperator::gershgorin(matrix.clone())?,
                Some((lambda, rho)) => {
                    SymmetricNonpositiveOperator::new(matrix.clone(), lambda, rho, "construction")?
                }
            };
            let n = rows.len();
            cases.push(PolyCase {
                id,
                matrix,
                operator,
                vectors: poly_vectors(n),
            });
            Ok(())
        };
    let diagonal = |n: usize, rho: f64| -> Vec<Vec<f64>> {
        (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| {
                        if i == j {
                            -0.1 * (rho / 0.1).powf(i as f64 / (n - 1) as f64)
                        } else {
                            0.0
                        }
                    })
                    .collect()
            })
            .collect()
    };
    let laplacian = |n: usize, rho: f64| -> Vec<Vec<f64>> {
        let scale = rho / 4.0;
        (0..n)
            .map(|i: usize| {
                (0..n)
                    .map(|j: usize| match i.abs_diff(j) {
                        0 => -2.0 * scale,
                        1 => scale,
                        _ => 0.0,
                    })
                    .collect()
            })
            .collect()
    };
    push("diagonal24-rho10", diagonal(24, 10.0), None)?;
    push("diagonal24-rho100", diagonal(24, 100.0), None)?;
    push("laplacian64-rho100", laplacian(64, 100.0), None)?;
    push("laplacian128-rho400", laplacian(128, 400.0), None)?;
    // -(M M^T), symmetric by construction, spectrum in [-||M||_F^2, 0].
    let n = 32;
    let m = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| (((i * 7 + j * 13) % 17) as f64 - 8.0) / 8.0)
                .collect::<Vec<f64>>()
        })
        .collect::<Vec<_>>();
    let mut gram = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let value = -(0..n).map(|k| m[i][k] * m[j][k]).sum::<f64>();
            gram[i][j] = value;
            gram[j][i] = value;
        }
    }
    let frobenius = m.iter().flatten().map(|x| x * x).sum::<f64>();
    push("gram32", gram, Some((0.0, frobenius * (1.0 + 1.0e-12))))?;
    Ok(cases)
}

/// One joint action of an arm, with `cache` persisting across calls for the
/// warm arms.
fn poly03_run(
    case: &PolyCase,
    arm: &str,
    cache: &mut CoefficientCache,
    work: &mut WorkCounters,
) -> Result<Vec<f64>> {
    let basis = match arm {
        "chebyshev-cold" | "chebyshev-warm" => PolynomialBasis::Chebyshev,
        "laguerre-cold" | "laguerre-warm" => PolynomialBasis::Laguerre,
        reference if reference == Study::Poly03.reference() => {
            let operator = Arc::new(DenseOperator::new(case.matrix.clone())?);
            let report = fused_phi_action(
                operator,
                1.0,
                &case.vectors,
                FusedPhiKrylovConfig {
                    maximum_dimension: case.matrix.nrows().min(64),
                    relative_tolerance: 1.0e-12,
                    absolute_tolerance: 1.0e-15,
                    maximum_substeps: 64,
                    ..FusedPhiKrylovConfig::default()
                },
                work,
            )?;
            if !report.converged {
                bail!("{}: Arnoldi fused phi did not converge", case.id);
            }
            return Ok(report.value);
        }
        other => bail!("unknown POLY-03 arm {other}"),
    };
    let warm = arm.ends_with("-warm");
    let report = joint_phi_action_unbounded(
        &case.operator,
        1.0,
        JointPhiInput::Distinct(&case.vectors),
        basis,
        POLY03_BUDGET,
        if warm { Some(cache) } else { None },
        work,
    )?;
    Ok(report.fused)
}

// ---------------------------------------------------------------------------
// Verification (untimed), one process per arm
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct CaseVerification {
    case_id: String,
    success: bool,
    message: Option<String>,
    /// HOM-06: largest tolerance ratio against the tight reference;
    /// POLY-03: relative 2-norm error against the dense augmented exponential.
    accuracy: f64,
    accuracy_gate: f64,
    accuracy_pass: bool,
    work: WorkCounters,
    details: serde_json::Value,
}

#[derive(Serialize)]
struct ArmVerification {
    study: String,
    arm: String,
    cases: Vec<CaseVerification>,
    /// VmHWM of this process after all cases.
    peak_rss_kib: Option<u64>,
}

fn peak_rss_kib() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    status
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))
        .and_then(|rest| rest.split_whitespace().next()?.parse().ok())
}

/// HOM-06 accuracy gate: tolerance ratio at most 10 on every grid value.
pub const HOM06_ACCURACY_GATE: f64 = 10.0;
/// POLY-03 accuracy gate: relative error at most 1e-9.
pub const POLY03_ACCURACY_GATE: f64 = 1.0e-9;

pub fn verify_arm(study: Study, arm: &str, output: &Path) -> Result<()> {
    let mut cases = Vec::new();
    match study {
        Study::Hom06 => {
            for case in hom06_cases()? {
                let reference = hom06_reference_states(&case)?;
                let verification = match hom06_run(&case, arm) {
                    Ok((states, work, details)) => {
                        let ratio = tolerance_ratio(&states, &reference);
                        CaseVerification {
                            case_id: case.id.into(),
                            success: true,
                            message: None,
                            accuracy: ratio,
                            accuracy_gate: HOM06_ACCURACY_GATE,
                            accuracy_pass: ratio <= HOM06_ACCURACY_GATE,
                            work,
                            details,
                        }
                    }
                    Err(error) => CaseVerification {
                        case_id: case.id.into(),
                        success: false,
                        message: Some(error.to_string()),
                        accuracy: f64::INFINITY,
                        accuracy_gate: HOM06_ACCURACY_GATE,
                        accuracy_pass: false,
                        work: WorkCounters::default(),
                        details: serde_json::Value::Null,
                    },
                };
                cases.push(verification);
            }
        }
        Study::Poly03 => {
            for case in poly03_cases()? {
                let reference = dense_phi_combination(&case.matrix, 1.0, &case.vectors)?;
                let norm = reference.iter().map(|x| x * x).sum::<f64>().sqrt().max(1.0);
                let mut cache = CoefficientCache::default();
                let mut work = WorkCounters::default();
                // Two calls, so the warm arms show their reuse.
                let result = poly03_run(&case, arm, &mut cache, &mut work)
                    .and_then(|_| poly03_run(&case, arm, &mut cache, &mut work));
                let verification = match result {
                    Ok(value) => {
                        let error = value
                            .iter()
                            .zip(&reference)
                            .map(|(a, b)| (a - b) * (a - b))
                            .sum::<f64>()
                            .sqrt()
                            / norm;
                        CaseVerification {
                            case_id: case.id.into(),
                            success: true,
                            message: None,
                            accuracy: error,
                            accuracy_gate: POLY03_ACCURACY_GATE,
                            accuracy_pass: error <= POLY03_ACCURACY_GATE,
                            work,
                            details: serde_json::json!({
                                "dimension": case.matrix.nrows(),
                                "enclosure": case.operator.enclosure(),
                                "calls": 2,
                            }),
                        }
                    }
                    Err(error) => CaseVerification {
                        case_id: case.id.into(),
                        success: false,
                        message: Some(error.to_string()),
                        accuracy: f64::INFINITY,
                        accuracy_gate: POLY03_ACCURACY_GATE,
                        accuracy_pass: false,
                        work,
                        details: serde_json::Value::Null,
                    },
                };
                cases.push(verification);
            }
        }
    }
    let report = ArmVerification {
        study: study.name().into(),
        arm: arm.into(),
        cases,
        peak_rss_kib: peak_rss_kib(),
    };
    crate::write_json(output, &report)
}

// ---------------------------------------------------------------------------
// Paired timing sessions and campaigns
// ---------------------------------------------------------------------------

fn fair(error: impl std::fmt::Display) -> FairError {
    FairError::Invalid(error.to_string())
}

pub fn run_session(
    study: Study,
    arm: &str,
    campaign_id: &str,
    session: u32,
    seed: u64,
    batches: &BTreeMap<String, usize>,
) -> Result<SessionRecord> {
    if !study.arms().contains(&arm) {
        bail!("{arm} is not a {} candidate arm", study.name());
    }
    let protocol = PairedTimingProtocol::authoritative(seed);
    let record = match study {
        Study::Hom06 => {
            let cases = hom06_cases()?;
            let mut workloads = cases
                .iter()
                .map(|case| {
                    let run = move |arm: &'static str| {
                        move || -> FairResult<()> { hom06_run(case, arm).map(|_| ()).map_err(fair) }
                    };
                    let arm: &'static str = Study::Hom06
                        .arms()
                        .iter()
                        .find(|candidate| **candidate == arm)
                        .copied()
                        .expect("checked arm");
                    PairedWorkload {
                        case_id: case.id.into(),
                        candidate: Box::new(run(arm)),
                        reference: Box::new(run(Study::Hom06.reference())),
                    }
                })
                .collect::<Vec<_>>();
            measure_paired_session(campaign_id, session, &protocol, &mut workloads, batches)?
        }
        Study::Poly03 => {
            let cases = poly03_cases()?;
            let arm: &'static str = Study::Poly03
                .arms()
                .iter()
                .find(|candidate| **candidate == arm)
                .copied()
                .expect("checked arm");
            let mut workloads = cases
                .iter()
                .map(|case| {
                    let mut candidate_cache = CoefficientCache::default();
                    let mut reference_cache = CoefficientCache::default();
                    PairedWorkload {
                        case_id: case.id.into(),
                        candidate: Box::new(move || -> FairResult<()> {
                            let mut work = WorkCounters::default();
                            poly03_run(case, arm, &mut candidate_cache, &mut work)
                                .map(|_| ())
                                .map_err(fair)
                        }),
                        reference: Box::new(move || -> FairResult<()> {
                            let mut work = WorkCounters::default();
                            poly03_run(
                                case,
                                Study::Poly03.reference(),
                                &mut reference_cache,
                                &mut work,
                            )
                            .map(|_| ())
                            .map_err(fair)
                        }),
                    }
                })
                .collect::<Vec<_>>();
            measure_paired_session(campaign_id, session, &protocol, &mut workloads, batches)?
        }
    };
    Ok(record)
}

/// Each session in a fresh process of this executable; the first session
/// fixes the batch per case. Failed sessions stay in the receipt.
pub fn run_campaign(
    study: Study,
    arm: &str,
    sessions: u32,
    seed: u64,
    output: &Path,
) -> Result<PairedTimingEvidence> {
    let executable = std::env::current_exe()?;
    let executable_sha256 = sha256_hex(&fs::read(&executable)?);
    let campaign_id = format!(
        "r3-{}-{arm}-{}-{}",
        study.name(),
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    );
    let directory = output.with_extension(format!("{arm}.sessions"));
    fs::create_dir_all(&directory)?;
    let batches_path = directory.join("batches.json");
    let mut records = Vec::new();
    let mut failed_sessions = Vec::new();
    for session in 0..sessions {
        let record_path = directory.join(format!("session-{session}.json"));
        let mut command = std::process::Command::new(&executable);
        command
            .arg("r3-campaign-session")
            .args(["--study", study.name()])
            .args(["--arm", arm])
            .args(["--campaign-id", &campaign_id])
            .args(["--session", &session.to_string()])
            .args(["--seed", &seed.to_string()])
            .arg("--output")
            .arg(&record_path);
        if session > 0 && batches_path.exists() {
            command.arg("--batches").arg(&batches_path);
        }
        let status = command.status()?;
        let record = if status.success() {
            fs::read(&record_path)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<SessionRecord>(&bytes).ok())
        } else {
            None
        };
        let Some(record) = record else {
            failed_sessions.push(SessionFailure {
                session,
                case_id: "*".into(),
                message: format!("session process exited with {status} and no readable record"),
            });
            continue;
        };
        if session == 0 {
            let batches = record
                .cases
                .iter()
                .map(|case| (case.case_id.clone(), case.batch_iterations))
                .collect::<BTreeMap<_, _>>();
            crate::write_json(&batches_path, &batches)?;
        }
        records.push(record);
    }
    let workload_id = format!("r3-{}-seed-{seed}", study.name());
    let receipt = PairedTimingReceipt::from_sessions_with_failures(
        &campaign_id,
        ArmIdentity {
            arm_id: arm.to_owned(),
            executable_sha256: executable_sha256.clone(),
            workload_id: workload_id.clone(),
        },
        ArmIdentity {
            arm_id: study.reference().to_owned(),
            executable_sha256,
            workload_id,
        },
        PairedTimingProtocol::authoritative(seed),
        records,
        failed_sessions,
    )?;
    Ok(PairedTimingEvidence::from_receipt(receipt)?)
}
