use rodas5p_core::{CoreError, CoreResult, LinearMethod, LinearSolverConfig};
use serde::{Deserialize, Serialize};

use crate::exponential::{FusedOrthogonalization, FusedPhiKrylovConfig};

const INNER_RELATIVE_FRACTION: f64 = 3.0e-2;
const INNER_ABSOLUTE_FRACTION: f64 = 3.0e-4;
const INNER_RELATIVE_FLOOR: f64 = 1.0e-12;
const INNER_ABSOLUTE_FLOOR: f64 = 1.0e-14;
const LEGACY_LINEAR_RTOL: f64 = 1.0e-10;
const LEGACY_LINEAR_ATOL: f64 = 1.0e-12;
/// Dimensionless allocation used to set per-stage WRMS *residual* targets.
///
/// This is not an endpoint-error budget. Translating a certified residual
/// `r_i` into a stage error requires a problem-dependent bound on
/// `W^{-1} r_i`, and no such resolvent bound is available in the generic
/// matrix-free interface.
pub const RODAS5P_INNER_RESIDUAL_HEURISTIC_FRACTION: f64 = 0.1;
pub const RODAS5P_INNER_FORCING_FLOOR: f64 = 64.0 * f64::EPSILON;
pub const RODAS5P_INNER_FORCING_ETA_MAX: f64 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Rodas5pInnerForcingClaimScope {
    StageResidualHeuristicRequiresResolventBound,
}

pub const RODAS5P_INNER_FORCING_CLAIM_SCOPE: Rodas5pInnerForcingClaimScope =
    Rodas5pInnerForcingClaimScope::StageResidualHeuristicRequiresResolventBound;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rodas5pInnerForcingTarget {
    pub eta: f64,
    pub tau: f64,
}

pub fn rodas5p_inner_forcing_target(
    flow_wrms: f64,
    rhs_wrms: f64,
    output_weight_l1: f64,
) -> CoreResult<Rodas5pInnerForcingTarget> {
    if !flow_wrms.is_finite() || flow_wrms < 0.0 {
        return Err(CoreError::InvalidInput(
            "RODAS5P inner-forcing flow norm must be finite and nonnegative".into(),
        ));
    }
    if !rhs_wrms.is_finite() || rhs_wrms < 0.0 {
        return Err(CoreError::InvalidInput(
            "RODAS5P inner-forcing RHS norm must be finite and nonnegative".into(),
        ));
    }
    if !output_weight_l1.is_finite() || output_weight_l1 <= 0.0 {
        return Err(CoreError::InvalidInput(
            "RODAS5P inner-forcing output-weight norm must be finite and positive".into(),
        ));
    }

    let forcing_scale = flow_wrms.max(rhs_wrms);
    let unclamped_eta = if forcing_scale == 0.0 {
        RODAS5P_INNER_FORCING_ETA_MAX
    } else {
        RODAS5P_INNER_RESIDUAL_HEURISTIC_FRACTION / (output_weight_l1 * forcing_scale)
    };
    let eta = unclamped_eta.clamp(RODAS5P_INNER_FORCING_FLOOR, RODAS5P_INNER_FORCING_ETA_MAX);
    let relative_target = eta * rhs_wrms;
    let tau = relative_target.max(RODAS5P_INNER_FORCING_FLOOR);
    let roundoff_floor_is_active = unclamped_eta < RODAS5P_INNER_FORCING_FLOOR
        || relative_target < RODAS5P_INNER_FORCING_FLOOR;
    if roundoff_floor_is_active
        && output_weight_l1 * tau > RODAS5P_INNER_RESIDUAL_HEURISTIC_FRACTION
    {
        return Err(CoreError::LinearSolve(
            "RODAS5P inner-forcing roundoff floor exceeds the stage-residual heuristic allocation"
                .into(),
        ));
    }
    Ok(Rodas5pInnerForcingTarget { eta, tau })
}

/// Explicit GMRES tolerance arm used by the G4/S5B0 authority replay.
///
/// Both arms preserve the exact pre-A1 phi-Krylov arithmetic. Only the
/// protected linear residual thresholds differ. Equal numeric values do not
/// imply equal forward/backward error or equal outer-error contribution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum G4S5B0LinearToleranceArm {
    LegacyFixed,
    OuterScaledNumericParity,
}

impl G4S5B0LinearToleranceArm {
    pub const ALL: [Self; 2] = [Self::LegacyFixed, Self::OuterScaledNumericParity];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::LegacyFixed => "legacy-fixed",
            Self::OuterScaledNumericParity => "outer-scaled-numeric-parity",
        }
    }
}

/// The six protected G4/S5B0 runtime lanes that consume an inner policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum G4S5B0InnerToleranceLane {
    RegimeAtlas,
    AttemptTrace,
    ActualLevel1Prefix,
    ActualLevel2Prefix,
    StageGrowthSafety,
    FrozenFullEShadow,
}

impl G4S5B0InnerToleranceLane {
    pub const ALL: [Self; 6] = [
        Self::RegimeAtlas,
        Self::AttemptTrace,
        Self::ActualLevel1Prefix,
        Self::ActualLevel2Prefix,
        Self::StageGrowthSafety,
        Self::FrozenFullEShadow,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::RegimeAtlas => "regime-atlas",
            Self::AttemptTrace => "attempt-trace",
            Self::ActualLevel1Prefix => "actual-level1-prefix",
            Self::ActualLevel2Prefix => "actual-level2-prefix",
            Self::StageGrowthSafety => "stage-growth-safety",
            Self::FrozenFullEShadow => "frozen-full-e-shadow",
        }
    }
}

/// Committed arm while the two-arm authority replay is being generated.
///
/// This remains `LegacyFixed` until the predeclared replay decision classifies
/// the outer-scaled arm as admissible and discriminating.
pub const G4_S5B0_COMMITTED_LINEAR_TOLERANCE_ARM: G4S5B0LinearToleranceArm =
    G4S5B0LinearToleranceArm::LegacyFixed;

pub fn committed_g4_s5b0_linear_tolerance_arm() -> G4S5B0LinearToleranceArm {
    G4_S5B0_COMMITTED_LINEAR_TOLERANCE_ARM
}

/// Numeric tolerance configuration for one declared G4/S5B0 lane and arm.
///
/// Linear residual thresholds and phi-action forward-error thresholds are
/// stored separately. `OuterScaledNumericParity` makes their numeric values
/// equal for a controlled replay; it does not assert semantic equivalence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct G4S5B0InnerTolerancePolicy {
    lane: G4S5B0InnerToleranceLane,
    arm: G4S5B0LinearToleranceArm,
    outer_rtol: f64,
    linear_relative_tolerance: f64,
    linear_absolute_tolerance: f64,
    phi_relative_tolerance: f64,
    phi_absolute_tolerance: f64,
}

impl G4S5B0InnerTolerancePolicy {
    pub fn try_for_lane(
        lane: G4S5B0InnerToleranceLane,
        arm: G4S5B0LinearToleranceArm,
        outer_rtol: f64,
    ) -> CoreResult<Self> {
        if !outer_rtol.is_finite() || outer_rtol <= 0.0 {
            return Err(CoreError::InvalidInput(
                "G4/S5B0 outer rtol must be finite and positive".into(),
            ));
        }

        let phi_relative_tolerance =
            (INNER_RELATIVE_FRACTION * outer_rtol).max(INNER_RELATIVE_FLOOR);
        let phi_absolute_tolerance =
            (INNER_ABSOLUTE_FRACTION * outer_rtol).max(INNER_ABSOLUTE_FLOOR);
        let (linear_relative_tolerance, linear_absolute_tolerance) = match arm {
            G4S5B0LinearToleranceArm::LegacyFixed => (LEGACY_LINEAR_RTOL, LEGACY_LINEAR_ATOL),
            G4S5B0LinearToleranceArm::OuterScaledNumericParity => {
                (phi_relative_tolerance, phi_absolute_tolerance)
            }
        };

        Ok(Self {
            lane,
            arm,
            outer_rtol,
            linear_relative_tolerance,
            linear_absolute_tolerance,
            phi_relative_tolerance,
            phi_absolute_tolerance,
        })
    }

    pub fn committed_for_lane(lane: G4S5B0InnerToleranceLane, outer_rtol: f64) -> CoreResult<Self> {
        Self::try_for_lane(lane, committed_g4_s5b0_linear_tolerance_arm(), outer_rtol)
    }

    /// Compatibility constructor for callers that do not need an explicit
    /// lane. New runtime wiring should use `committed_for_lane` or
    /// `try_for_lane`.
    pub fn try_from_outer_rtol(outer_rtol: f64) -> CoreResult<Self> {
        Self::committed_for_lane(G4S5B0InnerToleranceLane::RegimeAtlas, outer_rtol)
    }

    pub fn lane(self) -> G4S5B0InnerToleranceLane {
        self.lane
    }

    pub fn arm(self) -> G4S5B0LinearToleranceArm {
        self.arm
    }

    pub fn outer_rtol(self) -> f64 {
        self.outer_rtol
    }

    pub fn linear_relative_tolerance(self) -> f64 {
        self.linear_relative_tolerance
    }

    pub fn linear_absolute_tolerance(self) -> f64 {
        self.linear_absolute_tolerance
    }

    pub fn phi_relative_tolerance(self) -> f64 {
        self.phi_relative_tolerance
    }

    pub fn phi_absolute_tolerance(self) -> f64 {
        self.phi_absolute_tolerance
    }

    /// Compatibility accessor for the linear residual relative tolerance.
    pub fn relative_tolerance(self) -> f64 {
        self.linear_relative_tolerance
    }

    /// Compatibility accessor for the linear residual absolute tolerance.
    pub fn absolute_tolerance(self) -> f64 {
        self.linear_absolute_tolerance
    }

    pub fn linear_config(self) -> LinearSolverConfig {
        LinearSolverConfig {
            method: LinearMethod::Gmres,
            rtol: self.linear_relative_tolerance,
            atol: self.linear_absolute_tolerance,
            restart: 32,
            maxiter: 256,
            ..LinearSolverConfig::default()
        }
    }

    pub fn phi_config(self, dimension: usize) -> FusedPhiKrylovConfig {
        FusedPhiKrylovConfig {
            minimum_dimension: 2,
            maximum_dimension: dimension.saturating_add(4).min(32),
            dimension_increment: 2,
            relative_tolerance: self.phi_relative_tolerance,
            absolute_tolerance: self.phi_absolute_tolerance,
            orthogonalization: FusedOrthogonalization::FullMgs,
            maximum_substeps: 16,
        }
    }
}
