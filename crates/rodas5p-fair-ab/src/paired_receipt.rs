//! Raw receipts binding a paired wall-time decision to observed sessions
//! (re-audit R3 of 2026-10-01, STAT-DEV-02).
//!
//! A [`PairedTimingAssessment`] alone is a summary: its gate is re-derived
//! from its own fields, and [`PairedTimingAssessment::verify_against_raw`]
//! needs the raw cases. A [`PairedTimingEvidence`] carries both, plus the
//! campaign identity, the process that measured each session, the arm
//! identities, and every failure a session met. A consumer gates only on
//! [`PairedTimingEvidence::verified_decision`], which recomputes the
//! assessment from the raw cases and checks the provenance; without a
//! receipt the wall criterion is not evaluated.
//!
//! [`measure_paired_session`] measures one session in the calling process
//! with a monotonic clock; a campaign runs it in separate processes (the CLI
//! `paired-timing-campaign` command) and merges the sessions with
//! [`PairedTimingReceipt::from_sessions`].

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::paired_timing::session_abba_order;
use crate::{
    FairError, FairResult, PairedTimingAssessment, PairedTimingCase, PairedTimingDecision,
    PairedTimingProtocol, TimingHostMetadata, abba_pair_order, assess_paired_timing,
    calibrate_batch_iterations, detect_timing_host_metadata,
};

pub const PAIRED_TIMING_RECEIPT_SCHEMA: &str = "vigilode-paired-timing-receipt-v1";
/// The only clock a receipt may name: `std::time::Instant` (monotonic).
pub const PAIRED_TIMING_MONOTONIC_CLOCK: &str = "std::time::Instant";

/// What an arm ran: the executable (by content hash) and the workload.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArmIdentity {
    pub arm_id: String,
    pub executable_sha256: String,
    pub workload_id: String,
}

/// One measured session: a separate process with its own clock origin.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionProvenance {
    pub session: u32,
    pub process_id: u32,
    pub started_unix_seconds: f64,
    pub finished_unix_seconds: f64,
    pub clock: String,
    pub host: TimingHostMetadata,
}

/// A case an arm could not complete in a session. Failures are kept in the
/// receipt; a receipt with any failure never gates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionFailure {
    pub session: u32,
    pub case_id: String,
    pub message: String,
}

/// The raw output of one session: its provenance, the candidate-against-
/// reference cases, the reference-against-reference (A/A) cases, and the
/// failures.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub campaign_id: String,
    pub provenance: SessionProvenance,
    pub cases: Vec<PairedTimingCase>,
    pub aa_cases: Vec<PairedTimingCase>,
    pub failures: Vec<SessionFailure>,
}

/// A case to time: `candidate` and `reference` run one iteration each.
pub struct PairedWorkload<'a> {
    pub case_id: String,
    pub candidate: Box<dyn FnMut() -> FairResult<()> + 'a>,
    pub reference: Box<dyn FnMut() -> FairResult<()> + 'a>,
}

fn unix_seconds() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |elapsed| elapsed.as_secs_f64())
}

/// Warmups, then `protocol.pairs` batched pairs in seeded ABBA order, with
/// `batch` iterations per sample when given (a campaign fixes the batch of
/// every session to the first session's calibration) and the calibrated
/// batch otherwise.
fn measure_case(
    case_id: &str,
    session: u32,
    protocol: &PairedTimingProtocol,
    batch: Option<usize>,
    candidate: &mut dyn FnMut() -> FairResult<()>,
    reference: &mut dyn FnMut() -> FairResult<()>,
) -> FairResult<PairedTimingCase> {
    let origin = Instant::now();
    let clock = || origin.elapsed().as_secs_f64();
    let mut run = |first_is_candidate: bool, iterations: usize| -> FairResult<f64> {
        let started = clock();
        for _ in 0..iterations {
            if first_is_candidate {
                candidate()?;
            } else {
                reference()?;
            }
        }
        Ok(((clock() - started) / iterations as f64).max(0.0))
    };
    let mut warmup_seconds = Vec::with_capacity(2 * protocol.warmups);
    for round in 0..protocol.warmups {
        let first = round % 2 == 0;
        warmup_seconds.push(run(first, 1)?);
        warmup_seconds.push(run(!first, 1)?);
    }
    let batch_iterations = match batch {
        Some(batch) => batch,
        None => calibrate_batch_iterations(&warmup_seconds, protocol)?,
    };
    let order = abba_pair_order(protocol.pairs, protocol.seed);
    let mut candidate_seconds = Vec::with_capacity(protocol.pairs);
    let mut reference_seconds = Vec::with_capacity(protocol.pairs);
    for arms in &order {
        for arm in arms {
            let is_candidate = *arm == crate::PairedArm::Candidate;
            let seconds = run(is_candidate, batch_iterations)?;
            if is_candidate {
                candidate_seconds.push(seconds);
            } else {
                reference_seconds.push(seconds);
            }
        }
    }
    Ok(PairedTimingCase {
        case_id: case_id.to_owned(),
        batch_iterations,
        warmup_seconds,
        order,
        candidate_seconds,
        reference_seconds,
        process_blocks: vec![session; protocol.pairs],
    })
}

/// Measure one session in this process. Each workload is timed against its
/// reference and, as the A/A control, its reference against itself. An arm
/// that fails ends that case and is recorded as a failure; the session goes
/// on. `batches` maps a case to the batch fixed by the first session.
pub fn measure_paired_session(
    campaign_id: &str,
    session: u32,
    protocol: &PairedTimingProtocol,
    workloads: &mut [PairedWorkload<'_>],
    batches: &BTreeMap<String, usize>,
) -> FairResult<SessionRecord> {
    protocol.validate()?;
    let started = unix_seconds();
    let mut cases = Vec::new();
    let mut aa_cases = Vec::new();
    let mut failures = Vec::new();
    for workload in workloads.iter_mut() {
        let batch = batches.get(&workload.case_id).copied();
        let measured = measure_case(
            &workload.case_id,
            session,
            protocol,
            batch,
            &mut *workload.candidate,
            &mut *workload.reference,
        );
        match measured {
            Ok(case) => cases.push(case),
            Err(error) => {
                failures.push(SessionFailure {
                    session,
                    case_id: workload.case_id.clone(),
                    message: format!("candidate against reference: {error}"),
                });
                continue;
            }
        }
        // A/A: the reference arm against itself, with the same batch.
        let aa_batch = batch.or_else(|| cases.last().map(|case| case.batch_iterations));
        let reference = std::cell::RefCell::new(&mut workload.reference);
        let mut left = || (reference.borrow_mut())();
        let mut right = || (reference.borrow_mut())();
        let aa = measure_case(
            &format!("{}#aa", workload.case_id),
            session,
            protocol,
            aa_batch,
            &mut left,
            &mut right,
        );
        match aa {
            Ok(case) => aa_cases.push(case),
            Err(error) => failures.push(SessionFailure {
                session,
                case_id: format!("{}#aa", workload.case_id),
                message: format!("A/A control: {error}"),
            }),
        }
    }
    Ok(SessionRecord {
        campaign_id: campaign_id.to_owned(),
        provenance: SessionProvenance {
            session,
            process_id: std::process::id(),
            started_unix_seconds: started,
            finished_unix_seconds: unix_seconds(),
            clock: PAIRED_TIMING_MONOTONIC_CLOCK.into(),
            host: detect_timing_host_metadata(1),
        },
        cases,
        aa_cases,
        failures,
    })
}

/// Concatenate one case's per-session measurements, in session order. The
/// merged case keeps the first session's warmups and batch (every session
/// ran that batch) and its order is [`session_abba_order`] of the labels.
pub fn merge_session_cases(parts: &[&PairedTimingCase]) -> FairResult<PairedTimingCase> {
    let Some(first) = parts.first() else {
        return Err(FairError::Invalid("no session measured this case".into()));
    };
    let mut merged = PairedTimingCase {
        case_id: first.case_id.clone(),
        batch_iterations: first.batch_iterations,
        warmup_seconds: first.warmup_seconds.clone(),
        order: Vec::new(),
        candidate_seconds: Vec::new(),
        reference_seconds: Vec::new(),
        process_blocks: Vec::new(),
    };
    for part in parts {
        if part.case_id != merged.case_id || part.batch_iterations != merged.batch_iterations {
            return Err(FairError::Invalid(format!(
                "session parts of case {} disagree on identity or batch",
                merged.case_id
            )));
        }
        merged.order.extend_from_slice(&part.order);
        merged
            .candidate_seconds
            .extend_from_slice(&part.candidate_seconds);
        merged
            .reference_seconds
            .extend_from_slice(&part.reference_seconds);
        merged
            .process_blocks
            .extend_from_slice(&part.process_blocks);
    }
    Ok(merged)
}

/// Everything a wall decision was computed from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PairedTimingReceipt {
    pub schema: String,
    pub campaign_id: String,
    pub candidate: ArmIdentity,
    pub reference: ArmIdentity,
    pub protocol: PairedTimingProtocol,
    pub sessions: Vec<SessionProvenance>,
    pub cases: Vec<PairedTimingCase>,
    pub aa_cases: Vec<PairedTimingCase>,
    pub failures: Vec<SessionFailure>,
    /// Per-session raw records exactly as the session processes wrote
    /// them, including their own warmups.
    pub session_records: Vec<SessionRecord>,
}

impl PairedTimingReceipt {
    /// Merge session records of one campaign into a receipt.
    pub fn from_sessions(
        campaign_id: &str,
        candidate: ArmIdentity,
        reference: ArmIdentity,
        protocol: PairedTimingProtocol,
        mut records: Vec<SessionRecord>,
    ) -> FairResult<Self> {
        records.sort_by_key(|record| record.provenance.session);
        let merge = |select: fn(&SessionRecord) -> &Vec<PairedTimingCase>| {
            let mut by_case = BTreeMap::<String, Vec<&PairedTimingCase>>::new();
            for record in &records {
                for case in select(record) {
                    by_case.entry(case.case_id.clone()).or_default().push(case);
                }
            }
            by_case
                .values()
                .map(|parts| merge_session_cases(parts))
                .collect::<FairResult<Vec<_>>>()
        };
        let cases = merge(|record| &record.cases)?;
        let aa_cases = merge(|record| &record.aa_cases)?;
        let receipt = Self {
            schema: PAIRED_TIMING_RECEIPT_SCHEMA.into(),
            campaign_id: campaign_id.to_owned(),
            candidate,
            reference,
            protocol,
            sessions: records
                .iter()
                .map(|record| record.provenance.clone())
                .collect(),
            cases,
            aa_cases,
            failures: records
                .iter()
                .flat_map(|record| record.failures.clone())
                .collect(),
            session_records: records,
        };
        receipt.validate()?;
        Ok(receipt)
    }

    /// Provenance checks: current schema, a campaign identity carried by
    /// every session record, one monotonic clock, distinct sessions measured
    /// by distinct processes, and every session label used by a case
    /// declared by a session record of this campaign. The merged cases must
    /// be the session records' own measurements.
    pub fn validate(&self) -> FairResult<()> {
        let reject = |reason: String| {
            Err(FairError::Invalid(format!(
                "TIMING_NOT_EVALUATED: {reason}"
            )))
        };
        if self.schema != PAIRED_TIMING_RECEIPT_SCHEMA {
            return reject("receipt schema is not the current one".into());
        }
        if self.campaign_id.trim().is_empty() {
            return reject("receipt has no campaign identity".into());
        }
        if self.candidate.executable_sha256.is_empty()
            || self.reference.executable_sha256.is_empty()
        {
            return reject("arm identities must name their executables".into());
        }
        let mut sessions = BTreeSet::new();
        let mut processes = BTreeSet::new();
        for session in &self.sessions {
            if session.clock != PAIRED_TIMING_MONOTONIC_CLOCK {
                return reject(format!(
                    "session {} used clock {}",
                    session.session, session.clock
                ));
            }
            if !sessions.insert(session.session) || !processes.insert(session.process_id) {
                return reject(format!(
                    "session {} is not a distinct session in a distinct process",
                    session.session
                ));
            }
        }
        if self.session_records.len() != self.sessions.len()
            || self
                .session_records
                .iter()
                .zip(&self.sessions)
                .any(|(record, session)| {
                    record.campaign_id != self.campaign_id || &record.provenance != session
                })
        {
            return reject("session records do not match the declared sessions".into());
        }
        for case in self.cases.iter().chain(&self.aa_cases) {
            if case
                .process_blocks
                .iter()
                .any(|label| !sessions.contains(label))
            {
                return reject(format!(
                    "case {} uses a session no record of this campaign declares",
                    case.case_id
                ));
            }
            if case.order != session_abba_order(&case.process_blocks, self.protocol.seed) {
                return reject(format!(
                    "case {} is not the concatenation of its sessions",
                    case.case_id
                ));
            }
        }
        let rebuilt = Self::from_sessions_unchecked(self);
        if rebuilt.0 != self.cases || rebuilt.1 != self.aa_cases {
            return reject("merged cases differ from the session records".into());
        }
        Ok(())
    }

    fn from_sessions_unchecked(&self) -> (Vec<PairedTimingCase>, Vec<PairedTimingCase>) {
        let merge = |select: fn(&SessionRecord) -> &Vec<PairedTimingCase>| {
            let mut by_case = BTreeMap::<String, Vec<&PairedTimingCase>>::new();
            for record in &self.session_records {
                for case in select(record) {
                    by_case.entry(case.case_id.clone()).or_default().push(case);
                }
            }
            by_case
                .values()
                .filter_map(|parts| merge_session_cases(parts).ok())
                .collect::<Vec<_>>()
        };
        (
            merge(|record| &record.cases),
            merge(|record| &record.aa_cases),
        )
    }

    /// Assess the receipt's raw cases.
    pub fn assess(&self) -> FairResult<PairedTimingAssessment> {
        self.validate()?;
        let host = self
            .sessions
            .first()
            .map(|session| session.host.clone())
            .unwrap_or_else(|| detect_timing_host_metadata(1));
        assess_paired_timing(&self.protocol, &self.cases, self.aa_option(), host)
    }

    /// The A/A cases, or `None` when the campaign measured none.
    fn aa_option(&self) -> Option<&[PairedTimingCase]> {
        (!self.aa_cases.is_empty()).then_some(self.aa_cases.as_slice())
    }
}

/// A wall decision with the receipt it was computed from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PairedTimingEvidence {
    pub assessment: PairedTimingAssessment,
    pub receipt: PairedTimingReceipt,
}

impl PairedTimingEvidence {
    pub fn from_receipt(receipt: PairedTimingReceipt) -> FairResult<Self> {
        Ok(Self {
            assessment: receipt.assess()?,
            receipt,
        })
    }

    /// The only gate a consumer may use: the receipt's provenance checks,
    /// the assessment recomputed bit for bit from its raw cases, and no
    /// failure in any session (a failed case would otherwise drop out of
    /// the denominator silently).
    pub fn verified_decision(&self) -> FairResult<PairedTimingDecision> {
        self.receipt.validate()?;
        if self.assessment.protocol != self.receipt.protocol {
            return Err(FairError::Invalid(
                "TIMING_NOT_EVALUATED: assessment protocol differs from the receipt".into(),
            ));
        }
        let decision = self
            .assessment
            .verify_against_raw(&self.receipt.cases, self.receipt.aa_option())?;
        if !self.receipt.failures.is_empty() {
            return Ok(PairedTimingDecision::Inconclusive);
        }
        Ok(decision)
    }
}
