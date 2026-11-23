//! A linear tolerance tied to the step tolerance (loop node V17).
//!
//! The protected stopping test ([`super::StageStoppingRule`]) compares the
//! stage residual with the right-hand side: `rtol_lin ||b||`, with
//! `rtol_lin = 1e-10` whatever the step tolerance is. In the scaled system
//! `(I - h gamma J) U = rhs` the solution is an increment of the state, so
//! the natural yardstick of its residual is the error weight
//! `atol + rtol |y_i|` of the step controller. [`StepTiedTolerance`] adds
//! the bound
//!
//! ```text
//! ||rhs - (I - h gamma J) U||_2 <= kappa sqrt(n) min_i (atol + rtol |y_i|)
//! ```
//!
//! which implies `||r||_WRMS <= kappa` for the weights
//! `1 / (atol + rtol |y_i|)` (and is the same statement when the weights are
//! uniform). A client that is given the policy accepts on the larger of the
//! protected threshold and this bound, so it never asks for more than the
//! protected rule.
//!
//! The declared constant [`StepTiedTolerance::KAPPA`] and its justification
//! are in `research/production_loop_20261116/mf_stage_solve/PREDECLARED.json`.

use rodas5p_core::{CoreError, CoreResult};

/// The step-tied residual bound; see the module documentation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepTiedTolerance {
    kappa: f64,
    rtol: f64,
    atol: f64,
}

impl StepTiedTolerance {
    /// The constant declared for loop node V17.
    pub const KAPPA: f64 = 1.0e-3;

    /// The policy with the fraction `kappa` of the local tolerance and the
    /// step tolerances `rtol` and `atol` (all finite and nonnegative).
    pub fn new(kappa: f64, rtol: f64, atol: f64) -> CoreResult<Self> {
        let valid = |value: f64| value.is_finite() && value >= 0.0;
        if !(valid(kappa) && valid(rtol) && valid(atol)) {
            return Err(CoreError::InvalidInput(
                "step-tied tolerance needs finite nonnegative kappa, rtol and atol".into(),
            ));
        }
        Ok(Self { kappa, rtol, atol })
    }

    /// The policy with [`Self::KAPPA`].
    pub fn declared(rtol: f64, atol: f64) -> CoreResult<Self> {
        Self::new(Self::KAPPA, rtol, atol)
    }

    /// The fraction of the local tolerance.
    pub fn kappa(&self) -> f64 {
        self.kappa
    }

    /// The relative step tolerance.
    pub fn rtol(&self) -> f64 {
        self.rtol
    }

    /// The absolute step tolerance.
    pub fn atol(&self) -> f64 {
        self.atol
    }

    /// `min_i (atol + rtol |y_i|)`, the smallest error weight denominator
    /// at the state `y` (`atol` for an empty state).
    pub fn weight_floor(&self, y: &[f64]) -> CoreResult<f64> {
        let mut floor = f64::INFINITY;
        for value in y {
            let scale = self.atol + self.rtol * value.abs();
            if !scale.is_finite() {
                return Err(CoreError::NonFinite(
                    "step-tied tolerance: the state contains NaN/Inf".into(),
                ));
            }
            floor = floor.min(scale);
        }
        Ok(if floor.is_finite() { floor } else { self.atol })
    }

    /// The bound in the client's unscaled form,
    /// `kappa sqrt(n) weight_floor / (h gamma)`, for a system of order `n`
    /// and `inv_h_gamma = 1 / (h gamma)`.
    pub fn bound(&self, n: usize, weight_floor: f64, inv_h_gamma: f64) -> CoreResult<f64> {
        let bound = self.kappa * (n as f64).sqrt() * weight_floor * inv_h_gamma.abs();
        if bound.is_finite() {
            Ok(bound)
        } else {
            Err(CoreError::InvalidInput(
                "step-tied residual bound is not finite".into(),
            ))
        }
    }
}
