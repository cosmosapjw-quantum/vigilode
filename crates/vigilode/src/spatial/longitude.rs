//! Matrix-free inverse of the shifted diffusion operator along longitude.

use super::{DiffusionGrid, Geometry, SpatialRows, invalid};
use crate::SolveError;
use rayon::prelude::*;
use rodas5p_core::{CoreError, CoreResult, Preconditioner};

#[derive(Debug)]
struct RingFactor {
    alpha: f64,
    rho: f64,
    inverse_closure: f64,
}

impl RingFactor {
    fn new(strength: f64, width: usize) -> Result<Self, SolveError> {
        let twice = 2.0 * strength;
        if !twice.is_finite() || 1.0 + twice == twice {
            return Err(invalid(
                "the shifted longitude diagonal loses its unit term in binary64",
            ));
        }
        // I+c(2-S-S^-1) = (I-rho S)(I-rho S^-1)/alpha^2,
        // alpha=1-rho, c=rho/alpha^2. Stable also at c=0.
        let root = (1.0 + 4.0 * strength).sqrt();
        let alpha = 2.0 / (1.0 + root);
        let rho = if strength < 0.5 {
            strength * alpha * alpha
        } else {
            1.0 - alpha
        };
        let closure = if rho < 0.5 {
            1.0 - rho.powf(width as f64)
        } else {
            -(width as f64 * (-alpha).ln_1p()).exp_m1()
        };
        Ok(Self {
            alpha,
            rho,
            inverse_closure: 1.0 / closure,
        })
    }

    fn solve(&self, rhs: &[f64], out: &mut [f64]) {
        if self.rho == 0.0 {
            out.copy_from_slice(rhs);
            return;
        }
        let recurrence = |last: f64, value: f64| self.alpha * value + self.rho * last;
        let mut last = rhs.iter().copied().fold(0.0, recurrence) * self.inverse_closure;
        for (&value, z) in rhs.iter().zip(out.iter_mut()) {
            last = recurrence(last, value);
            *z = last;
        }
        last = out.iter().rev().copied().fold(0.0, recurrence) * self.inverse_closure;
        for z in out.iter_mut().rev() {
            last = recurrence(last, *z);
            *z = last;
        }
    }
}

/// Left preconditioner for one positive-real spherical diffusion stage.
/// Applies (I - sigma L_longitude)^-1 independently on each latitude ring in linear
/// time with four sweeps over a row and no heap allocation in the serial action.
#[derive(Debug)]
pub struct LongitudeLinePreconditioner {
    width: usize,
    rings: Vec<RingFactor>,
    rows: SpatialRows,
}

impl DiffusionGrid {
    /// Factor each periodic longitude line for a fixed nonnegative shift (sigma = gamma*h).
    /// Retain the result across every stage with the same shift. Rebuild when the shift
    /// changes. Box geometry or an unrepresentable shift returns an error.
    pub fn longitude_preconditioner(
        &self,
        sigma: f64,
        rows: SpatialRows,
    ) -> Result<LongitudeLinePreconditioner, SolveError> {
        let Geometry::Sphere(rings) = &*self.geometry else {
            return Err(invalid(
                "longitude lines require a spherical diffusion grid",
            ));
        };
        if !sigma.is_finite() || sigma < 0.0 {
            return Err(invalid(
                "longitude line shift must be finite and nonnegative",
            ));
        }
        let factors = rings
            .iter()
            .map(|r| RingFactor::new(sigma * r.azimuth, self.width))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(LongitudeLinePreconditioner {
            width: self.width,
            rings: factors,
            rows,
        })
    }
}

impl LongitudeLinePreconditioner {
    /// Write the left-preconditioned vector into `out`. The two slices have grid dimension.
    pub fn apply(&self, rhs: &[f64], out: &mut [f64]) -> CoreResult<()> {
        if rhs.len() != self.dimension() || out.len() != self.dimension() {
            return Err(CoreError::Dimension(
                "longitude preconditioner/state length mismatch".into(),
            ));
        }
        let apply_row = |(i, row): (usize, &mut [f64])| {
            let start = i * self.width;
            self.rings[i].solve(&rhs[start..start + self.width], row);
        };
        match self.rows {
            SpatialRows::Serial => out.chunks_mut(self.width).enumerate().for_each(apply_row),
            SpatialRows::Parallel => out
                .par_chunks_mut(self.width)
                .enumerate()
                .for_each(apply_row),
        }
        Ok(())
    }
}

impl Preconditioner for LongitudeLinePreconditioner {
    fn dimension(&self) -> usize {
        self.width * self.rings.len()
    }
    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> CoreResult<()> {
        Self::apply(self, rhs, out)
    }
}
