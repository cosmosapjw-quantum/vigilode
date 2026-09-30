//! The stage equations a certificate is about (re-audit R3 of 2026-10-01,
//! HOM-01).
//!
//! The Rodas5P tableau is derived at load time: `Gamma = inv(I / gamma - C)`
//! by a partial-pivot LU, `alpha = A Gamma`, `beta = alpha + Gamma`,
//! `L = beta - gamma I`. `A` and `C` are exactly strictly lower triangular,
//! but the rounded inverse leaves roundoff above the diagonal: 28 diagonal or
//! upper entries of `alpha` (largest 5.6e-16) and 34 of `L` (largest 3.8e-16)
//! are nonzero, so neither `alpha^8` nor `L^8` is exactly zero as reals.
//!
//! The consumers see different stage equations:
//!
//! * the sequential stepper (`sequential.rs`) reads only the strictly lower
//!   rows `alpha[i][..i]` and `Gamma[i][..i]` and uses `gamma` on the
//!   diagonal: its target is strictly lower triangular by construction;
//! * the full-block operator (`block.rs`) sums over every column of `beta`,
//!   `alpha` and `L`, including the leakage;
//! * a certificate that linearizes the coupling needs `alpha_ij + Gamma_ij`
//!   as an exact real, while the stored `L` holds its rounding.
//!
//! [`StageTarget`] names the equations a bound refers to and carries their
//! coefficients as data: row `i` of `alpha_rows` and `coupling_rows` has
//! exactly `i` entries, so the strict lower structure, and with it the
//! nilpotency any Neumann or path-sum argument uses, is part of the target
//! itself ([`StageTarget::strictly_lower_nilpotent`]). No coefficient is
//! zeroed inside a certificate; the ignored upper entries are recorded by
//! [`native_coefficient_leakage`] and bounded by [`block_sequential_allowance`].

use rodas5p_core::{
    CoreError, CoreResult, RODAS5P_COEFFICIENT_SNAPSHOT_SHA256, Rodas5pCoefficients,
    directed::Interval,
};
use serde::{Deserialize, Serialize};

/// The sequential stepper's stage equations: native coefficient bits taken
/// as exact reals, strictly lower `alpha` and `Gamma`, coupling
/// `alpha_ij + Gamma_ij` enclosed exactly.
pub const STAGE_TARGET_SEQUENTIAL: &str =
    "rodas5p/snapshot-211255f6/sequential-strict-lower/alpha+Gamma-exact-reals/v1";
/// The target of the R3 research fixtures: strictly lower native `alpha` and
/// the rounded native `L` taken as exact.
pub const STAGE_TARGET_STRICT_LOWER_PROJECTION: &str =
    "r3/strict-lower-projection/native-alpha-l/v1";

/// Stage equations with their coefficients.
#[derive(Clone, Debug, PartialEq)]
pub struct StageTarget {
    pub id: &'static str,
    pub snapshot_sha256: &'static str,
    pub gamma: f64,
    pub c: Vec<f64>,
    pub gamma_rows: Vec<f64>,
    /// `alpha[i][..i]`.
    pub alpha_rows: Vec<Vec<f64>>,
    /// Coupling `L*[i][..i]` of the linearized residual.
    pub coupling_rows: Vec<Vec<Interval>>,
    pub b: Vec<f64>,
    pub btilde: Vec<f64>,
}

impl StageTarget {
    /// [`STAGE_TARGET_SEQUENTIAL`].
    pub fn sequential(coeffs: &Rodas5pCoefficients) -> CoreResult<Self> {
        Self::build(coeffs, STAGE_TARGET_SEQUENTIAL, |i, j| {
            Interval::exact_sum(coeffs.alpha[(i, j)], coeffs.gamma_matrix[(i, j)])
        })
    }

    /// [`STAGE_TARGET_STRICT_LOWER_PROJECTION`].
    pub fn strict_lower_projection(coeffs: &Rodas5pCoefficients) -> CoreResult<Self> {
        Self::build(coeffs, STAGE_TARGET_STRICT_LOWER_PROJECTION, |i, j| {
            Interval::point(coeffs.l[(i, j)])
        })
    }

    fn build(
        coeffs: &Rodas5pCoefficients,
        id: &'static str,
        coupling: impl Fn(usize, usize) -> CoreResult<Interval>,
    ) -> CoreResult<Self> {
        let s = coeffs.stages();
        let alpha_rows = (0..s)
            .map(|i| (0..i).map(|j| coeffs.alpha[(i, j)]).collect())
            .collect();
        let coupling_rows = (0..s)
            .map(|i| {
                (0..i)
                    .map(|j| coupling(i, j))
                    .collect::<CoreResult<Vec<_>>>()
            })
            .collect::<CoreResult<Vec<_>>>()?;
        Ok(Self {
            id,
            snapshot_sha256: RODAS5P_COEFFICIENT_SNAPSHOT_SHA256,
            gamma: coeffs.gamma,
            c: coeffs.c.clone(),
            gamma_rows: coeffs.gamma_rows.clone(),
            alpha_rows,
            coupling_rows,
            b: coeffs.b.clone(),
            btilde: coeffs.btilde.clone(),
        })
    }

    pub fn stages(&self) -> usize {
        self.alpha_rows.len()
    }

    /// The target's own coupling pattern `P` (`P_ij` = row `i` has a
    /// nonzero coupling or `alpha` entry in column `j`) satisfies
    /// `P^s = 0` exactly, checked by boolean matrix products; this is the
    /// structural premise of every finite path sum over the stages. It is a
    /// property of the target's data, not of the native full matrices
    /// (whose `alpha^8` is not zero).
    pub fn strictly_lower_nilpotent(&self) -> bool {
        let s = self.stages();
        let mut pattern = vec![vec![false; s]; s];
        for i in 0..s {
            if self.alpha_rows[i].len() != i || self.coupling_rows[i].len() != i {
                return false;
            }
            for j in 0..i {
                pattern[i][j] =
                    self.alpha_rows[i][j] != 0.0 || self.coupling_rows[i][j].mag() != 0.0;
            }
        }
        let mut power = pattern.clone();
        for _ in 1..s {
            let mut next = vec![vec![false; s]; s];
            for i in 0..s {
                for j in 0..s {
                    next[i][j] = (0..s).any(|k| power[i][k] && pattern[k][j]);
                }
            }
            power = next;
        }
        power.iter().flatten().all(|entry| !entry)
    }

    /// Hex bits of every coefficient, keyed by name, for a receipt.
    pub fn coefficient_bits(&self) -> StageTargetBits {
        let hex = |value: f64| format!("{:016x}", value.to_bits());
        StageTargetBits {
            id: self.id.into(),
            snapshot_sha256: self.snapshot_sha256.into(),
            gamma: hex(self.gamma),
            c: self.c.iter().copied().map(hex).collect(),
            gamma_rows: self.gamma_rows.iter().copied().map(hex).collect(),
            alpha_rows: self
                .alpha_rows
                .iter()
                .map(|row| row.iter().copied().map(hex).collect())
                .collect(),
            coupling_rows: self
                .coupling_rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|interval| [hex(interval.lo), hex(interval.hi)])
                        .collect()
                })
                .collect(),
            b: self.b.iter().copied().map(hex).collect(),
            btilde: self.btilde.iter().copied().map(hex).collect(),
        }
    }
}

/// [`StageTarget`] coefficient bits (hex of the binary64 encodings).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StageTargetBits {
    pub id: String,
    pub snapshot_sha256: String,
    pub gamma: String,
    pub c: Vec<String>,
    pub gamma_rows: Vec<String>,
    pub alpha_rows: Vec<Vec<String>>,
    pub coupling_rows: Vec<Vec<[String; 2]>>,
    pub b: Vec<String>,
    pub btilde: Vec<String>,
}

/// Diagonal or upper entries of the native matrices that a strictly lower
/// target leaves out.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoefficientLeakage {
    pub alpha_nonzero: usize,
    pub alpha_max: f64,
    pub l_nonzero: usize,
    pub l_max: f64,
}

pub fn native_coefficient_leakage(coeffs: &Rodas5pCoefficients) -> CoefficientLeakage {
    let s = coeffs.stages();
    let census = |matrix: &rodas5p_core::DenseMatrix| {
        let mut count = 0;
        let mut largest = 0.0_f64;
        for i in 0..s {
            for j in i..s {
                let value = matrix[(i, j)].abs();
                if value != 0.0 {
                    count += 1;
                    largest = largest.max(value);
                }
            }
        }
        (count, largest)
    };
    let (alpha_nonzero, alpha_max) = census(&coeffs.alpha);
    let (l_nonzero, l_max) = census(&coeffs.l);
    CoefficientLeakage {
        alpha_nonzero,
        alpha_max,
        l_nonzero,
        l_max,
    }
}

/// Upper bounds `(D, U)` with `D_ij >= |beta_ij - alpha_ij - [j<i] Gamma_ij
/// - [j=i] gamma|` and `U_ij = |alpha_ij|` for `j >= i` (0 below).
///
/// For a stage vector `K` and an `f` with Jacobian bound `|J|` and Lipschitz
/// bound `Lip` near the stages, the full-block residual differs from the
/// sequential one by at most
/// `h |J| sum_j D_ij |K_j| + h Lip sum_j U_ij |K_j|` (plus the rounding of
/// evaluating either): the quantified data perturbation between the two
/// consumers (HOM-01).
pub fn block_sequential_allowance(
    coeffs: &Rodas5pCoefficients,
) -> CoreResult<(Vec<Vec<f64>>, Vec<Vec<f64>>)> {
    let s = coeffs.stages();
    let mut discrepancy = vec![vec![0.0; s]; s];
    let mut upper_alpha = vec![vec![0.0; s]; s];
    for i in 0..s {
        for j in 0..s {
            let subtract = if j < i {
                Interval::exact_sum(coeffs.alpha[(i, j)], coeffs.gamma_matrix[(i, j)])?
            } else if j == i {
                Interval::exact_sum(coeffs.alpha[(i, j)], coeffs.gamma)?
            } else {
                Interval::point(coeffs.alpha[(i, j)])?
            };
            let difference = Interval::point(coeffs.beta[(i, j)])?.sub(subtract)?;
            discrepancy[i][j] = difference.mag();
            if j >= i {
                upper_alpha[i][j] = coeffs.alpha[(i, j)].abs();
            }
        }
    }
    if !discrepancy.iter().flatten().all(|value| value.is_finite()) {
        return Err(CoreError::NonFinite(
            "stage target allowance is not finite".into(),
        ));
    }
    Ok((discrepancy, upper_alpha))
}
