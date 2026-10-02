//! Laguerre adjoint envelope cache and cost (research node
//! `research/thread_transfer_laguerre_cache_20261002`, thread-transfer DAG
//! node P2-LAGUERRE-CACHE). The exact check of the same-vector combination
//! is `tools/thread_transfer_laguerre_cache_check.py`.

use rodas5p_core::{
    DenseMatrix,
    directed::{Interval, add_up, mul_up},
    laguerre_adjoint::{
        EnvelopeKey, LAGUERRE_ADJOINT_DEPTH, LAGUERRE_ADJOINT_PROOF_VERSION, LaguerreEnvelopeCache,
        laguerre_adjoint_envelopes_counted, laguerre_adjoint_envelopes_interval,
    },
    polynomial_action::{laguerre_coefficient_enclosures, laguerre_recurrence_with_residuals},
};
use serde_json::{Value, json};

const TERMS: usize = 5;

fn hex(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

fn midpoint(e: Interval) -> f64 {
    (0.5 * e.lo + 0.5 * e.hi).clamp(e.lo, e.hi)
}

/// Stored (midpoint) coefficients `c_(n,k)`, as the action chooses them.
fn stored_table(h: f64, degree: usize) -> Vec<[f64; TERMS]> {
    laguerre_coefficient_enclosures(h, 1.0, degree)
        .unwrap()
        .iter()
        .map(|row| row.map(midpoint))
        .collect()
}

fn points(values: &[f64]) -> Vec<Interval> {
    values
        .iter()
        .map(|v| Interval::point(*v).unwrap())
        .collect()
}

fn bound(envelopes: &[f64], local: &[f64]) -> f64 {
    envelopes[1..].iter().zip(local).fold(0.0, |acc, (b, e)| {
        add_up(acc, mul_up(*b, *e).unwrap()).unwrap()
    })
}

fn invalidation() -> (bool, bool, Value) {
    let (extent, depth) = (4.0, LAGUERRE_ADJOINT_DEPTH);
    let table = stored_table(2.0, 32);
    let base = points(&table.iter().map(|r| r[0]).collect::<Vec<_>>());
    let mut cache = LaguerreEnvelopeCache::default();
    let (cold, hit0, cold_ops) = cache.envelopes(&base, extent, depth).unwrap();
    let (warm, hit1, warm_ops) = cache.envelopes(&base, extent, depth).unwrap();
    let identical = cold
        .iter()
        .zip(&warm)
        .all(|(a, b)| a.to_bits() == b.to_bits());
    let mut bumped = base.clone();
    bumped[5] = Interval::point(base[5].lo.next_up()).unwrap();
    let other_h = points(
        &stored_table(2.0_f64.next_up(), 32)
            .iter()
            .map(|r| r[0])
            .collect::<Vec<_>>(),
    );
    let variants: Vec<(&str, Vec<Interval>, f64, usize)> = vec![
        ("degree", base[..32].to_vec(), extent, depth),
        ("extent-one-ulp", base.clone(), extent.next_up(), depth),
        ("depth", base.clone(), extent, depth + 1),
        ("coefficient-one-ulp", bumped, extent, depth),
        ("coefficients-of-another-h", other_h.clone(), extent, depth),
    ];
    let mut rows = Vec::new();
    let mut all_miss = other_h != base;
    for (name, coefficients, x, d) in variants {
        let before = cache.misses;
        let (_, hit, ops) = cache.envelopes(&coefficients, x, d).unwrap();
        let missed = !hit && cache.misses == before + 1 && ops > 0;
        all_miss &= missed;
        rows.push(json!({"change": name, "missed": missed, "setup_operations": ops}));
    }
    let version_differs = EnvelopeKey::new(&base, extent, depth, LAGUERRE_ADJOINT_PROOF_VERSION)
        != EnvelopeKey::new(
            &base,
            extent,
            depth,
            "laguerre-adjoint-bernstein-interval-v2",
        );
    all_miss &= version_differs;
    rows.push(json!({"change": "proof-version", "missed": version_differs}));
    let invalidation_ok = !hit0 && hit1 && identical && all_miss;
    let separated = cold_ops > 0 && warm_ops == 0 && cache.hits == 1 && cache.misses == 6;
    (
        invalidation_ok,
        separated,
        json!({"identical_key_hits_bitwise": hit1 && identical, "variants": rows,
               "cold_operations": cold_ops, "warm_operations": warm_ops,
               "hits": cache.hits, "misses": cache.misses}),
    )
}

/// A diagonal X = diag(L/8, L/2, L) as A = -X with beta = 1.
fn diagonal(extent: f64) -> DenseMatrix {
    let mut a = DenseMatrix::zeros(3, 3);
    for (i, x) in [extent / 8.0, extent / 2.0, extent].iter().enumerate() {
        a[(i, i)] = -x;
    }
    a
}

fn combination_case(degree: usize, extent: f64) -> Value {
    let h = 2.0;
    let table = stored_table(h, degree);
    let scales: [f64; TERMS] = std::array::from_fn(|k| 1.0 / (k + 1) as f64);
    let source = vec![1.0, -0.75, 0.125];
    let a = diagonal(extent);
    let (vectors, local) = laguerre_recurrence_with_residuals(&a, 1.0, &source, degree).unwrap();
    // Per column: |s_k| sum_j beta_j(c_k) eps_j.
    let mut column_sum = 0.0;
    let mut columns = Vec::new();
    for k in 0..TERMS {
        let stored = table.iter().map(|r| r[k]).collect::<Vec<_>>();
        let (beta, _) =
            laguerre_adjoint_envelopes_counted(&stored, extent, LAGUERRE_ADJOINT_DEPTH).unwrap();
        let b = mul_up(scales[k].abs(), bound(&beta, &local)).unwrap();
        column_sum = add_up(column_sum, b).unwrap();
        columns.push(b);
    }
    // Combined: C_n = sum_k s_k c_(n,k) enclosed exactly.
    let combined = table
        .iter()
        .map(|row| {
            let mut total = Interval::point(0.0).unwrap();
            for k in 0..TERMS {
                total = total
                    .add(
                        Interval::point(scales[k])
                            .unwrap()
                            .mul(Interval::point(row[k]).unwrap())
                            .unwrap(),
                    )
                    .unwrap();
            }
            total
        })
        .collect::<Vec<_>>();
    let (beta, ops) =
        laguerre_adjoint_envelopes_interval(&combined, extent, LAGUERRE_ADJOINT_DEPTH).unwrap();
    let combined_bound = bound(&beta, &local);
    json!({
        "degree": degree, "extent": hex(extent), "h": hex(h),
        "matrix": (0..3).map(|i| (0..3).map(|j| hex(a[(i, j)])).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "source": source.iter().map(|v| hex(*v)).collect::<Vec<_>>(),
        "scales": scales.iter().map(|v| hex(*v)).collect::<Vec<_>>(),
        "stored": table.iter().map(|r| r.iter().map(|v| hex(*v)).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "vectors": vectors.iter().map(|v| v.iter().map(|x| hex(*x)).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "combined_bound": hex(combined_bound),
        "summary": {"combined_bound": combined_bound, "column_bound_sum": column_sum,
            "combined_over_columns": combined_bound / column_sum, "column_bounds": columns,
            "combined_setup_operations": ops},
    })
}

fn cases() -> Vec<Value> {
    vec![combination_case(16, 4.0), combination_case(32, 4.0)]
}

#[test]
#[ignore = "writes fixtures/thread_transfer_laguerre_cache_cases.json"]
fn write_laguerre_cache_cases() {
    let value =
        json!({"schema": "vigilode-thread-transfer-laguerre-cache-cases-v1", "cases": cases()});
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/thread_transfer_laguerre_cache_cases.json");
    std::fs::write(&path, serde_json::to_string_pretty(&value).unwrap() + "\n").unwrap();
}

#[test]
fn laguerre_envelope_cache_and_cost() {
    let (invalidation_ok, separated, invalidation_rows) = invalidation();
    // Setup cost against the action.
    let mut cost = Vec::new();
    let mut setups = Vec::new();
    for m in [16_usize, 32, 64, 128] {
        let stored = (0..=m)
            .map(|n| 0.25 * 0.75_f64.powi(n as i32))
            .collect::<Vec<_>>();
        let (_, ops) =
            laguerre_adjoint_envelopes_counted(&stored, 4.0, LAGUERRE_ADJOINT_DEPTH).unwrap();
        setups.push((m as f64, ops as f64));
        let actions = [8_u64, 64].map(|n| {
            let flops = m as u64 * (n * n + 8 * n);
            json!({"n": n, "action_flops": flops, "actions_to_amortize_one_setup": ops as f64 / flops as f64})
        });
        cost.push(json!({"degree": m, "setup_interval_operations": ops, "per_action": actions}));
    }
    let exponent = (setups[3].1 / setups[0].1).ln() / (setups[3].0 / setups[0].0).ln();
    let combos = cases();
    let result = json!({
        "schema": "vigilode-thread-transfer-laguerre-cache-v1",
        "gate_native": {"invalidation": invalidation_ok, "cold_warm_separated": separated},
        "note": "gate item 3 (combination validity) is in EXACT_CHECK.json",
        "invalidation": invalidation_rows,
        "setup_cost": cost,
        "setup_cost_fitted_exponent": exponent,
        "combination": combos.iter().map(|c| json!({"degree": c["degree"], "summary": c["summary"]})).collect::<Vec<_>>(),
    });
    if let Ok(path) = std::env::var("THREAD_TRANSFER_LAGUERRE_CACHE_OUTPUT") {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        std::fs::write(&path, serde_json::to_string_pretty(&result).unwrap() + "\n").unwrap();
    }
    println!(
        "{}",
        json!({"gate_native": result["gate_native"], "exponent": exponent,
        "combination": result["combination"]})
    );
    assert!(invalidation_ok && separated);
}
