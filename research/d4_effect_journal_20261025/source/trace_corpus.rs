//! Exhaustive short D4 protocol corpus; new adapter work only, no ODE stages.
//! Compile/link against the frozen d4_effects and d3_fixture crates.
//! Usage: trace_corpus NEW_OUTPUT.jsonl (create_new; existing files are preserved).
#![forbid(unsafe_code)]
extern crate d3_fixture;
extern crate d4_effects;

use d3_fixture::admission::{Fault, Metric};
use d4_effects::*;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::OpenOptions;
use std::io::{self, BufWriter, Write};

const ALPHABET: [u8; 8] = *b"ARFBLSKW";
const WORDS: u64 = 8 * 8 * 8 * 8;
const EVENT_KIND: u32 = 17;
const WRONG_RUN_OFFSET: u128 = 65_536;

fn optional(value: Option<u64>) -> String {
    value.map_or_else(|| "null".to_owned(), |v| v.to_string())
}
fn payload_value(events: &[Event]) -> Option<u64> {
    if events.len() != 1 || events[0].kind != EVENT_KIND || events[0].payload.len() != 8 {
        return None;
    }
    Some(u64::from_le_bytes(events[0].payload.as_slice().try_into().ok()?))
}
fn terminal_code(value: Option<Terminal>) -> &'static str {
    match value {
        None => "none",
        Some(Terminal::Session(Fault::Budget)) => "budget",
        Some(Terminal::CallbackFatal) => "fatal",
        Some(Terminal::CallbackPanic) => "panic",
        Some(Terminal::Journal(_)) => "journal",
        Some(Terminal::Session(_)) => "other_session",
    }
}
fn advance_code(value: Advance) -> &'static str {
    match value {
        Advance::Accepted(_) => "accepted",
        Advance::Rejected(Rejection::RecoverableCallback) => "recoverable",
        Advance::Rejected(_) => "other_rejection",
        Advance::WaitingDelivery(_) => "waiting",
        Advance::Finished => "finished",
        Advance::Terminated(Terminal::Session(Fault::Budget)) => "budget",
        Advance::Terminated(Terminal::CallbackFatal) => "fatal",
        Advance::Terminated(_) => "other_terminal",
    }
}

/// A controlled in-memory atomic receiver: duplicate IDs bind the identical
/// payload and produce no further application. This is not durable storage.
#[derive(Default)]
struct Receiver {
    bound: BTreeMap<BatchId, Vec<Event>>,
    successful_delivery_ids: BTreeSet<BatchId>,
    calls: u64,
    apply_requests: u64,
    applications: u64,
    dedup_hits: u64,
    binding_conflicts: u64,
    successes: u64,
}
impl Receiver {
    fn receive(&mut self, action: u8, batch: &Batch) -> Result<(), &'static str> {
        self.calls += 1;
        if action == b'B' {
            return Err("before_apply");
        }
        self.apply_requests += 1;
        if let Some(old) = self.bound.get(&batch.id()) {
            if old.as_slice() != batch.events() {
                self.binding_conflicts += 1;
                return Err("id_payload_conflict");
            }
            self.dedup_hits += 1;
        } else {
            self.bound.insert(batch.id(), batch.events().to_vec());
            self.applications += 1;
        }
        if action == b'L' {
            return Err("applied_ack_lost");
        }
        self.successes += 1;
        self.successful_delivery_ids.insert(batch.id());
        Ok(())
    }
    fn records_json(&self) -> String {
        let records: Vec<String> = self.bound.iter().map(|(id, events)| {
            format!(
                "{{\"run_id\":{},\"attempt\":{},\"epoch\":{},\"payload\":{},\"kind\":{},\"bytes\":{}}}",
                id.run_id, id.attempt, id.epoch, optional(payload_value(events)),
                events.first().map_or(0, |e| e.kind),
                events.iter().map(|e| e.payload.len()).sum::<usize>()
            )
        }).collect();
        format!("[{}]", records.join(","))
    }
}
#[derive(Default)]
struct AckObservations {
    last: Option<BatchId>,
    requests: u64,
    acknowledged: u64,
    duplicates: u64,
    errors: u64,
    wrong_errors: u64,
    no_pending_errors: u64,
    without_sink_effect: u64,
    without_success: u64,
}
fn execute_case<W: Write>(output: &mut W, cap: u64, word: u64) -> io::Result<()> {
    let case_id = cap * WORDS + word;
    let run_id = u128::from(case_id + 1);
    let sequence: [u8; 4] = std::array::from_fn(|i| {
        ALPHABET[((word >> (3 * (3 - i))) & 7) as usize]
    });
    let sequence_string = String::from_utf8(sequence.to_vec()).expect("ASCII alphabet");
    let mut driver = Driver::new(
        run_id, 0.0, 1.0e6, [0.0; 30], 1.0, cap,
        Metric { atol: [1.0e-3; 33], rtol: [0.0; 33] },
        JournalLimits { max_events: 1, max_payload_bytes: 8 },
    ).map_err(|e| io::Error::new(io::ErrorKind::Other, format!("fixture_init:{e:?}")))?;
    let mut receiver = Receiver::default();
    let mut ack = AckObservations::default();
    let mut producer_observed = 0u64;
    for (step, &action) in sequence.iter().enumerate() {
        let result: &'static str;
        let mut ack_received: Option<BatchId> = None;
        match action {
            b'A' | b'R' | b'F' => {
                let outcome = driver.advance(|accepted, lease, journal| {
                    producer_observed += 1;
                    // One owned record is attempted even for rejected/fatal trials.
                    journal.emit(EVENT_KIND, (case_id + lease.id).to_le_bytes().to_vec())
                        .expect("frozen one-event/eight-byte fixture");
                    match action {
                        b'R' => Err(CallbackFault::Recoverable),
                        b'F' => Err(CallbackFault::Fatal),
                        _ => Ok(Candidate { state: accepted.y, error: [0.0; 30] }),
                    }
                });
                result = advance_code(outcome);
            }
            b'B' | b'L' | b'S' => {
                result = match driver.deliver_once(|batch| receiver.receive(action, batch)) {
                    Delivery::NoPending => "no_pending",
                    Delivery::Delivered(_) => "delivered",
                    Delivery::Failed { .. } => "delivery_error",
                    Delivery::Panicked(_) => "delivery_panic",
                };
            }
            b'K' | b'W' => {
                ack.requests += 1;
                let exact = driver.pending().map(Batch::id).or(ack.last)
                    .unwrap_or(BatchId { run_id, attempt: 0, epoch: 0 });
                let received = if action == b'K' { exact } else {
                    BatchId { run_id: run_id + WRONG_RUN_OFFSET, ..exact }
                };
                ack_received = Some(received);
                result = match driver.acknowledge(received) {
                    Ok(Ack::Acknowledged) => {
                        ack.acknowledged += 1;
                        ack.last = Some(received);
                        if !receiver.bound.contains_key(&received) {
                            ack.without_sink_effect += 1;
                        }
                        if !receiver.successful_delivery_ids.contains(&received) {
                            ack.without_success += 1;
                        }
                        "acknowledged"
                    }
                    Ok(Ack::AlreadyAcknowledged) => {
                        ack.duplicates += 1;
                        "already_acknowledged"
                    }
                    Err(AckError::WrongBatch { .. }) => {
                        ack.errors += 1;
                        ack.wrong_errors += 1;
                        "wrong_ack"
                    }
                    Err(AckError::NoPending(_)) => {
                        ack.errors += 1;
                        ack.no_pending_errors += 1;
                        "no_pending_ack"
                    }
                };
            }
            _ => unreachable!("frozen alphabet"),
        }
        let pending = driver.pending();
        let pending_id = pending.map(Batch::id);
        let state = driver.snapshot();
        let counters = driver.counters();
        writeln!(output,
            concat!(
                "{{\"schema\":\"d4.protocol.trace.v1\",",
                "\"case_id\":{},\"word\":{},\"run_id\":{},\"sequence\":\"{}\",\"step\":{},\"action\":\"{}\",\"cap\":{},",
                "\"result\":\"{}\",\"attempts\":{},\"accepted\":{},\"rejected\":{},\"producer\":{},\"producer_observed\":{},",
                "\"attempted_events\":{},\"attempted_payload_bytes\":{},\"recoverable_callbacks\":{},\"fatal_callbacks\":{},\"domain_rejected\":{},",
                "\"pending_run_id\":{},\"pending_attempt\":{},\"pending_epoch\":{},\"pending_payload\":{},\"pending_events\":{},\"pending_bytes\":{},",
                "\"terminalkind\":\"{}\",\"epoch\":{},\"t_bits\":{},\"state_zero\":{},",
                "\"deliverycalls\":{},\"deliveryerrors\":{},\"sink_calls\":{},\"apply_requests\":{},\"deliverysuccesses\":{},",
                "\"published\":{},\"sink_unique_batches\":{},\"dedup_hits\":{},\"binding_conflicts\":{},\"published_records\":{},",
                "\"ack_requests\":{},\"acknowledged\":{},\"acknowledged_observed\":{},\"ack_duplicates\":{},\"ack_errors\":{},\"wrong_ack_errors\":{},\"no_pending_ack_errors\":{},",
                "\"acks_without_sink_effect\":{},\"acks_without_success\":{},\"last_ack_attempt\":{},\"last_ack_epoch\":{},",
                "\"ack_received_run_id\":{},\"ack_received_attempt\":{},\"ack_received_epoch\":{}}}"
            ),
            case_id, word, run_id, sequence_string, step, char::from(action), cap,
            result, driver.attempts(), driver.accepted(), driver.rejected(),
            counters.producer_calls, producer_observed,
            counters.attempted_events, counters.attempted_payload_bytes,
            counters.recoverable_callbacks, counters.fatal_callbacks, driver.domain_rejected(),
            pending_id.map_or_else(|| "null".to_owned(), |id| id.run_id.to_string()),
            optional(pending_id.map(|id| id.attempt)),
            optional(pending_id.map(|id| id.epoch)),
            optional(pending.and_then(|b| payload_value(b.events()))),
            pending.map_or(0, |b| b.events().len()), pending.map_or(0, Batch::payload_bytes),
            terminal_code(driver.terminal()), state.epoch, state.t.to_bits(),
            state.y.iter().all(|v| v.to_bits() == 0),
            counters.delivery_calls, counters.delivery_errors, receiver.calls,
            receiver.apply_requests, receiver.successes,
            receiver.applications, receiver.bound.len(), receiver.dedup_hits,
            receiver.binding_conflicts, receiver.records_json(),
            ack.requests, counters.acknowledged_batches, ack.acknowledged, ack.duplicates,
            ack.errors, ack.wrong_errors, ack.no_pending_errors,
            ack.without_sink_effect, ack.without_success,
            optional(ack.last.map(|id| id.attempt)), optional(ack.last.map(|id| id.epoch)),
            ack_received.map_or_else(|| "null".to_owned(), |id| id.run_id.to_string()),
            optional(ack_received.map(|id| id.attempt)),
            optional(ack_received.map(|id| id.epoch)),
        )?;
    }
    Ok(())
}
fn main() -> io::Result<()> {
    let mut args = env::args_os();
    let _program = args.next();
    let destination = args.next().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "usage: trace_corpus NEW_OUTPUT.jsonl")
    })?;
    if args.next().is_some() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "exactly one output path required"));
    }
    let file = OpenOptions::new().write(true).create_new(true).open(destination)?;
    let mut output = BufWriter::new(file);
    for cap in 0..=3 {
        for word in 0..WORDS {
            execute_case(&mut output, cap, word)?;
        }
    }
    output.flush()?;
    println!("{{\"cases\":16384,\"rows\":65536,\"alphabet\":\"ARFBLSKW\",\"sequence_length\":4,\"caps\":[0,1,2,3],\"ode_stage_calls\":0}}");
    Ok(())
}

