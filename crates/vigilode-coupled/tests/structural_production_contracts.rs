use rodas5p_core::CoreError;
use rodas5p_integrators::ParallelExecution;
use vigilode_coupled::structural_mean_field::{MeanFieldAffine, StructuralWork};

#[allow(dead_code)]
#[path = "../examples/support/structural_mean_field_pr91.rs"]
mod pr91;

fn fixture(n: usize) -> (Vec<f64>, Vec<f64>) {
    (
        (0..n).map(|i| 0.5 + 0.1 * (0.7 * i as f64).sin()).collect(),
        (0..n)
            .map(|i| 0.2 + 0.05 * (1.9 * i as f64).cos())
            .collect(),
    )
}

#[test]
fn fused_direct_screen_preserves_pr91_bits_and_successful_work() {
    for n in [1, 3, 96] {
        let (y, b) = fixture(n);
        for (d, k, h) in [
            (0.0, 0.0, 0.125),
            (2.0, 2.0, 0.125),
            (20.0, 19.0, 1.0 / 64.0),
        ] {
            for threads in [1, 2] {
                let e = ParallelExecution::rayon(threads).unwrap();
                let old = pr91::MeanFieldAffine::new(d, k, b.clone()).unwrap();
                let model = MeanFieldAffine::new(d, k, b.clone()).unwrap();
                let baseline = old.plan(h).unwrap().step(&old, &y, 1e-9, &e).unwrap();
                let candidate = model
                    .plan(h)
                    .unwrap()
                    .step_charged(&model, &y, 1e-9, &e)
                    .unwrap();
                assert_eq!(baseline.y_new, candidate.y_new);
                assert_eq!(baseline.stages_u, candidate.stages_u);
                assert_eq!(baseline.error_vector, candidate.error_vector);
                assert_eq!(
                    baseline.full_residual_linf.to_bits(),
                    candidate.full_residual_linf.to_bits()
                );
                assert_eq!(
                    baseline.work.rhs_evaluations,
                    candidate.work.rhs_evaluations
                );
                assert_eq!(
                    baseline.work.physical_linear_actions,
                    candidate.work.physical_linear_actions
                );
                assert_eq!(
                    baseline.work.full_state_causal_accumulations,
                    candidate.work.full_state_causal_accumulations
                );
                let a = old.sequential_step(&y, h, 1e-9, &e).unwrap();
                let z = model.sequential_step_charged(&y, h, 1e-9, &e).unwrap();
                assert_eq!(a.y_new, z.y_new);
                assert_eq!(a.stages_u, z.stages_u);
                assert_eq!(
                    a.full_residual_linf.to_bits(),
                    z.full_residual_linf.to_bits()
                );
            }
        }
    }
}

#[test]
fn pre_attempt_refusals_have_zero_work() {
    let m = MeanFieldAffine::new(2.0, 1.0, vec![0.0]).unwrap();
    let p = m.plan(0.125).unwrap();
    let e = ParallelExecution::sequential();
    for (y, cap) in [(&[][..], 1e-9), (&[f64::NAN][..], 1e-9), (&[1.0][..], 0.0)] {
        assert_eq!(
            p.step_charged(&m, y, cap, &e).unwrap_err().work,
            StructuralWork::default()
        );
        assert_eq!(
            m.sequential_step_charged(y, 0.125, cap, &e)
                .unwrap_err()
                .work,
            StructuralWork::default()
        );
    }
    let other = MeanFieldAffine::new(3.0, 1.0, vec![0.0]).unwrap();
    assert_eq!(
        p.step_charged(&other, &[1.0], 1e-9, &e).unwrap_err().work,
        StructuralWork::default()
    );
    assert!(p.enclosed_screen(0).is_err());
    let ep = p.enclosed_screen(2).unwrap();
    assert_eq!(
        ep.step(&m, &[1.0], 1e-9, &e).unwrap_err().work,
        StructuralWork::default()
    );
}

#[test]
fn failed_rhs_attempt_is_charged_before_return() {
    let m = MeanFieldAffine::new(2.0, 0.0, vec![0.0]).unwrap();
    let e = ParallelExecution::sequential();
    let p = m.plan(0.125).unwrap();
    for failure in [
        p.step_charged(&m, &[-f64::MAX], 1e-9, &e).unwrap_err(),
        m.sequential_step_charged(&[-f64::MAX], 0.125, 1e-9, &e)
            .unwrap_err(),
        p.enclosed_screen(1)
            .unwrap()
            .step(&m, &[-f64::MAX], 1e-9, &e)
            .unwrap_err(),
    ] {
        assert!(matches!(failure.error, CoreError::NonFinite(_)));
        assert_eq!(failure.work.rhs_evaluations, 1);
        assert_eq!(failure.work.lifted_components, 0);
    }
}

#[test]
fn every_scheduled_failed_lift_is_joined_and_charged() {
    let n = 5;
    let m = MeanFieldAffine::new(0.0, 0.0, vec![1e307; n]).unwrap();
    let p = m.plan(1e4).unwrap();
    let mut failures = Vec::new();
    for threads in [1, 4] {
        let failure = p
            .step_charged(
                &m,
                &vec![0.0; n],
                1e-9,
                &ParallelExecution::rayon(threads).unwrap(),
            )
            .unwrap_err();
        assert!(matches!(failure.error, CoreError::NonFinite(_)));
        assert_eq!(failure.work.rhs_evaluations, 1);
        assert_eq!(failure.work.lifted_components, 8 * n as u64);
        assert_eq!(failure.work.original_space_residual_components, 0);
        failures.push(failure.work);
    }
    assert_eq!(failures[0], failures[1]);
}

#[test]
fn residual_gate_refusals_keep_complete_direct_and_enclosed_work() {
    let (y, b) = fixture(17);
    let m = MeanFieldAffine::new(20.0, 19.0, b).unwrap();
    let p = m.plan(1.0 / 64.0).unwrap();
    let ep = p.enclosed_screen(17).unwrap();
    for threads in [1, 4] {
        let e = ParallelExecution::rayon(threads).unwrap();
        let direct = p.step_charged(&m, &y, 1e-9, &e).unwrap();
        assert!(direct.full_residual_linf > 0.0);
        let failure = p.step_charged(&m, &y, f64::from_bits(1), &e).unwrap_err();
        assert_eq!(failure.work, direct.work);
        assert_eq!(failure.work.rhs_evaluations, 9);
        assert_eq!(failure.work.physical_linear_actions, 8);
        assert_eq!(failure.work.original_space_residual_components, 8 * 17);
        let enclosed = ep.step(&m, &y, 1e-9, &e).unwrap();
        let failure = ep.step(&m, &y, f64::from_bits(1), &e).unwrap_err();
        assert_eq!(failure.work, enclosed.work);
        assert_eq!(failure.work.rhs_evaluations, 1);
        assert_eq!(failure.work.enclosed_mean_components, 2 * 17);
        assert_eq!(failure.work.factorized_residual_stages, 8);
    }
}

#[test]
fn full_stage_enclosures_preserve_lift_endpoint_and_parallel_bits() {
    for n in [1, 3, 65] {
        let (y, b) = fixture(n);
        for (d, k, h) in [
            (0.0, 0.0, 0.125),
            (2.0, 2.0, 0.125),
            (20.0, 19.0, 1.0 / 64.0),
        ] {
            let model = MeanFieldAffine::new(d, k, b.clone()).unwrap();
            let p = model.plan(h).unwrap();
            let ep = p.enclosed_screen(n).unwrap();
            let direct = p
                .step_charged(&model, &y, 1e-9, &ParallelExecution::sequential())
                .unwrap();
            let serial = ep
                .step(&model, &y, 1e-9, &ParallelExecution::sequential())
                .unwrap();
            let parallel = ep
                .step(&model, &y, 1e-9, &ParallelExecution::rayon(2).unwrap())
                .unwrap();
            assert_eq!(direct.y_new, serial.y_new);
            assert_eq!(direct.stages_u, serial.stages_u);
            assert_eq!(direct.error_vector, serial.error_vector);
            assert_eq!(serial.y_new, parallel.y_new);
            assert_eq!(serial.stages_u, parallel.stages_u);
            assert_eq!(
                serial.stage_residual_upper_bounds,
                parallel.stage_residual_upper_bounds
            );
            assert_eq!(serial.work, parallel.work);
            assert!(serial.full_residual_upper_bound.is_finite());
            assert_eq!(serial.work.rhs_evaluations, 1);
            assert_eq!(serial.work.physical_linear_actions, 0);
            assert_eq!(serial.work.full_state_causal_accumulations, 0);
            assert_eq!(serial.work.original_space_residual_components, 8 * n as u64);
        }
    }
}
