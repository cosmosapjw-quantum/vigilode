//! Robertson as an index-1 DAE with mass matrix diag(1, 1, 0).
//!
//! The third equation is the algebraic constraint y0 + y1 + y2 - 1 = 0.
//! Check: the constraint residual at the final state is at most 1e-12.

use vigilode::{Atol, CallbackError, Jacobian, Options, Problem, Rodas5p};

fn main() -> Result<(), vigilode::SolveError> {
    let problem = Problem::new(3, |_t, y, dy| {
        dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
        dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
        dy[2] = y[0] + y[1] + y[2] - 1.0;
        Ok::<(), CallbackError>(())
    })
    .jacobian(Jacobian::dense(|_t, y, jac| {
        jac.copy_from_slice(&[
            -0.04,
            1.0e4 * y[2],
            1.0e4 * y[1],
            0.04,
            -1.0e4 * y[2] - 6.0e7 * y[1],
            -1.0e4 * y[1],
            1.0,
            1.0,
            1.0,
        ]);
        Ok(())
    }))
    .mass(vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0])
    .autonomous();

    let opts = Options {
        rtol: 1e-6,
        atol: Atol::Scalar(1e-10),
        ..Default::default()
    };
    let sol = Rodas5p::solve(&problem, (0.0, 40.0), &[1.0, 0.0, 0.0], &opts)?;
    assert!(sol.is_success(), "run did not reach t1");
    let y = sol
        .final_state()
        .expect("a successful run has a final state");
    let residual = (y[0] + y[1] + y[2] - 1.0).abs();
    assert!(
        residual <= 1e-12,
        "constraint residual {residual:e} exceeds 1e-12"
    );
    println!("dae_mass: y(40) = {y:?}, constraint residual {residual:.2e}");
    Ok(())
}
