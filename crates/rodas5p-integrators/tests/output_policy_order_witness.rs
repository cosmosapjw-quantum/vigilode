//! Exact-start local defects; not method-order inference from adaptive tolerance.
use rodas5p_core::{LinearSolverConfig, WorkCounters};
use rodas5p_integrators::{rodas5p_dense_output, scalar_linear_problem, sequential_step};
fn slopes(e: &[f64]) -> Vec<f64> {
    e.windows(2).map(|x| (x[0] / x[1]).log2()).collect()
}
#[test]
fn dense_power_five_rejects_linear_interpolation_negative_control() {
    let (problem, y0) = scalar_linear_problem(-2.0, 1.0);
    for theta in [0.25, 0.5, 0.75] {
        let (mut good, mut linear, mut endpoint) = (Vec::new(), Vec::new(), Vec::new());
        for h in [0.125, 0.0625, 0.03125] {
            let s = sequential_step(
                &problem,
                0.,
                &y0,
                h,
                &LinearSolverConfig::default(),
                None,
                1e-14,
                1e-12,
                true,
                &mut WorkCounters::default(),
            )
            .unwrap();
            let exact = (-2. * theta * h).exp();
            good.push((rodas5p_dense_output(&s, theta).unwrap()[0] - exact).abs());
            linear.push(((1. - theta) * s.y_old[0] + theta * s.y_new[0] - exact).abs());
            endpoint.push((s.y_new[0] - (-2. * h).exp()).abs());
        }
        let (a, b, c) = (slopes(&good), slopes(&linear), slopes(&endpoint));
        eprintln!("theta={theta}: dense-local={a:?}; endpoint-local={c:?}; linear-mutant={b:?}");
        assert!(a.iter().all(|p| (4.5..5.5).contains(p)), "{good:?}");
        assert!(b.iter().all(|p| (1.5..2.5).contains(p)), "{linear:?}");
        assert!(!b.iter().all(|p| (4.5..5.5).contains(p)));
        // The scalar linear stability function is superconvergent through z^6 to
        // coefficient precision. Do not impose an upper endpoint slope bound.
        assert!(c.iter().all(|p| *p > 5.4), "{endpoint:?}");
    }
}
