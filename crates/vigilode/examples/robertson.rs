//! Robertson's stiff chemical kinetics with an analytic dense Jacobian.
//!
//! Check: total mass y0 + y1 + y2 is conserved to 1e-8 and the run succeeds.

use vigilode::{Atol, CallbackError, Jacobian, Options, Problem, Rodas5p};

fn main() -> Result<(), vigilode::SolveError> {
    let problem = Problem::new(3, |_t, y, dy| {
        dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
        dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
        dy[2] = 3.0e7 * y[1] * y[1];
        Ok::<(), CallbackError>(())
    })
    .jacobian(Jacobian::dense(|_t, y, jac| {
        // Row major: jac[i * 3 + j] = d f_i / d y_j.
        jac.copy_from_slice(&[
            -0.04,
            1.0e4 * y[2],
            1.0e4 * y[1],
            0.04,
            -1.0e4 * y[2] - 6.0e7 * y[1],
            -1.0e4 * y[1],
            0.0,
            6.0e7 * y[1],
            0.0,
        ]);
        Ok(())
    }))
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
    let drift = (y[0] + y[1] + y[2] - 1.0).abs();
    assert!(drift < 1e-8, "mass drift {drift:e} exceeds 1e-8");
    println!("robertson: y(40) = {y:?}, mass drift {drift:.2e}");
    Ok(())
}
