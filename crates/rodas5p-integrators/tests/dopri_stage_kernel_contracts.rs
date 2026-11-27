use rodas5p_core::WorkCounters;
use rodas5p_integrators::{Dopri5Workspace, DopriStageKernel};
#[test]
fn component_stage_recurrence_reduces_buffer_writes() {
    let rhs = |t: f64, y: &[f64], out: &mut [f64]| {
        for (i, (v, x)) in out.iter_mut().zip(y).enumerate() {
            *v = -(i as f64 + 1.) * x + t;
        }
        Ok(())
    };
    let y = [0.125, -0.25, 0.5, 0.75];
    let mut a = [0.; 4];
    let mut b = [0.; 4];
    let mut e = [0.; 4];
    let mut f = [0.; 4];
    let mut old = Dopri5Workspace::new_with_kernel(4, DopriStageKernel::Rows).unwrap();
    let mut new = Dopri5Workspace::new_with_kernel(4, DopriStageKernel::Components).unwrap();
    let mut c = WorkCounters::default();
    let mut d = WorkCounters::default();
    old.step_into(&rhs, 0., &y, 0.001, &mut a, &mut e, &mut c)
        .unwrap();
    new.step_into(&rhs, 0., &y, 0.001, &mut b, &mut f, &mut d)
        .unwrap();
    assert_eq!(a.map(f64::to_bits), b.map(f64::to_bits));
    assert_eq!(e.map(f64::to_bits), f.map(f64::to_bits));
    assert_eq!(c.rhs_calls, d.rhs_calls);
    println!(
        "DOPRI_RED_GREEN {:?} {:?}",
        old.stage_work(),
        new.stage_work()
    );
    assert!(new.stage_work().state_writes < old.stage_work().state_writes);
    assert!(new.stage_work().error_writes < old.stage_work().error_writes);
}

#[test]
fn component_kernel_retries_and_partial_identity() {
    use rodas5p_core::CoreError;
    use rodas5p_integrators::{
        AdaptiveStepConfig, Dopri5Config, OutputSchedule,
        integrate_dopri5_dense_observed_with_kernel,
    };
    use std::cell::Cell;
    let cfg = Dopri5Config {
        adaptive: AdaptiveStepConfig {
            initial_step: 0.125,
            atol: 1e-12,
            rtol: 1e-9,
            ..Default::default()
        },
        recover_nonfinite_rhs: true,
        ..Default::default()
    };
    let times = OutputSchedule::new(vec![0., 0.03125, 0.0625, 0.125]).unwrap();
    for fatal in [false, true] {
        let mut runs = vec![];
        for k in [DopriStageKernel::Rows, DopriStageKernel::Components] {
            let failed = Cell::new(false);
            let rhs = |t: f64, y: &[f64], out: &mut [f64]| {
                if fatal && t > 0.06 {
                    return Err(CoreError::InvalidInput("intentional fatal".into()));
                }
                if !fatal && t > 0.03 && !failed.replace(true) {
                    return Err(CoreError::NonFinite("intentional retry".into()));
                }
                for (o, &v) in out.iter_mut().zip(y) {
                    *o = -64. * v;
                }
                Ok(())
            };
            let r = integrate_dopri5_dense_observed_with_kernel(
                &rhs,
                (0., 0.125),
                &[0.25, -0.5],
                &cfg,
                &times,
                k,
            )
            .unwrap();
            assert_eq!(r.observed.success, !fatal);
            if !fatal {
                assert!(failed.get() && r.rejected_steps > 0 && r.fsal_reuses > 0);
            }
            runs.push(r);
        }
        let a = &runs[0];
        let b = &runs[1];
        assert_eq!(a.observed.t, b.observed.t);
        assert_eq!(a.observed.y, b.observed.y);
        assert_eq!(a.final_state, b.final_state);
        assert_eq!(a.final_time, b.final_time);
        assert_eq!(a.stop_reason, b.stop_reason);
        assert_eq!(
            (
                a.attempts,
                a.accepted_steps,
                a.rejected_steps,
                a.rhs_completed,
                a.fsal_reuses,
                a.observed.counters.rhs_calls
            ),
            (
                b.attempts,
                b.accepted_steps,
                b.rejected_steps,
                b.rhs_completed,
                b.fsal_reuses,
                b.observed.counters.rhs_calls
            )
        );
        println!(
            "DOPRI_RETRY fatal={} attempts={} accepted={} rejected={} rhs={} fsal={}",
            fatal,
            a.attempts,
            a.accepted_steps,
            a.rejected_steps,
            a.observed.counters.rhs_calls,
            a.fsal_reuses
        );
    }
}

#[test]
fn blocked_stage_recurrence_actual_writes() {
    let y = std::array::from_fn::<_, 31, _>(|i| (i as f64 - 16.) / 64.);
    let rhs = |t: f64, y: &[f64], out: &mut [f64]| {
        for (i, (v, x)) in out.iter_mut().zip(y).enumerate() {
            *v = -((i % 4) as f64 + 1.) * x + t;
        }
        Ok(())
    };
    let mut prev = None;
    for k in [
        DopriStageKernel::Rows,
        DopriStageKernel::Block4,
        DopriStageKernel::Block8,
        DopriStageKernel::Block16,
        DopriStageKernel::Block32,
    ] {
        let mut w = Dopri5Workspace::new_with_kernel(y.len(), k).unwrap();
        let mut a = [0.; 31];
        let mut e = [0.; 31];
        w.step_into(
            &rhs,
            0.,
            &y,
            0.001,
            &mut a,
            &mut e,
            &mut WorkCounters::default(),
        )
        .unwrap();
        let b = (a.map(f64::to_bits), e.map(f64::to_bits));
        if let Some(p) = prev {
            assert_eq!(b, p);
        } else {
            prev = Some(b);
        }
        println!("BLOCK_RED_GREEN {:?} {:?}", k, w.stage_work());
        if k != DopriStageKernel::Rows {
            assert_eq!(w.stage_work().state_writes, 6 * 31);
            assert_eq!(w.stage_work().error_writes, 31);
        }
    }
}
