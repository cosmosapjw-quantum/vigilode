use rodas5p_core::{CoreError, CoreResult, binary_power, binary_scale, binary_split};
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

    /// Recheck the constructors' invariants. The variants are public and
    /// deserializable, so a policy can bypass the constructors; `budget`
    /// calls this on every evaluation (audit 2026-09-30, CERT-01: a direct
    /// `StepPower { exponent: u32::MAX, .. }` cast to exponent -1 and gave a
    /// budget that grows as h shrinks, and `Mixed { eta: NaN, .. }` was
    /// hidden by `f64::min`).
    pub fn validate(&self) -> CoreResult<()> {
        match self {
            Self::Absolute { epsilon } => validate_nonnegative("absolute output budget", *epsilon),
            Self::EmbeddedRelative { eta } => {
                validate_nonnegative("embedded-relative coefficient", *eta)
            }
            Self::StepPower {
                epsilon_ref,
                h_ref,
                exponent,
            } => {
                validate_nonnegative("step-power reference budget", *epsilon_ref)?;
                validate_reference_step(*h_ref)?;
                validate_exponent(*exponent)
            }
            Self::Mixed {
                eta,
                epsilon_ref,
                h_ref,
                exponent,
            } => {
                validate_nonnegative("mixed embedded-relative coefficient", *eta)?;
                validate_nonnegative("mixed step-power reference budget", *epsilon_ref)?;
                validate_reference_step(*h_ref)?;
                validate_exponent(*exponent)
            }
        }
    }

    pub fn budget(&self, embedded_error: f64, h: f64) -> CoreResult<f64> {
        self.validate()?;
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
            } => step_power_term(*epsilon_ref, h, *h_ref, *exponent),
            Self::Mixed {
                eta,
                epsilon_ref,
                h_ref,
                exponent,
            } => {
                // Each component is evaluated on its own and must be a
                // number before the minimum is taken; `f64::min` drops a
                // NaN operand (re-audit R2-POL-01).
                let embedded = eta * embedded_error;
                let step = step_power_term(*epsilon_ref, h, *h_ref, *exponent);
                if embedded.is_nan() || step.is_nan() {
                    return Err(CoreError::NonFinite(
                        "mixed output budget component is NaN".into(),
                    ));
                }
                embedded.min(step)
            }
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

/// `epsilon_ref * (h / h_ref)^exponent` without intermediate overflow or
/// underflow (re-audit R2-POL-01).
///
/// Each operand is split into a mantissa in [1/2, 1) and a binary exponent;
/// the power is taken by squaring on renormalized mantissas and the binary
/// exponents are summed as integers (`rodas5p_core::binary_power`), so the
/// only roundings are the mantissa products and the final scaling. The direct form overflowed
/// `(h / h_ref)^p` to +inf for h = 1, h_ref = 1e-160, p = 2, and the product
/// with epsilon_ref = 1e-320 then gave +inf instead of about 0.99999, which
/// `min` turned into the embedded term. A true value above `f64::MAX` is
/// +inf (the component does not bind), below the smallest subnormal it is
/// 0, and `epsilon_ref = 0` gives 0 for every finite ratio.
pub(crate) fn step_power_term(epsilon_ref: f64, h: f64, h_ref: f64, exponent: u32) -> f64 {
    if epsilon_ref == 0.0 {
        return 0.0;
    }
    let (m_eps, e_eps) = binary_split(epsilon_ref);
    let (m_h, e_h) = binary_split(h.abs());
    let (m_ref, e_ref) = binary_split(h_ref);
    // m_h / m_ref lies in (1/2, 2); its power is formed on binary exponents.
    let (power_m, power_e) = binary_power(m_h / m_ref, exponent);
    binary_scale(
        m_eps * power_m,
        e_eps + i64::from(exponent) * (e_h - e_ref) + power_e,
    )
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
