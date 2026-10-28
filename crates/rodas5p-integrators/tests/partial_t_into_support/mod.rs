//! Non-autonomous fixtures shared by `partial_t_into_contracts.rs` (and by
//! the one-off golden probe run against the pre-change tree): a
//! Prothero-Robinson system `y_i' = l_i (y_i - phi_i(t)) + phi_i'(t)` with
//! `phi_i(t) = sin(t + i)`, with an analytic `f_t` or the finite-difference
//! fallback, and a digest of a whole fast-driver run.

use std::sync::Arc;

use rodas5p_core::DenseMatrix;
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSchedule, integrate_rodas5p_fast_observed,
};

pub const RATES: [f64; 3] = [-1.0e4, -50.0, -1.0];

type PartialT = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> rodas5p_core::CoreResult<()> + Send + Sync>;

/// The fixture; `analytic_ft` supplies `partial_t`, otherwise the problem
/// relies on the library's finite-difference `f_t`.
pub fn prothero_robinson(analytic_ft: bool) -> OdeProblem {
    let partial_t: Option<PartialT> = if analytic_ft {
        Some(Arc::new(|t, _, out| {
            for (i, (o, l)) in out.iter_mut().zip(RATES).enumerate() {
                let s = t + i as f64;
                *o = -l * s.cos() - s.sin();
            }
            Ok(())
        }))
    } else {
        None
    };
    OdeProblem::new(
        "partial-t-prothero-robinson",
        3,
        Arc::new(|t, y, out| {
            for (i, ((o, l), yi)) in out.iter_mut().zip(RATES).zip(y).enumerate() {
                let s = t + i as f64;
                *o = l * (yi - s.sin()) + s.cos();
            }
            Ok(())
        }),
        None,
        Some(Arc::new(|_, _| {
            let mut m = DenseMatrix::zeros(3, 3);
            for (i, l) in RATES.iter().enumerate() {
                m[(i, i)] = *l;
            }
            Ok(m)
        })),
        None,
        partial_t,
        false,
        None,
        None,
    )
    .unwrap()
    .with_jacobian_into(Arc::new(|_, _, m| {
        for (i, l) in RATES.iter().enumerate() {
            m[(i, i)] = *l;
        }
        Ok(())
    }))
}

pub fn y0() -> Vec<f64> {
    (0..3).map(|i| (i as f64).sin()).collect()
}

pub fn config(rtol: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 1.0e-3 * rtol,
        rtol,
        initial_step: 1.0e-4,
        max_attempts: 1_000_000,
        ..AdaptiveStepConfig::default()
    }
}

pub fn schedule() -> OutputSchedule {
    OutputSchedule::new((0..=8).map(|k| 0.25 * k as f64).collect()).unwrap()
}

fn fnv(hash: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *hash ^= u64::from(*b);
        *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

/// FNV-1a of every output time and state bit pattern, the serialized work
/// counters and the attempt/accept/reject counts of one fast-driver run.
pub fn run_digest(analytic_ft: bool, rtol: f64) -> u64 {
    let problem = prothero_robinson(analytic_ft);
    let fast =
        integrate_rodas5p_fast_observed(&problem, (0.0, 2.0), &y0(), &config(rtol), &schedule())
            .unwrap();
    assert!(fast.observed.success, "{}", fast.observed.message);
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for (t, y) in fast.observed.t.iter().zip(&fast.observed.y) {
        fnv(&mut hash, &t.to_bits().to_le_bytes());
        for v in y {
            fnv(&mut hash, &v.to_bits().to_le_bytes());
        }
    }
    fnv(
        &mut hash,
        serde_json::to_string(&fast.observed.counters)
            .unwrap()
            .as_bytes(),
    );
    for c in [fast.attempts, fast.accepted_steps, fast.rejected_steps] {
        fnv(&mut hash, &(c as u64).to_le_bytes());
    }
    hash
}
