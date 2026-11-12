//! PPSW1-6b: an opt-in, singly implicit, common-shift, two-step Peer-W method
//! (Schmitt & Weiner, SINUM 42(1), 2004; variant 6b, `s = 6`, `gamma = 0.6194119750...`).
//!
//! Reachable only as `rodas5p_integrators::ppsw1`: not a catalog candidate, not default routed,
//! not derivative-free, not RODAS5P.  Evidence: `docs/agent/unified_local_agent_20261109/
//! PW04_PPSW1_6BC_VERIFICATION_KO.md`.  Variable-step order is exactly 5.
//!
//! # Step (stage form, node-row Vandermonde `V_ij = c_i^(j-1)`)
//!
//! ```text
//! (I - gamma h T) Y_i = sum_j B_ij Y_prev_j + h sum_j A_ij F_prev_j + h T sum_j Gamma_ij Y_prev_j
//! ```
//!
//! with `T = J(t_m, u_m)` frozen at the step start (class
//! [`DerivativeClass::ExactJacobianOrJvpFrozenAtStepStart`]), `sigma = h / h_prev`.  One LU of
//! `I - h gamma T` serves all six independent stage solves.  Per-step solves are linear; Newton is
//! used only by the starter.  `B, A, Gamma, beta` are computed from the closed form of the paper
//! (`Theta = V S P V^-1`, `Gamma = -gamma Theta`, `B = (I + gamma E) Theta + sigma beta E`,
//! `A = beta + gamma Theta`) in double-double arithmetic and rounded once to `f64`.  `gamma` is the
//! real root of `720 g^6 - 7920 g^5 + 32760 g^4 - 64200 g^3 + 62130 g^2 - 28098 g + 4683` near
//! 0.6194; its first 38 decimals are embedded (certified to 60 digits by the contract tests).
//!
//! # Starter (owner decision OD-23)
//!
//! Grid reference `t0 = t_init + h0`, so the six nodes `t0 + h0 c_j` lie in `[t_init, t_init + 2 h0]`.
//! One forward 5-stage Radau IIA step of size `2 h0` (nodes and Lagrange-integral tableau built
//! here), simple Newton with the frozen `T = J(t_init, y0)`, node values from the degree-5
//! collocation polynomial.  `h0` rule (OD-24): `h0 * |lambda_max| <~ 1`, with `|lambda_max|`
//! bounded by the infinity norm of the frozen Jacobian ([`select_h0`]); a user-supplied `h0` is
//! also allowed.  Newton is never used in per-step solves.
//!
//! # Estimator and controller (OD-25, OD-26; OWNER_CHOICE, not literature-certified for PPSW1)
//!
//! `Est = Y_ms - sum_{j=2..6} w_j(sigma) Y_{m-1,j}` with exact Lagrange weights through
//! `c = -1/2, 0, 2/5, 4/5, 1` at `x = 1 + sigma`; RMS norm with `atol + rtol |y|`; exponent 1/5.
//! Controller: `sigma* = min{cap, max{0.5, 0.85 Err^(-1/5)}}`, keep `h` in `[0.95, 1.05]`, keep
//! `h` when growth is proposed and any of the last 6 applied ratios exceeded 1 (Podhaisky thesis
//! controller); NaN halves `h` and rejects.  Accepted ratios are capped by 1.10 (owner rule); the
//! paper's 1.5 is recorded separately.
//!
//! Mass matrices are not supported.

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, LuFactorization, WorkCounters};

use crate::{OdeProblem, ParallelExecution};

pub const METHOD_ID: &str = "ppsw1-6b-schmitt-weiner-singly-implicit-peer-w";
pub const STAGES: usize = 6;
pub const FORMAL_ORDER: usize = 5;
pub const ACCEPTED_STEP_SIGMA_MAX: f64 = 1.10;
pub const PAPER_STEP_RATIO_BOUND: f64 = 1.5;

#[derive(Clone, Copy, Debug)]
struct Dd {
    hi: f64,
    lo: f64,
}
fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bb = s - a;
    (s, (a - (s - bb)) + (b - bb))
}
fn quick(a: f64, b: f64) -> Dd {
    let s = a + b;
    Dd {
        hi: s,
        lo: b - (s - a),
    }
}
fn dd(x: f64) -> Dd {
    Dd { hi: x, lo: 0.0 }
}
fn add(x: Dd, y: Dd) -> Dd {
    let (s, e) = two_sum(x.hi, y.hi);
    let (t, f) = two_sum(x.lo, y.lo);
    let u = quick(s, e + t);
    quick(u.hi, u.lo + f)
}
fn neg(x: Dd) -> Dd {
    Dd {
        hi: -x.hi,
        lo: -x.lo,
    }
}
fn sub(x: Dd, y: Dd) -> Dd {
    add(x, neg(y))
}
fn mul(x: Dd, y: Dd) -> Dd {
    let p = x.hi * y.hi;
    let e = x.hi.mul_add(y.hi, -p) + (x.hi * y.lo + x.lo * y.hi);
    quick(p, e)
}
fn div(x: Dd, y: Dd) -> Dd {
    let q1 = x.hi / y.hi;
    let r = sub(x, mul(y, dd(q1)));
    let q2 = r.hi / y.hi;
    let r = sub(r, mul(y, dd(q2)));
    let q3 = r.hi / y.hi;
    add(quick(q1, q2), dd(q3))
}
fn from_i128(n: i128) -> Dd {
    let hi = n as f64;
    quick(hi, (n - hi as i128) as f64)
}

type D6 = [[Dd; 6]; 6];
fn z6() -> D6 {
    [[dd(0.0); 6]; 6]
}
fn i6() -> D6 {
    let mut m = z6();
    for (i, row) in m.iter_mut().enumerate() {
        row[i] = dd(1.0);
    }
    m
}
fn mm(a: &D6, b: &D6) -> D6 {
    let mut m = z6();
    for i in 0..6 {
        for j in 0..6 {
            let mut s = dd(0.0);
            for k in 0..6 {
                s = add(s, mul(a[i][k], b[k][j]));
            }
            m[i][j] = s;
        }
    }
    m
}
fn inv(a: &D6) -> D6 {
    let mut m = *a;
    let mut r = i6();
    for col in 0..6 {
        let mut piv = col;
        for row in col..6 {
            if m[row][col].hi.abs() > m[piv][col].hi.abs() {
                piv = row;
            }
        }
        m.swap(col, piv);
        r.swap(col, piv);
        let d = m[col][col];
        for j in 0..6 {
            m[col][j] = div(m[col][j], d);
            r[col][j] = div(r[col][j], d);
        }
        for row in 0..6 {
            if row != col {
                let f = m[row][col];
                for j in 0..6 {
                    m[row][j] = sub(m[row][j], mul(f, m[col][j]));
                    r[row][j] = sub(r[row][j], mul(f, r[col][j]));
                }
            }
        }
    }
    r
}

fn gamma_dd() -> Dd {
    // First 38 decimals of gamma_hat (60-digit value certified in the contract tests).
    let n38: i128 = 61_941_197_506_625_051_631_076_337_400_290_510_912;
    let ten38: i128 = 100_000_000_000_000_000_000_000_000_000_000_000_000;
    div(from_i128(n38), from_i128(ten38))
}

pub fn gamma() -> f64 {
    let g = gamma_dd();
    g.hi + g.lo
}
pub fn nodes() -> [f64; 6] {
    [-1.0, -0.5, 0.0, 0.4, 0.8, 1.0]
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tableau {
    pub gamma: f64,
    pub nodes: [f64; 6],
    pub b_matrix: [[f64; 6]; 6],
    pub a_matrix: [[f64; 6]; 6],
    pub gamma_matrix: [[f64; 6]; 6],
    pub beta: [[f64; 6]; 6],
}

pub fn tableau(sigma: f64) -> CoreResult<Tableau> {
    if !(sigma.is_finite() && sigma > 0.0) {
        return Err(CoreError::InvalidInput(
            "sigma must be finite and positive".into(),
        ));
    }
    let c = [
        dd(-1.0),
        dd(-0.5),
        dd(0.0),
        div(dd(2.0), dd(5.0)),
        div(dd(4.0), dd(5.0)),
        dd(1.0),
    ];
    let sg = dd(sigma);
    let mut v = z6();
    let mut s = z6();
    let mut p = z6();
    let mut d = z6();
    let mut di = z6();
    let mut f0 = z6();
    for i in 0..6 {
        let mut pw = dd(1.0);
        for j in 0..6 {
            v[i][j] = pw;
            pw = mul(pw, c[i]);
            // binom(j,i)
            let mut b = 1.0;
            if j >= i {
                for t in 0..i {
                    b = b * (j - t) as f64 / (t + 1) as f64;
                }
            } else {
                b = 0.0;
            }
            p[i][j] = dd(b.round());
        }
        let mut sp = dd(1.0);
        for _ in 0..i {
            sp = mul(sp, sg);
        }
        s[i][i] = sp;
        d[i][i] = dd((i + 1) as f64);
        di[i][i] = div(dd(1.0), dd((i + 1) as f64));
        if i >= 1 {
            f0[i][i - 1] = dd(1.0);
        }
    }
    let vi = inv(&v);
    let theta = mm(&mm(&mm(&v, &s), &p), &vi);
    let mut ft = z6();
    for i in 0..6 {
        for j in 0..6 {
            ft[i][j] = f0[j][i];
        }
    }
    let mut e = mm(&mm(&mm(&v, &d), &ft), &vi);
    for row in e.iter_mut() {
        for x in row.iter_mut() {
            *x = neg(*x);
        }
    }
    let beta = mm(&mm(&mm(&mm(&v, &f0), &s), &di), &vi);
    let g = gamma_dd();
    let mut ie = i6();
    for i in 0..6 {
        for j in 0..6 {
            ie[i][j] = add(ie[i][j], mul(g, e[i][j]));
        }
    }
    let be = mm(&beta, &e);
    let t1 = mm(&ie, &theta);
    let mut out = Tableau {
        gamma: g.hi + g.lo,
        nodes: nodes(),
        b_matrix: [[0.0; 6]; 6],
        a_matrix: [[0.0; 6]; 6],
        gamma_matrix: [[0.0; 6]; 6],
        beta: [[0.0; 6]; 6],
    };
    let f = |x: Dd| x.hi + x.lo;
    for i in 0..6 {
        for j in 0..6 {
            let bb = add(t1[i][j], mul(sg, be[i][j]));
            let gam = neg(mul(g, theta[i][j]));
            out.b_matrix[i][j] = f(bb);
            out.gamma_matrix[i][j] = f(gam);
            out.a_matrix[i][j] = f(sub(beta[i][j], gam));
            out.beta[i][j] = f(beta[i][j]);
        }
    }
    Ok(out)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DerivativeClass {
    ExactJacobianOrJvpFrozenAtStepStart,
}

#[derive(Clone, Debug)]
pub struct Identity {
    pub method_id: &'static str,
    pub order: usize,
    pub stages: usize,
    pub accepted_step_sigma_max: f64,
    pub paper_step_ratio_bound: f64,
    pub derivative_class: DerivativeClass,
    pub default_routed: bool,
    pub reuses_rodas5p_starter: bool,
    pub reuses_rodas5p_estimator: bool,
    pub reuses_rodas5p_controller: bool,
    pub claims_shared_krylov_recurrence: bool,
    pub claims_g0_qualified: bool,
    pub claims_derivative_free: bool,
}

pub fn identity() -> Identity {
    Identity {
        method_id: METHOD_ID,
        order: FORMAL_ORDER,
        stages: STAGES,
        accepted_step_sigma_max: ACCEPTED_STEP_SIGMA_MAX,
        paper_step_ratio_bound: PAPER_STEP_RATIO_BOUND,
        derivative_class: DerivativeClass::ExactJacobianOrJvpFrozenAtStepStart,
        default_routed: false,
        reuses_rodas5p_starter: false,
        reuses_rodas5p_estimator: false,
        reuses_rodas5p_controller: false,
        claims_shared_krylov_recurrence: false,
        claims_g0_qualified: false,
        claims_derivative_free: false,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub t: f64,
    pub y: [Vec<f64>; 6],
    pub f: [Vec<f64>; 6],
    pub h_prev: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StageInput {
    pub t: f64,
    pub explicit: Vec<f64>,
    pub transfer: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StageSolve {
    pub y: [Vec<f64>; 6],
}

fn to_six(v: Vec<Vec<f64>>) -> CoreResult<[Vec<f64>; 6]> {
    <[Vec<f64>; 6]>::try_from(v)
        .map_err(|_| CoreError::LinearSolve("PPSW1 expected exactly six stage vectors".into()))
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

fn check_sigma(h: f64, h_prev: f64, cap: f64) -> CoreResult<f64> {
    check_positive("step size", h)?;
    check_positive("previous step size", h_prev)?;
    let sigma = h / h_prev;
    if sigma > cap.min(ACCEPTED_STEP_SIGMA_MAX) {
        return Err(CoreError::InvalidInput(format!(
            "step ratio {sigma} exceeds the PPSW1 cap {cap}"
        )));
    }
    Ok(sigma)
}

pub fn stage_inputs(state: &State, h: f64) -> CoreResult<[StageInput; 6]> {
    let sigma = check_sigma(h, state.h_prev, ACCEPTED_STEP_SIGMA_MAX)?;
    stage_inputs_unchecked(state, h, sigma)
}

fn stage_inputs_unchecked(state: &State, h: f64, sigma: f64) -> CoreResult<[StageInput; 6]> {
    let tab = tableau(sigma)?;
    let n = state.y[5].len();
    if state.y.iter().any(|v| v.len() != n) || state.f.iter().any(|v| v.len() != n) {
        return Err(CoreError::Dimension("PPSW1 state shape mismatch".into()));
    }
    let build = |i: usize| StageInput {
        t: state.t + tab.nodes[i] * h,
        explicit: (0..n)
            .map(|r| {
                (0..6)
                    .map(|j| {
                        tab.b_matrix[i][j] * state.y[j][r] + h * tab.a_matrix[i][j] * state.f[j][r]
                    })
                    .sum()
            })
            .collect(),
        transfer: (0..n)
            .map(|r| (0..6).map(|j| tab.gamma_matrix[i][j] * state.y[j][r]).sum())
            .collect(),
    };
    Ok(std::array::from_fn(build))
}

pub fn solve_stages(
    problem: &OdeProblem,
    state: &State,
    h: f64,
    inputs: &[StageInput; 6],
    exec: &ParallelExecution,
    counters: &mut WorkCounters,
) -> CoreResult<StageSolve> {
    if problem.mass_matrix.is_some() {
        return Err(CoreError::InvalidInput("identity mass only".into()));
    }
    check_positive("step size", h)?;
    let n = problem.dimension;
    if state.y[5].len() != n
        || inputs
            .iter()
            .any(|s| s.explicit.len() != n || s.transfer.len() != n)
    {
        return Err(CoreError::Dimension("PPSW1 stage shape mismatch".into()));
    }
    let jac = problem.dense_jacobian(state.t, &state.y[5], counters)?;
    let shift = h * gamma();
    let mut w = DenseMatrix::identity(n);
    for i in 0..n {
        for j in 0..n {
            w[(i, j)] -= shift * jac[(i, j)];
        }
    }
    counters.direct_factorizations += 1;
    let lu = LuFactorization::new(&w)?;
    let rhs: Vec<Vec<f64>> = inputs
        .iter()
        .map(|inp| {
            (0..n)
                .map(|r| {
                    inp.explicit[r] + h * (0..n).map(|k| jac[(r, k)] * inp.transfer[k]).sum::<f64>()
                })
                .collect()
        })
        .collect();
    counters.linear_solves += 6;
    counters.direct_solve_calls += 6;
    let idx = [0usize, 1, 2, 3, 4, 5];
    let xs = exec.map_ordered(&idx, |&i| lu.solve(&rhs[i]))?;
    Ok(StageSolve { y: to_six(xs)? })
}

#[derive(Clone, Debug, PartialEq)]
pub struct StepOutput {
    pub next: State,
}

pub fn advance(
    problem: &OdeProblem,
    state: &State,
    h: f64,
    exec: &ParallelExecution,
    counters: &mut WorkCounters,
) -> CoreResult<StepOutput> {
    advance_capped(problem, state, h, ACCEPTED_STEP_SIGMA_MAX, exec, counters)
}

fn advance_capped(
    problem: &OdeProblem,
    state: &State,
    h: f64,
    cap: f64,
    exec: &ParallelExecution,
    counters: &mut WorkCounters,
) -> CoreResult<StepOutput> {
    let sigma = check_sigma(h, state.h_prev, cap)?;
    let inputs = stage_inputs_unchecked(state, h, sigma)?;
    let sol = solve_stages(problem, state, h, &inputs, exec, counters)?;
    let mut f_new: Vec<Vec<f64>> = Vec::with_capacity(6);
    for (inp, yi) in inputs.iter().zip(&sol.y) {
        f_new.push(problem.eval_rhs(inp.t, yi, counters)?);
    }
    if sol
        .y
        .iter()
        .chain(f_new.iter())
        .any(|v| v.iter().any(|x| !x.is_finite()))
    {
        return Err(CoreError::NonFinite("PPSW1 step produced NaN/Inf".into()));
    }
    let f = to_six(f_new)?;
    Ok(StepOutput {
        next: State {
            t: state.t + h,
            y: sol.y,
            f,
            h_prev: h,
        },
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub step_ratio_cap: f64,
}
impl Default for Config {
    fn default() -> Self {
        Config {
            step_ratio_cap: ACCEPTED_STEP_SIGMA_MAX,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ControllerState {
    pub accepted_ratios: Vec<f64>,
}

pub struct Attempt {
    next: State,
    sigma: f64,
    generation: u64,
}
impl Attempt {
    pub fn next_state(&self) -> &State {
        &self.next
    }
    pub fn ratio(&self) -> f64 {
        self.sigma
    }
}

pub struct Session<'a> {
    problem: &'a OdeProblem,
    state: State,
    config: Config,
    exec: ParallelExecution,
    controller: ControllerState,
    counters: WorkCounters,
    generation: u64,
}

impl<'a> Session<'a> {
    pub fn new(
        problem: &'a OdeProblem,
        state: State,
        config: Config,
        exec: ParallelExecution,
    ) -> CoreResult<Self> {
        if !(config.step_ratio_cap.is_finite()
            && config.step_ratio_cap > 0.0
            && config.step_ratio_cap <= ACCEPTED_STEP_SIGMA_MAX)
        {
            return Err(CoreError::InvalidInput(
                "step_ratio_cap must lie in (0, 1.10]".into(),
            ));
        }
        Ok(Self {
            problem,
            state,
            config,
            exec,
            controller: ControllerState {
                accepted_ratios: Vec::new(),
            },
            counters: WorkCounters::default(),
            generation: 0,
        })
    }
    pub fn state(&self) -> &State {
        &self.state
    }
    pub fn controller(&self) -> &ControllerState {
        &self.controller
    }
    pub fn counters(&self) -> WorkCounters {
        self.counters
    }
    pub fn attempt(&mut self, h: f64) -> CoreResult<Attempt> {
        let sigma = check_sigma(h, self.state.h_prev, self.config.step_ratio_cap)?;
        let mut work = self.counters;
        let r = advance_capped(
            self.problem,
            &self.state,
            h,
            self.config.step_ratio_cap,
            &self.exec,
            &mut work,
        );
        self.counters = work;
        Ok(Attempt {
            next: r?.next,
            sigma,
            generation: self.generation,
        })
    }
    pub fn accept(&mut self, attempt: Attempt) -> CoreResult<()> {
        if attempt.generation != self.generation {
            return Err(CoreError::InvalidInput("stale attempt".into()));
        }
        self.controller.accepted_ratios.push(attempt.sigma);
        let excess = self
            .controller
            .accepted_ratios
            .len()
            .saturating_sub(HYSTERESIS_HISTORY);
        self.controller.accepted_ratios.drain(..excess);
        self.state = attempt.next;
        self.counters.accepted_steps += 1;
        self.generation += 1;
        Ok(())
    }
    pub fn reject(&mut self, attempt: Attempt) -> CoreResult<()> {
        if attempt.generation != self.generation {
            return Err(CoreError::InvalidInput("stale attempt".into()));
        }
        self.counters.rejected_steps += 1;
        Ok(())
    }

    /// Attempt one step of size `h`, decide with the OWNER_CHOICE estimator/controller, and
    /// commit (accept) or discard (reject).  Work of failed attempts stays charged.  A
    /// non-finite step or a NaN error norm rejects with a halving proposal.  Ratios above the
    /// cap are refused with `CoreError::InvalidInput` and no side effects.
    pub fn try_step(&mut self, h: f64, tol: &Tolerances) -> CoreResult<Decision> {
        tol.validate()?;
        let attempt = match self.attempt(h) {
            Ok(a) => a,
            Err(CoreError::NonFinite(_)) => {
                self.counters.rejected_steps += 1;
                return Ok(Decision::Rejected {
                    error_norm: f64::NAN,
                    next_ratio: RATIO_CLIP.0,
                    rounding_floor_warning: false,
                });
            }
            Err(e) => return Err(e),
        };
        let est = estimator_vector(&self.state, &attempt.next.y[5], attempt.sigma);
        let err = error_norm(&attempt.next.y[5], &est, tol.atol, tol.rtol);
        let warn = rounding_floor_warning(&self.state, &attempt.next.y[5], attempt.sigma, tol);
        if err.is_nan() {
            self.reject(attempt)?;
            return Ok(Decision::Rejected {
                error_norm: err,
                next_ratio: RATIO_CLIP.0,
                rounding_floor_warning: warn,
            });
        }
        if accepts(err) {
            self.accept(attempt)?;
            let next_ratio = propose_ratio(
                err,
                &self.controller.accepted_ratios,
                self.config.step_ratio_cap,
            );
            Ok(Decision::Accepted {
                error_norm: err,
                next_ratio,
                rounding_floor_warning: warn,
            })
        } else {
            let next_ratio = propose_ratio(
                err,
                &self.controller.accepted_ratios,
                self.config.step_ratio_cap,
            );
            self.reject(attempt)?;
            Ok(Decision::Rejected {
                error_norm: err,
                next_ratio,
                rounding_floor_warning: warn,
            })
        }
    }
}

// ---------------------------------------------------------------------------------------
// Estimator and controller (OWNER_CHOICE, OD-25 / OD-26)
// ---------------------------------------------------------------------------------------

/// Number of past applied ratios inspected by the growth-history rule (`s = 6`, thesis p.41-42).
pub const HYSTERESIS_HISTORY: usize = 6;
/// Safety factor of the controller.
pub const SAFETY: f64 = 0.85;
/// Lower clip of the proposed ratio (the upper clip is the configured cap, at most 1.10).
pub const RATIO_CLIP: (f64, f64) = (0.5, ACCEPTED_STEP_SIGMA_MAX);
/// Keep-`h` band of the controller.
pub const HYSTERESIS_BAND: (f64, f64) = (0.95, 1.05);
/// Control exponent: the estimator is an order-4 interpolation defect, so `Err ~ h^5`.
pub const CONTROL_EXPONENT: f64 = 1.0 / 5.0;
/// A rounding-floor warning is raised when `sum|w| eps |Y|` exceeds this fraction of the tolerance.
pub const ROUNDING_FLOOR_FRACTION: f64 = 0.1;

/// Exact Lagrange weights `w_j(sigma)` through the nodes `c = -1/2, 0, 2/5, 4/5, 1`
/// (previous-step stages 2..6) evaluated at `x = 1 + sigma`.
pub fn estimator_weights(sigma: f64) -> [f64; 5] {
    let c = nodes();
    let x = 1.0 + sigma;
    let mut w = [0.0; 5];
    for (j, wj) in w.iter_mut().enumerate() {
        let mut v = 1.0;
        for k in 0..5 {
            if k != j {
                v *= (x - c[k + 1]) / (c[j + 1] - c[k + 1]);
            }
        }
        *wj = v;
    }
    w
}

/// `Est = Y_new,6 - sum_{j=2..6} w_j(sigma) Y_prev,j`: zero for data of polynomial degree <= 4.
pub fn estimator_vector(prev: &State, y_new: &[f64], sigma: f64) -> Vec<f64> {
    let w = estimator_weights(sigma);
    (0..y_new.len())
        .map(|r| {
            let mut acc = y_new[r];
            for (wj, yj) in w.iter().zip(&prev.y[1..]) {
                acc -= wj * yj.get(r).copied().unwrap_or(f64::NAN);
            }
            acc
        })
        .collect()
}

/// Tolerances of the error norm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tolerances {
    /// Absolute tolerance (`>= 0`).
    pub atol: f64,
    /// Relative tolerance (`>= 0`); `atol + rtol` must be positive.
    pub rtol: f64,
}

impl Tolerances {
    fn validate(&self) -> CoreResult<()> {
        if self.atol.is_finite()
            && self.rtol.is_finite()
            && self.atol >= 0.0
            && self.rtol >= 0.0
            && self.atol + self.rtol > 0.0
        {
            Ok(())
        } else {
            Err(CoreError::InvalidInput(
                "tolerances must be finite, nonnegative and not both zero".into(),
            ))
        }
    }
}

/// RMS norm `sqrt(mean((e_i / (atol + rtol |y_i|))^2))`; NaN for empty/mismatched input or a
/// non-positive scale.
pub fn error_norm(y: &[f64], e: &[f64], atol: f64, rtol: f64) -> f64 {
    if y.is_empty() || y.len() != e.len() {
        return f64::NAN;
    }
    let mut sum = 0.0;
    for (yi, ei) in y.iter().zip(e) {
        let scale = atol + rtol * yi.abs();
        if !(scale > 0.0 && scale.is_finite()) {
            return f64::NAN;
        }
        sum += (ei / scale).powi(2);
    }
    (sum / y.len() as f64).sqrt()
}

/// A step is accepted iff its error norm is `<= 1` (NaN is rejected).
pub fn accepts(err: f64) -> bool {
    err <= 1.0
}

/// Proposed ratio `h*/h`: `min{cap, max{0.5, 0.85 Err^(-1/5)}}`, keep `h` inside the band or when
/// growth is proposed after any of the last [`HYSTERESIS_HISTORY`] ratios exceeded 1; NaN halves.
pub fn propose_ratio(err: f64, last_ratios: &[f64], cap: f64) -> f64 {
    if err.is_nan() {
        return RATIO_CLIP.0;
    }
    let cap = cap.min(ACCEPTED_STEP_SIGMA_MAX);
    let raw = if err <= 0.0 {
        cap
    } else {
        SAFETY * err.powf(-CONTROL_EXPONENT)
    };
    let raw = raw.max(RATIO_CLIP.0).min(cap);
    if (HYSTERESIS_BAND.0..=HYSTERESIS_BAND.1).contains(&raw) {
        return 1.0;
    }
    let tail = &last_ratios[last_ratios.len().saturating_sub(HYSTERESIS_HISTORY)..];
    if raw > HYSTERESIS_BAND.1 && tail.iter().any(|&r| r > 1.0) {
        return 1.0;
    }
    raw
}

/// `true` when `sum_j |w_j| eps |Y|` is not small relative to the tolerance in some component
/// (the estimator then sits on the rounding floor).
pub fn rounding_floor_warning(prev: &State, y_new: &[f64], sigma: f64, tol: &Tolerances) -> bool {
    let w = estimator_weights(sigma);
    (0..y_new.len()).any(|r| {
        let mut mag = y_new[r].abs();
        for (wj, yj) in w.iter().zip(&prev.y[1..]) {
            mag += wj.abs() * yj.get(r).copied().unwrap_or(0.0).abs();
        }
        let scale = tol.atol + tol.rtol * y_new[r].abs();
        mag * f64::EPSILON > ROUNDING_FLOOR_FRACTION * scale
    })
}

/// Outcome of [`Session::try_step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Decision {
    /// Step accepted and committed.
    Accepted {
        /// Error norm of the step.
        error_norm: f64,
        /// Proposed ratio for the next step.
        next_ratio: f64,
        /// Rounding-floor diagnostic.
        rounding_floor_warning: bool,
    },
    /// Step rejected; history and controller untouched, work charged.
    Rejected {
        /// Error norm (NaN possible).
        error_norm: f64,
        /// Proposed ratio for the retry.
        next_ratio: f64,
        /// Rounding-floor diagnostic.
        rounding_floor_warning: bool,
    },
}

// ---------------------------------------------------------------------------------------
// h0 rule (OD-24)
// ---------------------------------------------------------------------------------------

/// Cap of `h0 * |lambda_max|` used by [`select_h0`].
pub const H0_LAMBDA_PRODUCT: f64 = 1.0;

/// Infinity norm of the frozen Jacobian `J(t, y)`: an upper bound of `|lambda_max|`.
pub fn jacobian_norm_bound(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    counters: &mut WorkCounters,
) -> CoreResult<f64> {
    let jac = problem.dense_jacobian(t, y, counters)?;
    let n = problem.dimension;
    let mut best = 0.0_f64;
    for i in 0..n {
        let row: f64 = (0..n).map(|j| jac[(i, j)].abs()).sum();
        best = best.max(row);
    }
    if best.is_finite() {
        Ok(best)
    } else {
        Err(CoreError::NonFinite("Jacobian norm is not finite".into()))
    }
}

/// `h0 = min{h_max, H0_LAMBDA_PRODUCT / norm_bound}` so that `h0 |lambda_max| <~ 1`.
/// Refuses a non-finite/negative bound or a non-finite/non-positive `h_max`.
pub fn select_h0(norm_bound: f64, h_max: f64) -> CoreResult<f64> {
    check_positive("h_max", h_max)?;
    if !(norm_bound.is_finite() && norm_bound >= 0.0) {
        return Err(CoreError::InvalidInput(
            "Jacobian norm bound must be finite and nonnegative".into(),
        ));
    }
    if norm_bound == 0.0 {
        Ok(h_max)
    } else {
        Ok(h_max.min(H0_LAMBDA_PRODUCT / norm_bound))
    }
}

// ---------------------------------------------------------------------------------------
// Starter (OD-23): forward 5-stage Radau IIA over [t_init, t_init + 2 h0]
// ---------------------------------------------------------------------------------------

fn legendre_shifted(n: usize, x: f64) -> f64 {
    let t = 2.0 * x - 1.0;
    let (mut p0, mut p1) = (1.0, t);
    if n == 0 {
        return p0;
    }
    for k in 2..=n {
        let kf = k as f64;
        let p2 = ((2.0 * kf - 1.0) * t * p1 - (kf - 1.0) * p0) / kf;
        p0 = p1;
        p1 = p2;
    }
    p1
}

/// Nodes of the 5-stage Radau IIA method (roots of `P5(2x-1) - P4(2x-1)` on `(0, 1]`).
pub fn radau_nodes() -> CoreResult<[f64; 5]> {
    let q = |x: f64| legendre_shifted(5, x) - legendre_shifted(4, x);
    let grid = 4000usize;
    let mut roots: Vec<f64> = Vec::new();
    for k in 1..grid {
        let (mut a, mut b) = ((k - 1) as f64 / grid as f64, k as f64 / grid as f64);
        let (qa, qb) = (q(a), q(b));
        if qa == 0.0 && a > 0.0 {
            roots.push(a);
        } else if qa * qb < 0.0 {
            for _ in 0..200 {
                let m = 0.5 * (a + b);
                if q(a) * q(m) <= 0.0 {
                    b = m;
                } else {
                    a = m;
                }
            }
            roots.push(0.5 * (a + b));
        }
    }
    roots.push(1.0);
    <[f64; 5]>::try_from(roots)
        .map_err(|_| CoreError::Coefficients("Radau IIA node search failed".into()))
}

/// Integrated Lagrange polynomials: `coef[j][m]` multiplies `tau^m` in `int_0^tau L_j`.
fn lagrange_integrals(c: &[f64; 5]) -> [[f64; 6]; 5] {
    let mut out = [[0.0; 6]; 5];
    for j in 0..5 {
        let mut poly = [0.0_f64; 5];
        poly[0] = 1.0;
        let mut deg = 0usize;
        for k in 0..5 {
            if k == j {
                continue;
            }
            let denom = c[j] - c[k];
            let mut next = [0.0_f64; 5];
            for m in 0..=deg {
                next[m + 1] += poly[m] / denom;
                next[m] -= poly[m] * c[k] / denom;
            }
            poly = next;
            deg += 1;
        }
        for m in 0..5 {
            out[j][m + 1] = poly[m] / (m + 1) as f64;
        }
    }
    out
}

fn eval_poly6(coef: &[f64; 6], x: f64) -> f64 {
    coef.iter().rev().fold(0.0, |acc, &c| acc * x + c)
}

fn all_finite(v: &[f64]) -> bool {
    v.iter().all(|x| x.is_finite())
}

/// Maximum simple-Newton iterations of the starter.
pub const STARTER_MAX_NEWTON: usize = 50;

/// Build the first [`State`] at `t_init + 2 h0` from `(t_init, y0)` (grid reference
/// `t0 = t_init + h0`).  Charged work: one Jacobian provider call, one LU of the `5n x 5n`
/// Newton matrix, one nonlinear solve with `k` iterations (`5k` RHS calls, `k` linear solves),
/// and six RHS calls for the node derivatives.  Every callback time is `>= t_init`.
pub fn start(
    problem: &OdeProblem,
    t_init: f64,
    y0: &[f64],
    h0: f64,
    counters: &mut WorkCounters,
) -> CoreResult<State> {
    if problem.mass_matrix.is_some() {
        return Err(CoreError::InvalidInput(
            "PPSW1 supports identity mass matrices only".into(),
        ));
    }
    check_positive("first step size h0", h0)?;
    let n = problem.dimension;
    if y0.len() != n || !t_init.is_finite() || !all_finite(y0) {
        return Err(CoreError::InvalidInput(
            "PPSW1 start needs a finite state of the problem dimension".into(),
        ));
    }
    let rc = radau_nodes()?;
    let li = lagrange_integrals(&rc);
    let big_h = 2.0 * h0;
    let a_at = |i: usize, j: usize| eval_poly6(&li[j], rc[i]);
    let jac = problem.dense_jacobian(t_init, y0, counters)?;
    let dim = 5 * n;
    let mut m = DenseMatrix::identity(dim);
    for i in 0..5 {
        for j in 0..5 {
            let aij = a_at(i, j);
            for r in 0..n {
                for k in 0..n {
                    m[(i * n + r, j * n + k)] -= big_h * aij * jac[(r, k)];
                }
            }
        }
    }
    counters.direct_factorizations += 1;
    let lu = LuFactorization::new(&m)?;
    counters.nonlinear_solves += 1;
    let mut y: Vec<Vec<f64>> = (0..5).map(|_| y0.to_vec()).collect();
    let mut f: Vec<Vec<f64>> = Vec::new();
    let mut converged = false;
    for _ in 0..STARTER_MAX_NEWTON {
        counters.nonlinear_iterations += 1;
        f.clear();
        for i in 0..5 {
            f.push(problem.eval_rhs(t_init + rc[i] * big_h, &y[i], counters)?);
        }
        let mut g = vec![0.0; dim];
        for i in 0..5 {
            for r in 0..n {
                let acc: f64 = f.iter().enumerate().map(|(j, fj)| a_at(i, j) * fj[r]).sum();
                g[i * n + r] = y[i][r] - y0[r] - big_h * acc;
            }
        }
        counters.linear_solves += 1;
        counters.direct_solve_calls += 1;
        let delta = lu.solve(&g)?;
        let mut dmax = 0.0_f64;
        let mut ymax = 0.0_f64;
        for i in 0..5 {
            for r in 0..n {
                y[i][r] -= delta[i * n + r];
                dmax = dmax.max(delta[i * n + r].abs());
                ymax = ymax.max(y[i][r].abs());
            }
        }
        if !(dmax.is_finite() && ymax.is_finite()) {
            counters.nonlinear_failures += 1;
            return Err(CoreError::NonFinite("PPSW1 starter Newton diverged".into()));
        }
        if dmax <= 1e-13 * (1.0 + ymax) {
            converged = true;
            break;
        }
    }
    if !converged {
        counters.nonlinear_failures += 1;
        return Err(CoreError::NonlinearSolve(
            "PPSW1 starter Newton did not converge".into(),
        ));
    }
    // Node values from the degree-5 collocation polynomial u(theta H), theta = (1 + c_j)/2.
    let c = nodes();
    let mut yn: Vec<Vec<f64>> = Vec::with_capacity(6);
    let mut tn: Vec<f64> = Vec::with_capacity(6);
    for (k, &ck) in c.iter().enumerate() {
        let theta = (1.0 + ck) / 2.0;
        tn.push(t_init + h0 * (1.0 + ck));
        let v: Vec<f64> = if k == 0 {
            y0.to_vec()
        } else if k == 5 {
            y[4].clone()
        } else {
            (0..n)
                .map(|r| {
                    let mut acc = 0.0;
                    for j in 0..5 {
                        acc += f[j][r] * eval_poly6(&li[j], theta);
                    }
                    y0[r] + big_h * acc
                })
                .collect()
        };
        yn.push(v);
    }
    let mut fn_: Vec<Vec<f64>> = Vec::with_capacity(6);
    for k in 0..6 {
        fn_.push(problem.eval_rhs(tn[k], &yn[k], counters)?);
    }
    if !(yn.iter().all(|v| all_finite(v)) && fn_.iter().all(|v| all_finite(v))) {
        return Err(CoreError::NonFinite(
            "PPSW1 starter produced NaN/Inf".into(),
        ));
    }
    Ok(State {
        t: t_init + big_h,
        y: to_six(yn)?,
        f: to_six(fn_)?,
        h_prev: h0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radau_nodes_match_known_values_and_end_at_one() {
        let c = radau_nodes().unwrap();
        let known = [
            0.057_104_196_114_517_6,
            0.276_843_013_638_123_8,
            0.583_590_432_368_916_8,
            0.860_240_135_656_219_5,
            1.0,
        ];
        for (a, b) in c.iter().zip(known) {
            assert!((a - b).abs() < 1e-14, "{a} vs {b}");
        }
    }

    #[test]
    fn radau_tableau_rows_integrate_polynomials_exactly() {
        // Collocation condition C(5): sum_j a_ij c_j^(k-1) = c_i^k / k for k = 1..5.
        let c = radau_nodes().unwrap();
        let li = lagrange_integrals(&c);
        for i in 0..5 {
            for k in 1..=5usize {
                let lhs: f64 = (0..5)
                    .map(|j| eval_poly6(&li[j], c[i]) * c[j].powi(k as i32 - 1))
                    .sum();
                let rhs = c[i].powi(k as i32) / k as f64;
                assert!((lhs - rhs).abs() < 1e-13, "i={i} k={k}");
            }
        }
    }

    #[test]
    fn estimator_weights_at_sigma_one_match_exact_values() {
        let w = estimator_weights(1.0);
        let want = [
            512.0 / 117.0,
            -30.0,
            625.0 / 9.0,
            -1250.0 / 13.0,
            160.0 / 3.0,
        ];
        for (a, b) in w.iter().zip(want) {
            assert!((a - b).abs() < 1e-12);
        }
    }

    #[test]
    fn gamma_is_a_root_of_the_defining_polynomial_to_double_precision() {
        let g = gamma();
        let p = 720.0 * g.powi(6) - 7920.0 * g.powi(5) + 32760.0 * g.powi(4) - 64200.0 * g.powi(3)
            + 62130.0 * g * g
            - 28098.0 * g
            + 4683.0;
        assert!(p.abs() < 1e-9, "{p}");
    }
}
