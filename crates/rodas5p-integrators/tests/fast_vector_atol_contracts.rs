//! Contracts of the per-component absolute tolerance of the fast drivers
//! (wave-3 node `p0-atol`): a uniform vector is the scalar run bit for bit,
//! a nonuniform vector controls the error in its own weights, invalid
//! vectors are refused, and a zero entry follows the zero-scale rule.

use std::sync::Arc;

use rodas5p_core::{CoreError, DenseMatrix};
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, OdeProblem, OutputSchedule,
    integrate_rodas5p_fast_banded_observed, integrate_rodas5p_fast_banded_observed_with_atol,
    integrate_rodas5p_fast_observed_with_atol, integrate_rodas5p_fast_observed_with_partial,
    robertson_problem,
};

#[path = "b1_problems/mod.rs"]
mod b1_problems;

const RTOL: f64 = 1.0e-6;

fn config(atol: f64, rtol: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol,
        rtol,
        initial_step: 1.0e-3,
        max_attempts: 200_000,
        ..AdaptiveStepConfig::default()
    }
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// The 1D Brusselator's band (u and v interleaved: lower = upper = 2).
fn brusselator_band(cells: usize) -> BandedJacobian {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    BandedJacobian {
        lower: 2,
        upper: 2,
        fill: Arc::new(move |_t, y: &[f64], band: &mut [f64]| {
            let n = 2 * cells;
            let at = |band: &mut [f64], i: usize, j: usize, v: f64| {
                band[i * 5 + (j + 2 - i)] = v;
            };
            for i in 0..cells {
                let (u, v) = (y[2 * i], y[2 * i + 1]);
                let (a, b) = (2 * i, 2 * i + 1);
                at(band, a, a, 2.0 * u * v - 4.0 - 2.0 * c);
                at(band, a, b, u * u);
                at(band, b, a, 3.0 - 2.0 * u * v);
                at(band, b, b, -u * u - 2.0 * c);
                if i > 0 {
                    at(band, a, a - 2, c);
                    at(band, b, b - 2, c);
                }
                if i + 1 < cells {
                    at(band, a, a + 2, c);
                    at(band, b, b + 2, c);
                }
            }
            debug_assert_eq!(n, 2 * cells);
            Ok(())
        }),
    }
}

#[test]
fn uniform_vector_equals_scalar_bitwise() {
    // Dense: Robertson.
    let (problem, y0) = robertson_problem().unwrap();
    let schedule = OutputSchedule::new(vec![0.0, 40.0]).unwrap();
    let adaptive = config(1.0e-10, RTOL);
    let (scalar, _) = integrate_rodas5p_fast_observed_with_partial(
        &problem,
        (0.0, 40.0),
        &y0,
        &adaptive,
        &schedule,
    )
    .unwrap();
    let (vector, _) = integrate_rodas5p_fast_observed_with_atol(
        &problem,
        (0.0, 40.0),
        &y0,
        &adaptive,
        &schedule,
        &[1.0e-10; 3],
    )
    .unwrap();
    assert!(scalar.observed.success && vector.observed.success);
    assert_eq!(bits(&scalar.observed.y[1]), bits(&vector.observed.y[1]));
    assert_eq!(scalar.observed.counters, vector.observed.counters);
    assert_eq!(scalar.attempts, vector.attempts);
    assert_eq!(scalar.accepted_steps, vector.accepted_steps);
    assert_eq!(scalar.rejected_steps, vector.rejected_steps);
    assert_eq!(scalar.jacobian_reuses, vector.jacobian_reuses);

    // Banded: Brusselator, 50 cells.
    let cells = 50;
    let (problem, y0) = b1_problems::brusselator_problem(cells).unwrap();
    let band = brusselator_band(cells);
    let schedule = OutputSchedule::new(vec![0.0, 2.0]).unwrap();
    let adaptive = config(1.0e-8, RTOL);
    let scalar = integrate_rodas5p_fast_banded_observed(
        &problem,
        &band,
        (0.0, 2.0),
        &y0,
        &adaptive,
        &schedule,
    )
    .unwrap();
    let vector = integrate_rodas5p_fast_banded_observed_with_atol(
        &problem,
        &band,
        (0.0, 2.0),
        &y0,
        &adaptive,
        &schedule,
        &vec![1.0e-8; 2 * cells],
    )
    .unwrap();
    assert!(scalar.fast.observed.success && vector.fast.observed.success);
    assert_eq!(
        bits(&scalar.fast.observed.y[1]),
        bits(&vector.fast.observed.y[1])
    );
    assert_eq!(scalar.fast.observed.counters, vector.fast.observed.counters);
    assert_eq!(scalar.fast.attempts, vector.fast.attempts);
    assert_eq!(scalar.fast.rejected_steps, vector.fast.rejected_steps);
    assert_eq!(scalar.work, vector.work);
}

/// WRMS of `e` in the weights `atol_i + rtol max(|a_i|, |b_i|)`.
fn weighted_rms(e: &[f64], a: &[f64], b: &[f64], atol: &[f64], rtol: f64) -> f64 {
    let sum: f64 = (0..e.len())
        .map(|i| (e[i] / (atol[i] + rtol * a[i].abs().max(b[i].abs()))).powi(2))
        .sum();
    (sum / e.len() as f64).sqrt()
}

#[test]
fn robertson_vector_atol_accuracy() {
    let (problem, y0) = robertson_problem().unwrap();
    let schedule = OutputSchedule::new(vec![0.0, 40.0]).unwrap();
    let atol = [1.0e-8, 1.0e-14, 1.0e-8];
    let (run, _) = integrate_rodas5p_fast_observed_with_atol(
        &problem,
        (0.0, 40.0),
        &y0,
        // `adaptive.atol` is unused when a vector is given: make it loose.
        &config(1.0e-6, RTOL),
        &schedule,
        &atol,
    )
    .unwrap();
    assert!(run.observed.success);
    let (reference, _) = integrate_rodas5p_fast_observed_with_partial(
        &problem,
        (0.0, 40.0),
        &y0,
        &config(1.0e-14, 1.0e-11),
        &schedule,
    )
    .unwrap();
    let (y, r) = (&run.observed.y[1], &reference.observed.y[1]);
    let e: Vec<f64> = y.iter().zip(r).map(|(a, b)| a - b).collect();
    let units = weighted_rms(&e, y, r, &atol, RTOL);
    assert!(units <= 2.0, "endpoint error {units} tolerance units");
    // The tiny atol on the intermediate species costs steps against a
    // loose scalar run (the weights matter).
    let (loose, _) = integrate_rodas5p_fast_observed_with_partial(
        &problem,
        (0.0, 40.0),
        &y0,
        &config(1.0e-8, RTOL),
        &schedule,
    )
    .unwrap();
    assert!(run.accepted_steps > loose.accepted_steps);
}

#[test]
fn invalid_vector_rejected() {
    let (problem, y0) = robertson_problem().unwrap();
    let schedule = OutputSchedule::new(vec![0.0, 1.0]).unwrap();
    let adaptive = config(1.0e-9, RTOL);
    for bad in [
        vec![1.0e-9; 2],
        vec![1.0e-9; 4],
        vec![],
        vec![1.0e-9, f64::NAN, 1.0e-9],
        vec![1.0e-9, f64::INFINITY, 1.0e-9],
        vec![1.0e-9, -1.0e-9, 1.0e-9],
    ] {
        let dense = integrate_rodas5p_fast_observed_with_atol(
            &problem,
            (0.0, 1.0),
            &y0,
            &adaptive,
            &schedule,
            &bad,
        );
        assert!(matches!(dense, Err(CoreError::InvalidInput(_))), "{bad:?}");
    }
    let cells = 4;
    let (problem, y0) = b1_problems::brusselator_problem(cells).unwrap();
    let band = brusselator_band(cells);
    for bad in [vec![1.0e-9; 7], vec![f64::NAN; 8], vec![-1.0; 8]] {
        let banded = integrate_rodas5p_fast_banded_observed_with_atol(
            &problem,
            &band,
            (0.0, 1.0),
            &y0,
            &adaptive,
            &schedule,
            &bad,
        );
        assert!(matches!(banded, Err(CoreError::InvalidInput(_))), "{bad:?}");
    }
}

/// `y0' = -y0`, `y1' = k y1` (`k = 0` makes `y1` identically zero from 0).
fn pair(k: f64) -> OdeProblem {
    OdeProblem::new(
        "vector-atol-pair",
        2,
        Arc::new(move |_, y, out| {
            out[0] = -y[0];
            out[1] = k * y[1];
            Ok(())
        }),
        None,
        Some(Arc::new(move |_, _| {
            DenseMatrix::new(2, 2, vec![-1.0, 0.0, 0.0, k])
        })),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

#[test]
fn zero_component_follows_zero_scale_rule() {
    // rtol = 0 makes the scale of component 1 exactly atol[1] = 0. The
    // facade-level `adaptive.atol` is validated but unused.
    let schedule = OutputSchedule::new(vec![0.0, 1.0]).unwrap();
    let adaptive = config(1.0e-6, 0.0);
    // y1 is identically zero: zero scale, zero error, contributes 0.
    let (ok, _) = integrate_rodas5p_fast_observed_with_atol(
        &pair(0.0),
        (0.0, 1.0),
        &[1.0, 0.0],
        &adaptive,
        &schedule,
        &[1.0e-6, 0.0],
    )
    .unwrap();
    assert!(ok.observed.success);
    assert_eq!(ok.observed.y[1][1].to_bits() & !(1 << 63), 0);
    // y1 decays with a nonzero embedded error and a zero scale: +inf on
    // every attempt, so nothing is accepted.
    let (bad, partial) = integrate_rodas5p_fast_observed_with_atol(
        &pair(-1.0),
        (0.0, 1.0),
        &[1.0, 1.0],
        &AdaptiveStepConfig {
            max_attempts: 40,
            ..adaptive
        },
        &schedule,
        &[1.0e-6, 0.0],
    )
    .unwrap();
    assert!(!bad.observed.success);
    assert_eq!(bad.accepted_steps, 0);
    assert!(bad.observed.counters.local_error_failures >= 1);
    assert!(partial.is_some());
}
