//! Opt-in research action for a family of positive real shifted resolvents.
//!
//! For one fixed, dissipative real `J`, this evaluates `(I - gamma*h*J)^-1 B`
//! using a shared center. With `R0 = (I - gamma0*h*J)^-1`, the normalized jet is
//! `W0 = R0 B`, `W(k+1) = R0 Wk - Wk`, evaluated at
//! `z = (gamma - gamma0)/gamma0`. No inverse powers of the center are stored.
//! Every supplied RHS column is retained; independence/rank is not inferred.
//!
//! Candidate generation is approximate. Its only acceptance authority is an
//! outward residual recomputed from the *current exact binary inputs*:
//! `r = B - (I - gamma*h*J) U`. A verified row bound of `(J+J^T)/2 <= 0`
//! implies `||(I-gamma*h*J)^-1||_2 <= 1`. Thus `||error||_2 <= ||r||_1`.
//! Rounding in the LU, jet, normalized coordinate and evaluation is covered by
//! this residual gate. Unsupported inputs and interval overflow return errors.
//!
//! This module does not select a solver, reuse operators across calls, certify
//! nonlinear/time integration errors, execute shifts in parallel or claim a
//! speedup. Its certificate/report types deliberately do not implement
//! `Deserialize`; certificates cannot be loaded as trusted authority.

use serde::Serialize;

use crate::directed::{Interval, add_up};
use crate::{CoreError, CoreResult, DenseMatrix, LuFactorization};

/// Explicit caller budget, independent of numerical acceptance.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct SharedShiftJetConfig {
    /// Polynomial degree; at most 128 in this bounded implementation.
    pub degree: usize,
    /// Positive absolute Euclidean error tolerance for each RHS column.
    pub absolute_tolerance: f64,
    /// Upper bound on explicit scalar slots; excludes opaque LU allocations.
    pub max_stored_scalars: usize,
    /// Structural budget units (see [`JetWork::planned_work_units`]).
    pub max_work_units: usize,
}

impl Default for SharedShiftJetConfig {
    fn default() -> Self {
        Self {
            degree: 24,
            absolute_tolerance: 1e-10,
            max_stored_scalars: 1_000_000,
            max_work_units: 100_000_000,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum CertificateStatus {
    Certified,
    Rejected,
}

/// Immutable evidence for the candidate passed to the certificate function.
/// The scalar upper bounds enclose Euclidean error, not relative error.
#[derive(Clone, Debug, Serialize)]
pub struct ResidualCertificate {
    status: CertificateStatus,
    rhs_error_upper: Vec<f64>,
    absolute_tolerance: f64,
    dissipativity_row_upper: Vec<f64>,
}

impl ResidualCertificate {
    pub fn status(&self) -> CertificateStatus {
        self.status
    }

    pub fn rhs_error_upper(&self) -> &[f64] {
        &self.rhs_error_upper
    }

    pub fn absolute_tolerance(&self) -> f64 {
        self.absolute_tolerance
    }

    pub fn dissipativity_row_upper(&self) -> &[f64] {
        &self.dissipativity_row_upper
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ShiftCandidate {
    gamma: f64,
    rhs_columns: Vec<Vec<f64>>,
    certificate: ResidualCertificate,
}

impl ShiftCandidate {
    pub fn gamma(&self) -> f64 {
        self.gamma
    }

    pub fn rhs_columns(&self) -> &[Vec<f64>] {
        &self.rhs_columns
    }

    pub fn certificate(&self) -> &ResidualCertificate {
        &self.certificate
    }
}

/// Executed high-level work counts, not timings or library-internal FLOPs.
#[derive(Clone, Debug, Serialize)]
pub struct JetWork {
    pub factorizations: usize,
    pub solve_batches: usize,
    /// Number of RHS columns actually submitted to LU solves, including W0.
    pub rhs_solves: usize,
    pub recurrence_depth: usize,
    pub supplied_rhs_columns: usize,
    pub target_shifts: usize,
    pub evaluation_multiply_add_pairs: usize,
    /// Number of A_ij*U_j terms in directed residual evaluations.
    pub residual_matrix_entry_products: usize,
    pub dissipativity_symmetric_entries: usize,
    pub retained_jet_scalars: usize,
    /// Conservative bound for explicitly owned f64 scalar slots in this
    /// function, including result/certificate data, a center matrix, scratch,
    /// and LU conversion input. Excludes faer-owned LU/solver workspace,
    /// allocator capacity, vector metadata, and caller-owned input storage.
    pub explicit_storage_upper_scalars_excluding_lu: usize,
    /// Prospective structural budget: n^3 + n^2*r*(degree+1) + evaluated
    /// multiply/add pairs + residual terms + n^2. Not measured FLOPs/cost.
    pub planned_work_units: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct SharedShiftJetReport {
    candidates: Vec<ShiftCandidate>,
    work: JetWork,
}

impl SharedShiftJetReport {
    pub fn candidates(&self) -> &[ShiftCandidate] {
        &self.candidates
    }

    pub fn work(&self) -> &JetWork {
        &self.work
    }
}

fn invalid(message: &str) -> CoreError {
    CoreError::InvalidInput(format!("shared shift jet: {message}"))
}

fn checked_product(values: &[usize]) -> CoreResult<usize> {
    values.iter().try_fold(1_usize, |a, b| {
        a.checked_mul(*b)
            .ok_or_else(|| invalid("resource size overflow"))
    })
}

fn checked_sum(values: &[usize]) -> CoreResult<usize> {
    values.iter().try_fold(0_usize, |a, b| {
        a.checked_add(*b)
            .ok_or_else(|| invalid("resource size overflow"))
    })
}

fn validate_current_inputs(
    j: &DenseMatrix,
    h: f64,
    gamma: f64,
    rhs_columns: &[Vec<f64>],
    tolerance: f64,
) -> CoreResult<()> {
    if j.nrows() == 0 || j.nrows() != j.ncols() {
        return Err(invalid("J must be nonempty and square"));
    }
    if !j.as_slice().iter().all(|v| v.is_finite()) {
        return Err(invalid("J contains NaN/Inf"));
    }
    if !h.is_finite() || h < 0.0 || !gamma.is_finite() || gamma <= 0.0 {
        return Err(invalid(
            "h must be finite nonnegative and gamma finite positive",
        ));
    }
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(invalid("absolute tolerance must be finite positive"));
    }
    if rhs_columns.is_empty()
        || rhs_columns
            .iter()
            .any(|b| b.len() != j.nrows() || !b.iter().all(|v| v.is_finite()))
    {
        return Err(invalid("RHS columns must be nonempty, finite and match J"));
    }
    Ok(())
}

fn dissipativity_rows(j: &DenseMatrix) -> CoreResult<Vec<f64>> {
    let mut rows = Vec::with_capacity(j.nrows());
    for i in 0..j.nrows() {
        let mut upper = j[(i, i)];
        for k in 0..j.ncols() {
            if k != i {
                let symmetric = Interval::exact_sum(j[(i, k)], j[(k, i)])?.scale(0.5)?;
                upper = add_up(upper, symmetric.mag())?;
            }
        }
        if upper > 0.0 {
            return Err(invalid(
                "symmetric-part row bound does not prove dissipativity",
            ));
        }
        rows.push(upper);
    }
    Ok(rows)
}

fn residual_certificate(
    j: &DenseMatrix,
    h: f64,
    gamma: f64,
    rhs_columns: &[Vec<f64>],
    candidates: &[Vec<f64>],
    tolerance: f64,
    dissipativity_row_upper: &[f64],
) -> CoreResult<ResidualCertificate> {
    let shift = Interval::point(gamma)?.mul(Interval::point(h)?)?;
    let mut rhs_error_upper = Vec::with_capacity(rhs_columns.len());
    for (b, u) in rhs_columns.iter().zip(candidates) {
        let mut residual_l1 = 0.0;
        for i in 0..j.nrows() {
            let mut residual = Interval::point(b[i])?;
            for k in 0..j.ncols() {
                let diagonal = if i == k { 1.0 } else { 0.0 };
                let entry =
                    Interval::point(diagonal)?.sub(shift.mul(Interval::point(j[(i, k)])?)?)?;
                residual = residual.sub(entry.mul(Interval::point(u[k])?)?)?;
            }
            residual_l1 = add_up(residual_l1, residual.mag())?;
        }
        rhs_error_upper.push(residual_l1);
    }
    let status = if rhs_error_upper.iter().all(|upper| *upper <= tolerance) {
        CertificateStatus::Certified
    } else {
        CertificateStatus::Rejected
    };
    Ok(ResidualCertificate {
        status,
        rhs_error_upper,
        absolute_tolerance: tolerance,
        dissipativity_row_upper: dissipativity_row_upper.to_vec(),
    })
}

/// Independently certify arbitrary candidate columns against a current target.
/// A rejected certificate is a valid residual enclosure that missed tolerance;
/// malformed/unsupported/overflowing inputs return `Err` instead.
pub fn certify_shift_candidate(
    j: &DenseMatrix,
    h: f64,
    gamma: f64,
    rhs_columns: &[Vec<f64>],
    candidates: &[Vec<f64>],
    absolute_tolerance: f64,
) -> CoreResult<ResidualCertificate> {
    validate_current_inputs(j, h, gamma, rhs_columns, absolute_tolerance)?;
    if candidates.len() != rhs_columns.len()
        || candidates
            .iter()
            .any(|u| u.len() != j.nrows() || !u.iter().all(|v| v.is_finite()))
    {
        return Err(invalid(
            "candidate columns must be finite and match all RHS columns",
        ));
    }
    let rows = dissipativity_rows(j)?;
    residual_certificate(
        j,
        h,
        gamma,
        rhs_columns,
        candidates,
        absolute_tolerance,
        &rows,
    )
}

/// Build and evaluate one shared jet while borrowing a fixed current operator.
/// Radius is checked outward: the exact binary-input normalized displacement
/// must be strictly less than one. No cross-call operator cache is accepted.
pub fn shared_shift_jet(
    j: &DenseMatrix,
    h: f64,
    gamma0: f64,
    rhs_columns: &[Vec<f64>],
    gammas: &[f64],
    config: SharedShiftJetConfig,
) -> CoreResult<SharedShiftJetReport> {
    validate_current_inputs(j, h, gamma0, rhs_columns, config.absolute_tolerance)?;
    if config.degree > 128 || gammas.is_empty() {
        return Err(invalid("degree must be at most 128 and targets nonempty"));
    }
    let n = j.nrows();
    let r = rhs_columns.len();
    let s = gammas.len();
    let levels = config.degree + 1;
    let nn = checked_product(&[n, n])?;
    let nr = checked_product(&[n, r])?;
    let jet_scalars = checked_product(&[nr, levels])?;
    let evaluation = checked_product(&[s, nr, config.degree])?;
    let residual_terms = checked_product(&[s, r, nn])?;
    let storage = checked_sum(&[
        jet_scalars,
        checked_product(&[s, nr])?,
        checked_product(&[s, r])?,
        checked_product(&[s, n])?,
        checked_product(&[2, nn])?,
        checked_product(&[4, nr])?,
        n,
        checked_product(&[3, s])?,
    ])?;
    let work_units = checked_sum(&[
        checked_product(&[n, nn])?,
        checked_product(&[nn, r, levels])?,
        evaluation,
        residual_terms,
        nn,
    ])?;
    if storage > config.max_stored_scalars || work_units > config.max_work_units {
        return Err(invalid(
            "explicit storage or structural work budget exceeded",
        ));
    }
    let rows = dissipativity_rows(j)?;
    let mut coordinates = Vec::with_capacity(s);
    for &gamma in gammas {
        if !gamma.is_finite() || gamma <= 0.0 {
            return Err(invalid("target shifts must be finite positive"));
        }
        let radius = Interval::point(gamma)?
            .sub(Interval::point(gamma0)?)?
            .div(Interval::point(gamma0)?)?;
        if radius.mag() >= 1.0 {
            return Err(invalid(
                "outward normalized shift radius must be less than one",
            ));
        }
        coordinates.push((gamma - gamma0) / gamma0);
    }
    let center_scale = gamma0 * h;
    if !center_scale.is_finite() {
        return Err(invalid("center scale overflow"));
    }
    let mut center = DenseMatrix::identity(n);
    for i in 0..n {
        for k in 0..n {
            center[(i, k)] -= center_scale * j[(i, k)];
        }
    }
    if !center.as_slice().iter().all(|v| v.is_finite()) {
        return Err(invalid("center matrix overflow"));
    }
    let lu = LuFactorization::new(&center)?;
    let mut jet = Vec::with_capacity(levels);
    jet.push(lu.solve_rows(rhs_columns)?);
    for _ in 0..config.degree {
        let previous = &jet[jet.len() - 1];
        let mut next = lu.solve_rows(previous)?;
        for (next_column, previous_column) in next.iter_mut().zip(previous) {
            for (value, old) in next_column.iter_mut().zip(previous_column) {
                *value -= old;
                if !value.is_finite() {
                    return Err(invalid("jet recurrence overflow"));
                }
            }
        }
        jet.push(next);
    }
    let mut candidates = Vec::with_capacity(s);
    for (&gamma, &z) in gammas.iter().zip(&coordinates) {
        let mut columns = jet[config.degree].clone();
        for level in (0..config.degree).rev() {
            for (u, coefficient) in columns.iter_mut().zip(&jet[level]) {
                for (value, w) in u.iter_mut().zip(coefficient) {
                    *value = z.mul_add(*value, *w);
                    if !value.is_finite() {
                        return Err(invalid("candidate evaluation overflow"));
                    }
                }
            }
        }
        let certificate = residual_certificate(
            j,
            h,
            gamma,
            rhs_columns,
            &columns,
            config.absolute_tolerance,
            &rows,
        )?;
        candidates.push(ShiftCandidate {
            gamma,
            rhs_columns: columns,
            certificate,
        });
    }
    Ok(SharedShiftJetReport {
        candidates,
        work: JetWork {
            factorizations: 1,
            solve_batches: levels,
            rhs_solves: checked_product(&[r, levels])?,
            recurrence_depth: config.degree,
            supplied_rhs_columns: r,
            target_shifts: s,
            evaluation_multiply_add_pairs: evaluation,
            residual_matrix_entry_products: residual_terms,
            dissipativity_symmetric_entries: nn - n,
            retained_jet_scalars: jet_scalars,
            explicit_storage_upper_scalars_excluding_lu: storage,
            planned_work_units: work_units,
        },
    })
}
