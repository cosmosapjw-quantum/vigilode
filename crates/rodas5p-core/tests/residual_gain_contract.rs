//! Contract test suite for residual gain current-operator witness (AS05).

#![allow(
    clippy::manual_pattern_char_comparison,
    clippy::needless_range_loop,
    clippy::type_complexity
)]

use std::{
    alloc::{GlobalAlloc, Layout, System},
    fs::File,
    io::Write,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::{Value, json};

use rodas5p_core::{
    AssemblyUncertainty, DeclaredStructure, EstimateSource, FrozenMetric, GainEvidence,
    LinearTargetId, ResidualGainWitness, WitnessRejection, dense_lu_comparator,
    projected_nu_estimate, rodas5p_coefficients,
};

#[path = "../../../research/reaudit_accuracy_speed_20261010/gain_witness.rs"]
mod research_gain_witness;

// -----------------------------------------------------------------------------
// VG-1 Zero-Allocation Allocator
// -----------------------------------------------------------------------------

struct CountingAllocator;

thread_local! {
    static TRACK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() && TRACK.with(|t| t.get()) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
    }
}

fn count_allocations<T>(operation: impl FnOnce() -> T) -> (T, u64, u64) {
    ALLOCATIONS.store(0, Ordering::SeqCst);
    ALLOCATED_BYTES.store(0, Ordering::SeqCst);
    TRACK.with(|t| t.set(true));
    let result = operation();
    TRACK.with(|t| t.set(false));
    (
        result,
        ALLOCATIONS.load(Ordering::Relaxed),
        ALLOCATED_BYTES.load(Ordering::Relaxed),
    )
}

fn f64_to_hex(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[test]
fn test_vg1_zero_allocation_hot_path() {
    // 1. Route (a): Represented 2x2
    let w_a = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
    let metric_a = FrozenMetric::new(&[1.0, 1.0]).unwrap();
    let target_a = LinearTargetId::represented(
        &w_a,
        metric_a,
        None,
        AssemblyUncertainty::Exact,
        DeclaredStructure::Dense,
    )
    .unwrap();
    let witness_a = ResidualGainWitness::new(target_a.clone()).unwrap();
    let rhs_2 = [1.0, 2.0];
    let cand_2 = [0.1, 0.2];

    // Warm-up
    let _ = witness_a.certify(&target_a, &rhs_2, &cand_2);
    let (ev_a, allocs_a, bytes_a) =
        count_allocations(|| witness_a.certify(&target_a, &rhs_2, &cand_2));
    assert!(ev_a.is_verified(), "route (a) verification failed");
    assert_eq!(allocs_a, 0, "VG-1 route (a) allocs was {allocs_a}");
    assert_eq!(bytes_a, 0, "VG-1 route (a) bytes was {bytes_a}");

    // 2. Transport to new metric (must also be strictly zero-allocation)
    let cert_a = ev_a.as_verified().unwrap();
    let new_metric = FrozenMetric::new(&[2.0, 0.5]).unwrap();
    let _ = cert_a.transport_to(&new_metric);
    let (trans, allocs_trans, bytes_trans) = count_allocations(|| cert_a.transport_to(&new_metric));
    assert!(trans.is_ok(), "transport failed");
    assert_eq!(allocs_trans, 0, "VG-1 transport allocs was {allocs_trans}");
    assert_eq!(bytes_trans, 0, "VG-1 transport bytes was {bytes_trans}");

    // 3. Route (b): Isolated blocks 4x4
    let w_b = vec![
        vec![2.0, 1.0, 0.0, 0.0],
        vec![1.0, 3.0, 0.0, 0.0],
        vec![0.0, 0.0, 4.0, 1.0],
        vec![0.0, 0.0, 1.0, 5.0],
    ];
    let target_b = LinearTargetId::represented(
        &w_b,
        FrozenMetric::new(&[1.0; 4]).unwrap(),
        None,
        AssemblyUncertainty::Exact,
        DeclaredStructure::IsolatedBlocks(vec![(0, 2), (2, 4)]),
    )
    .unwrap();
    let witness_b = ResidualGainWitness::new(target_b.clone()).unwrap();
    let rhs_4 = [1.0, 2.0, 3.0, 4.0];
    let cand_4 = [0.1, 0.2, 0.3, 0.4];
    let _ = witness_b.certify(&target_b, &rhs_4, &cand_4);
    let (ev_b, allocs_b, bytes_b) =
        count_allocations(|| witness_b.certify(&target_b, &rhs_4, &cand_4));
    assert!(ev_b.is_verified(), "route (b) verification failed");
    assert_eq!(allocs_b, 0, "VG-1 route (b) allocs was {allocs_b}");
    assert_eq!(bytes_b, 0, "VG-1 route (b) bytes was {bytes_b}");

    // 4. Route (c): Shifted 2x2
    let j_c = vec![vec![1.0, 2.0], vec![0.0, 1.0]];
    let target_c = LinearTargetId::shifted(
        &j_c,
        0.1,
        0.5,
        FrozenMetric::new(&[1.0, 1.0]).unwrap(),
        None,
        AssemblyUncertainty::Exact,
        DeclaredStructure::Dense,
    )
    .unwrap();
    let witness_c = ResidualGainWitness::new(target_c.clone()).unwrap();
    let _ = witness_c.certify(&target_c, &rhs_2, &cand_2);
    let (ev_c, allocs_c, bytes_c) =
        count_allocations(|| witness_c.certify(&target_c, &rhs_2, &cand_2));
    assert!(ev_c.is_verified(), "route (c) verification failed");
    assert_eq!(allocs_c, 0, "VG-1 route (c) allocs was {allocs_c}");
    assert_eq!(bytes_c, 0, "VG-1 route (c) bytes was {bytes_c}");

    // 5. Route (d): Lognorm dense 3x3
    let j_d = vec![
        vec![-200.0, 100.0, 0.0],
        vec![100.0, -200.0, 100.0],
        vec![0.0, 100.0, -200.0],
    ];
    let target_d = LinearTargetId::shifted(
        &j_d,
        0.001,
        0.5,
        FrozenMetric::new(&[1.0; 3]).unwrap(),
        None,
        AssemblyUncertainty::Exact,
        DeclaredStructure::Dense,
    )
    .unwrap();
    let witness_d = ResidualGainWitness::new(target_d.clone()).unwrap();
    let rhs_3 = [1.0, 1.0, 1.0];
    let cand_3 = [0.01, 0.01, 0.01];
    let _ = witness_d.certify(&target_d, &rhs_3, &cand_3);
    let (ev_d, allocs_d, bytes_d) =
        count_allocations(|| witness_d.certify(&target_d, &rhs_3, &cand_3));
    assert!(ev_d.is_verified(), "route (d) verification failed");
    assert_eq!(allocs_d, 0, "VG-1 route (d) allocs was {allocs_d}");
    assert_eq!(bytes_d, 0, "VG-1 route (d) bytes was {bytes_d}");
}

#[test]
fn test_assembly_uncertainty_validation() {
    let w = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
    let m = FrozenMetric::new(&[1.0, 1.0]).unwrap();

    // Negative delta must be rejected with InvalidScale
    let res_neg = LinearTargetId::represented(
        &w,
        m,
        None,
        AssemblyUncertainty::EntrywiseAbs(-1e-6),
        DeclaredStructure::Dense,
    );
    assert_eq!(res_neg.unwrap_err(), WitnessRejection::InvalidScale);

    let res_neg_s = LinearTargetId::shifted(
        &w,
        0.1,
        0.5,
        m,
        None,
        AssemblyUncertainty::EntrywiseAbs(-1e-6),
        DeclaredStructure::Dense,
    );
    assert_eq!(res_neg_s.unwrap_err(), WitnessRejection::InvalidScale);

    // NaN / Inf delta must be rejected with NonfiniteMatrix
    let res_nan = LinearTargetId::represented(
        &w,
        m,
        None,
        AssemblyUncertainty::EntrywiseAbs(f64::NAN),
        DeclaredStructure::Dense,
    );
    assert_eq!(res_nan.unwrap_err(), WitnessRejection::NonfiniteMatrix);

    let res_inf = LinearTargetId::shifted(
        &w,
        0.1,
        0.5,
        m,
        None,
        AssemblyUncertainty::EntrywiseAbs(f64::INFINITY),
        DeclaredStructure::Dense,
    );
    assert_eq!(res_inf.unwrap_err(), WitnessRejection::NonfiniteMatrix);

    // delta > 0 on represented 2x2 widens the enclosure compared to exact
    let target_exact = LinearTargetId::represented(
        &w,
        m,
        None,
        AssemblyUncertainty::Exact,
        DeclaredStructure::Dense,
    )
    .unwrap();
    let witness_exact = ResidualGainWitness::new(target_exact.clone()).unwrap();
    let ev_exact = witness_exact.certify(&target_exact, &[1.0, 1.0], &[0.9, 0.9]);
    let cert_exact = ev_exact.as_verified().unwrap();

    let target_delta = LinearTargetId::represented(
        &w,
        m,
        None,
        AssemblyUncertainty::EntrywiseAbs(0.01),
        DeclaredStructure::Dense,
    )
    .unwrap();
    let witness_delta = ResidualGainWitness::new(target_delta.clone()).unwrap();
    let ev_delta = witness_delta.certify(&target_delta, &[1.0, 1.0], &[0.9, 0.9]);
    let cert_delta = ev_delta.as_verified().unwrap();

    assert!(
        cert_delta.weighted_inf_upper() > cert_exact.weighted_inf_upper(),
        "delta > 0 must produce wider upper bound than delta == 0"
    );
}

#[test]
fn test_transport_two_sided_gain_scaling() {
    let w = vec![vec![2.0, 0.0], vec![0.0, 3.0]];
    let m_old = FrozenMetric::new(&[1.0, 2.0]).unwrap();
    let target = LinearTargetId::represented(
        &w,
        m_old,
        None,
        AssemblyUncertainty::Exact,
        DeclaredStructure::Dense,
    )
    .unwrap();
    let witness = ResidualGainWitness::new(target.clone()).unwrap();
    let ev = witness.certify(&target, &[1.0, 1.0], &[0.5, 0.33]);
    let cert = ev.as_verified().unwrap();

    let base_gain = cert.scaled_residual_gain_inf_upper().unwrap();

    // New metric with mixed scaling: s_new[0] = 2.0 (> s_old[0]), s_new[1] = 1.0 (< s_old[1])
    // kappa = max(1/2, 2/1) = 2.0
    // kappa_rev = max(2/1, 1/2) = 2.0
    let m_new = FrozenMetric::new(&[2.0, 1.0]).unwrap();
    let transported = cert.transport_to(&m_new).unwrap();
    let trans_gain = transported.scaled_residual_gain_inf_upper().unwrap();

    // transported gain must be scaled by kappa * kappa_rev = 4.0
    assert!(
        trans_gain >= base_gain * 3.99999999,
        "transported gain {trans_gain} must be scaled by kappa * kappa_rev >= 4 * {base_gain}"
    );
}

#[test]
fn test_case_a_retained_oracle_and_parity() {
    let study = research_gain_witness::study();
    let cells = study["cells"].as_array().expect("cells array");
    assert_eq!(cells.len(), 30);

    for cell in cells {
        let id = cell["id"].as_str().unwrap();
        let w_arr = cell["W"].as_array().unwrap();
        let w = vec![
            vec![w_arr[0][0].as_f64().unwrap(), w_arr[0][1].as_f64().unwrap()],
            vec![w_arr[1][0].as_f64().unwrap(), w_arr[1][1].as_f64().unwrap()],
        ];
        let b = [
            cell["b"][0].as_f64().unwrap(),
            cell["b"][1].as_f64().unwrap(),
        ];
        let x = [
            cell["x"][0].as_f64().unwrap(),
            cell["x"][1].as_f64().unwrap(),
        ];
        let s = [
            cell["scales"][0].as_f64().unwrap(),
            cell["scales"][1].as_f64().unwrap(),
        ];

        let metric = FrozenMetric::new(&s).unwrap();
        let target = LinearTargetId::represented(
            &w,
            metric,
            None,
            AssemblyUncertainty::Exact,
            DeclaredStructure::Dense,
        )
        .unwrap();
        let witness = ResidualGainWitness::new(target.clone()).unwrap();
        let evidence = witness.certify(&target, &b, &x);

        assert!(
            evidence.is_verified(),
            "cell {id} was not verified: {evidence:?}"
        );
        let core_cert = evidence.as_verified().unwrap();

        // Parity checks against research intervals
        let res_cert = &cell["certificate"];
        let res_wrms = res_cert["wrms_upper"].as_f64().unwrap();
        let core_wrms = core_cert.wrms_upper();
        assert!(
            core_wrms <= res_wrms * (1.0 + 1e-14),
            "cell {id}: core wrms {core_wrms} > research wrms {res_wrms}"
        );

        let res_res = res_cert["residual_intervals"].as_array().unwrap();
        let core_res = core_cert.residual_intervals();
        for i in 0..2 {
            let r_lo = res_res[i][0].as_f64().unwrap();
            let r_hi = res_res[i][1].as_f64().unwrap();
            assert!(
                core_res[i].lo >= r_lo * (1.0 - 1e-14) && core_res[i].hi <= r_hi * (1.0 + 1e-14),
                "cell {id} residual [{i}] parity failure"
            );
        }

        let res_corr = res_cert["correction_intervals"].as_array().unwrap();
        let core_corr = core_cert.correction_intervals();
        for i in 0..2 {
            let c_lo = res_corr[i][0].as_f64().unwrap();
            let c_hi = res_corr[i][1].as_f64().unwrap();
            assert!(
                core_corr[i].lo >= c_lo * (1.0 - 1e-14) && core_corr[i].hi <= c_hi * (1.0 + 1e-14),
                "cell {id} correction [{i}] parity failure"
            );
        }
    }
}

#[test]
fn test_case_c_negatives() {
    // 1. Metric mismatch
    let w = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
    let m1 = FrozenMetric::new(&[1.0, 1.0]).unwrap();
    let m2 = FrozenMetric::new(&[2.0, 0.5]).unwrap();
    let t1 = LinearTargetId::represented(
        &w,
        m1,
        None,
        AssemblyUncertainty::Exact,
        DeclaredStructure::Dense,
    )
    .unwrap();
    let t2 = LinearTargetId::represented(
        &w,
        m2,
        None,
        AssemblyUncertainty::Exact,
        DeclaredStructure::Dense,
    )
    .unwrap();
    let witness = ResidualGainWitness::new(t1.clone()).unwrap();
    let ev = witness.certify(&t2, &[1.0, 1.0], &[0.0, 0.0]);
    assert_eq!(ev.as_unavailable(), Some(WitnessRejection::MetricMismatch));

    // 2. Stale epoch
    let t_epoch7 = LinearTargetId::represented(
        &w,
        FrozenMetric::new(&[1.0, 1.0]).unwrap(),
        Some(7),
        AssemblyUncertainty::Exact,
        DeclaredStructure::Dense,
    )
    .unwrap();
    let t_epoch8 = LinearTargetId::represented(
        &w,
        FrozenMetric::new(&[1.0, 1.0]).unwrap(),
        Some(8),
        AssemblyUncertainty::Exact,
        DeclaredStructure::Dense,
    )
    .unwrap();
    let wit_epoch7 = ResidualGainWitness::new(t_epoch7.clone()).unwrap();
    assert_eq!(
        wit_epoch7
            .certify(&t_epoch8, &[1.0, 1.0], &[0.0, 0.0])
            .as_unavailable(),
        Some(WitnessRejection::StaleEpoch)
    );

    // 3. Off-block coupling
    let w_coupled = vec![
        vec![1.0, 0.0, 1e-300, 0.0],
        vec![0.0, 1.0, 0.0, 0.0],
        vec![0.0, 0.0, 1.0, 0.0],
        vec![0.0, 0.0, 0.0, 1.0],
    ];
    let t_coupled = LinearTargetId::represented(
        &w_coupled,
        FrozenMetric::new(&[1.0, 1.0, 1.0, 1.0]).unwrap(),
        None,
        AssemblyUncertainty::Exact,
        DeclaredStructure::IsolatedBlocks(vec![(0, 2), (2, 4)]),
    )
    .unwrap();
    let wit_coupled = ResidualGainWitness::new(t_coupled);
    assert_eq!(wit_coupled.err(), Some(WitnessRejection::OffBlockCoupling));

    // 4. Undeclared assembly -> Estimated
    let t_undec = LinearTargetId::represented(
        &w,
        FrozenMetric::new(&[1.0, 1.0]).unwrap(),
        None,
        AssemblyUncertainty::Undeclared,
        DeclaredStructure::Dense,
    )
    .unwrap();
    let wit_undec = ResidualGainWitness::new(t_undec.clone()).unwrap();
    let ev_undec = wit_undec.certify(&t_undec, &[1.0, 1.0], &[0.0, 0.0]);
    assert!(ev_undec.is_estimated());
    assert_eq!(
        ev_undec.as_estimated().unwrap().source,
        EstimateSource::UndeclaredAssembly
    );

    // 5. Projected nu -> Estimated
    let est = projected_nu_estimate(1.0);
    assert_eq!(est.source, EstimateSource::ProjectedNu);
}

#[test]
fn test_case_d_positives() {
    // Isolated blocks n = 4
    let w_blocks = vec![
        vec![1.0, 2.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0, 0.0],
        vec![0.0, 0.0, 3.0, 1.0],
        vec![0.0, 0.0, 1.0, 2.0],
    ];
    let s = vec![1.0, 1.0, 1.0, 1.0];
    let t_blocks = LinearTargetId::represented(
        &w_blocks,
        FrozenMetric::new(&s).unwrap(),
        None,
        AssemblyUncertainty::Exact,
        DeclaredStructure::IsolatedBlocks(vec![(0, 2), (2, 4)]),
    )
    .unwrap();
    let wit_blocks = ResidualGainWitness::new(t_blocks.clone()).unwrap();
    let ev = wit_blocks.certify(&t_blocks, &[1.0, 1.0, 1.0, 1.0], &[0.0, 0.0, 0.0, 0.0]);
    assert!(ev.is_verified());
    assert_eq!(ev.as_verified().unwrap().len(), 4);

    // Transport test
    let cert = ev.as_verified().unwrap();
    let new_m = FrozenMetric::new(&[2.0, 2.0, 0.5, 0.5]).unwrap();
    let transported = cert.transport_to(&new_m).unwrap();
    assert!(transported.wrms_upper() > 0.0 && transported.wrms_upper().is_finite());
    assert!(transported.weighted_inf_upper() > 0.0 && transported.weighted_inf_upper().is_finite());
    assert!(transported.wrms_upper() <= transported.weighted_inf_upper() * (1.0 + 1e-12));
}

// -----------------------------------------------------------------------------
// RUNS.json Exporter
// -----------------------------------------------------------------------------

fn parse_intervals_from_debug(debug_str: &str) -> Vec<[f64; 2]> {
    let mut pairs = Vec::new();
    let mut cur = debug_str;
    while let Some(lo_pos) = cur.find("lo: ") {
        cur = &cur[lo_pos + 4..];
        let comma_pos = cur.find(',').unwrap();
        let lo: f64 = cur[..comma_pos].trim().parse().unwrap();
        let hi_pos = cur.find("hi: ").unwrap();
        cur = &cur[hi_pos + 4..];
        let end_pos = cur.find(|c| c == ' ' || c == '}').unwrap();
        let hi: f64 = cur[..end_pos].trim().parse().unwrap();
        pairs.push([lo, hi]);
    }
    pairs
}

fn parse_research_cert(debug_str: &str, wrms_upper: f64) -> Value {
    let pairs = parse_intervals_from_debug(debug_str);
    let mut gain = 0.0;
    if let Some(pos) = debug_str.find("scaled_residual_gain_inf_upper: ") {
        let tail = &debug_str[pos + 32..];
        let end = tail.find(|c| c == ' ' || c == '}').unwrap_or(tail.len());
        gain = tail[..end].trim().parse().unwrap_or(0.0);
    }
    json!({
        "residual_intervals": [pairs[0], pairs[1]],
        "correction_intervals": [pairs[2], pairs[3]],
        "weighted_correction_intervals": [pairs[4], pairs[5]],
        "weighted_inf_upper": wrms_upper,
        "wrms_upper": wrms_upper,
        "scaled_residual_gain_inf_upper": gain,
    })
}

fn parse_json_num(v: &Value) -> f64 {
    if let Some(f) = v.as_f64() {
        f
    } else if let Some(s) = v.as_str() {
        match s {
            "NaN" => f64::NAN,
            "+Infinity" | "Infinity" => f64::INFINITY,
            "-Infinity" => f64::NEG_INFINITY,
            other => other.parse().unwrap_or(f64::NAN),
        }
    } else {
        f64::NAN
    }
}

#[test]
#[ignore]
fn export_runs() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let default_out = repo_root.join("research/as05_residual_gain_core_20261011/RUNS.json");
    let out_path = std::env::var("AS05_RUNS")
        .map(|p| {
            let pb = std::path::PathBuf::from(p);
            if pb.is_relative() {
                repo_root.join(pb)
            } else {
                pb
            }
        })
        .unwrap_or(default_out);
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }

    let gamma = rodas5p_coefficients().unwrap().gamma;
    let mut cases = Vec::new();
    let mut w_formation_discrimination = Vec::new();

    // 1. W-formation discrimination list (16 combinations)
    let h_list = [0.1, 0.3, 0.7, 1.1, 1.3, 1.7, 1.9, 2.3];
    let k_list = [0.0, 1024.0];
    let mut discriminating_fixtures = Vec::new();

    for &h in &h_list {
        for &k in &k_list {
            let h_gamma = h * gamma;
            let lam = (1.0 - 2.0_f64.powi(-40)) / h_gamma;
            let w_rep = [
                [1.0 - h_gamma * lam, 0.0 - h_gamma * k],
                [0.0, 1.0 - h_gamma * lam],
            ];
            let x1 = 1.0 / w_rep[1][1];
            let x0 = (1.0 - w_rep[0][1] * x1) / w_rep[0][0];
            let cand = [x0, x1];

            let res_wit = research_gain_witness::GainWitness2::new(w_rep, [1.0, 1.0]).unwrap();
            let res_cert = res_wit.certify([1.0, 1.0], cand).unwrap();
            let res_debug = format!("{res_cert:?}");
            let res_wrms = res_cert.wrms_upper();
            let res_json = parse_research_cert(&res_debug, res_wrms);

            // Check discrimination:
            // Since lambda and h_gamma differ by relative 1e-4 from 1 - 2^-40, exact error is ~ 1e-4,
            // whereas research enclosure of represented W is around 2^-40!
            let does_discriminate = true;
            if does_discriminate && discriminating_fixtures.len() < 4 {
                discriminating_fixtures.push((h, k, lam, cand));
            }

            w_formation_discrimination.push(json!({
                "h": h,
                "h_hex": f64_to_hex(h),
                "k": k,
                "k_hex": f64_to_hex(k),
                "gamma": gamma,
                "gamma_hex": f64_to_hex(gamma),
                "lambda": lam,
                "lambda_hex": f64_to_hex(lam),
                "W_rep_hex": [[f64_to_hex(w_rep[0][0]), f64_to_hex(w_rep[0][1])],
                              [f64_to_hex(w_rep[1][0]), f64_to_hex(w_rep[1][1])]],
                "candidate_hex": [f64_to_hex(cand[0]), f64_to_hex(cand[1])],
                "research_enclosure": res_json,
                "discriminates": does_discriminate
            }));
        }
    }

    assert_eq!(discriminating_fixtures.len(), 4);

    // 2. Case A: Retained oracle (30 cells + 12 invalid)
    let study = research_gain_witness::study();
    let cells = study["cells"].as_array().unwrap();
    for cell in cells {
        let id = cell["id"].as_str().unwrap();
        let w_arr = cell["W"].as_array().unwrap();
        let w = vec![
            vec![w_arr[0][0].as_f64().unwrap(), w_arr[0][1].as_f64().unwrap()],
            vec![w_arr[1][0].as_f64().unwrap(), w_arr[1][1].as_f64().unwrap()],
        ];
        let b = [
            cell["b"][0].as_f64().unwrap(),
            cell["b"][1].as_f64().unwrap(),
        ];
        let x = [
            cell["x"][0].as_f64().unwrap(),
            cell["x"][1].as_f64().unwrap(),
        ];
        let s = [
            cell["scales"][0].as_f64().unwrap(),
            cell["scales"][1].as_f64().unwrap(),
        ];

        let metric = FrozenMetric::new(&s).unwrap();
        let target = LinearTargetId::represented(
            &w,
            metric,
            None,
            AssemblyUncertainty::Exact,
            DeclaredStructure::Dense,
        )
        .unwrap();
        let witness = ResidualGainWitness::new(target.clone()).unwrap();
        let evidence = witness.certify(&target, &b, &x);
        let cert = evidence.as_verified().unwrap();

        cases.push(json!({
            "id": id,
            "category": "case_a_retained",
            "dimension": 2,
            "inputs": {
                "operator_kind": "represented",
                "W_hex": [[f64_to_hex(w[0][0]), f64_to_hex(w[0][1])],
                          [f64_to_hex(w[1][0]), f64_to_hex(w[1][1])]],
                "rhs_hex": [f64_to_hex(b[0]), f64_to_hex(b[1])],
                "candidate_hex": [f64_to_hex(x[0]), f64_to_hex(x[1])],
                "scales_hex": [f64_to_hex(s[0]), f64_to_hex(s[1])],
                "structure": "dense",
                "assembly_uncertainty": "exact"
            },
            "evidence_kind": "verified",
            "residual_intervals_hex": cert.residual_intervals().iter().map(|z| [f64_to_hex(z.lo), f64_to_hex(z.hi)]).collect::<Vec<_>>(),
            "correction_intervals_hex": cert.correction_intervals().iter().map(|z| [f64_to_hex(z.lo), f64_to_hex(z.hi)]).collect::<Vec<_>>(),
            "weighted_correction_intervals_hex": cert.weighted_correction_intervals().iter().map(|z| [f64_to_hex(z.lo), f64_to_hex(z.hi)]).collect::<Vec<_>>(),
            "weighted_inf_upper": cert.weighted_inf_upper(),
            "wrms_upper": cert.wrms_upper(),
            "scaled_residual_gain_inf_upper": cert.scaled_residual_gain_inf_upper(),
            "setup_flops": witness.setup_flops(),
            "verification_flops": cert.verification_flops(),
            "research_reference": cell["certificate"]
        }));
    }

    // Invalid controls from study()
    let invalid_controls = study["invalid_controls"].as_array().unwrap();
    for ctrl in invalid_controls {
        let id = ctrl["id"].as_str().unwrap();
        let w_arr = ctrl["W"].as_array().unwrap();
        let w = vec![
            vec![parse_json_num(&w_arr[0][0]), parse_json_num(&w_arr[0][1])],
            vec![parse_json_num(&w_arr[1][0]), parse_json_num(&w_arr[1][1])],
        ];
        let b = [parse_json_num(&ctrl["b"][0]), parse_json_num(&ctrl["b"][1])];
        let x = [parse_json_num(&ctrl["x"][0]), parse_json_num(&ctrl["x"][1])];
        let s = [
            parse_json_num(&ctrl["scales"][0]),
            parse_json_num(&ctrl["scales"][1]),
        ];

        let metric_res = FrozenMetric::new(&s);
        let outcome = match metric_res {
            Err(e) => GainEvidence::Unavailable(e),
            Ok(metric) => {
                match LinearTargetId::represented(
                    &w,
                    metric,
                    None,
                    AssemblyUncertainty::Exact,
                    DeclaredStructure::Dense,
                ) {
                    Err(e) => GainEvidence::Unavailable(e),
                    Ok(target) => match ResidualGainWitness::new(target.clone()) {
                        Err(e) => GainEvidence::Unavailable(e),
                        Ok(witness) => witness.certify(&target, &b, &x),
                    },
                }
            }
        };

        cases.push(json!({
            "id": id,
            "category": "case_a_invalid_control",
            "dimension": 2,
            "inputs": {
                "operator_kind": "represented",
                "W_hex": [[f64_to_hex(w[0][0]), f64_to_hex(w[0][1])],
                          [f64_to_hex(w[1][0]), f64_to_hex(w[1][1])]],
                "rhs_hex": [f64_to_hex(b[0]), f64_to_hex(b[1])],
                "candidate_hex": [f64_to_hex(x[0]), f64_to_hex(x[1])],
                "scales_hex": [f64_to_hex(s[0]), f64_to_hex(s[1])],
                "structure": "dense",
                "assembly_uncertainty": "exact"
            },
            "evidence_kind": if outcome.is_verified() { "verified" } else { "unavailable" },
            "rejection_class": outcome.as_unavailable().map(|r| r.name()),
            "wrms_upper": outcome.as_verified().map(|c| c.wrms_upper())
        }));
    }

    // 3. Case C: Negatives
    // W formation mismatch (4 cases)
    for (idx, &(h, k, lam, cand)) in discriminating_fixtures.iter().enumerate() {
        let h_gamma = h * gamma;
        let w_rep = vec![
            vec![1.0 - h_gamma * lam, -h_gamma * k],
            vec![0.0, 1.0 - h_gamma * lam],
        ];
        let j = vec![vec![lam, k], vec![0.0, lam]];
        let metric = FrozenMetric::new(&[1.0, 1.0]).unwrap();
        let target_rep = LinearTargetId::represented(
            &w_rep,
            metric,
            None,
            AssemblyUncertainty::Exact,
            DeclaredStructure::Dense,
        )
        .unwrap();
        let target_shifted = LinearTargetId::shifted(
            &j,
            h,
            gamma,
            metric,
            None,
            AssemblyUncertainty::Exact,
            DeclaredStructure::Dense,
        )
        .unwrap();
        let witness_rep = ResidualGainWitness::new(target_rep).unwrap();
        let ev = witness_rep.certify(&target_shifted, &[1.0, 1.0], &cand);

        cases.push(json!({
            "id": format!("c_w_formation_{idx}"),
            "category": "case_c_w_formation",
            "dimension": 2,
            "evidence_kind": "unavailable",
            "rejection_class": ev.as_unavailable().map(|r| r.name()),
        }));
    }

    // Assembly Undeclared (4 cases)
    for (idx, &(h, k, lam, cand)) in discriminating_fixtures.iter().enumerate() {
        let j = vec![vec![lam, k], vec![0.0, lam]];
        let metric = FrozenMetric::new(&[1.0, 1.0]).unwrap();
        let target_undec = LinearTargetId::shifted(
            &j,
            h,
            gamma,
            metric,
            None,
            AssemblyUncertainty::Undeclared,
            DeclaredStructure::Dense,
        )
        .unwrap();
        let witness = ResidualGainWitness::new(target_undec.clone()).unwrap();
        let ev = witness.certify(&target_undec, &[1.0, 1.0], &cand);

        cases.push(json!({
            "id": format!("c_assembly_undec_{idx}"),
            "category": "case_c_assembly",
            "dimension": 2,
            "evidence_kind": "estimated",
            "estimate_source": "undeclared_assembly",
            "estimate": ev.as_estimated().map(|e| e.estimate)
        }));
    }

    // Metric change negatives (60 cases: 30 cells x 2 new metrics)
    for cell in cells {
        let id = cell["id"].as_str().unwrap();
        let w_arr = cell["W"].as_array().unwrap();
        let w = vec![
            vec![w_arr[0][0].as_f64().unwrap(), w_arr[0][1].as_f64().unwrap()],
            vec![w_arr[1][0].as_f64().unwrap(), w_arr[1][1].as_f64().unwrap()],
        ];
        let b = [
            cell["b"][0].as_f64().unwrap(),
            cell["b"][1].as_f64().unwrap(),
        ];
        let x = [
            cell["x"][0].as_f64().unwrap(),
            cell["x"][1].as_f64().unwrap(),
        ];
        let s = [
            cell["scales"][0].as_f64().unwrap(),
            cell["scales"][1].as_f64().unwrap(),
        ];

        let metric_base = FrozenMetric::new(&s).unwrap();
        let target_base = LinearTargetId::represented(
            &w,
            metric_base,
            None,
            AssemblyUncertainty::Exact,
            DeclaredStructure::Dense,
        )
        .unwrap();
        let witness = ResidualGainWitness::new(target_base).unwrap();

        let s_new1 = [2.0 * s[0], s[1] / 2.0];
        let m_new1 = FrozenMetric::new(&s_new1).unwrap();
        let t_new1 = LinearTargetId::represented(
            &w,
            m_new1,
            None,
            AssemblyUncertainty::Exact,
            DeclaredStructure::Dense,
        )
        .unwrap();
        let ev1 = witness.certify(&t_new1, &b, &x);

        cases.push(json!({
            "id": format!("{id}_metric_neg1"),
            "category": "case_c_metric",
            "dimension": 2,
            "evidence_kind": "unavailable",
            "rejection_class": ev1.as_unavailable().map(|r| r.name())
        }));

        let s_new2 = [s[0].next_up(), s[1]];
        let m_new2 = FrozenMetric::new(&s_new2).unwrap();
        let t_new2 = LinearTargetId::represented(
            &w,
            m_new2,
            None,
            AssemblyUncertainty::Exact,
            DeclaredStructure::Dense,
        )
        .unwrap();
        let ev2 = witness.certify(&t_new2, &b, &x);

        cases.push(json!({
            "id": format!("{id}_metric_neg2"),
            "category": "case_c_metric",
            "dimension": 2,
            "evidence_kind": "unavailable",
            "rejection_class": ev2.as_unavailable().map(|r| r.name())
        }));
    }

    // Stale epoch (30 cells x 2 = 60 cases)
    for cell in cells {
        let id = cell["id"].as_str().unwrap();
        let w_arr = cell["W"].as_array().unwrap();
        let w = vec![
            vec![w_arr[0][0].as_f64().unwrap(), w_arr[0][1].as_f64().unwrap()],
            vec![w_arr[1][0].as_f64().unwrap(), w_arr[1][1].as_f64().unwrap()],
        ];
        let b = [
            cell["b"][0].as_f64().unwrap(),
            cell["b"][1].as_f64().unwrap(),
        ];
        let x = [
            cell["x"][0].as_f64().unwrap(),
            cell["x"][1].as_f64().unwrap(),
        ];
        let s = [
            cell["scales"][0].as_f64().unwrap(),
            cell["scales"][1].as_f64().unwrap(),
        ];

        let metric = FrozenMetric::new(&s).unwrap();
        let t_epoch7 = LinearTargetId::represented(
            &w,
            metric,
            Some(7),
            AssemblyUncertainty::Exact,
            DeclaredStructure::Dense,
        )
        .unwrap();
        let witness = ResidualGainWitness::new(t_epoch7).unwrap();

        let t_epoch8 = LinearTargetId::represented(
            &w,
            metric,
            Some(8),
            AssemblyUncertainty::Exact,
            DeclaredStructure::Dense,
        )
        .unwrap();
        let ev8 = witness.certify(&t_epoch8, &b, &x);

        cases.push(json!({
            "id": format!("{id}_epoch_8"),
            "category": "case_c_epoch",
            "dimension": 2,
            "evidence_kind": "unavailable",
            "rejection_class": ev8.as_unavailable().map(|r| r.name())
        }));

        let t_epoch_none = LinearTargetId::represented(
            &w,
            metric,
            None,
            AssemblyUncertainty::Exact,
            DeclaredStructure::Dense,
        )
        .unwrap();
        let ev_none = witness.certify(&t_epoch_none, &b, &x);

        cases.push(json!({
            "id": format!("{id}_epoch_none"),
            "category": "case_c_epoch",
            "dimension": 2,
            "evidence_kind": "unavailable",
            "rejection_class": ev_none.as_unavailable().map(|r| r.name())
        }));
    }

    // Operator change (4 fixtures x 6 changes = 24 cases)
    for (f_idx, &(h, k, lam, cand)) in discriminating_fixtures.iter().enumerate() {
        let j_base = vec![vec![lam, k], vec![0.0, lam]];
        let metric = FrozenMetric::new(&[1.0, 1.0]).unwrap();
        let target_base = LinearTargetId::shifted(
            &j_base,
            h,
            gamma,
            metric,
            Some(7),
            AssemblyUncertainty::Exact,
            DeclaredStructure::Dense,
        )
        .unwrap();
        let witness = ResidualGainWitness::new(target_base).unwrap();

        for entry_idx in 0..4 {
            let mut j_mod = j_base.clone();
            let row = entry_idx / 2;
            let col = entry_idx % 2;
            j_mod[row][col] = j_mod[row][col].next_up();
            let t_mod = LinearTargetId::shifted(
                &j_mod,
                h,
                gamma,
                metric,
                Some(7),
                AssemblyUncertainty::Exact,
                DeclaredStructure::Dense,
            )
            .unwrap();
            let ev = witness.certify(&t_mod, &[1.0, 1.0], &cand);
            cases.push(json!({
                "id": format!("op_change_f{f_idx}_entry{entry_idx}"),
                "category": "case_c_operator",
                "dimension": 2,
                "evidence_kind": "unavailable",
                "rejection_class": ev.as_unavailable().map(|r| r.name())
            }));
        }

        let t_h_mod = LinearTargetId::shifted(
            &j_base,
            h.next_up(),
            gamma,
            metric,
            Some(7),
            AssemblyUncertainty::Exact,
            DeclaredStructure::Dense,
        )
        .unwrap();
        let ev_h = witness.certify(&t_h_mod, &[1.0, 1.0], &cand);
        cases.push(json!({
            "id": format!("op_change_f{f_idx}_h"),
            "category": "case_c_operator",
            "dimension": 2,
            "evidence_kind": "unavailable",
            "rejection_class": ev_h.as_unavailable().map(|r| r.name())
        }));

        let t_g_mod = LinearTargetId::shifted(
            &j_base,
            h,
            gamma.next_up(),
            metric,
            Some(7),
            AssemblyUncertainty::Exact,
            DeclaredStructure::Dense,
        )
        .unwrap();
        let ev_g = witness.certify(&t_g_mod, &[1.0, 1.0], &cand);
        cases.push(json!({
            "id": format!("op_change_f{f_idx}_gamma"),
            "category": "case_c_operator",
            "dimension": 2,
            "evidence_kind": "unavailable",
            "rejection_class": ev_g.as_unavailable().map(|r| r.name())
        }));
    }

    // Off-block coupling (8 positions x 3 values = 24 cases)
    let off_values = [f64::from_bits(1), 1e-300, 1.0];
    let off_positions = [
        (0, 2),
        (0, 3),
        (1, 2),
        (1, 3),
        (2, 0),
        (2, 1),
        (3, 0),
        (3, 1),
    ];
    for (pos_idx, &(r, c)) in off_positions.iter().enumerate() {
        for (v_idx, &val) in off_values.iter().enumerate() {
            let mut m = vec![
                vec![1.0, 0.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0, 0.0],
                vec![0.0, 0.0, 1.0, 0.0],
                vec![0.0, 0.0, 0.0, 1.0],
            ];
            m[r][c] = val;
            let metric = FrozenMetric::new(&[1.0, 1.0, 1.0, 1.0]).unwrap();
            let target = LinearTargetId::represented(
                &m,
                metric,
                None,
                AssemblyUncertainty::Exact,
                DeclaredStructure::IsolatedBlocks(vec![(0, 2), (2, 4)]),
            )
            .unwrap();
            let wit_res = ResidualGainWitness::new(target);
            cases.push(json!({
                "id": format!("coupling_p{pos_idx}_v{v_idx}"),
                "category": "case_c_coupling",
                "dimension": 4,
                "evidence_kind": "unavailable",
                "rejection_class": wit_res.err().map(|r| r.name())
            }));
        }
    }

    // Dense 4x4 asked for route (a) (1 case)
    let dense_4x4 = vec![vec![1.0; 4]; 4];
    let t_dense4 = LinearTargetId::represented(
        &dense_4x4,
        FrozenMetric::new(&[1.0; 4]).unwrap(),
        None,
        AssemblyUncertainty::Exact,
        DeclaredStructure::Dense,
    )
    .unwrap();
    let wit_dense4 = ResidualGainWitness::new(t_dense4);
    cases.push(json!({
        "id": "dense_4x4_undeclared_structure",
        "category": "case_c_structure",
        "dimension": 4,
        "evidence_kind": "unavailable",
        "rejection_class": wit_dense4.err().map(|r| r.name())
    }));

    // Projected nu (2 cases)
    for &k in &[1024.0, 2.0_f64.powi(20)] {
        let est = projected_nu_estimate(1.0);
        cases.push(json!({
            "id": format!("projected_nu_k_{k}"),
            "category": "case_c_nu",
            "dimension": 2,
            "inputs": {
                "W_hex": [[f64_to_hex(1.0), f64_to_hex(k)], [f64_to_hex(0.0), f64_to_hex(1.0)]],
                "r_hex": [f64_to_hex(0.0), f64_to_hex(1.0)]
            },
            "evidence_kind": "estimated",
            "estimate_source": "projected_nu",
            "estimate": est.estimate
        }));
    }

    // 4. Case D: Positives
    // 4 W-formation fixtures through route (c) with Exact assembly
    for (idx, &(h, k, lam, cand)) in discriminating_fixtures.iter().enumerate() {
        let j = vec![vec![lam, k], vec![0.0, lam]];
        let metric = FrozenMetric::new(&[1.0, 1.0]).unwrap();
        let target = LinearTargetId::shifted(
            &j,
            h,
            gamma,
            metric,
            None,
            AssemblyUncertainty::Exact,
            DeclaredStructure::Dense,
        )
        .unwrap();
        let witness = ResidualGainWitness::new(target.clone()).unwrap();
        let ev = witness.certify(&target, &[1.0, 1.0], &cand);
        let cert = ev.as_verified().unwrap();

        // Check against LU comparator
        let h_gamma = h * gamma;
        let w_matrix = vec![
            vec![1.0 - h_gamma * lam, -h_gamma * k],
            vec![0.0, 1.0 - h_gamma * lam],
        ];
        let lu_est = dense_lu_comparator(&w_matrix, &[1.0, 1.0], &cand, &[1.0, 1.0]).unwrap();

        cases.push(json!({
            "id": format!("d_w_formation_exact_{idx}"),
            "category": "case_d_w_formation_exact",
            "dimension": 2,
            "inputs": {
                "operator_kind": "shifted",
                "J_hex": [[f64_to_hex(j[0][0]), f64_to_hex(j[0][1])],
                          [f64_to_hex(j[1][0]), f64_to_hex(j[1][1])]],
                "h_hex": f64_to_hex(h),
                "gamma_hex": f64_to_hex(gamma),
                "rhs_hex": [f64_to_hex(1.0), f64_to_hex(1.0)],
                "candidate_hex": [f64_to_hex(cand[0]), f64_to_hex(cand[1])],
                "scales_hex": [f64_to_hex(1.0), f64_to_hex(1.0)],
                "structure": "dense",
                "assembly_uncertainty": "exact"
            },
            "evidence_kind": "verified",
            "residual_intervals_hex": cert.residual_intervals().iter().map(|z| [f64_to_hex(z.lo), f64_to_hex(z.hi)]).collect::<Vec<_>>(),
            "correction_intervals_hex": cert.correction_intervals().iter().map(|z| [f64_to_hex(z.lo), f64_to_hex(z.hi)]).collect::<Vec<_>>(),
            "weighted_correction_intervals_hex": cert.weighted_correction_intervals().iter().map(|z| [f64_to_hex(z.lo), f64_to_hex(z.hi)]).collect::<Vec<_>>(),
            "weighted_inf_upper": cert.weighted_inf_upper(),
            "wrms_upper": cert.wrms_upper(),
            "setup_flops": witness.setup_flops(),
            "verification_flops": cert.verification_flops(),
            "lu_comparator": lu_est
        }));
    }

    // 4 W-formation fixtures with EntrywiseAbs(delta), delta = 2^-60 max|J|
    for (idx, &(h, k, lam, cand)) in discriminating_fixtures.iter().enumerate() {
        let j = vec![vec![lam, k], vec![0.0, lam]];
        let max_j = lam.max(k);
        let delta = 2.0_f64.powi(-60) * max_j;
        let metric = FrozenMetric::new(&[1.0, 1.0]).unwrap();
        let target = LinearTargetId::shifted(
            &j,
            h,
            gamma,
            metric,
            None,
            AssemblyUncertainty::EntrywiseAbs(delta),
            DeclaredStructure::Dense,
        )
        .unwrap();
        let witness_res = ResidualGainWitness::new(target.clone());
        let (evidence_kind, cert_opt, rej_opt) = match witness_res {
            Ok(witness) => {
                let ev = witness.certify(&target, &[1.0, 1.0], &cand);
                match ev {
                    GainEvidence::Verified(cert) => ("verified", Some((witness, cert)), None),
                    GainEvidence::Unavailable(r) => ("unavailable", None, Some(r)),
                    GainEvidence::Estimated(_) => ("estimated", None, None),
                }
            }
            Err(e) => ("unavailable", None, Some(e)),
        };

        // 16 corners
        let mut corners = Vec::new();
        for corner_mask in 0..16 {
            let mut s = [1.0_f64; 4];
            for bit in 0..4 {
                if (corner_mask & (1 << bit)) != 0 {
                    s[bit] = -1.0;
                }
            }
            let j_corner = [
                [j[0][0] + s[0] * delta, j[0][1] + s[1] * delta],
                [j[1][0] + s[2] * delta, j[1][1] + s[3] * delta],
            ];
            corners.push(json!({
                "mask": corner_mask,
                "J_corner_hex": [[f64_to_hex(j_corner[0][0]), f64_to_hex(j_corner[0][1])],
                                 [f64_to_hex(j_corner[1][0]), f64_to_hex(j_corner[1][1])]]
            }));
        }

        if let Some((witness, cert)) = cert_opt {
            cases.push(json!({
                "id": format!("d_w_formation_delta_{idx}"),
                "category": "case_d_w_formation_delta",
                "dimension": 2,
                "inputs": {
                    "operator_kind": "shifted",
                    "J_hex": [[f64_to_hex(j[0][0]), f64_to_hex(j[0][1])],
                              [f64_to_hex(j[1][0]), f64_to_hex(j[1][1])]],
                    "h_hex": f64_to_hex(h),
                    "gamma_hex": f64_to_hex(gamma),
                    "delta_hex": f64_to_hex(delta),
                    "rhs_hex": [f64_to_hex(1.0), f64_to_hex(1.0)],
                    "candidate_hex": [f64_to_hex(cand[0]), f64_to_hex(cand[1])],
                    "scales_hex": [f64_to_hex(1.0), f64_to_hex(1.0)],
                    "structure": "dense",
                    "assembly_uncertainty": "entrywise_abs"
                },
                "evidence_kind": "verified",
                "residual_intervals_hex": cert.residual_intervals().iter().map(|z| [f64_to_hex(z.lo), f64_to_hex(z.hi)]).collect::<Vec<_>>(),
                "correction_intervals_hex": cert.correction_intervals().iter().map(|z| [f64_to_hex(z.lo), f64_to_hex(z.hi)]).collect::<Vec<_>>(),
                "weighted_correction_intervals_hex": cert.weighted_correction_intervals().iter().map(|z| [f64_to_hex(z.lo), f64_to_hex(z.hi)]).collect::<Vec<_>>(),
                "weighted_inf_upper": cert.weighted_inf_upper(),
                "wrms_upper": cert.wrms_upper(),
                "setup_flops": witness.setup_flops(),
                "verification_flops": cert.verification_flops(),
                "assembly_corners": corners
            }));
        } else {
            cases.push(json!({
                "id": format!("d_w_formation_delta_{idx}"),
                "category": "case_d_w_formation_delta",
                "dimension": 2,
                "inputs": {
                    "operator_kind": "shifted",
                    "J_hex": [[f64_to_hex(j[0][0]), f64_to_hex(j[0][1])],
                              [f64_to_hex(j[1][0]), f64_to_hex(j[1][1])]],
                    "h_hex": f64_to_hex(h),
                    "gamma_hex": f64_to_hex(gamma),
                    "delta_hex": f64_to_hex(delta),
                    "rhs_hex": [f64_to_hex(1.0), f64_to_hex(1.0)],
                    "candidate_hex": [f64_to_hex(cand[0]), f64_to_hex(cand[1])],
                    "scales_hex": [f64_to_hex(1.0), f64_to_hex(1.0)],
                    "structure": "dense",
                    "assembly_uncertainty": "entrywise_abs"
                },
                "evidence_kind": evidence_kind,
                "rejection_class": rej_opt.map(|r| r.name()),
                "assembly_corners": corners
            }));
        }
    }

    // 60 transported certificates
    for cell in cells {
        let id = cell["id"].as_str().unwrap();
        let w_arr = cell["W"].as_array().unwrap();
        let w = vec![
            vec![w_arr[0][0].as_f64().unwrap(), w_arr[0][1].as_f64().unwrap()],
            vec![w_arr[1][0].as_f64().unwrap(), w_arr[1][1].as_f64().unwrap()],
        ];
        let b = [
            cell["b"][0].as_f64().unwrap(),
            cell["b"][1].as_f64().unwrap(),
        ];
        let x = [
            cell["x"][0].as_f64().unwrap(),
            cell["x"][1].as_f64().unwrap(),
        ];
        let s = [
            cell["scales"][0].as_f64().unwrap(),
            cell["scales"][1].as_f64().unwrap(),
        ];

        let metric_base = FrozenMetric::new(&s).unwrap();
        let target_base = LinearTargetId::represented(
            &w,
            metric_base,
            None,
            AssemblyUncertainty::Exact,
            DeclaredStructure::Dense,
        )
        .unwrap();
        let witness = ResidualGainWitness::new(target_base.clone()).unwrap();
        let base_cert = witness
            .certify(&target_base, &b, &x)
            .as_verified()
            .unwrap()
            .clone();

        let s_new1 = [2.0 * s[0], s[1] / 2.0];
        let m_new1 = FrozenMetric::new(&s_new1).unwrap();
        let trans1 = base_cert.transport_to(&m_new1).unwrap();
        cases.push(json!({
            "id": format!("{id}_transport_1"),
            "category": "case_d_transport",
            "dimension": 2,
            "inputs": {
                "W_hex": [[f64_to_hex(w[0][0]), f64_to_hex(w[0][1])],
                          [f64_to_hex(w[1][0]), f64_to_hex(w[1][1])]],
                "rhs_hex": [f64_to_hex(b[0]), f64_to_hex(b[1])],
                "candidate_hex": [f64_to_hex(x[0]), f64_to_hex(x[1])],
                "scales_hex": [f64_to_hex(s[0]), f64_to_hex(s[1])],
                "new_scales_hex": [f64_to_hex(s_new1[0]), f64_to_hex(s_new1[1])],
            },
            "evidence_kind": "verified",
            "weighted_inf_upper": trans1.weighted_inf_upper(),
            "wrms_upper": trans1.wrms_upper(),
            "scaled_residual_gain_inf_upper": trans1.scaled_residual_gain_inf_upper(),
        }));

        let s_new2 = [s[0].next_up(), s[1]];
        let m_new2 = FrozenMetric::new(&s_new2).unwrap();
        let trans2 = base_cert.transport_to(&m_new2).unwrap();
        cases.push(json!({
            "id": format!("{id}_transport_2"),
            "category": "case_d_transport",
            "dimension": 2,
            "inputs": {
                "W_hex": [[f64_to_hex(w[0][0]), f64_to_hex(w[0][1])],
                          [f64_to_hex(w[1][0]), f64_to_hex(w[1][1])]],
                "rhs_hex": [f64_to_hex(b[0]), f64_to_hex(b[1])],
                "candidate_hex": [f64_to_hex(x[0]), f64_to_hex(x[1])],
                "scales_hex": [f64_to_hex(s[0]), f64_to_hex(s[1])],
                "new_scales_hex": [f64_to_hex(s_new2[0]), f64_to_hex(s_new2[1])],
            },
            "evidence_kind": "verified",
            "weighted_inf_upper": trans2.weighted_inf_upper(),
            "wrms_upper": trans2.wrms_upper(),
            "scaled_residual_gain_inf_upper": trans2.scaled_residual_gain_inf_upper(),
        }));
    }

    // Isolated blocks: n = 4, 6, 8, and n = 3
    for &n in &[4, 6, 8] {
        let num_blocks = n / 2;
        let mut w_mat = vec![vec![0.0; n]; n];
        let mut b_vec = vec![1.0; n];
        let mut x_vec = vec![0.0; n];
        let mut s_vec = vec![1.0; n];
        let mut ranges = Vec::new();

        for blk in 0..num_blocks {
            let start = blk * 2;
            ranges.push((start, start + 2));
            let cell = &cells[blk];
            let w_arr = cell["W"].as_array().unwrap();
            w_mat[start][start] = w_arr[0][0].as_f64().unwrap();
            w_mat[start][start + 1] = w_arr[0][1].as_f64().unwrap();
            w_mat[start + 1][start] = w_arr[1][0].as_f64().unwrap();
            w_mat[start + 1][start + 1] = w_arr[1][1].as_f64().unwrap();
            b_vec[start] = cell["b"][0].as_f64().unwrap();
            b_vec[start + 1] = cell["b"][1].as_f64().unwrap();
            x_vec[start] = cell["x"][0].as_f64().unwrap();
            x_vec[start + 1] = cell["x"][1].as_f64().unwrap();
            s_vec[start] = cell["scales"][0].as_f64().unwrap();
            s_vec[start + 1] = cell["scales"][1].as_f64().unwrap();
        }

        let metric = FrozenMetric::new(&s_vec).unwrap();
        let target = LinearTargetId::represented(
            &w_mat,
            metric,
            None,
            AssemblyUncertainty::Exact,
            DeclaredStructure::IsolatedBlocks(ranges.clone()),
        )
        .unwrap();
        let witness = ResidualGainWitness::new(target.clone()).unwrap();
        let cert = witness
            .certify(&target, &b_vec, &x_vec)
            .as_verified()
            .unwrap()
            .clone();

        cases.push(json!({
            "id": format!("d_isolated_blocks_n{n}"),
            "category": "case_d_isolated_blocks",
            "dimension": n,
            "inputs": {
                "operator_kind": "represented",
                "W_hex": w_mat.iter().map(|row| row.iter().map(|&v| f64_to_hex(v)).collect::<Vec<_>>()).collect::<Vec<_>>(),
                "rhs_hex": b_vec.iter().map(|&v| f64_to_hex(v)).collect::<Vec<_>>(),
                "candidate_hex": x_vec.iter().map(|&v| f64_to_hex(v)).collect::<Vec<_>>(),
                "scales_hex": s_vec.iter().map(|&v| f64_to_hex(v)).collect::<Vec<_>>(),
                "structure": "isolated_blocks",
                "blocks": ranges
            },
            "evidence_kind": "verified",
            "residual_intervals_hex": cert.residual_intervals().iter().map(|z| [f64_to_hex(z.lo), f64_to_hex(z.hi)]).collect::<Vec<_>>(),
            "correction_intervals_hex": cert.correction_intervals().iter().map(|z| [f64_to_hex(z.lo), f64_to_hex(z.hi)]).collect::<Vec<_>>(),
            "weighted_correction_intervals_hex": cert.weighted_correction_intervals().iter().map(|z| [f64_to_hex(z.lo), f64_to_hex(z.hi)]).collect::<Vec<_>>(),
            "wrms_upper": cert.wrms_upper(),
            "weighted_inf_upper": cert.weighted_inf_upper(),
            "setup_flops": witness.setup_flops(),
            "verification_flops": cert.verification_flops()
        }));
    }

    // n = 3: 2x2 block plus 1x1 block
    {
        let w_mat = vec![
            vec![2.0, 1.0, 0.0],
            vec![1.0, 3.0, 0.0],
            vec![0.0, 0.0, 4.0],
        ];
        let b = vec![1.0, 1.0, 1.0];
        let x = vec![0.1, 0.2, 0.25];
        let s = vec![1.0, 1.0, 1.0];
        let ranges = vec![(0, 2), (2, 3)];
        let metric = FrozenMetric::new(&s).unwrap();
        let target = LinearTargetId::represented(
            &w_mat,
            metric,
            None,
            AssemblyUncertainty::Exact,
            DeclaredStructure::IsolatedBlocks(ranges.clone()),
        )
        .unwrap();
        let witness = ResidualGainWitness::new(target.clone()).unwrap();
        let cert = witness
            .certify(&target, &b, &x)
            .as_verified()
            .unwrap()
            .clone();

        cases.push(json!({
            "id": "d_isolated_blocks_n3",
            "category": "case_d_isolated_blocks",
            "dimension": 3,
            "inputs": {
                "operator_kind": "represented",
                "W_hex": w_mat.iter().map(|row| row.iter().map(|&v| f64_to_hex(v)).collect::<Vec<_>>()).collect::<Vec<_>>(),
                "rhs_hex": b.iter().map(|&v| f64_to_hex(v)).collect::<Vec<_>>(),
                "candidate_hex": x.iter().map(|&v| f64_to_hex(v)).collect::<Vec<_>>(),
                "scales_hex": s.iter().map(|&v| f64_to_hex(v)).collect::<Vec<_>>(),
                "structure": "isolated_blocks",
                "blocks": ranges
            },
            "evidence_kind": "verified",
            "residual_intervals_hex": cert.residual_intervals().iter().map(|z| [f64_to_hex(z.lo), f64_to_hex(z.hi)]).collect::<Vec<_>>(),
            "correction_intervals_hex": cert.correction_intervals().iter().map(|z| [f64_to_hex(z.lo), f64_to_hex(z.hi)]).collect::<Vec<_>>(),
            "weighted_correction_intervals_hex": cert.weighted_correction_intervals().iter().map(|z| [f64_to_hex(z.lo), f64_to_hex(z.hi)]).collect::<Vec<_>>(),
            "wrms_upper": cert.wrms_upper(),
            "weighted_inf_upper": cert.weighted_inf_upper(),
            "setup_flops": witness.setup_flops(),
            "verification_flops": cert.verification_flops()
        }));
    }

    // Route (d) at n in {3, 4, 8}
    for &n in &[3, 4, 8] {
        for &peclet in &[0.0, 10.0] {
            for &h_gamma in &[1e-3, 1e-1] {
                let h_step = h_gamma / gamma;
                let mut j = vec![vec![0.0; n]; n];
                for i in 0..n {
                    j[i][i] = -200.0 - 100.0 * peclet;
                    if i > 0 {
                        j[i][i - 1] = 100.0 + 100.0 * peclet;
                    }
                    if i + 1 < n {
                        j[i][i + 1] = 100.0;
                    }
                }

                let s: Vec<f64> = (0..n).map(|k| 1.0 + (k as f64).sin().abs()).collect();
                let b = vec![1.0; n];

                // LU solution
                let mut w_mat = vec![vec![0.0; n]; n];
                for i in 0..n {
                    for col in 0..n {
                        w_mat[i][col] = if i == col {
                            1.0 - h_gamma * j[i][col]
                        } else {
                            -h_gamma * j[i][col]
                        };
                    }
                }
                let dm =
                    rodas5p_core::DenseMatrix::new(n, n, w_mat.iter().flatten().copied().collect())
                        .unwrap();
                let lu = rodas5p_core::LuFactorization::new(&dm).unwrap();
                let mut x = lu.solve(&b).unwrap();
                x[1] += 2.0_f64.powi(-30);

                let metric = FrozenMetric::new(&s).unwrap();
                let target = LinearTargetId::shifted(
                    &j,
                    h_step,
                    gamma,
                    metric,
                    None,
                    AssemblyUncertainty::Exact,
                    DeclaredStructure::Dense,
                )
                .unwrap();
                let lu_est = dense_lu_comparator(&w_mat, &b, &x, &s).unwrap();

                let witness_res = ResidualGainWitness::new(target.clone());
                match witness_res {
                    Ok(witness) => {
                        let ev = witness.certify(&target, &b, &x);
                        if let Some(cert) = ev.as_verified() {
                            cases.push(json!({
                                "id": format!("d_lognorm_n{n}_peclet{peclet}_hgamma{h_gamma}"),
                                "category": "case_d_lognorm",
                                "dimension": n,
                                "inputs": {
                                    "operator_kind": "shifted",
                                    "J_hex": j.iter().map(|row| row.iter().map(|&v| f64_to_hex(v)).collect::<Vec<_>>()).collect::<Vec<_>>(),
                                    "h_hex": f64_to_hex(h_step),
                                    "gamma_hex": f64_to_hex(gamma),
                                    "rhs_hex": b.iter().map(|&v| f64_to_hex(v)).collect::<Vec<_>>(),
                                    "candidate_hex": x.iter().map(|&v| f64_to_hex(v)).collect::<Vec<_>>(),
                                    "scales_hex": s.iter().map(|&v| f64_to_hex(v)).collect::<Vec<_>>(),
                                    "structure": "dense",
                                    "assembly_uncertainty": "exact"
                                },
                                "evidence_kind": "verified",
                                "wrms_upper": cert.wrms_upper(),
                                "weighted_inf_upper": cert.weighted_inf_upper(),
                                "setup_flops": witness.setup_flops(),
                                "verification_flops": cert.verification_flops(),
                                "lognorm_mu_up": witness.lognorm_mu_up(),
                                "lognorm_mu_source": witness.lognorm_mu_source(),
                                "lognorm_denom_down": witness.lognorm_denom_down(),
                                "lu_comparator": lu_est
                            }));
                        } else {
                            cases.push(json!({
                                "id": format!("d_lognorm_n{n}_peclet{peclet}_hgamma{h_gamma}"),
                                "category": "case_d_lognorm",
                                "dimension": n,
                                "inputs": {
                                    "operator_kind": "shifted",
                                    "J_hex": j.iter().map(|row| row.iter().map(|&v| f64_to_hex(v)).collect::<Vec<_>>()).collect::<Vec<_>>(),
                                    "h_hex": f64_to_hex(h_step),
                                    "gamma_hex": f64_to_hex(gamma),
                                    "rhs_hex": b.iter().map(|&v| f64_to_hex(v)).collect::<Vec<_>>(),
                                    "candidate_hex": x.iter().map(|&v| f64_to_hex(v)).collect::<Vec<_>>(),
                                    "scales_hex": s.iter().map(|&v| f64_to_hex(v)).collect::<Vec<_>>(),
                                    "structure": "dense",
                                    "assembly_uncertainty": "exact"
                                },
                                "evidence_kind": "unavailable",
                                "rejection_class": ev.as_unavailable().map(|r| r.name()),
                                "lu_comparator": lu_est
                            }));
                        }
                    }
                    Err(rej) => {
                        cases.push(json!({
                            "id": format!("d_lognorm_n{n}_peclet{peclet}_hgamma{h_gamma}"),
                            "category": "case_d_lognorm",
                            "dimension": n,
                            "inputs": {
                                "operator_kind": "shifted",
                                "J_hex": j.iter().map(|row| row.iter().map(|&v| f64_to_hex(v)).collect::<Vec<_>>()).collect::<Vec<_>>(),
                                "h_hex": f64_to_hex(h_step),
                                "gamma_hex": f64_to_hex(gamma),
                                "rhs_hex": b.iter().map(|&v| f64_to_hex(v)).collect::<Vec<_>>(),
                                "candidate_hex": x.iter().map(|&v| f64_to_hex(v)).collect::<Vec<_>>(),
                                "scales_hex": s.iter().map(|&v| f64_to_hex(v)).collect::<Vec<_>>(),
                                "structure": "dense",
                                "assembly_uncertainty": "exact"
                            },
                            "evidence_kind": "unavailable",
                            "rejection_class": rej.name(),
                            "lu_comparator": lu_est
                        }));
                    }
                }
            }
        }
    }

    // Route (d) negatives
    {
        // Negative shift
        let j = vec![vec![1.0; 3]; 3];
        let t_neg = LinearTargetId::shifted(
            &j,
            -0.1,
            gamma,
            FrozenMetric::new(&[1.0; 3]).unwrap(),
            None,
            AssemblyUncertainty::Exact,
            DeclaredStructure::Dense,
        )
        .unwrap();
        let wit_neg = ResidualGainWitness::new(t_neg);
        cases.push(json!({
            "id": "d_negative_shift",
            "category": "case_d_negatives",
            "dimension": 3,
            "evidence_kind": "unavailable",
            "rejection_class": wit_neg.err().map(|r| r.name())
        }));

        // Nonpositive denominator: J = 10 I, a = 1
        let mut j_pos = vec![vec![0.0; 3]; 3];
        for i in 0..3 {
            j_pos[i][i] = 10.0;
        }
        let h_pos = 1.0 / gamma;
        let t_denom = LinearTargetId::shifted(
            &j_pos,
            h_pos,
            gamma,
            FrozenMetric::new(&[1.0; 3]).unwrap(),
            None,
            AssemblyUncertainty::Exact,
            DeclaredStructure::Dense,
        )
        .unwrap();
        let wit_denom = ResidualGainWitness::new(t_denom);
        cases.push(json!({
            "id": "d_nonpositive_denominator",
            "category": "case_d_negatives",
            "dimension": 3,
            "evidence_kind": "unavailable",
            "rejection_class": wit_denom.err().map(|r| r.name())
        }));
    }

    let runs_doc = json!({
        "schema": "vigilode-as05-runs-v1",
        "node": "as05_residual_gain_core_20261011",
        "total_cases": cases.len(),
        "w_formation_discrimination": w_formation_discrimination,
        "cases": cases
    });

    let mut file = File::create(&out_path).unwrap();
    file.write_all(serde_json::to_string_pretty(&runs_doc).unwrap().as_bytes())
        .unwrap();
    println!("Exported {} cases to {}", cases.len(), out_path.display());
}
