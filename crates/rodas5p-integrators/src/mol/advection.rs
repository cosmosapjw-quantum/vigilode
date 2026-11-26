//! First-order upwind advection in conservative form on a Cartesian grid.

use rodas5p_core::CoreResult;

use super::cartesian::{CartesianGrid, Side};
use super::{
    Declarations, MolOperator, OperatorStructure, SpatialOrder, StructureBuilder, check_len,
    invalid,
};

/// `u_t + sum_a c_a d_a u = 0` with constant velocities `c_a` and the first-order upwind
/// flux `F = max(c, 0) u_left + min(c, 0) u_right` through every face:
/// `f_i = -sum_a (F_(i+1/2) - F_(i-1/2)) / h_a`.
///
/// First order (declared and tested). Boundaries: [`super::Boundary::Periodic`] or
/// [`super::Boundary::DirichletZero`] (zero inflow value; the outflow face carries the
/// upwind cell value); `NeumannZero` is refused. Conservative when every axis is periodic.
/// It is dissipative but not symmetric in the weighted inner product, so
/// [`super::certify_weighted_self_adjoint`] refuses it.
#[derive(Clone, Debug)]
pub struct UpwindAdvection {
    name: String,
    grid: CartesianGrid,
    velocity: [f64; 3],
    structure: OperatorStructure,
}

impl UpwindAdvection {
    /// Advection with one finite velocity per axis.
    pub fn new(grid: CartesianGrid, velocity: &[f64]) -> CoreResult<Self> {
        grid.require_no_neumann("upwind advection")?;
        if velocity.len() != grid.dims() || !velocity.iter().all(|c| c.is_finite()) {
            return Err(invalid("one finite velocity per grid axis is required"));
        }
        let dims = grid.dims();
        let n = grid.count();
        let mut builder = StructureBuilder::new(n, 1 + 2 * dims)?;
        grid.for_each_cell(|cell, coords| {
            grid.set_neighbour_slots(&mut builder, cell, 1, 1, 0, cell, coords);
        });
        let structure = builder.finish(Declarations {
            species: 1,
            weights: grid.weights(1),
            stencil_radius: 1,
            linear: true,
            conservative: grid
                .boundaries()
                .iter()
                .all(|b| *b == super::Boundary::Periodic),
            order: SpatialOrder::First,
            lines: grid.lines(1),
        })?;
        let mut c = [0.0; 3];
        c[..dims].copy_from_slice(velocity);
        Ok(Self {
            name: format!("mol-upwind-advection-{}", grid.name()),
            grid,
            velocity: c,
            structure,
        })
    }

    /// The grid.
    pub fn grid(&self) -> &CartesianGrid {
        &self.grid
    }

    /// Velocity per axis.
    pub fn velocity(&self) -> &[f64] {
        &self.velocity[..self.grid.dims()]
    }

    fn apply_unchecked(&self, u: &[f64], out: &mut [f64]) {
        let grid = &self.grid;
        grid.for_each_cell(|cell, coords| {
            let c = u[cell];
            let mut acc = 0.0;
            for axis in 0..grid.dims() {
                let (cp, cm) = (self.velocity[axis].max(0.0), self.velocity[axis].min(0.0));
                let up = match grid.prev(cell, coords[axis], axis) {
                    Side::Cell(p) => u[p],
                    Side::Ghost => 0.0,
                };
                let un = match grid.next(cell, coords[axis], axis) {
                    Side::Cell(p) => u[p],
                    Side::Ghost => 0.0,
                };
                let f_minus = cp * up + cm * c;
                let f_plus = cp * c + cm * un;
                acc -= (f_plus - f_minus) / grid.spacing()[axis];
            }
            out[cell] = acc;
        });
    }
}

impl MolOperator for UpwindAdvection {
    fn name(&self) -> &str {
        &self.name
    }

    fn structure(&self) -> &OperatorStructure {
        &self.structure
    }

    fn apply_into(&self, _t: f64, u: &[f64], out: &mut [f64]) -> CoreResult<()> {
        let n = self.grid.count();
        check_len(n, u.len(), "advection state")?;
        check_len(n, out.len(), "advection output")?;
        self.apply_unchecked(u, out);
        Ok(())
    }

    fn jvp_into(&self, _t: f64, u: &[f64], v: &[f64], out: &mut [f64]) -> CoreResult<()> {
        let n = self.grid.count();
        check_len(n, u.len(), "advection JVP state")?;
        check_len(n, v.len(), "advection JVP direction")?;
        check_len(n, out.len(), "advection JVP output")?;
        self.apply_unchecked(v, out);
        Ok(())
    }

    fn jacobian_values_into(&self, _t: f64, u: &[f64], values: &mut [f64]) -> CoreResult<()> {
        let n = self.grid.count();
        check_len(n, u.len(), "advection Jacobian state")?;
        check_len(
            self.structure.pattern().nnz(),
            values.len(),
            "advection Jacobian values",
        )?;
        values.fill(0.0);
        let grid = &self.grid;
        let width = 1 + 2 * grid.dims();
        grid.for_each_cell(|cell, coords| {
            let mut row = [0.0; 7];
            for axis in 0..grid.dims() {
                let h = grid.spacing()[axis];
                let (cp, cm) = (self.velocity[axis].max(0.0), self.velocity[axis].min(0.0));
                row[0] -= (cp - cm) / h;
                if let Side::Cell(_) = grid.prev(cell, coords[axis], axis) {
                    row[1 + 2 * axis] = cp / h;
                }
                if let Side::Cell(_) = grid.next(cell, coords[axis], axis) {
                    row[2 + 2 * axis] = -cm / h;
                }
            }
            self.structure.scatter(values, cell, &row[..width]);
        });
        Ok(())
    }
}
