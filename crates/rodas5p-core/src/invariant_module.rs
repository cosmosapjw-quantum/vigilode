//! An admitted finite invariant-module chart for shifted linear solves.
//!
//! The immutable binary inputs are `B,V,W,H`, with `WV` close to `I` and
//! `BV` close to `VH`. A candidate for `(I-alpha B)x=b` is
//! `x=V (I-alpha H)^-1 Wb`. Multiplicities/Jordan entries are retained in H;
//! the implementation never replaces H by its distinct eigenvalues.
//!
//! Setup encloses both defects and the chart amplification. Every candidate
//! is then certified against the original B by the existing outward residual
//! verifier. B must pass its dissipativity bound, so residual l1 bounds solution
//! error l2 for alpha>=0. Projection or invariance alone never accepts a solve.
//! The module is an opt-in dense structural provider, not automatic reduction
//! of a general nonlinear ODE. Repeated identical shifts share a factorization
//! within one call; one shared RODAS diagonal is counted as one shift.

use serde::Serialize;

use crate::directed::{Interval, add_up, mul_up};
use crate::shared_shift_jet::{
    CertificateStatus, ResidualCertificate, dissipativity_rows, residual_certificate,
};
use crate::{CoreError, CoreResult, DenseMatrix, LuFactorization};

fn invalid(message: &str) -> CoreError {
    CoreError::InvalidInput(format!("invariant module: {message}"))
}

fn product(values: &[usize]) -> CoreResult<usize> {
    values.iter().try_fold(1usize, |a, b| {
        a.checked_mul(*b)
            .ok_or_else(|| invalid("resource overflow"))
    })
}
fn sum(values: &[usize]) -> CoreResult<usize> {
    values.iter().try_fold(0usize, |a, b| {
        a.checked_add(*b)
            .ok_or_else(|| invalid("resource overflow"))
    })
}
fn charge(counter: &mut usize, n: usize) -> CoreResult<()> {
    *counter = counter
        .checked_add(n)
        .ok_or_else(|| invalid("work counter overflow"))?;
    Ok(())
}
fn matrix_norm_inf(a: &DenseMatrix) -> CoreResult<f64> {
    let mut result: f64 = 0.0;
    for i in 0..a.nrows() {
        let mut row = 0.0;
        for j in 0..a.ncols() {
            row = add_up(row, a[(i, j)].abs())?;
        }
        result = result.max(row);
    }
    Ok(result)
}
fn entry_product(a: &DenseMatrix, b: &DenseMatrix, i: usize, j: usize) -> CoreResult<Interval> {
    let mut result = Interval::point(0.0)?;
    for k in 0..a.ncols() {
        result = result.add(Interval::point(a[(i, k)])?.mul(Interval::point(b[(k, j)])?)?)?;
    }
    Ok(result)
}

/// Admission budgets include owned matrix entries, excluding opaque faer LU
/// workspaces. Defect tolerances concern exact binary matrix inputs.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct InvariantModuleConfig {
    pub left_inverse_tolerance: f64,
    pub invariance_tolerance: f64,
    pub max_projector_amplification: f64,
    pub max_stored_scalars: usize,
    pub max_setup_work_units: usize,
}
impl Default for InvariantModuleConfig {
    fn default() -> Self {
        Self {
            left_inverse_tolerance: 1e-12,
            invariance_tolerance: 1e-12,
            max_projector_amplification: 1e6,
            max_stored_scalars: 1_000_000,
            max_setup_work_units: 100_000_000,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct InvariantModuleEvidence {
    pub dimension: usize,
    pub reduced_dimension: usize,
    pub epoch: u64,
    pub left_inverse_defect_inf_upper: f64,
    pub invariance_defect_inf_upper: f64,
    pub lift_norm_inf_upper: f64,
    pub restriction_norm_inf_upper: f64,
    pub projector_amplification_inf_upper: f64,
    pub dissipativity_row_upper: Vec<f64>,
    pub owned_matrix_scalars: usize,
    pub setup_work_units: usize,
}

/// No mutable matrix access and no Deserialize: evidence stays bound to the
/// matrix bytes used to create it. Changing the operator requires a new plan.
#[derive(Clone, Debug)]
pub struct InvariantModulePlan {
    b: DenseMatrix,
    v: DenseMatrix,
    w: DenseMatrix,
    h: DenseMatrix,
    evidence: InvariantModuleEvidence,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct InvariantModuleSolveBudget {
    pub max_targets: usize,
    pub max_distinct_shifts: usize,
    pub max_rhs_columns: usize,
    pub max_output_scalars: usize,
    pub max_work_units: usize,
}
impl Default for InvariantModuleSolveBudget {
    fn default() -> Self {
        Self {
            max_targets: 64,
            max_distinct_shifts: 16,
            max_rhs_columns: 64,
            max_output_scalars: 1_000_000,
            max_work_units: 100_000_000,
        }
    }
}

/// Attempted work survives errors through the caller-owned value.
#[derive(Clone, Debug, Default, Serialize)]
pub struct InvariantModuleWork {
    pub factorizations: usize,
    pub reused_shift_factorizations: usize,
    pub rhs_restriction_products: usize,
    pub reduced_rhs_solves: usize,
    pub lift_products: usize,
    pub original_residual_products: usize,
    /// Zero during evaluation: the immutable plan reuses its setup certificate.
    pub dissipativity_symmetric_entries: usize,
    pub accepted_shift_groups: usize,
    pub refused_shift_groups: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct InvariantModuleCandidate {
    alpha: f64,
    accepted: bool,
    values: Vec<Vec<f64>>,
    error_l2_upper: Vec<f64>,
}
impl InvariantModuleCandidate {
    pub fn alpha(&self) -> f64 {
        self.alpha
    }
    pub fn accepted_values(&self) -> Option<&[Vec<f64>]> {
        self.accepted.then_some(&self.values)
    }
    pub fn error_l2_upper(&self) -> &[f64] {
        &self.error_l2_upper
    }
    /// Diagnostic candidates may be refused; use accepted_values for admission.
    pub fn candidate_values(&self) -> &[Vec<f64>] {
        &self.values
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct InvariantModuleReport {
    pub candidates: Vec<InvariantModuleCandidate>,
    pub distinct_shifts: usize,
    pub planned_work_units: usize,
}

impl InvariantModulePlan {
    pub fn new(
        epoch: u64,
        b: DenseMatrix,
        v: DenseMatrix,
        w: DenseMatrix,
        h: DenseMatrix,
        config: InvariantModuleConfig,
    ) -> CoreResult<Self> {
        let n = b.nrows();
        let m = h.nrows();
        if n == 0
            || m == 0
            || m > n
            || b.ncols() != n
            || h.ncols() != m
            || v.nrows() != n
            || v.ncols() != m
            || w.nrows() != m
            || w.ncols() != n
        {
            return Err(invalid("inconsistent nonempty chart dimensions"));
        }
        if [&b, &v, &w, &h]
            .iter()
            .any(|a| a.as_slice().iter().any(|x| !x.is_finite()))
        {
            return Err(invalid("nonfinite matrix input"));
        }
        if !config.left_inverse_tolerance.is_finite()
            || config.left_inverse_tolerance < 0.0
            || config.left_inverse_tolerance >= 1.0
            || !config.invariance_tolerance.is_finite()
            || config.invariance_tolerance < 0.0
            || !config.max_projector_amplification.is_finite()
            || config.max_projector_amplification < 1.0
        {
            return Err(invalid("invalid admission bounds"));
        }
        let stored = sum(&[product(&[n, n])?, product(&[2, n, m])?, product(&[m, m])?])?;
        let setup = sum(&[
            product(&[m, n, m])?,
            product(&[n, n, m])?,
            product(&[n, m, m])?,
            product(&[n, n])?,
        ])?;
        if stored > config.max_stored_scalars || setup > config.max_setup_work_units {
            return Err(invalid("setup storage/work budget exceeded"));
        }
        let mut left: f64 = 0.0;
        for i in 0..m {
            let mut row = 0.0;
            for j in 0..m {
                let e = entry_product(&w, &v, i, j)?.sub(Interval::point(if i == j {
                    1.0
                } else {
                    0.0
                })?)?;
                row = add_up(row, e.mag())?;
            }
            left = left.max(row);
        }
        let mut invariant: f64 = 0.0;
        for i in 0..n {
            let mut row = 0.0;
            for j in 0..m {
                let e = entry_product(&b, &v, i, j)?.sub(entry_product(&v, &h, i, j)?)?;
                row = add_up(row, e.mag())?;
            }
            invariant = invariant.max(row);
        }
        let vn = matrix_norm_inf(&v)?;
        let wn = matrix_norm_inf(&w)?;
        let amplification = mul_up(vn, wn)?;
        if left > config.left_inverse_tolerance {
            return Err(invalid("left inverse/rank check refused"));
        }
        if invariant > config.invariance_tolerance {
            return Err(invalid("invariance/leakage check refused"));
        }
        if amplification > config.max_projector_amplification {
            return Err(invalid("chart amplification refused"));
        }
        let rows = dissipativity_rows(&b)?;
        let evidence = InvariantModuleEvidence {
            dimension: n,
            reduced_dimension: m,
            epoch,
            left_inverse_defect_inf_upper: left,
            invariance_defect_inf_upper: invariant,
            lift_norm_inf_upper: vn,
            restriction_norm_inf_upper: wn,
            projector_amplification_inf_upper: amplification,
            dissipativity_row_upper: rows,
            owned_matrix_scalars: stored,
            setup_work_units: setup,
        };
        Ok(Self {
            b,
            v,
            w,
            h,
            evidence,
        })
    }

    pub fn evidence(&self) -> &InvariantModuleEvidence {
        &self.evidence
    }

    /// Certify arbitrary candidates using the plan's immutable full operator.
    /// Candidates need not lie in the admitted module. Reusing the bound only
    /// removes the symmetric-part scan; every original residual is recomputed.
    pub fn certify_candidates(
        &self,
        expected_epoch: u64,
        alpha: f64,
        rhs_columns: &[Vec<f64>],
        candidates: &[Vec<f64>],
        absolute_tolerance: f64,
    ) -> CoreResult<ResidualCertificate> {
        if expected_epoch != self.evidence.epoch {
            return Err(invalid("stale operator epoch"));
        }
        let n = self.evidence.dimension;
        if !alpha.is_finite()
            || alpha < 0.0
            || !absolute_tolerance.is_finite()
            || absolute_tolerance <= 0.0
            || rhs_columns.is_empty()
            || candidates.len() != rhs_columns.len()
            || rhs_columns
                .iter()
                .chain(candidates.iter())
                .any(|u| u.len() != n || u.iter().any(|v| !v.is_finite()))
        {
            return Err(invalid(
                "invalid certification shifts, columns or tolerance",
            ));
        }
        residual_certificate(
            &self.b,
            alpha,
            1.0,
            rhs_columns,
            candidates,
            absolute_tolerance,
            &self.evidence.dissipativity_row_upper,
        )
    }

    /// Fresh original-space residuals determine every acceptance. A RHS outside
    /// the module is retained and normally refused rather than silently projected.
    pub fn evaluate(
        &self,
        expected_epoch: u64,
        alphas: &[f64],
        rhs_columns: &[Vec<f64>],
        absolute_tolerance: f64,
        budget: InvariantModuleSolveBudget,
        work: &mut InvariantModuleWork,
    ) -> CoreResult<InvariantModuleReport> {
        if expected_epoch != self.evidence.epoch {
            return Err(invalid("stale operator epoch"));
        }
        let n = self.evidence.dimension;
        let m = self.evidence.reduced_dimension;
        let r = rhs_columns.len();
        if alphas.is_empty()
            || alphas.len() > budget.max_targets
            || r == 0
            || r > budget.max_rhs_columns
            || !absolute_tolerance.is_finite()
            || absolute_tolerance <= 0.0
            || alphas.iter().any(|a| !a.is_finite() || *a < 0.0)
            || rhs_columns
                .iter()
                .any(|b| b.len() != n || b.iter().any(|v| !v.is_finite()))
        {
            return Err(invalid("invalid shifts, RHS or tolerance"));
        }
        let mut distinct = Vec::new();
        for &a in alphas {
            let a = if a == 0.0 { 0.0 } else { a };
            if !distinct.contains(&a) {
                distinct.push(a);
            }
        }
        if distinct.len() > budget.max_distinct_shifts {
            return Err(invalid("distinct shift budget exceeded"));
        }
        if product(&[alphas.len(), r, n])? > budget.max_output_scalars {
            return Err(invalid("output storage budget exceeded"));
        }
        let units = sum(&[
            product(&[distinct.len(), m, m, m])?,
            product(&[r, m, n])?,
            product(&[alphas.len(), r, m, m])?,
            product(&[alphas.len(), r, n, m])?,
            product(&[alphas.len(), r, n, n])?,
        ])?;
        if units > budget.max_work_units {
            return Err(invalid("solve work budget exceeded"));
        }
        charge(&mut work.rhs_restriction_products, product(&[r, m, n])?)?;
        let reduced = rhs_columns
            .iter()
            .map(|b| self.w.matvec(b))
            .collect::<CoreResult<Vec<_>>>()?;
        let mut factors: Vec<(f64, LuFactorization)> = Vec::new();
        let mut candidates = Vec::with_capacity(alphas.len());
        for &alpha in alphas {
            let index = if let Some(i) = factors.iter().position(|(a, _)| *a == alpha) {
                charge(&mut work.reused_shift_factorizations, 1)?;
                i
            } else {
                let mut a = DenseMatrix::identity(m);
                for i in 0..m {
                    for j in 0..m {
                        a[(i, j)] -= alpha * self.h[(i, j)];
                        if !a[(i, j)].is_finite() {
                            return Err(invalid("shift matrix overflow"));
                        }
                    }
                }
                charge(&mut work.factorizations, 1)?;
                factors.push((alpha, LuFactorization::new(&a)?));
                factors.len() - 1
            };
            charge(&mut work.reduced_rhs_solves, r)?;
            let z = factors[index].1.solve_rows(&reduced)?;
            charge(&mut work.lift_products, product(&[r, n, m])?)?;
            let values = z
                .iter()
                .map(|x| self.v.matvec(x))
                .collect::<CoreResult<Vec<_>>>()?;
            charge(&mut work.original_residual_products, product(&[r, n, n])?)?;
            // gamma=1, h=alpha covers alpha=0 without a special target formula.
            let certificate = self.certify_candidates(
                expected_epoch,
                alpha,
                rhs_columns,
                &values,
                absolute_tolerance,
            )?;
            let accepted = certificate.status() == CertificateStatus::Certified;
            if accepted {
                charge(&mut work.accepted_shift_groups, 1)?;
            } else {
                charge(&mut work.refused_shift_groups, 1)?;
            }
            candidates.push(InvariantModuleCandidate {
                alpha,
                accepted,
                values,
                error_l2_upper: certificate.rhs_error_upper().to_vec(),
            });
        }
        Ok(InvariantModuleReport {
            candidates,
            distinct_shifts: distinct.len(),
            planned_work_units: units,
        })
    }
}
