use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use vigilode::{
    Atol, Jacobian, MfStageSolvePolicy as Policy, Options, Problem, Rodas5p, SolveError, Tier,
    VigilODE, coloring::SparsityPattern,
};

#[test]
fn matrix_free_policies_use_the_supplied_jvp_and_report_the_executed_driver() {
    let count = Arc::new(AtomicUsize::new(0));
    let invoked = Arc::clone(&count);
    let rates = [2.0, 30.0, 100.0, 500.0];
    let p = Problem::new(4, move |_, y, out| {
        for i in 0..4 {
            out[i] = -rates[i] * y[i];
        }
        Ok(())
    })
    .autonomous()
    .jacobian(Jacobian::jvp(move |_, _, v, out| {
        invoked.fetch_add(1, Ordering::Relaxed);
        for i in 0..4 {
            out[i] = -rates[i] * v[i];
        }
        Ok(())
    }));
    let options = Options::default();
    let y0 = [1.; 4];
    let span = (0., 0.02);
    let original = Rodas5p::solve(&p, span, &y0, &options).unwrap();
    for policy in [
        Policy::Shipped,
        Policy::ColumnTested,
        Policy::ColumnTestedDoubling,
    ] {
        count.store(0, Ordering::Relaxed);
        let run = VigilODE::solve_matrix_free(&p, span, &y0, &options, policy).unwrap();
        assert!(run.is_success());
        assert_eq!(run.stats.route.driver_id, policy.driver_id());
        let w = run.stats.counters;
        assert_eq!(w.jvp_calls, count.load(Ordering::Relaxed) as u64);
        assert!(w.jvp_calls > 0);
        assert_eq!(w.jacobian_builds, 0);
        assert_eq!(w.direct_factorizations, 0);
        assert_eq!(w.nonlinear_iterations, 0);
        let error = run
            .final_state()
            .unwrap()
            .iter()
            .zip(rates)
            .map(|(y, a)| (y - (-a * span.1).exp()).abs())
            .fold(0., f64::max);
        assert!(error < 1e-6, "{policy:?}: {error:e}");
        if policy == Policy::Shipped {
            assert_eq!(
                serde_json::to_value(&run.stats).unwrap(),
                serde_json::to_value(&original.stats).unwrap()
            );
            assert_eq!(run.y, original.y);
        } else {
            assert_eq!(run.stats.tier_executed, Tier::Performance);
        }
        println!(
            "{} max_abs={error:e} attempts={} accepted={} jvp={} rhs={} iterations={} linear_solves={}",
            policy.id(),
            run.stats.attempts,
            run.stats.accepted_steps,
            w.jvp_calls,
            w.rhs_calls,
            w.linear_iterations,
            w.linear_solves
        );
    }
}

#[test]
fn matrix_free_policy_refuses_dense_banded_mass_and_vector_routes_before_callbacks() {
    let rhs = |_, _: &[f64], _: &mut [f64]| panic!("preflight invoked the RHS");
    let jvp = |_, _: &[f64], _: &[f64], _: &mut [f64]| panic!("preflight invoked JVP");
    let independent = Problem::new(3, rhs);
    let raw = independent.clone().jacobian(Jacobian::jvp(jvp));
    let opts = Options::default();
    let band = SparsityPattern::from_rows(3, &[vec![0], vec![1], vec![2]]).unwrap();
    let sparse_band = independent
        .clone()
        .jacobian(Jacobian::sparse_jvp(band, jvp));
    let general = SparsityPattern::from_rows(3, &[vec![0, 1, 2], vec![1], vec![0, 2]]).unwrap();
    let sparse_general = independent
        .clone()
        .jacobian(Jacobian::sparse_jvp(general, jvp));
    let vector = Options {
        atol: Atol::Vector(vec![1e-9; 3]),
        ..opts.clone()
    };
    let dense = Options {
        dense_output: true,
        ..opts.clone()
    };
    for (p, o) in [
        (&independent, &opts),
        (&raw.clone().banded(1, 1), &opts),
        (&sparse_band, &opts),
        (&raw.clone().mass(vec![0.; 9]), &opts),
        (&raw, &vector),
        (&raw, &dense),
        (&sparse_general, &vector),
        (&sparse_general, &dense),
        (&sparse_general.clone().banded(1, 1), &opts),
    ] {
        assert!(matches!(
            VigilODE::solve_matrix_free(p, (0., 0.1), &[1.; 3], o, Policy::ColumnTested),
            Err(SolveError::Unsupported(_))
        ));
    }
}
