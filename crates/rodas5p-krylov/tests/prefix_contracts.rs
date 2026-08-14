use rodas5p_core::{ClosureOperator, IdentityPreconditioner, WorkCounters, safe_l2};
use rodas5p_krylov::{GmresConfig, GmresPrefixSession, solve_gmres_incremental};

fn nonnormal_shifted_operator(
    n: usize,
) -> ClosureOperator<impl Fn(&[f64], &mut [f64]) -> rodas5p_core::CoreResult<()> + Send + Sync> {
    ClosureOperator::new(n, move |x, y| {
        for i in 0..n {
            let diagonal = 1.0 + 0.02 * (i + 1) as f64;
            let upper = if i + 1 < n { -0.18 * x[i + 1] } else { 0.0 };
            let lower = if i > 0 { 0.03 * x[i - 1] } else { 0.0 };
            y[i] = diagonal * x[i] + upper + lower;
        }
        Ok(())
    })
}

#[test]
fn gmres_prefix_resume_matches_cold_incremental_path() {
    let n = 48;
    let operator = nonnormal_shifted_operator(n);
    let pc = IdentityPreconditioner::new(n);
    let rhs: Vec<f64> = (0..n)
        .map(|i| ((i + 1) as f64 * 0.17).sin() + 0.2)
        .collect();
    let config = GmresConfig {
        restart: 32,
        max_arnoldi: 32,
        rtol: 1.0e-11,
        atol: 1.0e-13,
    };

    let mut cold_work = WorkCounters::default();
    let cold = solve_gmres_incremental(&operator, &pc, &rhs, None, &config, &mut cold_work)
        .expect("cold incremental GMRES");

    let mut resumed_work = WorkCounters::default();
    let session =
        GmresPrefixSession::begin(&operator, &pc, &rhs, None, &config, 2, &mut resumed_work)
            .expect("GMRES prefix");
    let prediction = session.prediction();
    assert_eq!(prediction.prefix_iterations, 2);
    assert!(prediction.predicted_total_iterations >= 2);
    assert!(prediction.predicted_total_iterations <= config.max_arnoldi);
    assert!(prediction.residual_norm.is_finite());

    let resumed = session
        .finish(&operator, &pc, &mut resumed_work)
        .expect("resume GMRES");
    let defect: Vec<f64> = cold.x.iter().zip(&resumed.x).map(|(a, b)| a - b).collect();
    assert!(safe_l2(&defect) <= 1.0e-12 * safe_l2(&cold.x).max(1.0));
    assert_eq!(cold.iterations, resumed.iterations);
    assert_eq!(cold_work.linear_matvecs, resumed_work.linear_matvecs);
    assert_eq!(
        cold_work.preconditioner_apps,
        resumed_work.preconditioner_apps
    );
}
