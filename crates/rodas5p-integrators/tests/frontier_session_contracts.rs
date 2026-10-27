//! SW02 stage 1: native immutable endpoint sessions against independent legacy loops.
use rodas5p_core::{CoreError, CoreResult, DenseMatrix};
use rodas5p_integrators::{
    AdaptiveStepConfig, ControllerKind, Dopri5Config, Dopri5Session,
    NativeSessionAdvance as Advance, NativeSessionIdentity as Identity,
    NativeSessionMethod as Method, OdeProblem, OutputSchedule, Rodas5pFastSession,
    integrate_dopri5_observed, integrate_rodas5p_fast_observed_with_partial,
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

struct Counted;
thread_local! {static TRACK:Cell<bool>=const{Cell::new(false)};static ALLOCS:Cell<usize>=const{Cell::new(0)};}
fn charge() {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = ALLOCS.try_with(|n| n.set(n.get() + 1));
    }
}
unsafe impl GlobalAlloc for Counted {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        charge();
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        charge();
        unsafe { System.realloc(p, l, n) }
    }
}
#[global_allocator]
static ALLOCATOR: Counted = Counted;
fn plan() -> OutputSchedule {
    OutputSchedule::new(vec![0.0, 0.07, 0.070000000001, 0.33, 1.0]).unwrap()
}
fn adaptive(controller: ControllerKind) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        initial_step: 0.4,
        rtol: 1e-10,
        atol: 1e-12,
        controller,
        ..Default::default()
    }
}
fn rhs(_: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
    out[0] = -4.0 * y[0];
    Ok(())
}
fn problem(recover_first: bool, fatal_at: u64) -> (OdeProblem, Arc<AtomicU64>) {
    let calls = Arc::new(AtomicU64::new(0));
    let seen = calls.clone();
    let p = OdeProblem::new(
        "session-decay",
        1,
        Arc::new(move |t, y, out| {
            let n = seen.fetch_add(1, Ordering::SeqCst) + 1;
            if recover_first && n == 1 {
                return Err(CoreError::NonFinite("recover first".into()));
            }
            if n == fatal_at {
                return Err(CoreError::FatalCallback("session fatal".into()));
            }
            rhs(t, y, out)
        }),
        None,
        Some(Arc::new(|_, _| DenseMatrix::new(1, 1, vec![-4.0]))),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap()
    .with_jacobian_into(Arc::new(|_, _, j| {
        j[(0, 0)] = -4.0;
        Ok(())
    }));
    (p, calls)
}
fn assert_bits(a: &[f64], b: &[f64]) {
    assert_eq!(
        a.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
        b.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
    );
}
fn assert_observed(
    a: &rodas5p_integrators::ObservedIntegrationResult,
    b: &rodas5p_integrators::ObservedIntegrationResult,
) {
    assert_bits(&a.t, &b.t);
    assert_eq!(a.y.len(), b.y.len());
    for (x, y) in a.y.iter().zip(&b.y) {
        assert_bits(x, y);
    }
    assert_eq!(a.counters, b.counters);
    assert_eq!(a.success, b.success);
    assert_eq!(a.internal_steps, b.internal_steps);
    assert_eq!(a.output_clipped_steps, b.output_clipped_steps);
}
#[test]
fn dopri_registered_checkpoints_and_quota_yields_keep_legacy_bits_fsal_and_controller() {
    for controller in [
        ControllerKind::Integral,
        ControllerKind::Pi,
        ControllerKind::Predictive,
        ControllerKind::PredictiveCapped,
        ControllerKind::PredictiveCapped2,
    ] {
        for fsal in [true, false] {
            let cfg = Dopri5Config {
                adaptive: adaptive(controller),
                fsal,
                ..Default::default()
            };
            let schedule = plan();
            let id = Identity::new(Method::Dopri5, 1);
            let legacy =
                integrate_dopri5_observed(&rhs, (0.0, 1.0), &[1.0], &cfg, &schedule).unwrap();
            let mut split =
                Dopri5Session::new(&rhs, (0.0, 1.0), &[1.0], &cfg, &schedule, id).unwrap();
            let mut continuous =
                Dopri5Session::new(&rhs, (0.0, 1.0), &[1.0], &cfg, &schedule, id).unwrap();
            for target in [0.07, 0.33, 1.0] {
                let expected = continuous.advance_to(target, usize::MAX, id).unwrap();
                loop {
                    let result = split.advance_to(target, 1, id).unwrap();
                    if result != Advance::Yield {
                        assert_eq!(result, expected);
                        break;
                    }
                }
                let a = split.checkpoint();
                let b = continuous.checkpoint();
                assert_eq!(a.t.to_bits(), b.t.to_bits());
                assert_bits(a.y, b.y);
                assert_eq!(a.proposed_h.to_bits(), b.proposed_h.to_bits());
                assert_eq!(a.counters, b.counters);
            }
            let split = split.finish().unwrap();
            assert_observed(&split.observed, &legacy.observed);
            assert_bits(&split.final_state, &legacy.final_state);
            assert_eq!(split.attempts, legacy.attempts);
            assert_eq!(split.rejected_steps, legacy.rejected_steps);
            assert!(split.rejected_steps > 0);
            assert_eq!(split.fsal_reuses, legacy.fsal_reuses);
            assert_eq!(split.rhs_completed, legacy.rhs_completed);
        }
    }
}
#[test]
fn rodas_registered_checkpoints_and_rejection_yields_keep_legacy_work_and_controller() {
    for controller in [
        ControllerKind::Integral,
        ControllerKind::Pi,
        ControllerKind::Predictive,
        ControllerKind::PredictiveCapped,
        ControllerKind::PredictiveCapped2,
    ] {
        let cfg = adaptive(controller);
        let schedule = plan();
        let id = Identity::new(Method::Rodas5pFast, 1);
        let (p, _) = problem(false, u64::MAX);
        let (legacy, partial) =
            integrate_rodas5p_fast_observed_with_partial(&p, (0.0, 1.0), &[1.0], &cfg, &schedule)
                .unwrap();
        assert!(partial.is_none());
        let mut split =
            Rodas5pFastSession::new(&p, (0.0, 1.0), &[1.0], &cfg, &schedule, id).unwrap();
        let mut continuous =
            Rodas5pFastSession::new(&p, (0.0, 1.0), &[1.0], &cfg, &schedule, id).unwrap();
        for target in [0.07, 0.33, 1.0] {
            let expected = continuous.advance_to(target, usize::MAX, id).unwrap();
            loop {
                let result = split.advance_to(target, 1, id).unwrap();
                if result != Advance::Yield {
                    assert_eq!(result, expected);
                    break;
                }
            }
            let a = split.checkpoint();
            let b = continuous.checkpoint();
            assert_eq!(a.t.to_bits(), b.t.to_bits());
            assert_bits(a.y, b.y);
            assert_eq!(a.proposed_h.to_bits(), b.proposed_h.to_bits());
            assert_eq!(a.counters, b.counters);
        }
        let split = split.finish().unwrap();
        assert!(matches!(
            split.termination,
            rodas5p_integrators::Rodas5pSessionTermination::Complete
        ));
        assert_observed(&split.run.observed, &legacy.observed);
        assert_eq!(split.run.attempts, legacy.attempts);
        assert_eq!(split.run.jacobian_reuses, legacy.jacobian_reuses);
        assert!(split.run.rejected_steps > 0);
    }
}
#[test]
fn recoverable_retry_and_fatal_are_preserved_across_native_session_yields() {
    let schedule = plan();
    for fatal_at in [1, 40, u64::MAX] {
        let recover = fatal_at == u64::MAX;
        let (p, calls) = problem(recover, fatal_at);
        let mut cfg = Dopri5Config {
            adaptive: adaptive(ControllerKind::Pi),
            recover_nonfinite_rhs: true,
            ..Default::default()
        };
        let id = Identity::new(Method::Dopri5, 1);
        let callback =
            |t, y: &[f64], out: &mut [f64]| p.eval_rhs_into(t, y, out, &mut Default::default());
        let mut s = Dopri5Session::new(&callback, (0.0, 1.0), &[1.0], &cfg, &schedule, id).unwrap();
        let end = loop {
            let r = s.advance_to(1.0, 1, id).unwrap();
            if r != Advance::Yield {
                break r;
            }
        };
        assert_eq!(
            end,
            if recover {
                Advance::Complete
            } else {
                Advance::Stopped
            }
        );
        let before = calls.load(Ordering::SeqCst);
        assert_eq!(s.advance_to(1.0, 10, id).unwrap(), end);
        assert_eq!(calls.load(Ordering::SeqCst), before);
        let result = s.finish().unwrap();
        assert_eq!(result.observed.counters.rhs_calls, before);
        if fatal_at == 40 {
            assert!(result.accepted_steps > 0);
        }
        let (reference, _) = problem(recover, fatal_at);
        let callback = |t, y: &[f64], out: &mut [f64]| {
            reference.eval_rhs_into(t, y, out, &mut Default::default())
        };
        let legacy =
            integrate_dopri5_observed(&callback, (0.0, 1.0), &[1.0], &cfg, &schedule).unwrap();
        assert_observed(&result.observed, &legacy.observed);
        assert_eq!(result.stop_reason, legacy.stop_reason);
        let (p, calls) = problem(recover, fatal_at);
        let id = Identity::new(Method::Rodas5pFast, 1);
        cfg.adaptive = adaptive(ControllerKind::Pi);
        let mut s =
            Rodas5pFastSession::new(&p, (0.0, 1.0), &[1.0], &cfg.adaptive, &schedule, id).unwrap();
        let end = loop {
            let r = s.advance_to(1.0, 1, id).unwrap();
            if r != Advance::Yield {
                break r;
            }
        };
        assert_eq!(
            end,
            if recover {
                Advance::Complete
            } else {
                Advance::Stopped
            }
        );
        let before = calls.load(Ordering::SeqCst);
        assert_eq!(s.advance_to(1.0, 10, id).unwrap(), end);
        assert_eq!(calls.load(Ordering::SeqCst), before);
        let result = s.finish().unwrap();
        assert_eq!(result.run.observed.counters.rhs_calls, before);
        let (reference, _) = problem(recover, fatal_at);
        let (legacy, legacy_partial) = integrate_rodas5p_fast_observed_with_partial(
            &reference,
            (0.0, 1.0),
            &[1.0],
            &cfg.adaptive,
            &schedule,
        )
        .unwrap();
        assert_observed(&result.run.observed, &legacy.observed);
        if let Some(partial) = legacy_partial {
            assert_eq!(result.final_time.to_bits(), partial.t.to_bits());
            assert_bits(&result.final_state, &partial.y);
            assert!(matches!(
                result.termination,
                rodas5p_integrators::Rodas5pSessionTermination::Failure(
                    rodas5p_integrators::Rodas5pSessionFailure {
                        error: CoreError::FatalCallback(_),
                        ..
                    }
                )
            ));
        } else {
            assert!(matches!(
                result.termination,
                rodas5p_integrators::Rodas5pSessionTermination::Complete
            ));
        }
    }
}
#[test]
fn identities_and_unregistered_targets_refuse_before_callbacks_without_changing_checkpoint() {
    let schedule = plan();
    let cfg = Dopri5Config {
        adaptive: adaptive(ControllerKind::Pi),
        ..Default::default()
    };
    for method in [Method::Dopri5, Method::Rodas5pFast] {
        let (p, calls) = problem(false, u64::MAX);
        let id = Identity::new(method, 1);
        let callback =
            |t, y: &[f64], out: &mut [f64]| p.eval_rhs_into(t, y, out, &mut Default::default());
        let mut d = Dopri5Session::new(
            &callback,
            (0.0, 1.0),
            &[1.0],
            &cfg,
            &schedule,
            Identity::new(Method::Dopri5, 1),
        )
        .unwrap();
        let mut r = Rodas5pFastSession::new(
            &p,
            (0.0, 1.0),
            &[1.0],
            &cfg.adaptive,
            &schedule,
            Identity::new(Method::Rodas5pFast, 1),
        )
        .unwrap();
        assert_eq!(
            if method == Method::Dopri5 {
                d.advance_to(0.07, usize::MAX, id)
            } else {
                r.advance_to(0.07, usize::MAX, id)
            }
            .unwrap(),
            Advance::Checkpoint
        );
        let mut changed = Vec::new();
        for field in 0..6 {
            let mut key = id;
            match field {
                0 => key.model_epoch += 1,
                1 => key.provider_epoch += 1,
                2 => key.norm_epoch += 1,
                3 => key.tolerance_epoch += 1,
                4 => key.dimension += 1,
                _ => {
                    key.method = if method == Method::Dopri5 {
                        Method::Rodas5pFast
                    } else {
                        Method::Dopri5
                    }
                }
            }
            changed.push(key);
        }
        for key in changed {
            let before = calls.load(Ordering::SeqCst);
            let result = if method == Method::Dopri5 {
                d.advance_to(0.07, 1, key)
            } else {
                r.advance_to(0.07, 1, key)
            };
            assert!(result.is_err());
            assert_eq!(calls.load(Ordering::SeqCst), before);
        }
        let before = calls.load(Ordering::SeqCst);
        let result = if method == Method::Dopri5 {
            d.advance_to(0.1, 1, id)
        } else {
            r.advance_to(0.1, 1, id)
        };
        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), before);
        assert_eq!(
            if method == Method::Dopri5 {
                d.advance_to(1.0, 0, id)
            } else {
                r.advance_to(1.0, 0, id)
            }
            .unwrap(),
            Advance::Yield
        );
        assert_eq!(calls.load(Ordering::SeqCst), before);
        assert_eq!(
            if method == Method::Dopri5 {
                d.advance_to(0.33, usize::MAX, id)
            } else {
                r.advance_to(0.33, usize::MAX, id)
            }
            .unwrap(),
            Advance::Checkpoint
        );
    }
}

#[test]
fn sorted_target_lookup_preserves_signed_zero_and_refuses_nonfinite_or_missing_targets() {
    for scheduled_zero in [-0.0_f64, 0.0_f64] {
        let mut times: Vec<f64> = (0..=1024).map(|i| i as f64 / 512.0 - 1.0).collect();
        times[512] = scheduled_zero;
        let schedule = OutputSchedule::new(times).unwrap();
        let config = Dopri5Config::default();
        let (p, calls) = problem(false, u64::MAX);
        let callback =
            |t, y: &[f64], out: &mut [f64]| p.eval_rhs_into(t, y, out, &mut Default::default());
        let explicit_id = Identity::new(Method::Dopri5, 1);
        let implicit_id = Identity::new(Method::Rodas5pFast, 1);
        let mut explicit = Dopri5Session::new(
            &callback,
            (-1.0, 1.0),
            &[1.0],
            &config,
            &schedule,
            explicit_id,
        )
        .unwrap();
        let mut implicit = Rodas5pFastSession::new(
            &p,
            (-1.0, 1.0),
            &[1.0],
            &config.adaptive,
            &schedule,
            implicit_id,
        )
        .unwrap();
        // A zero quota isolates target validation from numerical stepping.
        // Either IEEE sign must locate the single mathematical zero entry.
        for target in [-0.0, 0.0, -0.5, 0.5, 1.0] {
            assert_eq!(
                explicit.advance_to(target, 0, explicit_id).unwrap(),
                Advance::Yield
            );
            assert_eq!(
                implicit.advance_to(target, 0, implicit_id).unwrap(),
                Advance::Yield
            );
        }
        for target in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            0.0001,
            -1.1,
            1.1,
        ] {
            assert!(explicit.advance_to(target, 1, explicit_id).is_err());
            assert!(implicit.advance_to(target, 1, implicit_id).is_err());
        }
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        for checkpoint in [explicit.checkpoint(), implicit.checkpoint()] {
            assert_eq!(checkpoint.t, -1.0);
            assert_eq!(checkpoint.y, &[1.0]);
            assert_eq!(checkpoint.attempts, 0);
            assert_eq!(*checkpoint.counters, Default::default());
        }
    }
}
#[test]
fn cumulative_budget_is_sticky_and_true_final_arrival_wins() {
    let schedule = plan();
    let (p, calls) = problem(false, u64::MAX);
    let cfg = AdaptiveStepConfig {
        max_attempts: 1,
        ..adaptive(ControllerKind::Integral)
    };
    let callback =
        |t, y: &[f64], out: &mut [f64]| p.eval_rhs_into(t, y, out, &mut Default::default());
    let id = Identity::new(Method::Dopri5, 1);
    let mut d = Dopri5Session::new(
        &callback,
        (0.0, 1.0),
        &[1.0],
        &Dopri5Config {
            adaptive: cfg.clone(),
            ..Default::default()
        },
        &schedule,
        id,
    )
    .unwrap();
    assert_eq!(d.advance_to(1.0, 1, id).unwrap(), Advance::Stopped);
    let before = calls.load(Ordering::SeqCst);
    assert_eq!(d.advance_to(1.0, usize::MAX, id).unwrap(), Advance::Stopped);
    assert_eq!(calls.load(Ordering::SeqCst), before);
    assert_eq!(d.finish().unwrap().attempts, 1);
    let id = Identity::new(Method::Rodas5pFast, 1);
    let mut r = Rodas5pFastSession::new(&p, (0.0, 1.0), &[1.0], &cfg, &schedule, id).unwrap();
    assert_eq!(r.advance_to(1.0, 1, id).unwrap(), Advance::Stopped);
    let before = calls.load(Ordering::SeqCst);
    assert_eq!(r.advance_to(1.0, usize::MAX, id).unwrap(), Advance::Stopped);
    assert_eq!(calls.load(Ordering::SeqCst), before);
    assert_eq!(r.finish().unwrap().run.attempts, 1);
    let zero = |_: f64, _: &[f64], out: &mut [f64]| {
        out.fill(0.0);
        Ok(())
    };
    let cfg = AdaptiveStepConfig {
        initial_step: 1.0,
        max_attempts: 1,
        ..Default::default()
    };
    let schedule = OutputSchedule::new(vec![0.0, 1.0]).unwrap();
    let id = Identity::new(Method::Dopri5, 1);
    let mut d = Dopri5Session::new(
        &zero,
        (0.0, 1.0),
        &[1.0],
        &Dopri5Config {
            adaptive: cfg.clone(),
            ..Default::default()
        },
        &schedule,
        id,
    )
    .unwrap();
    assert_eq!(d.advance_to(1.0, 1, id).unwrap(), Advance::Complete);
    assert!(d.finish().unwrap().observed.success);
    let p = OdeProblem::new(
        "zero",
        1,
        Arc::new(zero),
        None,
        Some(Arc::new(|_, _| Ok(DenseMatrix::zeros(1, 1)))),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let id = Identity::new(Method::Rodas5pFast, 1);
    let mut r = Rodas5pFastSession::new(&p, (0.0, 1.0), &[1.0], &cfg, &schedule, id).unwrap();
    assert_eq!(r.advance_to(1.0, 1, id).unwrap(), Advance::Complete);
    assert!(r.finish().unwrap().run.observed.success);
}
#[test]
fn zero_and_adjacent_representable_spans_keep_legacy_results() {
    for span in [(1.0, 1.0), (1.0, 1.0_f64.next_up())] {
        let schedule = OutputSchedule::new(if span.0 == span.1 {
            vec![span.0]
        } else {
            vec![span.0, span.1]
        })
        .unwrap();
        let cfg = Dopri5Config::default();
        let id = Identity::new(Method::Dopri5, 1);
        let mut d = Dopri5Session::new(&rhs, span, &[1.0], &cfg, &schedule, id).unwrap();
        let _ = d.advance_to(span.1, usize::MAX, id).unwrap();
        let d = d.finish().unwrap();
        let reference = integrate_dopri5_observed(&rhs, span, &[1.0], &cfg, &schedule).unwrap();
        assert_observed(&d.observed, &reference.observed);
        let (p, _) = problem(false, u64::MAX);
        let id = Identity::new(Method::Rodas5pFast, 1);
        let mut r =
            Rodas5pFastSession::new(&p, span, &[1.0], &cfg.adaptive, &schedule, id).unwrap();
        let _ = r.advance_to(span.1, usize::MAX, id).unwrap();
        let r = r.finish().unwrap();
        let (reference, rp) = integrate_rodas5p_fast_observed_with_partial(
            &p,
            span,
            &[1.0],
            &cfg.adaptive,
            &schedule,
        )
        .unwrap();
        assert_observed(&r.run.observed, &reference.observed);
        assert!(rp.is_none());
        assert!(matches!(
            r.termination,
            rodas5p_integrators::Rodas5pSessionTermination::Complete
        ));
    }
}
#[test]
fn repeated_native_quota_yields_allocate_zero_between_registered_outputs() {
    let schedule = OutputSchedule::new(vec![0.0, 10.0]).unwrap();
    let cfg = AdaptiveStepConfig {
        initial_step: 1e-4,
        max_step: 1e-4,
        ..Default::default()
    };
    let id = Identity::new(Method::Dopri5, 1);
    let mut d = Dopri5Session::new(
        &rhs,
        (0.0, 10.0),
        &[1.0],
        &Dopri5Config {
            adaptive: cfg.clone(),
            ..Default::default()
        },
        &schedule,
        id,
    )
    .unwrap();
    d.advance_to(10.0, 1, id).unwrap();
    let (p, _) = problem(false, u64::MAX);
    let rid = Identity::new(Method::Rodas5pFast, 1);
    let mut r = Rodas5pFastSession::new(&p, (0.0, 10.0), &[1.0], &cfg, &schedule, rid).unwrap();
    r.advance_to(10.0, 1, rid).unwrap();
    ALLOCS.with(|n| n.set(0));
    TRACK.with(|f| f.set(true));
    for _ in 0..20 {
        assert_eq!(d.advance_to(10.0, 1, id).unwrap(), Advance::Yield);
        assert_eq!(r.advance_to(10.0, 1, rid).unwrap(), Advance::Yield);
    }
    TRACK.with(|f| f.set(false));
    assert_eq!(ALLOCS.with(Cell::get), 0);
}

#[test]
fn unsupported_native_rodas_routes_refuse_before_user_callbacks() {
    let schedule = plan();
    let adaptive = AdaptiveStepConfig::default();
    let identity = Identity::new(Method::Rodas5pFast, 1);
    let (mut mass, calls) = problem(false, u64::MAX);
    mass.mass_matrix = Some(DenseMatrix::identity(1));
    assert!(
        Rodas5pFastSession::new(&mass, (0.0, 1.0), &[1.0], &adaptive, &schedule, identity).is_err()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let jvp_only = OdeProblem::new(
        "jvp unsupported",
        1,
        Arc::new(|_, _, _| panic!("unsupported RHS")),
        None,
        None,
        Some(Arc::new(|_, _, _, _| panic!("unsupported JVP"))),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    assert!(
        Rodas5pFastSession::new(
            &jvp_only,
            (0.0, 1.0),
            &[1.0],
            &adaptive,
            &schedule,
            identity
        )
        .is_err()
    );
}

#[test]
fn native_rodas_invalid_callback_errors_are_sticky_and_keep_original_detail() {
    use rodas5p_integrators::{
        Rodas5pSessionFailureKind as Kind, Rodas5pSessionTermination as Term,
    };
    for dimension_error in [false, true] {
        for after_prefix in [false, true] {
            let calls = Arc::new(AtomicU64::new(0));
            let seen = calls.clone();
            let p = OdeProblem::new(
                "invalid callback",
                1,
                Arc::new(move |t, y, out| {
                    seen.fetch_add(1, Ordering::SeqCst);
                    if !after_prefix || t > 0.1 {
                        return Err(if dimension_error {
                            CoreError::Dimension("original dimension callback".into())
                        } else {
                            CoreError::InvalidInput("original invalid callback".into())
                        });
                    }
                    rhs(t, y, out)
                }),
                None,
                Some(Arc::new(|_, _| DenseMatrix::new(1, 1, vec![-4.0]))),
                None,
                None,
                true,
                None,
                None,
            )
            .unwrap();
            let schedule = plan();
            let cfg = AdaptiveStepConfig {
                initial_step: 0.001,
                ..Default::default()
            };
            let id = Identity::new(Method::Rodas5pFast, 1);
            let mut s =
                Rodas5pFastSession::new(&p, (0.0, 1.0), &[1.0], &cfg, &schedule, id).unwrap();
            assert_eq!(s.advance_to(1.0, usize::MAX, id).unwrap(), Advance::Stopped);
            let saved = s.checkpoint();
            let prefix = (
                saved.t,
                saved.y.to_vec(),
                saved.attempts,
                saved.accepted_steps,
                saved.rejected_steps,
                *saved.counters,
            );
            assert_eq!(prefix.2, prefix.3 + prefix.4);
            if after_prefix {
                assert!(prefix.3 > 0);
            } else {
                assert_eq!(prefix.2, 1);
                assert_eq!(prefix.3, 0);
                assert_eq!(prefix.4, 1);
            }
            let before = calls.load(Ordering::SeqCst);
            assert_eq!(prefix.5.rhs_calls, before);
            assert_eq!(s.advance_to(1.0, usize::MAX, id).unwrap(), Advance::Stopped);
            assert_eq!(calls.load(Ordering::SeqCst), before);
            let result = s.finish().unwrap();
            assert_eq!(result.final_time.to_bits(), prefix.0.to_bits());
            assert_bits(&result.final_state, &prefix.1);
            assert_eq!(result.run.observed.counters, prefix.5);
            match result.termination {
                Term::Failure(failure) => {
                    assert_eq!(failure.kind, Kind::CallbackError);
                    match failure.error {
                        CoreError::Dimension(message) if dimension_error => {
                            assert_eq!(message, "original dimension callback")
                        }
                        CoreError::InvalidInput(message) if !dimension_error => {
                            assert_eq!(message, "original invalid callback")
                        }
                        other => panic!("wrong error {other:?}"),
                    }
                }
                other => panic!("wrong termination {other:?}"),
            }
        }
    }
}

#[test]
fn native_rodas_preentry_clock_refusal_does_not_invent_a_trial() {
    use rodas5p_integrators::{
        MaxStepPolicy, Rodas5pSessionFailureKind as Kind, Rodas5pSessionTermination as Term,
    };
    let start = 1e12_f64;
    let end = start + 4.0 * (start.next_up() - start);
    let schedule = OutputSchedule::new(vec![start, end]).unwrap();
    let (p, calls) = problem(false, u64::MAX);
    let cfg = AdaptiveStepConfig {
        initial_step: 1e-4,
        max_step: 1e-4,
        max_step_policy: MaxStepPolicy::StrictRepresentedCap,
        ..Default::default()
    };
    let id = Identity::new(Method::Rodas5pFast, 1);
    let mut s = Rodas5pFastSession::new(&p, (start, end), &[1.0], &cfg, &schedule, id).unwrap();
    assert_eq!(s.advance_to(end, 1, id).unwrap(), Advance::Stopped);
    assert_eq!(s.checkpoint().attempts, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(s.advance_to(end, 1, id).unwrap(), Advance::Stopped);
    let result = s.finish().unwrap();
    assert_eq!(result.final_time, start);
    assert_eq!(result.final_state, vec![1.0]);
    assert!(matches!(
        result.termination,
        Term::Failure(rodas5p_integrators::Rodas5pSessionFailure {
            kind: Kind::Clock,
            ..
        })
    ));
}
