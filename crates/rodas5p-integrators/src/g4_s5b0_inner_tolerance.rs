use rodas5p_core::{CoreError, CoreResult, LinearMethod, LinearSolverConfig};

use crate::exponential::{FusedOrthogonalization, FusedPhiKrylovConfig};

const INNER_RELATIVE_FRACTION: f64 = 3.0e-2;
const INNER_ABSOLUTE_FRACTION: f64 = 3.0e-4;
const INNER_RELATIVE_FLOOR: f64 = 1.0e-12;
const INNER_ABSOLUTE_FLOOR: f64 = 1.0e-14;

/// One outer-error contract for both G4/S5B0 inner Krylov paths.
///
/// The protected RODAS5P/GMRES and exponential phi-Krylov paths consume the
/// same relative and absolute tolerances. Structural solver settings remain
/// lane-specific and are not changed by this policy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct G4S5B0InnerTolerancePolicy {
    outer_rtol: f64,
    relative_tolerance: f64,
    absolute_tolerance: f64,
}

impl G4S5B0InnerTolerancePolicy {
    pub fn try_from_outer_rtol(outer_rtol: f64) -> CoreResult<Self> {
        if !outer_rtol.is_finite() || outer_rtol <= 0.0 {
            return Err(CoreError::InvalidInput(
                "G4/S5B0 outer rtol must be finite and positive".into(),
            ));
        }
        Ok(Self {
            outer_rtol,
            relative_tolerance: (INNER_RELATIVE_FRACTION * outer_rtol).max(INNER_RELATIVE_FLOOR),
            absolute_tolerance: (INNER_ABSOLUTE_FRACTION * outer_rtol).max(INNER_ABSOLUTE_FLOOR),
        })
    }

    pub fn outer_rtol(self) -> f64 {
        self.outer_rtol
    }

    pub fn relative_tolerance(self) -> f64 {
        self.relative_tolerance
    }

    pub fn absolute_tolerance(self) -> f64 {
        self.absolute_tolerance
    }

    pub fn linear_config(self) -> LinearSolverConfig {
        LinearSolverConfig {
            method: LinearMethod::Gmres,
            rtol: self.relative_tolerance,
            atol: self.absolute_tolerance,
            restart: 32,
            maxiter: 256,
            ..LinearSolverConfig::default()
        }
    }

    pub fn phi_config(self, dimension: usize) -> FusedPhiKrylovConfig {
        FusedPhiKrylovConfig {
            minimum_dimension: 2,
            maximum_dimension: (dimension + 4).min(32),
            dimension_increment: 2,
            relative_tolerance: self.relative_tolerance,
            absolute_tolerance: self.absolute_tolerance,
            orthogonalization: FusedOrthogonalization::FullMgs,
            maximum_substeps: 16,
        }
    }
}
