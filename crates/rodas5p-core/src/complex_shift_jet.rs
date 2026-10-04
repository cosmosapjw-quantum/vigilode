//! Opt-in research action for complex shifted resolvents (research node PP16).
//!
//! For one fixed real `J` with a verified row bound of `(J+J^T)/2 <= 0`,
//! `h >= 0` and `Re gamma > 0`: `Re <x, x/gamma - h J x> >= Re(1/gamma)
//! ||x||^2`, so `||(I - gamma*h*J)^-1||_2 <= |gamma| / Re gamma`. A candidate
//! `u` for `(I - gamma*h*J) x = b` is then accepted only through
//! `||x - u||_2 <= gain_up * ||r||_2,up` with `r = b - (I - gamma*h*J) u`
//! recomputed outward from the current exact binary inputs and `gain_up` an
//! upward enclosure of `|gamma| / Re gamma`. The real-positive gain 1 is never
//! assumed for complex `gamma`. Only `H = I` is supported.
//!
//! Candidates may come from the real normalized jet of `shared_shift_jet`
//! (center `gamma0 > 0` real) evaluated at complex `z = (gamma-gamma0)/gamma0`,
//! or from anywhere else; generation is approximate and never an authority.
//! Partial-fraction outputs `y = c0 b + sum_i w_i u_i` are enclosed in
//! interval arithmetic; every listed pole is evaluated, conjugate pairs are
//! never merged. No solver selection, timing or speedup claim. Certificate
//! and report types deliberately do not implement `Deserialize`.

use serde::Serialize;

use crate::directed::{Interval, add_up, div_up, mul_up, sqrt_up, sub_up, sum_up};
use crate::shared_shift_jet::{CertificateStatus, dissipativity_rows};
use crate::{CoreError, CoreResult, DenseMatrix, LuFactorization};

/// Explicit caller budget, independent of numerical acceptance.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct ComplexShiftJetConfig {
    /// Polynomial degree; at most 128 in this bounded implementation.
    pub degree: usize,
    /// Positive absolute Euclidean error tolerance for each RHS column.
    pub absolute_tolerance: f64,
    /// Upper bound on explicit scalar slots; excludes opaque LU allocations.
    pub max_stored_scalars: usize,
    /// Structural budget units (see [`ComplexJetWork::planned_work_units`]).
    pub max_work_units: usize,
}

impl Default for ComplexShiftJetConfig {
    fn default() -> Self {
        Self {
            degree: 24,
            absolute_tolerance: 1e-10,
            max_stored_scalars: 1_000_000,
            max_work_units: 100_000_000,
        }
    }
}

/// Evidence for one complex candidate column against one current target.
#[derive(Clone, Debug, Serialize)]
pub struct ComplexShiftCertificate {
    status: CertificateStatus,
    gamma_re: f64,
    gamma_im: f64,
    error_upper: f64,
    residual_l2_upper: f64,
    gain_upper: f64,
    absolute_tolerance: f64,
    dissipativity_row_upper: Vec<f64>,
}

impl ComplexShiftCertificate {
    pub fn status(&self) -> CertificateStatus {
        self.status
    }

    pub fn gamma(&self) -> (f64, f64) {
        (self.gamma_re, self.gamma_im)
    }

    /// Upper bound on the Euclidean error `||x - u||_2`.
    pub fn error_upper(&self) -> f64 {
        self.error_upper
    }

    pub fn residual_l2_upper(&self) -> f64 {
        self.residual_l2_upper
    }

    /// Upward enclosure of `|gamma| / Re gamma`.
    pub fn gain_upper(&self) -> f64 {
        self.gain_upper
    }

    pub fn absolute_tolerance(&self) -> f64 {
        self.absolute_tolerance
    }

    pub fn dissipativity_row_upper(&self) -> &[f64] {
        &self.dissipativity_row_upper
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ComplexShiftCandidate {
    gamma_re: f64,
    gamma_im: f64,
    z_modulus_upper: f64,
    columns_re: Vec<Vec<f64>>,
    columns_im: Vec<Vec<f64>>,
    certificates: Vec<ComplexShiftCertificate>,
}

impl ComplexShiftCandidate {
    pub fn gamma(&self) -> (f64, f64) {
        (self.gamma_re, self.gamma_im)
    }

    /// Outward upper bound on `|(gamma - gamma0) / gamma0|`.
    pub fn z_modulus_upper(&self) -> f64 {
        self.z_modulus_upper
    }

    pub fn columns_re(&self) -> &[Vec<f64>] {
        &self.columns_re
    }

    pub fn columns_im(&self) -> &[Vec<f64>] {
        &self.columns_im
    }

    /// One certificate per RHS column.
    pub fn certificates(&self) -> &[ComplexShiftCertificate] {
        &self.certificates
    }

    /// Certified only when every column is.
    pub fn status(&self) -> CertificateStatus {
        if self
            .certificates
            .iter()
            .all(|c| c.status == CertificateStatus::Certified)
        {
            CertificateStatus::Certified
        } else {
            CertificateStatus::Rejected
        }
    }
}

/// Executed high-level work counts, not timings or library-internal FLOPs.
/// Complex RHS columns are solved as their real and imaginary parts.
#[derive(Clone, Debug, Serialize)]
pub struct ComplexJetWork {
    pub factorizations: usize,
    pub solve_batches: usize,
    /// Real columns submitted to LU solves (two per complex RHS), incl. W0.
    pub rhs_solves: usize,
    pub recurrence_depth: usize,
    /// Complex RHS columns supplied by the caller.
    pub supplied_rhs_columns: usize,
    pub target_shifts: usize,
    /// Real fused multiply-adds of the complex Horner evaluation.
    pub evaluation_multiply_add_pairs: usize,
    /// Interval products in directed complex residuals (4 per entry).
    pub residual_matrix_entry_products: usize,
    pub dissipativity_symmetric_entries: usize,
    pub retained_jet_scalars: usize,
    /// Conservative bound for explicitly owned f64 slots (result and
    /// certificate data, center matrix, scratch, LU conversion input).
    /// Excludes faer-owned LU workspace and caller-owned inputs.
    pub explicit_storage_upper_scalars_excluding_lu: usize,
    /// Prospective structural budget: n^3 + n^2*2r*(degree+1) + evaluation
    /// pairs + residual products + n^2. Not measured FLOPs/cost.
    pub planned_work_units: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct ComplexShiftJetReport {
    candidates: Vec<ComplexShiftCandidate>,
    work: ComplexJetWork,
}

impl ComplexShiftJetReport {
    pub fn candidates(&self) -> &[ComplexShiftCandidate] {
        &self.candidates
    }

    pub fn work(&self) -> &ComplexJetWork {
        &self.work
    }
}

/// One term `w (I - gamma*h*J)^-1 b` of a partial fraction, with candidate `u`.
#[derive(Clone, Copy, Debug)]
pub struct PartialFractionTerm<'a> {
    pub gamma_re: f64,
    pub gamma_im: f64,
    pub weight_re: f64,
    pub weight_im: f64,
    pub candidate_re: &'a [f64],
    pub candidate_im: &'a [f64],
}

#[derive(Clone, Debug, Serialize)]
pub struct PartialFractionWork {
    pub terms: usize,
    pub residual_matrix_entry_products: usize,
    /// Interval products forming `c0 b + sum_i w_i u_i` (4 per entry).
    pub combination_interval_products: usize,
    pub dissipativity_symmetric_entries: usize,
}

/// Evidence for the reported output `y` (the interval midpoints).
#[derive(Clone, Debug, Serialize)]
pub struct PartialFractionCertificate {
    status: CertificateStatus,
    output_re: Vec<f64>,
    output_im: Vec<f64>,
    error_upper: f64,
    output_radius_l2_upper: f64,
    weighted_term_error_upper: f64,
    weight_modulus_upper: Vec<f64>,
    term_certificates: Vec<ComplexShiftCertificate>,
    absolute_tolerance: f64,
    work: PartialFractionWork,
}

impl PartialFractionCertificate {
    pub fn status(&self) -> CertificateStatus {
        self.status
    }

    pub fn output_re(&self) -> &[f64] {
        &self.output_re
    }

    pub fn output_im(&self) -> &[f64] {
        &self.output_im
    }

    /// Upper bound on `||y - (c0 b + sum_i w_i x_i)||_2`, `x_i` exact.
    pub fn error_upper(&self) -> f64 {
        self.error_upper
    }

    pub fn output_radius_l2_upper(&self) -> f64 {
        self.output_radius_l2_upper
    }

    pub fn weighted_term_error_upper(&self) -> f64 {
        self.weighted_term_error_upper
    }

    pub fn weight_modulus_upper(&self) -> &[f64] {
        &self.weight_modulus_upper
    }

    pub fn term_certificates(&self) -> &[ComplexShiftCertificate] {
        &self.term_certificates
    }

    pub fn absolute_tolerance(&self) -> f64 {
        self.absolute_tolerance
    }

    pub fn work(&self) -> &PartialFractionWork {
        &self.work
    }
}

fn invalid(message: &str) -> CoreError {
    CoreError::InvalidInput(format!("complex shift jet: {message}"))
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

fn validate_operator(j: &DenseMatrix, h: f64, tolerance: f64) -> CoreResult<()> {
    if j.nrows() == 0 || j.nrows() != j.ncols() {
        return Err(invalid("J must be nonempty and square"));
    }
    if !j.as_slice().iter().all(|v| v.is_finite()) {
        return Err(invalid("J contains NaN/Inf"));
    }
    if !h.is_finite() || h < 0.0 {
        return Err(invalid("h must be finite nonnegative"));
    }
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(invalid("absolute tolerance must be finite positive"));
    }
    Ok(())
}

// The inputs are exact binary numbers, so the outward lower endpoint of
// Re gamma is gamma_re itself; -0.0 and 0.0 are refused alike.
fn validate_shift(gamma_re: f64, gamma_im: f64) -> CoreResult<()> {
    if !gamma_re.is_finite() || !gamma_im.is_finite() {
        return Err(invalid("gamma must be finite"));
    }
    if gamma_re <= 0.0 {
        return Err(invalid("outward Re gamma must be positive"));
    }
    Ok(())
}

fn validate_vector(v: &[f64], n: usize, what: &str) -> CoreResult<()> {
    if v.len() != n || !v.iter().all(|x| x.is_finite()) {
        return Err(invalid(&format!("{what} must be finite and match J")));
    }
    Ok(())
}

fn modulus_up(re: f64, im: f64) -> CoreResult<f64> {
    if im == 0.0 {
        return Ok(re.abs());
    }
    if re == 0.0 {
        return Ok(im.abs());
    }
    sqrt_up(add_up(mul_up(re, re)?, mul_up(im, im)?)?)
}

/// Assumes validated inputs and verified dissipativity rows.
#[allow(clippy::too_many_arguments)]
fn complex_residual_certificate(
    j: &DenseMatrix,
    h: f64,
    gamma_re: f64,
    gamma_im: f64,
    b_re: &[f64],
    b_im: &[f64],
    u_re: &[f64],
    u_im: &[f64],
    tolerance: f64,
    dissipativity_row_upper: &[f64],
) -> CoreResult<ComplexShiftCertificate> {
    let n = j.nrows();
    let h = Interval::point(h)?;
    let shift_re = Interval::point(gamma_re)?.mul(h)?;
    let shift_im = Interval::point(gamma_im)?.mul(h)?;
    // (I - gamma h J) = P - i Q with P = I - Re(gamma) h J, Q = Im(gamma) h J.
    let mut squares = Vec::with_capacity(n);
    for i in 0..n {
        let mut res_re = Interval::point(b_re[i])?;
        let mut res_im = Interval::point(b_im[i])?;
        for k in 0..n {
            let jik = Interval::point(j[(i, k)])?;
            let diagonal = if i == k { 1.0 } else { 0.0 };
            let p = Interval::point(diagonal)?.sub(shift_re.mul(jik)?)?;
            let q = shift_im.mul(jik)?;
            let ur = Interval::point(u_re[k])?;
            let ui = Interval::point(u_im[k])?;
            res_re = res_re.sub(p.mul(ur)?.add(q.mul(ui)?)?)?;
            res_im = res_im.sub(p.mul(ui)?.sub(q.mul(ur)?)?)?;
        }
        squares.push(add_up(
            mul_up(res_re.mag(), res_re.mag())?,
            mul_up(res_im.mag(), res_im.mag())?,
        )?);
    }
    let residual_l2_upper = sqrt_up(sum_up(squares)?)?;
    let gain_upper = div_up(modulus_up(gamma_re, gamma_im)?, gamma_re)?;
    let error_upper = mul_up(gain_upper, residual_l2_upper)?;
    let status = if error_upper <= tolerance {
        CertificateStatus::Certified
    } else {
        CertificateStatus::Rejected
    };
    Ok(ComplexShiftCertificate {
        status,
        gamma_re,
        gamma_im,
        error_upper,
        residual_l2_upper,
        gain_upper,
        absolute_tolerance: tolerance,
        dissipativity_row_upper: dissipativity_row_upper.to_vec(),
    })
}

/// Independently certify one complex candidate `u` for
/// `(I - gamma*h*J) x = b`. Rejected means a valid bound above tolerance;
/// malformed, nondissipative, `Re gamma <= 0` or overflowing inputs are `Err`.
#[allow(clippy::too_many_arguments)]
pub fn certify_complex_shift_candidate(
    j: &DenseMatrix,
    h: f64,
    gamma_re: f64,
    gamma_im: f64,
    b_re: &[f64],
    b_im: &[f64],
    u_re: &[f64],
    u_im: &[f64],
    absolute_tolerance: f64,
) -> CoreResult<ComplexShiftCertificate> {
    validate_operator(j, h, absolute_tolerance)?;
    validate_shift(gamma_re, gamma_im)?;
    let n = j.nrows();
    for (v, what) in [
        (b_re, "Re b"),
        (b_im, "Im b"),
        (u_re, "Re u"),
        (u_im, "Im u"),
    ] {
        validate_vector(v, n, what)?;
    }
    let rows = dissipativity_rows(j)?;
    complex_residual_certificate(
        j,
        h,
        gamma_re,
        gamma_im,
        b_re,
        b_im,
        u_re,
        u_im,
        absolute_tolerance,
        &rows,
    )
}

/// Build the real normalized jet at the real center `gamma0` and evaluate it
/// at complex targets by complex Horner. A target is refused unless the
/// outward `|z|`, `z = (gamma - gamma0)/gamma0`, is strictly below one. The
/// jet is real and linear, so it acts on Re B and Im B separately.
pub fn complex_shift_jet(
    j: &DenseMatrix,
    h: f64,
    gamma0: f64,
    rhs_re: &[Vec<f64>],
    rhs_im: &[Vec<f64>],
    gammas: &[(f64, f64)],
    config: ComplexShiftJetConfig,
) -> CoreResult<ComplexShiftJetReport> {
    validate_operator(j, h, config.absolute_tolerance)?;
    if !gamma0.is_finite() || gamma0 <= 0.0 {
        return Err(invalid("center gamma0 must be finite positive"));
    }
    let n = j.nrows();
    if rhs_re.is_empty() || rhs_re.len() != rhs_im.len() {
        return Err(invalid("RHS real and imaginary column sets must match"));
    }
    for (re, im) in rhs_re.iter().zip(rhs_im) {
        validate_vector(re, n, "Re B columns")?;
        validate_vector(im, n, "Im B columns")?;
    }
    if config.degree > 128 || gammas.is_empty() {
        return Err(invalid("degree must be at most 128 and targets nonempty"));
    }
    let r = rhs_re.len();
    let s = gammas.len();
    let levels = config.degree + 1;
    let nn = checked_product(&[n, n])?;
    let real_columns = checked_product(&[2, r])?;
    let nr2 = checked_product(&[n, real_columns])?;
    let jet_scalars = checked_product(&[nr2, levels])?;
    let evaluation = checked_product(&[4, s, r, n, config.degree])?;
    let residual_terms = checked_product(&[4, s, r, nn])?;
    let storage = checked_sum(&[
        jet_scalars,
        checked_product(&[s, nr2])?,
        checked_product(&[s, r, n])?,
        checked_product(&[8, s, r])?,
        checked_product(&[2, nn])?,
        checked_product(&[4, nr2])?,
        n,
        checked_product(&[4, s])?,
    ])?;
    let work_units = checked_sum(&[
        checked_product(&[n, nn])?,
        checked_product(&[nn, real_columns, levels])?,
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
    let g0 = Interval::point(gamma0)?;
    let mut coordinates = Vec::with_capacity(s);
    for &(gamma_re, gamma_im) in gammas {
        validate_shift(gamma_re, gamma_im)?;
        let z_re = Interval::point(gamma_re)?.sub(g0)?.div(g0)?;
        let z_im = Interval::point(gamma_im)?.div(g0)?;
        let modulus = sqrt_up(add_up(
            mul_up(z_re.mag(), z_re.mag())?,
            mul_up(z_im.mag(), z_im.mag())?,
        )?)?;
        if modulus >= 1.0 {
            return Err(invalid(
                "outward normalized complex shift modulus must be less than one",
            ));
        }
        coordinates.push(((gamma_re - gamma0) / gamma0, gamma_im / gamma0, modulus));
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
    let stacked: Vec<Vec<f64>> = rhs_re.iter().chain(rhs_im).cloned().collect();
    let mut jet = Vec::with_capacity(levels);
    jet.push(lu.solve_rows(&stacked)?);
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
    for (&(gamma_re, gamma_im), &(z_re, z_im, modulus)) in gammas.iter().zip(&coordinates) {
        let mut acc = jet[config.degree].clone();
        for level in (0..config.degree).rev() {
            let coefficient = &jet[level];
            for c in 0..r {
                let (re_part, im_part) = acc.split_at_mut(r);
                for i in 0..n {
                    let ar = re_part[c][i];
                    let ai = im_part[c][i];
                    // With z_im = 0 this is the real jet's fused step (up to signed zeros).
                    let next_re = z_re.mul_add(ar, (-z_im).mul_add(ai, coefficient[c][i]));
                    let next_im = z_re.mul_add(ai, z_im.mul_add(ar, coefficient[r + c][i]));
                    if !next_re.is_finite() || !next_im.is_finite() {
                        return Err(invalid("candidate evaluation overflow"));
                    }
                    re_part[c][i] = next_re;
                    im_part[c][i] = next_im;
                }
            }
        }
        let columns_im = acc.split_off(r);
        let columns_re = acc;
        let mut certificates = Vec::with_capacity(r);
        for c in 0..r {
            certificates.push(complex_residual_certificate(
                j,
                h,
                gamma_re,
                gamma_im,
                &rhs_re[c],
                &rhs_im[c],
                &columns_re[c],
                &columns_im[c],
                config.absolute_tolerance,
                &rows,
            )?);
        }
        candidates.push(ComplexShiftCandidate {
            gamma_re,
            gamma_im,
            z_modulus_upper: modulus,
            columns_re,
            columns_im,
            certificates,
        });
    }
    Ok(ComplexShiftJetReport {
        candidates,
        work: ComplexJetWork {
            factorizations: 1,
            solve_batches: levels,
            rhs_solves: checked_product(&[real_columns, levels])?,
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

fn midpoint_and_radius(value: Interval) -> CoreResult<(f64, f64)> {
    let mid = (0.5 * value.lo + 0.5 * value.hi).clamp(value.lo, value.hi);
    let radius = sub_up(mid, value.lo)?.max(sub_up(value.hi, mid)?);
    Ok((mid, radius))
}

/// Certify `y = c0 b + sum_i w_i (I - gamma_i*h*J)^-1 b` evaluated with the
/// given candidates. Each term is re-certified from the current exact inputs;
/// the bound is `sum_i |w_i|_up err_i + ||radius(y)||_2,up` for the reported
/// midpoint `y`. Every listed term is evaluated; no conjugate merging.
#[allow(clippy::too_many_arguments)]
pub fn certify_partial_fraction(
    j: &DenseMatrix,
    h: f64,
    c0_re: f64,
    c0_im: f64,
    terms: &[PartialFractionTerm<'_>],
    b_re: &[f64],
    b_im: &[f64],
    absolute_tolerance: f64,
) -> CoreResult<PartialFractionCertificate> {
    validate_operator(j, h, absolute_tolerance)?;
    let n = j.nrows();
    if !c0_re.is_finite() || !c0_im.is_finite() {
        return Err(invalid("c0 must be finite"));
    }
    if terms.is_empty() {
        return Err(invalid("partial fraction needs at least one term"));
    }
    validate_vector(b_re, n, "Re b")?;
    validate_vector(b_im, n, "Im b")?;
    for term in terms {
        validate_shift(term.gamma_re, term.gamma_im)?;
        if !term.weight_re.is_finite() || !term.weight_im.is_finite() {
            return Err(invalid("weights must be finite"));
        }
        validate_vector(term.candidate_re, n, "Re u")?;
        validate_vector(term.candidate_im, n, "Im u")?;
    }
    let nn = checked_product(&[n, n])?;
    let residual_terms = checked_product(&[4, terms.len(), nn])?;
    let combination = checked_product(&[4, checked_sum(&[terms.len(), 1])?, n])?;
    let rows = dissipativity_rows(j)?;
    let mut term_certificates = Vec::with_capacity(terms.len());
    let mut weight_modulus_upper = Vec::with_capacity(terms.len());
    let mut weighted = Vec::with_capacity(terms.len());
    for term in terms {
        let certificate = complex_residual_certificate(
            j,
            h,
            term.gamma_re,
            term.gamma_im,
            b_re,
            b_im,
            term.candidate_re,
            term.candidate_im,
            absolute_tolerance,
            &rows,
        )?;
        let modulus = modulus_up(term.weight_re, term.weight_im)?;
        weighted.push(mul_up(modulus, certificate.error_upper)?);
        weight_modulus_upper.push(modulus);
        term_certificates.push(certificate);
    }
    let weighted_term_error_upper = sum_up(weighted)?;
    let c0r = Interval::point(c0_re)?;
    let c0i = Interval::point(c0_im)?;
    let mut output_re = Vec::with_capacity(n);
    let mut output_im = Vec::with_capacity(n);
    let mut squares = Vec::with_capacity(n);
    for i in 0..n {
        let br = Interval::point(b_re[i])?;
        let bi = Interval::point(b_im[i])?;
        let mut y_re = c0r.mul(br)?.sub(c0i.mul(bi)?)?;
        let mut y_im = c0r.mul(bi)?.add(c0i.mul(br)?)?;
        for term in terms {
            let wr = Interval::point(term.weight_re)?;
            let wi = Interval::point(term.weight_im)?;
            let ur = Interval::point(term.candidate_re[i])?;
            let ui = Interval::point(term.candidate_im[i])?;
            y_re = y_re.add(wr.mul(ur)?.sub(wi.mul(ui)?)?)?;
            y_im = y_im.add(wr.mul(ui)?.add(wi.mul(ur)?)?)?;
        }
        let (mid_re, rad_re) = midpoint_and_radius(y_re)?;
        let (mid_im, rad_im) = midpoint_and_radius(y_im)?;
        output_re.push(mid_re);
        output_im.push(mid_im);
        squares.push(add_up(mul_up(rad_re, rad_re)?, mul_up(rad_im, rad_im)?)?);
    }
    let output_radius_l2_upper = sqrt_up(sum_up(squares)?)?;
    let error_upper = add_up(weighted_term_error_upper, output_radius_l2_upper)?;
    let status = if error_upper <= absolute_tolerance {
        CertificateStatus::Certified
    } else {
        CertificateStatus::Rejected
    };
    Ok(PartialFractionCertificate {
        status,
        output_re,
        output_im,
        error_upper,
        output_radius_l2_upper,
        weighted_term_error_upper,
        weight_modulus_upper,
        term_certificates,
        absolute_tolerance,
        work: PartialFractionWork {
            terms: terms.len(),
            residual_matrix_entry_products: residual_terms,
            combination_interval_products: combination,
            dissipativity_symmetric_entries: nn - n,
        },
    })
}
