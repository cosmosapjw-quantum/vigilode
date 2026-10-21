//! Prothero-Robinson with dense output and `Solution::at`.
//!
//! y' = lambda (y - sin t) + cos t, y(0) = 0, exact solution y = sin t.
//! Check: at points strictly inside accepted steps, the continuous extension
//! is within 2 tolerance units, atol + rtol |y|, of the exact solution.

use vigilode::{Atol, CallbackError, Jacobian, Options, Problem, Rodas5p};

fn main() -> Result<(), vigilode::SolveError> {
    let lambda = -1.0e3_f64;
    let problem = Problem::new(1, move |t, y, dy| {
        dy[0] = lambda * (y[0] - t.sin()) + t.cos();
        Ok::<(), CallbackError>(())
    })
    .jacobian(Jacobian::dense(move |_t, _y, jac| {
        jac[0] = lambda;
        Ok(())
    }));

    let (rtol, atol) = (1e-6, 1e-9);
    let opts = Options {
        rtol,
        atol: Atol::Scalar(atol),
        dense_output: true,
        ..Default::default()
    };
    let sol = Rodas5p::solve(&problem, (0.0, 3.0), &[0.0], &opts)?;
    assert!(sol.is_success(), "run did not reach t1");
    assert!(sol.t.len() > 2, "dense output keeps every accepted step");

    let mut worst: f64 = 0.0;
    for pair in sol.t.windows(2) {
        let t = 0.5 * (pair[0] + pair[1]);
        let y = sol.at(t).expect("t lies inside the integrated span");
        let exact = t.sin();
        let units = (y[0] - exact).abs() / (atol + rtol * exact.abs());
        worst = worst.max(units);
    }
    assert!(
        worst <= 2.0,
        "interior error {worst:.3} tolerance units exceeds 2"
    );
    println!(
        "dense_output: {} steps, worst interior error {worst:.3} tolerance units",
        sol.t.len() - 1
    );
    Ok(())
}
