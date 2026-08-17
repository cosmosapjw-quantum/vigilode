use rodas5p_core::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};

const TINY: f64 = 1.0e-300;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CausalRjfStep {
    pub step_index: usize,
    pub h: f64,
    pub embedded_error: f64,
    pub jvp_vectors: u64,
    pub linear_matvecs: u64,
    pub rodas_wall_seconds: f64,
    /// log10(error) from exactly two accepted R-JF steps earlier, if available.
    pub log_error_two_steps_ago: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PointFeature {
    JvpPressure,
    StepContraction,
    ErrorCurvature,
}

impl PointFeature {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::JvpPressure => "jvp-pressure",
            Self::StepContraction => "step-contraction",
            Self::ErrorCurvature => "error-curvature",
        }
    }
}

pub fn causal_feature_value(step: &CausalRjfStep, feature: PointFeature) -> Option<f64> {
    let value = match feature {
        PointFeature::JvpPressure => {
            (step.jvp_vectors.max(1) as f64 / step.h.abs().max(TINY)).log10()
        }
        PointFeature::StepContraction => -step.h.abs().max(TINY).log10(),
        PointFeature::ErrorCurvature => {
            let previous = step.log_error_two_steps_ago?;
            -(step.embedded_error.max(TINY).log10() - previous)
        }
    };
    value.is_finite().then_some(value)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProbeAction {
    NoProbe,
    PrefixProbe,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistenceLatch {
    required_consecutive: usize,
    streak: usize,
    latched: bool,
}

impl PersistenceLatch {
    pub fn new(required_consecutive: usize) -> CoreResult<Self> {
        if required_consecutive == 0 {
            return Err(CoreError::InvalidInput(
                "persistence confirmation requires k>=1".into(),
            ));
        }
        Ok(Self {
            required_consecutive,
            streak: 0,
            latched: false,
        })
    }

    /// Returns true exactly once per maximal true excursion, when the required
    /// number of consecutive true observations has been reached.
    pub fn update(&mut self, signal: bool) -> bool {
        if !signal {
            self.streak = 0;
            self.latched = false;
            return false;
        }
        if self.latched {
            return false;
        }
        self.streak = self.streak.saturating_add(1);
        if self.streak >= self.required_consecutive {
            self.latched = true;
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PrefixBudget {
    delta: f64,
    committed_r_wall: f64,
    speculative_wall: f64,
}

impl PrefixBudget {
    pub fn new(delta: f64) -> CoreResult<Self> {
        if !delta.is_finite() || !(0.0..1.0).contains(&delta) {
            return Err(CoreError::InvalidInput(
                "prefix budget delta must be finite and satisfy 0<=delta<1".into(),
            ));
        }
        Ok(Self {
            delta,
            committed_r_wall: 0.0,
            speculative_wall: 0.0,
        })
    }

    pub fn record_committed_r(&mut self, wall: f64) -> CoreResult<()> {
        if !wall.is_finite() || wall < 0.0 {
            return Err(CoreError::InvalidInput(
                "committed R-JF wall must be finite and nonnegative".into(),
            ));
        }
        self.committed_r_wall += wall;
        Ok(())
    }

    pub fn can_probe(&self, prospective_prefix_upper_bound: f64) -> bool {
        prospective_prefix_upper_bound.is_finite()
            && prospective_prefix_upper_bound >= 0.0
            && self.speculative_wall + prospective_prefix_upper_bound
                <= self.delta * self.committed_r_wall
    }

    pub fn record_prefix(
        &mut self,
        actual_wall: f64,
        certified_upper_bound: f64,
    ) -> CoreResult<()> {
        if !actual_wall.is_finite()
            || actual_wall < 0.0
            || !certified_upper_bound.is_finite()
            || certified_upper_bound < 0.0
        {
            return Err(CoreError::InvalidInput(
                "prefix wall and bound must be finite and nonnegative".into(),
            ));
        }
        let roundoff = 64.0 * f64::EPSILON * certified_upper_bound.abs().max(1.0);
        if actual_wall > certified_upper_bound + roundoff {
            return Err(CoreError::InvalidInput(
                "realized prefix cost exceeded its certified prospective bound".into(),
            ));
        }
        self.speculative_wall += actual_wall;
        let budget_roundoff =
            64.0 * f64::EPSILON * (self.delta * self.committed_r_wall).abs().max(1.0);
        if self.speculative_wall > self.delta * self.committed_r_wall + budget_roundoff {
            return Err(CoreError::InvalidInput(
                "realized prefix cost violated the pathwise speculative budget".into(),
            ));
        }
        Ok(())
    }

    pub fn delta(&self) -> f64 {
        self.delta
    }

    pub fn committed_r_wall(&self) -> f64 {
        self.committed_r_wall
    }

    pub fn speculative_wall(&self) -> f64 {
        self.speculative_wall
    }
}
