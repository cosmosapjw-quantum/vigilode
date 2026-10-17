//! Contract tests and case exporter for power-of-two fused normalization (RVJ DAG node PY04).
//!
//! Research node: `research/py04_fused_normalization_20261011`
//! Preregistration: `research/py04_fused_normalization_20261011/PREREGISTRATION.md`

use std::alloc::{GlobalAlloc, Layout, System};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use rodas5p_core::DenseMatrix;
use rodas5p_core::directed::{
    PowerOfTwo, scale_pow2_down, scale_pow2_is_exact, scale_pow2_nearest,
    scale_pow2_nearest_counted, scale_pow2_up,
};
use rodas5p_core::taylor_phi::taylor_phi_action;
use rodas5p_core::taylor_phi_total::{
    FUSED_PHI_RANGE_UNSUPPORTED, FusedPhiCertificate, NormalizedFusedPhiCertificate,
    NormalizedFusedPhiWork, certify_fused_phi_total, certify_fused_phi_total_normalized,
};
use serde_json::{Value, json};

// -----------------------------------------------------------------------------
// VG-1 Zero-Allocation Allocator
// -----------------------------------------------------------------------------

struct CountingAllocator;

thread_local! {
    static TRACK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() && TRACK.with(|t| t.get()) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
    }
}

fn track_allocations<R>(f: impl FnOnce() -> R) -> (R, u64) {
    ALLOCATIONS.store(0, Ordering::SeqCst);
    TRACK.with(|t| t.set(true));
    let result = f();
    TRACK.with(|t| t.set(false));
    let count = ALLOCATIONS.load(Ordering::SeqCst);
    (result, count)
}

fn hex(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn hexes(v: &[f64]) -> Vec<String> {
    v.iter().map(|x| hex(*x)).collect()
}

// -----------------------------------------------------------------------------
// PRNG and Matrix Generator (PP11 style)
// -----------------------------------------------------------------------------

struct SplitMix(u64);

impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Exact binary value in [-1, 1).
    fn symmetric(&mut self) -> f64 {
        2.0 * ((self.next() >> 11) as f64 * 2f64.powi(-53)) - 1.0
    }
}

#[allow(clippy::needless_range_loop)]
fn base_matrix(n: usize, symmetric: bool, rng: &mut SplitMix) -> Vec<Vec<f64>> {
    let mut a = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            if i == j || (symmetric && j < i) {
                continue;
            }
            a[i][j] = rng.symmetric();
            if symmetric {
                a[j][i] = a[i][j];
            }
        }
    }
    for i in 0..n {
        let row: f64 = (0..n).filter(|&j| j != i).map(|j| a[i][j].abs()).sum();
        let col: f64 = (0..n).filter(|&j| j != i).map(|j| a[j][i].abs()).sum();
        a[i][i] = -(row + col) / 2.0 - 1.0;
    }
    a
}

fn one_norm(a: &[Vec<f64>]) -> f64 {
    let n = a.len();
    (0..n)
        .map(|j| (0..n).map(|i| a[i][j].abs()).sum::<f64>())
        .fold(0.0, f64::max)
}

fn record_unnormalized(cert: &FusedPhiCertificate) -> Value {
    json!({
        "schema": cert.schema(),
        "status": cert.status(),
        "unbounded_reason": cert.unbounded_reason(),
        "bound": hex(cert.bound()),
        "candidate_distance": hex(cert.candidate_distance()),
        "stepped_error": hex(cert.stepped_error()),
        "perturbation": hex(cert.perturbation()),
        "dominant": cert.dominant(),
        "delta_one_norm": hex(cert.delta_one_norm()),
        "delta_inf_norm": hex(cert.delta_inf_norm()),
        "delta_two_norm": hex(cert.delta_two_norm()),
        "omega": hex(cert.omega()),
        "v_two_norm": hex(cert.v_two_norm()),
        "chosen_metric": cert.chosen_metric(),
        "stepped_candidate": hexes(cert.stepped_candidate()),
        "attempts": cert.attempts().iter().map(|a| json!({
            "metric": a.metric(), "steps": a.steps(), "bounded": a.bounded(),
            "error_upper": hex(a.error_upper()), "note": a.note(),
        })).collect::<Vec<_>>(),
    })
}

fn record_normalized(cert: &NormalizedFusedPhiCertificate) -> Value {
    json!({
        "schema": cert.schema(),
        "status": cert.status(),
        "dimension": cert.dimension(),
        "h": hex(cert.h()),
        "exponent": cert.exponent(),
        "candidate": hexes(cert.candidate()),
        "f_hat": hexes(cert.f_hat()),
        "bound": hex(cert.bound()),
        "f_hat_bound": hex(cert.f_hat_bound()),
        "caller_distance": hex(cert.caller_distance()),
        "normalized_bound": hex(cert.normalized_bound()),
        "rescaled_bound": hex(cert.rescaled_bound()),
        "r_term": hex(cert.r_term()),
        "l_term": hex(cert.l_term()),
        "mu": cert.mu().map(hex),
        "v": cert.v().iter().map(|vk| hexes(vk)).collect::<Vec<_>>(),
        "z": hexes(cert.z()),
        "stepped_error": hex(cert.stepped_error()),
        "perturbation": hex(cert.perturbation()),
        "normalized_distance": hex(cert.normalized_distance()),
        "work_entries_scaled": cert.work_entries_scaled(),
        "work_stages": cert.work_stages(),
        "work_total": cert.work_total(),
        "work": cert.work(),
        "dominant": cert.dominant(),
        "normalized_certificate": record_unnormalized(cert.normalized_certificate()),
    })
}

// -----------------------------------------------------------------------------
// VG-1 Contract Tests
// -----------------------------------------------------------------------------

#[test]
fn test_scale_pow2_zero_allocations() {
    let test_values = [
        0.0,
        1.0,
        -1.0,
        1.5,
        1.0e-310,
        1.0e300,
        f64::MIN_POSITIVE,
        f64::MAX,
    ];
    let exponents = [-2500, -1074, -1023, -1000, -50, 0, 50, 1000, 1023, 2500];

    for &x in &test_values {
        for &e in &exponents {
            let (_, allocs_nearest) = track_allocations(|| scale_pow2_nearest(x, e));
            assert_eq!(allocs_nearest, 0, "scale_pow2_nearest allocated");

            let (_, allocs_exact) = track_allocations(|| scale_pow2_is_exact(x, e));
            assert_eq!(allocs_exact, 0, "scale_pow2_is_exact allocated");

            // Directed variants stay finite for x == 0, e <= 0, or non-overflowing e > 0
            if x == 0.0 || e <= 0 || (x <= 1.0 && e <= 1023) {
                let (_, allocs_up) = track_allocations(|| scale_pow2_up(x, e));
                assert_eq!(allocs_up, 0, "scale_pow2_up allocated");

                let (_, allocs_down) = track_allocations(|| scale_pow2_down(x, e));
                assert_eq!(allocs_down, 0, "scale_pow2_down allocated");
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Extreme Exponents and Directed Scaling
// -----------------------------------------------------------------------------

#[test]
fn test_staged_scaling_at_extreme_exponents() {
    // e = -1074 with no 2^1074 ever formed
    let smallest_subnormal = f64::from_bits(1);
    assert_eq!(scale_pow2_nearest(1.0, -1074), smallest_subnormal);
    assert!(scale_pow2_is_exact(1.0, -1074));

    // Underflow into subnormal half-quantum
    assert_eq!(scale_pow2_nearest(1.0, -1075), 0.0);
    assert!(!scale_pow2_is_exact(1.0, -1075));
    assert_eq!(scale_pow2_up(1.0, -1075).unwrap(), smallest_subnormal);
    assert_eq!(scale_pow2_down(1.0, -1075).unwrap(), -smallest_subnormal);

    // PowerOfTwo inversion
    let p = PowerOfTwo::new(1050);
    assert_eq!(p.inverse().exponent, -1050);
    assert_eq!(p.inverse().inverse().exponent, 1050);
}

#[test]
fn test_extreme_i64_exponents_regression() {
    let extreme_exponents = [
        i64::MIN,
        i64::MIN + 1,
        -1_000_000_000_000_i64,
        -100_000_i64,
        100_000_i64,
        1_000_000_000_000_i64,
        i64::MAX - 1,
        i64::MAX,
    ];

    for &e in &extreme_exponents {
        // Zero always stays exact zero with 0 stages and zero allocations
        let ((val_zero, stages_zero), allocs_zero) =
            track_allocations(|| scale_pow2_nearest_counted(0.0, e));
        assert_eq!(allocs_zero, 0);
        assert_eq!(val_zero, 0.0);
        assert_eq!(stages_zero, 0);
        assert!(scale_pow2_is_exact(0.0, e));

        let (val_neg_zero, stages_neg_zero) = scale_pow2_nearest_counted(-0.0, e);
        assert_eq!(val_neg_zero, -0.0);
        assert_eq!(stages_neg_zero, 0);
        assert!(scale_pow2_is_exact(-0.0, e));

        if e < 0 {
            // Negative extreme exponent -> underflows without hang, panic, or allocations
            let ((v_pos, st_pos), allocs_pos) =
                track_allocations(|| scale_pow2_nearest_counted(1.0, e));
            assert_eq!(allocs_pos, 0);
            assert_eq!(v_pos, 0.0);
            assert_eq!(st_pos, 1);
            assert!(!scale_pow2_is_exact(1.0, e));

            let (v_neg, st_neg) = scale_pow2_nearest_counted(-1.0, e);
            assert_eq!(v_neg, -0.0);
            assert_eq!(st_neg, 1);
            assert!(!scale_pow2_is_exact(-1.0, e));

            let up = scale_pow2_up(1.0, e).unwrap();
            assert_eq!(up, f64::from_bits(1));
            let down = scale_pow2_down(-1.0, e).unwrap();
            assert_eq!(down, -f64::from_bits(1));
        } else {
            // Positive extreme exponent -> overflows to infinity without hang, panic, or allocations
            let ((v_pos, st_pos), allocs_pos) =
                track_allocations(|| scale_pow2_nearest_counted(1.0, e));
            assert_eq!(allocs_pos, 0);
            assert_eq!(v_pos, f64::INFINITY);
            assert_eq!(st_pos, 1);
            assert!(!scale_pow2_is_exact(1.0, e));

            let (v_neg, st_neg) = scale_pow2_nearest_counted(-1.0, e);
            assert_eq!(v_neg, f64::NEG_INFINITY);
            assert_eq!(st_neg, 1);
            assert!(!scale_pow2_is_exact(-1.0, e));

            assert!(scale_pow2_up(1.0, e).is_err());
            assert!(scale_pow2_down(1.0, e).is_err());
            assert!(scale_pow2_up(-1.0, e).is_err());
            assert!(scale_pow2_down(-1.0, e).is_err());
        }
    }
}

// -----------------------------------------------------------------------------
// VG-3 Homogeneity on S7 Triple
// -----------------------------------------------------------------------------

#[test]
fn test_exact_homogeneity_s7() {
    let n = 5;
    let mut rng = SplitMix(41000);
    let base = base_matrix(n, true, &mut rng);
    let norm = 1.0;
    let h = 0.25; // Power of two control
    let factor = norm / (h * one_norm(&base));
    let a: Vec<Vec<f64>> = base
        .iter()
        .map(|row| row.iter().map(|x| x * factor).collect())
        .collect();

    let base_w: [Vec<f64>; 5] = std::array::from_fn(|_| (0..n).map(|_| rng.symmetric()).collect());

    // s in {1, 2^600, 2^-600}
    let s_exponents = [0_i64, 600, -600];
    let certs: Vec<NormalizedFusedPhiCertificate> = s_exponents
        .iter()
        .map(|&e| {
            let w: [Vec<f64>; 5] = std::array::from_fn(|k| {
                base_w[k]
                    .iter()
                    .map(|&x| scale_pow2_nearest(x, e))
                    .collect()
            });
            certify_fused_phi_total_normalized(&a, h, &w, None).unwrap()
        })
        .collect();

    let c0 = &certs[0];
    for (idx, c) in certs.iter().enumerate().skip(1) {
        let e = s_exponents[idx];
        // Bitwise identical normalized internals
        for k in 0..5 {
            for i in 0..n {
                assert_eq!(c.v()[k][i].to_bits(), c0.v()[k][i].to_bits());
            }
        }
        for i in 0..n {
            assert_eq!(c.z()[i].to_bits(), c0.z()[i].to_bits());
        }
        assert_eq!(
            c.normalized_bound().to_bits(),
            c0.normalized_bound().to_bits()
        );
        assert_eq!(c.stepped_error().to_bits(), c0.stepped_error().to_bits());
        assert_eq!(c.perturbation().to_bits(), c0.perturbation().to_bits());
        assert_eq!(
            c.normalized_distance().to_bits(),
            c0.normalized_distance().to_bits()
        );

        // Bitwise scaling of bound: bound_s == s * bound_1
        let expected_bound = scale_pow2_nearest(c0.bound(), e);
        assert_eq!(
            c.bound().to_bits(),
            expected_bound.to_bits(),
            "homogeneity bound mismatch for e={e}"
        );
    }
}

// -----------------------------------------------------------------------------
// Range Refusal and Accounting Tests
// -----------------------------------------------------------------------------

#[test]
fn test_range_refusal_unsupported() {
    let _n = 2;
    let a = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
    let h = 1.0;
    // Huge w that causes overflow
    let w: [Vec<f64>; 5] = std::array::from_fn(|_| vec![f64::MAX, f64::MAX]);
    let res = certify_fused_phi_total_normalized(&a, h, &w, None);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(err.to_string().contains(FUSED_PHI_RANGE_UNSUPPORTED));
}

#[test]
fn test_normalization_accounting_l_and_r() {
    let n = 5;
    let mut rng = SplitMix(41001);
    let base = base_matrix(n, false, &mut rng);
    let h = 0.3;
    let factor = 1.0 / (h * one_norm(&base));
    let a: Vec<Vec<f64>> = base
        .iter()
        .map(|row| row.iter().map(|x| x * factor).collect())
        .collect();
    let base_w: [Vec<f64>; 5] = std::array::from_fn(|_| (0..n).map(|_| rng.symmetric()).collect());

    // S6 mixed: w_0 x 2^1000, w_1..w_3 x 1, w_4 x 2^-1070
    let mut w_s6: [Vec<f64>; 5] = base_w.clone();
    for x in &mut w_s6[0] {
        *x = scale_pow2_nearest(*x, 1000);
    }
    for x in &mut w_s6[4] {
        *x = scale_pow2_nearest(*x, -1070);
    }

    let cert_s6 = certify_fused_phi_total_normalized(&a, h, &w_s6, None).unwrap();
    // In S6, w_4 underflows during normalization, so L > 0
    assert!(
        cert_s6.l_term() > 0.0,
        "L term must be positive for S6 mixed"
    );
}

#[test]
fn test_taylor_rejection_speculative_work_preserved() {
    let n = 2;
    // ||hA||_1 = 70.0 > TAYLOR_NORM_LIMIT (64.0), forcing Taylor action rejection
    // Stable negative diagonal ensures the stepped exp certificate succeeds
    let a = vec![vec![-70.0, 0.0], vec![0.0, -70.0]];
    let h = 1.0;
    let w: [Vec<f64>; 5] = std::array::from_fn(|_| vec![1.0, 0.5]);

    let cert = certify_fused_phi_total_normalized(&a, h, &w, None)
        .expect("normalized certification should succeed via fallback probe");

    // Speculative work accounting contract assertions
    let work: &NormalizedFusedPhiWork = cert.work();
    assert!(work.taylor_attempted);
    assert!(!cert.taylor_accepted());
    assert_eq!(cert.work().taylor_vector_products, 0);
    assert_eq!(cert.work().taylor_witness_entries, (n * n) as u64);

    assert!(cert.fallback_probe_used());
    let probe_steps = cert.work().fallback_probe_steps;
    assert!(probe_steps > 0, "fallback probe must record nonzero steps");
    let cert_steps = cert.work().certificate_steps;
    assert!(
        cert_steps > 0,
        "final certificate must record nonzero steps"
    );

    let min_expected_work = (probe_steps as u64)
        .saturating_mul(20)
        .saturating_add((cert_steps as u64).saturating_mul(20));
    assert!(
        cert.work_total() >= min_expected_work,
        "work_total ({}) must be >= probe_steps * 20 + cert_steps * 20 ({})",
        cert.work_total(),
        min_expected_work
    );
}

#[test]
fn test_range_refusal_l_term_overflow() {
    let _n = 2;
    // Extremely large positive diagonal causes mu_max to overflow in exp_interval
    let a = vec![vec![1000.0, 0.0], vec![0.0, 1000.0]];
    let h = 1.0;
    let mut w: [Vec<f64>; 5] = std::array::from_fn(|_| vec![1.0, 1.0]);
    // Set w_0 to large power of two, w_4 to tiny so w scaling underflows into subnormal
    w[0][0] = scale_pow2_nearest(1.0, 1000);
    w[4][0] = scale_pow2_nearest(1.0, -1070);

    let res = certify_fused_phi_total_normalized(&a, h, &w, None);
    assert!(res.is_err(), "must refuse when L term growth overflows");
    let err = res.unwrap_err();
    assert!(
        err.to_string().contains(FUSED_PHI_RANGE_UNSUPPORTED),
        "error must be FUSED_PHI_RANGE_UNSUPPORTED, got: {err}"
    );
}

// -----------------------------------------------------------------------------
// Case Exporter
// -----------------------------------------------------------------------------

#[test]
#[ignore = "export PY04 cases; set PY04_CASES"]
fn export_cases() {
    let Ok(raw_path) = std::env::var("PY04_CASES") else {
        println!("PY04_CASES not set: skipping export");
        return;
    };

    // Resolve relative path against workspace root
    let target = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(&raw_path);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }

    let mut cases = Vec::new();
    const TAYLOR_BUDGET: f64 = 1e-13;

    // 16 matrices
    for (s_idx, symmetric) in [true, false].into_iter().enumerate() {
        for (n_idx, n) in [5_usize, 12].into_iter().enumerate() {
            for (norm_idx, norm) in [1.0_f64, 8.0].into_iter().enumerate() {
                for (h_idx, h) in [0.3_f64, 0.25].into_iter().enumerate() {
                    let matrix_idx = s_idx * 8 + n_idx * 4 + norm_idx * 2 + h_idx;
                    let seed = 41000 + matrix_idx as u64;
                    let mut rng = SplitMix(seed);
                    let base = base_matrix(n, symmetric, &mut rng);
                    let base_w: [Vec<f64>; 5] =
                        std::array::from_fn(|_| (0..n).map(|_| rng.symmetric()).collect());

                    let factor = norm / (h * one_norm(&base));
                    let a: Vec<Vec<f64>> = base
                        .iter()
                        .map(|row| row.iter().map(|x| x * factor).collect())
                        .collect();

                    let rows: Vec<&[f64]> = a.iter().map(Vec::as_slice).collect();
                    let dense = DenseMatrix::from_rows(&rows).unwrap();

                    // 9 input families
                    let families = [
                        ("S1", "s1-boundary-straddle"),
                        ("S2", "s2-mid-subnormal"),
                        ("S3", "s3-deep-subnormal"),
                        ("S4", "s4-near-overflow"),
                        ("S5", "s5-overflow-boundary"),
                        ("S6", "s6-mixed"),
                        ("S7a", "s7-invariance-1"),
                        ("S7b", "s7-invariance-pow2-pos"),
                        ("S7c", "s7-invariance-pow2-neg"),
                    ];

                    for (family_id, family_desc) in families {
                        let w: [Vec<f64>; 5] = match family_id {
                            "S1" => std::array::from_fn(|k| {
                                base_w[k]
                                    .iter()
                                    .map(|&x| scale_pow2_nearest(x, -1016))
                                    .collect()
                            }),
                            "S2" => std::array::from_fn(|k| {
                                base_w[k]
                                    .iter()
                                    .map(|&x| scale_pow2_nearest(x, -1050))
                                    .collect()
                            }),
                            "S3" => std::array::from_fn(|k| {
                                base_w[k]
                                    .iter()
                                    .map(|&x| scale_pow2_nearest(x, -1068))
                                    .collect()
                            }),
                            "S4" => std::array::from_fn(|k| {
                                base_w[k]
                                    .iter()
                                    .map(|&x| scale_pow2_nearest(x, 1018))
                                    .collect()
                            }),
                            "S5" => std::array::from_fn(|k| {
                                base_w[k]
                                    .iter()
                                    .map(|&x| scale_pow2_nearest(x, 1023))
                                    .collect()
                            }),
                            "S6" => {
                                let mut mixed = base_w.clone();
                                for x in &mut mixed[0] {
                                    *x = scale_pow2_nearest(*x, 1000);
                                }
                                for x in &mut mixed[4] {
                                    *x = scale_pow2_nearest(*x, -1070);
                                }
                                mixed
                            }
                            "S7a" => base_w.clone(),
                            "S7b" => std::array::from_fn(|k| {
                                base_w[k]
                                    .iter()
                                    .map(|&x| scale_pow2_nearest(x, 600))
                                    .collect()
                            }),
                            "S7c" => std::array::from_fn(|k| {
                                base_w[k]
                                    .iter()
                                    .map(|&x| scale_pow2_nearest(x, -600))
                                    .collect()
                            }),
                            _ => unreachable!(),
                        };

                        let label = format!(
                            "{}-n{n}-norm{norm}-h{h}-{family_desc}",
                            if symmetric { "sym" } else { "nonsym" }
                        );

                        // 1. Normalized certificate (gated candidate F_hat)
                        let norm_res = certify_fused_phi_total_normalized(&a, h, &w, None);
                        let (norm_record, f_hat_candidate) = match &norm_res {
                            Ok(cert) => (record_normalized(cert), Some(cert.f_hat().to_vec())),
                            Err(e) if e.to_string().contains(FUSED_PHI_RANGE_UNSUPPORTED) => (
                                json!({
                                    "status": "refused",
                                    "refusal": FUSED_PHI_RANGE_UNSUPPORTED,
                                }),
                                None,
                            ),
                            Err(e) => panic!("unexpected error for {label}: {e}"),
                        };

                        // 2. Taylor caller candidate where accepted
                        let taylor = taylor_phi_action(&dense, h, &w, TAYLOR_BUDGET);
                        let taylor_caller_record = match &taylor {
                            Ok(report) => {
                                let u = &report.fused;
                                let caller_cert_res =
                                    certify_fused_phi_total_normalized(&a, h, &w, Some(u));
                                match caller_cert_res {
                                    Ok(cert) => json!({
                                        "accepted": true,
                                        "candidate": hexes(u),
                                        "certificate": record_normalized(&cert),
                                    }),
                                    Err(e)
                                        if e.to_string().contains(FUSED_PHI_RANGE_UNSUPPORTED) =>
                                    {
                                        json!({
                                            "accepted": true,
                                            "candidate": hexes(u),
                                            "certificate": {
                                                "status": "refused",
                                                "refusal": FUSED_PHI_RANGE_UNSUPPORTED,
                                            }
                                        })
                                    }
                                    Err(e) => panic!("caller cert error: {e}"),
                                }
                            }
                            Err(err) => json!({
                                "accepted": false,
                                "rejection": err.to_string(),
                            }),
                        };

                        // 3. Baseline unnormalized certify_fused_phi_total
                        let candidate_for_baseline = f_hat_candidate.as_deref().unwrap_or(&w[0]);
                        let baseline_cert =
                            certify_fused_phi_total(&a, h, &w, candidate_for_baseline);
                        let baseline_record = match baseline_cert {
                            Ok(b_cert) => record_unnormalized(&b_cert),
                            Err(e) => json!({
                                "status": "error",
                                "error": e.to_string(),
                            }),
                        };

                        cases.push(json!({
                            "label": label,
                            "matrix_index": matrix_idx,
                            "seed": seed,
                            "family": family_id,
                            "symmetric": symmetric,
                            "n": n,
                            "target_hA_one_norm": hex(norm),
                            "h": hex(h),
                            "h_power_of_two": h == 0.25,
                            "a": a.iter().map(|row| hexes(row)).collect::<Vec<_>>(),
                            "base_w": base_w.iter().map(|wk| hexes(wk)).collect::<Vec<_>>(),
                            "w": w.iter().map(|wk| hexes(wk)).collect::<Vec<_>>(),
                            "normalized_certificate": norm_record,
                            "taylor_caller": taylor_caller_record,
                            "unnormalized_baseline": baseline_record,
                        }));
                    }
                }
            }
        }
    }

    assert_eq!(cases.len(), 144, "must export exactly 144 cases");

    // Directed-scaling table
    let mut table_rng = SplitMix(42000);
    let table_mantissas: Vec<f64> = (0..10).map(|_| table_rng.symmetric()).collect();
    let table_exponents = [-1074, -1073, -1023, -1022, -1021, 0, 1021, 1022, 1023];
    let mut scaling_table = Vec::new();

    for &m in &table_mantissas {
        for &e in &table_exponents {
            let nearest = scale_pow2_nearest(m, e);
            let up = scale_pow2_up(m, e);
            let down = scale_pow2_down(m, e);
            let is_exact = scale_pow2_is_exact(m, e);

            scaling_table.push(json!({
                "m": hex(m),
                "e": e,
                "nearest": hex(nearest),
                "up": up.map(hex).ok(),
                "down": down.map(hex).ok(),
                "is_exact": is_exact,
            }));
        }
    }

    let out = json!({
        "schema": "vigilode-py04-cases-v1",
        "cases": cases,
        "scaling_table": scaling_table,
    });

    std::fs::write(&target, serde_json::to_string_pretty(&out).unwrap() + "\n").unwrap();
    println!("Exported {} cases to {}", cases.len(), target.display());
}
