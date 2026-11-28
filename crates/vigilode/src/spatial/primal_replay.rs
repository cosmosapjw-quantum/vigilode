//! Source-owned native forward replay with a certified terminal goal budget.
//! Tolerance changes are proposals; only the actual signed error is authority.
use super::*;
use rodas5p_core::{CoreError, CoreResult, WorkCounters, directed::Interval as Iv};
use rodas5p_integrators::{
    AdaptiveStepConfig, FastArenaPlan, FastArenaReceipt, FastFlatOutputArena, OdeProblem,
    OutputSchedule, ReusableFastClientWorkspace,
};
use serde::Serialize;
use std::sync::Mutex;
fn err(s: &str) -> CoreError {
    CoreError::InvalidInput(format!("spatial primal replay: {s}"))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
/// Explicit replay proposals; none changes the acceptance budget.
pub enum SpatialPrimalReplayPlan {
    /// Tighten all pairs together.
    FixedUniform,
    /// Tighten only refused pairs.
    FixedSelective,
    /// Scale by the observed certified signed error (heuristic).
    GuidedSelective,
    /// Try exact initial centering before tightening a refused pair.
    CenteredSelective,
    /// Center from the first solve; tighten only refused pairs.
    CenteredFirst,
}
#[derive(Clone, Copy, Debug, Serialize)]
/// Bounded replay policy. min_scale multiplies the supplied native tolerances.
pub struct SpatialPrimalReplayPolicy {
    /// Unchanged absolute signed terminal goal budget.
    pub budget: f64,
    /// Maximum additional replay rounds.
    pub max_replays: usize,
    /// Minimum multiplier of both native tolerances.
    pub min_scale: f64,
    /// Proposal policy.
    pub plan: SpatialPrimalReplayPlan,
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
/// Work includes failed budget proposals and all actually executed native IVPs.
pub struct SpatialPrimalReplayWork {
    /// All pairs accepted.
    pub accepted: bool,
    /// Number of accepted pairs.
    pub accepted_pairs: usize,
    /// Actual native IVPs.
    pub solves: usize,
    /// Additional native IVPs after the initial batch.
    pub replays: usize,
    /// Native IVPs with exactly centered initial data.
    pub centered_solves: usize,
    /// Native IVPs that did not reach the terminal time.
    pub primal_stops: usize,
    /// Pairs that could not be tightened further.
    pub floor_refusals: usize,
    /// Actual sparse actions in the cached certificate.
    pub adjoint_actions: usize,
    /// Largest final signed error magnitude; MAX if incomplete.
    pub max_signed_error_upper: f64,
    /// Smallest tolerance multiplier executed.
    pub min_scale_used: f64,
    /// Sum of native solver counters.
    pub counters: WorkCounters,
    /// Total Fourier stage solves.
    pub stage_solves: usize,
    /// Total backward residual checks.
    pub stage_checks: usize,
    /// Total stage defect corrections.
    pub stage_corrections: usize,
}
/// Borrowed diagnostic of the final numerical proposal for one pair.
#[derive(Debug, Serialize)]
pub struct SpatialPrimalPair<'a> {
    /// Actual represented physical endpoint.
    pub endpoint: &'a [f64],
    /// Cached exact goal and final signed error enclosure.
    pub receipt: SpatialAdjointWork,
    /// Whether this individual goal meets the unchanged budget.
    pub accepted: bool,
    /// Multiplier of the original native tolerances.
    pub scale: f64,
    /// Whether the last solve actually used exact initial centering.
    pub centered: bool,
    /// Native replays after the first proposal for this pair.
    pub retries: usize,
}
#[derive(Clone, Copy, Debug)]
struct PairState {
    complete: bool,
    accepted: bool,
    scale: f64,
    centered: bool,
    used_center: bool,
    error: f64,
    retries: usize,
    floor: bool,
}
impl Default for PairState {
    fn default() -> Self {
        Self {
            complete: false,
            accepted: false,
            scale: 1.,
            centered: false,
            used_center: false,
            error: f64::MAX,
            retries: 0,
            floor: false,
        }
    }
}
/// Fixed source, Fourier stage client, adjoint certificate and reusable storage.
pub struct SpatialPrimalReplayWorkspace {
    n: usize,
    capacity: usize,
    duration: f64,
    problem: OdeProblem,
    driver: ReusableFastClientWorkspace,
    report: Arc<Mutex<SeparableReport>>,
    report_template: SeparableReport,
    schedule: OutputSchedule,
    arena: FastFlatOutputArena,
    atol: Vec<f64>,
    centered: Vec<f64>,
    endpoints: Vec<f64>,
    receipts: Vec<SpatialAdjointWork>,
    states: Vec<PairState>,
    last_count: usize,
    adjoint: SpatialAdjointBatchWorkspace,
    gradient: Vec<Iv>,
}
impl SpatialPrimalReplayWorkspace {
    /// Setup allocates the fixed-size arena and the solver buffers.
    pub fn new(grid: &DiffusionGrid, duration: f64, capacity: usize) -> CoreResult<Self> {
        let n = grid.dimension();
        if !duration.is_finite() || duration <= 0. || capacity == 0 || capacity > 4096 {
            return Err(err("duration/capacity"));
        }
        let size = n
            .checked_mul(capacity)
            .ok_or_else(|| err("batch dimension overflow"))?;
        let g = grid.clone();
        let j = grid.clone();
        let problem = OdeProblem::new(
            "represented spatial forward",
            n,
            Arc::new(move |_, u, o| {
                g.apply(u, o, SpatialRows::Serial)
                    .map_err(|e| err(&e.to_string()))
            }),
            None,
            None,
            Some(Arc::new(move |_, _, v, o| {
                j.apply(v, o, SpatialRows::Serial)
                    .map_err(|e| err(&e.to_string()))
            })),
            None,
            true,
            None,
            None,
        )?;
        let stages = grid.separable_stages(0.).map_err(|e| err(&e.to_string()))?;
        let report = Arc::new(Mutex::new(SeparableReport::default()));
        let driver = ReusableFastClientWorkspace::new(stages.client(report.clone()))?;
        let report_template = *report.lock().unwrap();
        Ok(Self {
            n,
            capacity,
            duration,
            problem,
            driver,
            report,
            report_template,
            schedule: OutputSchedule::new(vec![0., duration])?,
            arena: FastFlatOutputArena::new(n, 2)?,
            atol: vec![0.; n],
            centered: vec![0.; n],
            endpoints: vec![0.; size],
            receipts: vec![SpatialAdjointWork::default(); capacity],
            states: vec![PairState::default(); capacity],
            last_count: 0,
            adjoint: SpatialAdjointBatchWorkspace::new(grid, capacity)?,
            gradient: vec![Iv::point(0.)?; n],
        })
    }
    fn solve_pair(
        &mut self,
        i: usize,
        initial: &[f64],
        base: &AdaptiveStepConfig,
        work: &mut SpatialPrimalReplayWork,
    ) -> CoreResult<FastArenaReceipt> {
        let state = &mut self.states[i];
        let u = &initial[i * self.n..(i + 1) * self.n];
        let mut shift = 0.;
        state.used_center = false;
        if state.centered {
            let lo = u.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = u.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let s = 0.5 * lo + 0.5 * hi;
            let mut exact = true;
            for (&v, z) in u.iter().zip(&mut self.centered) {
                let p = Iv::point(v)?.sub(Iv::point(s)?)?;
                if p.lo != p.hi {
                    exact = false;
                    break;
                }
                *z = p.lo;
            }
            if exact {
                shift = s;
                state.used_center = true;
                work.centered_solves += 1;
            } else {
                self.centered.copy_from_slice(u);
            }
        } else {
            self.centered.copy_from_slice(u);
        }
        let mut cfg = base.clone();
        cfg.rtol *= state.scale;
        cfg.atol *= state.scale;
        cfg.validate()?;
        self.atol.fill(cfg.atol);
        *self.report.lock().unwrap() = self.report_template;
        let run = self.driver.solve_flat_into(
            &self.problem,
            (0., self.duration),
            &self.centered,
            &cfg,
            &self.schedule,
            &self.atol,
            FastArenaPlan::Legacy,
            &mut self.arena,
        )?;
        let receipt = run.receipt;
        let report = *self.report.lock().unwrap();
        work.solves += 1;
        work.counters.accumulate(receipt.counters);
        work.stage_solves += report.solves;
        work.stage_checks += report.residual_checks;
        work.stage_corrections += report.corrections;
        work.min_scale_used = work.min_scale_used.min(state.scale);
        state.complete = receipt.success;
        state.accepted = false;
        if !receipt.success {
            work.primal_stops += 1;
            state.error = f64::MAX;
            return Ok(receipt);
        }
        let endpoint = &mut self.endpoints[i * self.n..(i + 1) * self.n];
        for (o, &v) in endpoint.iter_mut().zip(run.last_state) {
            *o = v + shift;
        }
        if endpoint.iter().any(|v| !v.is_finite()) {
            return Err(err("nonfinite restored endpoint"));
        }
        Ok(receipt)
    }
    fn assess(&mut self, i: usize, goal: &[f64], budget: f64) -> CoreResult<()> {
        let state = &mut self.states[i];
        if !state.complete {
            return Ok(());
        }
        let end = &self.endpoints[i * self.n..(i + 1) * self.n];
        let mut num = Iv::point(0.)?;
        for (&c, &v) in goal.iter().zip(end) {
            num = num.add(Iv::point(c)?.scale(v)?)?;
        }
        let e = self.receipts[i].exact_goal;
        let signed = Iv::new(e[0], e[1])?.sub(num)?;
        self.receipts[i].signed_goal_error = signed.as_pair();
        state.error = signed.mag();
        state.accepted = state.error <= budget;
        Ok(())
    }
    /// Diagnostic of a completed pair from the last returned batch, including
    /// a budget-refused numerical candidate. Not a committed output.
    pub fn last_pair(&self, i: usize) -> Option<SpatialPrimalPair<'_>> {
        if i >= self.last_count || !self.states[i].complete {
            return None;
        }
        let s = self.states[i];
        Some(SpatialPrimalPair {
            endpoint: &self.endpoints[i * self.n..(i + 1) * self.n],
            receipt: self.receipts[i],
            accepted: s.accepted,
            scale: s.scale,
            centered: s.used_center,
            retries: s.retries,
        })
    }
    /// Same source and initial data for all proposals. Caller outputs are
    /// committed only when every pair meets its unchanged signed goal budget.
    pub fn solve_budgeted_into(
        &mut self,
        initial: &[f64],
        goal: &[f64],
        base: &AdaptiveStepConfig,
        policy: SpatialPrimalReplayPolicy,
        endpoint: &mut [f64],
        receipts: &mut [SpatialAdjointWork],
    ) -> CoreResult<SpatialPrimalReplayWork> {
        self.last_count = 0;
        let k = receipts.len();
        if k == 0
            || k > self.capacity
            || self.n.checked_mul(k) != Some(initial.len())
            || endpoint.len() != initial.len()
            || goal.len() != self.n
            || initial.iter().chain(goal).any(|x| !x.is_finite())
            || !policy.budget.is_finite()
            || policy.budget <= 0.
            || !policy.min_scale.is_finite()
            || policy.min_scale <= 0.
            || policy.min_scale > 1.
            || policy.max_replays > 32
        {
            return Err(err("replay input/policy"));
        }
        base.validate()?;
        let mut work = SpatialPrimalReplayWork {
            min_scale_used: 1.,
            ..Default::default()
        };
        self.states[..k].fill(PairState::default());
        // Exactly one shared action for this source, goal and duration.
        // The temporary endpoint equals initial only to obtain exact_goal.
        let a = self.adjoint.certify_batch_into(
            goal,
            initial,
            initial,
            self.duration,
            SpatialAdjointRoute::PoissonWideDecay,
            SpatialBatchPlan::SharedCenteredIntersect,
            &mut self.gradient,
            &mut self.receipts[..k],
        )?;
        work.adjoint_actions = a.actions;
        for i in 0..k {
            self.states[i].centered = policy.plan == SpatialPrimalReplayPlan::CenteredFirst;
            self.solve_pair(i, initial, base, &mut work)?;
            self.assess(i, goal, policy.budget)?;
        }
        for _ in 0..policy.max_replays {
            if self.states[..k].iter().all(|s| s.accepted) {
                break;
            }
            let mut any = false;
            for i in 0..k {
                let s = &mut self.states[i];
                if (s.accepted && policy.plan != SpatialPrimalReplayPlan::FixedUniform) || s.floor {
                    continue;
                }
                if policy.plan == SpatialPrimalReplayPlan::CenteredSelective && !s.centered {
                    s.centered = true;
                } else {
                    let guided = matches!(
                        policy.plan,
                        SpatialPrimalReplayPlan::GuidedSelective
                            | SpatialPrimalReplayPlan::CenteredSelective
                            | SpatialPrimalReplayPlan::CenteredFirst
                    );
                    let factor = if guided {
                        (0.5 * policy.budget / s.error).clamp(0.0625, 0.5)
                    } else {
                        0.25
                    };
                    let next = (s.scale * factor).max(policy.min_scale);
                    if next >= s.scale {
                        s.floor = true;
                        work.floor_refusals += 1;
                        continue;
                    }
                    s.scale = next;
                }
                s.retries += 1;
                work.replays += 1;
                any = true;
                self.solve_pair(i, initial, base, &mut work)?;
                self.assess(i, goal, policy.budget)?;
            }
            if !any {
                break;
            }
        }
        work.accepted_pairs = self.states[..k].iter().filter(|s| s.accepted).count();
        work.accepted = work.accepted_pairs == k;
        work.max_signed_error_upper = self.states[..k].iter().map(|s| s.error).fold(0., f64::max);
        if work.accepted {
            endpoint.copy_from_slice(&self.endpoints[..initial.len()]);
            receipts.copy_from_slice(&self.receipts[..k]);
        }
        self.last_count = k;
        Ok(work)
    }
}
