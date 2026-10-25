#![forbid(unsafe_code)]
extern crate d3_fixture;
#[path = "gateway.rs"]
mod gateway;

use gateway::{BoxDomain, CallbackFault, Counters, Fault, Gate, RuntimeWitness};
use std::cell::Cell;

fn good_probe() -> RuntimeWitness { RuntimeWitness { epoch: 7, environment_ok: true } }
fn actual_probe() -> RuntimeWitness {
    RuntimeWitness { epoch: 7, environment_ok: d3_fixture::admission::environment().is_ok() }
}
fn gate() -> Gate<2> {
    Gate::new(BoxDomain::new(-2.0, 2.0, [-4.0; 2], [4.0; 2]).unwrap(), 7)
}
fn rhs(t: f64, y: &[f64; 2], out: &mut [f64; 2]) -> Result<(), CallbackFault> {
    *out = [t + y[0], t - y[1]]; Ok(())
}
fn jac(_: f64, _: &[f64; 2], out: &mut [[f64; 2]; 2]) -> Result<(), CallbackFault> {
    *out = [[1.0, 0.0], [0.0, -1.0]]; Ok(())
}

#[test]
fn domain_requires_nonzero_dimension_and_finite_ordered_bounds() {
    assert!(matches!(BoxDomain::<0>::new(0.0, 1.0, [], []), Err(Fault::InvalidDomain)));
    for (lo, hi) in [(f64::NAN, 1.0), (0.0, f64::INFINITY), (2.0, 1.0)] {
        assert!(matches!(BoxDomain::new(lo, hi, [0.0; 2], [1.0; 2]), Err(Fault::InvalidDomain)));
    }
    for (lo, hi) in [([0.0, f64::NEG_INFINITY], [1.0; 2]), ([2.0, 0.0], [1.0; 2])] {
        assert!(matches!(BoxDomain::new(0.0, 1.0, lo, hi), Err(Fault::InvalidDomain)));
    }
    let mut gate = Gate::new(BoxDomain::new(0.0, 0.0, [1.0; 2], [1.0; 2]).unwrap(), 7);
    assert_eq!(gate.eval_rhs(0.0, &[1.0; 2], &mut good_probe, &mut rhs), Ok([1.0, -1.0]));
}

#[test]
fn full_write_rhs_and_jacobian_return_owned_values_and_charge_separately() {
    let mut g = gate();
    assert_eq!(g.eval_rhs(0.5, &[1.0, 2.0], &mut good_probe, &mut rhs), Ok([1.5, -1.5]));
    assert_eq!(g.eval_jacobian(0.5, &[1.0, 2.0], &mut good_probe, &mut jac), Ok([[1.0, 0.0], [0.0, -1.0]]));
    assert_eq!(g.counters(), Counters { rhs: 1, jac: 1, pairs: 0 });
    assert_eq!(g.terminal(), None);
}

#[test]
fn invalid_queries_dispatch_neither_probe_nor_callback_and_do_not_poison() {
    let mut g = gate();
    let probes = Cell::new(0);
    let calls = Cell::new(0);
    let mut probe = || { probes.set(probes.get() + 1); good_probe() };
    let mut callback = |_: f64, _: &[f64; 2], out: &mut [f64; 2]| {
        calls.set(calls.get() + 1); *out = [0.0; 2]; Ok(())
    };
    for (t, y) in [(f64::NAN, [0.0; 2]), (f64::INFINITY, [0.0; 2]),
        (-2.1, [0.0; 2]), (2.1, [0.0; 2]), (0.0, [f64::NAN, 0.0]),
        (0.0, [f64::INFINITY, 0.0]), (0.0, [-4.1, 0.0]), (0.0, [0.0, 4.1])]
    {
        assert_eq!(g.eval_rhs(t, &y, &mut probe, &mut callback), Err(Fault::OutsideDomain));
    }
    assert_eq!((probes.get(), calls.get()), (0, 0));
    assert_eq!(g.counters(), Counters::default());
    assert_eq!(g.terminal(), None);
    assert!(g.eval_rhs(-2.0, &[-4.0, 4.0], &mut probe, &mut callback).is_ok());
    assert_eq!((probes.get(), calls.get()), (2, 1));
}

#[test]
fn recoverable_partial_rhs_does_not_publish_scratch_and_allows_explicit_retry() {
    let mut g = gate();
    let mut visible = [19.0, 23.0];
    let result = g.eval_rhs(0.0, &[0.0; 2], &mut good_probe, &mut |_, _, out| {
        out[0] = 999.0; Err(CallbackFault::Recoverable)
    });
    if let Ok(values) = result { visible = values; }
    assert_eq!(result, Err(Fault::RecoverableCallback));
    assert_eq!(visible, [19.0, 23.0]);
    assert_eq!(g.terminal(), None);
    assert_eq!(g.counters().rhs, 1);
    assert!(g.eval_rhs(0.0, &[0.0; 2], &mut good_probe, &mut rhs).is_ok());
    assert_eq!(g.counters().rhs, 2);
}

#[test]
fn recoverable_partial_jacobian_does_not_publish_scratch_and_allows_retry() {
    let mut g = gate();
    let mut visible = [[19.0; 2]; 2];
    let result = g.eval_jacobian(0.0, &[0.0; 2], &mut good_probe, &mut |_, _, out| {
        out[0][0] = 999.0; Err(CallbackFault::Recoverable)
    });
    if let Ok(values) = result { visible = values; }
    assert_eq!(result, Err(Fault::RecoverableCallback));
    assert_eq!(visible, [[19.0; 2]; 2]);
    assert_eq!(g.terminal(), None);
    assert!(g.eval_jacobian(0.0, &[0.0; 2], &mut good_probe, &mut jac).is_ok());
    assert_eq!(g.counters().jac, 2);
}

#[test]
fn partial_ok_rhs_poison_is_sticky_and_prevents_new_dispatch() {
    let mut g = gate();
    assert_eq!(g.eval_rhs(0.0, &[0.0; 2], &mut good_probe, &mut |_, _, out| {
        out[0] = 1.0; Ok(())
    }), Err(Fault::NonFiniteOutput));
    assert_eq!(g.eval_rhs(0.0, &[0.0; 2], &mut || panic!("probe must not run"),
        &mut |_, _, _| panic!("RHS must not run")), Err(Fault::NonFiniteOutput));
    assert_eq!(g.counters().rhs, 1);
    assert_eq!(g.terminal(), Some(Fault::NonFiniteOutput));
}

#[test]
fn strict_gateway_rejects_legacy_sparse_partial_writer_without_changing_its_contract() {
    let mut sparse = |_: f64, _: &[f64; 2], out: &mut [[f64; 2]; 2]| {
        out[0][0] = 3.0; out[1][1] = 5.0; Ok(())
    };
    // A zero/previous-matrix API can intentionally admit this fixed sparsity writer.
    let mut legacy_buffer = [[0.0; 2]; 2];
    sparse(0.0, &[0.0; 2], &mut legacy_buffer).unwrap();
    assert_eq!(legacy_buffer, [[3.0, 0.0], [0.0, 5.0]]);
    let mut strict = gate();
    assert_eq!(strict.eval_jacobian(0.0, &[0.0; 2], &mut good_probe, &mut sparse), Err(Fault::NonFiniteOutput));
    assert_eq!(strict.terminal(), Some(Fault::NonFiniteOutput));
    assert_eq!(legacy_buffer, [[3.0, 0.0], [0.0, 5.0]]);
}

#[test]
fn rhs_nan_and_infinities_after_full_write_are_terminal() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut g = gate();
        assert_eq!(g.eval_rhs(0.0, &[0.0; 2], &mut good_probe, &mut |_, _, out| {
            *out = [1.0, bad]; Ok(())
        }), Err(Fault::NonFiniteOutput));
        assert_eq!(g.terminal(), Some(Fault::NonFiniteOutput));
        assert_eq!(g.counters().rhs, 1);
    }
}

#[test]
fn jacobian_nan_and_infinities_after_full_write_are_terminal() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut g = gate();
        assert_eq!(g.eval_jacobian(0.0, &[0.0; 2], &mut good_probe, &mut |_, _, out| {
            *out = [[0.0; 2]; 2]; out[1][0] = bad; Ok(())
        }), Err(Fault::NonFiniteOutput));
        assert_eq!(g.counters().jac, 1);
        assert_eq!(g.terminal(), Some(Fault::NonFiniteOutput));
    }
}

#[test]
fn fatal_rhs_and_handled_panic_charge_once_run_post_probe_and_stop() {
    for panic in [false, true] {
        let mut g = gate();
        let probes = Cell::new(0);
        let calls = Cell::new(0);
        let mut probe = || { probes.set(probes.get() + 1); good_probe() };
        let expected = if panic { Fault::CallbackPanic } else { Fault::FatalCallback };
        assert_eq!(g.eval_rhs(0.0, &[0.0; 2], &mut probe, &mut |_, _, out| {
            calls.set(calls.get() + 1); out[0] = 10.0;
            if panic { panic!("handled RHS unwind"); }
            Err(CallbackFault::Fatal)
        }), Err(expected));
        assert_eq!((probes.get(), calls.get()), (2, 1));
        assert_eq!(g.eval_rhs(0.0, &[0.0; 2], &mut probe, &mut rhs), Err(expected));
        assert_eq!(probes.get(), 2);
        assert_eq!(g.counters().rhs, 1);
        assert_eq!(g.terminal(), Some(expected));
    }
}

#[test]
fn fatal_jacobian_and_handled_panic_have_the_same_sticky_boundary() {
    for panic in [false, true] {
        let mut g = gate();
        let probes = Cell::new(0);
        let mut probe = || { probes.set(probes.get() + 1); good_probe() };
        let expected = if panic { Fault::CallbackPanic } else { Fault::FatalCallback };
        assert_eq!(g.eval_jacobian(0.0, &[0.0; 2], &mut probe, &mut |_, _, out| {
            out[0][0] = 10.0;
            if panic { panic!("handled Jacobian unwind"); }
            Err(CallbackFault::Fatal)
        }), Err(expected));
        assert_eq!(probes.get(), 2);
        assert_eq!(g.eval_jacobian(0.0, &[0.0; 2], &mut probe, &mut jac), Err(expected));
        assert_eq!(probes.get(), 2);
        assert_eq!(g.counters().jac, 1);
    }
}

#[test]
fn pre_witness_epoch_or_environment_fault_prevents_all_callback_work() {
    for (witness, expected) in [
        (RuntimeWitness { epoch: 8, environment_ok: true }, Fault::EpochMismatch),
        (RuntimeWitness { epoch: 7, environment_ok: false }, Fault::Environment),
    ] {
        let mut g = gate();
        assert_eq!(g.eval_rhs(0.0, &[0.0; 2], &mut || witness,
            &mut |_, _, _| panic!("callback must not run")), Err(expected));
        assert_eq!(g.counters(), Counters::default());
        assert_eq!(g.eval_jacobian(0.0, &[0.0; 2], &mut || panic!("poisoned probe"), &mut jac), Err(expected));
        assert_eq!(g.terminal(), Some(expected));
    }
}

#[test]
fn post_witness_epoch_or_environment_fault_discards_complete_rhs() {
    for (witness, expected) in [
        (RuntimeWitness { epoch: 8, environment_ok: true }, Fault::EpochMismatch),
        (RuntimeWitness { epoch: 7, environment_ok: false }, Fault::Environment),
    ] {
        let mut g = gate();
        let mut probes = 0;
        let mut probe = || { probes += 1; if probes == 1 { good_probe() } else { witness } };
        assert_eq!(g.eval_rhs(0.0, &[0.0; 2], &mut probe, &mut rhs), Err(expected));
        assert_eq!(g.counters().rhs, 1);
        assert_eq!(g.terminal(), Some(expected));
    }
}

#[test]
fn probe_panic_before_or_after_dispatch_is_terminal_and_counted_exactly() {
    for panic_at in [1, 2] {
        let mut g = gate();
        let mut probes = 0;
        let mut probe = || { probes += 1; if probes == panic_at { panic!("handled probe unwind"); } good_probe() };
        assert_eq!(g.eval_rhs(0.0, &[0.0; 2], &mut probe, &mut rhs), Err(Fault::ProbePanic));
        assert_eq!(g.counters().rhs, if panic_at == 1 { 0 } else { 1 });
        assert_eq!(g.terminal(), Some(Fault::ProbePanic));
    }
}

#[test]
fn post_witness_failure_overrides_recoverable_error_and_callback_panic() {
    for panic in [false, true] {
        let mut g = gate();
        let mut probes = 0;
        let mut probe = || { probes += 1; RuntimeWitness { epoch: 7, environment_ok: probes == 1 } };
        assert_eq!(g.eval_rhs(0.0, &[0.0; 2], &mut probe, &mut |_, _, _| {
            if panic { panic!("callback unwind before environment refusal"); }
            Err(CallbackFault::Recoverable)
        }), Err(Fault::Environment));
        assert_eq!(g.terminal(), Some(Fault::Environment));
        assert_eq!(g.counters().rhs, 1);
    }
}

#[test]
fn prepared_pair_binds_query_epoch_and_returns_only_after_both_callbacks() {
    let mut g = gate();
    let probes = Cell::new(0);
    let mut probe = || { probes.set(probes.get() + 1); good_probe() };
    let prepared = g.prepare_pair(0.5, &[1.0, 2.0], &mut probe, &mut rhs, &mut jac).unwrap();
    assert_eq!(prepared.t, 0.5);
    assert_eq!(prepared.y, [1.0, 2.0]);
    assert_eq!(prepared.epoch, 7);
    assert_eq!(prepared.rhs, [1.5, -1.5]);
    assert_eq!(prepared.jacobian, [[1.0, 0.0], [0.0, -1.0]]);
    assert_eq!(probes.get(), 4);
    assert_eq!(g.counters(), Counters { rhs: 1, jac: 1, pairs: 1 });
}

#[test]
fn pair_rhs_refusal_never_enters_jacobian() {
    let mut g = gate();
    assert_eq!(g.prepare_pair(0.0, &[0.0; 2], &mut good_probe,
        &mut |_, _, _| Err(CallbackFault::Recoverable),
        &mut |_, _, _| panic!("Jacobian must not run")), Err(Fault::RecoverableCallback));
    assert_eq!(g.counters(), Counters { rhs: 1, jac: 0, pairs: 0 });
    assert_eq!(g.terminal(), None);
}

#[test]
fn pair_jacobian_refusal_preserves_visible_pair_but_keeps_attempts_and_effects() {
    let mut g = gate();
    let mut visible = g.prepare_pair(0.0, &[1.0; 2], &mut good_probe, &mut rhs, &mut jac).unwrap();
    let before = visible;
    let external = Cell::new(0);
    let result = g.prepare_pair(1.0, &[2.0; 2], &mut good_probe,
        &mut |t, y, out| { external.set(external.get() + 1); rhs(t, y, out) },
        &mut |_, _, out| { external.set(external.get() + 1); out[0][0] = 99.0; Err(CallbackFault::Recoverable) });
    if let Ok(prepared) = result { visible = prepared; }
    assert_eq!(result, Err(Fault::RecoverableCallback));
    assert_eq!(visible, before);
    assert_eq!(external.get(), 2);
    assert_eq!(g.counters(), Counters { rhs: 2, jac: 2, pairs: 1 });
    assert_eq!(g.terminal(), None);
}

#[test]
fn pair_last_post_witness_refusal_never_publishes_pair() {
    let mut g = gate();
    let mut probes = 0;
    let mut probe = || { probes += 1; RuntimeWitness { epoch: if probes == 4 { 8 } else { 7 }, environment_ok: true } };
    assert_eq!(g.prepare_pair(0.0, &[0.0; 2], &mut probe, &mut rhs, &mut jac), Err(Fault::EpochMismatch));
    assert_eq!(g.counters(), Counters { rhs: 1, jac: 1, pairs: 0 });
    assert_eq!(g.terminal(), Some(Fault::EpochMismatch));
}

#[test]
fn rhs_counter_overflow_is_terminal_before_dispatch() {
    let mut g = gate();
    let before = Counters { rhs: u64::MAX, jac: 2, pairs: 1 };
    g.seed_counters_for_test(before);
    assert_eq!(g.eval_rhs(0.0, &[0.0; 2], &mut good_probe,
        &mut |_, _, _| panic!("overflowed callback")), Err(Fault::CounterOverflow));
    assert_eq!(g.counters(), before);
    assert_eq!(g.terminal(), Some(Fault::CounterOverflow));
}

#[test]
fn jacobian_counter_overflow_is_terminal_before_dispatch() {
    let mut g = gate();
    let before = Counters { rhs: 2, jac: u64::MAX, pairs: 1 };
    g.seed_counters_for_test(before);
    assert_eq!(g.eval_jacobian(0.0, &[0.0; 2], &mut good_probe,
        &mut |_, _, _| panic!("overflowed callback")), Err(Fault::CounterOverflow));
    assert_eq!(g.counters(), before);
    assert_eq!(g.terminal(), Some(Fault::CounterOverflow));
}

#[test]
fn pair_counter_overflow_preserves_both_attempts_but_returns_no_pair() {
    let mut g = gate();
    g.seed_counters_for_test(Counters { rhs: 0, jac: 0, pairs: u64::MAX });
    assert_eq!(g.prepare_pair(0.0, &[0.0; 2], &mut good_probe, &mut rhs, &mut jac), Err(Fault::CounterOverflow));
    assert_eq!(g.counters(), Counters { rhs: 1, jac: 1, pairs: u64::MAX });
    assert_eq!(g.terminal(), Some(Fault::CounterOverflow));
}

#[test]
fn centered_difference_preflights_both_times_before_any_rhs_dispatch() {
    for t in [-2.0, 2.0] {
        let mut g = gate();
        let probes = Cell::new(0);
        assert_eq!(g.estimate_partial_t_centered(t, &[0.0; 2], 0.1,
            &mut || { probes.set(probes.get() + 1); good_probe() },
            &mut |_, _, _| panic!("preflight must precede RHS")), Err(Fault::OutsideDomain));
        assert_eq!(probes.get(), 1);
        assert_eq!(g.counters(), Counters::default());
        assert_eq!(g.terminal(), None);
    }
}

#[test]
fn centered_difference_refuses_bad_or_unrepresented_offsets_before_dispatch() {
    for eps in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::MIN_POSITIVE] {
        let mut g = gate();
        let probes = Cell::new(0);
        assert_eq!(g.estimate_partial_t_centered(1.0, &[0.0; 2], eps,
            &mut || { probes.set(probes.get() + 1); good_probe() },
            &mut |_, _, _| panic!("invalid offset RHS")), Err(Fault::InvalidDifferenceStep));
        assert_eq!(probes.get(), if eps.is_finite() && eps > 0.0 { 1 } else { 0 });
        assert_eq!(g.counters(), Counters::default());
    }
}

#[test]
fn centered_difference_refuses_environment_before_stencil_arithmetic() {
    for eps in [0.1, f64::MIN_POSITIVE] {
        let mut g = gate();
        let probes = Cell::new(0);
        // MIN_POSITIVE would collapse the represented stencil at t=1.
        // Environment refusal must occur before that arithmetic/decision.
        assert_eq!(g.estimate_partial_t_centered(1.0, &[0.0; 2], eps,
            &mut || { probes.set(probes.get() + 1); RuntimeWitness { epoch: 7, environment_ok: false } },
            &mut |_, _, _| panic!("unadmitted environment RHS")), Err(Fault::Environment));
        assert_eq!(probes.get(), 1);
        assert_eq!(g.counters(), Counters::default());
        assert_eq!(g.terminal(), Some(Fault::Environment));
    }
}

#[test]
fn centered_difference_uses_actual_represented_denominator() {
    let mut g = gate();
    let mut times = Vec::new();
    let probes = Cell::new(0);
    let estimate = g.estimate_partial_t_centered(1.0, &[0.0; 2], 0.1,
        &mut || { probes.set(probes.get() + 1); good_probe() },
        &mut |t, _, out| { times.push(t); *out = [t, 2.0 * t]; Ok(()) }).unwrap();
    assert_eq!(times, vec![1.0 - 0.1, 1.0 + 0.1]);
    assert_ne!((times[1] - times[0]).to_bits(), (2.0_f64 * 0.1).to_bits());
    assert_eq!(estimate, [1.0, 2.0]);
    assert_eq!(probes.get(), 5);
    assert_eq!(g.counters(), Counters { rhs: 2, jac: 0, pairs: 0 });
}

#[test]
fn centered_difference_refusals_preserve_partial_work_without_hidden_retry() {
    for refuse_at in [1, 2] {
        let mut g = gate();
        let mut calls = 0;
        assert_eq!(g.estimate_partial_t_centered(0.0, &[0.0; 2], 0.1, &mut good_probe,
            &mut |t, _, out| { calls += 1; if calls == refuse_at { out[0] = 99.0; Err(CallbackFault::Recoverable) }
                else { *out = [t, t]; Ok(()) } }), Err(Fault::RecoverableCallback));
        assert_eq!(calls, refuse_at);
        assert_eq!(g.counters().rhs, refuse_at);
        assert_eq!(g.terminal(), None);
    }
}

#[test]
fn centered_difference_nonfinite_estimate_is_terminal_after_two_finite_rhs_values() {
    let mut g = gate();
    assert_eq!(g.estimate_partial_t_centered(0.0, &[0.0; 2], 0.1, &mut good_probe,
        &mut |t, _, out| { *out = [if t < 0.0 { -f64::MAX } else { f64::MAX }; 2]; Ok(()) }), Err(Fault::NonFiniteOutput));
    assert_eq!(g.counters().rhs, 2);
    assert_eq!(g.terminal(), Some(Fault::NonFiniteOutput));
}

#[test]
fn actual_reused_mxcsr_probe_is_a_separate_dependency_and_accepts_supported_host() {
    assert!(d3_fixture::admission::environment().is_ok());
    let mut g = gate();
    let pair = g.prepare_pair(0.5, &[1.0, 2.0], &mut actual_probe, &mut rhs, &mut jac).unwrap();
    assert_eq!(pair.rhs, [1.5, -1.5]);
    assert_eq!(pair.epoch, 7);
    assert_eq!(g.counters(), Counters { rhs: 1, jac: 1, pairs: 1 });
}
