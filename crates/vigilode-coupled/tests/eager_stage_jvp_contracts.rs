//! A1 defect (production plan 2026-10-20, section 6): the eager coupled path
//! computed, and charged, the m stage-0 derivative JVPs at every Newton
//! evaluation although only stages i > 0 use a derivative (the stage-i row
//! couples to stages j < i). The eager evaluation now performs (S - 1) m
//! stage JVPs per evaluation (S = 8); the output bits are unchanged.
use rodas5p_integrators::{OdeProblem, ParallelExecution};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use vigilode_coupled::{Config, coupled_step, coupled_step_residual_first};

const S: u64 = 8;

fn model(eps: f64, calls: Arc<AtomicU64>) -> OdeProblem {
    OdeProblem::new(
        "eager-stage-jvp",
        96,
        Arc::new(move |_, y, f| {
            for i in 0..96 {
                f[i] = [-1.0, -20.0, -100.0][i % 3] * y[i] + eps * y[i] * y[i];
            }
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, y, v, f| {
            calls.fetch_add(1, Ordering::SeqCst);
            for i in 0..96 {
                f[i] = ([-1.0, -20.0, -100.0][i % 3] + 2.0 * eps * y[i]) * v[i];
            }
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

fn bits(x: &[f64]) -> Vec<u64> {
    x.iter().map(|v| v.to_bits()).collect()
}

#[test]
fn eager_evaluation_charges_no_stage_zero_derivative() {
    for (eps, h) in [(0.0, 0.001), (1.0, 0.01), (10.0, 0.001)] {
        let calls = Arc::new(AtomicU64::new(0));
        let p = model(eps, calls.clone());
        let r = coupled_step(
            &p,
            0.0,
            &[0.1; 96],
            h,
            &Config::default(),
            &ParallelExecution::sequential(),
        );
        assert!(r.candidate.is_some(), "eps {eps}: {:?}", r.failure);
        // One trace point per completed eager evaluation, with its basis size.
        let evaluations: Vec<u64> = r
            .trace
            .iter()
            .filter(|p| p.event == "projected-correction")
            .map(|p| p.basis_dimension as u64)
            .collect();
        assert!(!evaluations.is_empty());
        let expected: u64 = evaluations.iter().map(|m| (S - 1) * m).sum();
        assert_eq!(
            r.work.stage_jvp, expected,
            "eps {eps}: stage JVPs {} for evaluations with bases {evaluations:?}",
            r.work.stage_jvp
        );
        assert_eq!(
            r.work.jvp_attempted,
            calls.load(Ordering::SeqCst),
            "eps {eps}: charged JVPs equal JVP callback entries"
        );
        assert_eq!(
            r.work.jvp_attempted,
            r.work.basis_jvp + r.work.stage_jvp + r.work.validation_jvp,
            "eps {eps}: JVP ledger"
        );
    }
}

#[test]
fn eager_solution_bits_equal_residual_first() {
    for eps in [0.0, 1.0, 10.0] {
        let p = model(eps, Arc::new(AtomicU64::new(0)));
        let e = ParallelExecution::rayon(4).unwrap();
        let c = Config::default();
        let a = coupled_step(&p, 0.0, &[0.1; 96], 0.001, &c, &e)
            .candidate
            .unwrap();
        let b = coupled_step_residual_first(&p, 0.0, &[0.1; 96], 0.001, &c, &e)
            .candidate
            .unwrap();
        assert_eq!(bits(&a.y_new), bits(&b.y_new));
        assert_eq!(bits(&a.error_vector), bits(&b.error_vector));
    }
}
