//! Machine-checked comparator fidelity labels (audit F-052/F-056, Tier B).
//!
//! The internal BDF and Radau IIA integrators are reference implementations:
//! under `NewtonConfig::default()` / `RadauConfig::default()` their Newton
//! solves stop at fixed absolute tolerances independent of the outer
//! tolerance, factor a fresh dense Jacobian at the start of every implicit
//! solve, and (Radau IIA3) factor a separate estimator matrix per trial.  Their
//! RHS, factorization and wall counts are therefore inflated relative to a
//! production implicit solver.  README.md already forbids a competitive
//! reading; these labels turn that disclosure into fields that every comparator
//! record carries and that gates consult before emitting a relative verdict.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComparatorFidelity {
    /// A RODAS5P-family arm under its production configuration.
    Production,
    /// Internal BDF/Radau comparator under the default fixed-tolerance,
    /// fresh-factorization Newton configuration (Tier B).
    ReferenceImplementationOnly,
    /// Internal BDF/Radau comparator with at least one opt-in Tier-A
    /// modified-Newton fairness flag.  Cross-step Jacobian/LU reuse is not
    /// implemented, so this label still does not admit a relative reading.
    TierAModifiedNewton,
}

impl ComparatorFidelity {
    /// Whether a relative-performance reading (front ranking, attainment, or a
    /// speedup verdict) may use a record carrying this label.
    pub fn admits_relative_performance_reading(self) -> bool {
        matches!(self, Self::Production)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComparativeReading {
    Permitted,
    /// At least one participant is a Tier-B (or partial Tier-A) comparator, so
    /// the ranking carries no relative-performance information.
    Forbidden,
}

impl ComparativeReading {
    /// Reading admissible for a ranking over every listed participant.  The
    /// whole participant set matters, not only the nondominated survivors,
    /// because a handicapped arm shapes which records survive.
    pub fn for_participants<I>(participants: I) -> Self
    where
        I: IntoIterator<Item = ComparatorFidelity>,
    {
        if participants
            .into_iter()
            .all(ComparatorFidelity::admits_relative_performance_reading)
        {
            Self::Permitted
        } else {
            Self::Forbidden
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RelativePerformanceVerdict {
    Promote,
    Block,
    /// Refused: the reference (or candidate) is not a production arm.
    NotEvaluated,
}

/// Three-valued relative-performance decision.  `passes` is consulted only
/// when both arms admit a relative reading; otherwise the gate refuses to
/// emit Promote/Block against a reference-implementation-only comparator.
pub fn relative_performance_verdict(
    reference: ComparatorFidelity,
    candidate: ComparatorFidelity,
    passes: bool,
) -> RelativePerformanceVerdict {
    if !reference.admits_relative_performance_reading()
        || !candidate.admits_relative_performance_reading()
    {
        RelativePerformanceVerdict::NotEvaluated
    } else if passes {
        RelativePerformanceVerdict::Promote
    } else {
        RelativePerformanceVerdict::Block
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_production_arms_admit_a_relative_reading() {
        use ComparatorFidelity::*;
        assert_eq!(
            ComparativeReading::for_participants([Production, Production]),
            ComparativeReading::Permitted
        );
        assert_eq!(
            ComparativeReading::for_participants([Production, ReferenceImplementationOnly]),
            ComparativeReading::Forbidden
        );
        assert_eq!(
            ComparativeReading::for_participants([Production, TierAModifiedNewton]),
            ComparativeReading::Forbidden
        );
        for passes in [true, false] {
            assert_eq!(
                relative_performance_verdict(ReferenceImplementationOnly, Production, passes),
                RelativePerformanceVerdict::NotEvaluated
            );
        }
        assert_eq!(
            relative_performance_verdict(Production, Production, true),
            RelativePerformanceVerdict::Promote
        );
        assert_eq!(
            relative_performance_verdict(Production, Production, false),
            RelativePerformanceVerdict::Block
        );
        assert_eq!(
            serde_json::to_string(&ReferenceImplementationOnly).unwrap(),
            "\"reference-implementation-only\""
        );
        assert_eq!(
            serde_json::to_string(&ComparativeReading::Forbidden).unwrap(),
            "\"forbidden\""
        );
    }
}
