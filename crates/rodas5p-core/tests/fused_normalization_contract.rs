//! Contracts and case exporter of research node
//! `research/py04_fused_normalization_20261011` (re-audit DAG node PY04):
//! power-of-two normalization of the fused Taylor phi total certificate.
//!
//! The contract tests (hand cases) run by default. `export_cases` is the
//! registered case exporter: 16 matrices (the PP11 generator, SplitMix64
//! seeds 41000..41015) times 9 inputs (families S1..S7, S7 three times), the
//! normalized certificate with the Taylor candidate on the original `w` as
//! caller candidate where Taylor accepts it, the unnormalized baseline, and
//! the directed-scaling table. Everything is written to `PY04_CASES`
//! (repo-relative, never overwritten) as IEEE-754 hex bits (`null` for a
//! non-finite value) and checked by `tools/py04_fused_normalization_check.py`.
//! Run with `--ignored --release`. No timings.

use rodas5p_core::DenseMatrix;
use rodas5p_core::directed::{
    POW2_STAGE_LIMIT, PowerOfTwo, scale_pow2_down, scale_pow2_is_exact, scale_pow2_nearest,
    scale_pow2_staged, scale_pow2_up,
};
use rodas5p_core::taylor_phi::taylor_phi_action;
use rodas5p_core::taylor_phi_total::{
    FUSED_PHI_NORMALIZED_SCHEMA, FUSED_PHI_RANGE_UNSUPPORTED, FusedPhiCertificate, FusedPhiStatus,
    NORMALIZED_TAYLOR_BUDGET, NormalizedFusedPhiCertificate, certify_fused_phi_total,
    certify_fused_phi_total_normalized,
};
use serde_json::{Value, json};

// ------------------------------------------------------------------ helpers

/// `2^k` for `-1074 <= k <= 1023`, from its bits (subnormal below -1022).
fn pow2(k: i64) -> f64 {
    assert!((-1074..=1023).contains(&k));
    if k >= -1022 {
        f64::from_bits(((k + 1023) as u64) << 52)
    } else {
        f64::from_bits(1_u64 << (k + 1074))
    }
}

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

fn vectors() -> [Vec<f64>; 5] {
    [
        vec![1.0, -0.5],
        vec![0.25, 1.0],
        vec![-0.5, 0.125],
        vec![0.0, 0.75],
        vec![0.5, -0.25],
    ]
}

fn nonsymmetric() -> Vec<Vec<f64>> {
    vec![vec![-1.3, 0.7], vec![0.3, -2.9]]
}

fn scaled(w: &[Vec<f64>; 5], exponents: [i64; 5]) -> [Vec<f64>; 5] {
    std::array::from_fn(|k| w[k].iter().map(|x| x * pow2(exponents[k])).collect())
}

// --------------------------------------------------------------- contracts

#[test]
fn staged_scaling_at_the_extreme_exponents() {
    // e = -1074: no 2^-1074 (or 2^1074) factor is formed; stages of at most
    // 2^1000.
    let s = scale_pow2_staged(1.0, -1074).unwrap();
    assert_eq!(s.nearest, f64::from_bits(1));
    assert!(s.is_exact());
    assert!(i64::from(s.largest_stage) <= POW2_STAGE_LIMIT);
    assert!(s.stages >= 2, "{s:?}");
    // 1.5 quanta rounds to even (2 quanta); 1.25 to 1 quantum.
    assert_eq!(scale_pow2_nearest(1.5, -1074), f64::from_bits(2));
    assert_eq!(scale_pow2_down(1.5, -1074).unwrap(), f64::from_bits(1));
    assert_eq!(scale_pow2_up(1.5, -1074).unwrap(), f64::from_bits(2));
    assert!(!scale_pow2_is_exact(1.5, -1074));
    assert_eq!(scale_pow2_nearest(1.25, -1074), f64::from_bits(1));
    assert_eq!(scale_pow2_up(1.25, -1074).unwrap(), f64::from_bits(2));
    assert_eq!(scale_pow2_down(-1.25, -1074).unwrap(), -f64::from_bits(2));
    // Below half a quantum: zero to nearest, one quantum outward.
    assert_eq!(scale_pow2_nearest(1.0, -1076), 0.0);
    assert_eq!(scale_pow2_up(1.0, -1076).unwrap(), f64::from_bits(1));
    assert_eq!(scale_pow2_down(1.0, -1076).unwrap(), 0.0);
    assert_eq!(scale_pow2_down(-1.0, -1076).unwrap(), -f64::from_bits(1));
    // Exactly half a quantum: ties to even (zero).
    assert_eq!(scale_pow2_nearest(1.0, -1075), 0.0);
    // Far beyond the range, and i64 extremes (clamped, no panic).
    assert_eq!(scale_pow2_nearest(f64::MAX, -5000), 0.0);
    assert_eq!(
        scale_pow2_up(f64::MAX, i64::MIN).unwrap(),
        f64::from_bits(1)
    );
    assert!(scale_pow2_up(f64::from_bits(1), i64::MAX).is_err());
    assert_eq!(
        scale_pow2_nearest(f64::from_bits(1), i64::MAX),
        f64::INFINITY
    );
    // e = 1023 and the overflow boundary.
    assert_eq!(scale_pow2_nearest(1.0, 1023), pow2(1023));
    assert!(scale_pow2_is_exact(1.999, 1023));
    assert_eq!(scale_pow2_nearest(2.0, 1023), f64::INFINITY);
    assert_eq!(scale_pow2_nearest(-2.0, 1023), f64::NEG_INFINITY);
    assert!(scale_pow2_up(2.0, 1023).is_err());
    assert!(scale_pow2_down(2.0, 1023).is_err());
    assert!(!scale_pow2_is_exact(2.0, 1023));
    // From the smallest subnormal to the top binade: 2097 is staged.
    let s = scale_pow2_staged(f64::from_bits(1), 2097).unwrap();
    assert_eq!(s.nearest, pow2(1023));
    assert!(s.is_exact() && s.stages == 3 && s.largest_stage == 1000);
    // The sigma type: its inverse is the negated exponent, never a value.
    let sigma = PowerOfTwo::floor_of(f64::from_bits(1)).unwrap();
    assert_eq!(sigma.exponent, -1074);
    assert_eq!(sigma.inverse().exponent, 1074);
    assert_eq!(PowerOfTwo::floor_of(f64::MAX).unwrap().exponent, 1023);
    assert_eq!(PowerOfTwo::floor_of(-0.75).unwrap().exponent, -1);
    assert_eq!(
        PowerOfTwo::floor_of(f64::from_bits(3)).unwrap().exponent,
        -1073
    );
    assert!(PowerOfTwo::floor_of(0.0).is_none());
    // Non-finite input: nearest passes it through, directed results refuse.
    assert!(scale_pow2_nearest(f64::NAN, 3).is_nan());
    assert!(scale_pow2_up(f64::INFINITY, -3).is_err());
}

#[test]
fn staged_scaling_matches_one_correctly_rounded_product() {
    // Where 2^e is a binary64 (|e| <= 1022 normal, or subnormal down to
    // -1074 with a normal x of exponent 0 the product rounds once), one
    // hardware product is the correctly rounded reference.
    let mut rng = SplitMix(7);
    for _ in 0..2000 {
        let bits = rng.next();
        let x = f64::from_bits(bits & !(0x7ff_u64 << 52) | ((bits >> 52) % 2046 + 1) << 52);
        if !x.is_finite() {
            continue;
        }
        let e = (rng.next() % 2045) as i64 - 1022;
        let reference = x * pow2(e);
        let s = scale_pow2_staged(x, e).unwrap();
        assert_eq!(
            s.nearest.to_bits(),
            reference.to_bits(),
            "x {x:e} e {e} {s:?}"
        );
        assert!(i64::from(s.largest_stage) <= POW2_STAGE_LIMIT);
        if reference.is_finite() {
            let up = scale_pow2_up(x, e).unwrap();
            let down = scale_pow2_down(x, e).unwrap();
            assert!(down <= reference && reference <= up);
            assert!(up == reference || up == reference.next_up());
            assert!(down == reference || down == reference.next_down());
            // Exact iff scaling back recovers x.
            let back = scale_pow2_nearest(reference, -e);
            assert_eq!(scale_pow2_is_exact(x, e), back == x && up == down);
        }
    }
}

#[test]
fn exact_homogeneity_on_a_small_case() {
    let a = nonsymmetric();
    let w = vectors();
    let base = certify_fused_phi_total_normalized(&a, 0.25, &w, None).unwrap();
    assert_eq!(base.schema(), FUSED_PHI_NORMALIZED_SCHEMA);
    assert_eq!(base.status(), FusedPhiStatus::Bounded);
    assert_eq!(base.exponent(), 0);
    assert_eq!(base.rescale_rounding(), 0.0);
    assert_eq!(base.normalization_rounding(), 0.0);
    for shift in [600_i64, -600, 1000, -1000] {
        let cert =
            certify_fused_phi_total_normalized(&a, 0.25, &scaled(&w, [shift; 5]), None).unwrap();
        assert_eq!(cert.exponent(), shift);
        assert_eq!(cert.normalized_vectors(), base.normalized_vectors());
        assert_eq!(cert.normalized_candidate(), base.normalized_candidate());
        assert_eq!(
            cert.normalized_certificate(),
            base.normalized_certificate(),
            "shift {shift}"
        );
        assert_eq!(cert.rescale_rounding(), 0.0);
        assert_eq!(cert.normalization_rounding(), 0.0);
        let factor = pow2(shift);
        if (base.bound() * factor).abs() >= f64::MIN_POSITIVE {
            // sigma E is normal: every rescaling is exact.
            assert_eq!(cert.bound(), base.bound() * factor, "shift {shift}");
        } else {
            // sigma E is subnormal (shift -1000): rounded up, never down.
            assert!(cert.bound() >= base.bound() * factor, "shift {shift}");
            assert!(cert.bound() <= (base.bound() * factor).next_up());
        }
        for (x, y) in cert.candidate().iter().zip(base.candidate()) {
            assert_eq!(*x, y * factor);
        }
    }
}

#[test]
fn subnormal_and_mixed_scales_enclose_the_normalization_rounding() {
    let a = nonsymmetric();
    let w = vectors();
    // Deep subnormal: v is exact, the rescaled candidate rounds (R > 0).
    let deep = certify_fused_phi_total_normalized(&a, 0.3, &scaled(&w, [-1068; 5]), None).unwrap();
    assert_eq!(deep.status(), FusedPhiStatus::Bounded);
    assert_eq!(deep.normalization_inexact(), [0; 5]);
    assert_eq!(deep.normalization_rounding(), 0.0);
    assert!(deep.rescale_inexact() > 0);
    assert!(deep.rescale_rounding() > 0.0);
    assert!(deep.bound() <= 16.0 * 2f64.sqrt() * f64::from_bits(1) + 1e-10 * 1e-320);
    // Mixed: w_4 / sigma underflows, so L > 0; the bound stays finite.
    let mixed =
        certify_fused_phi_total_normalized(&a, 0.3, &scaled(&w, [1000, 0, 0, 0, -1070]), None)
            .unwrap();
    assert_eq!(mixed.status(), FusedPhiStatus::Bounded);
    assert!(mixed.normalization_inexact()[4] > 0);
    assert!(mixed.normalization_rounding() > 0.0);
    assert!(mixed.bound().is_finite());
    assert!(mixed.bound() >= mixed.scaled_certificate());
    assert!(mixed.work().input_entries == 10 && mixed.work().candidate_entries == 2);
    assert!(mixed.work().stages() > 0);
}

#[test]
fn caller_candidate_adds_its_distance() {
    let a = nonsymmetric();
    let w = scaled(&vectors(), [-1050; 5]);
    let rows: Vec<&[f64]> = a.iter().map(Vec::as_slice).collect();
    let dense = DenseMatrix::from_rows(&rows).unwrap();
    let taylor = taylor_phi_action(&dense, 0.3, &w, NORMALIZED_TAYLOR_BUDGET).unwrap();
    let cert = certify_fused_phi_total_normalized(&a, 0.3, &w, Some(&taylor.fused)).unwrap();
    let distance = cert.caller_distance().unwrap();
    assert_eq!(cert.caller_candidate().unwrap(), taylor.fused.as_slice());
    assert!(cert.caller_bound().unwrap() >= cert.bound() + distance);
    // The candidate itself has distance 0.
    let own = cert.candidate().to_vec();
    let again = certify_fused_phi_total_normalized(&a, 0.3, &w, Some(&own)).unwrap();
    assert_eq!(again.caller_distance(), Some(0.0));
    assert_eq!(again.caller_bound(), Some(again.bound()));
}

#[test]
fn a_result_above_the_range_is_a_typed_refusal() {
    // A = 0: F = w_0 + w_1 + w_2 / 2 + w_3 / 6 + w_4 / 24 = 3 2^1023 > MAX.
    let big = 1.5 * pow2(1023);
    let w = [vec![big], vec![big], vec![0.0], vec![0.0], vec![0.0]];
    let refusal = certify_fused_phi_total_normalized(&[vec![0.0]], 0.25, &w, None).unwrap_err();
    assert!(
        refusal.to_string().contains(FUSED_PHI_RANGE_UNSUPPORTED),
        "{refusal}"
    );
    // Just inside the range: Bounded with a finite candidate and bound.
    let w = [
        vec![pow2(1022)],
        vec![pow2(1021)],
        vec![0.0],
        vec![0.0],
        vec![0.0],
    ];
    let cert = certify_fused_phi_total_normalized(&[vec![0.0]], 0.25, &w, None).unwrap();
    assert_eq!(cert.status(), FusedPhiStatus::Bounded);
    assert!(cert.candidate().iter().all(|x| x.is_finite()) && cert.bound().is_finite());
    // Invalid input is not a range refusal.
    let bad = [vec![f64::NAN], vec![0.0], vec![0.0], vec![0.0], vec![0.0]];
    let error = certify_fused_phi_total_normalized(&[vec![0.0]], 0.25, &bad, None).unwrap_err();
    assert!(!error.to_string().contains(FUSED_PHI_RANGE_UNSUPPORTED));
}

/// The normalized type is what a caller requiring it accepts (the
/// `compile_fail` doctests of `taylor_phi_total` show that the unnormalized
/// type is not).
fn requires_normalized(cert: &NormalizedFusedPhiCertificate) -> (f64, FusedPhiStatus) {
    (cert.bound(), cert.status())
}

#[test]
fn the_normalized_certificate_is_its_own_type() {
    let cert = certify_fused_phi_total_normalized(&nonsymmetric(), 0.25, &vectors(), None).unwrap();
    let (bound, status) = requires_normalized(&cert);
    assert_eq!(status, FusedPhiStatus::Bounded);
    assert!(bound.is_finite());
    // The inner certificate is a FusedPhiCertificate about the normalized
    // data, held by reference only.
    let inner: &FusedPhiCertificate = cert.normalized_certificate();
    assert_eq!(inner.bound(), cert.scaled_certificate());
}

// ----------------------------------------------------------------- exporter

/// Truncation budget of the Taylor candidate on the original `w` (PP11's).
const TAYLOR_BUDGET: f64 = 1e-13;
const SEED_BASE: u64 = 41000;
const TABLE_SEED: u64 = 41016;
const TABLE_EXPONENTS: [i64; 9] = [-1074, -1073, -1023, -1022, -1021, 0, 1021, 1022, 1023];

fn hex(x: f64) -> Value {
    if x.is_finite() {
        Value::String(format!("{:016x}", x.to_bits()))
    } else {
        Value::Null
    }
}

fn hexes(v: &[f64]) -> Value {
    Value::Array(v.iter().map(|x| hex(*x)).collect())
}

fn git(args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?} failed");
    String::from_utf8(out.stdout).expect("utf-8")
}

/// The PP11 generator: off-diagonal entries uniform in [-1, 1) (mirrored
/// when symmetric), the diagonal `-(r_i + c_i) / 2 - 1`.
#[allow(clippy::needless_range_loop)] // (i, j) and (j, i) are both read
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

fn inner_record(cert: &FusedPhiCertificate) -> Value {
    json!({
        "schema": cert.schema(),
        "status": cert.status(),
        "unbounded_reason": cert.unbounded_reason(),
        "bound": hex(cert.bound()),
        "candidate_distance": hex(cert.candidate_distance()),
        "stepped_error": hex(cert.stepped_error()),
        "perturbation": hex(cert.perturbation()),
        "dominant": cert.dominant(),
        "delta_two_norm": hex(cert.delta_two_norm()),
        "omega": hex(cert.omega()),
        "v_two_norm": hex(cert.v_two_norm()),
        "chosen_metric": cert.chosen_metric(),
        "stepped_candidate": hexes(cert.stepped_candidate()),
        "attempts": cert.attempts().iter().map(|a| json!({
            "metric": a.metric(), "steps": a.steps(), "bounded": a.bounded(),
            "error_upper": hex(a.error_upper()),
        })).collect::<Vec<_>>(),
    })
}

fn normalized_record(cert: &NormalizedFusedPhiCertificate) -> Value {
    let work = cert.work();
    json!({
        "schema": cert.schema(),
        "status": cert.status(),
        "unbounded_reason": cert.unbounded_reason(),
        "exponent": cert.exponent(),
        "normalized_vectors": cert.normalized_vectors().iter().map(|v| hexes(v)).collect::<Vec<_>>(),
        "normalization_inexact": cert.normalization_inexact(),
        "normalized_candidate": hexes(cert.normalized_candidate()),
        "normalized_source": cert.normalized_source(),
        "normalized_taylor_rejection": cert.normalized_taylor_rejection(),
        "inner": inner_record(cert.normalized_certificate()),
        "candidate": hexes(cert.candidate()),
        "rescale_inexact": cert.rescale_inexact(),
        "scaled_certificate": hex(cert.scaled_certificate()),
        "rescale_rounding": hex(cert.rescale_rounding()),
        "normalization_rounding": hex(cert.normalization_rounding()),
        "log_norm_upper": hex(cert.log_norm_upper()),
        "bound": hex(cert.bound()),
        "dominant": cert.dominant(),
        "caller_candidate": cert.caller_candidate().map(hexes),
        "caller_distance": cert.caller_distance().map(hex),
        "caller_bound": cert.caller_bound().map(hex),
        "work": {
            "input_entries": work.input_entries,
            "input_stages": work.input_stages,
            "candidate_entries": work.candidate_entries,
            "candidate_stages": work.candidate_stages,
            "bound_scalings": work.bound_scalings,
            "bound_stages": work.bound_stages,
            "entries_scaled": work.entries_scaled(),
            "stages": work.stages(),
        },
    })
}

/// The registered input families: (family, variant, exponent of s per w_k).
fn families() -> Vec<(&'static str, &'static str, [i64; 5])> {
    vec![
        ("S1", "S1", [-1016; 5]),
        ("S2", "S2", [-1050; 5]),
        ("S3", "S3", [-1068; 5]),
        ("S4", "S4", [1018; 5]),
        ("S5", "S5", [1023; 5]),
        ("S6", "S6", [1000, 0, 0, 0, -1070]),
        ("S7", "S7-s0", [0; 5]),
        ("S7", "S7-p600", [600; 5]),
        ("S7", "S7-m600", [-600; 5]),
    ]
}

fn scaling_table() -> Vec<Value> {
    let mut rng = SplitMix(TABLE_SEED);
    let mut rows = Vec::new();
    for e in TABLE_EXPONENTS {
        let mut xs = Vec::new();
        for j in [0_i64, 1, -1, -40] {
            let sign = if rng.next() & 1 == 1 { -1.0 } else { 1.0 };
            let m = 1.0 + (rng.next() >> 12) as f64 * 2f64.powi(-52);
            xs.push(sign * m * pow2(j));
        }
        for _ in 0..2 {
            let sign = if rng.next() & 1 == 1 { -1.0 } else { 1.0 };
            let k = (rng.next() >> 12).max(1);
            xs.push(sign * f64::from_bits(k));
        }
        let sign = if rng.next() & 1 == 1 { -1.0 } else { 1.0 };
        xs.push(sign * (1.0 + (rng.next() >> 44) as f64 * 2f64.powi(-20)));
        let sign = if rng.next() & 1 == 1 { -1.0 } else { 1.0 };
        xs.push(sign * (1.0 + (rng.next() >> 12) as f64 * 2f64.powi(-52)) * pow2(1000));
        for x in xs {
            let s = scale_pow2_staged(x, e).unwrap();
            rows.push(json!({
                "x": hex(x),
                "e": e,
                "nearest": hex(scale_pow2_nearest(x, e)),
                "up": scale_pow2_up(x, e).map_or(Value::Null, hex),
                "down": scale_pow2_down(x, e).map_or(Value::Null, hex),
                "exact": scale_pow2_is_exact(x, e),
                "overflow": s.overflow,
                "stages": s.stages,
                "largest_stage": s.largest_stage,
            }));
        }
    }
    rows
}

#[test]
#[ignore = "case exporter of research/py04_fused_normalization_20261011; release build"]
fn export_cases() {
    let Ok(path) = std::env::var("PY04_CASES") else {
        println!("PY04_CASES not set: cases not written");
        return;
    };
    let target = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(&path);
    assert!(
        !target.exists(),
        "immutable output exists: {}",
        target.display()
    );
    let source_commit = git(&["rev-parse", "HEAD"]).trim().to_string();
    let tree_status = git(&["status", "--porcelain"]);
    let mut cases = Vec::new();
    for (si, symmetric) in [true, false].into_iter().enumerate() {
        for (ni, n) in [5_usize, 12].into_iter().enumerate() {
            for (ti, norm) in [1.0_f64, 8.0].into_iter().enumerate() {
                for (hi, h) in [0.3_f64, 0.25].into_iter().enumerate() {
                    let index = ((si * 2 + ni) * 2 + ti) * 2 + hi;
                    let seed = SEED_BASE + index as u64;
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
                    for (family, variant, exponents) in families() {
                        let w = scaled(&base_w, exponents);
                        let label = format!(
                            "{}-n{n}-norm{norm}-h{h}-{variant}",
                            if symmetric { "sym" } else { "nonsym" }
                        );
                        let taylor = taylor_phi_action(&dense, h, &w, TAYLOR_BUDGET);
                        let taylor_record = match &taylor {
                            Ok(report) => json!({
                                "accepted": true, "fused": hexes(&report.fused), "rejection": null,
                            }),
                            Err(error) => json!({
                                "accepted": false, "fused": null, "rejection": error.to_string(),
                            }),
                        };
                        let caller = taylor.as_ref().ok().map(|r| r.fused.as_slice());
                        let normalized = certify_fused_phi_total_normalized(&a, h, &w, caller);
                        let normalized_record = match &normalized {
                            Ok(cert) => json!({
                                "outcome": "certificate", "error": null,
                                "certificate": normalized_record(cert),
                            }),
                            Err(error) => {
                                let text = error.to_string();
                                let outcome = if text.contains(FUSED_PHI_RANGE_UNSUPPORTED) {
                                    "refused"
                                } else {
                                    "error"
                                };
                                json!({"outcome": outcome, "error": text, "certificate": null})
                            }
                        };
                        // Baseline (reported only): the PP11 candidate rule.
                        let (baseline_candidate, baseline_source) = match &taylor {
                            Ok(report) => (report.fused.clone(), "taylor_phi_action"),
                            Err(_) => {
                                let probe = certify_fused_phi_total(&a, h, &w, &vec![0.0; n]);
                                match probe {
                                    Ok(p) if p.stepped_top().iter().all(|x| x.is_finite()) => {
                                        (p.stepped_top().to_vec(), "stepped_top")
                                    }
                                    _ => (vec![0.0; n], "zeros"),
                                }
                            }
                        };
                        let baseline = certify_fused_phi_total(&a, h, &w, &baseline_candidate);
                        let baseline_record = match &baseline {
                            Ok(cert) => json!({
                                "outcome": "certificate", "error": null,
                                "candidate_source": baseline_source,
                                "candidate": hexes(&baseline_candidate),
                                "certificate": inner_record(cert),
                            }),
                            Err(error) => json!({
                                "outcome": "error", "error": error.to_string(),
                                "candidate_source": baseline_source,
                                "candidate": hexes(&baseline_candidate),
                                "certificate": null,
                            }),
                        };
                        match &normalized {
                            Ok(cert) => println!(
                                "{label}: e {} {:?} bound {:e} (sigma E {:e}, R {:e}, L {:e}; {:?}) caller {:?}; baseline {:?}",
                                cert.exponent(),
                                cert.status(),
                                cert.bound(),
                                cert.scaled_certificate(),
                                cert.rescale_rounding(),
                                cert.normalization_rounding(),
                                cert.dominant(),
                                cert.caller_bound(),
                                baseline.as_ref().map(|b| (b.status(), b.bound())).ok(),
                            ),
                            Err(error) => println!("{label}: {error}"),
                        }
                        cases.push(json!({
                            "label": label,
                            "family": family,
                            "variant": variant,
                            "w_exponents": exponents,
                            "matrix_index": index,
                            "seed": seed,
                            "symmetric": symmetric,
                            "n": n,
                            "target_hA_one_norm": norm,
                            "h": hex(h),
                            "h_power_of_two": h == 0.25,
                            "a": a.iter().map(|row| hexes(row)).collect::<Vec<_>>(),
                            "base_w": base_w.iter().map(|wk| hexes(wk)).collect::<Vec<_>>(),
                            "w": w.iter().map(|wk| hexes(wk)).collect::<Vec<_>>(),
                            "taylor_original": taylor_record,
                            "normalized": normalized_record,
                            "baseline": baseline_record,
                        }));
                    }
                }
            }
        }
    }
    let count = cases.len();
    let table = scaling_table();
    std::fs::write(
        &target,
        serde_json::to_string(&json!({
            "schema": "vigilode-py04-cases-v1",
            "node": "research/py04_fused_normalization_20261011",
            "source_commit": source_commit,
            "tree_status": tree_status,
            "taylor_budget": hex(TAYLOR_BUDGET),
            "normalized_taylor_budget": hex(NORMALIZED_TAYLOR_BUDGET),
            "seed_base": SEED_BASE,
            "table_seed": TABLE_SEED,
            "cases": cases,
            "scaling_table": table,
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();
    println!("wrote {} ({count} cases)", target.display());
}
