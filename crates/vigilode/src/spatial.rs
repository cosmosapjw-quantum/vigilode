//! Conservative method-of-lines diffusion problems with an allocation-free RHS and exact JVP.
//!
//! Cartesian grids are periodic in every active direction. A sphere uses periodic longitude,
//! exact cell areas, and zero flux through the zero-length polar faces. Diffusivities have
//! units of length squared per time. The scalar field is sampled at cell centres.
//!
//! ```
//! use vigilode::{VigilODE, VigilodeOptions};
//! use vigilode::spatial::{DiffusionGrid, SpatialRows};
//! let grid = DiffusionGrid::sphere(8, 16, 1.0, 0.1)?;
//! let p = grid.problem(SpatialRows::Serial);
//! let s = VigilODE::solve(&p, (0.0, 0.1), &vec![1.0; grid.dimension()], VigilodeOptions::default())?;
//! assert_eq!(s.final_state().unwrap(), &vec![1.0; grid.dimension()]);
//! # Ok::<(), vigilode::SolveError>(())
//! ```

use std::{f64::consts::PI, sync::Arc};

use rayon::prelude::*;

use crate::{CallbackError, Jacobian, Problem, SolveError};

/// Execution of independent stencil rows within each RHS or JVP invocation.
#[derive(Clone, Copy, Debug, Default)]
pub enum SpatialRows {
    /// Serial execution works on any pool size, including one.
    #[default]
    Serial,
    /// Rayon worker pool. Callback arithmetic within a row is identical to serial execution.
    Parallel,
}

#[derive(Debug)]
struct Ring {
    area: f64,
    north: f64,
    south: f64,
    azimuth: f64,
}

#[derive(Debug)]
enum Geometry {
    Cartesian {
        ny: usize,
        nz: usize,
        cell_volume: f64,
        rates: [f64; 3],
    },
    Sphere(Vec<Ring>),
}

/// A heat operator on a regular periodic box or latitude-longitude surface grid.
/// Stores three Cartesian rates or one coefficient record per latitude ring.
#[derive(Clone, Debug)]
pub struct DiffusionGrid {
    dimension: usize,
    width: usize,
    geometry: Arc<Geometry>,
}

fn invalid(message: &str) -> SolveError {
    SolveError::InvalidInput(message.to_owned())
}

fn dimension(cells: &[usize]) -> Result<usize, SolveError> {
    if cells.iter().any(|&n| n < 2) {
        return Err(invalid("each spatial axis needs at least two cells"));
    }
    cells
        .iter()
        .try_fold(1usize, |size, &n| size.checked_mul(n))
        .filter(|&n| n <= isize::MAX as usize / size_of::<f64>())
        .ok_or_else(|| invalid("spatial dimension exceeds the state buffer limit"))
}

impl DiffusionGrid {
    /// Two- or three-dimensional box with axis-wise diffusivity and x-fastest layout.
    pub fn periodic_box<const D: usize>(
        cells: [usize; D],
        lengths: [f64; D],
        diffusivity: [f64; D],
    ) -> Result<Self, SolveError> {
        if D != 2 && D != 3 {
            return Err(invalid("periodic_box expects two or three spatial axes"));
        }
        let dimension = dimension(&cells)?;
        let mut cell_volume = 1.0;
        let mut rates = [0.0; 3];
        for (axis, &kappa) in diffusivity.iter().enumerate() {
            let h = lengths[axis] / cells[axis] as f64;
            if !h.is_finite() || h <= 0.0 || !kappa.is_finite() || kappa < 0.0 {
                return Err(invalid(
                    "box lengths must be positive and diffusivity nonnegative and finite",
                ));
            }
            cell_volume *= h;
            rates[axis] = kappa / h / h;
            if !rates[axis].is_finite() || (kappa > 0.0 && rates[axis] == 0.0) {
                return Err(invalid(
                    "Cartesian diffusion rate is outside binary64 range",
                ));
            }
        }
        if !cell_volume.is_finite() || cell_volume <= 0.0 {
            return Err(invalid("Cartesian cell measure is outside binary64 range"));
        }
        Ok(Self {
            dimension,
            width: cells[0],
            geometry: Arc::new(Geometry::Cartesian {
                ny: cells[1],
                nz: cells.get(2).copied().unwrap_or(1),
                cell_volume,
                rates,
            }),
        })
    }

    /// Sphere with longitude-fastest layout. The first and last rows are polar cap rings.
    pub fn sphere(nt: usize, np: usize, radius: f64, diffusivity: f64) -> Result<Self, SolveError> {
        let dimension = dimension(&[nt, np])?;
        if !radius.is_finite() || radius <= 0.0 || !diffusivity.is_finite() || diffusivity < 0.0 {
            return Err(invalid(
                "sphere radius must be positive and diffusivity nonnegative and finite",
            ));
        }
        let dt = PI / nt as f64;
        let dp = 2.0 * PI / np as f64;
        let mut rings = Vec::new();
        rings
            .try_reserve_exact(nt)
            .map_err(|_| invalid("latitude coefficient storage is unavailable"))?;
        for i in 0..nt {
            let sin_mid = ((i as f64 + 0.5) * dt).sin();
            // Cos(theta_lo) - cos(theta_hi) with no subtraction near the poles.
            let area = (radius * (0.5 * dt).sin()) * (radius * dp) * (2.0 * sin_mid);
            if !area.is_finite() || area <= 0.0 {
                return Err(invalid("spherical cell area is outside binary64 range"));
            }
            let theta_rate = |face: usize| {
                if face == 0 || face == nt {
                    0.0
                } else {
                    diffusivity * ((face as f64 * dt).sin() * dp / dt) / area
                }
            };
            let ring = Ring {
                area,
                north: theta_rate(i),
                south: theta_rate(i + 1),
                azimuth: diffusivity * (dt / (sin_mid * dp)) / area,
            };
            if [ring.north, ring.south, ring.azimuth]
                .iter()
                .any(|v| !v.is_finite())
                || (diffusivity > 0.0
                    && (ring.azimuth == 0.0
                        || (i > 0 && ring.north == 0.0)
                        || (i + 1 < nt && ring.south == 0.0)))
            {
                return Err(invalid(
                    "spherical diffusion rate is outside binary64 range",
                ));
            }
            rings.push(ring);
        }
        Ok(Self {
            dimension,
            width: np,
            geometry: Arc::new(Geometry::Sphere(rings)),
        })
    }

    /// Number of scalar cell values.
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Cell area or volume for the discrete mass and energy inner products.
    pub fn cell_measure(&self, index: usize) -> Option<f64> {
        if index >= self.dimension {
            return None;
        }
        Some(match &*self.geometry {
            Geometry::Cartesian { cell_volume, .. } => *cell_volume,
            Geometry::Sphere(rings) => rings[index / self.width].area,
        })
    }

    /// Write the diffusion operator applied to `u`. Allocates no per-cell work arrays.
    pub fn apply(
        &self,
        u: &[f64],
        out: &mut [f64],
        rows: SpatialRows,
    ) -> Result<(), CallbackError> {
        if u.len() != self.dimension || out.len() != self.dimension {
            return Err(CallbackError::Fatal(
                "diffusion grid/state length mismatch".into(),
            ));
        }
        let width = self.width;
        let apply_row = |(r, out): (usize, &mut [f64])| {
            let (prev_y, next_y, prev_z, next_z, west, north, south, vertical) =
                match &*self.geometry {
                    Geometry::Cartesian { ny, nz, rates, .. } => {
                        let y = r % ny;
                        let z = r / ny;
                        let sy = z * ny;
                        (
                            sy + (y + ny - 1) % ny,
                            sy + (y + 1) % ny,
                            y + (z + nz - 1) % nz * ny,
                            y + (z + 1) % nz * ny,
                            rates[0],
                            rates[1],
                            rates[1],
                            rates[2],
                        )
                    }
                    Geometry::Sphere(rings) => {
                        let ring = &rings[r];
                        (
                            r.saturating_sub(1),
                            (r + 1).min(rings.len() - 1),
                            r,
                            r,
                            ring.azimuth,
                            ring.north,
                            ring.south,
                            0.0,
                        )
                    }
                };
            let start = r * width;
            for (x, dst) in out.iter_mut().enumerate() {
                let c = u[start + x];
                let xm = if x == 0 { width - 1 } else { x - 1 };
                let xp = if x + 1 == width { 0 } else { x + 1 };
                let horizontal = west * ((u[start + xm] - c) + (u[start + xp] - c));
                *dst = horizontal
                    + north * (u[prev_y * width + x] - c)
                    + south * (u[next_y * width + x] - c);
                if vertical > 0.0 {
                    *dst += vertical * ((u[prev_z * width + x] - c) + (u[next_z * width + x] - c));
                }
            }
        };
        match rows {
            SpatialRows::Serial => out.chunks_mut(width).enumerate().for_each(apply_row),
            SpatialRows::Parallel => out.par_chunks_mut(width).enumerate().for_each(apply_row),
        }
        Ok(())
    }

    /// Build an autonomous problem for the existing public frontend. Both callbacks use the
    /// same linear map, hence no finite differences, Jacobian assembly or Newton iteration.
    /// To explicitly request the dense finite-difference fallback on a small grid, override
    /// the provider with `problem.jacobian(vigilode::Jacobian::FiniteDifference)`.
    pub fn problem(&self, rows: SpatialRows) -> Problem {
        let rhs = self.clone();
        let jvp = self.clone();
        Problem::new(self.dimension, move |_t, u, du| rhs.apply(u, du, rows))
            .jacobian(Jacobian::jvp(move |_t, _u, v, out| jvp.apply(v, out, rows)))
            .autonomous()
    }
}

mod longitude;
pub use longitude::LongitudeLinePreconditioner;

mod separable;
pub use separable::{
    BoxTransform, SEPARABLE_STAGE_CLIENT_ID, SeparableMode, SeparableReport, SeparableStages,
    SpectralBounds,
};

mod portfolio;
pub use portfolio::{EndpointRequest, adaptive_endpoint, adaptive_stage};

mod adjoint;
pub use adjoint::{SpatialAdjointRoute, SpatialAdjointWork, SpatialAdjointWorkspace};

mod shifted_adjoint;
pub use shifted_adjoint::*;
