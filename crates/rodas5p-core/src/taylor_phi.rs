//! A separately versioned scaled-Taylor block-phi backend (re-audit R4 of
//! 2026-10-01, POLY-DEV-06). Research only: no integrator calls this
//! module, and its results are never [`TaylorPhiStatus::Certified`].
//!
//! # Method
//!
//! `F = sum_{k=0}^{4} phi_k(hA) w_k` is the top block of `exp(M) v` for the
//! augmented matrix (Al-Mohy and Higham 2011, Theorem 2.1, with `t = 1`
//! and `hA` in place of `A`)
//!
//! `M = [[hA, W], [0, J]]`, `W = [w_4, w_3, w_2, w_1]`, `J` the 4x4 upper
//! shift, `v = [w_0; e_4]`.
//!
//! `exp(M) v` is `T_m(X)^s v` with `X = M / s`, `s = 2^j` (an exact
//! division) and `T_m` the degree-`m` Taylor polynomial.
//!
//! # Domain witness and bound
//!
//! The witness is the explicit dense matrix, so `nu >= ||X||_1` is computed
//! with upward rounding from its entries; nothing (symmetry, spectrum) is
//! inferred from eigenvalues. For **any** matrix, in exact arithmetic,
//! `||e^X - T_m(X)||_1 <= r_m = nu^(m+1) / (m+1)! / (1 - nu/(m+2))` and
//! `||T_m(X)||_1, ||e^X||_1 <= e^nu`, so the telescoped power gives
//!
//! `||exp(M) v - T_m(X)^s v||_2 <= ||.||_1 <= s e^((s-1) nu) r_m ||v||_1`.
//!
//! This is a truncation bound **in exact arithmetic for the binary64
//! matrix `M`** (`hA` is rounded once when `M` is formed). The rounding of
//! the `s m` products is not bounded, so the status is
//! [`TaylorPhiStatus::EstimateOnly`]; the bound is reported for what it is.
//! `||M||_1` above [`TAYLOR_NORM_LIMIT`] (where `e^nu` growth makes the
//! forward bound useless) or a budget the degree cap cannot meet is a
//! typed capability rejection, [`TAYLOR_DOMAIN_UNSUPPORTED`].

use serde::{Deserialize, Serialize};

use crate::directed::{add_up, div_up, mul_up, sub_down};
use crate::polynomial_action::{JOINT_PHI_TERMS, exp_up};
use crate::{CoreError, CoreResult, DenseMatrix, sha256_hex};

pub const TAYLOR_PHI_SCHEMA: &str = "vigilode-scaled-taylor-phi-v1";
pub const TAYLOR_PHI_BACKEND: &str = "scaled-taylor-augmented";
pub const TAYLOR_DOMAIN_UNSUPPORTED: &str = "TAYLOR_DOMAIN_UNSUPPORTED";
pub const TAYLOR_ROUNDING_NOT_BOUNDED: &str = "TAYLOR_ROUNDING_NOT_BOUNDED";
/// Largest `||M||_1` the forward bound is attempted for.
pub const TAYLOR_NORM_LIMIT: f64 = 64.0;
pub const TAYLOR_MAX_DEGREE: usize = 60;

/// The operator epoch the backend acts on: the content digest of the
/// binary64 entries and the domain witness derived from them. A changed
/// entry is a different epoch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OperatorEpoch {
    pub fingerprint: String,
    pub dimension: usize,
    pub witness: DomainWitness,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum DomainWitness {
    /// `||A||_1 <= one_norm_upper`, from the explicit entries; no
    /// structure assumed.
    DenseOneNorm { one_norm_upper: f64 },
}

impl OperatorEpoch {
    pub fn dense(matrix: &DenseMatrix) -> CoreResult<Self> {
        let n = matrix.nrows();
        if matrix.ncols() != n || matrix.as_slice().iter().any(|x| !x.is_finite()) {
            return Err(CoreError::InvalidInput(format!(
                "{TAYLOR_DOMAIN_UNSUPPORTED}: the operator must be square and finite"
            )));
        }
        let mut norm = 0.0_f64;
        for j in 0..n {
            let mut column = 0.0;
            for i in 0..n {
                column = add_up(column, matrix[(i, j)].abs())?;
            }
            norm = norm.max(column);
        }
        let bytes = matrix
            .as_slice()
            .iter()
            .flat_map(|x| x.to_bits().to_le_bytes())
            .collect::<Vec<_>>();
        Ok(Self {
            fingerprint: sha256_hex(&bytes),
            dimension: n,
            witness: DomainWitness::DenseOneNorm {
                one_norm_upper: norm,
            },
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "status")]
pub enum TaylorPhiStatus {
    /// Reserved: no rounding bound is derived for this backend yet.
    Certified { bound: f64 },
    EstimateOnly {
        reason: String,
        /// Exact-arithmetic truncation bound (see the module docs).
        truncation_bound: f64,
    },
}

/// Work of one action: `scaling_steps * degree` products of the
/// `(n + 4)`-dimensional augmented matrix with one vector.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaylorPhiWork {
    pub augmented_dimension: usize,
    pub scaling_steps: u64,
    pub degree: usize,
    pub augmented_vector_products: u64,
    /// Scalar multiply-adds of those products (dense).
    pub multiply_adds: u64,
    /// Entries read to build the domain witness (the 1-norm).
    pub witness_entries: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TaylorPhiReport {
    pub schema: String,
    pub backend: String,
    pub epoch: OperatorEpoch,
    pub h: f64,
    pub fused: Vec<f64>,
    pub status: TaylorPhiStatus,
    /// `||M||_1` upper bound, and `nu = ||M||_1 / s`.
    pub augmented_one_norm: f64,
    pub nu: f64,
    /// `||A A^T - A^T A||_F / ||A||_F^2` (0 for normal `A`, `None` for
    /// `A = 0`): the departure from normality, reported, never used to
    /// infer a class.
    pub nonnormality: Option<f64>,
    pub work: TaylorPhiWork,
}

/// `F = sum_k phi_k(hA) w_k` by the scaled Taylor method; `budget` bounds
/// the exact-arithmetic truncation (absolute, 2-norm).
pub fn taylor_phi_action(
    matrix: &DenseMatrix,
    h: f64,
    vectors: &[Vec<f64>; JOINT_PHI_TERMS],
    budget: f64,
) -> CoreResult<TaylorPhiReport> {
    let epoch = OperatorEpoch::dense(matrix)?;
    let n = epoch.dimension;
    let unsupported =
        |what: String| CoreError::InvalidInput(format!("{TAYLOR_DOMAIN_UNSUPPORTED}: {what}"));
    if !(h.is_finite() && h >= 0.0) {
        return Err(unsupported(format!("h = {h:e} must be finite and >= 0")));
    }
    if !(budget.is_finite() && budget > 0.0) {
        return Err(unsupported("the truncation budget must be positive".into()));
    }
    if vectors.iter().any(|w| w.len() != n) || !vectors.iter().flatten().all(|x| x.is_finite()) {
        return Err(unsupported(
            "every w_k must be finite with the operator dimension".into(),
        ));
    }
    let p = JOINT_PHI_TERMS - 1;
    let size = n + p;
    // M, row-major.
    let mut m = vec![0.0; size * size];
    for i in 0..n {
        for j in 0..n {
            m[i * size + j] = h * matrix[(i, j)];
        }
        for c in 0..p {
            m[i * size + n + c] = vectors[p - c][i];
        }
    }
    for c in 0..p - 1 {
        m[(n + c) * size + n + c + 1] = 1.0;
    }
    let mut norm = 0.0_f64;
    for j in 0..size {
        let mut column = 0.0;
        for i in 0..size {
            column = add_up(column, m[i * size + j].abs())?;
        }
        norm = norm.max(column);
    }
    if !norm.is_finite() || norm > TAYLOR_NORM_LIMIT {
        return Err(unsupported(format!(
            "||M||_1 = {norm:e} above {TAYLOR_NORM_LIMIT}"
        )));
    }
    let mut v = vec![0.0; size];
    v[..n].copy_from_slice(&vectors[0]);
    v[size - 1] = 1.0;
    let v_norm = v.iter().try_fold(0.0, |acc, x| add_up(acc, x.abs()))?;
    // s = 2^j with nu = ||M|| / s <= 1.
    let mut steps = 1_u64;
    let mut nu = norm;
    while nu > 1.0 {
        steps *= 2;
        nu /= 2.0;
    }
    // Smallest degree whose bound meets the budget.
    let growth = exp_up(mul_up((steps - 1) as f64, nu)?)?;
    let prefactor = mul_up(mul_up(steps as f64, growth)?, v_norm)?;
    let mut chosen = None;
    let mut term = 1.0_f64; // nu^(m+1)/(m+1)! for m = -1
    for degree in 0..=TAYLOR_MAX_DEGREE {
        term = div_up(mul_up(term, nu)?, (degree + 1) as f64)?;
        let tail_ratio = sub_down(1.0, div_up(nu, (degree + 2) as f64)?)?;
        if tail_ratio <= 0.0 {
            continue;
        }
        let remainder = div_up(term, tail_ratio)?;
        let bound = mul_up(prefactor, remainder)?;
        if bound <= budget {
            chosen = Some((degree, bound));
            break;
        }
    }
    let Some((degree, truncation_bound)) = chosen else {
        return Err(unsupported(format!(
            "no degree <= {TAYLOR_MAX_DEGREE} meets the budget {budget:e}"
        )));
    };
    // T_m(X)^s v with X = M / s (exact: s is a power of two).
    let scale = 1.0 / steps as f64;
    let x = m.iter().map(|value| value * scale).collect::<Vec<_>>();
    let mut products = 0_u64;
    for _ in 0..steps {
        let mut sum = v.clone();
        let mut term = v.clone();
        for k in 1..=degree {
            let mut next = vec![0.0; size];
            for (i, out) in next.iter_mut().enumerate() {
                let row = &x[i * size..(i + 1) * size];
                *out = row.iter().zip(&term).map(|(a, b)| a * b).sum::<f64>() / k as f64;
            }
            products += 1;
            for (s, t) in sum.iter_mut().zip(&next) {
                *s += t;
            }
            term = next;
        }
        v = sum;
    }
    let fused = v[..n].to_vec();
    if !fused.iter().all(|x| x.is_finite()) {
        return Err(CoreError::NonFinite(
            "scaled Taylor phi action produced NaN/Inf".into(),
        ));
    }
    Ok(TaylorPhiReport {
        schema: TAYLOR_PHI_SCHEMA.into(),
        backend: TAYLOR_PHI_BACKEND.into(),
        h,
        fused,
        status: TaylorPhiStatus::EstimateOnly {
            reason: format!(
                "{TAYLOR_ROUNDING_NOT_BOUNDED}: {} products of the augmented matrix and the rounding of h A are not bounded",
                products
            ),
            truncation_bound,
        },
        augmented_one_norm: norm,
        nu,
        nonnormality: nonnormality(matrix),
        work: TaylorPhiWork {
            augmented_dimension: size,
            scaling_steps: steps,
            degree,
            augmented_vector_products: products,
            multiply_adds: products * (size * size) as u64,
            witness_entries: (n * n) as u64,
        },
        epoch,
    })
}

fn nonnormality(matrix: &DenseMatrix) -> Option<f64> {
    let n = matrix.nrows();
    let frobenius_sq = matrix.as_slice().iter().map(|x| x * x).sum::<f64>();
    if frobenius_sq == 0.0 {
        return None;
    }
    let mut commutator_sq = 0.0;
    for i in 0..n {
        for j in 0..n {
            let mut value = 0.0;
            for k in 0..n {
                value += matrix[(i, k)] * matrix[(j, k)] - matrix[(k, i)] * matrix[(k, j)];
            }
            commutator_sq += value * value;
        }
    }
    Some(commutator_sq.sqrt() / frobenius_sq)
}
