//! Deterministic research studies of re-audit R4 of 2026-10-01: the Laguerre
//! majorant and scale selection (R4-POLY-DEV-03, R4-POLY-DEV-05), the
//! frozen-operator polynomial regimes and their cost separation
//! (R4-POLY-DEV-04, R4-STAT-DEV-05) and the homotopy certificate cost
//! (R4-HOM-DEV-07). Each is preregistered in its `research/r4_*` node.
//!
//! Every verdict is on deterministic work counters, enclosures and accuracy.
//! Wall seconds are recorded as diagnostics only: the statistical authority
//! of timing is on hold (`rodas5p_fair_ab::timing_authority_registry`), so no
//! study here makes a speed claim.

use std::time::Instant;

use anyhow::Result;
use rodas5p_core::{
    DenseMatrix, WorkCounters,
    directed::Interval,
    polynomial_action::{
        CoefficientCache, JointPhiInput, JointPhiReport, PolynomialBasis,
        SymmetricNonpositiveOperator, TotalErrorStatus, joint_phi_action,
        joint_phi_action_laguerre_scales, joint_phi_action_unbounded, laguerre_scale_for_degree,
        scalar_phi_enclosure,
    },
    rodas5p_coefficients,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, InverseWitness, OutputSchedule, ParallelExecution, Q2Admission,
    Q2CertificateSource, QuadraticModel, StageTarget, TransactionalQ1Q2Config,
    blocked_doubling_certificate_with_execution, certify_stage_target,
    doubling_certificate_with_execution,
    integrate_transactional_q1_q2_adaptive_observed_with_admission,
};
use serde_json::{Value, json};

pub const TIMING_STATUS: &str =
    "NOT_EVALUATED: STATISTICAL_AUTHORITY_HOLD (wall seconds are diagnostics, no speed claim)";

type Vectors = [Vec<f64>; 5];

fn geomspace(lo: f64, hi: f64, n: usize) -> Vec<f64> {
    if n == 1 {
        return vec![lo];
    }
    (0..n)
        .map(|i| lo * (hi / lo).powf(i as f64 / (n - 1) as f64))
        .collect()
}

/// A diagonal operator `diag(-lambda_i)` with a verified enclosure.
fn diagonal(eigenvalues: &[f64]) -> Result<SymmetricNonpositiveOperator> {
    let n = eigenvalues.len();
    let mut rows = vec![vec![0.0; n]; n];
    for (i, value) in eigenvalues.iter().enumerate() {
        rows[i][i] = -value;
    }
    let matrix = DenseMatrix::from_rows(&rows.iter().map(Vec::as_slice).collect::<Vec<_>>())?;
    Ok(SymmetricNonpositiveOperator::gershgorin(matrix)?)
}

/// Deterministic input vectors of the R3 probe shape: cosine patterns,
/// normalized and scaled by `[1, -1, 0.5, -0.25, 0.125]`.
fn vectors(n: usize, variant: usize) -> Vectors {
    let scales = [1.0, -1.0, 0.5, -0.25, 0.125];
    std::array::from_fn(|k| {
        let raw = (0..n)
            .map(|i| {
                (i as f64 * (0.23 + 0.11 * k as f64 + 0.017 * variant as f64)).cos()
                    + 0.2 * (k + 1) as f64
            })
            .collect::<Vec<_>>();
        let norm = raw.iter().map(|x| x * x).sum::<f64>().sqrt();
        raw.iter().map(|x| x / norm * scales[k]).collect()
    })
}

fn weight(w: &Vectors) -> f64 {
    let factorial = [1.0, 1.0, 2.0, 6.0, 24.0];
    w.iter()
        .zip(factorial)
        .map(|(v, f)| v.iter().map(|x| x * x).sum::<f64>().sqrt() / f)
        .sum()
}

/// `|fused - F|_2` against the scalar phi enclosures of a diagonal operator
/// (midpoints; enclosure widths are at roundoff level).
fn diagonal_error(eigenvalues: &[f64], h: f64, w: &Vectors, fused: &[f64]) -> Result<f64> {
    let mut square = 0.0;
    for (i, lambda) in eigenvalues.iter().enumerate() {
        let z = -h * lambda;
        let phi: [f64; 5] = if z >= -600.0 {
            let enclosure = scalar_phi_enclosure(Interval::new(z, z)?)?;
            std::array::from_fn(|k| 0.5 * enclosure[k].lo + 0.5 * enclosure[k].hi)
        } else {
            // Beyond the enclosure's range: e^z < 1e-260 is negligible and
            // phi_k(z) = (phi_(k-1)(z) - 1/(k-1)!) / z has no cancellation
            // for z < -600.
            let mut values = [0.0; 5];
            let mut factorial = 1.0;
            for k in 1..5 {
                values[k] = (values[k - 1] - 1.0 / factorial) / z;
                factorial *= k as f64;
            }
            values
        };
        let reference = (0..5).map(|k| phi[k] * w[k][i]).sum::<f64>();
        square += (fused[i] - reference).powi(2);
    }
    Ok(square.sqrt())
}

/// The smallest degree whose continuous scale `L*(m)` meets the budget by
/// the f64 tail estimate `e^(L/2) q^(m+1) W` (selection only; the action
/// re-derives its tail rigorously).
fn continuous_choice(h: f64, rho: f64, w: f64, budget: f64) -> Option<(usize, f64)> {
    (0..=4096).find_map(|m| {
        let scale = laguerre_scale_for_degree(h, rho, m)?;
        let beta = rho / scale;
        let q = h * beta / (1.0 + h * beta);
        let tail = (scale / 2.0).exp() * q.powi((m + 1) as i32) * w;
        (tail <= budget).then_some((m, scale))
    })
}

fn report_row(report: &JointPhiReport, error: f64) -> Value {
    let (status, bound) = match &report.total_error {
        TotalErrorStatus::Certified { bound } => ("certified", *bound),
        TotalErrorStatus::EstimateOnly {
            bounded_components, ..
        } => ("estimate-only", *bounded_components),
    };
    json!({
        "degree": report.degree,
        "laguerre_scale": report.laguerre_scale,
        "status": status,
        "bound": bound,
        "laguerre_majorant_total": report.laguerre_majorant_total,
        "truncation": report.truncation_bound_exact_arithmetic,
        "observed_error": error,
    })
}

/// R4-POLY-DEV-03 (majorant tightness and rejection) and R4-POLY-DEV-05
/// (continuous scale against the grid).
pub fn laguerre_study() -> Result<Value> {
    let families = [
        ("diag24-wide", geomspace(0.1, 100.0, 24)),
        ("diag24-narrow", geomspace(1.0, 4.0, 24)),
        ("diag8-stiff", geomspace(0.01, 1000.0, 8)),
    ];
    let mut rows = Vec::new();
    for (name, eigenvalues) in &families {
        let op = diagonal(eigenvalues)?;
        let rho = eigenvalues.iter().cloned().fold(0.0_f64, f64::max);
        let w = vectors(eigenvalues.len(), 0);
        let total_weight = weight(&w);
        for h in [1.0e-3, 1.0e-2, 0.1, 1.0] {
            for budget in [1.0e-6, 1.0e-10] {
                let mut work = WorkCounters::default();
                let grid = joint_phi_action(
                    &op,
                    h,
                    JointPhiInput::Distinct(&w),
                    PolynomialBasis::Laguerre,
                    budget,
                    None,
                    &mut work,
                );
                let grid_row = match &grid {
                    Ok(report) => {
                        let error = diagonal_error(eigenvalues, h, &w, &report.fused)?;
                        let mut row = report_row(report, error);
                        row["block_products"] = json!(work.poly_block_products);
                        row
                    }
                    Err(error) => json!({ "rejected": error.to_string() }),
                };
                let mut chebyshev_work = WorkCounters::default();
                let chebyshev = joint_phi_action(
                    &op,
                    h,
                    JointPhiInput::Distinct(&w),
                    PolynomialBasis::Chebyshev,
                    budget,
                    None,
                    &mut chebyshev_work,
                );
                let chebyshev_row = match &chebyshev {
                    Ok(report) => {
                        report_row(report, diagonal_error(eigenvalues, h, &w, &report.fused)?)
                    }
                    Err(error) => json!({ "rejected": error.to_string() }),
                };
                let continuous_row = match continuous_choice(h, rho, total_weight, budget) {
                    None => {
                        json!({ "rejected": "no degree <= 4096 with a positive continuous scale meets the budget" })
                    }
                    Some((target_degree, scale)) => {
                        let mut work = WorkCounters::default();
                        match joint_phi_action_laguerre_scales(
                            &op,
                            h,
                            JointPhiInput::Distinct(&w),
                            &[scale],
                            budget,
                            &mut work,
                        ) {
                            Ok(report) => {
                                let error = diagonal_error(eigenvalues, h, &w, &report.fused)?;
                                let mut row = report_row(&report, error);
                                row["selected_degree_estimate"] = json!(target_degree);
                                row["block_products"] = json!(work.poly_block_products);
                                row
                            }
                            Err(error) => json!({ "rejected": error.to_string(), "scale": scale }),
                        }
                    }
                };
                rows.push(json!({
                    "family": name, "h": h, "budget": budget, "rho": rho, "weight": total_weight,
                    "grid": grid_row, "continuous": continuous_row, "chebyshev": chebyshev_row,
                }));
            }
        }
    }
    // POLY-DEV-03: soundness and rejection of the majorant.
    let mut enclosed = 0;
    let mut evaluated = 0;
    let mut rejected = 0;
    let mut ratios = Vec::new();
    for row in &rows {
        for arm in ["grid", "continuous"] {
            let r = &row[arm];
            if let (Some(total), Some(error)) = (
                r["laguerre_majorant_total"].as_f64(),
                r["observed_error"].as_f64(),
            ) {
                evaluated += 1;
                if total >= error {
                    enclosed += 1;
                }
                if total > row["budget"].as_f64().unwrap() {
                    rejected += 1;
                }
                ratios.push(total / error.max(f64::MIN_POSITIVE));
            }
        }
    }
    ratios.sort_by(f64::total_cmp);
    let rejection_rate = rejected as f64 / evaluated.max(1) as f64;
    let poly03_pass = evaluated > 0 && enclosed == evaluated && rejection_rate <= 0.5;
    // POLY-DEV-05: retain the continuous scale only if it never costs more
    // degree, saves degree somewhere, keeps the majorant within 10x and
    // meets the accuracy contract.
    let mut compared = 0;
    let mut not_worse = 0;
    let mut better = 0;
    let mut majorant_ok = 0;
    let mut accurate = 0;
    for row in &rows {
        let (g, c) = (&row["grid"], &row["continuous"]);
        let budget = row["budget"].as_f64().unwrap();
        if let (Some(gd), Some(cd)) = (g["degree"].as_u64(), c["degree"].as_u64()) {
            compared += 1;
            not_worse += usize::from(cd <= gd);
            better += usize::from(cd < gd);
            let (gm, cm) = (
                g["laguerre_majorant_total"]
                    .as_f64()
                    .unwrap_or(f64::INFINITY),
                c["laguerre_majorant_total"]
                    .as_f64()
                    .unwrap_or(f64::INFINITY),
            );
            majorant_ok += usize::from(cm <= 10.0 * gm);
            accurate += usize::from(
                c["observed_error"].as_f64().unwrap_or(f64::INFINITY) <= 10.0 * budget + 1.0e-13,
            );
        }
    }
    let poly05_pass = compared > 0
        && not_worse == compared
        && better * 4 >= compared
        && majorant_ok == compared
        && accurate == compared;
    Ok(json!({
        "schema": "vigilode-r4-laguerre-study-v1",
        "timing_status": TIMING_STATUS,
        "rows": rows,
        "poly03": {
            "evaluated": evaluated, "majorant_encloses": enclosed, "rejected_above_budget": rejected,
            "rejection_rate": rejection_rate,
            "tightness_ratio_median": ratios.get(ratios.len() / 2),
            "tightness_ratio_max": ratios.last(),
            "verdict": if poly03_pass { "PASS" } else { "FAIL" },
        },
        "poly05": {
            "compared": compared, "degree_not_worse": not_worse, "degree_better": better,
            "majorant_within_10x": majorant_ok, "accurate": accurate,
            "verdict": if poly05_pass { "PASS" } else { "FAIL" },
        },
    }))
}

/// R4-POLY-DEV-04 and R4-STAT-DEV-05: cached/uncached x certified/unbounded
/// arms on one frozen operator, and their cost decomposition.
pub fn polynomial_regimes_study(actions: usize) -> Result<Value> {
    let eigenvalues = geomspace(0.1, 100.0, 24);
    let n = eigenvalues.len();
    let op = diagonal(&eigenvalues)?;
    let h = 0.1;
    let budget = 1.0e-12;
    let mut arms = Vec::new();
    let mut contract_ok = true;
    for basis in [PolynomialBasis::Chebyshev, PolynomialBasis::Laguerre] {
        for certified in [true, false] {
            for cached in [true, false] {
                let mut cache = CoefficientCache::default();
                let mut work = WorkCounters::default();
                let (mut cold, mut warm, mut max_error, mut degrees, mut hits) =
                    (0.0, 0.0, 0.0_f64, 0_u64, 0_u64);
                let mut statuses = std::collections::BTreeMap::<String, usize>::new();
                for action in 0..actions {
                    let w = vectors(n, action);
                    let started = Instant::now();
                    let cache_ref = cached.then_some(&mut cache);
                    let report = if certified {
                        joint_phi_action(
                            &op,
                            h,
                            JointPhiInput::Distinct(&w),
                            basis,
                            budget,
                            cache_ref,
                            &mut work,
                        )?
                    } else {
                        joint_phi_action_unbounded(
                            &op,
                            h,
                            JointPhiInput::Distinct(&w),
                            basis,
                            budget,
                            cache_ref,
                            &mut work,
                        )?
                    };
                    let seconds = started.elapsed().as_secs_f64();
                    if action == 0 {
                        cold += seconds
                    } else {
                        warm += seconds
                    }
                    let error = diagonal_error(&eigenvalues, h, &w, &report.fused)? / weight(&w);
                    max_error = max_error.max(error);
                    degrees += report.degree as u64;
                    hits += u64::from(report.coefficient_cache_hit);
                    let status = match report.total_error {
                        TotalErrorStatus::Certified { .. } => "certified",
                        TotalErrorStatus::EstimateOnly { .. } => "estimate-only",
                    };
                    *statuses.entry(status.into()).or_default() += 1;
                }
                let expected_hits = if cached { actions as u64 - 1 } else { 0 };
                let expected_status = if certified && basis == PolynomialBasis::Chebyshev {
                    "certified"
                } else {
                    "estimate-only"
                };
                let reconciles = work.poly_block_products == degrees
                    && work.poly_coefficient_setups + work.poly_coefficient_reuses
                        == actions as u64
                    && work.poly_coefficient_reuses == expected_hits
                    && hits == expected_hits;
                let arm_ok = reconciles
                    && max_error <= 1.0e-10
                    && statuses.get(expected_status) == Some(&actions);
                contract_ok &= arm_ok;
                arms.push(json!({
                    "basis": format!("{basis:?}").to_lowercase(),
                    "path": if certified { "certified-enclosures" } else { "unbounded-timing" },
                    "cache": if cached { "cached" } else { "uncached" },
                    "actions": actions,
                    "max_relative_error": max_error,
                    "statuses": statuses,
                    "work": {
                        "block_products": work.poly_block_products,
                        "vector_products": work.poly_vector_products,
                        "coefficient_setups": work.poly_coefficient_setups,
                        "coefficient_reuses": work.poly_coefficient_reuses,
                        "degree_sum": degrees,
                        "formal_multiply_adds": work.poly_vector_products * (n * n) as u64,
                    },
                    "wall_seconds_diagnostic": { "cold_first_action": cold, "warm_remaining_actions": warm },
                    "counters_reconcile": reconciles,
                    "contract_ok": arm_ok,
                }));
            }
        }
    }
    // Cache identity: alternating h never reuses across h.
    let mut cache = CoefficientCache::default();
    let mut work = WorkCounters::default();
    for action in 0..actions {
        let w = vectors(n, action);
        let step = if action % 2 == 0 { 0.1 } else { 0.2 };
        joint_phi_action(
            &op,
            step,
            JointPhiInput::Distinct(&w),
            PolynomialBasis::Chebyshev,
            budget,
            Some(&mut cache),
            &mut work,
        )?;
    }
    let identity_ok = work.poly_coefficient_setups == 2
        && work.poly_coefficient_reuses == actions as u64 - 2
        && cache.len() == 2;
    contract_ok &= identity_ok;
    // Formal-cost decomposition against an amortized cached eigensystem
    // (dense symmetric: ~9 n^3 setup, 2 n^2 + 25 n per action of five phi).
    let chebyshev_degree = arms[0]["work"]["degree_sum"].as_u64().unwrap() as f64 / actions as f64;
    let per_action_poly = 5.0 * chebyshev_degree * (n * n) as f64;
    let eig_setup = 9.0 * (n * n * n) as f64;
    let eig_per_action = 2.0 * (n * n) as f64 + 25.0 * n as f64;
    let crossover = if per_action_poly > eig_per_action {
        (eig_setup / (per_action_poly - eig_per_action)).ceil()
    } else {
        f64::INFINITY
    };
    // The timing arm's work must be the verification path's work: for each
    // basis and cache mode the unbounded-timing arm runs the same degrees,
    // products and coefficient setups as the certified arm, and every arm's
    // counters reconcile.
    let mut separated = arms.iter().all(|arm| arm["counters_reconcile"] == true);
    for certified in arms
        .iter()
        .filter(|arm| arm["path"] == "certified-enclosures")
    {
        let twin = arms.iter().find(|arm| {
            arm["path"] == "unbounded-timing"
                && arm["basis"] == certified["basis"]
                && arm["cache"] == certified["cache"]
        });
        separated &= twin.is_some_and(|twin| {
            twin["work"]["degree_sum"] == certified["work"]["degree_sum"]
                && twin["work"]["block_products"] == certified["work"]["block_products"]
                && twin["work"]["coefficient_setups"] == certified["work"]["coefficient_setups"]
        });
    }
    Ok(json!({
        "schema": "vigilode-r4-polynomial-regimes-v1",
        "timing_status": TIMING_STATUS,
        "operator": { "family": "diag24-wide", "dimension": n, "h": h, "budget": budget },
        "arms": arms,
        "cache_identity": { "alternating_h_setups": work.poly_coefficient_setups, "reuses": work.poly_coefficient_reuses, "keys": cache.len(), "ok": identity_ok },
        "formal_costs": {
            "chebyshev_mean_degree": chebyshev_degree,
            "polynomial_multiply_adds_per_action": per_action_poly,
            "eigensystem_setup_multiply_adds": eig_setup,
            "eigensystem_multiply_adds_per_action": eig_per_action,
            "eigensystem_amortized_crossover_actions": crossover,
            "model": "dense symmetric eigensystem ~9 n^3 once; Q^T w and Q (phi o) per action, 2 n^2 + 25 n; polynomial 5 m n^2 (five distinct columns)",
        },
        "poly04": { "verdict": if contract_ok { "PASS" } else { "FAIL" } },
        "stat05": { "cost_fields_separated": separated, "verdict": if separated { "PASS" } else { "FAIL" } },
    }))
}

/// R4-HOM-DEV-07: serial, full doubling and component-blocked certificate
/// cost on one candidate per dimension, and the q1/q2/fallback rates that
/// enter the batch budget `(7 - p1) ceil(8/P) + 8 pf`.
pub fn homotopy_cost_study() -> Result<Value> {
    let coeffs = rodas5p_coefficients()?;
    let target = StageTarget::sequential(coeffs)?;
    let execution = ParallelExecution::sequential();
    let mut rows = Vec::new();
    let mut all_identical = true;
    for n in [1_usize, 2, 4, 8, 16] {
        let a = (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| if i == j { -1.0 - i as f64 } else { 0.0 })
                    .collect()
            })
            .collect::<Vec<Vec<f64>>>();
        let q = (0..n)
            .map(|i| -0.05 * (1 + i % 3) as f64)
            .collect::<Vec<_>>();
        let model = QuadraticModel::new(format!("diagonal-quadratic-{n}"), a, q)?;
        let y = (0..n).map(|i| 1.0 + 0.1 * i as f64).collect::<Vec<_>>();
        let h = 0.05;
        let problem = model.stage_problem(0.0, &y, h)?;
        let f = (0..n)
            .map(|i| {
                problem.jacobian[i][i] * y[i] - 2.0 * problem.q[i] * y[i] * y[i]
                    + problem.q[i] * y[i] * y[i]
            })
            .collect::<Vec<_>>();
        let candidate = vec![f.iter().map(|x| h * x).collect::<Vec<_>>(); 8];
        let y_hat = y.iter().zip(&f).map(|(y, f)| y + h * f).collect::<Vec<_>>();
        let e_hat = vec![0.0; n];
        let witness = InverseWitness::diagonal(&problem, target.gamma)?;
        let serial = certify_stage_target(
            &target, &problem, &candidate, &y_hat, &e_hat, &witness, 1.0e-8, 1.0e-6,
        )?;
        let full = doubling_certificate_with_execution(
            &target, &problem, &candidate, &y_hat, &e_hat, &witness, 1.0e-8, 1.0e-6, 1.0e-3, 6,
            &execution,
        )?;
        let blocked = blocked_doubling_certificate_with_execution(
            &target, &problem, &candidate, &y_hat, &e_hat, &witness, 1.0e-8, 1.0e-6, 1.0e-3, 6,
            &execution,
        )?;
        let identical = match (&full.certificate, &blocked.doubling.certificate) {
            (Some(f), Some(b)) => f
                .stage_bound
                .iter()
                .flatten()
                .zip(b.stage_bound.iter().flatten())
                .all(|(x, y)| x.to_bits() == y.to_bits()),
            // Both must certify: two failures are not an identity.
            _ => false,
        };
        all_identical &= identical;
        let full_ops = full.certificate.as_ref().map(|c| c.directed_operations);
        let blocked_ops = blocked
            .doubling
            .certificate
            .as_ref()
            .map(|c| c.directed_operations);
        // The full path charges its formal dense products, (2L - 1) (8n)^3
        // per radius attempt; the blocked path counts its operations. Both
        // are reported, neither is gated (the ratio is n^2 by construction).
        let width_ratio = full.certificate.as_ref().map(|c| {
            c.output_bound
                .iter()
                .zip(&serial.output_bound)
                .map(|(d, s)| d / s.max(f64::MIN_POSITIVE))
                .fold(0.0_f64, f64::max)
        });
        // Lane rates of a short certified integration.
        let ode = model.ode_problem()?;
        let run = integrate_transactional_q1_q2_adaptive_observed_with_admission(
            &ode,
            (0.0, 0.5),
            &y,
            &TransactionalQ1Q2Config::default(),
            &AdaptiveStepConfig {
                atol: 1.0e-8,
                rtol: 1.0e-6,
                initial_step: 1.0e-3,
                max_attempts: 2000,
                ..Default::default()
            },
            &OutputSchedule::new(vec![0.0, 0.5])?,
            Q2Admission::NativeTargetCertificate(&model),
        )?;
        let d = &run.transactional;
        let accepted = d.accepted_steps().max(1) as f64;
        let (p1, pf) = (
            d.accepted_q1_fast_steps as f64 / accepted,
            d.accepted_sequential_fallback_steps as f64 / accepted,
        );
        let budget = [1_u64, 2, 4, 8].map(|p| {
            let waves = 8_u64.div_ceil(p) as f64;
            json!({ "workers": p, "batch_units": (7.0 - p1) * waves + 8.0 * pf, "sequential_units": 8.0 })
        });
        rows.push(json!({
            "dimension": n,
            "serial_operations": serial.directed_operations,
            "doubling_operations_formal_dense": full_ops,
            "blocked_operations_counted": blocked_ops,
            "blocked_allocated_values": blocked.work.allocated_values,
            "full_block_matrix_values": (8 * n) * (8 * n),
            "blocked_bit_identical": identical,
            "doubling_over_serial_output_bound_max": width_ratio,
            "radius_attempts": full.attempts.len(),
            "lane_rates": { "p1": p1, "pf": pf, "accepted_steps": d.accepted_steps(), "certificate_attempts": d.certificate_attempts, "certificate_admissions": d.certificate_admissions, "certificate_operations": d.certificate_operations, "w_solve_batches": d.total_w_solve_batches, "success": run.observed.success },
            "batch_budget_idealized": budget,
        }));
    }
    let pass = all_identical;
    Ok(json!({
        "schema": "vigilode-r4-homotopy-certificate-cost-v1",
        "timing_status": TIMING_STATUS,
        "speedup_status": "SPEEDUP_UNPROVEN: the matched paired-timing campaign is not run while timing authority is on hold",
        "budget_assumptions": "equal W-solve cost c_W per vector, s = 8, ceil(8/P) waves per batch, fallback adds 8 sequential solves; certificate, RHS, pool and dispatch costs are excluded and must be below 1 + p1 - 8 pf solves for any gain",
        "rows": rows,
        "verdict": if pass { "PASS" } else { "FAIL" },
    }))
}
