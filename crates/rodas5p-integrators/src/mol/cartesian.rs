//! Uniform cell-centred Cartesian grids and their Laplacian (1D, 2D, 3D, anisotropic box).

use rodas5p_core::CoreResult;

use super::{
    Declarations, LineLayout, MolOperator, OperatorStructure, SpatialOrder, StructureBuilder,
    check_len, invalid,
};

/// Boundary condition of one Cartesian axis, at both of its ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// `u(x + L) = u(x)`: the first and the last cell are neighbours.
    Periodic,
    /// Zero normal derivative: no diffusive flux through the boundary face (ghost value =
    /// cell value). Conservative.
    NeumannZero,
    /// `u = 0` on the boundary face: the diffusion ghost value is minus the cell value
    /// (linear interpolation to the face), an advective inflow carries the value zero.
    DirichletZero,
}

/// A neighbour along one axis: a cell, or the ghost beyond a non-periodic boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    Cell(usize),
    Ghost,
}

/// A uniform cell-centred grid on `[0, L_0] x ... x [0, L_(d-1)]`, `d` in `1..=3`, with
/// `N_a` cells of width `h_a = L_a / N_a` per axis, cell centres at `(i + 1/2) h_a` and the
/// first axis fastest in the state layout.
#[derive(Clone, Debug, PartialEq)]
pub struct CartesianGrid {
    dims: usize,
    cells: [usize; 3],
    lengths: [f64; 3],
    spacing: [f64; 3],
    boundary: [Boundary; 3],
    strides: [usize; 3],
    count: usize,
    cell_volume: f64,
}

impl CartesianGrid {
    /// A grid with `cells.len()` axes (one to three). Every axis needs at least two cells, a
    /// positive finite length and a boundary condition.
    pub fn new(cells: &[usize], lengths: &[f64], boundary: &[Boundary]) -> CoreResult<Self> {
        let dims = cells.len();
        if !(1..=3).contains(&dims) || lengths.len() != dims || boundary.len() != dims {
            return Err(invalid(
                "a Cartesian grid has one to three axes, each with a cell count, a length and a boundary",
            ));
        }
        if cells.iter().any(|&n| n < 2) {
            return Err(invalid("each Cartesian axis needs at least two cells"));
        }
        let mut grid = Self {
            dims,
            cells: [1; 3],
            lengths: [1.0; 3],
            spacing: [1.0; 3],
            boundary: [Boundary::Periodic; 3],
            strides: [0; 3],
            count: 1,
            cell_volume: 1.0,
        };
        let mut stride = 1usize;
        for a in 0..dims {
            let h = lengths[a] / cells[a] as f64;
            if !lengths[a].is_finite() || lengths[a] <= 0.0 || !h.is_finite() || h <= 0.0 {
                return Err(invalid("Cartesian axis lengths must be positive and finite"));
            }
            grid.cells[a] = cells[a];
            grid.lengths[a] = lengths[a];
            grid.spacing[a] = h;
            grid.boundary[a] = boundary[a];
            grid.strides[a] = stride;
            grid.cell_volume *= h;
            stride = stride
                .checked_mul(cells[a])
                .filter(|&s| s <= isize::MAX as usize / (16 * std::mem::size_of::<f64>()))
                .ok_or_else(|| invalid("Cartesian grid size overflows"))?;
        }
        if !grid.cell_volume.is_finite() || grid.cell_volume <= 0.0 {
            return Err(invalid("Cartesian cell measure is outside binary64 range"));
        }
        grid.count = stride;
        Ok(grid)
    }

    /// Number of axes.
    pub fn dims(&self) -> usize {
        self.dims
    }

    /// Cells per axis.
    pub fn cells(&self) -> &[usize] {
        &self.cells[..self.dims]
    }

    /// Axis lengths.
    pub fn lengths(&self) -> &[f64] {
        &self.lengths[..self.dims]
    }

    /// Cell widths `h_a`.
    pub fn spacing(&self) -> &[f64] {
        &self.spacing[..self.dims]
    }

    /// Boundary condition per axis.
    pub fn boundaries(&self) -> &[Boundary] {
        &self.boundary[..self.dims]
    }

    /// Number of cells.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Cell measure (length, area or volume): the quadrature weight of every cell.
    pub fn cell_volume(&self) -> f64 {
        self.cell_volume
    }

    /// Grid indices `[i, j, k]` of a cell (unused axes 0).
    pub fn coordinates(&self, cell: usize) -> [usize; 3] {
        let mut out = [0; 3];
        let mut rest = cell;
        for (a, slot) in out.iter_mut().enumerate().take(self.dims) {
            *slot = rest % self.cells[a];
            rest /= self.cells[a];
        }
        out
    }

    /// Centre of a cell (unused axes 0).
    pub fn centre(&self, cell: usize) -> [f64; 3] {
        let c = self.coordinates(cell);
        let mut x = [0.0; 3];
        for a in 0..self.dims {
            x[a] = (c[a] as f64 + 0.5) * self.spacing[a];
        }
        x
    }

    /// Whether no axis has a [`Boundary::DirichletZero`] end.
    pub fn is_closed(&self) -> bool {
        self.boundaries()
            .iter()
            .all(|b| *b != Boundary::DirichletZero)
    }

    /// One line layout per axis for `species` interleaved fields.
    pub fn lines(&self, species: usize) -> Vec<LineLayout> {
        (0..self.dims)
            .map(|a| LineLayout {
                axis: a,
                count: self.count * species / self.cells[a],
                length: self.cells[a],
                stride: species * self.strides[a],
                periodic: self.boundary[a] == Boundary::Periodic,
            })
            .collect()
    }

    pub(crate) fn weights(&self, species: usize) -> Vec<f64> {
        vec![self.cell_volume; self.count * species]
    }

    pub(crate) fn name(&self) -> String {
        let cells: Vec<String> = self.cells().iter().map(|n| n.to_string()).collect();
        format!("{}d-{}", self.dims, cells.join("x"))
    }

    #[inline]
    pub(crate) fn prev(&self, cell: usize, coord: usize, axis: usize) -> Side {
        if coord > 0 {
            Side::Cell(cell - self.strides[axis])
        } else if self.boundary[axis] == Boundary::Periodic {
            Side::Cell(cell + (self.cells[axis] - 1) * self.strides[axis])
        } else {
            Side::Ghost
        }
    }

    #[inline]
    pub(crate) fn next(&self, cell: usize, coord: usize, axis: usize) -> Side {
        if coord + 1 < self.cells[axis] {
            Side::Cell(cell + self.strides[axis])
        } else if self.boundary[axis] == Boundary::Periodic {
            Side::Cell(cell - (self.cells[axis] - 1) * self.strides[axis])
        } else {
            Side::Ghost
        }
    }

    /// `f(cell, [i, j, k])` for every cell in index order.
    #[inline]
    pub(crate) fn for_each_cell(&self, mut f: impl FnMut(usize, [usize; 3])) {
        let [nx, ny, nz] = self.cells;
        let mut cell = 0;
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    f(cell, [i, j, k]);
                    cell += 1;
                }
            }
        }
    }

    /// `sum_a rates[a] ((u_prev - u_c) + (u_next - u_c))` for field `s` of `species`
    /// interleaved fields, with the ghost values of the boundaries.
    #[inline]
    pub(crate) fn laplacian_at(
        &self,
        rates: &[f64; 3],
        u: &[f64],
        species: usize,
        s: usize,
        cell: usize,
        coords: [usize; 3],
    ) -> f64 {
        let c = u[s + species * cell];
        let mut acc = 0.0;
        for axis in 0..self.dims {
            let ghost = match self.boundary[axis] {
                Boundary::DirichletZero => -c - c,
                _ => 0.0,
            };
            let dp = match self.prev(cell, coords[axis], axis) {
                Side::Cell(p) => u[s + species * p] - c,
                Side::Ghost => ghost,
            };
            let dn = match self.next(cell, coords[axis], axis) {
                Side::Cell(p) => u[s + species * p] - c,
                Side::Ghost => ghost,
            };
            acc += rates[axis] * (dp + dn);
        }
        acc
    }

    /// Coefficients of [`Self::laplacian_at`]: the centre coefficient and, per axis, the
    /// coefficients of the previous and the next neighbour (zero for a ghost).
    #[inline]
    pub(crate) fn laplacian_row(&self, rates: &[f64; 3], cell: usize, coords: [usize; 3]) -> (f64, [f64; 6]) {
        let mut centre = 0.0;
        let mut off = [0.0; 6];
        for axis in 0..self.dims {
            let sides = [
                self.prev(cell, coords[axis], axis),
                self.next(cell, coords[axis], axis),
            ];
            for (k, side) in sides.into_iter().enumerate() {
                match side {
                    Side::Cell(_) => {
                        off[2 * axis + k] = rates[axis];
                        centre -= rates[axis];
                    }
                    Side::Ghost => {
                        if self.boundary[axis] == Boundary::DirichletZero {
                            centre -= 2.0 * rates[axis];
                        }
                    }
                }
            }
        }
        (centre, off)
    }

    /// Sets the neighbour slots `first_slot + 2 a` (previous) and `first_slot + 2 a + 1`
    /// (next) of row `row` to the columns of field `s`; a ghost keeps the diagonal.
    pub(crate) fn set_neighbour_slots(
        &self,
        builder: &mut StructureBuilder,
        row: usize,
        first_slot: usize,
        species: usize,
        s: usize,
        cell: usize,
        coords: [usize; 3],
    ) {
        for axis in 0..self.dims {
            if let Side::Cell(p) = self.prev(cell, coords[axis], axis) {
                builder.set(row, first_slot + 2 * axis, s + species * p);
            }
            if let Side::Cell(p) = self.next(cell, coords[axis], axis) {
                builder.set(row, first_slot + 2 * axis + 1, s + species * p);
            }
        }
    }

    /// `D_a / h_a^2` per axis; refuses a negative or non-finite diffusivity and a rate
    /// outside binary64 range.
    pub(crate) fn rates(&self, diffusivity: &[f64]) -> CoreResult<[f64; 3]> {
        if diffusivity.len() != self.dims {
            return Err(invalid("one diffusivity per grid axis is required"));
        }
        let mut rates = [0.0; 3];
        for a in 0..self.dims {
            let d = diffusivity[a];
            let rate = d / self.spacing[a] / self.spacing[a];
            if !d.is_finite() || d < 0.0 || !rate.is_finite() || (d > 0.0 && rate == 0.0) {
                return Err(invalid(
                    "diffusivities must be finite and nonnegative with a representable rate",
                ));
            }
            rates[a] = rate;
        }
        Ok(rates)
    }

    /// Refuses a [`Boundary::NeumannZero`] axis (advective operators define only periodic and
    /// zero-inflow ends).
    pub(crate) fn require_no_neumann(&self, what: &str) -> CoreResult<()> {
        if self.boundaries().contains(&Boundary::NeumannZero) {
            Err(invalid(&format!(
                "{what} supports Periodic and DirichletZero boundaries only"
            )))
        } else {
            Ok(())
        }
    }
}

/// The cell-centred Laplacian `u_t = sum_a D_a d_aa u` with per-axis diffusivity: the
/// five-point (2D) or seven-point (3D) finite-volume stencil, second order. With different
/// `D_a` and `h_a` per axis it is the anisotropic box operator. Conservative when no axis is
/// [`Boundary::DirichletZero`]; symmetric and negative semidefinite in the weighted inner
/// product for every boundary.
#[derive(Clone, Debug)]
pub struct CartesianLaplacian {
    name: String,
    grid: CartesianGrid,
    diffusivity: [f64; 3],
    rates: [f64; 3],
    structure: OperatorStructure,
}

impl CartesianLaplacian {
    /// The Laplacian on `grid` with one diffusivity per axis.
    pub fn new(grid: CartesianGrid, diffusivity: &[f64]) -> CoreResult<Self> {
        let rates = grid.rates(diffusivity)?;
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
            conservative: grid.is_closed(),
            order: SpatialOrder::Second,
            lines: grid.lines(1),
        })?;
        let mut d = [0.0; 3];
        d[..dims].copy_from_slice(diffusivity);
        Ok(Self {
            name: format!("mol-laplacian-{}", grid.name()),
            grid,
            diffusivity: d,
            rates,
            structure,
        })
    }

    /// One-dimensional Laplacian on `[0, length]`.
    pub fn line(cells: usize, length: f64, diffusivity: f64, boundary: Boundary) -> CoreResult<Self> {
        Self::new(
            CartesianGrid::new(&[cells], &[length], &[boundary])?,
            &[diffusivity],
        )
    }

    /// Two-dimensional Laplacian.
    pub fn rectangle(
        cells: [usize; 2],
        lengths: [f64; 2],
        diffusivity: [f64; 2],
        boundary: [Boundary; 2],
    ) -> CoreResult<Self> {
        Self::new(CartesianGrid::new(&cells, &lengths, &boundary)?, &diffusivity)
    }

    /// Three-dimensional (anisotropic) box Laplacian.
    pub fn cuboid(
        cells: [usize; 3],
        lengths: [f64; 3],
        diffusivity: [f64; 3],
        boundary: [Boundary; 3],
    ) -> CoreResult<Self> {
        Self::new(CartesianGrid::new(&cells, &lengths, &boundary)?, &diffusivity)
    }

    /// The grid.
    pub fn grid(&self) -> &CartesianGrid {
        &self.grid
    }

    /// Diffusivity per axis.
    pub fn diffusivity(&self) -> &[f64] {
        &self.diffusivity[..self.grid.dims()]
    }

    /// `D_a / h_a^2` per axis.
    pub fn rates(&self) -> &[f64] {
        &self.rates[..self.grid.dims()]
    }
}

impl MolOperator for CartesianLaplacian {
    fn name(&self) -> &str {
        &self.name
    }

    fn structure(&self) -> &OperatorStructure {
        &self.structure
    }

    fn apply_into(&self, _t: f64, u: &[f64], out: &mut [f64]) -> CoreResult<()> {
        let n = self.grid.count();
        check_len(n, u.len(), "Laplacian state")?;
        check_len(n, out.len(), "Laplacian output")?;
        self.grid.for_each_cell(|cell, coords| {
            out[cell] = self.grid.laplacian_at(&self.rates, u, 1, 0, cell, coords);
        });
        Ok(())
    }

    fn jvp_into(&self, t: f64, u: &[f64], v: &[f64], out: &mut [f64]) -> CoreResult<()> {
        check_len(self.grid.count(), u.len(), "Laplacian JVP state")?;
        self.apply_into(t, v, out)
    }

    fn jacobian_values_into(&self, _t: f64, u: &[f64], values: &mut [f64]) -> CoreResult<()> {
        check_len(self.grid.count(), u.len(), "Laplacian Jacobian state")?;
        check_len(
            self.structure.pattern().nnz(),
            values.len(),
            "Laplacian Jacobian values",
        )?;
        values.fill(0.0);
        let width = 1 + 2 * self.grid.dims();
        self.grid.for_each_cell(|cell, coords| {
            let (centre, off) = self.grid.laplacian_row(&self.rates, cell, coords);
            let mut row = [0.0; 7];
            row[0] = centre;
            row[1..7].copy_from_slice(&off);
            self.structure.scatter(values, cell, &row[..width]);
        });
        Ok(())
    }
}
