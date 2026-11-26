//! Latitude-longitude finite-volume Laplacian on the sphere.
//!
//! The conventions are those of the facade's spatial module (`DiffusionGrid::sphere`):
//! `N_t` colatitude rings of width `dtheta = pi / N_t`, `N_p` longitudes of width
//! `dphi = 2 pi / N_p`, longitude fastest (`index = ring N_p + j`), exact ring areas
//! `R^2 dphi (cos(theta_lo) - cos(theta_hi)) = R^2 dphi 2 sin(dtheta / 2) sin(theta_mid)`,
//! a flux `D (face length / centre distance) (u_neighbour - u)` through every face, periodic
//! longitude and zero flux through the zero-length polar faces.
//!
//! # Pole rings
//!
//! The first and the last ring are triangular caps around the poles. For zonal fields
//! (independent of longitude) they are as accurate as the other rings, but for a field that
//! varies with longitude (for example the spherical harmonic `Y_1^1 = sin(theta) cos(phi)`)
//! their truncation error is only **first order** in the grid spacing: loop node U09
//! measured observed orders 0.98 and 0.99 for `Y_1^1`, and the contract test
//! `mol_sphere_contracts` measures them again. Away from the poles (any fixed colatitude band
//! `[theta_0, pi - theta_0]`, `theta_0 > 0`) the operator is second order. Conservation and
//! the weighted symmetry hold on the whole sphere, pole rings included.

use std::f64::consts::PI;

use rodas5p_core::CoreResult;

use super::{
    Declarations, LineLayout, MolOperator, OperatorStructure, SpatialOrder, StructureBuilder,
    check_len, invalid,
};

/// A latitude-longitude grid of `N_t` rings and `N_p` longitudes on the sphere of radius
/// `R`.
#[derive(Clone, Debug, PartialEq)]
pub struct SphereGrid {
    rings: usize,
    longitudes: usize,
    radius: f64,
    dtheta: f64,
    dphi: f64,
    areas: Vec<f64>,
    sin_mid: Vec<f64>,
}

impl SphereGrid {
    /// `rings >= 2`, `longitudes >= 2`, a positive finite radius.
    pub fn new(rings: usize, longitudes: usize, radius: f64) -> CoreResult<Self> {
        if rings < 2 || longitudes < 2 {
            return Err(invalid(
                "a sphere grid needs at least two rings and two longitudes",
            ));
        }
        rings
            .checked_mul(longitudes)
            .filter(|&n| n <= isize::MAX as usize / (16 * std::mem::size_of::<f64>()))
            .ok_or_else(|| invalid("sphere grid size overflows"))?;
        if !radius.is_finite() || radius <= 0.0 {
            return Err(invalid("sphere radius must be positive and finite"));
        }
        let dtheta = PI / rings as f64;
        let dphi = 2.0 * PI / longitudes as f64;
        let mut areas = Vec::with_capacity(rings);
        let mut sin_mid = Vec::with_capacity(rings);
        for i in 0..rings {
            let s = ((i as f64 + 0.5) * dtheta).sin();
            // cos(theta_lo) - cos(theta_hi) without a subtraction near the poles.
            let area = (radius * (0.5 * dtheta).sin()) * (radius * dphi) * (2.0 * s);
            if !area.is_finite() || area <= 0.0 {
                return Err(invalid("spherical cell area is outside binary64 range"));
            }
            areas.push(area);
            sin_mid.push(s);
        }
        Ok(Self {
            rings,
            longitudes,
            radius,
            dtheta,
            dphi,
            areas,
            sin_mid,
        })
    }

    /// Number of colatitude rings.
    pub fn rings(&self) -> usize {
        self.rings
    }

    /// Number of longitudes.
    pub fn longitudes(&self) -> usize {
        self.longitudes
    }

    /// Radius.
    pub fn radius(&self) -> f64 {
        self.radius
    }

    /// Number of cells.
    pub fn count(&self) -> usize {
        self.rings * self.longitudes
    }

    /// Ring width `pi / N_t`.
    pub fn dtheta(&self) -> f64 {
        self.dtheta
    }

    /// Longitude width `2 pi / N_p`.
    pub fn dphi(&self) -> f64 {
        self.dphi
    }

    /// Area of every cell of `ring`.
    pub fn ring_area(&self, ring: usize) -> f64 {
        self.areas[ring]
    }

    /// `(colatitude, longitude)` of a cell centre.
    pub fn centre(&self, cell: usize) -> (f64, f64) {
        let ring = cell / self.longitudes;
        let j = cell % self.longitudes;
        (
            (ring as f64 + 0.5) * self.dtheta,
            (j as f64 + 0.5) * self.dphi,
        )
    }

    /// Line layouts: axis 0 the periodic longitude lines (one per ring), axis 1 the
    /// colatitude lines from pole to pole (one per longitude).
    pub fn lines(&self) -> Vec<LineLayout> {
        vec![
            LineLayout {
                axis: 0,
                count: self.rings,
                length: self.longitudes,
                stride: 1,
                periodic: true,
            },
            LineLayout {
                axis: 1,
                count: self.longitudes,
                length: self.rings,
                stride: self.longitudes,
                periodic: false,
            },
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RingRates {
    north: f64,
    south: f64,
    azimuth: f64,
}

/// The finite-volume Laplace-Beltrami operator `u_t = D Delta_S u` on a [`SphereGrid`]:
/// second order away from the poles, first order on the two pole rings for fields that vary
/// with longitude (see the module documentation), conservative, and symmetric negative
/// semidefinite in the area-weighted inner product.
#[derive(Clone, Debug)]
pub struct SphereLaplacian {
    name: String,
    grid: SphereGrid,
    diffusivity: f64,
    rates: Vec<RingRates>,
    structure: OperatorStructure,
}

impl SphereLaplacian {
    /// The operator with diffusivity `D >= 0` (length squared per time).
    pub fn new(grid: SphereGrid, diffusivity: f64) -> CoreResult<Self> {
        if !diffusivity.is_finite() || diffusivity < 0.0 {
            return Err(invalid("sphere diffusivity must be finite and nonnegative"));
        }
        let (nt, np) = (grid.rings, grid.longitudes);
        let (dt, dp) = (grid.dtheta, grid.dphi);
        let mut rates = Vec::with_capacity(nt);
        for i in 0..nt {
            let area = grid.areas[i];
            let theta_rate = |face: usize| {
                if face == 0 || face == nt {
                    0.0
                } else {
                    diffusivity * ((face as f64 * dt).sin() * dp / dt) / area
                }
            };
            let ring = RingRates {
                north: theta_rate(i),
                south: theta_rate(i + 1),
                azimuth: diffusivity * (dt / (grid.sin_mid[i] * dp)) / area,
            };
            if [ring.north, ring.south, ring.azimuth]
                .iter()
                .any(|v| !v.is_finite())
                || (diffusivity > 0.0
                    && (ring.azimuth == 0.0
                        || (i > 0 && ring.north == 0.0)
                        || (i + 1 < nt && ring.south == 0.0)))
            {
                return Err(invalid("spherical diffusion rate is outside binary64 range"));
            }
            rates.push(ring);
        }
        let n = grid.count();
        let mut builder = StructureBuilder::new(n, 5)?;
        for ring in 0..nt {
            for j in 0..np {
                let cell = ring * np + j;
                let start = ring * np;
                builder.set(cell, 1, start + if j == 0 { np - 1 } else { j - 1 });
                builder.set(cell, 2, start + if j + 1 == np { 0 } else { j + 1 });
                if ring > 0 {
                    builder.set(cell, 3, cell - np);
                }
                if ring + 1 < nt {
                    builder.set(cell, 4, cell + np);
                }
            }
        }
        let weights = (0..n).map(|cell| grid.areas[cell / np]).collect();
        let structure = builder.finish(Declarations {
            species: 1,
            weights,
            stencil_radius: 1,
            linear: true,
            conservative: true,
            order: SpatialOrder::Second,
            lines: grid.lines(),
        })?;
        Ok(Self {
            name: format!("mol-sphere-laplacian-{nt}x{np}"),
            grid,
            diffusivity,
            rates,
            structure,
        })
    }

    /// The grid.
    pub fn grid(&self) -> &SphereGrid {
        &self.grid
    }

    /// The diffusivity.
    pub fn diffusivity(&self) -> f64 {
        self.diffusivity
    }

    /// Longitude coupling `D dtheta / (sin(theta_mid) dphi) / area` of every cell of `ring`
    /// (the rate of a periodic longitude line).
    pub fn azimuth_rate(&self, ring: usize) -> f64 {
        self.rates[ring].azimuth
    }

    fn apply_unchecked(&self, u: &[f64], out: &mut [f64]) {
        let np = self.grid.longitudes;
        let nt = self.grid.rings;
        for (ring, rate) in self.rates.iter().enumerate() {
            let start = ring * np;
            let north_row = if ring > 0 { start - np } else { start };
            let south_row = if ring + 1 < nt { start + np } else { start };
            for x in 0..np {
                let c = u[start + x];
                let xm = if x == 0 { np - 1 } else { x - 1 };
                let xp = if x + 1 == np { 0 } else { x + 1 };
                let horizontal = rate.azimuth * ((u[start + xm] - c) + (u[start + xp] - c));
                out[start + x] = horizontal
                    + rate.north * (u[north_row + x] - c)
                    + rate.south * (u[south_row + x] - c);
            }
        }
    }
}

impl MolOperator for SphereLaplacian {
    fn name(&self) -> &str {
        &self.name
    }

    fn structure(&self) -> &OperatorStructure {
        &self.structure
    }

    fn apply_into(&self, _t: f64, u: &[f64], out: &mut [f64]) -> CoreResult<()> {
        let n = self.grid.count();
        check_len(n, u.len(), "sphere Laplacian state")?;
        check_len(n, out.len(), "sphere Laplacian output")?;
        self.apply_unchecked(u, out);
        Ok(())
    }

    fn jvp_into(&self, _t: f64, u: &[f64], v: &[f64], out: &mut [f64]) -> CoreResult<()> {
        let n = self.grid.count();
        check_len(n, u.len(), "sphere Laplacian JVP state")?;
        check_len(n, v.len(), "sphere Laplacian JVP direction")?;
        check_len(n, out.len(), "sphere Laplacian JVP output")?;
        self.apply_unchecked(v, out);
        Ok(())
    }

    fn jacobian_values_into(&self, _t: f64, u: &[f64], values: &mut [f64]) -> CoreResult<()> {
        let n = self.grid.count();
        check_len(n, u.len(), "sphere Laplacian Jacobian state")?;
        check_len(
            self.structure.pattern().nnz(),
            values.len(),
            "sphere Laplacian Jacobian values",
        )?;
        values.fill(0.0);
        let np = self.grid.longitudes;
        for (ring, rate) in self.rates.iter().enumerate() {
            let row = [
                -rate.azimuth - rate.azimuth - rate.north - rate.south,
                rate.azimuth,
                rate.azimuth,
                rate.north,
                rate.south,
            ];
            for x in 0..np {
                self.structure.scatter(values, ring * np + x, &row);
            }
        }
        Ok(())
    }
}
