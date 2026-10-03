//! End-to-end Laguerre total admission (research node
//! `research/rnext04_laguerre_admission_20261003`, remaining-only DAG node
//! R-NEXT-04). Writes the cases to `RNEXT04_CASES` for the independent
//! 50-digit check `tools/rnext04_laguerre_check.py`.

use rodas5p_core::{
    CoreError, DenseMatrix, WorkCounters,
    laguerre_adjoint::{
        EnvelopeKey, LAGUERRE_ADJOINT_DEGREE_LIMIT, LAGUERRE_ADJOINT_MAX_DEPTH,
        LAGUERRE_ADJOINT_PROOF_VERSION, LaguerreEnvelopeCache, laguerre_adjoint_envelopes,
    },
    polynomial_action::{
        EnclosureEvidence, JointPhiInput, JointPhiReport, PolynomialBasis,
        SymmetricNonpositiveOperator, TotalErrorAdmission, TotalErrorStatus, joint_phi_action,
        joint_phi_action_unbounded,
    },
};
use serde_json::{Value, json};

fn hex(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}

fn splitmix(seed: u64, n: usize) -> Vec<f64> {
    let mut state = seed;
    (0..n)
        .map(|_| {
            state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^= z >> 31;
            2.0 * ((z >> 11) as f64 / (1u64 << 53) as f64) - 1.0
        })
        .collect()
}

/// Diagonally dominant symmetric nonpositive matrix: `a_ii = -d_i` with
/// `d_i in [0.3 rho, rho]`, symmetric off-diagonal entries whose row sums
/// stay below `0.2 d_i`, so the Gershgorin discs lie in `[-1.2 rho, 0)`.
fn matrix(n: usize, rho: f64, seed: u64) -> DenseMatrix {
    let d: Vec<f64> = splitmix(seed, n)
        .iter()
        .map(|u| rho * (0.65 + 0.35 * u))
        .collect();
    let raw = splitmix(seed + 1000, n * n);
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = -d[i];
        for j in 0..i {
            let cap = 0.2 * d[i].min(d[j]) / n as f64;
            let v = cap * raw[i * n + j];
            a[(i, j)] = v;
            a[(j, i)] = v;
        }
    }
    a
}

fn operator(a: DenseMatrix, rho: f64) -> SymmetricNonpositiveOperator {
    SymmetricNonpositiveOperator::new(a, 0.0, 1.2 * rho * (1.0 + 1.0e-12), "test").unwrap()
}

struct Outcome {
    record: Value,
    admitted: bool,
}

fn admission_json(a: &TotalErrorAdmission) -> Value {
    match a {
        TotalErrorAdmission::Admitted { bound, .. } => {
            json!({"admitted": true, "bound": hex(*bound), "bound_f64": bound})
        }
        TotalErrorAdmission::Rejected { reason } => json!({"admitted": false, "reason": reason}),
    }
}

/// Gate item 1 (as amended): the registry's recomputation equals the total
/// inside the normalization window and bounds it closely outside.
fn coverage(report: &JointPhiReport) -> Value {
    let recomputed = report.laguerre_total_recomputed().unwrap();
    match (recomputed, report.laguerre_adjoint_total) {
        (Some(r), Some(total)) => {
            let n = report.dimension as f64;
            let holds = if report.normalization_shift == 0 {
                r.to_bits() == total.to_bits()
            } else {
                r >= total
                    && r <= total * (1.0 + 1.0e-12) + (5.0 * n.sqrt() + 64.0) * f64::from_bits(1)
            };
            json!({"recomputed": r, "total": total, "holds": holds})
        }
        (None, None) => json!({"recomputed": null, "total": null, "holds": true}),
        (r, t) => json!({"recomputed": r, "total": t, "holds": false}),
    }
}

fn record(
    label: &str,
    a: &DenseMatrix,
    h: f64,
    w: &[Vec<f64>; 5],
    report: &JointPhiReport,
    budget: f64,
) -> Outcome {
    let admission = report.admit_laguerre_total(budget);
    let admitted = matches!(admission, TotalErrorAdmission::Admitted { .. });
    let n = a.nrows();
    Outcome {
        admitted,
        record: json!({
            "label": label,
            "n": n,
            "a": (0..n).map(|i| (0..n).map(|j| hex(a[(i, j)])).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "h": hex(h),
            "w": w.iter().map(|v| v.iter().map(|x| hex(*x)).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "fused": report.fused.iter().map(|x| hex(*x)).collect::<Vec<_>>(),
            "basis": format!("{:?}", report.basis),
            "branch": report.branch,
            "degree": report.degree,
            "laguerre_scale": report.laguerre_scale,
            "normalization_shift": report.normalization_shift,
            "evidence": format!("{:?}", report.evidence),
            "execution": report.execution,
            "total_error_is_estimate": matches!(report.total_error, TotalErrorStatus::EstimateOnly { .. }),
            "condition_proxy": report.condition_proxy,
            "budget": budget,
            "admission": admission_json(&admission),
            "coverage": coverage(report),
        }),
    }
}

fn distinct(n: usize, seed: u64, amplitude: f64) -> [Vec<f64>; 5] {
    std::array::from_fn(|k| {
        splitmix(seed + 77 * k as u64, n)
            .iter()
            .map(|v| amplitude * v)
            .collect()
    })
}

fn same_vector(n: usize, seed: u64) -> ([Vec<f64>; 5], Vec<f64>, [f64; 5]) {
    let v = splitmix(seed, n);
    let scales = [1.0, -0.5, 0.25, 2.0, -1.0];
    let w = std::array::from_fn(|k| v.iter().map(|x| scales[k] * x).collect());
    (w, v, scales)
}

const BUDGET_FACTOR: f64 = 1.0e-6;

#[test]
fn laguerre_admission_cases() {
    let mut cases = Vec::new();
    let mut guards = serde_json::Map::new();
    let mut gate_coverage = true;
    let mut gate_guards = true;
    let mut gate_estimate = true;

    // Main suite: verified non-scalar symmetric operators, admitted where
    // the total meets a generous budget.
    let mut seed = 1;
    for n in [6, 12, 20] {
        for rho in [1.0, 50.0, 400.0] {
            for h in [1.0e-3, 1.0e-2, 0.1] {
                seed += 1;
                let a = matrix(n, rho, seed);
                let op = operator(a.clone(), rho);
                for same in [false, true] {
                    let mut work = WorkCounters::default();
                    let (w, report) = if same {
                        let (w, v, scales) = same_vector(n, seed + 5);
                        let report = joint_phi_action(
                            &op,
                            h,
                            JointPhiInput::SameVector { vector: &v, scales },
                            PolynomialBasis::Laguerre,
                            1.0e-10,
                            None,
                            &mut work,
                        );
                        (w, report)
                    } else {
                        let w = distinct(n, seed + 9, 1.0);
                        let report = joint_phi_action(
                            &op,
                            h,
                            JointPhiInput::Distinct(&w),
                            PolynomialBasis::Laguerre,
                            1.0e-10,
                            None,
                            &mut work,
                        );
                        (w, report)
                    };
                    let label = format!(
                        "n{n}-rho{rho}-h{h}-{}",
                        if same { "same" } else { "distinct" }
                    );
                    match report {
                        Ok(report) => {
                            let norm = report.fused.iter().map(|x| x * x).sum::<f64>().sqrt();
                            let out =
                                record(&label, &a, h, &w, &report, BUDGET_FACTOR * norm.max(1.0));
                            gate_coverage &= out.record["coverage"]["holds"].as_bool().unwrap();
                            gate_estimate &= report.branch == "scalar"
                                || out.record["total_error_is_estimate"].as_bool().unwrap();
                            cases.push(out.record);
                        }
                        Err(error) => {
                            cases.push(json!({"label": label, "error": error.to_string()}))
                        }
                    }
                }
            }
        }
    }

    // Near cancellation: w_0 = v, w_1 = -v.
    let (n, rho, h) = (12, 1.0, 1.0e-3);
    let a = matrix(n, rho, 501);
    let v = splitmix(502, n);
    let mut w: [Vec<f64>; 5] = std::array::from_fn(|_| vec![0.0; n]);
    w[0] = v.clone();
    w[1] = v.iter().map(|x| -x).collect();
    let report = joint_phi_action(
        &operator(a.clone(), rho),
        h,
        JointPhiInput::Distinct(&w),
        PolynomialBasis::Laguerre,
        1.0e-12,
        None,
        &mut WorkCounters::default(),
    )
    .unwrap();
    let out = record("near-cancellation", &a, h, &w, &report, 1.0e-6);
    gate_coverage &= out.record["coverage"]["holds"].as_bool().unwrap();
    cases.push(out.record);

    // Extreme amplitudes: subnormal and large inputs (normalization path).
    for (label, amplitude) in [("subnormal-1e-310", 1.0e-310), ("large-1e300", 1.0e300)] {
        let (n, rho, h) = (6, 50.0, 1.0e-2);
        let a = matrix(n, rho, 601);
        let w = distinct(n, 602, amplitude);
        let report = joint_phi_action(
            &operator(a.clone(), rho),
            h,
            JointPhiInput::Distinct(&w),
            PolynomialBasis::Laguerre,
            1.0e-10 * amplitude,
            None,
            &mut WorkCounters::default(),
        )
        .unwrap();
        let out = record(label, &a, h, &w, &report, amplitude);
        gate_coverage &= out.record["coverage"]["holds"].as_bool().unwrap();
        gate_estimate &= out.record["total_error_is_estimate"].as_bool().unwrap();
        cases.push(out.record);
    }

    // Scalar branch: A = -2 I defers to total_error.
    let mut scalar = DenseMatrix::zeros(4, 4);
    for i in 0..4 {
        scalar[(i, i)] = -2.0;
    }
    let w = distinct(4, 701, 1.0);
    let report = joint_phi_action(
        &SymmetricNonpositiveOperator::new(scalar.clone(), 2.0, 2.0, "test").unwrap(),
        0.1,
        JointPhiInput::Distinct(&w),
        PolynomialBasis::Laguerre,
        1.0e-10,
        None,
        &mut WorkCounters::default(),
    )
    .unwrap();
    let out = record("scalar-branch", &scalar, 0.1, &w, &report, 1.0e-6);
    let defers = report.branch == "scalar"
        && (out.admitted
            == matches!(
                report.admit_total_error(1.0e-6),
                TotalErrorAdmission::Admitted { .. }
            ));
    gate_guards &= defers;
    guards.insert("scalar_branch_defers".into(), json!(defers));
    cases.push(out.record);

    // Guards that must reject.
    let (n, rho, h) = (6, 50.0, 1.0e-2);
    let a = matrix(n, rho, 801);
    let w = distinct(n, 802, 1.0);
    // Declared enclosure: the largest diagonal magnitude, which contains the
    // diagonal but not the Gershgorin discs, so it cannot be verified.
    let diagonal_max = (0..n).map(|i| a[(i, i)].abs()).fold(0.0, f64::max);
    let declared =
        SymmetricNonpositiveOperator::new(a.clone(), 0.0, diagonal_max, "declared").unwrap();
    let report = joint_phi_action(
        &declared,
        h,
        JointPhiInput::Distinct(&w),
        PolynomialBasis::Laguerre,
        1.0e-10,
        None,
        &mut WorkCounters::default(),
    )
    .unwrap();
    let declared_ok = matches!(report.evidence, EnclosureEvidence::Declared { .. })
        && !matches!(
            report.admit_laguerre_total(1.0),
            TotalErrorAdmission::Admitted { .. }
        );
    guards.insert("declared_enclosure_rejected".into(), json!(declared_ok));
    gate_guards &= declared_ok;

    let verified = operator(a.clone(), rho);
    let timing = joint_phi_action_unbounded(
        &verified,
        h,
        JointPhiInput::Distinct(&w),
        PolynomialBasis::Laguerre,
        1.0e-10,
        None,
        &mut WorkCounters::default(),
    )
    .unwrap();
    let timing_ok = !matches!(
        timing.admit_laguerre_total(1.0),
        TotalErrorAdmission::Admitted { .. }
    );
    guards.insert("timing_execution_rejected".into(), json!(timing_ok));
    gate_guards &= timing_ok;

    let chebyshev = joint_phi_action(
        &verified,
        h,
        JointPhiInput::Distinct(&w),
        PolynomialBasis::Chebyshev,
        1.0e-10,
        None,
        &mut WorkCounters::default(),
    )
    .unwrap();
    let cheb_ok = !matches!(
        chebyshev.admit_laguerre_total(1.0),
        TotalErrorAdmission::Admitted { .. }
    );
    guards.insert("chebyshev_rejected".into(), json!(cheb_ok));
    gate_guards &= cheb_ok;

    let good = joint_phi_action(
        &verified,
        h,
        JointPhiInput::Distinct(&w),
        PolynomialBasis::Laguerre,
        1.0e-10,
        None,
        &mut WorkCounters::default(),
    )
    .unwrap();
    let bound = good.laguerre_adjoint_total.unwrap();
    let budget_ok = [f64::NAN, -1.0, f64::INFINITY, bound * 0.5]
        .iter()
        .all(|b| {
            !matches!(
                good.admit_laguerre_total(*b),
                TotalErrorAdmission::Admitted { .. }
            )
        })
        && matches!(
            good.admit_laguerre_total(bound),
            TotalErrorAdmission::Admitted { .. }
        );
    guards.insert("budget_guards".into(), json!(budget_ok));
    gate_guards &= budget_ok;

    // Degree above the adjoint limit: a tight budget at a large h rho.
    let big = matrix(6, 400.0, 901);
    let big_op = operator(big, 400.0);
    let mut above = json!({"found": false});
    for (h, budget) in [(0.5, 1.0e-14), (1.0, 1.0e-14), (1.4, 1.0e-15)] {
        if let Ok(report) = joint_phi_action(
            &big_op,
            h,
            JointPhiInput::Distinct(&distinct(6, 902, 1.0)),
            PolynomialBasis::Laguerre,
            budget,
            None,
            &mut WorkCounters::default(),
        ) && report.degree > LAGUERRE_ADJOINT_DEGREE_LIMIT
        {
            let rejected = !matches!(
                report.admit_laguerre_total(1.0e300),
                TotalErrorAdmission::Admitted { .. }
            );
            above = json!({"found": true, "h": h, "degree": report.degree, "rejected": rejected,
                           "total_error_is_estimate": matches!(report.total_error, TotalErrorStatus::EstimateOnly { .. })});
            break;
        }
    }
    let above_ok = above["found"].as_bool().unwrap() && above["rejected"].as_bool().unwrap();
    guards.insert("degree_above_limit".into(), above);
    gate_guards &= above_ok;

    // Item 4: bounded resources.
    let stored = vec![0.5, -0.25, 0.125];
    let refused = matches!(
        laguerre_adjoint_envelopes(&stored, 4.0, LAGUERRE_ADJOINT_MAX_DEPTH + 1),
        Err(CoreError::InvalidInput(ref m)) if m.contains("LAGUERRE_ADJOINT_UNSUPPORTED")
    );
    let refused_huge = laguerre_adjoint_envelopes(&stored, 4.0, usize::MAX).is_err();
    let max_ok = laguerre_adjoint_envelopes(&stored, 4.0, LAGUERRE_ADJOINT_MAX_DEPTH).is_ok();
    let intervals: Vec<_> = stored
        .iter()
        .map(|c| rodas5p_core::directed::Interval::point(*c).unwrap())
        .collect();
    let key = EnvelopeKey::new(&intervals, 4.0, 3, LAGUERRE_ADJOINT_PROOF_VERSION);
    let mut other_limits = key.clone();
    other_limits.max_depth = LAGUERRE_ADJOINT_MAX_DEPTH + 4;
    let mut other_degree = key.clone();
    other_degree.degree_limit = LAGUERRE_ADJOINT_DEGREE_LIMIT * 2;
    let keys_differ = key != other_limits && key != other_degree;
    let mut cache = LaguerreEnvelopeCache::default();
    cache.envelopes(&intervals, 4.0, 3).unwrap();
    let (_, hit, _) = cache.envelopes(&intervals, 4.0, 3).unwrap();
    let resources = json!({"depth_above_max_refused": refused, "depth_usize_max_refused": refused_huge,
        "depth_at_max_accepted": max_ok, "keys_with_other_limits_differ": keys_differ, "same_key_hits": hit});
    let gate_resources = refused && refused_huge && max_ok && keys_differ && hit;

    let admitted = cases
        .iter()
        .filter(|c| c["admission"]["admitted"].as_bool() == Some(true))
        .count();
    let report = json!({
        "schema": "vigilode-rnext04-laguerre-cases-v1",
        "node": "research/rnext04_laguerre_admission_20261003",
        "cases": cases,
        "guards": guards,
        "resources": resources,
        "native_gate": {
            "1_coverage": gate_coverage,
            "3_guards": gate_guards,
            "3_laguerre_recurrence_reports_stay_estimate_only": gate_estimate,
            "4_bounded_resources": gate_resources,
        },
        "admitted_cases": admitted,
    });
    if let Ok(path) = std::env::var("RNEXT04_CASES") {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        assert!(!path.exists(), "immutable output exists");
        std::fs::write(&path, serde_json::to_string_pretty(&report).unwrap() + "\n").unwrap();
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report["native_gate"]).unwrap()
    );
    println!(
        "admitted {admitted} of {}",
        report["cases"].as_array().unwrap().len()
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&report["guards"]).unwrap()
    );
}
