//! 1D Brusselator, n = 100 (50 cells, u and v interleaved), with a
//! finite-difference Jacobian and a declared band (2, 2).
//!
//! Check: the banded run succeeds, uses far fewer right-hand-side calls for
//! Jacobians than a dense finite-difference Jacobian would, and agrees with
//! the same problem solved without the band declaration to 1e-4.

use vigilode::{CallbackError, Jacobian, Options, Problem, Rodas5p};

const CELLS: usize = 50;

fn brusselator() -> Problem {
    let (a, b) = (1.0_f64, 3.0_f64);
    let h = 1.0 / (CELLS as f64 + 1.0);
    let alpha = 0.02 / (h * h);
    Problem::new(2 * CELLS, move |_t, y, dy| {
        for i in 0..CELLS {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            // Dirichlet boundaries u = a, v = b / a.
            let (ul, vl) = if i == 0 {
                (a, b / a)
            } else {
                (y[2 * i - 2], y[2 * i - 1])
            };
            let (ur, vr) = if i + 1 == CELLS {
                (a, b / a)
            } else {
                (y[2 * i + 2], y[2 * i + 3])
            };
            dy[2 * i] = a + u * u * v - (b + 1.0) * u + alpha * (ul - 2.0 * u + ur);
            dy[2 * i + 1] = b * u - u * u * v + alpha * (vl - 2.0 * v + vr);
        }
        Ok::<(), CallbackError>(())
    })
    .jacobian(Jacobian::FiniteDifference)
    .autonomous()
}

fn main() -> Result<(), vigilode::SolveError> {
    let y0: Vec<f64> = (0..CELLS)
        .flat_map(|i| {
            let x = (i as f64 + 1.0) / (CELLS as f64 + 1.0);
            [1.0 + (2.0 * std::f64::consts::PI * x).sin(), 3.0]
        })
        .collect();
    let opts = Options {
        rtol: 1e-6,
        ..Default::default()
    };

    let banded = Rodas5p::solve(&brusselator().banded(2, 2), (0.0, 1.0), &y0, &opts)?;
    let dense = Rodas5p::solve(&brusselator(), (0.0, 1.0), &y0, &opts)?;
    assert!(
        banded.is_success() && dense.is_success(),
        "a run did not reach t1"
    );

    let (yb, yd) = (banded.final_state().unwrap(), dense.final_state().unwrap());
    let worst = yb
        .iter()
        .zip(yd)
        .map(|(p, q)| (p - q).abs())
        .fold(0.0_f64, f64::max);
    assert!(worst < 1e-4, "banded and dense runs differ by {worst:e}");

    let fd_banded = banded.stats.finite_difference.rhs_calls;
    let fd_dense = dense.stats.finite_difference.rhs_calls;
    assert!(
        fd_banded < fd_dense,
        "banding must reduce Jacobian rhs calls"
    );
    println!(
        "banded_fd: max difference {worst:.2e}; Jacobian rhs calls {fd_banded} vs {fd_dense} dense"
    );
    Ok(())
}
