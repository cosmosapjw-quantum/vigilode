//! New D4 protocol/failure-injection contracts only; no prior campaign replay.
#![forbid(unsafe_code)]
extern crate d3_fixture;
extern crate d4_effects;

use d3_fixture::admission::{Accepted, Fault, Metric};
use d3_fixture::{stage_safe, trajectory};
use d4_effects::*;
use std::collections::BTreeMap;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

fn initial() -> [f64; 30] {
    let mut state = [0.0; 30];
    state[0] = 0.25;
    state
}
fn metric() -> Metric { Metric { atol: [1.0e-6; 33], rtol: [1.0e-6; 33] } }
fn limits() -> JournalLimits { JournalLimits { max_events: 16, max_payload_bytes: 1024 } }
fn driver(tf: f64, h: f64, cap: u64) -> Driver {
    Driver::new(0xd4, 0.0, tf, initial(), h, cap, metric(), limits()).unwrap()
}
fn candidate(a: &Accepted) -> Candidate { Candidate { state: a.y, error: [0.0; 30] } }
fn accepted(result: Advance) -> BatchId {
    match result { Advance::Accepted(id) => id, other => panic!("expected accept, got {other:?}") }
}
fn assert_state_bits(a: Accepted, b: Accepted) {
    assert_eq!(a.t.to_bits(), b.t.to_bits());
    assert_eq!(a.y.map(f64::to_bits), b.y.map(f64::to_bits));
    assert_eq!(a.epoch, b.epoch);
    assert_eq!(a.last_norm.map(f64::to_bits), b.last_norm.map(f64::to_bits));
}
fn emit_one(j: &mut Journal) { j.emit(7, vec![10, 20]).unwrap(); }

#[test]
fn d4_accept_stages_owned_payload_and_commits_one_scientific_prefix() {
    let mut d = driver(1.0, 0.1, 10);
    let id = accepted(d.advance(|a, _, j| {
        emit_one(j);
        let mut c = candidate(a);
        c.state[0] += 1.0e-8;
        Ok(c)
    }));
    assert_eq!(id, BatchId { run_id: 0xd4, attempt: 1, epoch: 1 });
    assert_eq!(d.snapshot().t, 0.1);
    assert_eq!(d.snapshot().y[0], 0.25 + 1.0e-8);
    assert_eq!(d.pending().unwrap().events(), &[Event { kind: 7, payload: vec![10, 20] }]);
    assert_eq!(d.pending().unwrap().payload_bytes(), 2);
    assert_eq!((d.attempts(), d.accepted(), d.rejected()), (1, 1, 0));
    assert_eq!(d.counters().attempted_events, 1);
}

#[test]
fn d4_recoverable_callback_discards_events_keeps_work_and_reuses_session() {
    let mut d = driver(1.0, 0.1, 4);
    let before = d.snapshot();
    assert_eq!(d.advance(|_, _, j| { emit_one(j); Err(CallbackFault::Recoverable) }),
        Advance::Rejected(Rejection::RecoverableCallback));
    assert_state_bits(d.snapshot(), before);
    assert!(d.pending().is_none());
    assert_eq!((d.attempts(), d.accepted(), d.rejected()), (1, 0, 1));
    let id = accepted(d.advance(|a, lease, j| {
        assert_eq!(lease.h, 0.05);
        emit_one(j);
        Ok(candidate(a))
    }));
    assert_eq!((id.attempt, id.epoch), (2, 1));
    assert_eq!(d.counters().producer_calls, 2);
    assert_eq!(d.counters().attempted_events, 2);
    assert_eq!(d.pending().unwrap().events().len(), 1);
}

#[test]
fn d4_norm_refusal_discards_trial_events_without_state_change() {
    let mut d = driver(1.0, 0.1, 4);
    let before = d.snapshot();
    assert_eq!(d.advance(|a, _, j| {
        emit_one(j);
        let mut c = candidate(a);
        c.error[0] = 1.0;
        Ok(c)
    }), Advance::Rejected(Rejection::Norm));
    assert_state_bits(d.snapshot(), before);
    assert!(d.pending().is_none());
    assert_eq!((d.attempts(), d.rejected(), d.domain_rejected()), (1, 1, 0));
    assert_eq!(d.counters().attempted_events, 1);
}

#[test]
fn d4_domain_refusal_discards_trial_events_after_norm_acceptance() {
    let mut d = driver(1.0, 0.1, 4);
    let before = d.snapshot();
    assert_eq!(d.advance(|a, _, j| {
        emit_one(j);
        let mut c = candidate(a);
        c.state[0] = -0.01;
        Ok(c)
    }), Advance::Rejected(Rejection::Domain));
    assert_state_bits(d.snapshot(), before);
    assert_eq!((d.attempts(), d.rejected(), d.domain_rejected()), (1, 1, 1));
    assert!(d.pending().is_none());
}

#[test]
fn d4_inherited_attempt_cap_bounds_producers_without_budget_reset() {
    let mut d = driver(1.0, 0.1, 3);
    let mut external_entries = 0;
    for _ in 0..3 {
        assert_eq!(d.advance(|_, _, j| {
            external_entries += 1;
            emit_one(j);
            Err(CallbackFault::Recoverable)
        }), Advance::Rejected(Rejection::RecoverableCallback));
    }
    for _ in 0..2 {
        assert_eq!(d.advance(|_, _, _| { external_entries += 1; panic!("budget must preclude entry") }),
            Advance::Terminated(Terminal::Session(Fault::Budget)));
    }
    assert_eq!(external_entries, 3);
    assert_eq!((d.attempts(), d.rejected(), d.accepted()), (3, 3, 0));
    assert_eq!((d.counters().producer_calls, d.counters().attempted_events), (3, 3));
}

#[test]
fn d4_represented_clock_failure_prevents_callback_entry() {
    let t = 1.0e16_f64;
    let mut d = Driver::new(3, t, t.next_up(), initial(), 0.25, 3, metric(), limits()).unwrap();
    let before = d.snapshot();
    for _ in 0..2 {
        assert_eq!(d.advance(|_, _, _| panic!("clock failure must preclude entry")),
            Advance::Terminated(Terminal::Session(Fault::Clock)));
    }
    assert_state_bits(d.snapshot(), before);
    assert_eq!((d.attempts(), d.counters().producer_calls), (0, 0));
}

#[test]
fn d4_nonfinite_candidate_halts_with_no_published_events() {
    let mut d = driver(1.0, 0.1, 3);
    let before = d.snapshot();
    let result = d.advance(|a, _, j| {
        emit_one(j);
        let mut c = candidate(a);
        c.state[0] = f64::NAN;
        Ok(c)
    });
    assert_eq!(result, Advance::Terminated(Terminal::Session(Fault::InvalidState)));
    assert_state_bits(d.snapshot(), before);
    assert!(d.pending().is_none());
    assert_eq!((d.attempts(), d.accepted(), d.counters().attempted_events), (1, 0, 1));
    assert_eq!(d.advance(|_, _, _| panic!("sticky terminal")), result);
}

#[test]
fn d4_final_time_pending_delivery_precedes_finished_and_empty_batch_needs_ack() {
    let mut d = driver(0.1, 0.1, 1);
    let id = accepted(d.advance(|a, _, _| Ok(candidate(a))));
    assert!(d.pending().unwrap().events().is_empty());
    assert_eq!(d.snapshot().t, 0.1);
    assert_eq!(d.advance(|_, _, _| panic!("pending final batch")), Advance::WaitingDelivery(id));
    assert_eq!(d.attempts(), 1);
    assert_eq!(d.acknowledge(id), Ok(Ack::Acknowledged));
    assert_eq!(d.advance(|_, _, _| panic!("finished")), Advance::Finished);
}

#[test]
fn d4_wrong_ack_cannot_delete_pending_batch() {
    let mut d = driver(1.0, 0.1, 3);
    let id = accepted(d.advance(|a, _, j| { emit_one(j); Ok(candidate(a)) }));
    let saved = d.pending().unwrap().clone();
    for wrong in [BatchId { run_id: 77, ..id }, BatchId { attempt: 99, ..id }, BatchId { epoch: 88, ..id }] {
        assert_eq!(d.acknowledge(wrong), Err(AckError::WrongBatch { expected: id, received: wrong }));
        assert_eq!(d.pending(), Some(&saved));
    }
    assert_eq!(d.counters().acknowledged_batches, 0);
}

#[test]
fn d4_duplicate_last_ack_is_idempotent_while_new_batch_remains_pending() {
    let mut d = driver(1.0, 0.1, 4);
    let first = accepted(d.advance(|a, _, _| Ok(candidate(a))));
    assert_eq!(d.acknowledge(first), Ok(Ack::Acknowledged));
    assert_eq!(d.acknowledge(first), Ok(Ack::AlreadyAcknowledged));
    let second = accepted(d.advance(|a, _, j| { emit_one(j); Ok(candidate(a)) }));
    assert_ne!(first, second);
    let saved = d.pending().unwrap().clone();
    assert_eq!(d.acknowledge(first), Ok(Ack::AlreadyAcknowledged));
    assert_eq!(d.pending(), Some(&saved));
    assert_eq!(d.acknowledge(second), Ok(Ack::Acknowledged));
    assert_eq!(d.acknowledge(first), Err(AckError::NoPending(first)));
    assert_eq!(d.counters().acknowledged_batches, 2);
}

#[test]
fn d4_ignored_event_limit_error_poison_blocks_mathematical_acceptance() {
    let mut d = Driver::new(1, 0.0, 1.0, initial(), 0.1, 3, metric(),
        JournalLimits { max_events: 0, max_payload_bytes: 100 }).unwrap();
    let before = d.snapshot();
    assert_eq!(d.advance(|a, _, j| {
        assert_eq!(j.emit(1, vec![1]), Err(JournalFault::EventLimit));
        assert_eq!(j.emit(2, Vec::new()), Err(JournalFault::EventLimit));
        assert_eq!(j.poison(), Some(JournalFault::EventLimit));
        Ok(candidate(a))
    }), Advance::Terminated(Terminal::Journal(JournalFault::EventLimit)));
    assert_state_bits(d.snapshot(), before);
    assert!(d.pending().is_none());
    assert_eq!((d.accepted(), d.counters().attempted_events, d.counters().poisoned_journals), (0, 2, 1));
    assert_eq!(d.advance(|_, _, _| panic!("poison must be terminal")),
        Advance::Terminated(Terminal::Journal(JournalFault::EventLimit)));
}

#[test]
fn d4_payload_limit_poison_discards_earlier_owned_records() {
    let mut d = Driver::new(1, 0.0, 1.0, initial(), 0.1, 3, metric(),
        JournalLimits { max_events: 10, max_payload_bytes: 3 }).unwrap();
    let before = d.snapshot();
    assert_eq!(d.advance(|a, _, j| {
        j.emit(1, vec![1, 2]).unwrap();
        assert_eq!(j.emit(2, vec![3, 4]), Err(JournalFault::PayloadLimit));
        assert_eq!(j.payload_bytes(), 2);
        assert_eq!(j.len(), 1);
        Ok(candidate(a))
    }), Advance::Terminated(Terminal::Journal(JournalFault::PayloadLimit)));
    assert_state_bits(d.snapshot(), before);
    assert!(d.pending().is_none());
    assert_eq!(d.counters().attempted_payload_bytes, 4);
}

#[test]
fn d4_exact_journal_limits_preserve_order_and_complete_payloads() {
    let mut d = Driver::new(1, 0.0, 1.0, initial(), 0.1, 3, metric(),
        JournalLimits { max_events: 2, max_payload_bytes: 3 }).unwrap();
    accepted(d.advance(|a, _, j| {
        j.emit(1, vec![1, 2]).unwrap();
        j.emit(2, vec![3]).unwrap();
        assert_eq!(j.poison(), None);
        Ok(candidate(a))
    }));
    let b = d.pending().unwrap();
    assert_eq!(b.payload_bytes(), 3);
    assert_eq!(b.events(), &[Event { kind: 1, payload: vec![1, 2] }, Event { kind: 2, payload: vec![3] }]);
}

#[test]
fn d4_delivery_failure_before_write_preserves_batch_and_blocks_production() {
    let mut d = driver(1.0, 0.1, 3);
    let id = accepted(d.advance(|a, _, j| { emit_one(j); Ok(candidate(a)) }));
    let saved = d.pending().unwrap().clone();
    let state = d.snapshot();
    let mut entries = 0;
    assert_eq!(d.deliver_once(|_: &Batch| { entries += 1; Err("before-write") }),
        Delivery::Failed { id, error: "before-write" });
    assert_eq!(entries, 1);
    assert_eq!(d.pending(), Some(&saved));
    assert_state_bits(d.snapshot(), state);
    assert_eq!(d.advance(|_, _, _| panic!("blocked by failed delivery")), Advance::WaitingDelivery(id));
    assert_eq!((d.attempts(), d.counters().producer_calls), (1, 1));
    assert_eq!(d.deliver_once(|b| { assert_eq!(b, &saved); Ok::<_, &str>(()) }), Delivery::Delivered(id));
    assert_eq!(d.pending(), Some(&saved));
    assert_eq!(d.acknowledge(id), Ok(Ack::Acknowledged));
}

#[test]
fn d4_after_write_failure_retries_same_id_with_explicit_sink_deduplication() {
    let mut d = driver(1.0, 0.1, 3);
    let id = accepted(d.advance(|a, _, j| { emit_one(j); Ok(candidate(a)) }));
    let saved = d.pending().unwrap().clone();
    let mut sink = BTreeMap::<BatchId, Vec<Event>>::new();
    let first = d.deliver_once(|b| {
        sink.insert(b.id(), b.events().to_vec());
        Err("receipt-lost-after-write")
    });
    assert_eq!(first, Delivery::Failed { id, error: "receipt-lost-after-write" });
    assert_eq!(d.pending(), Some(&saved));
    let second = d.deliver_once(|b| {
        if let Some(existing) = sink.get(&b.id()) { assert_eq!(existing, b.events()); }
        else { sink.insert(b.id(), b.events().to_vec()); }
        Ok::<_, &str>(())
    });
    assert_eq!(second, Delivery::Delivered(id));
    assert_eq!(sink.len(), 1);
    assert_eq!(sink.get(&id).unwrap(), saved.events());
    assert_eq!((d.attempts(), d.counters().producer_calls, d.counters().delivery_calls), (1, 1, 2));
    assert_eq!(d.acknowledge(id), Ok(Ack::Acknowledged));
    assert_eq!(d.acknowledge(id), Ok(Ack::AlreadyAcknowledged));
}

#[test]
fn d4_delivery_unwind_retains_pending_payload_and_prior_external_write() {
    let mut d = driver(1.0, 0.1, 3);
    let id = accepted(d.advance(|a, _, j| { emit_one(j); Ok(candidate(a)) }));
    let saved = d.pending().unwrap().clone();
    let mut external = Vec::new();
    let result: Delivery<()> = d.deliver_once(|b| {
        external.extend_from_slice(b.events());
        panic!("intentional delivery unwind after external write")
    });
    assert_eq!(result, Delivery::Panicked(id));
    assert_eq!(external, saved.events());
    assert_eq!(d.pending(), Some(&saved));
    assert_eq!(d.advance(|_, _, _| panic!("pending after sink unwind")), Advance::WaitingDelivery(id));
    assert_eq!(d.counters().delivery_panics, 1);
}

#[test]
fn d4_producer_unwind_discards_journal_and_stops_further_attempts() {
    let mut d = driver(1.0, 0.1, 3);
    let before = d.snapshot();
    let mut external = 0;
    let result = d.advance(|_, _, j| {
        emit_one(j);
        external += 1;
        panic!("intentional producer unwind after journal emission")
    });
    assert_eq!(result, Advance::Terminated(Terminal::CallbackPanic));
    assert_state_bits(d.snapshot(), before);
    assert!(d.pending().is_none());
    assert_eq!(external, 1);
    assert_eq!(d.advance(|_, _, _| panic!("sticky fatal")), result);
    assert_eq!((d.attempts(), d.counters().producer_calls, d.counters().attempted_events), (1, 1, 1));
    assert_eq!(d.counters().producer_panics, 1);
}

#[test]
fn d4_fatal_callback_does_not_reset_attempt_accounting_or_publish_events() {
    let mut d = driver(1.0, 0.1, 3);
    let before = d.snapshot();
    let result = d.advance(|_, _, j| { emit_one(j); Err(CallbackFault::Fatal) });
    assert_eq!(result, Advance::Terminated(Terminal::CallbackFatal));
    assert_state_bits(d.snapshot(), before);
    assert!(d.pending().is_none());
    assert_eq!(d.advance(|_, _, _| panic!("fatal")), result);
    assert_eq!((d.attempts(), d.accepted(), d.counters().fatal_callbacks), (1, 0, 1));
}

#[test]
fn d4_delivery_without_pending_batch_never_enters_sink() {
    let mut d = driver(1.0, 0.1, 3);
    let result: Delivery<()> = d.deliver_once(|_| panic!("no pending batch"));
    assert_eq!(result, Delivery::NoPending);
    assert_eq!(d.counters().delivery_calls, 0);
}

#[test]
fn d4_clone_alias_counterexample_keeps_external_capture_changes_on_refusal() {
    let mut d = driver(1.0, 0.1, 3);
    let external = Arc::new(AtomicUsize::new(0));
    let alias = Arc::clone(&external);
    let before = d.snapshot();
    assert_eq!(d.advance(|_, _, j| {
        alias.fetch_add(1, Ordering::SeqCst);
        emit_one(j);
        Err(CallbackFault::Recoverable)
    }), Advance::Rejected(Rejection::RecoverableCallback));
    assert_eq!(external.load(Ordering::SeqCst), 1);
    assert_state_bits(d.snapshot(), before);
    assert!(d.pending().is_none());
    assert_eq!(d.counters().attempted_events, 1);
}

#[test]
fn d4_existing_d2_stage_callback_failure_drops_two_owned_rhs_events() {
    let mut d = driver(1.0, 1.0e-5, 3);
    let before = d.snapshot();
    let mut partial = None;
    let mut entries = 0;
    let result = d.advance(|a, lease, journal| {
        let trial = stage_safe::attempt(&a.y, lease.h, 0.0, true,
            &mut |stage, y, out| {
                entries += 1;
                journal.emit(stage as u32, y[0].to_bits().to_le_bytes().to_vec()).unwrap();
                if stage == 1 { return Err(stage_safe::CallbackFault::Recoverable); }
                for k in 0..30 { out[k] = -trajectory::rate(k) * y[k]; }
                Ok(())
            }, &mut |_, _, inside, called| { assert!(inside); assert!(called); });
        let failure = trial.unwrap_err();
        partial = Some(failure.work);
        Err(CallbackFault::Recoverable)
    });
    assert_eq!(result, Advance::Rejected(Rejection::RecoverableCallback));
    assert_eq!(partial, Some(stage_safe::Work { rhs: 2, solves: 1, checked: 2 }));
    assert_eq!(entries, 2);
    assert_state_bits(d.snapshot(), before);
    assert!(d.pending().is_none());
    assert_eq!((d.attempts(), d.rejected(), d.counters().attempted_events), (1, 1, 2));
}

#[test]
fn d4_existing_d2_stage_success_publishes_eight_rhs_records_after_admission() {
    let mut d = driver(1.0e-5, 1.0e-5, 2);
    let mut work = None;
    let id = accepted(d.advance(|a, lease, journal| {
        let trial = stage_safe::attempt(&a.y, lease.h, 0.0, true,
            &mut |stage, y, out| {
                journal.emit(stage as u32, y[0].to_bits().to_le_bytes().to_vec()).unwrap();
                for k in 0..30 { out[k] = -trajectory::rate(k) * y[k]; }
                Ok(())
            }, &mut |_, _, inside, called| { assert!(inside); assert!(called); }).unwrap();
        work = Some(trial.work);
        Ok(Candidate { state: trial.candidate, error: trial.error })
    }));
    assert_eq!(work, Some(stage_safe::Work { rhs: 8, solves: 8, checked: 8 }));
    assert_eq!(d.pending().unwrap().events().iter().map(|e| e.kind).collect::<Vec<_>>(), (0..8).collect::<Vec<_>>());
    assert_eq!(d.pending().unwrap().payload_bytes(), 64);
    assert_eq!((d.attempts(), d.accepted(), d.counters().attempted_events), (1, 1, 8));
    assert_eq!(d.advance(|_, _, _| panic!("final pending")), Advance::WaitingDelivery(id));
    assert_eq!(d.acknowledge(id), Ok(Ack::Acknowledged));
    assert_eq!(d.advance(|_, _, _| panic!("finished")), Advance::Finished);
}
