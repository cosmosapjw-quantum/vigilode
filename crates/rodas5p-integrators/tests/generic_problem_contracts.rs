use rodas5p_core::{WorkCounters, safe_l2};
use rodas5p_integrators::{
    complex_dahlquist_problem, oscillatory_prothero_robinson_problem, robertson_problem,
    semilinear_advection_diffusion_problem, stiff_van_der_pol_problem,
};

fn relative_jvp_mismatch(
    problem: &rodas5p_integrators::OdeProblem,
    t: f64,
    y: &[f64],
    v: &[f64],
) -> f64 {
    let operator = problem.linearize_matrix_free(t, y).unwrap();
    let mut analytic = vec![0.0; problem.dimension];
    operator.apply(v, &mut analytic).unwrap();
    let mut counters = WorkCounters::default();
    let eps = 1.0e-7;
    let yp = y
        .iter()
        .zip(v)
        .map(|(yi, vi)| yi + eps * vi)
        .collect::<Vec<_>>();
    let ym = y
        .iter()
        .zip(v)
        .map(|(yi, vi)| yi - eps * vi)
        .collect::<Vec<_>>();
    let fp = problem.eval_rhs(t, &yp, &mut counters).unwrap();
    let fm = problem.eval_rhs(t, &ym, &mut counters).unwrap();
    let fd = fp
        .iter()
        .zip(fm)
        .map(|(a, b)| (a - b) / (2.0 * eps))
        .collect::<Vec<_>>();
    let difference = analytic
        .iter()
        .zip(&fd)
        .map(|(a, b)| a - b)
        .collect::<Vec<_>>();
    safe_l2(&difference) / safe_l2(&analytic).max(1.0e-14)
}

#[test]
fn exact_generic_problems_satisfy_rhs_and_jvp_contracts() {
    let exact_cases = vec![
        complex_dahlquist_problem(3, 12.0, 19.0, 0.0).unwrap(),
        oscillatory_prothero_robinson_problem(-500.0, 30.0, 17.0, 0.0).unwrap(),
        semilinear_advection_diffusion_problem(8, 0.02, 0.7, -1.0, 4.0, 0.0).unwrap(),
    ];
    for (problem, _) in exact_cases {
        let t = 0.037;
        let y = problem.exact(t).expect("analytic state");
        let dt = 1.0e-7;
        let yp = problem.exact(t + dt).unwrap();
        let ym = problem.exact(t - dt).unwrap();
        let derivative = yp
            .iter()
            .zip(ym)
            .map(|(a, b)| (a - b) / (2.0 * dt))
            .collect::<Vec<_>>();
        let mut counters = WorkCounters::default();
        let rhs = problem.eval_rhs(t, &y, &mut counters).unwrap();
        let defect = rhs
            .iter()
            .zip(derivative)
            .map(|(a, b)| a - b)
            .collect::<Vec<_>>();
        assert!(
            safe_l2(&defect) <= 2.0e-7 * safe_l2(&rhs).max(1.0),
            "{} exact-path RHS mismatch: {}",
            problem.name,
            safe_l2(&defect)
        );
        let v = (0..problem.dimension)
            .map(|i| ((i + 1) as f64 * 0.31).sin())
            .collect::<Vec<_>>();
        assert!(
            relative_jvp_mismatch(&problem, t, &y, &v) < 2.0e-7,
            "{} JVP mismatch",
            problem.name
        );
    }
}

#[test]
fn robertson_conserves_total_population_and_all_generic_problems_are_matrix_free_capable() {
    let (robertson, y) = robertson_problem().unwrap();
    let mut counters = WorkCounters::default();
    let rhs = robertson.eval_rhs(0.0, &y, &mut counters).unwrap();
    assert!(rhs.iter().sum::<f64>().abs() < 1.0e-14);

    let problems = vec![
        robertson,
        stiff_van_der_pol_problem(1000.0).unwrap().0,
        complex_dahlquist_problem(2, 10.0, 20.0, 0.0).unwrap().0,
        oscillatory_prothero_robinson_problem(-1000.0, 100.0, 25.0, 0.0)
            .unwrap()
            .0,
        semilinear_advection_diffusion_problem(6, 0.01, 1.0, -1.0, 2.0, 0.0)
            .unwrap()
            .0,
    ];
    for problem in problems {
        assert!(problem.supports_matrix_free_jvp(), "{}", problem.name);
        let matrix_free = problem.jvp_only_clone().unwrap();
        assert!(matrix_free.supports_matrix_free_jvp());
        assert!(!matrix_free.has_explicit_jacobian());
    }
}
