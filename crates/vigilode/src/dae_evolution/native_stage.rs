//! Source-bound original transformed RODAS5P stage experiments.
//! Local stage perturbation is distinct from LTE and full slab qualification.
use super::*;
use rodas5p_core::{Rodas5pCoefficients, WorkCounters, directed::div_down, rodas5p_coefficients};
use rodas5p_integrators::{
    OdeProblem, StageLinearSystem,
    stage_clients::{DenseLuMode, dense_lu_reuse::DenseLuReuseStageClient},
};
const N: usize = 32;
const S: usize = 8;
const Z: Iv = Iv { lo: 0., hi: 0. };
const STAGE_CAP: f64 = 1e-9;
const GOAL_CAP: f64 = 1e-10;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum NativeStagePlan {
    Legacy,
    ReuseLu,
    Triangular,
    Reciprocal,
    GoalUniform,
    GoalAdjoint,
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct NativeStageWork {
    pub source_jacobian_skips: usize,
    pub preparations: usize,
    pub diagonal_proposals: usize,
    pub jacobi_proposals: usize,
    pub diagonal_accepts: usize,
    pub jacobi_accepts: usize,
    pub direct_solves: usize,
    pub residual_checks: usize,
    pub residual_refusals: usize,
    pub completed_certified_attempts: usize,
    pub max_stage_error_upper: f64,
    pub max_goal_error_upper: f64,
    pub max_goal_uniform_upper: f64,
    pub triangular_products: usize,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct StageDefectReceipt {
    pub max_stage_error_upper: f64,
    pub endpoint_goal_error_upper: f64,
    pub signed_goal_enclosure: [f64; 2],
    pub uniform_goal_upper: f64,
    pub stages: usize,
}
/// Exact constant homogeneous reduced source. Structural admission rejects
/// clock/offset factors and nonlinear state products, not fitted matrices.
#[derive(Clone)]
pub struct HomogeneousStageModel {
    proof: de::ConstantTerminalAdjoint,
    lower: bool,
}
impl HomogeneousStageModel {
    pub fn from_source(src: &DaeSource, goal: &[f64]) -> CoreResult<Self> {
        let proof = de::ConstantTerminalAdjoint::from_homogeneous_source(src, goal)?;
        let n = proof.dimension();
        let a = proof.matrix();
        let lower = (0..n).all(|i| (i + 1..n).all(|j| a[i * n + j] == 0.));
        let upper = (0..n).all(|i| (0..i).all(|j| a[i * n + j] == 0.));
        if !lower && !upper {
            return Err(bad("native stage requires triangular reduced source"));
        }
        Ok(Self { proof, lower })
    }
    pub fn dimension(&self) -> usize {
        self.proof.dimension()
    }
    pub fn matrix(&self) -> &[f64] {
        self.proof.matrix()
    }
    pub fn goal(&self) -> &[f64] {
        self.proof.terminal_gradient()
    }
}
/// Fixed-capacity exact interval replay of the original transformed tableau.
/// All coefficients are represented inputs; no assertion of analytic order
/// beyond the existing integrator. Use the actual proposed h, not a later
/// rounded difference of stored clocks.
pub struct StageDefectWorkspace {
    model: HomogeneousStageModel,
    table: &'static Rodas5pCoefficients,
    wdiag: [Iv; N],
    ch: [Iv; S * S],
    base: [Iv; N],
    lambda: [[Iv; N]; S],
    stage: [[Iv; N]; S],
    errors: [[Iv; N]; S],
    next: usize,
    valid: bool,
    max_lambda_l1: f64,
    max_stage: f64,
    sum_mag: f64,
    sum_signed: Iv,
    uniform: f64,
}
impl StageDefectWorkspace {
    pub fn new(model: HomogeneousStageModel) -> CoreResult<Self> {
        let table = rodas5p_coefficients()?;
        if table.stages() != S {
            return Err(bad("stage tableau size"));
        }
        Ok(Self {
            model,
            table,
            wdiag: [Z; N],
            ch: [Z; S * S],
            base: [Z; N],
            lambda: [[Z; N]; S],
            stage: [[Z; N]; S],
            errors: [[Z; N]; S],
            next: 0,
            valid: false,
            max_lambda_l1: 0.,
            max_stage: 0.,
            sum_mag: 0.,
            sum_signed: Z,
            uniform: 0.,
        })
    }
    fn n(&self) -> usize {
        self.model.dimension()
    }
    fn action(&self, v: &[Iv; N], transpose: bool) -> CoreResult<[Iv; N]> {
        let n = self.n();
        let a = self.model.matrix();
        let mut out = [Z; N];
        for i in 0..n {
            for j in 0..n {
                let c = a[if transpose { j * n + i } else { i * n + j }];
                if c != 0. {
                    out[i] = out[i].add(v[j].scale(c)?)?;
                }
            }
        }
        Ok(out)
    }
    fn t_action(&self, i: usize, j: usize, v: &[Iv; N], transpose: bool) -> CoreResult<[Iv; N]> {
        let n = self.n();
        let a = self.table.a[(i, j)];
        let c = self.ch[i * S + j];
        let mut out = if a == 0. {
            [Z; N]
        } else {
            self.action(v, transpose)?
        };
        for k in 0..n {
            out[k] = out[k].scale(a)?.add(c.mul(v[k])?)?;
        }
        Ok(out)
    }
    fn triangular_iv(&self, rhs: &[Iv; N], transpose: bool) -> CoreResult<[Iv; N]> {
        let n = self.n();
        let a = self.model.matrix();
        let lower = self.model.lower != transpose;
        let mut out = [Z; N];
        for k in 0..n {
            let i = if lower { k } else { n - 1 - k };
            let mut s = rhs[i];
            for q in 0..k {
                let j = if lower { q } else { n - 1 - q };
                let c = a[if transpose { j * n + i } else { i * n + j }];
                if c != 0. {
                    s = s.add(out[j].scale(c)?)?;
                }
            }
            out[i] = s.div(self.wdiag[i])?;
        }
        Ok(out)
    }
    pub fn prepare(&mut self, y: &[f64], h: f64) -> CoreResult<()> {
        self.valid = false;
        let n = self.n();
        if y.len() != n || !h.is_finite() || h <= 0. {
            return Err(bad("stage target shape/step"));
        }
        let mut yp = [Z; N];
        for (i, &v) in y.iter().enumerate() {
            yp[i] = Iv::point(v)?;
        }
        self.base = self.action(&yp, false)?;
        let shift = Iv::point(1.)?.div(Iv::point(h)?.scale(self.table.gamma)?)?;
        for i in 0..n {
            self.wdiag[i] = shift.sub(Iv::point(self.model.matrix()[i * n + i])?)?;
            if self.wdiag[i].lo <= 0. && self.wdiag[i].hi >= 0. {
                return Err(bad("stage target singular diagonal"));
            }
        }
        for i in 0..S {
            for j in 0..i {
                self.ch[i * S + j] = Iv::point(self.table.c_matrix[(i, j)])?.div(Iv::point(h)?)?;
            }
        }
        self.lambda = [[Z; N]; S];
        self.stage = [[Z; N]; S];
        self.errors = [[Z; N]; S];
        self.max_lambda_l1 = 0.;
        for i in (0..S).rev() {
            let mut rhs = [Z; N];
            for (k, &c) in self.model.goal().iter().enumerate() {
                rhs[k] = Iv::point(c)?.scale(self.table.b_code[i])?;
            }
            for j in i + 1..S {
                let v = self.t_action(j, i, &self.lambda[j], true)?;
                for k in 0..n {
                    rhs[k] = rhs[k].add(v[k])?;
                }
            }
            self.lambda[i] = self.triangular_iv(&rhs, true)?;
            let norm = self.lambda[i][..n]
                .iter()
                .try_fold(0., |s, v| add_up(s, v.mag()))?;
            self.max_lambda_l1 = self.max_lambda_l1.max(norm);
        }
        self.next = 0;
        self.max_stage = 0.;
        self.sum_mag = 0.;
        self.sum_signed = Z;
        self.uniform = 0.;
        self.valid = true;
        Ok(())
    }
    fn candidate(&self, u: &[f64]) -> CoreResult<([Iv; N], [Iv; N], Iv, f64, f64)> {
        if !self.valid || self.next >= S || u.len() != self.n() {
            return Err(bad("stage target not prepared"));
        }
        let i = self.next;
        let n = self.n();
        let mut v = [Z; N];
        for (k, &x) in u.iter().enumerate() {
            v[k] = Iv::point(x)?;
        }
        let mut r = self.base;
        for j in 0..i {
            let t = self.t_action(i, j, &self.stage[j], false)?;
            for k in 0..n {
                r[k] = r[k].add(t[k])?;
            }
        }
        let a = self.model.matrix();
        for k in 0..n {
            r[k] = r[k].sub(self.wdiag[k].mul(v[k])?)?;
            for l in 0..n {
                if l != k && a[k * n + l] != 0. {
                    r[k] = r[k].add(v[l].scale(a[k * n + l])?)?;
                }
            }
        }
        let contribution = (0..n).try_fold(Z, |s, k| s.add(self.lambda[i][k].mul(r[k])?))?;
        let uniform = mul_up(
            self.max_lambda_l1,
            r[..n].iter().map(|v| v.mag()).fold(0., f64::max),
        )?;
        let mut erhs = r;
        for j in 0..i {
            let t = self.t_action(i, j, &self.errors[j], false)?;
            for k in 0..n {
                erhs[k] = erhs[k].add(t[k])?;
            }
        }
        let error = self.triangular_iv(&erhs, false)?;
        let mag = error[..n].iter().map(|v| v.mag()).fold(0., f64::max);
        Ok((v, error, contribution, uniform, mag))
    }
    fn record(&mut self, u: &[f64], gate: Option<bool>) -> CoreResult<bool> {
        let (v, e, contribution, uniform, mag) = self.candidate(u)?;
        let goal = if gate == Some(false) {
            uniform
        } else {
            contribution.mag()
        };
        if gate.is_some() && (mag > STAGE_CAP || goal > div_down(GOAL_CAP, S as f64)?) {
            return Ok(false);
        }
        let i = self.next;
        self.stage[i] = v;
        self.errors[i] = e;
        self.max_stage = self.max_stage.max(mag);
        self.sum_signed = self.sum_signed.add(contribution)?;
        self.sum_mag = add_up(self.sum_mag, contribution.mag())?;
        self.uniform = add_up(self.uniform, uniform)?;
        self.next += 1;
        Ok(true)
    }
    /// Independent replay of a complete actual native packet (no gate).
    /// Each accepted output is the exact transformed-stage target error
    /// before final endpoint floating summation, not an IVP error bound.
    pub fn replay_into(
        &mut self,
        y: &[f64],
        h: f64,
        stages: &[&[f64]],
    ) -> CoreResult<StageDefectReceipt> {
        if stages.len() != S {
            return Err(bad("stage replay row count"));
        }
        self.prepare(y, h)?;
        for row in stages {
            self.record(row, None)?;
        }
        self.receipt()
    }
    fn receipt(&self) -> CoreResult<StageDefectReceipt> {
        if !self.valid || self.next != S {
            return Err(bad("stage target incomplete"));
        }
        Ok(StageDefectReceipt {
            max_stage_error_upper: self.max_stage,
            endpoint_goal_error_upper: self.sum_signed.mag(),
            signed_goal_enclosure: self.sum_signed.as_pair(),
            uniform_goal_upper: self.uniform,
            stages: self.next,
        })
    }
}
pub(super) struct StageRequest {
    pub plan: NativeStagePlan,
    pub model: Option<HomogeneousStageModel>,
    pub work: Arc<Mutex<NativeStageWork>>,
}
impl StageRequest {
    pub fn new(src: &DaeSource, plan: NativeStagePlan, goal: &[f64]) -> CoreResult<Self> {
        let model = if matches!(plan, NativeStagePlan::Legacy | NativeStagePlan::ReuseLu) {
            None
        } else {
            Some(HomogeneousStageModel::from_source(src, goal)?)
        };
        Ok(Self {
            plan,
            model,
            work: Arc::new(Mutex::new(NativeStageWork::default())),
        })
    }
    pub fn client(&self, n: usize) -> CoreResult<Box<dyn StageLinearSystem>> {
        if self.plan == NativeStagePlan::ReuseLu {
            return Ok(Box::new(DenseLuReuseStageClient::new(
                n,
                DenseLuMode::InPlace,
            )?));
        }
        let model = self
            .model
            .as_ref()
            .ok_or_else(|| bad("missing native stage source proof"))?
            .clone();
        if model.dimension() != n {
            return Err(bad("native stage model shape"));
        }
        let target = if matches!(
            self.plan,
            NativeStagePlan::GoalUniform | NativeStagePlan::GoalAdjoint
        ) {
            Some(StageDefectWorkspace::new(model.clone())?)
        } else {
            None
        };
        Ok(Box::new(SourceTriangleClient {
            plan: self.plan,
            model,
            diag: [0.; N],
            reciprocal: [0.; N],
            target,
            ready: false,
            work: self.work.clone(),
        }))
    }
}
struct SourceTriangleClient {
    plan: NativeStagePlan,
    model: HomogeneousStageModel,
    diag: [f64; N],
    reciprocal: [f64; N],
    target: Option<StageDefectWorkspace>,
    ready: bool,
    work: Arc<Mutex<NativeStageWork>>,
}
impl SourceTriangleClient {
    fn direct(&self, b: &[f64], out: &mut [f64; N], w: &mut NativeStageWork) {
        let n = self.model.dimension();
        let a = self.model.matrix();
        let lower = self.model.lower;
        for k in 0..n {
            let i = if lower { k } else { n - 1 - k };
            let mut s = b[i];
            for q in 0..k {
                let j = if lower { q } else { n - 1 - q };
                let c = a[i * n + j];
                if c != 0. {
                    s += c * out[j];
                    w.triangular_products += 1;
                }
            }
            out[i] = if self.plan == NativeStagePlan::Reciprocal {
                s * self.reciprocal[i]
            } else {
                s / self.diag[i]
            };
        }
        w.direct_solves += 1;
    }
}
impl StageLinearSystem for SourceTriangleClient {
    fn dimension(&self) -> usize {
        self.model.dimension()
    }
    fn linearize(
        &mut self,
        problem: &OdeProblem,
        _t: f64,
        _y: &[f64],
        _counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        self.ready = false;
        if problem.dimension != self.dimension() || problem.mass_matrix.is_some() {
            return Err(bad("bound triangular problem shape"));
        }
        // Only constructed inside the DAE facade from the identical source.
        self.work.lock().expect("stage work").source_jacobian_skips += 1;
        Ok(())
    }
    fn begin_attempt(&mut self, _t: f64, y: &[f64], h: f64) -> CoreResult<()> {
        self.ready = false;
        if let Some(w) = &mut self.target {
            w.prepare(y, h)?;
        }
        Ok(())
    }
    fn factor(&mut self, inv: f64, _counters: &mut WorkCounters) -> CoreResult<()> {
        let n = self.dimension();
        let a = self.model.matrix();
        self.ready = false;
        for i in 0..n {
            self.diag[i] = -a[i * n + i] + inv;
            if self.diag[i] == 0. || !self.diag[i].is_finite() {
                return Err(CoreError::LinearSolve("triangular diagonal".into()));
            }
            self.reciprocal[i] = 1. / self.diag[i];
        }
        self.work.lock().expect("stage work").preparations += 1;
        self.ready = true;
        Ok(())
    }
    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        let n = self.dimension();
        if !self.ready || b.len() != n {
            return Err(bad("triangular stage not prepared"));
        }
        let mut w = self.work.lock().expect("stage work");
        let mut x = [0.; N];
        if self.target.is_none() {
            self.direct(b, &mut x, &mut w);
            counters.direct_solve_calls += 1;
        } else {
            let gate = Some(self.plan == NativeStagePlan::GoalAdjoint);
            let check = |target: &mut StageDefectWorkspace,
                         v: &[f64],
                         w: &mut NativeStageWork|
             -> CoreResult<bool> {
                w.residual_checks += 1;
                let ok = target.record(v, gate)?;
                if !ok {
                    w.residual_refusals += 1;
                }
                Ok(ok)
            };
            for i in 0..n {
                x[i] = b[i] / self.diag[i];
            }
            w.diagonal_proposals += 1;
            if check(self.target.as_mut().unwrap(), &x[..n], &mut w)? {
                w.diagonal_accepts += 1;
            } else {
                let a = self.model.matrix();
                let mut v = [0.; N];
                for i in 0..n {
                    let mut s = b[i];
                    for j in 0..n {
                        if j != i && a[i * n + j] != 0. {
                            s += a[i * n + j] * x[j];
                            w.triangular_products += 1;
                        }
                    }
                    v[i] = s / self.diag[i];
                }
                w.jacobi_proposals += 1;
                if check(self.target.as_mut().unwrap(), &v[..n], &mut w)? {
                    x = v;
                    w.jacobi_accepts += 1;
                } else {
                    self.direct(b, &mut x, &mut w);
                    counters.direct_solve_calls += 1;
                    if !check(self.target.as_mut().unwrap(), &x[..n], &mut w)? {
                        return Err(CoreError::LinearSolve(
                            "original stage goal/state budget refused".into(),
                        ));
                    }
                }
            }
            let target = self.target.as_ref().unwrap();
            if target.next == S {
                let receipt = target.receipt()?;
                // Check the actual outward accumulated contribution, not
                // only the nominal downward per-stage allocation.
                let goal = if gate == Some(true) {
                    target.sum_mag
                } else {
                    target.uniform
                };
                if goal > GOAL_CAP {
                    return Err(CoreError::LinearSolve(
                        "accumulated stage goal budget".into(),
                    ));
                }
                w.completed_certified_attempts += 1;
                w.max_stage_error_upper =
                    w.max_stage_error_upper.max(receipt.max_stage_error_upper);
                w.max_goal_error_upper = w
                    .max_goal_error_upper
                    .max(receipt.endpoint_goal_error_upper);
                w.max_goal_uniform_upper = w.max_goal_uniform_upper.max(receipt.uniform_goal_upper);
            }
        }
        if x[..n].iter().any(|v| !v.is_finite()) {
            return Err(CoreError::LinearSolve("triangular nonfinite stage".into()));
        }
        b.copy_from_slice(&x[..n]);
        Ok(())
    }
}
