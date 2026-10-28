//! Local exact probes of research node
//! `research/safe_enclosure_composition_20261004` (RVJ DAG node
//! SAFE-ENCLOSURE). Exports, as IEEE bits, the base and repaired values of
//! three compositions; `tools/safe_enclosure_check.py` checks them at 60
//! digits. The base C2 and C3 values are the base closures of
//! `certify_exp_action_stepped`, copied verbatim (they were not functions).

use rodas5p_core::directed::{Interval, exp_interval, mul_down, mul_up, sub_up};
use rodas5p_core::nonnormal_certificate::{decay_upper, midpoint_radius};
use rodas5p_core::polynomial_action::{exp_neg_enclosure, exp_neg_interval_enclosure};
use serde_json::{Value, json};

struct SplitMix(u64);

impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    /// Uniform in [0, 1).
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn log_uniform(&mut self, lo: f64, hi: f64) -> f64 {
        (lo.ln() + self.unit() * (hi.ln() - lo.ln())).exp()
    }
}

fn hx(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}

/// Base C2: `mul_up(mul_up(count, h), a)` then the capped exponential.
fn base_decay(count: usize, h: f64, a: f64) -> Option<f64> {
    let exponent = mul_up(mul_up(count as f64, h).ok()?, a).ok()?;
    if exponent <= 0.0 {
        Some(exp_interval(exponent).map(|e| e.hi).unwrap_or(1.0).min(1.0))
    } else {
        exp_interval(exponent).ok().map(|e| e.hi)
    }
}

/// Base C3: the rounded midpoint and the rounded-up half width.
fn base_midpoint(lo: f64, hi: f64) -> Option<(f64, f64)> {
    let m = 0.5 * lo + 0.5 * hi;
    Some((m, mul_up(sub_up(hi, lo).ok()?, 0.5).ok()?))
}

fn probe_c1() -> Vec<Value> {
    let mut rng = SplitMix(0x5afe_e1c1);
    let mut pairs = Vec::new();
    while pairs.len() < 20_000 {
        let kappa = rng.log_uniform(1.0e-3, 1.0e3);
        let tau = rng.log_uniform(1.0e-6, 1.0);
        if kappa * tau <= 700.0 {
            pairs.push((kappa, tau));
        }
    }
    // Hand pairs: kappa tau inexact near 1, 10, 100, 700.
    for target in [1.0, 10.0, 100.0, 700.0] {
        for k in 0..50 {
            let kappa = 3.0 + 0.1 * k as f64 + 1.0 / 3.0;
            let tau = target / kappa * (1.0 - 1.0e-15 * k as f64);
            pairs.push((kappa, tau));
        }
    }
    pairs
        .into_iter()
        .map(|(kappa, tau)| {
            let base = exp_neg_enclosure(mul_up(kappa, tau).unwrap()).unwrap();
            let repaired = exp_neg_interval_enclosure(
                Interval::new(mul_down(kappa, tau).unwrap(), mul_up(kappa, tau).unwrap()).unwrap(),
            )
            .unwrap();
            json!([
                hx(kappa),
                hx(tau),
                hx(base.lo),
                hx(base.hi),
                hx(repaired.lo),
                hx(repaired.hi)
            ])
        })
        .collect()
}

fn probe_c2() -> Vec<Value> {
    let mut rng = SplitMix(0x5afe_e1c2);
    let mut rows = Vec::new();
    while rows.len() < 20_000 {
        let tau = rng.log_uniform(1.0e-3, 10.0);
        let steps = 1usize << (rng.next() % 13);
        let h = tau / steps as f64;
        let count = (rng.next() % 4097) as usize;
        let a = if rng.next().is_multiple_of(4) {
            rng.log_uniform(1.0e-6, 10.0)
        } else {
            -rng.log_uniform(1.0e-6, 1.0e3)
        };
        let (Some(base), Ok(repaired)) = (base_decay(count, h, a), decay_upper(count, h, a)) else {
            continue;
        };
        rows.push(json!([count, hx(h), hx(a), hx(base), hx(repaired)]));
    }
    rows
}

fn probe_c3() -> Vec<Value> {
    let mut rng = SplitMix(0x5afe_e1c3);
    let mut rows = Vec::new();
    let push = |lo: f64, hi: f64, rows: &mut Vec<Value>| {
        if !(lo.is_finite() && hi.is_finite() && lo <= hi) {
            return;
        }
        let (Some((bm, br)), Ok((rm, rr))) = (
            base_midpoint(lo, hi),
            midpoint_radius(Interval::new(lo, hi).unwrap()),
        ) else {
            return;
        };
        rows.push(json!([hx(lo), hx(hi), hx(bm), hx(br), hx(rm), hx(rr)]));
    };
    while rows.len() < 20_000 {
        let kind = rng.next() % 4;
        let sign = if rng.next().is_multiple_of(2) {
            1.0
        } else {
            -1.0
        };
        match kind {
            0 => {
                // Narrow: hi = lo + k ulp.
                let lo = sign * rng.log_uniform(1.0e-300, 1.0e300);
                let mut hi = lo;
                for _ in 0..(1 + rng.next() % 64) {
                    hi = hi.next_up();
                }
                push(lo, hi, &mut rows);
            }
            1 => {
                // Wide, same sign.
                let a = sign * rng.log_uniform(1.0e-10, 1.0e10);
                let b = sign * rng.log_uniform(1.0e-10, 1.0e10);
                push(a.min(b), a.max(b), &mut rows);
            }
            2 => {
                // Sign-straddling.
                let lo = -rng.log_uniform(1.0e-10, 1.0e10);
                let hi = rng.log_uniform(1.0e-10, 1.0e10);
                push(lo, hi, &mut rows);
            }
            _ => {
                // Subnormal-adjacent.
                let lo = sign * f64::MIN_POSITIVE * rng.unit() * 4.0;
                let mut hi = lo;
                for _ in 0..(1 + rng.next() % 64) {
                    hi = hi.next_up();
                }
                push(lo, hi, &mut rows);
            }
        }
    }
    rows
}

#[test]
#[ignore = "export of research/safe_enclosure_composition_20261004"]
fn export_probes() {
    let out = json!({
        "schema": "vigilode-safe-enclosure-probes-v1",
        "c1": {"columns": ["kappa", "tau", "base_lo", "base_hi", "repaired_lo", "repaired_hi"], "rows": probe_c1()},
        "c2": {"columns": ["count", "h", "a", "base_upper", "repaired_upper"], "rows": probe_c2()},
        "c3": {"columns": ["lo", "hi", "base_m", "base_r", "repaired_m", "repaired_r"], "rows": probe_c3()},
    });
    if let Ok(path) = std::env::var("SAFE_ENCLOSURE_PROBES") {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        assert!(
            !path.exists(),
            "immutable output exists: {}",
            path.display()
        );
        std::fs::write(&path, serde_json::to_string(&out).unwrap() + "\n").unwrap();
        println!("wrote {}", path.display());
    }
}

/// The repaired compositions on hand cases where the base ones under-cover
/// by construction of the rounding (contract, not the recorded probe).
#[test]
fn repaired_midpoint_radius_covers_rounded_midpoint() {
    let lo = 1.0_f64;
    let hi = lo.next_up().next_up().next_up();
    let (m, r) = midpoint_radius(Interval::new(lo, hi).unwrap()).unwrap();
    assert!(m - lo <= r && hi - m <= r);
}

#[test]
fn repaired_decay_uses_lower_time_for_negative_rate() {
    // count * h rounds; for a < 0 the bound must be at least e^{t a} at the
    // lower time, so it is never below the base value.
    let h = 0.1;
    for count in 1..2000 {
        let base = base_decay(count, h, -3.0).unwrap();
        let repaired = decay_upper(count, h, -3.0).unwrap();
        assert!(repaired >= base, "count {count}");
    }
}
