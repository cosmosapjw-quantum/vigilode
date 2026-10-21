//! Structured stage solver of the LRS photon transport adapter (BASS/rei DAG
//! node P03): a client [`StageLinearSystem`] of the RODAS5P fast driver that
//! replaces the banded LU of `W = inv I - J` (`inv = 1 / (h gamma)`).
//!
//! Per energy group `i`, `W` holds a tridiagonal block `T_i + Gamma I` (the
//! angular transport, the energy outflow, absorption and Thomson loss) minus
//! the rank-two Thomson gain `Gamma P`, `P = d 1^T / 2 + (d o p) p^T / 4`
//! (`d` the angular widths, `p` the cell averages of `P2`). Energy transport
//! couples a cell only to its upstream neighbour in the same angular column,
//! through a diagonal. When no two columns drift in opposite directions (in
//! the physical coordinate, `H > 2|s|`), `W` is block triangular in energy
//! and [`Mode::Exact`] solves it in `O(n)`: group after group from upstream,
//! each by Thomas elimination plus a two-by-two correction, then the ledger
//! rows by back-substitution. A mixed drift direction is a typed refusal in
//! that mode; [`Mode::Krylov`] runs restarted GMRES on the structured
//! operator, right-preconditioned by a symmetric block Gauss-Seidel energy
//! sweep that uses the same exact group solves. With `two_level`, the
//! preconditioner is sweep, group-number coarse correction, sweep: the
//! coarse operator is `W` restricted to group sums and prolongated
//! isotropically (`d_j / 2` per unit), a tridiagonal in energy on which
//! Thomson scattering and angular transport vanish exactly. It removes the
//! slowly converging energy-diffusion mode of the sweep when scattering
//! dominates.
//!
//! The two-by-two correction is a Sherman-Morrison-Woodbury variant in
//! conservation form. With `y = (T_i + Gamma I)^-1 b`, `z1 = (T_i + Gamma
//! I)^-1 d / 2` and `z2 = (T_i + Gamma I)^-1 (d o p) / 4`, the solution is
//! `x = y + Gamma (m0 z1 + m2 z2)` with `m0 = 1^T x` and `m2 = p^T x`. The
//! textbook capacitance entry `1 - Gamma 1^T z1` tends to `inv / (inv +
//! Gamma)` and cancels catastrophically when `Gamma >> inv`. Angular
//! transport conserves number, `sum d = 2` and `sum d p = 0`, so the column
//! sums of `T_i` are `g = inv + kbar_i + qout_i` and `1^T W_i = g^T`; then
//! `m0 = 1^T x` is equivalent to `(g^T z1) m0 + (g^T z2) m2 = 1^T y`, whose
//! leading coefficient is a sum of positive terms (`T_i + Gamma I` is a
//! column diagonally dominant M-matrix, so `z1 > 0`).
//! [`Capacitance::Textbook`] keeps the cancelling form as a diagnostic arm.
//! The two identities hold for the stored floats only to rounding: their
//! exact residuals, `1 - sum d / 2` and `sum (d o p) / 4` (compensated
//! sums), enter `K11` and `K12` as corrections.
//!
//! Every kernel counts its floating-point operations (`+ - * /` once each,
//! negation free) as it runs; [`factor_flops`] and [`exact_solve_flops`] are
//! the closed forms the counts must equal.

#![allow(clippy::needless_range_loop)] // the kernels index several arrays by cell

use std::sync::{Arc, Mutex};

use rodas5p_core::{
    CoreError, CoreResult, ExactOperatorIdentity, IdentityPreconditioner, LinearOperator,
    Preconditioner, PreconditionerSide, WorkCounters,
};
use rodas5p_integrators::{OdeProblem, StageLinearSystem};
use rodas5p_krylov::{
    CapacityGrowth, GmresCapacity, GmresConfig, GmresWorkspace, solve_gmres_into,
};

use super::{Boundary, Model};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    /// Block-triangular elimination; a mixed drift direction is refused.
    Exact,
    /// Restarted right-preconditioned GMRES on the structured operator.
    /// `two_level` adds the group-number coarse correction to the sweep.
    Krylov {
        restart: usize,
        max_iterations: usize,
        rtol: f64,
        two_level: bool,
    },
}

/// The two-by-two correction's first equation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capacitance {
    /// `(g^T z1) m0 + (g^T z2) m2 = 1^T y` (no cancellation).
    Conservation,
    /// `(1 - Gamma 1^T z1) m0 - (Gamma 1^T z2) m2 = 1^T y` (diagnostic only).
    Textbook,
}

/// Energy drift direction of a linearization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// No column drifts to higher energy: block upper triangular.
    Down,
    /// No column drifts to lower energy: block lower triangular.
    Up,
    Mixed,
}

/// Floating-point operations by phase, and Krylov work.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OpCounts {
    pub linearize: u64,
    pub factor: u64,
    pub solve: u64,
    pub linearizations: u64,
    pub factorizations: u64,
    pub solves: u64,
    pub krylov_iterations: u64,
    pub krylov_matvecs: u64,
    pub sweeps: u64,
    pub max_krylov_iterations: u64,
}

/// Coefficients of one linearization plus their factorization.
struct Data {
    nq: usize,
    nm: usize,
    outflow: bool,
    /// Angular widths and `P2` cell averages.
    d: Vec<f64>,
    p: Vec<f64>,
    /// Per column: drifts upward (the adapter's `velocity >= 0` upwind rule).
    up: Vec<bool>,
    /// Energy flux coefficient of cell `(i, j)` (the adapter's `coef`).
    qc: Vec<f64>,
    /// Per interior angular face `k`, index `k - 1`: flows to larger `mu`.
    mu_up: Vec<bool>,
    /// Angular flux coefficient of face `k` in group `i`: `i (nm - 1) + k - 1`.
    mc: Vec<f64>,
    kbar: Vec<f64>,
    gamma: f64,
    direction: Direction,
    inv: f64,
    /// Thomas pivots, multipliers and super-diagonal per cell.
    piv: Vec<f64>,
    mul: Vec<f64>,
    sup: Vec<f64>,
    /// Column sums of the group blocks without Thomson.
    g: Vec<f64>,
    z1: Vec<f64>,
    z2: Vec<f64>,
    /// Per group `[K11, K12, K21, K22, det]` of the two-by-two correction.
    cap: Vec<[f64; 5]>,
    capacitance: Capacitance,
    /// `1 - sum d / 2` and `sum (d o p) / 4` of the stored floats
    /// (compensated): the exact corrections of the conservation form, which
    /// assumes `sum d = 2` and `sum d p = 0` (they hold only to rounding).
    rest0: f64,
    rest2: f64,
    /// Krylov two-level mode: the coarse tridiagonal (diagonal into
    /// `cpiv`, couplings to the groups below and above) and its Thomas
    /// factors, plus the preconditioner's scratch.
    two_level: bool,
    cpiv: Vec<f64>,
    cmul: Vec<f64>,
    csup: Vec<f64>,
    scratch: Mutex<Scratch>,
}

struct Scratch {
    res: Vec<f64>,
    tmp: Vec<f64>,
    coarse: Vec<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Empty,
    Linearized,
    Factored,
}

/// The structured client stage solver.
pub struct BlockStageSolver {
    model: Arc<Model>,
    epoch: u64,
    mode: Mode,
    data: Data,
    stage: Stage,
    /// Krylov mode: solution buffer and GMRES workspace.
    x: Vec<f64>,
    gmres: GmresWorkspace,
    pub ops: OpCounts,
}

impl BlockStageSolver {
    /// A solver bound to `model` at mesh `epoch`, sized once for its layout.
    pub fn new(
        model: Arc<Model>,
        epoch: u64,
        mode: Mode,
        capacitance: Capacitance,
    ) -> CoreResult<Self> {
        let (nq, nm) = (model.layout.nq, model.layout.nm);
        let n = model.layout.n();
        if nm < 2 {
            return Err(CoreError::InvalidInput(
                "structured solver needs at least two angular cells".into(),
            ));
        }
        let mut gmres = GmresWorkspace::with_ls_workspace();
        let mut two_level = false;
        if let Mode::Krylov {
            restart,
            max_iterations,
            rtol,
            two_level: coarse,
        } = mode
        {
            two_level = coarse;
            if restart == 0 || max_iterations == 0 || !(rtol.is_finite() && rtol > 0.0) {
                return Err(CoreError::InvalidInput(
                    "structured Krylov mode needs positive restart, iterations and rtol".into(),
                ));
            }
            gmres.reserve(n, restart.min(n))?;
            warm_least_squares(&mut gmres, n, restart.min(n));
        }
        let cells = nq * nm;
        let data = Data {
            nq,
            nm,
            outflow: model.layout.boundary == Boundary::Outflow,
            d: model.layout.dm.clone(),
            p: model.layout.p2.clone(),
            up: vec![false; nm],
            qc: vec![0.0; cells],
            mu_up: vec![false; nm - 1],
            mc: vec![0.0; nq * (nm - 1)],
            kbar: vec![0.0; nq],
            gamma: 0.0,
            direction: Direction::Down,
            inv: 0.0,
            piv: vec![0.0; cells],
            mul: vec![0.0; cells],
            sup: vec![0.0; cells],
            g: vec![0.0; cells],
            z1: vec![0.0; cells],
            z2: vec![0.0; cells],
            cap: vec![[0.0; 5]; nq],
            capacitance,
            rest0: 0.0,
            rest2: 0.0,
            two_level,
            cpiv: vec![0.0; nq],
            cmul: vec![0.0; nq],
            csup: vec![0.0; nq],
            scratch: Mutex::new(Scratch {
                res: vec![0.0; n],
                tmp: vec![0.0; n],
                coarse: vec![0.0; nq],
            }),
        };
        let mut data = data;
        data.grid_sums();
        Ok(Self {
            model,
            epoch,
            mode,
            data,
            stage: Stage::Empty,
            x: vec![0.0; n],
            gmres,
            ops: OpCounts::default(),
        })
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn direction(&self) -> Direction {
        self.data.direction
    }

    /// Rebinds to a model at a new mesh epoch; the old linearization and
    /// factorization become stale (a later factor or solve is refused).
    pub fn rebind(&mut self, model: Arc<Model>, epoch: u64) -> CoreResult<()> {
        if model.layout.nq != self.data.nq || model.layout.nm != self.data.nm {
            return Err(CoreError::Dimension(
                "structured solver rebind: layout shape changed".into(),
            ));
        }
        // the new mesh's angular widths, P2 averages and boundary policy
        self.data.d.copy_from_slice(&model.layout.dm);
        self.data.p.copy_from_slice(&model.layout.p2);
        self.data.outflow = model.layout.boundary == Boundary::Outflow;
        self.data.grid_sums();
        self.model = model;
        self.epoch = epoch;
        self.stage = Stage::Empty;
        Ok(())
    }

    /// Linearizes the model's operator at `t` (it is linear in the state).
    pub fn linearize_at(&mut self, t: f64) -> CoreResult<()> {
        let (h, s, _, _) = self.model.drift(t);
        let gamma = self.model.gamma;
        self.linearize_with(t, h, s, gamma)
    }

    /// Linearizes with the energy drift `-h - s c_j`, the angular drift of
    /// `s` and the Thomson rate `gamma` given explicitly (a bordered model's
    /// shear and ionization are states); the absorber is the model's at `t`.
    pub fn linearize_with(&mut self, t: f64, h: f64, s: f64, gamma: f64) -> CoreResult<()> {
        self.stage = Stage::Empty;
        let m = &*self.model;
        let l = &m.layout;
        let a = &mut self.data;
        let (nq, nm) = (a.nq, a.nm);
        let (mut any_down, mut any_up) = (false, false);
        for j in 0..nm {
            let v = -h - s * l.c[j];
            a.up[j] = v >= 0.0;
            any_down |= v < 0.0;
            any_up |= v > 0.0;
            for i in 0..nq {
                a.qc[i * nm + j] = v * (1.0 / (l.dq[i] * l.dm[j])) * l.dm[j];
            }
        }
        for k in 1..nm {
            let mu = l.mu_edges[k];
            let v = -3.0 * s * mu * (1.0 - mu * mu);
            let upstream = if v >= 0.0 { k - 1 } else { k };
            a.mu_up[k - 1] = v >= 0.0;
            for i in 0..nq {
                a.mc[i * (nm - 1) + k - 1] = v * (1.0 / (l.dq[i] * l.dm[upstream])) * l.dq[i];
            }
        }
        a.gamma = gamma;
        for i in 0..nq {
            a.kbar[i] = m.kbar(i, t).0;
        }
        // velocities 2 nm, energy coefficients 3 nq nm, angular velocities
        // 5 (nm - 1), angular coefficients 3 nq (nm - 1)
        self.ops.linearize += (2 * nm + 3 * nq * nm + 5 * (nm - 1) + 3 * nq * (nm - 1)) as u64;
        self.ops.linearizations += 1;
        if !(a
            .qc
            .iter()
            .chain(&a.mc)
            .chain(&a.kbar)
            .all(|v| v.is_finite())
            && a.gamma.is_finite())
        {
            return Err(CoreError::NonFinite(
                "structured solver: non-finite linearization".into(),
            ));
        }
        a.direction = match (any_down, any_up) {
            (true, true) => Direction::Mixed,
            (true, false) => Direction::Down,
            _ => Direction::Up,
        };
        if a.direction == Direction::Mixed && self.mode == Mode::Exact {
            return Err(CoreError::InvalidInput(
                "STRUCTURED_EXACT_MIXED_DIRECTION: the energy drift changes sign across columns"
                    .into(),
            ));
        }
        self.stage = Stage::Linearized;
        Ok(())
    }

    /// Factors `W = inv I - J` of the current linearization.
    pub fn factor_with(&mut self, inv: f64) -> CoreResult<()> {
        if self.stage == Stage::Empty {
            return Err(CoreError::InvalidInput(
                "STRUCTURED_NOT_LINEARIZED: factor without a current linearization".into(),
            ));
        }
        if !(inv.is_finite() && inv > 0.0) {
            return Err(CoreError::InvalidInput(
                "structured solver: inv = 1/(h gamma) must be finite and positive".into(),
            ));
        }
        self.stage = Stage::Linearized;
        let flops = self.data.factor(inv)?;
        self.ops.factor += flops;
        self.ops.factorizations += 1;
        self.stage = Stage::Factored;
        Ok(())
    }

    /// Solves `W x = b` in place.
    pub fn solve_with(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        if self.stage != Stage::Factored {
            return Err(CoreError::InvalidInput(
                "STRUCTURED_STALE: solve without a factorization of the current linearization"
                    .into(),
            ));
        }
        let n = self.x.len();
        if b.len() != n {
            return Err(CoreError::Dimension(
                "structured solver: right-hand side length mismatch".into(),
            ));
        }
        match self.mode {
            Mode::Exact => {
                self.ops.solve += self.data.exact_solve(b);
                counters.direct_solve_calls += 1;
            }
            Mode::Krylov {
                restart,
                max_iterations,
                rtol,
                ..
            } => {
                let config = GmresConfig {
                    restart,
                    max_arnoldi: max_iterations,
                    rtol,
                    atol: 0.0,
                };
                let report = solve_gmres_into(
                    &Operator { a: &self.data },
                    &Sweep { a: &self.data },
                    b,
                    None,
                    &config,
                    None,
                    &mut self.x,
                    &mut self.gmres,
                    GmresCapacity {
                        max_dimension: n,
                        max_columns: restart.min(n),
                        growth: CapacityGrowth::Refuse,
                    },
                    counters,
                )?;
                b.copy_from_slice(&self.x);
                self.ops.krylov_iterations += report.iterations;
                self.ops.krylov_matvecs += report.matvecs;
                self.ops.sweeps += report.preconditioner_apps;
                self.ops.max_krylov_iterations =
                    self.ops.max_krylov_iterations.max(report.iterations);
            }
        }
        if !b.iter().all(|v| v.is_finite()) {
            return Err(CoreError::NonFinite(
                "structured solver: non-finite solution".into(),
            ));
        }
        self.ops.solves += 1;
        Ok(())
    }

    /// `y = M^-1 r` of the Krylov preconditioner (diagnostics).
    pub fn precondition(&self, r: &[f64], y: &mut [f64]) -> CoreResult<()> {
        if self.stage != Stage::Factored {
            return Err(CoreError::InvalidInput(
                "STRUCTURED_STALE: precondition without a factorization".into(),
            ));
        }
        if self.data.two_level {
            self.data.two_level_apply(r, y);
        } else {
            self.data.sweep(r, y);
        }
        Ok(())
    }

    /// `y = W x` with the structured coefficients (model-parity checks).
    pub fn apply_w(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        if self.stage != Stage::Factored {
            return Err(CoreError::InvalidInput(
                "STRUCTURED_STALE: apply without a factorization".into(),
            ));
        }
        if x.len() != self.x.len() || y.len() != self.x.len() {
            return Err(CoreError::Dimension(
                "structured solver: operator vector length mismatch".into(),
            ));
        }
        self.data.apply(x, y);
        Ok(())
    }
}

/// Grows the reused least-squares workspace of `gmres` to its largest size
/// (`columns + 1` by `columns`) once, so that no later solve grows it: one
/// unpreconditioned cycle on a diagonal operator with distinct entries (no
/// happy breakdown), which then exhausts its budget by design.
fn warm_least_squares(gmres: &mut GmresWorkspace, n: usize, columns: usize) {
    struct Diagonal(usize);
    impl LinearOperator for Diagonal {
        fn dimension(&self) -> usize {
            self.0
        }
        fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
            for (k, (yk, xk)) in y.iter_mut().zip(x).enumerate() {
                *yk = (1.0 + k as f64) * xk;
            }
            Ok(())
        }
        fn token(&self) -> u64 {
            0
        }
    }
    let config = GmresConfig {
        restart: columns,
        max_arnoldi: columns,
        rtol: 0.0,
        atol: 0.0,
    };
    let rhs = vec![1.0; n];
    let mut out = vec![0.0; n];
    let _ = solve_gmres_into(
        &Diagonal(n),
        &IdentityPreconditioner::new(n),
        &rhs,
        None,
        &config,
        None,
        &mut out,
        gmres,
        GmresCapacity {
            max_dimension: n,
            max_columns: columns,
            growth: CapacityGrowth::Refuse,
        },
        &mut WorkCounters::default(),
    );
}

/// Thomas solve with stored multipliers, pivots and super-diagonal; returns
/// its flops `5 nm - 4`.
#[inline(always)]
fn thomas(mul: &[f64], piv: &[f64], sup: &[f64], x: &mut [f64]) -> u64 {
    let nm = x.len();
    for j in 1..nm {
        x[j] -= mul[j] * x[j - 1];
    }
    x[nm - 1] /= piv[nm - 1];
    for j in (0..nm - 1).rev() {
        x[j] = (x[j] - sup[j] * x[j + 1]) / piv[j];
    }
    (5 * nm - 4) as u64
}

/// Closed form of one factorization's flops, per group: assembly `5 nm`,
/// Thomas elimination `3 (nm - 1)`, the two right-hand sides `3 nm`, two
/// Thomas solves `2 (5 nm - 4)`, the capacitance sums (`8 nm + 2` with the
/// two grid corrections; textbook
/// `6 nm + 3`) and the two-by-two `6`.
pub fn factor_flops(nq: usize, nm: usize, capacitance: Capacitance) -> u64 {
    let sums = match capacitance {
        Capacitance::Conservation => 8 * nm + 2,
        Capacitance::Textbook => 6 * nm + 3,
    };
    (nq * (5 * nm + 3 * (nm - 1) + 3 * nm + 2 * (5 * nm - 4) + sums + 6)) as u64
}

/// Closed form of one exact solve's flops when every column drifts the
/// solve's way: per group Thomas `5 nm - 4`, moments `3 nm`, the correction
/// `10`, the update `4 nm`, the absorbed ledger `nm + 3`; the energy
/// coupling `2 nm` per group but the first; the boundary ledgers `2 nm + 4`
/// (outflow) or `2` (closed).
pub fn exact_solve_flops(nq: usize, nm: usize, outflow: bool) -> u64 {
    let group = (5 * nm - 4) + 3 * nm + 10 + 4 * nm + (nm + 3);
    let boundary = if outflow { 2 * nm + 4 } else { 2 };
    (nq * group + (nq - 1) * 2 * nm + boundary) as u64
}

/// Neumaier-compensated sum.
fn compensated(values: impl Iterator<Item = f64>) -> f64 {
    let (mut sum, mut c) = (0.0_f64, 0.0_f64);
    for v in values {
        let t = sum + v;
        c += if sum.abs() >= v.abs() {
            (sum - t) + v
        } else {
            (v - t) + sum
        };
        sum = t;
    }
    sum + c
}

impl Data {
    /// The conservation-form corrections of the current angular grid.
    fn grid_sums(&mut self) {
        let half: f64 = compensated(self.d.iter().map(|d| 0.5 * d));
        self.rest0 = 1.0 - half;
        self.rest2 = compensated(self.d.iter().zip(&self.p).map(|(d, p)| 0.25 * (d * p)));
    }

    /// Energy outflow rate of cell `(i, j)` (zero through a closed boundary).
    #[inline(always)]
    fn q_out(&self, i: usize, j: usize) -> f64 {
        let c = self.qc[i * self.nm + j];
        if self.up[j] {
            if i + 1 < self.nq || self.outflow {
                c
            } else {
                0.0
            }
        } else if i >= 1 || self.outflow {
            -c
        } else {
            0.0
        }
    }

    #[inline(always)]
    fn cells(&self, i: usize) -> std::ops::Range<usize> {
        let start = 1 + i * (self.nm + 1);
        start..start + self.nm
    }

    fn factor(&mut self, inv: f64) -> CoreResult<u64> {
        self.inv = inv;
        let (nq, nm) = (self.nq, self.nm);
        let gamma = self.gamma;
        let mut flops: u64 = 0;
        for i in 0..nq {
            let base = i * nm;
            let faces = i * (nm - 1);
            // assemble the tridiagonal: diagonal into piv, sub-diagonal into mul
            for j in 0..nm {
                let qout = self.q_out(i, j);
                let upper = if j + 1 < nm && self.mu_up[j] {
                    self.mc[faces + j]
                } else {
                    0.0
                };
                let lower = if j >= 1 && !self.mu_up[j - 1] {
                    -self.mc[faces + j - 1]
                } else {
                    0.0
                };
                let g = inv + self.kbar[i] + qout;
                self.g[base + j] = g;
                self.piv[base + j] = g + gamma + (upper + lower);
                self.mul[base + j] = if j >= 1 && self.mu_up[j - 1] {
                    -self.mc[faces + j - 1]
                } else {
                    0.0
                };
                self.sup[base + j] = if j + 1 < nm && !self.mu_up[j] {
                    self.mc[faces + j]
                } else {
                    0.0
                };
            }
            flops += (5 * nm) as u64;
            // Thomas elimination (no pivoting: column diagonally dominant)
            for j in 1..nm {
                let lambda = self.mul[base + j] / self.piv[base + j - 1];
                self.mul[base + j] = lambda;
                self.piv[base + j] -= lambda * self.sup[base + j - 1];
            }
            flops += (3 * (nm - 1)) as u64;
            if !self.piv[base..base + nm]
                .iter()
                .all(|v| v.is_finite() && *v > 0.0)
            {
                return Err(CoreError::LinearSolve(
                    "STRUCTURED_PIVOT: non-positive Thomas pivot".into(),
                ));
            }
            for j in 0..nm {
                self.z1[base + j] = 0.5 * self.d[j];
                self.z2[base + j] = 0.25 * (self.d[j] * self.p[j]);
            }
            flops += (3 * nm) as u64;
            let r = base..base + nm;
            flops += thomas(
                &self.mul[r.clone()],
                &self.piv[r.clone()],
                &self.sup[r.clone()],
                &mut self.z1[r.clone()],
            );
            flops += thomas(
                &self.mul[r.clone()],
                &self.piv[r.clone()],
                &self.sup[r.clone()],
                &mut self.z2[r],
            );
            let (mut c11, mut c12, mut c21, mut c22) = (0.0, 0.0, 0.0, 0.0);
            match self.capacitance {
                Capacitance::Conservation => {
                    for j in 0..nm {
                        c11 += self.g[base + j] * self.z1[base + j];
                        c12 += self.g[base + j] * self.z2[base + j];
                    }
                    c11 += self.rest0;
                    c12 -= self.rest2;
                    flops += (4 * nm + 2) as u64;
                }
                Capacitance::Textbook => {
                    let (mut s1, mut s2) = (0.0, 0.0);
                    for j in 0..nm {
                        s1 += self.z1[base + j];
                        s2 += self.z2[base + j];
                    }
                    c11 = 1.0 - gamma * s1;
                    c12 = -(gamma * s2);
                    flops += (2 * nm + 3) as u64;
                }
            }
            for j in 0..nm {
                c21 += self.p[j] * self.z1[base + j];
                c22 += self.p[j] * self.z2[base + j];
            }
            flops += (4 * nm) as u64;
            let (k11, k12, k21, k22) = (c11, c12, gamma * c21, gamma * c22 - 1.0);
            let det = k11 * k22 - k12 * k21;
            flops += 6;
            if !(det.is_finite() && det != 0.0) {
                return Err(CoreError::LinearSolve(
                    "STRUCTURED_SINGULAR: singular two-by-two correction".into(),
                ));
            }
            self.cap[i] = [k11, k12, k21, k22, det];
        }
        if self.two_level {
            self.factor_coarse()?;
        }
        Ok(flops)
    }

    /// The coarse tridiagonal `R W P` (group sums of `W` applied to
    /// isotropic groups) and its Thomas factors; column diagonally dominant
    /// by `sum_j (inv + kbar_i) d_j / 2`.
    fn factor_coarse(&mut self) -> CoreResult<()> {
        let (nq, nm) = (self.nq, self.nm);
        for i in 0..nq {
            let (mut diag, mut below, mut above) = (0.0, 0.0, 0.0);
            for j in 0..nm {
                let half = 0.5 * self.d[j];
                diag += self.g[i * nm + j] * half;
                if self.up[j] {
                    if i >= 1 {
                        below -= self.qc[(i - 1) * nm + j] * half;
                    }
                } else if i + 1 < nq {
                    above += self.qc[(i + 1) * nm + j] * half;
                }
            }
            self.cpiv[i] = diag;
            self.cmul[i] = below;
            self.csup[i] = above;
        }
        for i in 1..nq {
            let lambda = self.cmul[i] / self.cpiv[i - 1];
            self.cmul[i] = lambda;
            self.cpiv[i] -= lambda * self.csup[i - 1];
        }
        if !self.cpiv.iter().all(|v| v.is_finite() && *v > 0.0) {
            return Err(CoreError::LinearSolve(
                "STRUCTURED_COARSE_PIVOT: non-positive coarse pivot".into(),
            ));
        }
        Ok(())
    }

    /// Two-level preconditioner `y = M^-1 r`: sweep, coarse correction of
    /// the group sums of the residual, sweep of the new residual.
    fn two_level_apply(&self, r: &[f64], y: &mut [f64]) {
        let (nq, nm) = (self.nq, self.nm);
        self.sweep(r, y);
        let mut guard = self.scratch.lock().unwrap_or_else(|e| e.into_inner());
        let Scratch { res, tmp, coarse } = &mut *guard;
        self.apply(y, res);
        for (v, rk) in res.iter_mut().zip(r) {
            *v = rk - *v;
        }
        for i in 0..nq {
            let mut sum = 0.0;
            for v in &res[self.cells(i)] {
                sum += *v;
            }
            coarse[i] = sum;
        }
        thomas(&self.cmul, &self.cpiv, &self.csup, coarse);
        for i in 0..nq {
            let start = self.cells(i).start;
            for j in 0..nm {
                y[start + j] += 0.5 * self.d[j] * coarse[i];
            }
        }
        for i in 0..nq {
            let a = self.cells(i).end;
            y[a] = r[a];
        }
        let hi = y.len() - 1;
        y[0] = r[0];
        y[hi] = r[hi];
        self.ledgers(y);
        self.apply(y, res);
        for (v, rk) in res.iter_mut().zip(r) {
            *v = rk - *v;
        }
        self.sweep(res, tmp);
        for (v, t) in y.iter_mut().zip(tmp.iter()) {
            *v += *t;
        }
    }

    /// Solves group `i` in place: `x <- (T_i + Gamma (I - P))^-1 x`.
    #[inline(always)]
    fn group_solve(&self, i: usize, x: &mut [f64]) -> u64 {
        let nm = self.nm;
        let r = i * nm..(i + 1) * nm;
        let mut flops = thomas(&self.mul[r.clone()], &self.piv[r.clone()], &self.sup[r], x);
        let (mut s0, mut s2) = (0.0, 0.0);
        for j in 0..nm {
            s0 += x[j];
            s2 += self.p[j] * x[j];
        }
        let [k11, k12, k21, k22, det] = self.cap[i];
        let m0 = (s0 * k22 + k12 * s2) / det;
        let m2 = -(k11 * s2 + k21 * s0) / det;
        let (a0, a2) = (self.gamma * m0, self.gamma * m2);
        let base = i * nm;
        for j in 0..nm {
            x[j] += a0 * self.z1[base + j] + a2 * self.z2[base + j];
        }
        flops += (3 * nm + 10 + 4 * nm) as u64;
        flops
    }

    /// Adds the energy inflow into group `i` (cells `dst`) from the solved
    /// neighbour `from` (cells `src`) along the columns drifting `up`;
    /// returns its flops.
    #[inline(always)]
    fn inflow(&self, from: usize, up: bool, src: &[f64], dst: &mut [f64]) -> u64 {
        let nm = self.nm;
        let mut flops = 0;
        for j in 0..nm {
            if self.up[j] == up {
                let c = self.qc[from * nm + j];
                if up {
                    dst[j] += c * src[j];
                } else {
                    dst[j] -= c * src[j];
                }
                flops += 2;
            }
        }
        flops
    }

    /// Solves the ledger rows by back-substitution from solved cells.
    fn ledgers(&self, x: &mut [f64]) -> u64 {
        let (nq, nm) = (self.nq, self.nm);
        let mut flops = 0;
        for i in 0..nq {
            let r = self.cells(i);
            let mut sum = 0.0;
            for v in &x[r.clone()] {
                sum += *v;
            }
            let a = r.end;
            x[a] = (x[a] + self.kbar[i] * sum) / self.inv;
            flops += (nm + 3) as u64;
        }
        let hi = x.len() - 1;
        if self.outflow {
            let (mut lo_in, mut hi_in) = (0.0, 0.0);
            let (first, last) = (self.cells(0).start, self.cells(nq - 1).start);
            for j in 0..nm {
                if self.up[j] {
                    hi_in += self.qc[(nq - 1) * nm + j] * x[last + j];
                } else {
                    lo_in -= self.qc[j] * x[first + j];
                }
            }
            x[0] = (x[0] + lo_in) / self.inv;
            x[hi] = (x[hi] + hi_in) / self.inv;
            flops += (2 * nm + 4) as u64;
        } else {
            x[0] /= self.inv;
            x[hi] /= self.inv;
            flops += 2;
        }
        flops
    }

    /// Exact block-triangular solve in place; returns its flops.
    fn exact_solve(&self, x: &mut [f64]) -> u64 {
        let (nq, nm) = (self.nq, self.nm);
        let down = self.direction != Direction::Up;
        let mut flops = 0;
        for step in 0..nq {
            let i = if down { nq - 1 - step } else { step };
            let cells = self.cells(i);
            let (lo, rest) = x.split_at_mut(cells.start);
            let (mid, hi) = rest.split_at_mut(nm);
            if step > 0 {
                flops += if down {
                    self.inflow(i + 1, false, &hi[1..1 + nm], mid)
                } else {
                    self.inflow(i - 1, true, &lo[self.cells(i - 1)], mid)
                };
            }
            flops += self.group_solve(i, mid);
        }
        flops + self.ledgers(x)
    }

    /// `y = W x`.
    fn apply(&self, x: &[f64], y: &mut [f64]) {
        let (nq, nm) = (self.nq, self.nm);
        let n = x.len();
        for k in 0..n {
            y[k] = self.inv * x[k];
        }
        for i in 0..nq {
            let r = self.cells(i);
            let (mut m0, mut m2) = (0.0, 0.0);
            for j in 0..nm {
                m0 += x[r.start + j];
                m2 += self.p[j] * x[r.start + j];
            }
            let mut absorbed = 0.0;
            for j in 0..nm {
                let c = r.start + j;
                // losses: energy outflow, absorption and Thomson; Thomson gain
                y[c] += (self.q_out(i, j) + self.kbar[i] + self.gamma) * x[c];
                y[c] -= self.gamma * (self.d[j] / 2.0 * (m0 + 0.5 * self.p[j] * m2));
                absorbed += self.kbar[i] * x[c];
                // energy inflow into the downstream neighbour
                let q = self.qc[i * nm + j] * x[c];
                if self.up[j] {
                    if i + 1 < nq {
                        y[c + nm + 1] -= q;
                    } else if self.outflow {
                        y[n - 1] -= q;
                    }
                } else if i >= 1 {
                    y[c - nm - 1] += q;
                } else if self.outflow {
                    y[0] += q;
                }
            }
            y[r.end] -= absorbed;
            for k in 1..nm {
                let upstream = if self.mu_up[k - 1] { k - 1 } else { k };
                let f = self.mc[i * (nm - 1) + k - 1] * x[r.start + upstream];
                y[r.start + k - 1] += f;
                y[r.start + k] -= f;
            }
        }
    }

    /// Symmetric block Gauss-Seidel energy sweep `y = M^-1 r` with exact
    /// group solves: downward columns from the top, then both directions
    /// from the bottom (the downward inflow from the first sweep).
    fn sweep(&self, r: &[f64], y: &mut [f64]) {
        let (nq, nm) = (self.nq, self.nm);
        for i in (0..nq).rev() {
            let cells = self.cells(i);
            let (_, rest) = y.split_at_mut(cells.start);
            let (mid, hi) = rest.split_at_mut(nm);
            mid.copy_from_slice(&r[cells]);
            if i + 1 < nq {
                self.inflow(i + 1, false, &hi[1..1 + nm], mid);
            }
            self.group_solve(i, mid);
        }
        for i in 0..nq {
            let cells = self.cells(i);
            let (lo, rest) = y.split_at_mut(cells.start);
            let (mid, hi) = rest.split_at_mut(nm);
            mid.copy_from_slice(&r[cells]);
            if i + 1 < nq {
                self.inflow(i + 1, false, &hi[1..1 + nm], mid);
            }
            if i >= 1 {
                self.inflow(i - 1, true, &lo[self.cells(i - 1)], mid);
            }
            self.group_solve(i, mid);
        }
        for i in 0..nq {
            let a = self.cells(i).end;
            y[a] = r[a];
        }
        let hi = y.len() - 1;
        y[0] = r[0];
        y[hi] = r[hi];
        self.ledgers(y);
    }
}

struct Operator<'a> {
    a: &'a Data,
}

impl LinearOperator for Operator<'_> {
    fn dimension(&self) -> usize {
        2 + self.a.nq * (self.a.nm + 1)
    }

    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        self.a.apply(x, y);
        Ok(())
    }

    /// No exact identity: the operator changes with every factorization.
    fn exact_identity(&self) -> Option<ExactOperatorIdentity> {
        None
    }

    fn token(&self) -> u64 {
        self.a.inv.to_bits()
    }
}

struct Sweep<'a> {
    a: &'a Data,
}

impl Preconditioner for Sweep<'_> {
    fn dimension(&self) -> usize {
        2 + self.a.nq * (self.a.nm + 1)
    }

    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        if self.a.two_level {
            self.a.two_level_apply(x, y);
        } else {
            self.a.sweep(x, y);
        }
        Ok(())
    }

    fn side(&self) -> PreconditionerSide {
        PreconditionerSide::Right
    }
}

impl StageLinearSystem for BlockStageSolver {
    fn dimension(&self) -> usize {
        self.x.len()
    }

    fn linearize(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        _y: &[f64],
        _counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        if problem.dimension != self.x.len() {
            return Err(CoreError::Dimension(
                "structured solver: problem dimension mismatch".into(),
            ));
        }
        self.linearize_at(t)
    }

    fn factor(&mut self, inv_h_gamma: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        // charged per attempt: the work of a failed factorization stays charged
        counters.direct_factorizations += 1;
        self.factor_with(inv_h_gamma)
    }

    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        self.solve_with(b, counters)
    }
}
