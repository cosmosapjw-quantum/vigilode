//! The declarative corpus file.

use serde::{Deserialize, Serialize};

use crate::arms::Arm;
use crate::model::{Kind, Model};

/// Corpus schema id.
pub const CORPUS_SCHEMA: &str = "vigilode-bench.corpus.v1";

/// One problem of the corpus.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProblemSpec {
    /// Row id.
    pub id: String,
    /// Model kind.
    pub kind: Kind,
    /// Dimension.
    pub n: usize,
    /// End of the span (start is 0).
    pub t_end: f64,
    /// Absolute-tolerance scale vector: a run at `rtol` gives every arm the
    /// atol vector `rtol * atol_scale`. The protected drivers and the facade
    /// take one scalar atol, so every entry must be equal; the
    /// vector is what the error metrics use.
    pub atol_scale: Vec<f64>,
    /// Absolute tolerance of the reference runs.
    pub reference_atol: f64,
    /// Whether the band-structured arms run on this problem.
    pub banded: bool,
}

/// Predeclared, opt-in independent empirical reference tier. Both implementations
/// must complete every refinement level. Old corpora omit this object.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndependentReferenceSpec {
    /// Must be `radau5_cvode_refinement_v1`.
    pub tier: String,
    /// Strictly decreasing relative tolerances, coarse to fine.
    pub rtols: Vec<f64>,
    /// Strictly decreasing multipliers of each problem's reference_atol.
    pub atol_factors: Vec<f64>,
    /// Cross-family agreement threshold in the common error metric.
    pub agreement_threshold: f64,
    /// Within-family adjacent-refinement threshold in the same metric.
    pub refinement_threshold: f64,
}

/// How the reference solution is obtained and checked.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceSpec {
    /// The reference arm.
    pub arm: Arm,
    /// Its relative tolerance.
    pub rtol: f64,
    /// Attempt budget of reference runs (all reference arms).
    pub max_steps: usize,
    /// The decisive cross-check arm.
    pub crosscheck_arm: Arm,
    /// Tolerances tried for the cross-check, tightest first; the first
    /// successful one is used.
    pub crosscheck_rtols: Vec<f64>,
    /// Disagreement above which the problem is `reference-disputed`.
    pub dispute_threshold: f64,
    /// Further arms run at the reference tolerance; agreement recorded only.
    pub informational_arms: Vec<Arm>,
    /// Opt-in independent tier; absent preserves the historical reference policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub independent_protocol: Option<IndependentReferenceSpec>,
}

/// The corpus.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    /// Must be [`CORPUS_SCHEMA`].
    pub schema: String,
    /// Corpus id.
    pub id: String,
    /// Points of the uniform output grid on `[0, t_end]`.
    pub grid_points: usize,
    /// Relative tolerances of the measured runs.
    pub rtols: Vec<f64>,
    /// Discarded warm-up runs per arm and tolerance.
    pub warmups: usize,
    /// Measured repetitions per arm and tolerance.
    pub repetitions: usize,
    /// rtol of the tolerance-independent error metric `error_common`
    /// (weights `error_rtol * (atol_scale_i + |y_ref_i|)`).
    pub error_rtol: f64,
    /// Common error levels (`error_common`) at which work–precision curves
    /// are compared.
    pub error_levels: Vec<f64>,
    /// Reference policy.
    pub reference: ReferenceSpec,
    /// Arms (band arms run only on `banded` problems).
    pub arms: Vec<Arm>,
    /// Problems.
    pub problems: Vec<ProblemSpec>,
}

impl ProblemSpec {
    /// The model.
    pub fn model(&self) -> Result<Model, String> {
        Model::new(self.kind, self.n)
    }

    /// The scalar atol of a run at `rtol` (all scale entries are equal,
    /// checked by [`Corpus::validate`]).
    pub fn atol_scalar(&self, rtol: f64) -> f64 {
        rtol * self.atol_scale[0]
    }

    /// The atol vector of a run at `rtol`.
    pub fn atol_vector(&self, rtol: f64) -> Vec<f64> {
        self.atol_scale.iter().map(|s| rtol * s).collect()
    }
}

impl Corpus {
    /// Parse and validate.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let c: Corpus = serde_json::from_str(text).map_err(|e| e.to_string())?;
        c.validate()?;
        Ok(c)
    }

    /// Reject malformed corpora before any run.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != CORPUS_SCHEMA {
            return Err(format!("schema {} != {CORPUS_SCHEMA}", self.schema));
        }
        if self.grid_points < 2 || self.repetitions == 0 || self.rtols.is_empty() {
            return Err("grid_points >= 2, repetitions >= 1 and one rtol are required".into());
        }
        let pos = |v: f64| v.is_finite() && v > 0.0;
        if !self.rtols.iter().all(|&r| pos(r))
            || !pos(self.error_rtol)
            || !self.error_levels.iter().all(|&l| pos(l))
        {
            return Err("tolerances must be finite and positive".into());
        }
        crate::reference_policy::validate_reference_policy(
            self.reference.rtol,
            &self.reference.crosscheck_rtols,
            self.reference.max_steps,
            self.reference.dispute_threshold,
            self.reference.arm.implementation_family(),
            self.reference.crosscheck_arm.implementation_family(),
        )?;
        if let Some(protocol) = &self.reference.independent_protocol {
            crate::reference_policy::validate_independent_policy(
                &protocol.tier,
                &protocol.rtols,
                &protocol.atol_factors,
                protocol.agreement_threshold,
                protocol.refinement_threshold,
            )?;
            if self.reference.arm != Arm::Radau5
                || self.reference.crosscheck_arm != Arm::CvodeBdfDense
                || self.reference.rtol != *protocol.rtols.last().unwrap()
                || self.reference.crosscheck_rtols != vec![self.reference.rtol]
                || self.reference.dispute_threshold != protocol.agreement_threshold
                || !self.reference.informational_arms.is_empty()
                || self.error_rtol != 1.0
            {
                return Err("independent tier requires finest RADAU5 baseline, CVODE decisive check, matching finest/threshold legacy fields, no informational arms, and error_rtol = 1".into());
            }
        }
        if self.problems.is_empty() || self.arms.is_empty() || self.error_levels.is_empty() {
            return Err("at least one problem, solver arm and error level are required".into());
        }
        let unique_arms: std::collections::BTreeSet<_> = self.arms.iter().collect();
        if unique_arms.len() != self.arms.len() {
            return Err("solver arms must be unique".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        for p in &self.problems {
            if p.id.is_empty() || !ids.insert(&p.id) {
                return Err("problem ids must be nonempty and unique".into());
            }
            p.model()?;
            if p.atol_scale.len() != p.n {
                return Err(format!(
                    "{}: atol_scale has {} entries, n = {}",
                    p.id,
                    p.atol_scale.len(),
                    p.n
                ));
            }
            if !p.atol_scale.iter().all(|&a| pos(a) && a == p.atol_scale[0]) {
                return Err(format!(
                    "{}: atol_scale entries must be positive and equal (the protected drivers take a scalar atol)",
                    p.id
                ));
            }
            if !(pos(p.t_end) && pos(p.reference_atol)) {
                return Err(format!(
                    "{}: t_end and reference_atol must be positive",
                    p.id
                ));
            }
            if self.rtols.iter().any(|&r| !pos(p.atol_scalar(r)))
                || p.atol_vector(self.error_rtol).iter().any(|&a| !pos(a))
            {
                return Err(format!(
                    "{}: tolerance scaling overflowed or underflowed",
                    p.id
                ));
            }
            if self
                .reference
                .independent_protocol
                .as_ref()
                .is_some_and(|protocol| {
                    protocol
                        .atol_factors
                        .iter()
                        .any(|&factor| !pos(p.reference_atol * factor))
                })
            {
                return Err(format!(
                    "{}: independent reference atol scaling overflowed or underflowed",
                    p.id
                ));
            }
            if self.arms_for(p).is_empty() {
                return Err(format!("{}: no compatible measured arm", p.id));
            }
        }
        Ok(())
    }

    /// The uniform output grid of a problem: `t_k = t_end * k / (N - 1)`,
    /// last point exactly `t_end`.
    pub fn grid(&self, p: &ProblemSpec) -> Vec<f64> {
        let m = (self.grid_points - 1) as f64;
        let mut g: Vec<f64> = (0..self.grid_points)
            .map(|k| p.t_end * k as f64 / m)
            .collect();
        *g.last_mut().unwrap() = p.t_end;
        g
    }

    /// The arms that run on `p`.
    pub fn arms_for(&self, p: &ProblemSpec) -> Vec<Arm> {
        self.arms
            .iter()
            .copied()
            .filter(|a| p.banded || !a.banded())
            .collect()
    }
}
