use rodas5p_core::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};

/// Dimensionless acceptance budget for the output-directed homotopy correction.
///
/// `StepPower` and `Mixed` use the dimensionless ratio `|h| / h_ref`, so `h_ref`
/// must carry the same physical time unit as the integrator step size.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum OutputBudgetPolicy {
    Absolute {
        epsilon: f64,
    },
    EmbeddedRelative {
        eta: f64,
    },
    StepPower {
        epsilon_ref: f64,
        h_ref: f64,
        exponent: u32,
    },
    Mixed {
        eta: f64,
        epsilon_ref: f64,
        h_ref: f64,
        exponent: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputBudgetDecision {
    pub policy: OutputBudgetPolicy,
    pub budget: f64,
    pub output_wrms: f64,
    pub embedded_error: f64,
    pub accepted: bool,
}

impl OutputBudgetPolicy {
    pub fn absolute(epsilon: f64) -> CoreResult<Self> {
        validate_nonnegative("absolute output budget", epsilon)?;
        Ok(Self::Absolute { epsilon })
    }

    pub fn embedded_relative(eta: f64) -> CoreResult<Self> {
        validate_nonnegative("embedded-relative coefficient", eta)?;
        Ok(Self::EmbeddedRelative { eta })
    }

    pub fn step_power(epsilon_ref: f64, h_ref: f64, exponent: u32) -> CoreResult<Self> {
        validate_nonnegative("step-power reference budget", epsilon_ref)?;
        validate_reference_step(h_ref)?;
        validate_exponent(exponent)?;
        Ok(Self::StepPower {
            epsilon_ref,
            h_ref,
            exponent,
        })
    }

    pub fn mixed(eta: f64, epsilon_ref: f64, h_ref: f64, exponent: u32) -> CoreResult<Self> {
        validate_nonnegative("mixed embedded-relative coefficient", eta)?;
        validate_nonnegative("mixed step-power reference budget", epsilon_ref)?;
        validate_reference_step(h_ref)?;
        validate_exponent(exponent)?;
        Ok(Self::Mixed {
            eta,
            epsilon_ref,
            h_ref,
            exponent,
        })
    }

    pub fn family(&self) -> &'static str {
        match self {
            Self::Absolute { .. } => "absolute",
            Self::EmbeddedRelative { .. } => "embedded-relative",
            Self::StepPower { .. } => "step-power",
            Self::Mixed { .. } => "mixed",
        }
    }

    pub fn id(&self) -> String {
        match self {
            Self::Absolute { epsilon } => format!("absolute-e{epsilon:.6e}"),
            Self::EmbeddedRelative { eta } => format!("embedded-relative-eta{eta:.6e}"),
            Self::StepPower {
                epsilon_ref,
                h_ref,
                exponent,
            } => format!("step-power-e{epsilon_ref:.6e}-href{h_ref:.6e}-p{exponent}"),
            Self::Mixed {
                eta,
                epsilon_ref,
                h_ref,
                exponent,
            } => format!("mixed-eta{eta:.6e}-e{epsilon_ref:.6e}-href{h_ref:.6e}-p{exponent}"),
        }
    }

    pub fn budget(&self, embedded_error: f64, h: f64) -> CoreResult<f64> {
        validate_nonnegative("embedded error", embedded_error)?;
        if !(h > 0.0 && h.is_finite()) {
            return Err(if h.is_finite() {
                CoreError::InvalidInput("homotopy policy step size must be positive".into())
            } else {
                CoreError::NonFinite("homotopy policy step size contains NaN/Inf".into())
            });
        }
        let budget = match self {
            Self::Absolute { epsilon } => *epsilon,
            Self::EmbeddedRelative { eta } => eta * embedded_error,
            Self::StepPower {
                epsilon_ref,
                h_ref,
                exponent,
            } => epsilon_ref * (h.abs() / h_ref).powi(*exponent as i32),
            Self::Mixed {
                eta,
                epsilon_ref,
                h_ref,
                exponent,
            } => (eta * embedded_error).min(epsilon_ref * (h.abs() / h_ref).powi(*exponent as i32)),
        };
        if budget.is_finite() && budget >= 0.0 {
            Ok(budget)
        } else {
            Err(CoreError::NonFinite(
                "homotopy output budget evaluation produced NaN/Inf".into(),
            ))
        }
    }

    pub fn decide(
        &self,
        output_wrms: f64,
        embedded_error: f64,
        h: f64,
    ) -> CoreResult<OutputBudgetDecision> {
        validate_nonnegative("output WRMS", output_wrms)?;
        let budget = self.budget(embedded_error, h)?;
        Ok(OutputBudgetDecision {
            policy: self.clone(),
            budget,
            output_wrms,
            embedded_error,
            accepted: output_wrms <= budget,
        })
    }
}

fn validate_nonnegative(label: &str, value: f64) -> CoreResult<()> {
    if !value.is_finite() {
        return Err(CoreError::NonFinite(format!("{label} contains NaN/Inf")));
    }
    if value < 0.0 {
        return Err(CoreError::InvalidInput(format!(
            "{label} must be nonnegative"
        )));
    }
    Ok(())
}

fn validate_reference_step(h_ref: f64) -> CoreResult<()> {
    if !h_ref.is_finite() {
        return Err(CoreError::NonFinite(
            "homotopy policy reference step contains NaN/Inf".into(),
        ));
    }
    if h_ref <= 0.0 {
        return Err(CoreError::InvalidInput(
            "homotopy policy reference step must be positive".into(),
        ));
    }
    Ok(())
}

fn validate_exponent(exponent: u32) -> CoreResult<()> {
    if exponent == 0 || exponent > i32::MAX as u32 {
        return Err(CoreError::InvalidInput(
            "homotopy policy exponent must lie in 1..=i32::MAX".into(),
        ));
    }
    Ok(())
}
