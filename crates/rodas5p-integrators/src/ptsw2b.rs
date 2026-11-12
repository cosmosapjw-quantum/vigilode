//! PTSW2B: an opt-in, order-2, L-stable, two-stage parallel two-step W (Peer-W) CONTROL method.
//!
//! This module is reachable only as `rodas5p_integrators::ptsw2b`.  It is not a candidate in the
//! research catalog, not a router/default method, and not a G0 (order >= 4.8) candidate.
//!
//! # Method identity
//!
//! The tableau is Podhaisky (2002), Chapter 2 eq. (2.13)-(2.15), as reconstructed in
//! `docs/agent/solver_consolidation_20261108/PTSW2B_EXACT_TABLEAU_CERTIFICATE_20261109.md`
//! ("CERT"): `gamma = (3 - sqrt 3)/2`, `c = (3 - 2 sqrt 3, 1)`, `b = (0, gamma)`, with
//! step-ratio dependent `A(sigma)`, `Gamma(sigma)`, `v(sigma)` (CERT eq. 2.1-2.3).  One step is
//!
//! ```text
//! Y_i      = u_m + h sum_j A_ij(sigma) k_{m-1,j}
//! W k_i    = f(t_m + c_i h, Y_i) + h T sum_j Gamma_ij(sigma) k_{m-1,j},   W = I - h gamma T
//! u_{m+1}  = u_m + h sum_i ( b_i k_i + v_i(sigma) k_{m-1,i} ),            sigma = h / h_prev
//! ```
//!
//! The current-step stages depend only on previous-step data (`u_m`, `k_{m-1,*}`, `sigma`, `h`)
//! and share one matrix `W`; they are therefore independent.  The implementation uses the exact
//! rearrangement of CERT eq. (2.5): `W x_i = f_i + p_i`, `k_i = x_i - p_i`, with
//! `p_i = sum_j Gamma_ij/gamma k_{m-1,j}`, so no extra `T` product is needed on the right side.
//!
//! The embedded estimator (`b_e = 0.95 b`, `v_e(sigma)` of CERT eq. 5.1) and the controller
//! (`sigma* = min{2, max{0.5, 0.85 Err^(-1/2)}}` with hysteresis, CERT sec. 5) are this method's
//! own.  Nothing is shared with the RODAS5P estimator or controller.
//!
//! # Derivative permission (owner-approved class)
//!
//! [`DerivativeClass::ExactJacobianOrJvpFrozenAtStepStart`]: `T = J(t_m, u_m)` is obtained once
//! per step through [`OdeProblem::dense_jacobian`] (an analytic Jacobian, or `n` exact JVP calls,
//! both charged), and one dense LU of `W` serves both stages.  This is derivative use, not
//! derivative-free.  Common `W` does not imply a shared Krylov recurrence and none is claimed;
//! there is no matrix-free route here.  Identity mass matrices only.
//!
//! # Startup (owner-approved option S5)
//!
//! [`start`] is a self-contained starter inside this module: one step of the two-stage ROS2
//! Rosenbrock method (Verwer et al.) over `[t0, t1]`, `t1 = t0 + (2 sqrt 3 - 2) h0`, giving
//! `u1` and the peer derivatives `k_{0,1} = f(t0, u0)`, `k_{0,2} = f(t1, u1)` (CERT sec. 4).
//! It calls neither RODAS5P nor DOPRI5.  Behaviour for `t_end < t1`, events inside the startup
//! window and the starter's cost model beyond the counters it charges are UNRESOLVED.
//!
//! # Coefficients
//!
//! The exact source of truth is the algebraic form in Q(sqrt 3).  Runtime `f64` constants are
//! recomputed from integer data `(p + q sqrt 3)/d` by [`round_alg_checked`]: `sqrt 3` is bracketed
//! by an exact integer square root at scale `2^62`, the value is bracketed by integer division,
//! and the result is the correctly rounded `f64` provided both bracket ends round identically
//! (checked in the unit tests for every constant).
//!
//! # Work accounting
//!
//! Charges follow `sequential.rs` conventions and are made before user code or factorization is
//! entered, so failed attempts remain visible: per step one Jacobian provider call
//! (`jacobian_builds`, plus `jvp_calls` for a JVP-only problem), one `direct_factorizations`, two
//! `rhs_calls`/`rhs_evaluations`, two `linear_solves` and `direct_solve_calls`.
//!
//! # Not established
//!
//! Admissible step-ratio domain beyond the controller clip `[0.5, 2]` (UNRESOLVED), a full proof
//! of the L(90) sector (source-reported only), and any real-problem benchmark (NOT_RUN).

use std::sync::OnceLock;

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, LuFactorization, WorkCounters};

use crate::{OdeProblem, ParallelExecution};

/// Stable identifier of this method; distinct from PTSW2C, RODAS5P and the catalog's `peer-w`.
pub const METHOD_ID: &str = "ptsw2b-peer-w-order2-lstable-control";
/// Number of peer stages.
pub const STAGES: usize = 2;
/// Formal (global) order of the method.
pub const FORMAL_ORDER: usize = 2;
/// Owner-decided ceiling on accepted steps relative to protected RODAS5P (PW02 promotion gate).
pub const ACCEPTED_STEP_CEILING_VS_RODAS5P: f64 = 1.10;
/// Controller safety factor (CERT sec. 5).
pub const SAFETY: f64 = 0.85;
/// Clip of the proposed step ratio (CERT sec. 5).
pub const RATIO_CLIP: (f64, f64) = (0.5, 2.0);
/// Ratios in this closed band keep the step size unchanged (CERT sec. 5).
pub const HYSTERESIS_BAND: (f64, f64) = (0.95, 1.05);
/// Number of most recent applied ratios inspected for growth hysteresis (`s = 2`).
pub const HYSTERESIS_HISTORY: usize = 2;

// ---------------------------------------------------------------------------------------
// Exact algebraic constants and their correctly rounded f64 values
// ---------------------------------------------------------------------------------------

/// The algebraic number `(p + q sqrt 3) / d` with `d > 0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Alg {
    /// Rational part numerator.
    pub p: i128,
    /// Coefficient of `sqrt 3` (numerator).
    pub q: i128,
    /// Common positive denominator.
    pub d: i128,
}

/// `floor(sqrt(n))` by Newton iteration.
fn isqrt_u128(n: u128) -> u128 {
    if n < 2 {
        return n;
    }
    let mut x = 1u128 << (128 - n.leading_zeros()).div_ceil(2);
    loop {
        let y = (x + n / x) / 2;
        if y >= x {
            return x;
        }
        x = y;
    }
}

/// Correctly rounded `f64` of `(p + q sqrt 3)/d`, or `None` if the integer brackets disagree.
///
/// With `r = floor(sqrt 3 * 2^62)` (exact integer square root of `3 * 2^124`) the true value
/// lies in `[N_lo, N_hi] / (d 2^62)`.  Both integer quotients (floor and ceil) are converted with
/// the round-to-nearest integer-to-float conversion; equal results certify correct rounding.
pub fn round_alg_checked(a: Alg) -> Option<f64> {
    if a.d <= 0 {
        return None;
    }
    let r = isqrt_u128(3u128 << 124) as i128;
    let scale = 1i128 << 62;
    let (q_lo, q_hi) = if a.q >= 0 {
        (a.q * r, a.q * (r + 1))
    } else {
        (a.q * (r + 1), a.q * r)
    };
    let base = a.p * scale;
    let m_lo = (base + q_lo).div_euclid(a.d);
    let m_hi = (base + q_hi + a.d - 1).div_euclid(a.d);
    let (f_lo, f_hi) = (m_lo as f64, m_hi as f64);
    if f_lo == f_hi {
        Some(f_lo / (scale as f64))
    } else {
        None
    }
}

/// Runtime value: correctly rounded when certified, else the midpoint of the two bracket ends.
fn alg(p: i128, q: i128, d: i128) -> f64 {
    let a = Alg { p, q, d };
    match round_alg_checked(a) {
        Some(v) => v,
        None => (p as f64 + q as f64 * 3.0_f64.sqrt()) / d as f64,
    }
}

/// Affine-in-sigma coefficient `c0 + c1 sigma`.
#[derive(Clone, Copy)]
struct Affine {
    c0: f64,
    c1: f64,
}
impl Affine {
    fn at(self, sigma: f64) -> f64 {
        self.c0 + self.c1 * sigma
    }
}

struct Tables {
    gamma: f64,
    c: [f64; 2],
    b: [f64; 2],
    b_emb: [f64; 2],
    a: [[Affine; 2]; 2],
    g: [[Affine; 2]; 2],
    v: [Affine; 2],
    v_emb: [Affine; 2],
}

/// The certified integer data of every constant; `(p, q, d)` per CERT eq. (2.1)-(2.3), (5.1).
fn build_tables() -> Tables {
    let aff = |c0: (i128, i128, i128), c1: (i128, i128, i128)| Affine {
        c0: alg(c0.0, c0.1, c0.2),
        c1: alg(c1.0, c1.1, c1.2),
    };
    let z = (0, 0, 1);
    Tables {
        gamma: alg(3, -1, 2),
        c: [alg(3, -2, 1), 1.0],
        b: [0.0, alg(3, -1, 2)],
        b_emb: [0.0, alg(57, -19, 40)],
        a: [
            [aff(z, (15, -9, 8)), aff((24, -16, 8), (-15, 9, 8))],
            [aff(z, (-1, -1, 8)), aff((8, 0, 8), (1, 1, 8))],
        ],
        g: [
            [aff(z, (-6, 3, 4)), aff((-6, 2, 4), (6, -3, 4))],
            [aff(z, (0, 1, 4)), aff((-6, 2, 4), (0, -1, 4))],
        ],
        v: [aff(z, (-1, 1, 8)), aff((-4, 4, 8), (1, -1, 8))],
        v_emb: [aff(z, (-11, 8, 80)), aff((-17, 19, 40), (11, -8, 80))],
    }
}

fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(build_tables)
}

/// All `(p, q, d)` constants of [`build_tables`], for certification by the unit tests.
#[cfg(test)]
fn all_algebraic_constants() -> Vec<Alg> {
    let l: [(i128, i128, i128); 25] = [
        (3, -1, 2),
        (3, -2, 1),
        (57, -19, 40),
        (15, -9, 8),
        (24, -16, 8),
        (-15, 9, 8),
        (-1, -1, 8),
        (8, 0, 8),
        (1, 1, 8),
        (-6, 3, 4),
        (-6, 2, 4),
        (6, -3, 4),
        (0, 1, 4),
        (0, -1, 4),
        (-1, 1, 8),
        (-4, 4, 8),
        (1, -1, 8),
        (-11, 8, 80),
        (-17, 19, 40),
        (11, -8, 80),
        (0, 0, 1),
        (1, 0, 1),
        (3, 1, 3),
        (-3, 1, 7),
        (5, -7, 11),
    ];
    l.iter().map(|&(p, q, d)| Alg { p, q, d }).collect()
}

/// `gamma = (3 - sqrt 3)/2`.
pub fn gamma() -> f64 {
    tables().gamma
}
/// Stage nodes `c = (3 - 2 sqrt 3, 1)`; the first node is extrapolatory (`< 0`).
pub fn nodes() -> [f64; 2] {
    tables().c
}
/// Output weights `b = (0, gamma)`.
pub fn b() -> [f64; 2] {
    tables().b
}
/// `A(sigma)` of CERT eq. (2.1).
pub fn a_matrix(sigma: f64) -> [[f64; 2]; 2] {
    let t = &tables().a;
    [
        [t[0][0].at(sigma), t[0][1].at(sigma)],
        [t[1][0].at(sigma), t[1][1].at(sigma)],
    ]
}
/// `Gamma(sigma)` of CERT eq. (2.2).
pub fn gamma_matrix(sigma: f64) -> [[f64; 2]; 2] {
    let t = &tables().g;
    [
        [t[0][0].at(sigma), t[0][1].at(sigma)],
        [t[1][0].at(sigma), t[1][1].at(sigma)],
    ]
}
/// `v(sigma)` of CERT eq. (2.3).
pub fn v(sigma: f64) -> [f64; 2] {
    let t = &tables().v;
    [t[0].at(sigma), t[1].at(sigma)]
}
/// Embedded output weights `b_e = 0.95 b` (CERT sec. 5).
pub fn b_embedded() -> [f64; 2] {
    tables().b_emb
}
/// Embedded history weights `v_e(sigma)` of CERT eq. (5.1).
pub fn v_embedded(sigma: f64) -> [f64; 2] {
    let t = &tables().v_emb;
    [t[0].at(sigma), t[1].at(sigma)]
}

/// Constant-step stiff-limit amplification matrix `M(infinity)` (CERT eq. 3.1, `sigma = 1`).
/// It is nilpotent of index three, so its spectral radius is zero.
pub fn stiff_limit_matrix() -> [[f64; 3]; 3] {
    let (a, g, bw, vw) = (a_matrix(1.0), gamma_matrix(1.0), b(), v(1.0));
    let w = -1.0 / gamma();
    let mut beta = [[0.0; 2]; 2];
    for ((row, ar), gr) in beta.iter_mut().zip(&a).zip(&g) {
        for ((x, av), gv) in row.iter_mut().zip(ar).zip(gr) {
            *x = av + gv;
        }
    }
    let mut m = [[0.0; 3]; 3];
    for (mrow, brow) in m.iter_mut().zip(&beta) {
        mrow[0] = w * brow[0];
        mrow[1] = w * brow[1];
        mrow[2] = w;
    }
    for j in 0..2 {
        m[2][j] = bw[0] * w * beta[0][j] + bw[1] * w * beta[1][j] + vw[j];
    }
    m[2][2] = 1.0 + (bw[0] + bw[1]) * w;
    m
}

// ---------------------------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------------------------

/// Derivative-permission class of the method's operator `T`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DerivativeClass {
    /// `T = J(t_m, u_m)` from an analytic Jacobian or exact JVP, frozen at the step start.
    ExactJacobianOrJvpFrozenAtStepStart,
}

/// Machine-readable method identity and claim ceilings.
#[derive(Clone, Debug)]
pub struct Identity {
    /// See [`METHOD_ID`].
    pub method_id: &'static str,
    /// Formal order.
    pub order: usize,
    /// Peer stages.
    pub stages: usize,
    /// Identifier of this method's own embedded estimator.
    pub estimator_id: &'static str,
    /// Identifier of this method's own step controller.
    pub controller_id: &'static str,
    /// Identifier of the self-contained starter (owner option S5).
    pub startup_id: &'static str,
    /// Accepted-step ceiling versus protected RODAS5P (owner decision).
    pub accepted_step_ceiling_vs_rodas5p: f64,
    /// Derivative-permission class.
    pub derivative_class: DerivativeClass,
    /// Whether any default router selects this method (always `false`).
    pub default_routed: bool,
    /// Whether the RODAS5P estimator weights are used (always `false`).
    pub reuses_rodas5p_estimator: bool,
    /// Whether the RODAS5P controller is used (always `false`).
    pub reuses_rodas5p_controller: bool,
    /// Whether a shared Krylov recurrence is claimed (always `false`).
    pub claims_shared_krylov_recurrence: bool,
    /// Whether G0 qualification is claimed (always `false`).
    pub claims_g0_qualified: bool,
    /// Whether the method is derivative-free (always `false`).
    pub claims_derivative_free: bool,
}

/// The method identity.
pub fn identity() -> Identity {
    Identity {
        method_id: METHOD_ID,
        order: FORMAL_ORDER,
        stages: STAGES,
        estimator_id: "ptsw2b-embedded-order1-cert-eq5.1",
        controller_id: "ptsw2b-podhaisky-hysteresis-controller",
        startup_id: "ptsw2b-ros2-self-contained-starter",
        accepted_step_ceiling_vs_rodas5p: ACCEPTED_STEP_CEILING_VS_RODAS5P,
        derivative_class: DerivativeClass::ExactJacobianOrJvpFrozenAtStepStart,
        default_routed: false,
        reuses_rodas5p_estimator: false,
        reuses_rodas5p_controller: false,
        claims_shared_krylov_recurrence: false,
        claims_g0_qualified: false,
        claims_derivative_free: false,
    }
}

// ---------------------------------------------------------------------------------------
// Startup
// ---------------------------------------------------------------------------------------

/// Forward-only startup times (CERT sec. 4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StartupPlan {
    /// First PTSW step start `t1 = t0 + (2 sqrt 3 - 2) h0`.
    pub t1: f64,
    /// Times at which the peer derivatives `k_{0,i}` are evaluated: `[t0, t1]`.
    pub derivative_times: [f64; 2],
    /// Step ratio of the first PTSW step.
    pub first_ratio: f64,
}

/// The startup plan for initial time `t0` and first step `h0`.
pub fn startup_plan(t0: f64, h0: f64) -> StartupPlan {
    let dt = alg(-2, 2, 1); // 2 sqrt 3 - 2
    let t1 = t0 + dt * h0;
    StartupPlan {
        t1,
        derivative_times: [t0, t1],
        first_ratio: 1.0,
    }
}

// ---------------------------------------------------------------------------------------
// State, stage solves, one step
// ---------------------------------------------------------------------------------------

/// Committed two-step history: the state at `t`, the previous step's peer derivatives and its size.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    /// Current time `t_m`.
    pub t: f64,
    /// Solution `u_m`.
    pub u: Vec<f64>,
    /// Peer derivatives `k_{m-1,i}` of the previous step.
    pub k_prev: [Vec<f64>; 2],
    /// Previous step size `h_{m-1}`.
    pub h_prev: f64,
}

/// Right-hand-side data of one stage, depending only on previous-step data.
#[derive(Clone, Debug, PartialEq)]
pub struct StageInput {
    /// Stage time `t_m + c_i h`.
    pub t: f64,
    /// Stage argument `Y_i`.
    pub y: Vec<f64>,
    /// `p_i = sum_j Gamma_ij/gamma k_{m-1,j}` (CERT eq. 2.5).
    pub p: Vec<f64>,
}

/// Solutions of the two stage systems: `W x_i = f_i + p_i`, `k_i = x_i - p_i`.
#[derive(Clone, Debug, PartialEq)]
pub struct StageSolve {
    /// `x_i`.
    pub x: [Vec<f64>; 2],
    /// `k_i`.
    pub k: [Vec<f64>; 2],
}

fn at(v: &[f64], r: usize) -> f64 {
    // Out-of-range components poison the result with NaN; callers validate lengths and finiteness.
    v.get(r).copied().unwrap_or(f64::NAN)
}

/// Stage inputs for step size `h` and ratio `sigma` from previous-step data only.
///
/// A length mismatch between `u` and `k_prev` produces NaN entries (never a panic); the checked
/// entry points ([`advance`], [`solve_stages`]) reject such inputs with a typed error.
pub fn stage_inputs(
    t_m: f64,
    u: &[f64],
    k_prev: &[Vec<f64>; 2],
    h: f64,
    sigma: f64,
) -> [StageInput; 2] {
    let (a, gm, g, c) = (a_matrix(sigma), gamma_matrix(sigma), gamma(), nodes());
    let build = |i: usize| {
        let y = (0..u.len())
            .map(|r| u[r] + h * (a[i][0] * at(&k_prev[0], r) + a[i][1] * at(&k_prev[1], r)))
            .collect();
        let p = (0..u.len())
            .map(|r| (gm[i][0] * at(&k_prev[0], r) + gm[i][1] * at(&k_prev[1], r)) / g)
            .collect();
        StageInput {
            t: t_m + c[i] * h,
            y,
            p,
        }
    };
    [build(0), build(1)]
}

fn check_positive(name: &str, x: f64) -> CoreResult<()> {
    if x.is_finite() && x > 0.0 {
        Ok(())
    } else {
        Err(CoreError::InvalidInput(format!(
            "{name} must be finite and positive"
        )))
    }
}

fn check_problem(problem: &OdeProblem) -> CoreResult<()> {
    if problem.mass_matrix.is_some() {
        return Err(CoreError::InvalidInput(
            "PTSW2B supports identity mass matrices only".into(),
        ));
    }
    Ok(())
}

/// Solve both stage systems with one common `W = I - h gamma J(t_m, u_m)` and one LU factorization.
///
/// The two solves are independent and run through `exec` in stage order.  Work is charged before
/// entering user code or factorization.
pub fn solve_stages(
    problem: &OdeProblem,
    t_m: f64,
    u_m: &[f64],
    h: f64,
    inputs: &[StageInput; 2],
    exec: &ParallelExecution,
    counters: &mut WorkCounters,
) -> CoreResult<StageSolve> {
    check_problem(problem)?;
    check_positive("step size", h)?;
    let n = problem.dimension;
    if u_m.len() != n || inputs.iter().any(|s| s.y.len() != n || s.p.len() != n) {
        return Err(CoreError::Dimension("PTSW2B stage shape mismatch".into()));
    }
    let jac = problem.dense_jacobian(t_m, u_m, counters)?;
    let shift = h * gamma();
    let mut w = DenseMatrix::identity(n);
    for i in 0..n {
        for j in 0..n {
            w[(i, j)] -= shift * jac[(i, j)];
        }
    }
    counters.direct_factorizations += 1;
    let lu = LuFactorization::new(&w)?;
    let mut rhs: Vec<Vec<f64>> = Vec::with_capacity(2);
    for input in inputs {
        let f = problem.eval_rhs(input.t, &input.y, counters)?;
        rhs.push(f.iter().zip(&input.p).map(|(a, b)| a + b).collect());
    }
    counters.linear_solves += 2;
    counters.direct_solve_calls += 2;
    let idx = [0usize, 1];
    let xs = exec.map_ordered(&idx, |&i| lu.solve(&rhs[i]))?;
    let mut it = xs.into_iter();
    let (x0, x1) = match (it.next(), it.next()) {
        (Some(a), Some(b)) => (a, b),
        _ => {
            return Err(CoreError::LinearSolve(
                "PTSW2B stage solve returned fewer than two solutions".into(),
            ));
        }
    };
    let k0 = x0.iter().zip(&inputs[0].p).map(|(x, p)| x - p).collect();
    let k1 = x1.iter().zip(&inputs[1].p).map(|(x, p)| x - p).collect();
    Ok(StageSolve {
        x: [x0, x1],
        k: [k0, k1],
    })
}

/// Embedded error vector `u_{m+1} - u~_{m+1} = h sum_j[(b_j - b_e,j) k_j + (v_j - v_e,j) k_{m-1,j}]`.
pub fn embedded_error_vector(
    h: f64,
    sigma: f64,
    k: &[Vec<f64>; 2],
    k_prev: &[Vec<f64>; 2],
) -> Vec<f64> {
    let (bw, be, vw, ve) = (b(), b_embedded(), v(sigma), v_embedded(sigma));
    (0..k[0].len())
        .map(|r| {
            h * (0..2)
                .map(|j| (bw[j] - be[j]) * at(&k[j], r) + (vw[j] - ve[j]) * at(&k_prev[j], r))
                .sum::<f64>()
        })
        .collect()
}

/// Componentwise scaled RMS norm `sqrt(mean((e_i / (atol + rtol |u_i|))^2))` (CERT sec. 5).
///
/// Returns NaN for empty or length-mismatched input and for a non-positive scale; NaN is never
/// accepted by [`accepts`].
pub fn error_norm(u: &[f64], e: &[f64], atol: f64, rtol: f64) -> f64 {
    if u.is_empty() || u.len() != e.len() {
        return f64::NAN;
    }
    let mut sum = 0.0;
    for (ui, ei) in u.iter().zip(e) {
        let scale = atol + rtol * ui.abs();
        if !(scale > 0.0 && scale.is_finite()) {
            return f64::NAN;
        }
        sum += (ei / scale).powi(2);
    }
    (sum / u.len() as f64).sqrt()
}

/// A step is accepted iff its error norm is `<= 1` (NaN is rejected).
pub fn accepts(err: f64) -> bool {
    err <= 1.0
}

/// Proposed ratio `h*/h` after clip and hysteresis (CERT sec. 5).
///
/// `last_ratios` holds the applied ratios, most recent last; only the last
/// [`HYSTERESIS_HISTORY`] are inspected.  A non-finite `err` proposes the lower clip.
pub fn propose_ratio(err: f64, last_ratios: &[f64]) -> f64 {
    if err.is_nan() {
        return RATIO_CLIP.0;
    }
    let raw = if err <= 0.0 {
        RATIO_CLIP.1
    } else {
        SAFETY * err.powf(-0.5)
    };
    let raw = raw.clamp(RATIO_CLIP.0, RATIO_CLIP.1);
    if (HYSTERESIS_BAND.0..=HYSTERESIS_BAND.1).contains(&raw) {
        return 1.0;
    }
    let tail = &last_ratios[last_ratios.len().saturating_sub(HYSTERESIS_HISTORY)..];
    if raw > HYSTERESIS_BAND.1 && tail.iter().any(|&r| r > 1.0) {
        return 1.0;
    }
    raw
}

/// Result of one (not yet committed) step.
#[derive(Clone, Debug, PartialEq)]
pub struct StepOutput {
    /// The state after the step.
    pub next: State,
    /// Embedded difference `u_{m+1} - u~_{m+1}`.
    pub error_vector: Vec<f64>,
}

fn all_finite(v: &[f64]) -> bool {
    v.iter().all(|x| x.is_finite())
}

/// One PTSW2B step from `state` with size `h`; `state` is not modified.
pub fn advance(
    problem: &OdeProblem,
    state: &State,
    h: f64,
    exec: &ParallelExecution,
    counters: &mut WorkCounters,
) -> CoreResult<StepOutput> {
    check_problem(problem)?;
    check_positive("step size", h)?;
    check_positive("previous step size", state.h_prev)?;
    let n = problem.dimension;
    if state.u.len() != n || state.k_prev.iter().any(|k| k.len() != n) {
        return Err(CoreError::Dimension("PTSW2B state shape mismatch".into()));
    }
    if !(state.t.is_finite() && all_finite(&state.u) && state.k_prev.iter().all(|k| all_finite(k)))
    {
        return Err(CoreError::NonFinite("PTSW2B state is not finite".into()));
    }
    let sigma = h / state.h_prev;
    check_positive("step ratio", sigma)?;
    let inputs = stage_inputs(state.t, &state.u, &state.k_prev, h, sigma);
    let stages = solve_stages(problem, state.t, &state.u, h, &inputs, exec, counters)?;
    let (bw, vw) = (b(), v(sigma));
    let u: Vec<f64> = (0..n)
        .map(|r| {
            state.u[r]
                + h * (0..2)
                    .map(|j| bw[j] * stages.k[j][r] + vw[j] * state.k_prev[j][r])
                    .sum::<f64>()
        })
        .collect();
    let error_vector = embedded_error_vector(h, sigma, &stages.k, &state.k_prev);
    if !(all_finite(&u) && all_finite(&error_vector)) {
        return Err(CoreError::NonFinite("PTSW2B step produced NaN/Inf".into()));
    }
    Ok(StepOutput {
        next: State {
            t: state.t + h,
            u,
            k_prev: stages.k,
            h_prev: h,
        },
        error_vector,
    })
}

// ---------------------------------------------------------------------------------------
// Self-contained starter (owner option S5): one ROS2 step
// ---------------------------------------------------------------------------------------

/// Build the first PTSW2B state at `t1 = t0 + (2 sqrt 3 - 2) h0` from `(t0, y0)`.
///
/// One step of the two-stage ROS2 Rosenbrock method (`gamma_r = 1 + 1/sqrt 2`, order 2, no time
/// derivative needed) advances `y0` to `u1` over `[t0, t1]`; the peer derivatives are
/// `k_{0,1} = f(t0, y0)` and `k_{0,2} = f(t1, u1)`, and `h_prev = h0` (first ratio 1).  Charged
/// work: 1 Jacobian provider call, 1 factorization, 3 RHS calls, 2 linear solves.
pub fn start(
    problem: &OdeProblem,
    t0: f64,
    y0: &[f64],
    h0: f64,
    counters: &mut WorkCounters,
) -> CoreResult<State> {
    check_problem(problem)?;
    check_positive("first step size", h0)?;
    let n = problem.dimension;
    if y0.len() != n || !t0.is_finite() || !all_finite(y0) {
        return Err(CoreError::InvalidInput(
            "PTSW2B start needs a finite state of the problem dimension".into(),
        ));
    }
    let plan = startup_plan(t0, h0);
    let delta = plan.t1 - t0;
    let gamma_r = 1.0 + std::f64::consts::FRAC_1_SQRT_2;
    let jac = problem.dense_jacobian(t0, y0, counters)?;
    let mut w = DenseMatrix::identity(n);
    for i in 0..n {
        for j in 0..n {
            w[(i, j)] -= gamma_r * delta * jac[(i, j)];
        }
    }
    counters.direct_factorizations += 1;
    let lu = LuFactorization::new(&w)?;
    let f0 = problem.eval_rhs(t0, y0, counters)?;
    counters.linear_solves += 1;
    counters.direct_solve_calls += 1;
    let k1 = lu.solve(&f0)?;
    let y_stage: Vec<f64> = y0.iter().zip(&k1).map(|(y, k)| y + delta * k).collect();
    let f1 = problem.eval_rhs(t0 + delta, &y_stage, counters)?;
    let rhs2: Vec<f64> = f1.iter().zip(&k1).map(|(f, k)| f - 2.0 * k).collect();
    counters.linear_solves += 1;
    counters.direct_solve_calls += 1;
    let k2 = lu.solve(&rhs2)?;
    let u1: Vec<f64> = (0..n)
        .map(|r| y0[r] + delta * (1.5 * k1[r] + 0.5 * k2[r]))
        .collect();
    if !all_finite(&u1) {
        return Err(CoreError::NonFinite(
            "PTSW2B starter produced NaN/Inf".into(),
        ));
    }
    let f_t1 = problem.eval_rhs(plan.t1, &u1, counters)?;
    Ok(State {
        t: plan.t1,
        u: u1,
        k_prev: [f0, f_t1],
        h_prev: h0,
    })
}

// ---------------------------------------------------------------------------------------
// Transactional session
// ---------------------------------------------------------------------------------------

/// Tolerances of the method's own error indicator.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    /// Absolute tolerance (`>= 0`).
    pub atol: f64,
    /// Relative tolerance (`>= 0`); `atol + rtol` must be positive.
    pub rtol: f64,
}

/// Controller history committed only on accepted steps.
#[derive(Clone, Debug, PartialEq)]
pub struct ControllerState {
    /// The most recent applied step ratios (at most [`HYSTERESIS_HISTORY`]), most recent last.
    pub last_ratios: Vec<f64>,
}

/// Outcome of one attempt.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Attempt {
    /// The step was accepted and committed.
    Accepted {
        /// Embedded error norm of the step.
        error_norm: f64,
        /// Proposed ratio for the next step.
        next_ratio: f64,
    },
    /// The step was rejected; nothing was committed.
    Rejected {
        /// Embedded error norm of the step.
        error_norm: f64,
        /// Proposed ratio for the retry.
        next_ratio: f64,
    },
}

/// A transactional PTSW2B integration: attempts either commit history and controller state
/// atomically or leave them untouched, while all attempted work is charged.
pub struct Session<'a> {
    problem: &'a OdeProblem,
    state: State,
    config: Config,
    exec: ParallelExecution,
    controller: ControllerState,
    counters: WorkCounters,
}

impl<'a> Session<'a> {
    /// Create a session from committed history `state`.
    pub fn new(
        problem: &'a OdeProblem,
        state: State,
        config: Config,
        exec: ParallelExecution,
    ) -> CoreResult<Self> {
        check_problem(problem)?;
        if !(config.atol.is_finite()
            && config.rtol.is_finite()
            && config.atol >= 0.0
            && config.rtol >= 0.0
            && config.atol + config.rtol > 0.0)
        {
            return Err(CoreError::InvalidInput(
                "tolerances must be finite, nonnegative and not both zero".into(),
            ));
        }
        let n = problem.dimension;
        if state.u.len() != n || state.k_prev.iter().any(|k| k.len() != n) {
            return Err(CoreError::Dimension("PTSW2B state shape mismatch".into()));
        }
        Ok(Self {
            problem,
            state,
            config,
            exec,
            controller: ControllerState {
                last_ratios: Vec::new(),
            },
            counters: WorkCounters::default(),
        })
    }

    /// The committed history.
    pub fn state(&self) -> &State {
        &self.state
    }

    /// The committed controller state.
    pub fn controller(&self) -> &ControllerState {
        &self.controller
    }

    /// Cumulative work of every attempt, accepted or not.
    pub fn counters(&self) -> WorkCounters {
        self.counters
    }

    /// Attempt one step of size `h`.
    ///
    /// On rejection or error the committed state and controller are bit-identical to before; the
    /// work counters keep everything that was attempted (`rejected_steps` is incremented on a
    /// rejection).
    pub fn attempt(&mut self, h: f64) -> CoreResult<Attempt> {
        let mut work = self.counters;
        let result = advance(self.problem, &self.state, h, &self.exec, &mut work);
        self.counters = work;
        let out = result?;
        let err = error_norm(
            &out.next.u,
            &out.error_vector,
            self.config.atol,
            self.config.rtol,
        );
        if accepts(err) {
            let sigma = h / self.state.h_prev;
            let mut ratios = self.controller.last_ratios.clone();
            ratios.push(sigma);
            if ratios.len() > HYSTERESIS_HISTORY {
                ratios.drain(..ratios.len() - HYSTERESIS_HISTORY);
            }
            let next_ratio = propose_ratio(err, &ratios);
            self.controller.last_ratios = ratios;
            self.state = out.next;
            self.counters.accepted_steps += 1;
            Ok(Attempt::Accepted {
                error_norm: err,
                next_ratio,
            })
        } else {
            self.counters.rejected_steps += 1;
            Ok(Attempt::Rejected {
                error_norm: err,
                next_ratio: propose_ratio(err, &self.controller.last_ratios),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn isqrt_is_floor_sqrt() {
        for n in [0u128, 1, 2, 3, 4, 15, 16, 17, 1 << 100, (1u128 << 100) + 1] {
            let r = isqrt_u128(n);
            assert!(r * r <= n && (r + 1) * (r + 1) > n, "n={n}");
        }
        let r = isqrt_u128(3u128 << 124);
        // floor(sqrt(3) * 2^62), checked independently with Python math.isqrt.
        assert_eq!(r, 7_987_674_492_471_257_550);
    }

    #[test]
    fn every_runtime_constant_is_certified_correctly_rounded() {
        for a in all_algebraic_constants() {
            let exact = round_alg_checked(a).unwrap_or_else(|| panic!("ambiguous bracket: {a:?}"));
            // independent sanity: within a few ulps of the naive evaluation
            let naive = (a.p as f64 + a.q as f64 * 3.0_f64.sqrt()) / a.d as f64;
            assert!(
                (exact - naive).abs() <= 8.0 * f64::EPSILON * (1.0 + naive.abs()),
                "{a:?}"
            );
            assert_eq!(alg(a.p, a.q, a.d), exact);
        }
    }

    #[test]
    fn runtime_tableau_equals_the_correctly_rounded_exact_values() {
        let cr = |p, q, d| round_alg_checked(Alg { p, q, d }).unwrap();
        assert_eq!(gamma(), cr(3, -1, 2));
        assert_eq!(nodes(), [cr(3, -2, 1), 1.0]);
        assert_eq!(b(), [0.0, cr(3, -1, 2)]);
        assert_eq!(b_embedded(), [0.0, cr(57, -19, 40)]);
        let t = tables();
        assert_eq!(t.a[0][0].c1, cr(15, -9, 8));
        assert_eq!(t.a[0][1].c0, cr(24, -16, 8));
        assert_eq!(t.a[0][1].c1, cr(-15, 9, 8));
        assert_eq!(t.a[1][0].c1, cr(-1, -1, 8));
        assert_eq!((t.a[1][1].c0, t.a[1][1].c1), (1.0, cr(1, 1, 8)));
        assert_eq!(t.g[0][0].c1, cr(-6, 3, 4));
        assert_eq!((t.g[0][1].c0, t.g[0][1].c1), (cr(-6, 2, 4), cr(6, -3, 4)));
        assert_eq!(t.g[1][0].c1, cr(0, 1, 4));
        assert_eq!((t.g[1][1].c0, t.g[1][1].c1), (cr(-6, 2, 4), cr(0, -1, 4)));
        assert_eq!((t.v[1].c0, t.v[1].c1), (cr(-4, 4, 8), cr(1, -1, 8)));
        assert_eq!(t.v[0].c1, cr(-1, 1, 8));
        assert_eq!(t.v_emb[0].c1, cr(-11, 8, 80));
        assert_eq!(
            (t.v_emb[1].c0, t.v_emb[1].c1),
            (cr(-17, 19, 40), cr(11, -8, 80))
        );
    }

    fn scalar(lambda: f64) -> OdeProblem {
        OdeProblem::new(
            "ptsw2b-unit-scalar",
            1,
            Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
                out[0] = lambda * y[0] + t.cos();
                Ok(())
            }),
            None,
            Some(Arc::new(move |_t: f64, _y: &[f64]| {
                DenseMatrix::from_vec_rows(vec![vec![lambda]])
            })),
            None,
            None,
            false,
            None,
            None,
        )
        .unwrap()
    }

    #[test]
    fn starter_has_local_order_three_hence_u1_error_is_o_h_squared() {
        // y' = -y + cos t, y(0) = 0: y = (cos t + sin t - e^{-t})/2.
        let exact = |t: f64| 0.5 * (t.cos() + t.sin() - (-t).exp());
        let problem = scalar(-1.0);
        let mut errs = Vec::new();
        for h0 in [0.1, 0.05, 0.025] {
            let mut w = WorkCounters::default();
            let s = start(&problem, 0.0, &[0.0], h0, &mut w).unwrap();
            errs.push((s.u[0] - exact(s.t)).abs());
            assert_eq!(
                (
                    w.jacobian_builds,
                    w.direct_factorizations,
                    w.rhs_calls,
                    w.linear_solves
                ),
                (1, 1, 3, 2)
            );
        }
        for pair in errs.windows(2) {
            let order = (pair[0] / pair[1]).log2();
            assert!(order > 2.5, "starter local order {order}: {errs:?}");
        }
    }

    #[test]
    fn bad_input_gives_typed_errors_not_panics() {
        let problem = scalar(-1.0);
        let exec = ParallelExecution::sequential();
        let mut w = WorkCounters::default();
        let st = State {
            t: 0.0,
            u: vec![1.0],
            k_prev: [vec![1.0], vec![1.0]],
            h_prev: 0.1,
        };
        assert!(advance(&problem, &st, 0.0, &exec, &mut w).is_err());
        assert!(advance(&problem, &st, f64::NAN, &exec, &mut w).is_err());
        let bad = State {
            u: vec![1.0, 2.0],
            ..st.clone()
        };
        assert!(advance(&problem, &bad, 0.1, &exec, &mut w).is_err());
        assert!(start(&problem, 0.0, &[1.0, 2.0], 0.1, &mut w).is_err());
        assert!(start(&problem, 0.0, &[1.0], -1.0, &mut w).is_err());
        assert!(
            Session::new(
                &problem,
                st.clone(),
                Config {
                    atol: 0.0,
                    rtol: 0.0
                },
                ParallelExecution::sequential()
            )
            .is_err()
        );
        assert!(error_norm(&[], &[], 1.0, 1.0).is_nan());
        assert!(!accepts(f64::NAN));
        assert_eq!(propose_ratio(f64::NAN, &[]), RATIO_CLIP.0);
        // mismatched k_prev poisons with NaN instead of indexing out of bounds
        let inputs = stage_inputs(0.0, &[1.0, 2.0], &[vec![1.0], vec![1.0]], 0.1, 1.0);
        assert!(inputs[0].y[1].is_nan());
    }
}
