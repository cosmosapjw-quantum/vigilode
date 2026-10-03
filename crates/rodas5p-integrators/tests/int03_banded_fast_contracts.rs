//! Contracts of the banded fast RODAS5P pipeline (research node
//! `research/int03_banded_fast_20261003`, integrated DAG node INT-03): the
//! banded LU against dense partial pivoting, a singular band, a non-finite
//! provider, and the input checks.

use std::sync::Arc;

use rodas5p_core::{CoreError, DenseMatrix};
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, OdeProblem, OutputSchedule, constant_affine_mass_problem,
    integrate_rodas5p_fast_banded_observed, rodas5p_fast_banded_solve,
};

fn splitmix(state: &mut u64) -> f64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    2.0 * ((z >> 11) as f64 / (1u64 << 53) as f64) - 1.0
}

/// Dense partial-pivoting LU solve (first maximum wins), and its pivots.
#[allow(clippy::needless_range_loop)] // the textbook index form
fn dense_solve(a: &[Vec<f64>], b: &[f64]) -> (Vec<f64>, Vec<usize>) {
    let n = b.len();
    let mut m = a.to_vec();
    let mut x = b.to_vec();
    let mut pivots = vec![0; n];
    for k in 0..n {
        let mut p = k;
        for i in k + 1..n {
            if m[i][k].abs() > m[p][k].abs() {
                p = i;
            }
        }
        pivots[k] = p;
        m.swap(k, p);
        x.swap(k, p);
        for i in k + 1..n {
            let l = m[i][k] / m[k][k];
            for j in k..n {
                m[i][j] -= l * m[k][j];
            }
            x[i] -= l * x[k];
        }
    }
    for i in (0..n).rev() {
        let mut sum = x[i];
        for j in i + 1..n {
            sum -= m[i][j] * x[j];
        }
        x[i] = sum / m[i][i];
    }
    (x, pivots)
}

#[test]
fn the_banded_lu_matches_dense_partial_pivoting() {
    let mut seed = 77_u64;
    let mut swaps = 0;
    for n in [1usize, 2, 5, 17, 64] {
        for (l, u) in [(0usize, 0usize), (1, 1), (2, 1), (0, 3), (3, 0), (2, 2)] {
            let (l, u) = (l.min(n - 1), u.min(n - 1));
            let width = l + u + 1;
            // A small diagonal so that rows below win the pivot search.
            let mut a = vec![vec![0.0; n]; n];
            let mut band = vec![0.0; n * width];
            for i in 0..n {
                for j in i.saturating_sub(l)..=(i + u).min(n - 1) {
                    let v = if i == j {
                        0.1 * splitmix(&mut seed) + 0.05
                    } else {
                        splitmix(&mut seed)
                    };
                    a[i][j] = v;
                    // W = -J with inv = 0: pass J = -A.
                    band[i * width + (j + l - i)] = -v;
                }
            }
            let b: Vec<f64> = (0..n).map(|_| splitmix(&mut seed)).collect();
            let (expected, dense_pivots) = dense_solve(&a, &b);
            let (x, pivots) = rodas5p_fast_banded_solve(n, l, u, &band, 0.0, &b).unwrap();
            assert_eq!(pivots, dense_pivots, "n={n} l={l} u={u}");
            swaps += pivots.iter().enumerate().filter(|(k, p)| *k != **p).count();
            let scale = expected.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
            for (got, want) in x.iter().zip(&expected) {
                assert!(
                    (got - want).abs() <= 1.0e-12 * scale.max(1.0),
                    "n={n} l={l} u={u}: {got} vs {want}"
                );
            }
        }
    }
    assert!(swaps > 20, "{swaps} row interchanges");
}

#[test]
fn a_singular_band_is_a_linear_solve_error() {
    // Column 1 is zero.
    let (n, l, u) = (3, 1, 1);
    let a = [[2.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 0.0, 3.0]];
    let mut band = vec![0.0; n * (l + u + 1)];
    for i in 0..n {
        for j in i.saturating_sub(l)..=(i + u).min(n - 1) {
            band[i * (l + u + 1) + (j + l - i)] = -a[i][j];
        }
    }
    assert!(matches!(
        rodas5p_fast_banded_solve(n, l, u, &band, 0.0, &[1.0, 1.0, 1.0]),
        Err(CoreError::LinearSolve(_))
    ));
}

/// `y' = -2 y_i + y_{i-1} + y_{i+1}` (tridiagonal), and its band.
fn tridiagonal(n: usize) -> (OdeProblem, BandedJacobian) {
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        for i in 0..n {
            let left = if i > 0 { y[i - 1] } else { 0.0 };
            let right = if i + 1 < n { y[i + 1] } else { 0.0 };
            out[i] = -2.0 * y[i] + left + right;
        }
        Ok(())
    });
    let jacobian = Arc::new(move |_t: f64, _y: &[f64]| {
        let mut j = DenseMatrix::zeros(n, n);
        for i in 0..n {
            j[(i, i)] = -2.0;
            if i > 0 {
                j[(i, i - 1)] = 1.0;
            }
            if i + 1 < n {
                j[(i, i + 1)] = 1.0;
            }
        }
        Ok(j)
    });
    let problem = OdeProblem::new(
        "tridiagonal",
        n,
        rhs,
        None,
        Some(jacobian),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let band = BandedJacobian {
        lower: 1,
        upper: 1,
        fill: Arc::new(|_t, _y, band: &mut [f64]| {
            for row in band.chunks_exact_mut(3) {
                row.copy_from_slice(&[1.0, -2.0, 1.0]);
            }
            Ok(())
        }),
    };
    (problem, band)
}

fn adaptive() -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 1e-6,
        rtol: 1e-6,
        initial_step: 1e-3,
        max_attempts: 50,
        ..AdaptiveStepConfig::default()
    }
}

#[test]
fn a_provider_that_writes_nan_rejects_its_attempts() {
    let (problem, mut band) = tridiagonal(8);
    band.fill = Arc::new(|_t, _y, band: &mut [f64]| {
        band[4] = f64::NAN;
        Ok(())
    });
    let schedule = OutputSchedule::new(vec![0.0, 1.0]).unwrap();
    let result = integrate_rodas5p_fast_banded_observed(
        &problem,
        &band,
        (0.0, 1.0),
        &[1.0; 8],
        &adaptive(),
        &schedule,
    )
    .unwrap();
    assert!(!result.fast.observed.success);
    assert!(result.fast.rejected_steps >= 1 && result.fast.accepted_steps == 0);
    let c = result.fast.observed.counters;
    assert!(c.linear_solve_failures + c.nonfinite_step_failures >= 1);
}

#[test]
fn bad_inputs_are_rejected_before_integration() {
    let (problem, band) = tridiagonal(8);
    let schedule = OutputSchedule::new(vec![0.0, 1.0]).unwrap();
    let run = |p: &OdeProblem, b: &BandedJacobian, y0: &[f64]| {
        integrate_rodas5p_fast_banded_observed(p, b, (0.0, 1.0), y0, &adaptive(), &schedule)
    };
    assert!(run(&problem, &band, &[1.0; 8]).is_ok());
    assert!(run(&problem, &band, &[1.0; 7]).is_err());
    for (lower, upper) in [(8, 1), (1, 8), (9, 9)] {
        let wide = BandedJacobian {
            lower,
            upper,
            fill: band.fill.clone(),
        };
        assert!(run(&problem, &wide, &[1.0; 8]).is_err());
    }
    let (mass, mass_y0, _, _) = constant_affine_mass_problem();
    let n = mass.dimension;
    let mass_band = BandedJacobian {
        lower: 0,
        upper: n - 1,
        fill: Arc::new(|_t, _y, _band: &mut [f64]| Ok(())),
    };
    assert!(run(&mass, &mass_band, &mass_y0).is_err());
}
