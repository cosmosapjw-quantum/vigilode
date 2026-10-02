//! The raw transformed stage target `U = Gamma K` against the native K
//! target (research node `research/thread_transfer_mf_target_20261002`,
//! thread-transfer DAG node P0-MF-TARGET). Research only.
//!
//! With the snapshot's raw coefficients `A`, `C` (strictly lower), `gamma`
//! and `b_code`, a constant mass matrix `M` and the step's `J`,
//!
//! ```text
//! (M - h gamma J) U_i = h gamma f(t + c_i h, y + sum_{j<i} A_ij U_j)
//!                       + gamma M sum_{j<i} C_ij U_j + h^2 gamma gamma_i f_t,
//! ```
//!
//! and for the exact `Gamma = (I/gamma - C)^-1` the residuals satisfy
//! `r_U = gamma r_K`. The native K target ([`crate::StageTarget`]) uses the
//! binary64 faer inverse: strictly lower native `Gamma` and `alpha`, the
//! stated `gamma` on the diagonal. With `S` that lower `Gamma` plus
//! `gamma I` and `alpha0` the strictly lower native `alpha`, for `U = S K`
//!
//! ```text
//! r_U(SK) - gamma r_K(K) = ([(I - gamma C) S - gamma I] (x) M) K
//!                          - h gamma {F(y + A S K) - F(y + alpha0 K)},
//! ```
//!
//! so the targets differ by `D_Gamma = (I - gamma C) S - gamma I` and
//! `D_alpha = A S - alpha0`, and the projections by `b_code^T S - b^T`
//! (output), `e_s^T S - btilde^T` (embedded) and `H_raw S - D_dense` (dense
//! output). [`raw_stage_allowance`] encloses all of them outward; nothing is
//! zeroed and no bit identity between the two targets is assumed.
//!
//! Residual budgets: an absolute or WRMS budget `tau_K` of the K equation is
//! `|gamma| tau_K` for the U equation ([`raw_absolute_residual_budget`]); a
//! relative criterion refers to the right-hand side, which differs between
//! the forms, so it is recomputed from the U right-hand side
//! ([`raw_relative_residual_budget`]), never transported.

use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, Rodas5pCoefficients,
    directed::{Interval, add_up, mul_down, mul_up},
    sha256_hex,
};
use serde::{Deserialize, Serialize};

/// The raw transformed stage equations above, on the pinned snapshot.
pub const RAW_STAGE_TARGET_ID: &str =
    "rodas5p/snapshot-211255f6/raw-transformed-U/A-C-b_code/vs-sequential-S/v1";

type IntervalMatrix = Vec<Vec<Interval>>;

/// Enclosures of the discrepancies between the raw U target and the native
/// K target.
#[derive(Clone, Debug, PartialEq)]
pub struct RawStageAllowance {
    pub target_id: &'static str,
    pub coefficient_sha256: String,
    /// `(I - gamma C) S - gamma I`, `s x s`.
    pub d_gamma: IntervalMatrix,
    /// `A S - alpha0`, `s x s` (strictly lower).
    pub d_alpha: IntervalMatrix,
    /// `b_code^T S - b^T`.
    pub d_output: Vec<Interval>,
    /// `e_s^T S - btilde^T` (the embedded error is the last U stage).
    pub d_embedded: Vec<Interval>,
    /// `H_raw S - D_dense`.
    pub d_dense: IntervalMatrix,
}

fn magnitude(values: &[Vec<Interval>]) -> f64 {
    values
        .iter()
        .flatten()
        .map(Interval::mag)
        .fold(0.0, f64::max)
}

impl RawStageAllowance {
    pub fn d_gamma_max(&self) -> f64 {
        magnitude(&self.d_gamma)
    }
    pub fn d_alpha_max(&self) -> f64 {
        magnitude(&self.d_alpha)
    }
    pub fn d_output_max(&self) -> f64 {
        self.d_output.iter().map(Interval::mag).fold(0.0, f64::max)
    }
    pub fn d_embedded_max(&self) -> f64 {
        self.d_embedded
            .iter()
            .map(Interval::mag)
            .fold(0.0, f64::max)
    }
    pub fn d_dense_max(&self) -> f64 {
        magnitude(&self.d_dense)
    }
}

/// `S`: the strictly lower native `Gamma` with the stated `gamma` on the
/// diagonal, the matrix of the sequential target.
pub fn sequential_gamma(coeffs: &Rodas5pCoefficients) -> Vec<Vec<f64>> {
    let s = coeffs.stages();
    (0..s)
        .map(|i| {
            (0..s)
                .map(|j| match j.cmp(&i) {
                    std::cmp::Ordering::Less => coeffs.gamma_matrix[(i, j)],
                    std::cmp::Ordering::Equal => coeffs.gamma,
                    std::cmp::Ordering::Greater => 0.0,
                })
                .collect()
        })
        .collect()
}

fn hex_matrix(m: &DenseMatrix) -> String {
    m.as_slice()
        .iter()
        .map(|value| format!("{:016x}", value.to_bits()))
        .collect::<Vec<_>>()
        .join(",")
}

fn hex_vector(v: &[f64]) -> String {
    v.iter()
        .map(|value| format!("{:016x}", value.to_bits()))
        .collect::<Vec<_>>()
        .join(",")
}

/// SHA-256 of every coefficient bit the raw/native contract uses.
pub fn raw_coefficient_digest(coeffs: &Rodas5pCoefficients) -> String {
    let fields = [
        ("a", hex_matrix(&coeffs.a)),
        ("c_matrix", hex_matrix(&coeffs.c_matrix)),
        ("gamma", hex_vector(&[coeffs.gamma])),
        ("b_code", hex_vector(&coeffs.b_code)),
        ("gamma_matrix", hex_matrix(&coeffs.gamma_matrix)),
        ("alpha", hex_matrix(&coeffs.alpha)),
        ("b", hex_vector(&coeffs.b)),
        ("btilde", hex_vector(&coeffs.btilde)),
        ("dense_h", hex_matrix(&coeffs.dense_h)),
        ("dense_d", hex_matrix(&coeffs.dense_d)),
    ];
    let mut canonical = String::from("vigilode-raw-stage-coefficients-v1");
    for (name, value) in fields {
        canonical.push_str(&format!("|{name}:{}:{value}", value.len()));
    }
    sha256_hex(canonical.as_bytes())
}

fn point(value: f64) -> CoreResult<Interval> {
    Interval::point(value)
}

/// Outward enclosures of `D_Gamma`, `D_alpha` and the output, embedded and
/// dense discrepancies, from the coefficient bits taken as exact reals.
#[allow(clippy::needless_range_loop)] // index form mirrors the matrix formula
pub fn raw_stage_allowance(coeffs: &Rodas5pCoefficients) -> CoreResult<RawStageAllowance> {
    let s = coeffs.stages();
    for i in 0..s {
        for j in i..s {
            if coeffs.a[(i, j)] != 0.0 || coeffs.c_matrix[(i, j)] != 0.0 {
                return Err(CoreError::Coefficients(
                    "TARGET_SEMANTICS_UNRESOLVED: raw A and C must be strictly lower".into(),
                ));
            }
        }
    }
    if coeffs.dense_h.ncols() != s || coeffs.dense_d.ncols() != s {
        return Err(CoreError::Coefficients(
            "TARGET_SEMANTICS_UNRESOLVED: dense coefficients must have one column per stage".into(),
        ));
    }
    let sm = sequential_gamma(coeffs);
    let gamma = point(coeffs.gamma)?;
    let mut d_gamma = vec![vec![point(0.0)?; s]; s];
    let mut d_alpha = vec![vec![point(0.0)?; s]; s];
    for i in 0..s {
        for j in 0..s {
            // ((I - gamma C) S)_ij = S_ij - gamma sum_k C_ik S_kj.
            let mut cs = point(0.0)?;
            let mut a_s = point(0.0)?;
            for k in 0..s {
                cs = cs.add(point(coeffs.c_matrix[(i, k)])?.mul(point(sm[k][j])?)?)?;
                a_s = a_s.add(point(coeffs.a[(i, k)])?.mul(point(sm[k][j])?)?)?;
            }
            let diagonal = if i == j { gamma } else { point(0.0)? };
            d_gamma[i][j] = point(sm[i][j])?.sub(gamma.mul(cs)?)?.sub(diagonal)?;
            let alpha0 = if j < i { coeffs.alpha[(i, j)] } else { 0.0 };
            d_alpha[i][j] = a_s.sub(point(alpha0)?)?;
        }
    }
    let mut d_output = Vec::with_capacity(s);
    let mut d_embedded = Vec::with_capacity(s);
    for j in 0..s {
        let mut total = point(0.0)?;
        for (i, row) in sm.iter().enumerate() {
            total = total.add(point(coeffs.b_code[i])?.mul(point(row[j])?)?)?;
        }
        d_output.push(total.sub(point(coeffs.b[j])?)?);
        d_embedded.push(point(sm[s - 1][j])?.sub(point(coeffs.btilde[j])?)?);
    }
    let mut d_dense = Vec::with_capacity(coeffs.dense_h.nrows());
    for r in 0..coeffs.dense_h.nrows() {
        let mut row = Vec::with_capacity(s);
        for j in 0..s {
            let mut total = point(0.0)?;
            for (k, s_row) in sm.iter().enumerate() {
                total = total.add(point(coeffs.dense_h[(r, k)])?.mul(point(s_row[j])?)?)?;
            }
            row.push(total.sub(point(coeffs.dense_d[(r, j)])?)?);
        }
        d_dense.push(row);
    }
    Ok(RawStageAllowance {
        target_id: RAW_STAGE_TARGET_ID,
        coefficient_sha256: raw_coefficient_digest(coeffs),
        d_gamma,
        d_alpha,
        d_output,
        d_embedded,
        d_dense,
    })
}

/// What a U-form stage solve was set up for. Any difference from the
/// current inputs invalidates it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawStageReceipt {
    pub target_id: String,
    pub coefficient_sha256: String,
    pub h_bits: u64,
    pub gamma_bits: u64,
    /// Caller-supplied identity of the operator (for a JVP: problem, `t` and
    /// state bits; for a matrix: its digest).
    pub operator_identity: String,
    /// Digest of the residual scale (tolerances and weights).
    pub scale_sha256: String,
}

impl RawStageReceipt {
    pub fn new(
        coeffs: &Rodas5pCoefficients,
        h: f64,
        operator_identity: &str,
        scale_sha256: &str,
    ) -> CoreResult<Self> {
        if !(h.is_finite() && h != 0.0) {
            return Err(CoreError::InvalidInput(
                "RAW_STAGE_RECEIPT: h must be finite and nonzero".into(),
            ));
        }
        Ok(Self {
            target_id: RAW_STAGE_TARGET_ID.into(),
            coefficient_sha256: raw_coefficient_digest(coeffs),
            h_bits: h.to_bits(),
            gamma_bits: coeffs.gamma.to_bits(),
            operator_identity: operator_identity.into(),
            scale_sha256: scale_sha256.into(),
        })
    }

    /// Recomputes the receipt from the current inputs and requires equality.
    pub fn validate(
        &self,
        coeffs: &Rodas5pCoefficients,
        h: f64,
        operator_identity: &str,
        scale_sha256: &str,
    ) -> CoreResult<()> {
        let current = Self::new(coeffs, h, operator_identity, scale_sha256)?;
        if &current != self {
            return Err(CoreError::InvalidInput(
                "RAW_STAGE_RECEIPT_STALE: coefficients, h, operator or scale changed".into(),
            ));
        }
        Ok(())
    }
}

/// Digest of a residual scale: `atol`, `rtol` and the weights' bits.
pub fn residual_scale_digest(atol: f64, rtol: f64, weights: &[f64]) -> String {
    let mut text = format!(
        "vigilode-residual-scale-v1|{:016x}|{:016x}|",
        atol.to_bits(),
        rtol.to_bits()
    );
    text.push_str(&hex_vector(weights));
    sha256_hex(text.as_bytes())
}

/// The U-equation budget `|gamma| tau_K` of an absolute or WRMS K-residual
/// budget, rounded down (a smaller budget is the stricter requirement).
pub fn raw_absolute_residual_budget(tau_k: f64, gamma: f64) -> CoreResult<f64> {
    if !(tau_k.is_finite() && tau_k > 0.0) {
        return Err(CoreError::InvalidInput(format!(
            "RAW_STAGE_BUDGET: the K budget {tau_k:e} must be finite and positive"
        )));
    }
    if !(gamma.is_finite() && gamma != 0.0) {
        return Err(CoreError::InvalidInput(
            "RAW_STAGE_BUDGET: gamma must be finite and nonzero".into(),
        ));
    }
    mul_down(gamma.abs(), tau_k)
}

/// A relative criterion `eta ||rhs_U||`, from the U right-hand side itself.
pub fn raw_relative_residual_budget(eta: f64, rhs_u_norm: f64) -> CoreResult<f64> {
    if !(eta.is_finite() && eta > 0.0 && rhs_u_norm.is_finite() && rhs_u_norm >= 0.0) {
        return Err(CoreError::InvalidInput(
            "RAW_STAGE_BUDGET: eta and the right-hand-side norm must be finite, eta > 0".into(),
        ));
    }
    mul_down(eta, rhs_u_norm)
}

/// Per stage and component, an upper bound on
/// `|r_U(SK) - gamma r_K(K)|` for `M = I`:
/// `sum_j |D_Gamma|_ij |K_ja| + h |gamma| lip max_b sum_j |D_alpha|_ij |K_jb|`,
/// with `lip` a Lipschitz bound of `f` in the max norm near the stage states
/// (the caller's responsibility: the nonlinear term moves the argument of
/// every component of `f`), rounded upward. Evaluation rounding of either
/// residual is not included.
pub fn raw_residual_transport_bound(
    allowance: &RawStageAllowance,
    stages: &[Vec<f64>],
    h: f64,
    gamma: f64,
    lipschitz: f64,
) -> CoreResult<Vec<Vec<f64>>> {
    let s = allowance.d_gamma.len();
    if stages.len() != s || !(lipschitz.is_finite() && lipschitz >= 0.0) || !h.is_finite() {
        return Err(CoreError::InvalidInput(
            "RAW_STAGE_TRANSPORT: stage count, h or Lipschitz bound invalid".into(),
        ));
    }
    let n = stages.first().map_or(0, Vec::len);
    if stages.iter().any(|stage| stage.len() != n) {
        return Err(CoreError::InvalidInput(
            "RAW_STAGE_TRANSPORT: ragged stages".into(),
        ));
    }
    let scale = mul_up(mul_up(h.abs(), gamma.abs())?, lipschitz)?;
    let mut bound = vec![vec![0.0; n]; s];
    for (i, row) in bound.iter_mut().enumerate() {
        let mut argument = 0.0_f64;
        for b in 0..n {
            let mut shift = 0.0;
            for (j, stage) in stages.iter().enumerate() {
                shift = add_up(
                    shift,
                    mul_up(allowance.d_alpha[i][j].mag(), stage[b].abs())?,
                )?;
            }
            argument = argument.max(shift);
        }
        let nonlinear = mul_up(scale, argument)?;
        for (a, value) in row.iter_mut().enumerate() {
            let mut linear = 0.0;
            for (j, stage) in stages.iter().enumerate() {
                linear = add_up(
                    linear,
                    mul_up(allowance.d_gamma[i][j].mag(), stage[a].abs())?,
                )?;
            }
            *value = add_up(linear, nonlinear)?;
        }
    }
    Ok(bound)
}
