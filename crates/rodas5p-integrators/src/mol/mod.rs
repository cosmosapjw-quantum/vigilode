//! Method-of-lines operators with declared structure (loop node V11).
//!
//! Nothing in this module is selected by default: no router, driver or facade route reads it.
//!
//! # Operators
//!
//! | type | equation | order | conservative |
//! |---|---|---|---|
//! | [`CartesianLaplacian`] | `u_t = sum_a D_a d_aa u` in 1D, 2D, 3D (anisotropic box: per-axis `D_a` and `h_a`) | 2 | with `Periodic` / `NeumannZero` on every axis |
//! | [`SphereLaplacian`] | `u_t = D Laplace-Beltrami u` on the sphere of radius `R` | 2 away from the poles, 1 on the two pole rings | yes |
//! | [`UpwindAdvection`] | `u_t + sum_a c_a d_a u = 0`, first-order upwind flux | 1 | with `Periodic` on every axis |
//! | [`GrayScott`] | `u_t = D_u L u - u v^2 + F (1 - u)`, `v_t = D_v L v + u v^2 - (F + k) v` | 2 | no |
//! | [`Brusselator`] | `u_t = A + u^2 v - (B + 1) u + D_u L u`, `v_t = B u - u^2 v + D_v L v` | 2 | no |
//! | [`AllenCahn`] | `u_t = D L u + u - u^3` | 2 | no |
//! | [`SineGordon`] | `phi_t = psi`, `psi_t = c^2 L phi - sin(phi)` | 2 | no |
//! | [`Burgers`] | `u_t + sum_a d_a (u^2 / 2) = nu L u`, Engquist-Osher flux | 1 | with `Periodic` on every axis |
//!
//! # Conventions
//!
//! * Cartesian grids are uniform cell-centred finite volumes on `[0, L_a]` per axis:
//!   `h_a = L_a / N_a`, cell centres `x = (i + 1/2) h_a`, layout x fastest,
//!   `index = s + S (i + N_x (j + N_y k))` with `S` species interleaved.
//! * Boundaries ([`Boundary`]) are set per axis: `Periodic` (wrap), `NeumannZero` (zero face
//!   flux, ghost value = cell value), `DirichletZero` (face value zero: the diffusion ghost is
//!   minus the cell value, the advective inflow value is zero).
//! * The sphere follows the latitude-longitude finite-volume conventions of the facade's
//!   spatial module: `N_t` colatitude rings of width `pi / N_t`, `N_p` longitudes, longitude
//!   fastest (`index = ring N_p + j`), exact ring areas, zero flux through the zero-length
//!   polar faces. The two pole rings (first and last ring) are only first-order accurate for
//!   fields that are not zonal (a measured fact of loop node U09, tested again here); the
//!   rings in a fixed band away from the poles are second order.
//! * Diffusivities have units of length squared per time.
//!
//! # Declared structure
//!
//! Every operator implements [`MolOperator`]: allocation-free [`MolOperator::apply_into`],
//! an analytic [`MolOperator::jvp_into`], and [`MolOperator::jacobian_values_into`] on a
//! fixed [`SparsityPattern`]; its [`OperatorStructure`] declares quadrature weights (cell
//! measures), stencil radius, Jacobian bandwidth, grid lines, linearity, conservation and the
//! consistency order. [`certify_weighted_self_adjoint`] issues the weighted symmetry and
//! negative-semidefiniteness certificate `H L = L^T H`, `x^T H L x <= 0` (`H` the weights)
//! for linear operators that have it, and refuses every other operator (upwind advection is
//! refused as asymmetric even though it is dissipative).
//!
//! # From a geometry to a solver call
//!
//! ```
//! use std::sync::Arc;
//! use rodas5p_integrators::mol::{self, Boundary, Brusselator, CartesianGrid, MolOperator};
//! use rodas5p_integrators::stage_clients::sparse_lu::SparseLuStageClient;
//! use rodas5p_integrators::{
//!     AdaptiveStepConfig, OutputSchedule, integrate_rodas5p_fast_client_observed_with_atol,
//! };
//! let grid = CartesianGrid::new(&[16, 16], &[1.0, 1.0], &[Boundary::Periodic; 2])?;
//! let op: Arc<dyn MolOperator> = Arc::new(Brusselator::new(grid, 1.0, 3.4, 0.002, 0.002)?);
//! let problem = mol::ode_problem(op.clone())?;
//! let client = Box::new(SparseLuStageClient::for_operator(op.clone())?);
//! let y0 = vec![1.0; op.dimension()];
//! let config = AdaptiveStepConfig { rtol: 1e-6, atol: 1e-6, ..AdaptiveStepConfig::default() };
//! let output = OutputSchedule::new(vec![0.0, 0.1])?;
//! let atol = vec![1e-6; op.dimension()];
//! let (result, _, _) = integrate_rodas5p_fast_client_observed_with_atol(
//!     &problem, client, (0.0, 0.1), &y0, &config, &output, &atol,
//! )?;
//! assert!(result.observed.success);
//! # Ok::<(), rodas5p_core::CoreError>(())
//! ```

mod advection;
mod cartesian;
mod line;
mod reaction;
mod sphere;

pub use advection::UpwindAdvection;
pub use cartesian::{Boundary, CartesianGrid, CartesianLaplacian};
pub use line::LineTridiagonalPreconditioner;
pub use reaction::{AllenCahn, Brusselator, Burgers, GrayScott, SineGordon};
pub use sphere::{SphereGrid, SphereLaplacian};

use std::sync::Arc;

use rodas5p_core::{CoreError, CoreResult};

use crate::OdeProblem;

/// Consistency order an operator declares for smooth fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpatialOrder {
    /// First order (upwind fluxes).
    First,
    /// Second order (for [`SphereLaplacian`] away from the two pole rings, which are first
    /// order for fields that are not zonal).
    Second,
}

/// Compressed-column sparsity pattern of an operator's Jacobian: rows strictly increasing in
/// every column, the diagonal always present.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SparsityPattern {
    dimension: usize,
    col_ptr: Vec<usize>,
    row_idx: Vec<usize>,
    diagonal: Vec<usize>,
}

impl SparsityPattern {
    /// Order of the matrix.
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Number of stored entries.
    pub fn nnz(&self) -> usize {
        self.row_idx.len()
    }

    /// Column pointers (`dimension + 1` entries).
    pub fn col_ptr(&self) -> &[usize] {
        &self.col_ptr
    }

    /// Row index of every stored entry.
    pub fn row_idx(&self) -> &[usize] {
        &self.row_idx
    }

    /// Position of the diagonal entry of every column.
    pub fn diagonal_positions(&self) -> &[usize] {
        &self.diagonal
    }

    /// Position of entry `(row, col)`, if it is stored.
    pub fn position(&self, row: usize, col: usize) -> Option<usize> {
        if col >= self.dimension {
            return None;
        }
        let start = self.col_ptr[col];
        let rows = &self.row_idx[start..self.col_ptr[col + 1]];
        rows.binary_search(&row).ok().map(|k| start + k)
    }
}

/// The unknowns of one grid axis that a line solver treats together. Line `l` holds the
/// unknowns [`LineLayout::index`]`(l, p)` for `p` in `0..length`; consecutive positions
/// are `stride` apart in the state vector.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LineLayout {
    /// The grid axis of the lines.
    pub axis: usize,
    /// Number of lines (`dimension / length`).
    pub count: usize,
    /// Unknowns per line.
    pub length: usize,
    /// Distance in the state vector between consecutive unknowns of a line.
    pub stride: usize,
    /// Whether the last unknown of a line couples to the first.
    pub periodic: bool,
}

impl LineLayout {
    /// State index of position `p` of line `line`.
    pub fn index(&self, line: usize, p: usize) -> usize {
        let inner = line % self.stride;
        let outer = line / self.stride;
        inner + outer * self.stride * self.length + p * self.stride
    }
}

/// What an operator declares about itself; built once at construction.
#[derive(Clone, Debug, PartialEq)]
pub struct OperatorStructure {
    dimension: usize,
    species: usize,
    weights: Vec<f64>,
    stencil_radius: usize,
    lower_bandwidth: usize,
    upper_bandwidth: usize,
    pattern: SparsityPattern,
    slots_per_row: usize,
    slots: Vec<usize>,
    linear: bool,
    conservative: bool,
    order: SpatialOrder,
    lines: Vec<LineLayout>,
}

impl OperatorStructure {
    /// Number of unknowns.
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Unknowns per cell (interleaved).
    pub fn species(&self) -> usize {
        self.species
    }

    /// Quadrature weight of every unknown: the measure (length, area or volume) of its cell.
    pub fn weights(&self) -> &[f64] {
        &self.weights
    }

    /// Largest grid-index offset along any axis that a value of the operator reads.
    pub fn stencil_radius(&self) -> usize {
        self.stencil_radius
    }

    /// `(lower, upper)`: the Jacobian is zero below `lower` and above `upper` diagonals in
    /// the natural ordering (periodic wrap-around included).
    pub fn bandwidth(&self) -> (usize, usize) {
        (self.lower_bandwidth, self.upper_bandwidth)
    }

    /// The Jacobian's sparsity pattern.
    pub fn pattern(&self) -> &SparsityPattern {
        &self.pattern
    }

    /// Whether the operator is linear (its Jacobian does not depend on the state).
    pub fn is_linear(&self) -> bool {
        self.linear
    }

    /// Whether `sum_i w_i f_i(u) = 0` for every `u` in exact arithmetic.
    pub fn is_conservative(&self) -> bool {
        self.conservative
    }

    /// Declared consistency order.
    pub fn order(&self) -> SpatialOrder {
        self.order
    }

    /// One line layout per grid axis.
    pub fn lines(&self) -> &[LineLayout] {
        &self.lines
    }

    /// Adds the coefficients of row `row` (in slot order) to `values`.
    #[inline]
    pub(crate) fn scatter(&self, values: &mut [f64], row: usize, coefficients: &[f64]) {
        let slots = &self.slots[row * self.slots_per_row..(row + 1) * self.slots_per_row];
        for (&slot, &c) in slots.iter().zip(coefficients) {
            values[slot] += c;
        }
    }
}

/// A method-of-lines right-hand side with declared structure.
///
/// Every method checks the lengths of its slices against [`OperatorStructure::dimension`]
/// and returns [`CoreError::Dimension`] on a mismatch. None of the three evaluation methods
/// allocates (except for the message of an error).
pub trait MolOperator: Send + Sync {
    /// A short descriptive name.
    fn name(&self) -> &str;

    /// The declared structure.
    fn structure(&self) -> &OperatorStructure;

    /// `out = f(t, u)`.
    fn apply_into(&self, t: f64, u: &[f64], out: &mut [f64]) -> CoreResult<()>;

    /// `out = J(t, u) v` with the analytic Jacobian.
    fn jvp_into(&self, t: f64, u: &[f64], v: &[f64], out: &mut [f64]) -> CoreResult<()>;

    /// The Jacobian at `(t, u)` on [`OperatorStructure::pattern`]: `values` has one entry
    /// per stored position and is overwritten.
    fn jacobian_values_into(&self, t: f64, u: &[f64], values: &mut [f64]) -> CoreResult<()>;

    /// Number of unknowns.
    fn dimension(&self) -> usize {
        self.structure().dimension()
    }
}

pub(crate) fn check_len(expected: usize, actual: usize, what: &str) -> CoreResult<()> {
    if expected == actual {
        Ok(())
    } else {
        Err(CoreError::Dimension(format!(
            "{what}: length {actual}, expected {expected}"
        )))
    }
}

pub(crate) fn invalid(message: &str) -> CoreError {
    CoreError::InvalidInput(message.to_owned())
}

/// Builds an [`OperatorStructure`]: every row has `slots_per_row` slots, slot 0 is the
/// diagonal, an unused slot points at the diagonal and receives zero coefficients.
pub(crate) struct StructureBuilder {
    dimension: usize,
    slots_per_row: usize,
    columns: Vec<usize>,
}

/// The declarations [`StructureBuilder::finish`] attaches.
pub(crate) struct Declarations {
    pub(crate) species: usize,
    pub(crate) weights: Vec<f64>,
    pub(crate) stencil_radius: usize,
    pub(crate) linear: bool,
    pub(crate) conservative: bool,
    pub(crate) order: SpatialOrder,
    pub(crate) lines: Vec<LineLayout>,
}

impl StructureBuilder {
    pub(crate) fn new(dimension: usize, slots_per_row: usize) -> CoreResult<Self> {
        let total = dimension
            .checked_mul(slots_per_row)
            .filter(|&t| t <= isize::MAX as usize / (2 * std::mem::size_of::<usize>()))
            .ok_or_else(|| invalid("operator pattern size overflows"))?;
        let mut columns = vec![0usize; total];
        for (row, chunk) in columns.chunks_exact_mut(slots_per_row).enumerate() {
            chunk.fill(row);
        }
        Ok(Self {
            dimension,
            slots_per_row,
            columns,
        })
    }

    pub(crate) fn set(&mut self, row: usize, slot: usize, col: usize) {
        self.columns[row * self.slots_per_row + slot] = col;
    }

    pub(crate) fn finish(self, declarations: Declarations) -> CoreResult<OperatorStructure> {
        let n = self.dimension;
        let s = self.slots_per_row;
        if declarations.weights.len() != n
            || !declarations
                .weights
                .iter()
                .all(|w| w.is_finite() && *w > 0.0)
        {
            return Err(invalid(
                "quadrature weights must be finite and positive, one per unknown",
            ));
        }
        let mut entries: Vec<(usize, usize)> = self
            .columns
            .iter()
            .enumerate()
            .map(|(k, &col)| (col, k / s))
            .collect();
        entries.sort_unstable();
        entries.dedup();
        let mut col_ptr = vec![0usize; n + 1];
        for &(col, _) in &entries {
            col_ptr[col + 1] += 1;
        }
        for c in 0..n {
            col_ptr[c + 1] += col_ptr[c];
        }
        let row_idx: Vec<usize> = entries.iter().map(|&(_, row)| row).collect();
        let (mut lower, mut upper) = (0usize, 0usize);
        for &(col, row) in &entries {
            lower = lower.max(row.saturating_sub(col));
            upper = upper.max(col.saturating_sub(row));
        }
        drop(entries);
        let mut pattern = SparsityPattern {
            dimension: n,
            col_ptr,
            row_idx,
            diagonal: Vec::new(),
        };
        let diagonal = (0..n)
            .map(|i| pattern.position(i, i))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| invalid("operator pattern lacks a diagonal entry"))?;
        pattern.diagonal = diagonal;
        let slots = self
            .columns
            .iter()
            .enumerate()
            .map(|(k, &col)| pattern.position(k / s, col))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| invalid("operator slot outside its pattern"))?;
        Ok(OperatorStructure {
            dimension: n,
            species: declarations.species,
            weights: declarations.weights,
            stencil_radius: declarations.stencil_radius,
            lower_bandwidth: lower,
            upper_bandwidth: upper,
            pattern,
            slots_per_row: s,
            slots,
            linear: declarations.linear,
            conservative: declarations.conservative,
            order: declarations.order,
            lines: declarations.lines,
        })
    }
}

/// An [`OdeProblem`] whose right-hand side and JVP are the operator's (autonomous, no
/// explicit Jacobian). Use [`crate::stage_clients::sparse_lu::SparseLuStageClient::for_operator`] for a
/// sparse direct stage solver of the same operator.
pub fn ode_problem(op: Arc<dyn MolOperator>) -> CoreResult<OdeProblem> {
    let n = op.dimension();
    let name = op.name().to_owned();
    let rhs_op = op.clone();
    let jvp_op = op;
    OdeProblem::new(
        name,
        n,
        Arc::new(move |t, y, out| rhs_op.apply_into(t, y, out)),
        None,
        None,
        Some(Arc::new(move |t, y, v, out| jvp_op.jvp_into(t, y, v, out))),
        None,
        true,
        None,
        None,
    )
}

/// The weighted self-adjoint and dissipative certificate of a linear operator `L` with
/// weights `H = diag(w)`: `H L` is symmetric and `x^T H L x <= 0` (Gershgorin on the
/// symmetric part), both up to [`WeightedSelfAdjointCertificate::tolerance`] relative
/// rounding.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeightedSelfAdjointCertificate {
    /// Largest `|w_i L_ij - w_j L_ji| / max(|w_i L_ij|, |w_j L_ji|)` over stored pairs.
    pub symmetry_defect: f64,
    /// Largest `(S_ii + sum_(j != i) |S_ij|) / (|S_ii| + sum_(j != i) |S_ij|)` over rows of
    /// `S = (H L + (H L)^T) / 2`; at most the tolerance.
    pub dissipativity_margin: f64,
    /// The relative tolerance of both tests (`64 f64::EPSILON`).
    pub tolerance: f64,
}

/// Why [`certify_weighted_self_adjoint`] refused.
#[derive(Clone, Debug, PartialEq)]
pub enum CertificateRefusal {
    /// The operator is nonlinear; no state-independent certificate exists.
    Nonlinear,
    /// The Jacobian could not be evaluated or holds a non-finite value.
    Evaluation(String),
    /// `w_i L_ij != w_j L_ji` beyond the tolerance (for example upwind or central advection).
    Asymmetric {
        /// Row of the offending entry.
        row: usize,
        /// Column of the offending entry.
        col: usize,
        /// Its relative defect.
        defect: f64,
    },
    /// A Gershgorin row of the symmetric part is not nonpositive.
    NotDissipative {
        /// The row.
        row: usize,
        /// Its margin.
        margin: f64,
    },
}

/// Checks `H L = L^T H` and `x^T H L x <= 0` on the assembled matrix of a linear operator
/// (see [`WeightedSelfAdjointCertificate`]). Allocates; it is a diagnostic, not a step-loop
/// call.
pub fn certify_weighted_self_adjoint(
    op: &dyn MolOperator,
) -> Result<WeightedSelfAdjointCertificate, CertificateRefusal> {
    let structure = op.structure();
    if !structure.is_linear() {
        return Err(CertificateRefusal::Nonlinear);
    }
    let n = structure.dimension();
    let pattern = structure.pattern();
    let w = structure.weights();
    let mut values = vec![0.0; pattern.nnz()];
    let zero = vec![0.0; n];
    op.jacobian_values_into(0.0, &zero, &mut values)
        .map_err(|e| CertificateRefusal::Evaluation(e.to_string()))?;
    if !values.iter().all(|v| v.is_finite()) {
        return Err(CertificateRefusal::Evaluation(
            "non-finite Jacobian value".into(),
        ));
    }
    let tolerance = 64.0 * f64::EPSILON;
    let mut symmetry_defect = 0.0f64;
    let mut diagonal = vec![0.0; n];
    let mut off = vec![0.0; n];
    for col in 0..n {
        for k in pattern.col_ptr()[col]..pattern.col_ptr()[col + 1] {
            let row = pattern.row_idx()[k];
            let a = w[row] * values[k];
            if row == col {
                diagonal[row] = a;
                continue;
            }
            let b = pattern
                .position(col, row)
                .map(|p| w[col] * values[p])
                .unwrap_or(0.0);
            let scale = a.abs().max(b.abs());
            if scale > 0.0 {
                let defect = (a - b).abs() / scale;
                if defect > tolerance {
                    return Err(CertificateRefusal::Asymmetric { row, col, defect });
                }
                symmetry_defect = symmetry_defect.max(defect);
            }
            off[row] += (0.5 * (a + b)).abs();
        }
    }
    let mut dissipativity_margin = f64::NEG_INFINITY;
    for row in 0..n {
        let scale = diagonal[row].abs() + off[row];
        let margin = if scale > 0.0 {
            (diagonal[row] + off[row]) / scale
        } else {
            0.0
        };
        if margin > tolerance {
            return Err(CertificateRefusal::NotDissipative { row, margin });
        }
        dissipativity_margin = dissipativity_margin.max(margin);
    }
    Ok(WeightedSelfAdjointCertificate {
        symmetry_defect,
        dissipativity_margin,
        tolerance,
    })
}
