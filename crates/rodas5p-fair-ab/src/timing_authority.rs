//! Statistical authority of a paired wall-time decision, kept apart from the
//! decision itself (re-audit R4 of 2026-10-01, R4-STAT-DEV-01).
//!
//! [`crate::PairedTimingEvidence::verified_decision`] establishes integrity:
//! the assessment is the one its raw receipt implies. Whether that
//! assessment's interval may decide anything is a separate question, set by
//! the preregistered coverage studies of its design and estimand. Their
//! outcomes are compiled into [`timing_authority_registry`]; nothing a
//! receipt or a user JSON says can raise the status. A design covered by a
//! failed study, or by a passed one awaiting its independent review, is
//! [`TimingAuthorityStatus::Hold`], one covered by no study
//! is [`TimingAuthorityStatus::NotEvaluated`], and only a design covered by
//! admissible studies alone is [`TimingAuthorityStatus::Admissible`]. No
//! registry entry is admissible today: both R3 coverage studies failed
//! (ledger L-0007, L-0010), and the R4 authority gate admits nothing until
//! the session-median study has passed an independent domain review.

use serde::{Deserialize, Serialize};

use crate::{
    FairError, FairResult, PAIRED_TIMING_SCHEMA, POOLED_PAIR_MEDIAN_ESTIMAND, PairedTimingDecision,
    PairedTimingEvidence, SESSION_CELL_MEDIAN_ESTIMAND,
};

/// The schema the session-median interval designs are registered under.
pub const SESSION_MEDIAN_INTERVAL_SCHEMA: &str = "vigilode-r4-exact-session-median-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TimingAuthorityStatus {
    /// A study covering the design failed or awaits review.
    Hold,
    /// Every study covering the design passed and was reviewed.
    Admissible,
    /// No study covers the design.
    NotEvaluated,
}

/// The designs a study's verdict covers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimingAuthorityDomain {
    pub estimand: String,
    pub assessment_schema: String,
    pub min_sessions: usize,
    pub max_sessions: usize,
    /// When present, the session counts the study simulated; designs with
    /// another count inside `min_sessions..=max_sessions` are not covered.
    #[serde(default)]
    pub session_counts: Option<Vec<usize>>,
    pub case_counts: Vec<usize>,
}

impl TimingAuthorityDomain {
    pub fn covers(&self, design: &TimingDesignIdentity) -> bool {
        self.estimand == design.estimand
            && self.assessment_schema == design.assessment_schema
            && (self.min_sessions..=self.max_sessions).contains(&design.sessions)
            && self
                .session_counts
                .as_ref()
                .is_none_or(|counts| counts.contains(&design.sessions))
            && self.case_counts.contains(&design.cases)
    }
}

/// One preregistered study and its verdict.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimingAuthority {
    pub study_id: String,
    pub ledger_row: String,
    pub domain: TimingAuthorityDomain,
    pub status: TimingAuthorityStatus,
    pub reason: String,
}

/// What an assessment is: its estimand, schema and design size.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimingDesignIdentity {
    pub estimand: String,
    pub assessment_schema: String,
    pub sessions: usize,
    pub cases: usize,
}

/// The compiled study registry. Changing an entry is a code change tied to
/// a ledger row, never a field in a receipt.
pub fn timing_authority_registry() -> Vec<TimingAuthority> {
    let pooled = TimingAuthorityDomain {
        estimand: POOLED_PAIR_MEDIAN_ESTIMAND.into(),
        assessment_schema: PAIRED_TIMING_SCHEMA.into(),
        min_sessions: 1,
        max_sessions: usize::MAX,
        session_counts: None,
        case_counts: (1..=64).collect(),
    };
    vec![
        TimingAuthority {
            study_id: "r3_timing_coverage_study_20261001".into(),
            ledger_row: "L-0007".into(),
            domain: pooled.clone(),
            status: TimingAuthorityStatus::Hold,
            reason: "preregistered coverage study FAIL: the session-clustered bootstrap undercovers in its declared grid".into(),
        },
        TimingAuthority {
            study_id: "r3_timing_coverage_study_v2_20261001".into(),
            ledger_row: "L-0010".into(),
            domain: pooled,
            status: TimingAuthorityStatus::Hold,
            reason: "preregistered coverage study v2 FAIL (separate streams, scenario-keyed seeds): same undercoverage".into(),
        },
        TimingAuthority {
            study_id: "r4_session_median_coverage_20261001".into(),
            ledger_row: "L-0025".into(),
            domain: TimingAuthorityDomain {
                estimand: SESSION_CELL_MEDIAN_ESTIMAND.into(),
                assessment_schema: SESSION_MEDIAN_INTERVAL_SCHEMA.into(),
                min_sessions: 6,
                max_sessions: 24,
                session_counts: Some(vec![6, 8, 12, 24]),
                case_counts: vec![1, 5],
            },
            status: TimingAuthorityStatus::Hold,
            reason: "preregistered coverage study PASS within its simulated domain; held until an independent domain review (R4 authority gate)".into(),
        },
    ]
}

/// The authority of `design` in `registry`: Hold if any covering study
/// holds, Admissible if all covering studies are admissible, NotEvaluated
/// if none covers it.
pub fn select_timing_authority(
    registry: &[TimingAuthority],
    design: &TimingDesignIdentity,
) -> (TimingAuthorityStatus, Vec<TimingAuthority>) {
    let covering = registry
        .iter()
        .filter(|authority| authority.domain.covers(design))
        .cloned()
        .collect::<Vec<_>>();
    let status = if covering.is_empty() {
        TimingAuthorityStatus::NotEvaluated
    } else if covering
        .iter()
        .all(|authority| authority.status == TimingAuthorityStatus::Admissible)
    {
        TimingAuthorityStatus::Admissible
    } else if covering
        .iter()
        .any(|authority| authority.status == TimingAuthorityStatus::Hold)
    {
        TimingAuthorityStatus::Hold
    } else {
        TimingAuthorityStatus::NotEvaluated
    };
    (status, covering)
}

/// A claimed authority is genuine only if it is an entry of the compiled
/// registry, field for field.
pub fn verify_timing_authority(claimed: &TimingAuthority) -> FairResult<()> {
    if timing_authority_registry().contains(claimed) {
        Ok(())
    } else {
        Err(FairError::Invalid(format!(
            "TIMING_AUTHORITY_UNKNOWN: {} ({}) is not a registered study verdict",
            claimed.study_id, claimed.ledger_row
        )))
    }
}

/// The diagnostic decision, the authority of its design, and the decision a
/// consumer may act on. Its fields are private and it is not deserializable:
/// the only value of this type is one [`PairedTimingEvidence::admissible_decision`]
/// built from the compiled registry.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AdmissibleTimingDecision {
    diagnostic: PairedTimingDecision,
    design: TimingDesignIdentity,
    authority: TimingAuthorityStatus,
    studies: Vec<String>,
    admissible: Option<PairedTimingDecision>,
    reason: String,
}

impl AdmissibleTimingDecision {
    /// [`PairedTimingEvidence::verified_decision`], unchanged.
    pub fn diagnostic(&self) -> PairedTimingDecision {
        self.diagnostic
    }

    pub fn design(&self) -> &TimingDesignIdentity {
        &self.design
    }

    pub fn authority(&self) -> TimingAuthorityStatus {
        self.authority
    }

    /// The covering studies, as `study_id (ledger row)`.
    pub fn studies(&self) -> &[String] {
        &self.studies
    }

    /// `Some(diagnostic)` only when the authority is admissible.
    pub fn admissible(&self) -> Option<PairedTimingDecision> {
        self.admissible
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// A counterfactual decision under a registry that is not (or need not be)
/// the compiled one; see [`PairedTimingEvidence::hypothetical_decision_in`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HypotheticalTimingDecision {
    pub diagnostic: PairedTimingDecision,
    pub design: TimingDesignIdentity,
    pub authority: TimingAuthorityStatus,
    pub studies: Vec<String>,
    /// What the decision would admit under the supplied registry.
    pub would_admit: Option<PairedTimingDecision>,
    pub reason: String,
    /// Whether every entry of the supplied registry is a compiled one.
    pub registry_verified: bool,
}

impl PairedTimingEvidence {
    /// The design identity of this evidence.
    pub fn design_identity(&self) -> TimingDesignIdentity {
        TimingDesignIdentity {
            estimand: self.assessment.estimand.clone(),
            assessment_schema: self.assessment.schema.clone(),
            sessions: self.assessment.corpus.independent_blocks,
            cases: self.receipt.cases.len(),
        }
    }

    /// The verified diagnostic decision with the authority the compiled
    /// registry assigns to its design; integrity failures are errors, as
    /// before. This is the only constructor of a decision a consumer may act
    /// on: no registry, receipt or JSON field supplied at run time enters it.
    pub fn admissible_decision(&self) -> FairResult<AdmissibleTimingDecision> {
        self.decision_under(&timing_authority_registry())
    }

    /// What [`Self::admissible_decision`] would return if `registry` were the
    /// compiled one: a counterfactual for tests and for planning a future
    /// study, never authority. Its type carries no `admissible` field, and
    /// `registry_verified` says whether every entry is a compiled one (a
    /// registry with an entry that is not cannot be mistaken for the real
    /// verdict even by a caller that unwraps the decision).
    pub fn hypothetical_decision_in(
        &self,
        registry: &[TimingAuthority],
    ) -> FairResult<HypotheticalTimingDecision> {
        let decision = self.decision_under(registry)?;
        Ok(HypotheticalTimingDecision {
            registry_verified: registry
                .iter()
                .all(|entry| verify_timing_authority(entry).is_ok()),
            diagnostic: decision.diagnostic,
            design: decision.design,
            authority: decision.authority,
            studies: decision.studies,
            would_admit: decision.admissible,
            reason: decision.reason,
        })
    }

    fn decision_under(&self, registry: &[TimingAuthority]) -> FairResult<AdmissibleTimingDecision> {
        let diagnostic = self.verified_decision()?;
        let design = self.design_identity();
        let (authority, covering) = select_timing_authority(registry, &design);
        let studies = covering
            .iter()
            .map(|study| format!("{} ({})", study.study_id, study.ledger_row))
            .collect::<Vec<_>>();
        let reason = match authority {
            TimingAuthorityStatus::Admissible => "every covering study is admissible".into(),
            TimingAuthorityStatus::Hold => format!(
                "STATISTICAL_AUTHORITY_HOLD: {}",
                covering
                    .iter()
                    .filter(|study| study.status == TimingAuthorityStatus::Hold)
                    .map(|study| format!(
                        "{} ({}): {}",
                        study.study_id, study.ledger_row, study.reason
                    ))
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            TimingAuthorityStatus::NotEvaluated => format!(
                "STATISTICAL_AUTHORITY_NOT_EVALUATED: no registered study covers estimand {} with {} sessions and {} cases",
                design.estimand, design.sessions, design.cases
            ),
        };
        Ok(AdmissibleTimingDecision {
            diagnostic,
            admissible: (authority == TimingAuthorityStatus::Admissible).then_some(diagnostic),
            design,
            authority,
            studies,
            reason,
        })
    }
}
