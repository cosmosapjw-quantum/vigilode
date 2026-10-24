//! Bounded owned effects around the unchanged D3 research Session.
//!
//! All producer effects covered by this contract are owned journal records.
//! Callback captures, arbitrary I/O, process abort and crash recovery are outside
//! that contract. Attempt/work accounting remains visible after refusal. The
//! inherited Session owns the attempt cap, clock and mathematical admission.
//!
//! An accepted step always creates one pending batch, including an empty batch.
//! No further producer callback is entered until that exact batch is acknowledged.
//! Delivery retries retain the same identifier and payload. Exactly-once external
//! application requires an atomic, durable idempotency mechanism in the sink;
//! this in-memory wrapper supplies neither durability nor that sink mechanism.
#![crate_name = "d4_effects"]
#![forbid(unsafe_code)]

extern crate d3_fixture;

use d3_fixture::admission::{Accepted, Fault, Lease, Metric, Session};
use d3_fixture::integer_gate::Decision;
use std::panic::{catch_unwind, AssertUnwindSafe};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub kind: u32,
    pub payload: Vec<u8>,
}

/// Limits on record count and payload bytes, excluding Vec allocation overhead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JournalLimits {
    pub max_events: usize,
    pub max_payload_bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JournalFault {
    EventCountOverflow,
    PayloadCountOverflow,
    EventLimit,
    PayloadLimit,
}

#[derive(Debug)]
pub struct Journal {
    limits: JournalLimits,
    events: Vec<Event>,
    payload_bytes: usize,
    poison: Option<JournalFault>,
    attempted_events: u64,
    attempted_payload_bytes: u64,
}

impl Journal {
    fn new(limits: JournalLimits) -> Self {
        Self {
            limits,
            events: Vec::new(),
            payload_bytes: 0,
            poison: None,
            attempted_events: 0,
            attempted_payload_bytes: 0,
        }
    }

    /// The first refusal poisons the entire trial, even if the caller ignores Err.
    pub fn emit(&mut self, kind: u32, payload: Vec<u8>) -> Result<(), JournalFault> {
        self.attempted_events = self.attempted_events.saturating_add(1);
        self.attempted_payload_bytes = self
            .attempted_payload_bytes
            .saturating_add(u64::try_from(payload.len()).unwrap_or(u64::MAX));
        if let Some(fault) = self.poison {
            return Err(fault);
        }
        let check = (|| {
            let count = self.events.len().checked_add(1)
                .ok_or(JournalFault::EventCountOverflow)?;
            let bytes = self.payload_bytes.checked_add(payload.len())
                .ok_or(JournalFault::PayloadCountOverflow)?;
            if count > self.limits.max_events {
                return Err(JournalFault::EventLimit);
            }
            if bytes > self.limits.max_payload_bytes {
                return Err(JournalFault::PayloadLimit);
            }
            Ok(bytes)
        })();
        match check {
            Ok(bytes) => {
                self.events.push(Event { kind, payload });
                self.payload_bytes = bytes;
                Ok(())
            }
            Err(fault) => {
                self.poison = Some(fault);
                Err(fault)
            }
        }
    }

    pub fn poison(&self) -> Option<JournalFault> { self.poison }
    pub fn len(&self) -> usize { self.events.len() }
    pub fn is_empty(&self) -> bool { self.events.is_empty() }
    pub fn payload_bytes(&self) -> usize { self.payload_bytes }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BatchId {
    /// The caller must use a distinct identifier for each independent run.
    pub run_id: u128,
    pub attempt: u64,
    pub epoch: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Batch {
    id: BatchId,
    events: Vec<Event>,
    payload_bytes: usize,
}

impl Batch {
    pub fn id(&self) -> BatchId { self.id }
    pub fn events(&self) -> &[Event] { &self.events }
    pub fn payload_bytes(&self) -> usize { self.payload_bytes }
}

#[derive(Clone, Copy, Debug)]
pub struct Candidate {
    pub state: [f64; 30],
    pub error: [f64; 30],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallbackFault { Recoverable, Fatal }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terminal {
    Session(Fault),
    CallbackFatal,
    CallbackPanic,
    Journal(JournalFault),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejection { RecoverableCallback, Norm, Domain }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Advance {
    Accepted(BatchId),
    Rejected(Rejection),
    WaitingDelivery(BatchId),
    Finished,
    Terminated(Terminal),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Delivery<E> {
    NoPending,
    Delivered(BatchId),
    Failed { id: BatchId, error: E },
    Panicked(BatchId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ack { Acknowledged, AlreadyAcknowledged }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AckError {
    NoPending(BatchId),
    WrongBatch { expected: BatchId, received: BatchId },
}

/// Diagnostic counters saturate rather than wrapping; they grant no admission.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counters {
    pub producer_calls: u64,
    pub attempted_events: u64,
    pub attempted_payload_bytes: u64,
    pub recoverable_callbacks: u64,
    pub fatal_callbacks: u64,
    pub producer_panics: u64,
    pub poisoned_journals: u64,
    pub accepted_batches: u64,
    pub acknowledged_batches: u64,
    pub delivery_calls: u64,
    pub delivery_errors: u64,
    pub delivery_panics: u64,
}

pub struct Driver {
    session: Session,
    metric: Metric,
    limits: JournalLimits,
    run_id: u128,
    pending: Option<Batch>,
    last_ack: Option<BatchId>,
    terminal: Option<Terminal>,
    counters: Counters,
}

impl Driver {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        run_id: u128,
        t: f64,
        tf: f64,
        state: [f64; 30],
        h: f64,
        attempt_cap: u64,
        metric: Metric,
        limits: JournalLimits,
    ) -> Result<Self, Fault> {
        Ok(Self {
            session: Session::new_nonnegative(t, tf, state, h, attempt_cap)?,
            metric,
            limits,
            run_id,
            pending: None,
            last_ack: None,
            terminal: None,
            counters: Counters::default(),
        })
    }

    pub fn snapshot(&self) -> Accepted { self.session.snapshot() }
    pub fn attempts(&self) -> u64 { self.session.attempts() }
    pub fn accepted(&self) -> u64 { self.session.accepted() }
    pub fn rejected(&self) -> u64 { self.session.rejected() }
    pub fn domain_rejected(&self) -> u64 { self.session.domain_rejected() }
    pub fn counters(&self) -> Counters { self.counters }
    pub fn pending(&self) -> Option<&Batch> { self.pending.as_ref() }
    pub fn terminal(&self) -> Option<Terminal> { self.terminal }

    fn stop_trial(&mut self, lease: Lease, terminal: Terminal) -> Advance {
        let state = match self.session.abort_stage(lease, false) {
            Ok(()) => terminal,
            Err(fault) => Terminal::Session(fault),
        };
        self.terminal = Some(state);
        Advance::Terminated(state)
    }

    pub fn advance<F>(&mut self, mut producer: F) -> Advance
    where
        F: FnMut(&Accepted, Lease, &mut Journal) -> Result<Candidate, CallbackFault>,
    {
        // Delivery takes precedence even after the last scientific time point.
        if let Some(batch) = &self.pending {
            return Advance::WaitingDelivery(batch.id);
        }
        if let Some(state) = self.terminal {
            return Advance::Terminated(state);
        }
        let lease = match self.session.begin() {
            Ok(lease) => lease,
            Err(Fault::Finished) => return Advance::Finished,
            Err(fault) => {
                let terminal = Terminal::Session(fault);
                self.terminal = Some(terminal);
                return Advance::Terminated(terminal);
            }
        };
        let accepted = self.session.snapshot();
        let mut journal = Journal::new(self.limits);
        self.counters.producer_calls = self.counters.producer_calls.saturating_add(1);
        let result = catch_unwind(AssertUnwindSafe(|| producer(&accepted, lease, &mut journal)));
        self.counters.attempted_events = self.counters.attempted_events
            .saturating_add(journal.attempted_events);
        self.counters.attempted_payload_bytes = self.counters.attempted_payload_bytes
            .saturating_add(journal.attempted_payload_bytes);

        // An unwind is terminal; AssertUnwindSafe does not roll back captures.
        if result.is_err() {
            self.counters.producer_panics = self.counters.producer_panics.saturating_add(1);
            return self.stop_trial(lease, Terminal::CallbackPanic);
        }
        // An ignored emit error cannot reach mathematical Session::finish.
        if let Some(fault) = journal.poison {
            self.counters.poisoned_journals = self.counters.poisoned_journals.saturating_add(1);
            return self.stop_trial(lease, Terminal::Journal(fault));
        }
        let candidate = match result.expect("unwind checked above") {
            Ok(candidate) => candidate,
            Err(CallbackFault::Recoverable) => {
                self.counters.recoverable_callbacks = self.counters.recoverable_callbacks.saturating_add(1);
                return match self.session.abort_stage(lease, true) {
                    Ok(()) => Advance::Rejected(Rejection::RecoverableCallback),
                    Err(fault) => {
                        let terminal = Terminal::Session(fault);
                        self.terminal = Some(terminal);
                        Advance::Terminated(terminal)
                    }
                };
            }
            Err(CallbackFault::Fatal) => {
                self.counters.fatal_callbacks = self.counters.fatal_callbacks.saturating_add(1);
                return self.stop_trial(lease, Terminal::CallbackFatal);
            }
        };
        let outcome = match self.session.finish(lease, candidate.state, candidate.error, &self.metric) {
            Ok(outcome) => outcome,
            Err(fault) => {
                let terminal = Terminal::Session(fault);
                self.terminal = Some(terminal);
                return Advance::Terminated(terminal);
            }
        };
        match outcome.decision {
            Decision::Accept => {
                let id = BatchId { run_id: self.run_id, attempt: lease.id, epoch: self.session.snapshot().epoch };
                // Move already prepared owned data: no callback/allocation here.
                self.pending = Some(Batch { id, events: journal.events, payload_bytes: journal.payload_bytes });
                self.counters.accepted_batches = self.counters.accepted_batches.saturating_add(1);
                Advance::Accepted(id)
            }
            Decision::Reject => Advance::Rejected(if outcome.domain_violation.is_some() {
                Rejection::Domain
            } else {
                Rejection::Norm
            }),
            Decision::Indeterminate => {
                let terminal = Terminal::Session(Fault::NeedsPrecision);
                self.terminal = Some(terminal);
                Advance::Terminated(terminal)
            }
        }
    }

    /// One sink entry at most per invocation. Success still needs an explicit ack.
    pub fn deliver_once<F, E>(&mut self, mut sink: F) -> Delivery<E>
    where F: FnMut(&Batch) -> Result<(), E> {
        let Some(batch) = self.pending.as_ref() else { return Delivery::NoPending; };
        let id = batch.id;
        self.counters.delivery_calls = self.counters.delivery_calls.saturating_add(1);
        match catch_unwind(AssertUnwindSafe(|| sink(batch))) {
            Ok(Ok(())) => Delivery::Delivered(id),
            Ok(Err(error)) => {
                self.counters.delivery_errors = self.counters.delivery_errors.saturating_add(1);
                Delivery::Failed { id, error }
            }
            Err(_) => {
                self.counters.delivery_panics = self.counters.delivery_panics.saturating_add(1);
                Delivery::Panicked(id)
            }
        }
    }

    /// An acknowledgement is a caller assertion about sink completion.
    /// Matching an identifier is not cryptographic proof of external delivery.
    pub fn acknowledge(&mut self, received: BatchId) -> Result<Ack, AckError> {
        if self.last_ack == Some(received) {
            return Ok(Ack::AlreadyAcknowledged);
        }
        let Some(batch) = self.pending.as_ref() else {
            return Err(AckError::NoPending(received));
        };
        if batch.id != received {
            return Err(AckError::WrongBatch { expected: batch.id, received });
        }
        self.pending = None;
        self.last_ack = Some(received);
        self.counters.acknowledged_batches = self.counters.acknowledged_batches.saturating_add(1);
        Ok(Ack::Acknowledged)
    }
}
