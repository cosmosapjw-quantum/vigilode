//! Outward stage-target certificates for small problems (re-audit R3 of
//! 2026-10-01: HOM-02, HOM-03, HOM-04). Research only.
//!
//! For a declared [`StageTarget`] and the quadratic test family
//! `f(y + d) = f(y) + J d + q * d^2` (componentwise `q`), the exact stage
//! vectors `K*` solve, for `i = 0..s`,
//!
//! ```text
//! W K_i = h (J y - q y^2) + h J sum_{j<i} L*_ij K_j + h q (sum_{j<i} alpha_ij K_j)^2,
//! W = I - h gamma J.
//! ```
//!
//! [`certify_stage_target`] bounds `|K_hat - K*|` componentwise for any
//! candidate `K_hat` without knowing `K*`: an outward enclosure of the
//! residual, an [`InverseWitness`] `U >= |W^-1|` entrywise, and a positive
//! recurrence over the stages whose every operation rounds upward. The
//! output and embedded projections add the rounding of the candidate's own
//! `y_hat = y + sum b_i K_i` and `e_hat = sum btilde_i K_i`.
//!
//! What it certifies is a [`CertificateKind::StageTargetBound`]: the
//! distance of the candidate from the exact root of the declared stage
//! equations. The embedded quantity is a bound on the target's embedded
//! estimate, an error *proxy*; neither is a bound on the ODE's local or
//! global error.
//!
//! [`doubling_certificate`] (HOM-04) replaces the serial recurrence by a
//! finite path sum `E = (I + H^4)(I + H^2)(I + H) a` over the strictly lower
//! block matrix `H` (so `H^8 = 0` holds on the target's own structure) with an
//! a priori state radius `D`, which must close (`sum_{j<i} |alpha_ij| E_j <=
//! D_i`) or the attempt is rejected. The radius comes from
//! [`PastStepData`], which by construction holds no reference solution.

use rodas5p_core::{
    CoreError, CoreResult,
    directed::{Interval, add_up, div_up, mul_up, sqrt_up, sub_down, sum_up},
    sha256_hex,
};
use serde::{Deserialize, Serialize};

use crate::StageTarget;

/// What a certificate bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CertificateKind {
    /// `|K_hat - K*|` for the exact root `K*` of the declared stage
    /// equations, and the induced output/embedded projections.
    StageTargetBound,
}

/// The test family: `J`, `y`, `h` and the quadratic coefficient `q`.
#[derive(Clone, Debug, PartialEq)]
pub struct QuadraticStageProblem {
    pub jacobian: Vec<Vec<f64>>,
    pub y: Vec<f64>,
    pub h: f64,
    pub q: Vec<f64>,
}

impl QuadraticStageProblem {
    pub fn dimension(&self) -> usize {
        self.y.len()
    }

    fn validate(&self) -> CoreResult<()> {
        let n = self.dimension();
        if n == 0
            || self.q.len() != n
            || self.jacobian.len() != n
            || self.jacobian.iter().any(|row| row.len() != n)
            || !(self.h.is_finite() && self.h > 0.0)
            || !self
                .jacobian
                .iter()
                .flatten()
                .chain(&self.y)
                .chain(&self.q)
                .all(|value| value.is_finite())
        {
            return Err(CoreError::InvalidInput(
                "CERTIFICATE_NOT_VALIDATED: invalid quadratic stage problem".into(),
            ));
        }
        Ok(())
    }

    fn jacobian_digest(&self) -> String {
        let bits = self
            .jacobian
            .iter()
            .flatten()
            .map(|value| format!("{:016x}", value.to_bits()))
            .collect::<Vec<_>>()
            .join(",");
        sha256_hex(bits.as_bytes())
    }
}

/// What an inverse witness is a witness for: the operator
/// `W = M - h gamma J` with `M = I`, its structure and its tolerance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitnessIdentity {
    pub h_bits: u64,
    pub gamma_bits: u64,
    pub jacobian_sha256: String,
    pub structure: String,
    pub tolerance_bits: u64,
}

impl WitnessIdentity {
    pub fn for_problem(
        problem: &QuadraticStageProblem,
        gamma: f64,
        structure: &str,
        tolerance: f64,
    ) -> Self {
        Self {
            h_bits: problem.h.to_bits(),
            gamma_bits: gamma.to_bits(),
            jacobian_sha256: problem.jacobian_digest(),
            structure: structure.into(),
            tolerance_bits: tolerance.to_bits(),
        }
    }
}

/// Cost of building a witness (HOM-03).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitnessWork {
    pub directed_operations: u64,
    pub stored_values: u64,
}

/// An entrywise upper bound `U >= |W^-1|` tied to the operator it bounds.
///
/// The fields are sealed (re-audit R4, R4-HOM-DEV-01): a value of this type
/// exists only through [`InverseWitness::diagonal`],
/// [`InverseWitness::small`], [`InverseWitness::approximate`] or
/// [`UnverifiedWitness::verify`], each of which proves the bound from the
/// operator. Editing `upper` (a zero entry, an empty row) after
/// construction used to pass the identity check and certify a first stage
/// error of 0 against an exact 0.0617. Wire data deserializes into
/// [`UnverifiedWitness`] and is rebuilt before use; an identity digest
/// binds a witness to an operator, it is not a proof of the bound.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct InverseWitness {
    identity: WitnessIdentity,
    upper: Vec<Vec<f64>>,
    /// `||I - V W||_inf` (upper) for an approximate-inverse witness, 0 for an
    /// exact structural one.
    residual_norm_upper: f64,
    work: WitnessWork,
    /// The approximate inverse `V` an approximate witness was proved from,
    /// kept so that wire data can be re-verified.
    approximate_inverse: Option<Vec<Vec<f64>>>,
}

/// An [`InverseWitness`] as read from the wire: nothing in it is trusted
/// until [`UnverifiedWitness::verify`] rebuilds the bound from the operator.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UnverifiedWitness {
    pub identity: WitnessIdentity,
    pub upper: Vec<Vec<f64>>,
    pub residual_norm_upper: f64,
    pub work: WitnessWork,
    #[serde(default)]
    pub approximate_inverse: Option<Vec<Vec<f64>>>,
}

pub const WITNESS_NOT_VERIFIED: &str = "WITNESS_NOT_VERIFIED";

impl UnverifiedWitness {
    /// Rebuild the witness its identity names for `problem` and `gamma`
    /// (`diagonal`, `exact-small`, or an approximate inverse `V`), and
    /// accept it only if the supplied bound is bit for bit the rebuilt one.
    /// A shape, sign or value change, another operator, or a structure with
    /// no reconstruction is [`WITNESS_NOT_VERIFIED`].
    pub fn verify(self, problem: &QuadraticStageProblem, gamma: f64) -> CoreResult<InverseWitness> {
        let reject = |why: &str| CoreError::InvalidInput(format!("{WITNESS_NOT_VERIFIED}: {why}"));
        let rebuilt = match (self.identity.structure.as_str(), &self.approximate_inverse) {
            ("diagonal", None) => InverseWitness::diagonal(problem, gamma)?,
            ("exact-small", None) => InverseWitness::small(problem, gamma)?,
            (structure, Some(v)) => InverseWitness::approximate(problem, gamma, v, structure)?,
            (structure, None) => {
                return Err(reject(&format!(
                    "structure {structure:?} has no reconstruction"
                )));
            }
        };
        if rebuilt.identity != self.identity {
            return Err(reject("the identity is for another operator"));
        }
        let same_bits = |a: &[Vec<f64>], b: &[Vec<f64>]| {
            a.len() == b.len()
                && a.iter().zip(b).all(|(x, y)| {
                    x.len() == y.len() && x.iter().zip(y).all(|(p, q)| p.to_bits() == q.to_bits())
                })
        };
        if !same_bits(&rebuilt.upper, &self.upper)
            || rebuilt.residual_norm_upper.to_bits() != self.residual_norm_upper.to_bits()
        {
            return Err(reject(
                "the supplied bound is not the bound of the operator",
            ));
        }
        Ok(rebuilt)
    }
}

fn shifted_entry(
    problem: &QuadraticStageProblem,
    gamma: f64,
    a: usize,
    b: usize,
) -> CoreResult<Interval> {
    let identity = Interval::point(if a == b { 1.0 } else { 0.0 })?;
    let hgj = Interval::point(problem.h)?
        .mul(Interval::point(gamma)?)?
        .mul(Interval::point(problem.jacobian[a][b])?)?;
    identity.sub(hgj)
}

impl InverseWitness {
    pub fn identity(&self) -> &WitnessIdentity {
        &self.identity
    }

    /// `U >= |W^-1|` entrywise, `n x n`, finite and nonnegative.
    pub fn upper(&self) -> &[Vec<f64>] {
        &self.upper
    }

    pub fn residual_norm_upper(&self) -> f64 {
        self.residual_norm_upper
    }

    pub fn work(&self) -> WitnessWork {
        self.work
    }

    /// The wire form, to be re-verified on the other side.
    pub fn to_unverified(&self) -> UnverifiedWitness {
        UnverifiedWitness {
            identity: self.identity.clone(),
            upper: self.upper.clone(),
            residual_norm_upper: self.residual_norm_upper,
            work: self.work,
            approximate_inverse: self.approximate_inverse.clone(),
        }
    }

    /// Diagonal `J`: `U_aa = up(1 / |1 - h gamma J_aa|)`.
    #[allow(clippy::needless_range_loop)] // index form mirrors the matrix formula
    pub fn diagonal(problem: &QuadraticStageProblem, gamma: f64) -> CoreResult<Self> {
        problem.validate()?;
        let n = problem.dimension();
        let mut work = WitnessWork::default();
        let mut upper = vec![vec![0.0; n]; n];
        for a in 0..n {
            for b in 0..n {
                if a != b && problem.jacobian[a][b] != 0.0 {
                    return Err(CoreError::InvalidInput(
                        "INVERSE_WITNESS_UNAVAILABLE: Jacobian is not diagonal".into(),
                    ));
                }
            }
            let entry = shifted_entry(problem, gamma, a, a)?;
            if entry.contains_zero() {
                return Err(CoreError::InvalidInput(
                    "INVERSE_WITNESS_UNAVAILABLE: singular diagonal".into(),
                ));
            }
            upper[a][a] = div_up(1.0, entry.mig())?;
            work.directed_operations += 5;
        }
        work.stored_values = n as u64;
        Ok(Self {
            identity: WitnessIdentity::for_problem(problem, gamma, "diagonal", 0.0),
            upper,
            residual_norm_upper: 0.0,
            work,
            approximate_inverse: None,
        })
    }

    /// `n <= 2`: the adjugate over an interval determinant.
    pub fn small(problem: &QuadraticStageProblem, gamma: f64) -> CoreResult<Self> {
        problem.validate()?;
        let n = problem.dimension();
        let w = |a, b| shifted_entry(problem, gamma, a, b);
        let upper = match n {
            1 => {
                let entry = w(0, 0)?;
                if entry.contains_zero() {
                    return Err(CoreError::InvalidInput(
                        "INVERSE_WITNESS_UNAVAILABLE: singular W".into(),
                    ));
                }
                vec![vec![div_up(1.0, entry.mig())?]]
            }
            2 => {
                let det = w(0, 0)?.mul(w(1, 1)?)?.sub(w(0, 1)?.mul(w(1, 0)?)?)?;
                if det.contains_zero() {
                    return Err(CoreError::InvalidInput(
                        "INVERSE_WITNESS_UNAVAILABLE: determinant interval contains 0".into(),
                    ));
                }
                let adj = [[w(1, 1)?, -w(0, 1)?], [-w(1, 0)?, w(0, 0)?]];
                let mut upper = vec![vec![0.0; 2]; 2];
                for a in 0..2 {
                    for b in 0..2 {
                        upper[a][b] = adj[a][b].div(det)?.mag();
                    }
                }
                upper
            }
            _ => {
                return Err(CoreError::InvalidInput(
                    "INVERSE_WITNESS_UNAVAILABLE: small witness needs n <= 2".into(),
                ));
            }
        };
        Ok(Self {
            identity: WitnessIdentity::for_problem(problem, gamma, "exact-small", 0.0),
            work: WitnessWork {
                directed_operations: (12 * n * n) as u64,
                stored_values: (n * n) as u64,
            },
            upper,
            residual_norm_upper: 0.0,
            approximate_inverse: None,
        })
    }

    /// A general approximate inverse `V`: `R >= |I - V W|` outward and
    /// `theta = ||R||_inf < 1`; then `|W^-1| <= (I - R)^-1 |V| <= |V| + R |V|
    /// + theta^2 / (1 - theta) 1 colmax(|V|)`. A banded or block-triangular
    /// `W` gives a structured `V` with the same bound; nothing here assumes
    /// that a Jacobian-vector product gives entrywise bounds on `J`.
    pub fn approximate(
        problem: &QuadraticStageProblem,
        gamma: f64,
        v: &[Vec<f64>],
        structure: &str,
    ) -> CoreResult<Self> {
        problem.validate()?;
        let n = problem.dimension();
        if structure == "diagonal" || structure == "exact-small" {
            return Err(CoreError::InvalidInput(format!(
                "INVERSE_WITNESS_UNAVAILABLE: {structure:?} names an exact structural witness"
            )));
        }
        if !v.iter().flatten().all(|value| value.is_finite()) {
            return Err(CoreError::InvalidInput(
                "INVERSE_WITNESS_UNAVAILABLE: approximate inverse is not finite".into(),
            ));
        }
        if v.len() != n || v.iter().any(|row| row.len() != n) {
            return Err(CoreError::Dimension(
                "INVERSE_WITNESS_UNAVAILABLE: approximate inverse shape".into(),
            ));
        }
        let mut work = WitnessWork::default();
        let mut residual = vec![vec![0.0; n]; n];
        for a in 0..n {
            for b in 0..n {
                let mut entry = Interval::point(if a == b { 1.0 } else { 0.0 })?;
                for (c, v_ac) in v[a].iter().enumerate() {
                    entry = entry
                        .sub(Interval::point(*v_ac)?.mul(shifted_entry(problem, gamma, c, b)?)?)?;
                    work.directed_operations += 8;
                }
                residual[a][b] = entry.mag();
            }
        }
        let theta = residual
            .iter()
            .map(|row| sum_up(row.iter().copied()))
            .collect::<CoreResult<Vec<_>>>()?
            .into_iter()
            .fold(0.0_f64, f64::max);
        if theta.is_nan() || theta >= 1.0 {
            return Err(CoreError::InvalidInput(format!(
                "INVERSE_WITNESS_UNAVAILABLE: ||I - V W||_inf <= {theta:e} is not below 1"
            )));
        }
        let abs_v = v
            .iter()
            .map(|row| row.iter().map(|value| value.abs()).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let column_max = (0..n)
            .map(|b| abs_v.iter().map(|row| row[b]).fold(0.0_f64, f64::max))
            .collect::<Vec<_>>();
        let tail = div_up(mul_up(theta, theta)?, sub_down(1.0, theta)?)?;
        let mut upper = vec![vec![0.0; n]; n];
        for a in 0..n {
            for b in 0..n {
                let first = sum_up((0..n).map(|c| {
                    mul_up(residual[a][c], abs_v[c][b]).expect("finite by construction")
                }))?;
                upper[a][b] = add_up(add_up(abs_v[a][b], first)?, mul_up(tail, column_max[b])?)?;
                work.directed_operations += (2 * n + 3) as u64;
            }
        }
        work.stored_values = (2 * n * n) as u64;
        Ok(Self {
            identity: WitnessIdentity::for_problem(problem, gamma, structure, theta),
            upper,
            residual_norm_upper: theta,
            work,
            approximate_inverse: Some(v.to_vec()),
        })
    }

    fn apply_upper(&self, vector: &[f64]) -> CoreResult<Vec<f64>> {
        self.upper
            .iter()
            .map(|row| upper_dot(row, vector))
            .collect()
    }
}

fn upper_dot(a: &[f64], b: &[f64]) -> CoreResult<f64> {
    let mut total = 0.0;
    for (x, y) in a.iter().zip(b) {
        total = add_up(total, mul_up(x.abs(), y.abs())?)?;
    }
    Ok(total)
}

fn interval_dot(weights: &[Interval], values: &[f64]) -> CoreResult<Interval> {
    let mut total = Interval::point(0.0)?;
    for (w, v) in weights.iter().zip(values) {
        total = total.add(w.mul(Interval::point(*v)?)?)?;
    }
    Ok(total)
}

fn column(stages: &[Vec<f64>], a: usize, upto: usize) -> Vec<f64> {
    stages[..upto].iter().map(|stage| stage[a]).collect()
}

/// Residual intervals per stage and component, and increment bounds.
type ResidualEnclosure = (Vec<Vec<Interval>>, Vec<Vec<f64>>);

/// Residual intervals `r_i` of the declared stage equations at the
/// candidate and the upper bounds `|delta_i|` of the candidate's stage
/// increments.
fn residual_enclosure(
    target: &StageTarget,
    problem: &QuadraticStageProblem,
    candidate: &[Vec<f64>],
) -> CoreResult<ResidualEnclosure> {
    let n = problem.dimension();
    let h = Interval::point(problem.h)?;
    let gamma = Interval::point(target.gamma)?;
    let mut rhs = Vec::with_capacity(n);
    for a in 0..n {
        let jy = interval_dot(
            &problem.jacobian[a]
                .iter()
                .map(|value| Interval::point(*value))
                .collect::<CoreResult<Vec<_>>>()?,
            &problem.y,
        )?;
        let qyy = Interval::point(problem.q[a])?
            .mul(Interval::point(problem.y[a])?)?
            .mul(Interval::point(problem.y[a])?)?;
        rhs.push(h.mul(jy.sub(qyy)?)?);
    }
    let mut residuals = Vec::with_capacity(target.stages());
    let mut increments = Vec::with_capacity(target.stages());
    for i in 0..target.stages() {
        let alpha = target.alpha_rows[i]
            .iter()
            .map(|value| Interval::point(*value))
            .collect::<CoreResult<Vec<_>>>()?;
        let delta = (0..n)
            .map(|a| interval_dot(&alpha, &column(candidate, a, i)))
            .collect::<CoreResult<Vec<_>>>()?;
        let coupled = (0..n)
            .map(|a| interval_dot(&target.coupling_rows[i], &column(candidate, a, i)))
            .collect::<CoreResult<Vec<_>>>()?;
        let mut row = Vec::with_capacity(n);
        for a in 0..n {
            let mut wk = Interval::point(0.0)?;
            let mut jc = Interval::point(0.0)?;
            for b in 0..n {
                let identity = Interval::point(if a == b { 1.0 } else { 0.0 })?;
                let j_ab = Interval::point(problem.jacobian[a][b])?;
                let w_ab = identity.sub(h.mul(gamma)?.mul(j_ab)?)?;
                wk = wk.add(w_ab.mul(Interval::point(candidate[i][b])?)?)?;
                jc = jc.add(j_ab.mul(coupled[b])?)?;
            }
            let quadratic = h
                .mul(Interval::point(problem.q[a])?)?
                .mul(delta[a].mul(delta[a])?)?;
            row.push(wk.sub(h.mul(jc)?)?.sub(rhs[a])?.sub(quadratic)?);
        }
        increments.push(delta.iter().map(Interval::mag).collect());
        residuals.push(row);
    }
    Ok((residuals, increments))
}

/// A verified stage-target certificate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StageCertificate {
    pub kind: CertificateKind,
    pub target_id: String,
    pub witness: WitnessIdentity,
    /// SHA-256 of the candidate stage bits the bound is about.
    pub candidate_sha256: String,
    /// `E_i >= |K_hat_i - K*_i|` componentwise.
    pub stage_bound: Vec<Vec<f64>>,
    pub output_bound: Vec<f64>,
    pub embedded_difference_bound: Vec<f64>,
    pub output_wrms_upper: f64,
    pub embedded_difference_wrms_upper: f64,
    /// WRMS of `|e_hat| + B_e`: an upper bound on the target's embedded
    /// estimate (an error proxy, not an ODE error bound).
    pub embedded_target_wrms_upper: f64,
    /// WRMS of `(|e_hat| - B_e)_+`: a lower bound on the same.
    pub embedded_target_wrms_lower: f64,
    pub combined_proxy_upper: f64,
    pub directed_operations: u64,
    /// [`certificate_binding`] of everything the bound is about (re-audit
    /// R4, R4-HOM-DEV-03); see [`StageCertificate::is_bound_to`].
    #[serde(default)]
    pub binding_sha256: String,
}

impl StageCertificate {
    /// Whether this certificate was computed for exactly this target,
    /// problem, candidate, projections `y_hat` and `e_hat`, witness and
    /// output scale. Any changed coefficient, state, step, Jacobian entry,
    /// `q`, projection, witness identity, tolerance or candidate bit makes
    /// it stale. An identity check for consumers that keep certificates, not
    /// a proof: the bound itself comes from the enclosure that produced it.
    #[allow(clippy::too_many_arguments)]
    pub fn is_bound_to(
        &self,
        target: &StageTarget,
        problem: &QuadraticStageProblem,
        candidate: &[Vec<f64>],
        y_hat: &[f64],
        e_hat: &[f64],
        witness: &WitnessIdentity,
        atol: f64,
        rtol: f64,
    ) -> bool {
        self.binding_sha256
            == certificate_binding(
                target, problem, candidate, y_hat, e_hat, witness, atol, rtol,
            )
    }
}

/// Canonical SHA-256 of a certificate's subject (re-audit R4,
/// R4-HOM-DEV-03): the target id and every coefficient bit (`gamma`, `c`,
/// the `gamma`, `alpha` and coupling rows with both interval ends, `b`,
/// `btilde`), the problem's `y`, `h`, `J` and `q` bits, the candidate's
/// stage bits, the projections `y_hat` and `e_hat`, the witness identity
/// and the output scale `(atol, rtol)`. Every field is length-prefixed, so
/// no separator inside a string can make two subjects collide.
#[allow(clippy::too_many_arguments)]
pub fn certificate_binding(
    target: &StageTarget,
    problem: &QuadraticStageProblem,
    candidate: &[Vec<f64>],
    y_hat: &[f64],
    e_hat: &[f64],
    witness: &WitnessIdentity,
    atol: f64,
    rtol: f64,
) -> String {
    fn hex(values: impl IntoIterator<Item = f64>) -> String {
        values
            .into_iter()
            .map(|value| format!("{:016x}", value.to_bits()))
            .collect::<Vec<_>>()
            .join(",")
    }
    let rows = |rows: &[Vec<f64>]| {
        rows.iter()
            .map(|row| format!("[{}]", hex(row.iter().copied())))
            .collect::<Vec<_>>()
            .join(";")
    };
    let coupling = target
        .coupling_rows
        .iter()
        .map(|row| {
            format!(
                "[{}]",
                hex(row.iter().flat_map(|interval| [interval.lo, interval.hi]))
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    let fields = [
        ("target", target.id.to_string()),
        ("snapshot", target.snapshot_sha256.to_string()),
        ("gamma", hex([target.gamma])),
        ("c", hex(target.c.iter().copied())),
        ("gamma_rows", hex(target.gamma_rows.iter().copied())),
        ("alpha", rows(&target.alpha_rows)),
        ("coupling", coupling),
        ("b", hex(target.b.iter().copied())),
        ("btilde", hex(target.btilde.iter().copied())),
        ("y", hex(problem.y.iter().copied())),
        ("h", hex([problem.h])),
        ("J", rows(&problem.jacobian)),
        ("q", hex(problem.q.iter().copied())),
        ("candidate", rows(candidate)),
        ("y_hat", hex(y_hat.iter().copied())),
        ("e_hat", hex(e_hat.iter().copied())),
        ("witness_h", format!("{:016x}", witness.h_bits)),
        ("witness_gamma", format!("{:016x}", witness.gamma_bits)),
        ("witness_jacobian", witness.jacobian_sha256.clone()),
        ("witness_structure", witness.structure.clone()),
        (
            "witness_tolerance",
            format!("{:016x}", witness.tolerance_bits),
        ),
        ("atol", hex([atol])),
        ("rtol", hex([rtol])),
    ];
    let mut canonical = String::from("vigilode-stage-certificate-binding-v2");
    for (name, value) in fields {
        canonical.push_str(&format!("|{name}:{}:{value}", value.len()));
    }
    sha256_hex(canonical.as_bytes())
}

/// `L = ceil(log2 s)` doubling levels for `s >= 1` stages: the smallest `L`
/// with `2^L >= s` (0 for one stage, whose `H` is zero).
pub fn doubling_levels(stages: usize) -> usize {
    let mut levels = 0;
    while (1_usize << levels) < stages {
        levels += 1;
    }
    levels
}

/// Digest of a candidate's stage bits.
pub fn candidate_digest(candidate: &[Vec<f64>]) -> String {
    let bits = candidate
        .iter()
        .flatten()
        .map(|value| format!("{:016x}", value.to_bits()))
        .collect::<Vec<_>>()
        .join(",");
    sha256_hex(bits.as_bytes())
}

fn validate_inputs(
    target: &StageTarget,
    problem: &QuadraticStageProblem,
    candidate: &[Vec<f64>],
    witness: &InverseWitness,
) -> CoreResult<()> {
    problem.validate()?;
    let n = problem.dimension();
    let s = target.stages();
    if s == 0
        || target.coupling_rows.len() != s
        || target.b.len() != s
        || target.btilde.len() != s
        || !target.gamma.is_finite()
    {
        return Err(CoreError::InvalidInput(
            "TARGET_SEMANTICS_UNRESOLVED: a target needs s >= 1 stages with matching b, btilde and coupling rows".into(),
        ));
    }
    if !target.strictly_lower_nilpotent() {
        return Err(CoreError::InvalidInput(
            "TARGET_SEMANTICS_UNRESOLVED: target is not strictly lower".into(),
        ));
    }
    if candidate.len() != target.stages()
        || candidate
            .iter()
            .any(|stage| stage.len() != n || !stage.iter().all(|value| value.is_finite()))
    {
        return Err(CoreError::InvalidInput(
            "CERTIFICATE_NOT_VALIDATED: candidate shape or values".into(),
        ));
    }
    let expected = WitnessIdentity {
        structure: witness.identity.structure.clone(),
        tolerance_bits: witness.identity.tolerance_bits,
        ..WitnessIdentity::for_problem(problem, target.gamma, "", 0.0)
    };
    let well_formed = witness.upper.len() == n
        && witness
            .upper
            .iter()
            .all(|row| row.len() == n && row.iter().all(|x| x.is_finite() && *x >= 0.0));
    if witness.identity != expected || !well_formed {
        return Err(CoreError::InvalidInput(
            "CERTIFICATE_NOT_VALIDATED: the inverse witness is for another operator".into(),
        ));
    }
    Ok(())
}

/// WRMS upper bound with the scale bounded from below.
fn wrms_upper(values: &[f64], scale_lower: &[f64]) -> CoreResult<f64> {
    let mut total = 0.0;
    for (value, scale) in values.iter().zip(scale_lower) {
        let ratio = div_up(value.abs(), *scale)?;
        total = add_up(total, mul_up(ratio, ratio)?)?;
    }
    sqrt_up(div_up(total, values.len() as f64)?)
}

fn wrms_lower(values: &[f64], scale_upper: &[f64]) -> CoreResult<f64> {
    use rodas5p_core::directed::{add_down, div_down, mul_down, sqrt_down};
    let mut total = 0.0;
    for (value, scale) in values.iter().zip(scale_upper) {
        let ratio = div_down(value.abs(), *scale)?;
        total = add_down(total, mul_down(ratio, ratio)?)?;
    }
    sqrt_down(div_down(total, values.len() as f64)?)
}

/// The projection part shared by both certificates.
#[allow(clippy::too_many_arguments)]
fn finish_certificate(
    target: &StageTarget,
    problem: &QuadraticStageProblem,
    candidate: &[Vec<f64>],
    y_hat: &[f64],
    e_hat: &[f64],
    stage_bound: Vec<Vec<f64>>,
    witness: &InverseWitness,
    atol: f64,
    rtol: f64,
    mut operations: u64,
) -> CoreResult<StageCertificate> {
    use rodas5p_core::directed::{add_down, mul_down};
    let n = problem.dimension();
    if y_hat.len() != n || e_hat.len() != n {
        return Err(CoreError::Dimension(
            "CERTIFICATE_NOT_VALIDATED: projection shape".into(),
        ));
    }
    let b = target
        .b
        .iter()
        .map(|value| Interval::point(*value))
        .collect::<CoreResult<Vec<_>>>()?;
    let bt = target
        .btilde
        .iter()
        .map(|value| Interval::point(*value))
        .collect::<CoreResult<Vec<_>>>()?;
    let mut output_bound = Vec::with_capacity(n);
    let mut embedded_bound = Vec::with_capacity(n);
    for a in 0..n {
        let stages = column(candidate, a, target.stages());
        let bounds = column(&stage_bound, a, target.stages());
        let y_exact = Interval::point(problem.y[a])?.add(interval_dot(&b, &stages)?)?;
        let e_exact = interval_dot(&bt, &stages)?;
        let rounding_y = y_exact.sub(Interval::point(y_hat[a])?)?.mag();
        let rounding_e = e_exact.sub(Interval::point(e_hat[a])?)?.mag();
        output_bound.push(add_up(upper_dot(&target.b, &bounds)?, rounding_y)?);
        embedded_bound.push(add_up(upper_dot(&target.btilde, &bounds)?, rounding_e)?);
        operations += (8 * target.stages() + 4) as u64;
    }
    let scale_lower = (0..n)
        .map(|a| {
            add_down(
                atol,
                mul_down(rtol, problem.y[a].abs().max(y_hat[a].abs()))?,
            )
        })
        .collect::<CoreResult<Vec<_>>>()?;
    let scale_upper = (0..n)
        .map(|a| add_up(atol, mul_up(rtol, problem.y[a].abs().max(y_hat[a].abs()))?))
        .collect::<CoreResult<Vec<_>>>()?;
    if scale_lower
        .iter()
        .any(|scale| scale.is_nan() || *scale <= 0.0)
    {
        return Err(CoreError::InvalidInput(
            "CERTIFICATE_NOT_VALIDATED: nonpositive error scale".into(),
        ));
    }
    let output_wrms_upper = wrms_upper(&output_bound, &scale_lower)?;
    let embedded_difference_wrms_upper = wrms_upper(&embedded_bound, &scale_lower)?;
    let embedded_upper = (0..n)
        .map(|a| add_up(e_hat[a].abs(), embedded_bound[a]))
        .collect::<CoreResult<Vec<_>>>()?;
    let embedded_lower = (0..n)
        .map(|a| Ok(sub_down(e_hat[a].abs(), embedded_bound[a])?.max(0.0)))
        .collect::<CoreResult<Vec<_>>>()?;
    let embedded_target_wrms_upper = wrms_upper(&embedded_upper, &scale_lower)?;
    let embedded_target_wrms_lower = wrms_lower(&embedded_lower, &scale_upper)?;
    Ok(StageCertificate {
        kind: CertificateKind::StageTargetBound,
        target_id: target.id.into(),
        witness: witness.identity.clone(),
        candidate_sha256: candidate_digest(candidate),
        stage_bound,
        output_bound,
        embedded_difference_bound: embedded_bound,
        output_wrms_upper,
        embedded_difference_wrms_upper,
        embedded_target_wrms_upper,
        embedded_target_wrms_lower,
        combined_proxy_upper: add_up(output_wrms_upper, embedded_target_wrms_upper)?,
        directed_operations: operations + witness.work.directed_operations,
        binding_sha256: certificate_binding(
            target,
            problem,
            candidate,
            y_hat,
            e_hat,
            &witness.identity,
            atol,
            rtol,
        ),
    })
}

/// The serial certificate (HOM-02): residual enclosure and the positive
/// recurrence
/// `E_i = U [ |r_i| + h (|J| sum_{j<i} |L*_ij| E_j + |q| (2 |delta_i| d_i + d_i^2)) ]`,
/// `d_i = sum_{j<i} |alpha_ij| E_j`, every operation rounded upward. Any non-finite intermediate, a witness
/// for another operator or a target that is not strictly lower is a typed
/// rejection.
#[allow(clippy::too_many_arguments)]
pub fn certify_stage_target(
    target: &StageTarget,
    problem: &QuadraticStageProblem,
    candidate: &[Vec<f64>],
    y_hat: &[f64],
    e_hat: &[f64],
    witness: &InverseWitness,
    atol: f64,
    rtol: f64,
) -> CoreResult<StageCertificate> {
    validate_inputs(target, problem, candidate, witness)?;
    let n = problem.dimension();
    let (residuals, increments) = residual_enclosure(target, problem, candidate)?;
    let mut operations = (target.stages() * n * (6 * n + 12)) as u64;
    let mut bound: Vec<Vec<f64>> = Vec::with_capacity(target.stages());
    for i in 0..target.stages() {
        let coupling = target.coupling_rows[i]
            .iter()
            .map(Interval::mag)
            .collect::<Vec<_>>();
        let mut right = Vec::with_capacity(n);
        let d = (0..n)
            .map(|a| upper_dot(&target.alpha_rows[i], &column(&bound, a, i)))
            .collect::<CoreResult<Vec<_>>>()?;
        let lc = (0..n)
            .map(|a| upper_dot(&coupling, &column(&bound, a, i)))
            .collect::<CoreResult<Vec<_>>>()?;
        for a in 0..n {
            let linear = upper_dot(&problem.jacobian[a], &lc)?;
            let remainder = mul_up(
                problem.q[a].abs(),
                add_up(
                    mul_up(mul_up(2.0, increments[i][a])?, d[a])?,
                    mul_up(d[a], d[a])?,
                )?,
            )?;
            right.push(add_up(
                residuals[i][a].mag(),
                mul_up(problem.h, add_up(linear, remainder)?)?,
            )?);
        }
        bound.push(witness.apply_upper(&right)?);
        operations += (n * (4 * i + 4 * n + 8)) as u64;
    }
    finish_certificate(
        target, problem, candidate, y_hat, e_hat, bound, witness, atol, rtol, operations,
    )
}

/// Data from earlier accepted steps that may set the a priori radius. It
/// has no field for a reference solution or for the current step's serial
/// bound: a predictor cannot leak them (HOM-04).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PastStepData {
    /// Largest residual magnitude of the last accepted step, if any.
    pub previous_max_residual: Option<f64>,
    pub previous_h: Option<f64>,
    pub order: i32,
}

/// `D = max(D0, kappa max|r| (h / h_prev)^p)` from past accepted data only.
pub fn predict_state_radius(
    past: &PastStepData,
    h: f64,
    floor: f64,
    kappa: f64,
) -> CoreResult<f64> {
    let predicted = match (past.previous_max_residual, past.previous_h) {
        (Some(residual), Some(previous_h)) if previous_h > 0.0 => {
            let ratio = div_up(h, previous_h)?;
            let mut power = 1.0_f64;
            for _ in 0..past.order.max(0) {
                power = mul_up(power, ratio)?;
            }
            mul_up(mul_up(kappa, residual.abs())?, power)?
        }
        _ => 0.0,
    };
    Ok(floor.max(predicted))
}

/// One radius attempt, kept whether or not it closed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RadiusAttempt {
    pub attempt: usize,
    pub radius: f64,
    pub closes: bool,
    pub max_state_radius: f64,
    pub reason: Option<String>,
}

/// The path-sum certificate and the ledger of every radius tried.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DoublingCertificate {
    pub certificate: Option<StageCertificate>,
    pub attempts: Vec<RadiusAttempt>,
    pub workers: usize,
    /// Thread pools built by this call (re-audit R4, R4-HOM-DEV-04): one
    /// per call at most, none with a caller-owned execution context.
    #[serde(default)]
    pub pool_creations: u64,
}

type Matrix = Vec<Vec<f64>>;

/// `A B` rounded upward for nonnegative matrices, rows in parallel with a
/// fixed per-entry summation order (bitwise identical for any worker count).
fn upper_matmul(
    a: &Matrix,
    b: &Matrix,
    execution: &crate::ParallelExecution,
) -> CoreResult<Matrix> {
    let m = b.first().map_or(0, Vec::len);
    // Rows are independent and each is summed in a fixed order, so the
    // result is the same for any thread count.
    execution.map_ordered(a, |left: &Vec<f64>| -> CoreResult<Vec<f64>> {
        (0..m)
            .map(|c| {
                let mut total = 0.0;
                for (k, value) in left.iter().enumerate() {
                    if *value != 0.0 && b[k][c] != 0.0 {
                        total = add_up(total, mul_up(*value, b[k][c])?)?;
                    }
                }
                Ok(total)
            })
            .collect()
    })
}

/// HOM-04: `E = S a`, `S = (I + H^4)(I + H^2)(I + H)` evaluated as three
/// doubling levels (`S += Q S; Q = Q^2`), with
/// `H_(i,u),(j,v) = h sum_w U_uw (|J_wv| |L*_ij| + [w = v] ell_iw |alpha_ij|)`
/// for `j < i`, `ell_iw = |q_w| (2 |delta_iw| + D)`, and `a_i = U |r_i|`.
/// The product has `L = ceil(log2 s)` factors ([`doubling_levels`]).
/// The radius must close; each failed radius is recorded and retried at
/// four times the radius, up to `max_attempts`.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::needless_range_loop)] // index form mirrors the matrix formula
pub fn doubling_certificate(
    target: &StageTarget,
    problem: &QuadraticStageProblem,
    candidate: &[Vec<f64>],
    y_hat: &[f64],
    e_hat: &[f64],
    witness: &InverseWitness,
    atol: f64,
    rtol: f64,
    initial_radius: f64,
    max_attempts: usize,
    workers: usize,
) -> CoreResult<DoublingCertificate> {
    let execution = crate::ParallelExecution::rayon(workers.max(1))?;
    let mut certificate = doubling_certificate_with_execution(
        target,
        problem,
        candidate,
        y_hat,
        e_hat,
        witness,
        atol,
        rtol,
        initial_radius,
        max_attempts,
        &execution,
    )?;
    certificate.pool_creations = u64::from(execution.threads() > 1);
    Ok(certificate)
}

/// [`doubling_certificate`] on a caller-owned execution context (re-audit
/// R4, R4-HOM-DEV-04): every matrix product of every radius attempt uses
/// it; no pool is created per product.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::needless_range_loop)] // index form mirrors the matrix formula
pub fn doubling_certificate_with_execution(
    target: &StageTarget,
    problem: &QuadraticStageProblem,
    candidate: &[Vec<f64>],
    y_hat: &[f64],
    e_hat: &[f64],
    witness: &InverseWitness,
    atol: f64,
    rtol: f64,
    initial_radius: f64,
    max_attempts: usize,
    execution: &crate::ParallelExecution,
) -> CoreResult<DoublingCertificate> {
    let workers = execution.threads();
    validate_inputs(target, problem, candidate, witness)?;
    if !(initial_radius.is_finite() && initial_radius >= 0.0) {
        return Err(CoreError::InvalidInput(format!(
            "CERTIFICATE_NOT_VALIDATED: initial state radius {initial_radius:e} must be finite and >= 0"
        )));
    }
    let n = problem.dimension();
    let s = target.stages();
    let m = s * n;
    // prod_{l < L} (I + H^(2^l)) = sum_{j < 2^L} H^j covers every path of
    // the strictly lower H (H^s = 0) iff 2^L >= s (re-audit R4,
    // R4-HOM-DEV-02: the fixed L = 3 dropped H^8.. for s = 9 and 16).
    let levels = doubling_levels(s);
    let (residuals, increments) = residual_enclosure(target, problem, candidate)?;
    let mut a = Vec::with_capacity(m);
    for residual in &residuals {
        let magnitudes = residual.iter().map(Interval::mag).collect::<Vec<_>>();
        a.extend(witness.apply_upper(&magnitudes)?);
    }
    let mut attempts = Vec::new();
    let mut radius = initial_radius;
    for attempt in 0..max_attempts.max(1) {
        let mut h_matrix = vec![vec![0.0; m]; m];
        for i in 0..s {
            for j in 0..i {
                let coupling = target.coupling_rows[i][j].mag();
                let alpha = target.alpha_rows[i][j].abs();
                for u in 0..n {
                    for v in 0..n {
                        let mut inner = 0.0;
                        for w in 0..n {
                            let ell = mul_up(
                                problem.q[w].abs(),
                                add_up(mul_up(2.0, increments[i][w])?, radius)?,
                            )?;
                            let mut term = mul_up(problem.jacobian[w][v].abs(), coupling)?;
                            if w == v {
                                term = add_up(term, mul_up(ell, alpha)?)?;
                            }
                            inner = add_up(inner, mul_up(witness.upper[u][w], term)?)?;
                        }
                        h_matrix[i * n + u][j * n + v] = mul_up(problem.h, inner)?;
                    }
                }
            }
        }
        let mut sum = (0..m)
            .map(|r| (0..m).map(|c| if r == c { 1.0 } else { 0.0 }).collect())
            .collect::<Matrix>();
        let mut power = h_matrix;
        for level in 0..levels {
            let product = upper_matmul(&power, &sum, execution)?;
            for (row, add) in sum.iter_mut().zip(&product) {
                for (value, extra) in row.iter_mut().zip(add) {
                    *value = add_up(*value, *extra)?;
                }
            }
            if level + 1 < levels {
                power = upper_matmul(&power, &power, execution)?;
            }
        }
        let flat = sum
            .iter()
            .map(|row| upper_dot(row, &a))
            .collect::<CoreResult<Vec<_>>>()?;
        let bound = (0..s)
            .map(|i| flat[i * n..(i + 1) * n].to_vec())
            .collect::<Vec<_>>();
        let mut max_state_radius = 0.0_f64;
        for i in 0..s {
            for v in 0..n {
                let state = upper_dot(&target.alpha_rows[i], &column(&bound, v, i))?;
                max_state_radius = max_state_radius.max(state);
            }
        }
        let closes = max_state_radius <= radius;
        attempts.push(RadiusAttempt {
            attempt,
            radius,
            closes,
            max_state_radius,
            reason: (!closes).then(|| "RADIUS_CLOSURE_FAIL".into()),
        });
        if closes {
            let certificate = finish_certificate(
                target,
                problem,
                candidate,
                y_hat,
                e_hat,
                bound,
                witness,
                atol,
                rtol,
                ((2 * levels).saturating_sub(1) * m * m * m) as u64,
            )?;
            return Ok(DoublingCertificate {
                certificate: Some(certificate),
                attempts,
                workers,
                pool_creations: 0,
            });
        }
        radius = mul_up(radius, 4.0)?;
    }
    Ok(DoublingCertificate {
        certificate: None,
        attempts,
        workers,
        pool_creations: 0,
    })
}

/// Allocation and arithmetic of a component-blocked certificate, kept
/// apart (re-audit R4, R4-HOM-DEV-05): `allocated_values` counts the f64
/// slots actually allocated for the blocks, `nonzeros` the nonzero entries
/// of the `n` stage blocks of `H`, `directed_operations` every rounded
/// multiply and add of the products.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockedCertificateWork {
    pub components: usize,
    pub block_size: usize,
    pub levels: usize,
    pub allocated_values: u64,
    pub nonzeros: u64,
    pub directed_operations: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BlockedDoublingCertificate {
    pub doubling: DoublingCertificate,
    pub work: BlockedCertificateWork,
}

pub const CERTIFICATE_STRUCTURE_UNSUPPORTED: &str = "CERTIFICATE_STRUCTURE_UNSUPPORTED";

/// [`doubling_certificate_with_execution`] for a diagonal `J` with the
/// structural diagonal witness (re-audit R4, R4-HOM-DEV-05). Then `U`, `J`
/// and `q` couple no two components, so with the component-major order
/// `P H P^T = diag(H_1, .., H_n)`, `H_u` the `s x s` stage block
/// `(H_u)_ij = h U_uu (|J_uu| |L*_ij| + ell_iu |alpha_ij|)`, `j < i`, and
/// `E_u = prod_l (I + H_u^(2^l)) a_u` per component, in parallel over the
/// components on `execution`. The products skip zero terms in the same
/// order as the full `(sn) x (sn)` products, so the bound is bit for bit
/// the full one, with `n s^2` values per block matrix instead of `(sn)^2`
/// and `O(n s^3 log s)` formal work instead of `O((sn)^3 log s)`. Any other
/// structure (a non-diagonal `J`, an approximate or small witness) is
/// [`CERTIFICATE_STRUCTURE_UNSUPPORTED`]: nothing is applied silently
/// beyond the diagonal case.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::needless_range_loop)] // index form mirrors the matrix formula
pub fn blocked_doubling_certificate_with_execution(
    target: &StageTarget,
    problem: &QuadraticStageProblem,
    candidate: &[Vec<f64>],
    y_hat: &[f64],
    e_hat: &[f64],
    witness: &InverseWitness,
    atol: f64,
    rtol: f64,
    initial_radius: f64,
    max_attempts: usize,
    execution: &crate::ParallelExecution,
) -> CoreResult<BlockedDoublingCertificate> {
    validate_inputs(target, problem, candidate, witness)?;
    let n = problem.dimension();
    let diagonal_jacobian = (0..n).all(|a| (0..n).all(|b| a == b || problem.jacobian[a][b] == 0.0));
    if witness.identity.structure != "diagonal" || !diagonal_jacobian {
        return Err(CoreError::InvalidInput(format!(
            "{CERTIFICATE_STRUCTURE_UNSUPPORTED}: the blocked certificate needs a diagonal J and the diagonal witness (got {:?})",
            witness.identity.structure
        )));
    }
    if !(initial_radius.is_finite() && initial_radius >= 0.0) {
        return Err(CoreError::InvalidInput(format!(
            "CERTIFICATE_NOT_VALIDATED: initial state radius {initial_radius:e} must be finite and >= 0"
        )));
    }
    let s = target.stages();
    let levels = doubling_levels(s);
    let (residuals, increments) = residual_enclosure(target, problem, candidate)?;
    // a_(i,u) = U_uu |r_iu| (U is diagonal: the full apply_upper adds
    // exact zeros only).
    let mut a = vec![vec![0.0; s]; n];
    for (i, residual) in residuals.iter().enumerate() {
        let magnitudes = residual.iter().map(Interval::mag).collect::<Vec<_>>();
        let applied = witness.apply_upper(&magnitudes)?;
        for u in 0..n {
            a[u][i] = applied[u];
        }
    }
    let mut work = BlockedCertificateWork {
        components: n,
        block_size: s,
        levels,
        ..BlockedCertificateWork::default()
    };
    let components = (0..n).collect::<Vec<_>>();
    let mut attempts = Vec::new();
    let mut radius = initial_radius;
    for attempt in 0..max_attempts.max(1) {
        type BlockResult = (Vec<f64>, u64, u64, u64);
        let blocks = execution.map_ordered(&components, |&u| -> CoreResult<BlockResult> {
            let (mut allocated, mut nonzeros, mut operations) = (0_u64, 0_u64, 0_u64);
            let mut h_block = vec![vec![0.0; s]; s];
            allocated += (s * s) as u64;
            for i in 0..s {
                for j in 0..i {
                    let coupling = target.coupling_rows[i][j].mag();
                    let alpha = target.alpha_rows[i][j].abs();
                    let ell = mul_up(
                        problem.q[u].abs(),
                        add_up(mul_up(2.0, increments[i][u])?, radius)?,
                    )?;
                    let mut term = mul_up(problem.jacobian[u][u].abs(), coupling)?;
                    term = add_up(term, mul_up(ell, alpha)?)?;
                    let inner = add_up(0.0, mul_up(witness.upper[u][u], term)?)?;
                    h_block[i][j] = mul_up(problem.h, inner)?;
                    operations += 8;
                    nonzeros += u64::from(h_block[i][j] != 0.0);
                }
            }
            let mut sum = (0..s)
                .map(|r| (0..s).map(|c| if r == c { 1.0 } else { 0.0 }).collect())
                .collect::<Matrix>();
            allocated += (s * s) as u64;
            let product =
                |left: &Matrix, right: &Matrix, operations: &mut u64| -> CoreResult<Matrix> {
                    let mut out = vec![vec![0.0; s]; s];
                    for (r, row) in left.iter().enumerate() {
                        for c in 0..s {
                            let mut total = 0.0;
                            for (k, value) in row.iter().enumerate() {
                                if *value != 0.0 && right[k][c] != 0.0 {
                                    total = add_up(total, mul_up(*value, right[k][c])?)?;
                                    *operations += 2;
                                }
                            }
                            out[r][c] = total;
                        }
                    }
                    Ok(out)
                };
            let mut power = h_block;
            for level in 0..levels {
                let added = product(&power, &sum, &mut operations)?;
                allocated += (s * s) as u64;
                for (row, extra_row) in sum.iter_mut().zip(&added) {
                    for (value, extra) in row.iter_mut().zip(extra_row) {
                        *value = add_up(*value, *extra)?;
                        operations += 1;
                    }
                }
                if level + 1 < levels {
                    power = product(&power, &power, &mut operations)?;
                    allocated += (s * s) as u64;
                }
            }
            let bound = sum
                .iter()
                .map(|row| upper_dot(row, &a[u]))
                .collect::<CoreResult<Vec<_>>>()?;
            operations += (2 * s * s) as u64;
            Ok((bound, allocated, nonzeros, operations))
        })?;
        let mut bound = vec![vec![0.0; n]; s];
        for (u, (column_bound, allocated, nonzeros, operations)) in blocks.into_iter().enumerate() {
            for i in 0..s {
                bound[i][u] = column_bound[i];
            }
            work.allocated_values += allocated;
            if attempt == 0 {
                work.nonzeros += nonzeros;
            }
            work.directed_operations += operations;
        }
        let mut max_state_radius = 0.0_f64;
        for i in 0..s {
            for v in 0..n {
                let state = upper_dot(&target.alpha_rows[i], &column(&bound, v, i))?;
                max_state_radius = max_state_radius.max(state);
            }
        }
        let closes = max_state_radius <= radius;
        attempts.push(RadiusAttempt {
            attempt,
            radius,
            closes,
            max_state_radius,
            reason: (!closes).then(|| "RADIUS_CLOSURE_FAIL".into()),
        });
        if closes {
            let certificate = finish_certificate(
                target,
                problem,
                candidate,
                y_hat,
                e_hat,
                bound,
                witness,
                atol,
                rtol,
                work.directed_operations,
            )?;
            return Ok(BlockedDoublingCertificate {
                doubling: DoublingCertificate {
                    certificate: Some(certificate),
                    attempts,
                    workers: execution.threads(),
                    pool_creations: 0,
                },
                work,
            });
        }
        radius = mul_up(radius, 4.0)?;
    }
    Ok(BlockedDoublingCertificate {
        doubling: DoublingCertificate {
            certificate: None,
            attempts,
            workers: execution.threads(),
            pool_creations: 0,
        },
        work,
    })
}

/// The entries of [`blocked_doubling_certificate_with_execution`] as a
/// [`crate::MajorantEntries`] (thread-transfer nodes P1-PATH-ACTION and
/// P1-RADIUS-PROPOSAL): a diagonal `J` and the structural diagonal witness,
/// seeds `a_(u,i) = U_uu |r_iu|` from the residual enclosure, and
/// `H_u(D)_ij = h U_uu (|J_uu| |L*_ij| + |q_u| (2 |delta_iu| + D) |alpha_ij|)`
/// with the blocked certificate's own operations in its own order, so a
/// constant radius in matrix order reproduces it bit for bit. Built from the
/// step's own inputs; nothing is cached across steps.
pub struct DiagonalMajorant<'a> {
    target: &'a StageTarget,
    problem: &'a QuadraticStageProblem,
    witness: &'a InverseWitness,
    increments: Vec<Vec<f64>>,
    /// `seeds[u][i]`.
    seeds: Vec<Vec<f64>>,
}

impl<'a> DiagonalMajorant<'a> {
    pub fn new(
        target: &'a StageTarget,
        problem: &'a QuadraticStageProblem,
        candidate: &[Vec<f64>],
        witness: &'a InverseWitness,
    ) -> CoreResult<Self> {
        validate_inputs(target, problem, candidate, witness)?;
        let n = problem.dimension();
        let diagonal_jacobian =
            (0..n).all(|a| (0..n).all(|b| a == b || problem.jacobian[a][b] == 0.0));
        if witness.identity.structure != "diagonal" || !diagonal_jacobian {
            return Err(CoreError::InvalidInput(format!(
                "{CERTIFICATE_STRUCTURE_UNSUPPORTED}: the diagonal majorant needs a diagonal J and the diagonal witness (got {:?})",
                witness.identity.structure
            )));
        }
        let s = target.stages();
        let (residuals, increments) = residual_enclosure(target, problem, candidate)?;
        let mut seeds = vec![vec![0.0; s]; n];
        for (i, residual) in residuals.iter().enumerate() {
            let magnitudes = residual.iter().map(Interval::mag).collect::<Vec<_>>();
            let applied = witness.apply_upper(&magnitudes)?;
            for u in 0..n {
                seeds[u][i] = applied[u];
            }
        }
        Ok(Self {
            target,
            problem,
            witness,
            increments,
            seeds,
        })
    }
}

impl crate::MajorantEntries for DiagonalMajorant<'_> {
    fn stages(&self) -> usize {
        self.target.stages()
    }
    fn components(&self) -> usize {
        self.problem.dimension()
    }
    fn seed(&self, component: usize, stage: usize) -> f64 {
        self.seeds[component][stage]
    }
    fn alpha_abs(&self, stage: usize, column: usize) -> f64 {
        self.target.alpha_rows[stage][column].abs()
    }
    fn coupling(
        &self,
        component: usize,
        stage: usize,
        column: usize,
        radius: f64,
    ) -> CoreResult<f64> {
        let u = component;
        let coupling = self.target.coupling_rows[stage][column].mag();
        let alpha = self.target.alpha_rows[stage][column].abs();
        let ell = mul_up(
            self.problem.q[u].abs(),
            add_up(mul_up(2.0, self.increments[stage][u])?, radius)?,
        )?;
        let mut term = mul_up(self.problem.jacobian[u][u].abs(), coupling)?;
        term = add_up(term, mul_up(ell, alpha)?)?;
        let inner = add_up(0.0, mul_up(self.witness.upper[u][u], term)?)?;
        mul_up(self.problem.h, inner)
    }
    fn coupling_operations(&self) -> u64 {
        8
    }
}

/// A certificate at one radius box, with its evaluation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoxCertificate {
    pub certificate: Option<StageCertificate>,
    pub evaluation: crate::BoxEvaluation,
}

/// The component-blocked certificate at a given radius box `D_(u,i)`
/// (thread-transfer node P1-RADIUS-PROPOSAL). The entries are rebuilt from
/// this step's inputs, the box is evaluated in full in `mode`, and the
/// certificate is finished only if every state radius closes. Where the box
/// came from does not matter: this evaluation alone decides.
#[allow(clippy::too_many_arguments)]
pub fn blocked_box_certificate_with_execution(
    target: &StageTarget,
    problem: &QuadraticStageProblem,
    candidate: &[Vec<f64>],
    y_hat: &[f64],
    e_hat: &[f64],
    witness: &InverseWitness,
    atol: f64,
    rtol: f64,
    radii: &crate::RadiusBox,
    mode: crate::PathEvaluation,
    execution: &crate::ParallelExecution,
) -> CoreResult<BoxCertificate> {
    let entries = DiagonalMajorant::new(target, problem, candidate, witness)?;
    let evaluation = crate::evaluate_radius_box(&entries, radii, mode, execution)?;
    let certificate = if evaluation.closes {
        Some(finish_certificate(
            target,
            problem,
            candidate,
            y_hat,
            e_hat,
            evaluation.bound.clone(),
            witness,
            atol,
            rtol,
            evaluation.work.directed_operations,
        )?)
    } else {
        None
    };
    Ok(BoxCertificate {
        certificate,
        evaluation,
    })
}

/// [`blocked_doubling_certificate_with_execution`] with the action-first
/// path sum `e <- e + Q e; Q <- Q^2` (thread-transfer node P1-PATH-ACTION):
/// the same arguments, entries, radius schedule (initial radius, times four
/// per failed attempt) and result type; only the evaluation order of
/// `sum_k H_u^k a_u` differs, so the bound is a separate upward enclosure,
/// not bit-identical to the matrix order. `work.allocated_values` and
/// `work.directed_operations` count block formation and path sums of every
/// attempt.
#[allow(clippy::too_many_arguments)]
pub fn blocked_action_doubling_certificate_with_execution(
    target: &StageTarget,
    problem: &QuadraticStageProblem,
    candidate: &[Vec<f64>],
    y_hat: &[f64],
    e_hat: &[f64],
    witness: &InverseWitness,
    atol: f64,
    rtol: f64,
    initial_radius: f64,
    max_attempts: usize,
    execution: &crate::ParallelExecution,
) -> CoreResult<BlockedDoublingCertificate> {
    let entries = DiagonalMajorant::new(target, problem, candidate, witness)?;
    if !(initial_radius.is_finite() && initial_radius >= 0.0) {
        return Err(CoreError::InvalidInput(format!(
            "CERTIFICATE_NOT_VALIDATED: initial state radius {initial_radius:e} must be finite and >= 0"
        )));
    }
    let (n, s) = (problem.dimension(), target.stages());
    let mut work = BlockedCertificateWork {
        components: n,
        block_size: s,
        levels: doubling_levels(s),
        ..BlockedCertificateWork::default()
    };
    let mut attempts = Vec::new();
    let mut radius = initial_radius;
    for attempt in 0..max_attempts.max(1) {
        let radii = crate::RadiusBox::common(n, s, radius)?;
        let evaluation = crate::evaluate_radius_box(
            &entries,
            &radii,
            crate::PathEvaluation::ActionFirst,
            execution,
        )?;
        work.allocated_values += evaluation.work.allocated_values;
        work.directed_operations += evaluation.work.directed_operations;
        if attempt == 0 {
            work.nonzeros = evaluation.nonzeros;
        }
        attempts.push(RadiusAttempt {
            attempt,
            radius,
            closes: evaluation.closes,
            max_state_radius: evaluation.max_state_radius,
            reason: (!evaluation.closes).then(|| "RADIUS_CLOSURE_FAIL".into()),
        });
        if evaluation.closes {
            let certificate = finish_certificate(
                target,
                problem,
                candidate,
                y_hat,
                e_hat,
                evaluation.bound,
                witness,
                atol,
                rtol,
                work.directed_operations,
            )?;
            return Ok(BlockedDoublingCertificate {
                doubling: DoublingCertificate {
                    certificate: Some(certificate),
                    attempts,
                    workers: execution.threads(),
                    pool_creations: 0,
                },
                work,
            });
        }
        radius = mul_up(radius, 4.0)?;
    }
    Ok(BlockedDoublingCertificate {
        doubling: DoublingCertificate {
            certificate: None,
            attempts,
            workers: execution.threads(),
            pool_creations: 0,
        },
        work,
    })
}
