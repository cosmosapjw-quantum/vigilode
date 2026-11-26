//! Reaction-diffusion and transport right-hand sides on Cartesian grids: Gray-Scott,
//! Brusselator, Allen-Cahn, sine-Gordon and viscous Burgers. Species are interleaved
//! (`index = s + S cell`); the diffusion part is the cell-centred Laplacian of
//! [`super::CartesianLaplacian`] with the same boundaries.

use rodas5p_core::CoreResult;

use super::cartesian::{CartesianGrid, Side};
use super::{
    Declarations, MolOperator, OperatorStructure, SpatialOrder, StructureBuilder, check_len,
    invalid,
};

fn finite_parameters(values: &[f64], what: &str) -> CoreResult<()> {
    if values.iter().all(|v| v.is_finite()) {
        Ok(())
    } else {
        Err(invalid(&format!("{what} parameters must be finite")))
    }
}

fn isotropic(grid: &CartesianGrid, diffusivity: f64) -> CoreResult<[f64; 3]> {
    let d = [diffusivity; 3];
    grid.rates(&d[..grid.dims()])
}

/// Pointwise kinetics of a two-species system.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kinetics {
    GrayScott { feed: f64, kill: f64 },
    Brusselator { a: f64, b: f64 },
}

impl Kinetics {
    #[inline]
    fn reaction(&self, u: f64, v: f64) -> (f64, f64) {
        match *self {
            Self::GrayScott { feed, kill } => {
                let uv2 = u * v * v;
                (-uv2 + feed * (1.0 - u), uv2 - (feed + kill) * v)
            }
            Self::Brusselator { a, b } => {
                let u2v = u * u * v;
                (a + u2v - (b + 1.0) * u, b * u - u2v)
            }
        }
    }

    /// `(d R_u / du, d R_u / dv, d R_v / du, d R_v / dv)`.
    #[inline]
    fn jacobian(&self, u: f64, v: f64) -> (f64, f64, f64, f64) {
        match *self {
            Self::GrayScott { feed, kill } => {
                let (v2, uv2) = (v * v, 2.0 * u * v);
                (-v2 - feed, -uv2, v2, uv2 - (feed + kill))
            }
            Self::Brusselator { b, .. } => {
                let (uv2, u2) = (2.0 * u * v, u * u);
                (uv2 - (b + 1.0), u2, b - uv2, -u2)
            }
        }
    }
}

/// A two-species reaction-diffusion system with isotropic diffusivity per species.
#[derive(Clone, Debug)]
struct TwoSpecies {
    name: String,
    grid: CartesianGrid,
    rates: [[f64; 3]; 2],
    kinetics: Kinetics,
    structure: OperatorStructure,
}

impl TwoSpecies {
    fn new(
        label: &str,
        grid: CartesianGrid,
        diffusivity: [f64; 2],
        kinetics: Kinetics,
    ) -> CoreResult<Self> {
        let rates = [
            isotropic(&grid, diffusivity[0])?,
            isotropic(&grid, diffusivity[1])?,
        ];
        let dims = grid.dims();
        let n = 2 * grid.count();
        let cross = 1 + 2 * dims;
        let mut builder = StructureBuilder::new(n, 2 + 2 * dims)?;
        grid.for_each_cell(|cell, coords| {
            for s in 0..2 {
                let row = s + 2 * cell;
                grid.set_neighbour_slots(&mut builder, row, 1, 2, s, cell, coords);
                builder.set(row, cross, (1 - s) + 2 * cell);
            }
        });
        let structure = builder.finish(Declarations {
            species: 2,
            weights: grid.weights(2),
            stencil_radius: 1,
            linear: false,
            conservative: false,
            order: SpatialOrder::Second,
            lines: grid.lines(2),
        })?;
        Ok(Self {
            name: format!("mol-{label}-{}", grid.name()),
            grid,
            rates,
            kinetics,
            structure,
        })
    }

    fn apply_into(&self, u: &[f64], out: &mut [f64]) -> CoreResult<()> {
        let n = self.structure.dimension();
        check_len(n, u.len(), "reaction-diffusion state")?;
        check_len(n, out.len(), "reaction-diffusion output")?;
        let grid = &self.grid;
        grid.for_each_cell(|cell, coords| {
            let (a, b) = (u[2 * cell], u[2 * cell + 1]);
            let (ra, rb) = self.kinetics.reaction(a, b);
            out[2 * cell] = grid.laplacian_at(&self.rates[0], u, 2, 0, cell, coords) + ra;
            out[2 * cell + 1] = grid.laplacian_at(&self.rates[1], u, 2, 1, cell, coords) + rb;
        });
        Ok(())
    }

    fn jvp_into(&self, u: &[f64], v: &[f64], out: &mut [f64]) -> CoreResult<()> {
        let n = self.structure.dimension();
        check_len(n, u.len(), "reaction-diffusion JVP state")?;
        check_len(n, v.len(), "reaction-diffusion JVP direction")?;
        check_len(n, out.len(), "reaction-diffusion JVP output")?;
        let grid = &self.grid;
        grid.for_each_cell(|cell, coords| {
            let (a, b) = (u[2 * cell], u[2 * cell + 1]);
            let (da, db) = (v[2 * cell], v[2 * cell + 1]);
            let (j00, j01, j10, j11) = self.kinetics.jacobian(a, b);
            out[2 * cell] =
                grid.laplacian_at(&self.rates[0], v, 2, 0, cell, coords) + (j00 * da + j01 * db);
            out[2 * cell + 1] =
                grid.laplacian_at(&self.rates[1], v, 2, 1, cell, coords) + (j10 * da + j11 * db);
        });
        Ok(())
    }

    fn jacobian_values_into(&self, u: &[f64], values: &mut [f64]) -> CoreResult<()> {
        check_len(self.structure.dimension(), u.len(), "reaction-diffusion Jacobian state")?;
        check_len(
            self.structure.pattern().nnz(),
            values.len(),
            "reaction-diffusion Jacobian values",
        )?;
        values.fill(0.0);
        let grid = &self.grid;
        let dims = grid.dims();
        let width = 2 + 2 * dims;
        grid.for_each_cell(|cell, coords| {
            let (j00, j01, j10, j11) = self.kinetics.jacobian(u[2 * cell], u[2 * cell + 1]);
            let reaction = [(j00, j01), (j11, j10)];
            for (s, &(own, other)) in reaction.iter().enumerate() {
                let (centre, off) = grid.laplacian_row(&self.rates[s], cell, coords);
                let mut row = [0.0; 8];
                row[0] = centre + own;
                row[1..1 + 2 * dims].copy_from_slice(&off[..2 * dims]);
                row[1 + 2 * dims] = other;
                self.structure.scatter(values, s + 2 * cell, &row[..width]);
            }
        });
        Ok(())
    }
}

macro_rules! delegate_two_species {
    ($ty:ident) => {
        impl MolOperator for $ty {
            fn name(&self) -> &str {
                &self.0.name
            }

            fn structure(&self) -> &OperatorStructure {
                &self.0.structure
            }

            fn apply_into(&self, _t: f64, u: &[f64], out: &mut [f64]) -> CoreResult<()> {
                self.0.apply_into(u, out)
            }

            fn jvp_into(&self, _t: f64, u: &[f64], v: &[f64], out: &mut [f64]) -> CoreResult<()> {
                self.0.jvp_into(u, v, out)
            }

            fn jacobian_values_into(
                &self,
                _t: f64,
                u: &[f64],
                values: &mut [f64],
            ) -> CoreResult<()> {
                self.0.jacobian_values_into(u, values)
            }
        }

        impl $ty {
            /// The grid.
            pub fn grid(&self) -> &CartesianGrid {
                &self.0.grid
            }
        }
    };
}

/// Gray-Scott: `u_t = D_u L u - u v^2 + F (1 - u)`, `v_t = D_v L v + u v^2 - (F + k) v`,
/// species interleaved `(u, v)`. Second order; not conservative; nonlinear.
#[derive(Clone, Debug)]
pub struct GrayScott(TwoSpecies);

impl GrayScott {
    /// Diffusivities `D_u, D_v >= 0`, feed `F` and kill `k`.
    pub fn new(grid: CartesianGrid, du: f64, dv: f64, feed: f64, kill: f64) -> CoreResult<Self> {
        finite_parameters(&[feed, kill], "Gray-Scott")?;
        Ok(Self(TwoSpecies::new(
            "gray-scott",
            grid,
            [du, dv],
            Kinetics::GrayScott { feed, kill },
        )?))
    }
}

delegate_two_species!(GrayScott);

/// Brusselator: `u_t = A + u^2 v - (B + 1) u + D_u L u`, `v_t = B u - u^2 v + D_v L v`,
/// species interleaved `(u, v)`. Second order; not conservative; nonlinear.
#[derive(Clone, Debug)]
pub struct Brusselator(TwoSpecies);

impl Brusselator {
    /// Parameters `A`, `B` and diffusivities `D_u, D_v >= 0`.
    pub fn new(grid: CartesianGrid, a: f64, b: f64, du: f64, dv: f64) -> CoreResult<Self> {
        finite_parameters(&[a, b], "Brusselator")?;
        Ok(Self(TwoSpecies::new(
            "brusselator",
            grid,
            [du, dv],
            Kinetics::Brusselator { a, b },
        )?))
    }
}

delegate_two_species!(Brusselator);

/// Allen-Cahn: `u_t = D L u + u - u^3` (`D = epsilon^2`). Second order; not conservative;
/// nonlinear.
#[derive(Clone, Debug)]
pub struct AllenCahn {
    name: String,
    grid: CartesianGrid,
    rates: [f64; 3],
    structure: OperatorStructure,
}

impl AllenCahn {
    /// Diffusivity `D >= 0`.
    pub fn new(grid: CartesianGrid, diffusivity: f64) -> CoreResult<Self> {
        let rates = isotropic(&grid, diffusivity)?;
        let dims = grid.dims();
        let mut builder = StructureBuilder::new(grid.count(), 1 + 2 * dims)?;
        grid.for_each_cell(|cell, coords| {
            grid.set_neighbour_slots(&mut builder, cell, 1, 1, 0, cell, coords);
        });
        let structure = builder.finish(Declarations {
            species: 1,
            weights: grid.weights(1),
            stencil_radius: 1,
            linear: false,
            conservative: false,
            order: SpatialOrder::Second,
            lines: grid.lines(1),
        })?;
        Ok(Self {
            name: format!("mol-allen-cahn-{}", grid.name()),
            grid,
            rates,
            structure,
        })
    }

    /// The grid.
    pub fn grid(&self) -> &CartesianGrid {
        &self.grid
    }
}

impl MolOperator for AllenCahn {
    fn name(&self) -> &str {
        &self.name
    }

    fn structure(&self) -> &OperatorStructure {
        &self.structure
    }

    fn apply_into(&self, _t: f64, u: &[f64], out: &mut [f64]) -> CoreResult<()> {
        let n = self.grid.count();
        check_len(n, u.len(), "Allen-Cahn state")?;
        check_len(n, out.len(), "Allen-Cahn output")?;
        self.grid.for_each_cell(|cell, coords| {
            let c = u[cell];
            out[cell] = self.grid.laplacian_at(&self.rates, u, 1, 0, cell, coords) + (c - c * c * c);
        });
        Ok(())
    }

    fn jvp_into(&self, _t: f64, u: &[f64], v: &[f64], out: &mut [f64]) -> CoreResult<()> {
        let n = self.grid.count();
        check_len(n, u.len(), "Allen-Cahn JVP state")?;
        check_len(n, v.len(), "Allen-Cahn JVP direction")?;
        check_len(n, out.len(), "Allen-Cahn JVP output")?;
        self.grid.for_each_cell(|cell, coords| {
            let c = u[cell];
            out[cell] = self.grid.laplacian_at(&self.rates, v, 1, 0, cell, coords)
                + (1.0 - 3.0 * c * c) * v[cell];
        });
        Ok(())
    }

    fn jacobian_values_into(&self, _t: f64, u: &[f64], values: &mut [f64]) -> CoreResult<()> {
        check_len(self.grid.count(), u.len(), "Allen-Cahn Jacobian state")?;
        check_len(
            self.structure.pattern().nnz(),
            values.len(),
            "Allen-Cahn Jacobian values",
        )?;
        values.fill(0.0);
        let width = 1 + 2 * self.grid.dims();
        self.grid.for_each_cell(|cell, coords| {
            let c = u[cell];
            let (centre, off) = self.grid.laplacian_row(&self.rates, cell, coords);
            let mut row = [0.0; 7];
            row[0] = centre + (1.0 - 3.0 * c * c);
            row[1..7].copy_from_slice(&off);
            self.structure.scatter(values, cell, &row[..width]);
        });
        Ok(())
    }
}

/// Sine-Gordon: `phi_t = psi`, `psi_t = c^2 L phi - sin(phi)`, species interleaved
/// `(phi, psi)`. Second order; not conservative in the mass sense; nonlinear.
#[derive(Clone, Debug)]
pub struct SineGordon {
    name: String,
    grid: CartesianGrid,
    rates: [f64; 3],
    structure: OperatorStructure,
}

impl SineGordon {
    /// Squared wave speed `c^2 >= 0`.
    pub fn new(grid: CartesianGrid, wave_speed_squared: f64) -> CoreResult<Self> {
        let rates = isotropic(&grid, wave_speed_squared)?;
        let dims = grid.dims();
        let mut builder = StructureBuilder::new(2 * grid.count(), 2 + 2 * dims)?;
        grid.for_each_cell(|cell, coords| {
            builder.set(2 * cell, 1, 2 * cell + 1);
            builder.set(2 * cell + 1, 1, 2 * cell);
            grid.set_neighbour_slots(&mut builder, 2 * cell + 1, 2, 2, 0, cell, coords);
        });
        let structure = builder.finish(Declarations {
            species: 2,
            weights: grid.weights(2),
            stencil_radius: 1,
            linear: false,
            conservative: false,
            order: SpatialOrder::Second,
            lines: grid.lines(2),
        })?;
        Ok(Self {
            name: format!("mol-sine-gordon-{}", grid.name()),
            grid,
            rates,
            structure,
        })
    }

    /// The grid.
    pub fn grid(&self) -> &CartesianGrid {
        &self.grid
    }
}

impl MolOperator for SineGordon {
    fn name(&self) -> &str {
        &self.name
    }

    fn structure(&self) -> &OperatorStructure {
        &self.structure
    }

    fn apply_into(&self, _t: f64, u: &[f64], out: &mut [f64]) -> CoreResult<()> {
        let n = self.structure.dimension();
        check_len(n, u.len(), "sine-Gordon state")?;
        check_len(n, out.len(), "sine-Gordon output")?;
        self.grid.for_each_cell(|cell, coords| {
            out[2 * cell] = u[2 * cell + 1];
            out[2 * cell + 1] = self.grid.laplacian_at(&self.rates, u, 2, 0, cell, coords)
                - u[2 * cell].sin();
        });
        Ok(())
    }

    fn jvp_into(&self, _t: f64, u: &[f64], v: &[f64], out: &mut [f64]) -> CoreResult<()> {
        let n = self.structure.dimension();
        check_len(n, u.len(), "sine-Gordon JVP state")?;
        check_len(n, v.len(), "sine-Gordon JVP direction")?;
        check_len(n, out.len(), "sine-Gordon JVP output")?;
        self.grid.for_each_cell(|cell, coords| {
            out[2 * cell] = v[2 * cell + 1];
            out[2 * cell + 1] = self.grid.laplacian_at(&self.rates, v, 2, 0, cell, coords)
                - u[2 * cell].cos() * v[2 * cell];
        });
        Ok(())
    }

    fn jacobian_values_into(&self, _t: f64, u: &[f64], values: &mut [f64]) -> CoreResult<()> {
        check_len(self.structure.dimension(), u.len(), "sine-Gordon Jacobian state")?;
        check_len(
            self.structure.pattern().nnz(),
            values.len(),
            "sine-Gordon Jacobian values",
        )?;
        values.fill(0.0);
        let dims = self.grid.dims();
        let width = 2 + 2 * dims;
        self.grid.for_each_cell(|cell, coords| {
            let mut phi_row = [0.0; 8];
            phi_row[1] = 1.0;
            self.structure.scatter(values, 2 * cell, &phi_row[..width]);
            let (centre, off) = self.grid.laplacian_row(&self.rates, cell, coords);
            let mut psi_row = [0.0; 8];
            psi_row[1] = centre - u[2 * cell].cos();
            psi_row[2..2 + 2 * dims].copy_from_slice(&off[..2 * dims]);
            self.structure.scatter(values, 2 * cell + 1, &psi_row[..width]);
        });
        Ok(())
    }
}

/// Engquist-Osher flux of `u^2 / 2`.
#[inline]
fn eo_flux(left: f64, right: f64) -> f64 {
    let (a, b) = (left.max(0.0), right.min(0.0));
    0.5 * (a * a + b * b)
}

/// Viscous Burgers, scalar and conservative: `u_t + sum_a d_a (u^2 / 2) = nu L u`, with the
/// Engquist-Osher flux `(max(u_l, 0)^2 + min(u_r, 0)^2) / 2` (monotone, continuously
/// differentiable, first order) and the cell-centred Laplacian. Boundaries: `Periodic` or
/// `DirichletZero` (face value zero for the flux and the diffusion); `NeumannZero` is
/// refused. First order; conservative when every axis is periodic; nonlinear.
#[derive(Clone, Debug)]
pub struct Burgers {
    name: String,
    grid: CartesianGrid,
    rates: [f64; 3],
    structure: OperatorStructure,
}

impl Burgers {
    /// Viscosity `nu >= 0`.
    pub fn new(grid: CartesianGrid, viscosity: f64) -> CoreResult<Self> {
        grid.require_no_neumann("Burgers")?;
        let rates = isotropic(&grid, viscosity)?;
        let dims = grid.dims();
        let mut builder = StructureBuilder::new(grid.count(), 1 + 2 * dims)?;
        grid.for_each_cell(|cell, coords| {
            grid.set_neighbour_slots(&mut builder, cell, 1, 1, 0, cell, coords);
        });
        let structure = builder.finish(Declarations {
            species: 1,
            weights: grid.weights(1),
            stencil_radius: 1,
            linear: false,
            conservative: grid
                .boundaries()
                .iter()
                .all(|b| *b == super::Boundary::Periodic),
            order: SpatialOrder::First,
            lines: grid.lines(1),
        })?;
        Ok(Self {
            name: format!("mol-burgers-{}", grid.name()),
            grid,
            rates,
            structure,
        })
    }

    /// The grid.
    pub fn grid(&self) -> &CartesianGrid {
        &self.grid
    }

    #[inline]
    fn neighbours(&self, u: &[f64], cell: usize, coords: [usize; 3], axis: usize) -> (f64, f64) {
        let up = match self.grid.prev(cell, coords[axis], axis) {
            Side::Cell(p) => u[p],
            Side::Ghost => 0.0,
        };
        let un = match self.grid.next(cell, coords[axis], axis) {
            Side::Cell(p) => u[p],
            Side::Ghost => 0.0,
        };
        (up, un)
    }
}

impl MolOperator for Burgers {
    fn name(&self) -> &str {
        &self.name
    }

    fn structure(&self) -> &OperatorStructure {
        &self.structure
    }

    fn apply_into(&self, _t: f64, u: &[f64], out: &mut [f64]) -> CoreResult<()> {
        let n = self.grid.count();
        check_len(n, u.len(), "Burgers state")?;
        check_len(n, out.len(), "Burgers output")?;
        self.grid.for_each_cell(|cell, coords| {
            let c = u[cell];
            let mut advection = 0.0;
            for axis in 0..self.grid.dims() {
                let (up, un) = self.neighbours(u, cell, coords, axis);
                advection += (eo_flux(c, un) - eo_flux(up, c)) / self.grid.spacing()[axis];
            }
            out[cell] = self.grid.laplacian_at(&self.rates, u, 1, 0, cell, coords) - advection;
        });
        Ok(())
    }

    fn jvp_into(&self, _t: f64, u: &[f64], v: &[f64], out: &mut [f64]) -> CoreResult<()> {
        let n = self.grid.count();
        check_len(n, u.len(), "Burgers JVP state")?;
        check_len(n, v.len(), "Burgers JVP direction")?;
        check_len(n, out.len(), "Burgers JVP output")?;
        self.grid.for_each_cell(|cell, coords| {
            let c = u[cell];
            let mut advection = 0.0;
            for axis in 0..self.grid.dims() {
                let (up, un) = self.neighbours(u, cell, coords, axis);
                let (vp, vn) = self.neighbours(v, cell, coords, axis);
                let d_plus = c.max(0.0) * v[cell] + un.min(0.0) * vn;
                let d_minus = up.max(0.0) * vp + c.min(0.0) * v[cell];
                advection += (d_plus - d_minus) / self.grid.spacing()[axis];
            }
            out[cell] = self.grid.laplacian_at(&self.rates, v, 1, 0, cell, coords) - advection;
        });
        Ok(())
    }

    fn jacobian_values_into(&self, _t: f64, u: &[f64], values: &mut [f64]) -> CoreResult<()> {
        check_len(self.grid.count(), u.len(), "Burgers Jacobian state")?;
        check_len(
            self.structure.pattern().nnz(),
            values.len(),
            "Burgers Jacobian values",
        )?;
        values.fill(0.0);
        let width = 1 + 2 * self.grid.dims();
        self.grid.for_each_cell(|cell, coords| {
            let c = u[cell];
            let (centre, off) = self.grid.laplacian_row(&self.rates, cell, coords);
            let mut row = [0.0; 7];
            row[0] = centre;
            row[1..7].copy_from_slice(&off);
            for axis in 0..self.grid.dims() {
                let h = self.grid.spacing()[axis];
                let (up, un) = self.neighbours(u, cell, coords, axis);
                row[0] -= (c.max(0.0) - c.min(0.0)) / h;
                if let Side::Cell(_) = self.grid.prev(cell, coords[axis], axis) {
                    row[1 + 2 * axis] += up.max(0.0) / h;
                }
                if let Side::Cell(_) = self.grid.next(cell, coords[axis], axis) {
                    row[2 + 2 * axis] -= un.min(0.0) / h;
                }
            }
            self.structure.scatter(values, cell, &row[..width]);
        });
        Ok(())
    }
}
