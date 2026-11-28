//! Source-owned native forward replay with a certified terminal goal budget.
//! Tolerance changes are proposals; only the actual signed error is authority.
use super::adjoint::SpatialDualCache;
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
/// Cross-call reuse policy. Source and duration are owned by the workspace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SpatialReplayCachePlan {
    /// Original one-action-per-batch path.
    Off,
    /// Reuse exact projected goals only for identical ordered input bits.
    ExactBatch,
    /// Reuse certified covector for identical goal, reproject new initial data.
    GoalOnly,
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
    /// Exact-key certificate cache hit.
    pub cache_hit: bool,
    /// Goal projections actually evaluated this call.
    pub goal_projections: usize,
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
    dual_cache: SpatialDualCache,
    cached_goal: Vec<f64>,
    cached_initial: Vec<f64>,
    cached_receipts: Vec<SpatialAdjointWork>,
    cached_count: usize,
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
            dual_cache: SpatialDualCache::new(grid)?,
            cached_goal: vec![0.; n],
            cached_initial: vec![0.; size],
            cached_receipts: vec![SpatialAdjointWork::default(); capacity],
            cached_count: 0,
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
        self.solve_cached_budgeted_into(
            initial,
            goal,
            base,
            policy,
            SpatialReplayCachePlan::Off,
            endpoint,
            receipts,
        )
    }
    /// Opt-in cross-call cache; all original endpoint and budget semantics remain.
    pub fn solve_cached_budgeted_into(
        &mut self,
        initial: &[f64],
        goal: &[f64],
        base: &AdaptiveStepConfig,
        policy: SpatialPrimalReplayPolicy,
        cache: SpatialReplayCachePlan,
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
        match cache {
            SpatialReplayCachePlan::GoalOnly => {
                let (actions, hit) = self.dual_cache.prepare(goal, self.duration)?;
                work.adjoint_actions = actions;
                work.cache_hit = hit;
                work.goal_projections = k;
                for i in 0..k {
                    self.receipts[i] = self
                        .dual_cache
                        .project(&initial[i * self.n..(i + 1) * self.n])?;
                }
            }
            SpatialReplayCachePlan::ExactBatch
                if self.cached_count == k
                    && self
                        .cached_goal
                        .iter()
                        .zip(goal)
                        .all(|(a, b)| a.to_bits() == b.to_bits())
                    && self.cached_initial[..initial.len()]
                        .iter()
                        .zip(initial)
                        .all(|(a, b)| a.to_bits() == b.to_bits()) =>
            {
                self.receipts[..k].copy_from_slice(&self.cached_receipts[..k]);
                work.cache_hit = true;
            }
            _ => {
                if cache == SpatialReplayCachePlan::ExactBatch {
                    self.cached_count = 0;
                }
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
                work.goal_projections = k;
                if cache == SpatialReplayCachePlan::ExactBatch {
                    self.cached_goal.copy_from_slice(goal);
                    self.cached_initial[..initial.len()].copy_from_slice(initial);
                    self.cached_receipts[..k].copy_from_slice(&self.receipts[..k]);
                    self.cached_count = k;
                }
            }
        }
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

/// Four exact represented macro cells; native accepted dense packets are
/// retained within each. Each retry replaces a suffix from a shared endpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SpatialLocalReplayPlan {
    /// Uniform tolerance reduction on all cells.
    Uniform,
    /// Refine the largest magnitude integrated dual defect.
    MaxMagnitude,
    /// Prefer a contribution aligned with the total signed goal error.
    SignedAligned,
    /// Divide the magnitude score by the observed suffix step cost.
    SuffixWeighted,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct SpatialLocalReplayPolicy {
    /// Unchanged signed absolute terminal budget.
    pub budget: f64,
    /// Maximum additional suffix rounds.
    pub max_replays: usize,
    /// Minimum multiplier of native tolerances.
    pub min_scale: f64,
    /// Local refinement proposal.
    pub plan: SpatialLocalReplayPlan,
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct SpatialLocalReplayWork {
    /// Completed and signed budget accepted.
    pub accepted: bool,
    /// Actual final numerical endpoint available.
    pub completed: bool,
    /// Actual macro IVP solves.
    pub solves: usize,
    /// Additional suffix rounds executed.
    pub replay_rounds: usize,
    /// Count of tolerance reductions.
    pub refined_cells: usize,
    /// Additional macro IVPs executed.
    pub replayed_cells: usize,
    /// Sum of preserved macro cells over replay rounds.
    pub prefix_cells_reused: usize,
    /// Actual accepted packets, including discarded proposals.
    pub dense_packets: usize,
    /// New certified semigroup sparse actions.
    pub adjoint_actions: usize,
    /// Prepared boundary covectors reused.
    pub cache_hits: usize,
    /// Boundary projections computed.
    pub projections: usize,
    /// Refusal because no tighter tolerance is admitted.
    pub floor_refusals: usize,
    /// Native stops including bounded packet capacity.
    pub primal_stops: usize,
    /// Accumulated native work.
    pub counters: WorkCounters,
    /// Accumulated Fourier stage solves.
    pub stage_solves: usize,
    /// Final direct signed error if completed.
    pub signed_goal_error: Option<[f64; 2]>,
    /// Final magnitude upper or MAX on incomplete.
    pub max_signed_error_upper: f64,
    /// Per-cell tolerance refinement counts.
    pub selected: [usize; 4],
}
/// Fixed-source, bounded native packet and dual projection workspace.
/// Local replay starts from a retained represented prefix, not an exact
/// physical restart; the original initial condition and final gate stay fixed.
pub struct SpatialLocalReplayWorkspace {
    primal: SpatialPrimalReplayWorkspace,
    times: [f64; 5],
    schedules: Vec<OutputSchedule>,
    captures: Vec<rodas5p_integrators::FastDenseCapture>,
    dual: Vec<SpatialDualCache>,
    states: Vec<f64>,
    projections: [Iv; 5],
    defects: [Iv; 4],
    scales: [f64; 4],
    steps: [usize; 4],
    valid_cells: usize,
    has_initial: bool,
    last_receipt: Option<SpatialAdjointWork>,
}
impl SpatialLocalReplayWorkspace {
    /// Allocate four bounded native packet arenas and exact dual caches.
    pub fn new(
        grid: &DiffusionGrid,
        duration: f64,
        max_packets_per_cell: usize,
    ) -> CoreResult<Self> {
        if !duration.is_finite() || duration <= 0. {
            return Err(err("local duration"));
        }
        let times = [
            0.,
            duration * 0.25,
            duration * 0.5,
            duration * 0.75,
            duration,
        ];
        if times.windows(2).any(|x| x[0] >= x[1]) {
            return Err(err("local degenerate macro clock"));
        }
        for &t in &times {
            let d = Iv::point(duration)?.sub(Iv::point(t)?)?;
            if d.lo != d.hi {
                return Err(err("local nonrepresentable adjoint duration"));
            }
        }
        let n = grid.dimension();
        let size = n.checked_mul(5).ok_or_else(|| err("local state size"))?;
        let mut schedules = Vec::with_capacity(4);
        let mut captures = Vec::with_capacity(4);
        let mut dual = Vec::with_capacity(4);
        for j in 0..4 {
            schedules.push(OutputSchedule::new(vec![times[j], times[j + 1]])?);
            captures.push(rodas5p_integrators::FastDenseCapture::new(
                n,
                max_packets_per_cell,
            )?);
            dual.push(SpatialDualCache::new(grid)?);
        }
        Ok(Self {
            primal: SpatialPrimalReplayWorkspace::new(grid, duration, 1)?,
            times,
            schedules,
            captures,
            dual,
            states: vec![0.; size],
            projections: [Iv::point(0.)?; 5],
            defects: [Iv::point(0.)?; 4],
            scales: [1.; 4],
            steps: [0; 4],
            valid_cells: 0,
            has_initial: false,
            last_receipt: None,
        })
    }
    /// Committed macro prefix times.
    pub fn times(&self) -> &[f64] {
        if self.has_initial {
            &self.times[..=self.valid_cells]
        } else {
            &[]
        }
    }
    /// Flat committed macro prefix states.
    pub fn states(&self) -> &[f64] {
        if self.has_initial {
            &self.states[..(self.valid_cells + 1) * self.primal.n]
        } else {
            &[]
        }
    }
    /// Directed dual projections of the prefix.
    pub fn projections(&self) -> &[Iv] {
        if self.has_initial {
            &self.projections[..=self.valid_cells]
        } else {
            &[]
        }
    }
    /// Directed integrated residual contributions for completed cells.
    pub fn defects(&self) -> &[Iv] {
        &self.defects[..self.valid_cells]
    }
    /// Actual current tolerance multipliers.
    pub fn scales(&self) -> [f64; 4] {
        self.scales
    }
    /// Native dense packets of a completed cell.
    pub fn dense_cell(&self, k: usize) -> Option<&rodas5p_integrators::FastDenseSegments> {
        (k < self.valid_cells).then(|| self.captures[k].segments())
    }
    /// Last completed numerical proposal certificate, not necessarily accepted.
    pub fn last_receipt(&self) -> Option<SpatialAdjointWork> {
        self.last_receipt
    }
    fn run_suffix(
        &mut self,
        start: usize,
        goal: &[f64],
        base: &AdaptiveStepConfig,
        work: &mut SpatialLocalReplayWork,
    ) -> CoreResult<bool> {
        let n = self.primal.n;
        self.valid_cells = start;
        for k in start..4 {
            let mut cfg = base.clone();
            cfg.rtol *= self.scales[k];
            cfg.atol *= self.scales[k];
            cfg.validate()?;
            self.primal.atol.fill(cfg.atol);
            *self.primal.report.lock().unwrap() = self.primal.report_template;
            let (prefix, suffix) = self.states.split_at_mut((k + 1) * n);
            let input = &prefix[k * n..(k + 1) * n];
            let run = self.primal.driver.solve_flat_dense_into(
                &self.primal.problem,
                (self.times[k], self.times[k + 1]),
                input,
                &cfg,
                &self.schedules[k],
                &self.primal.atol,
                FastArenaPlan::Legacy,
                &mut self.primal.arena,
                &mut self.captures[k],
            )?;
            let receipt = run.receipt;
            let report = *self.primal.report.lock().unwrap();
            work.solves += 1;
            work.counters.accumulate(receipt.counters);
            work.stage_solves += report.solves;
            let seg = self.captures[k].segments();
            work.dense_packets += seg.len();
            if !receipt.success {
                work.primal_stops += 1;
                return Ok(false);
            }
            if seg.len() != receipt.accepted_steps
                || seg.is_empty()
                || seg.state(0) != input
                || seg.state(seg.len()) != run.last_state
                || seg.times()[0] != self.times[k]
                || seg.times()[seg.len()] != self.times[k + 1]
            {
                return Err(err("native dense packet identity"));
            }
            suffix[..n].copy_from_slice(run.last_state);
            self.steps[k] = seg.len();
            let end = if k == 3 {
                goal.iter()
                    .zip(&suffix[..n])
                    .try_fold(Iv::point(0.)?, |a, (&c, &x)| a.add(Iv::point(c)?.scale(x)?))?
            } else {
                let r = self.dual[k + 1].project(&suffix[..n])?;
                Iv::new(r.exact_goal[0], r.exact_goal[1])?
            };
            work.projections += 1;
            self.projections[k + 1] = end;
            self.defects[k] = self.projections[k].sub(end)?;
            self.valid_cells = k + 1;
        }
        Ok(true)
    }
    /// Final direct signed budget remains authority. Diagnostic local defects
    /// enclose the integrated continuous adjoint residual over each macro cell.
    pub fn solve_into(
        &mut self,
        initial: &[f64],
        goal: &[f64],
        base: &AdaptiveStepConfig,
        policy: SpatialLocalReplayPolicy,
        endpoint: &mut [f64],
        out: &mut SpatialAdjointWork,
    ) -> CoreResult<SpatialLocalReplayWork> {
        let n = self.primal.n;
        self.valid_cells = 0;
        self.has_initial = false;
        self.last_receipt = None;
        if initial.len() != n
            || goal.len() != n
            || endpoint.len() != n
            || initial.iter().chain(goal).any(|x| !x.is_finite())
            || !policy.budget.is_finite()
            || policy.budget <= 0.
            || !policy.min_scale.is_finite()
            || policy.min_scale <= 0.
            || policy.min_scale > 1.
            || policy.max_replays > 32
        {
            return Err(err("local input/policy"));
        }
        base.validate()?;
        let mut work = SpatialLocalReplayWork {
            max_signed_error_upper: f64::MAX,
            ..Default::default()
        };
        for k in 0..4 {
            let d = Iv::point(self.times[4])?.sub(Iv::point(self.times[k])?)?;
            if d.lo != d.hi {
                return Err(err("local adjoint clock"));
            }
            let (actions, hit) = self.dual[k].prepare(goal, d.lo)?;
            work.adjoint_actions += actions;
            work.cache_hits += usize::from(hit);
        }
        self.states[..n].copy_from_slice(initial);
        self.scales = [1.; 4];
        self.steps = [0; 4];
        let mut rec = self.dual[0].project(initial)?;
        self.projections[0] = Iv::new(rec.exact_goal[0], rec.exact_goal[1])?;
        work.projections = 1;
        self.has_initial = true;
        if !self.run_suffix(0, goal, base, &mut work)? {
            return Ok(work);
        }
        for round in 0..=policy.max_replays {
            let signed = self.projections[0].sub(self.projections[4])?;
            work.completed = true;
            work.max_signed_error_upper = signed.mag();
            work.signed_goal_error = Some(signed.as_pair());
            rec.signed_goal_error = signed.as_pair();
            self.last_receipt = Some(rec);
            if signed.mag() <= policy.budget {
                work.accepted = true;
                endpoint.copy_from_slice(&self.states[4 * n..5 * n]);
                *out = rec;
                break;
            }
            if round == policy.max_replays {
                break;
            }
            let start;
            if policy.plan == SpatialLocalReplayPlan::Uniform {
                let mut first = None;
                for i in 0..4 {
                    let next = (0.25 * self.scales[i]).max(policy.min_scale);
                    if next < self.scales[i] {
                        self.scales[i] = next;
                        work.refined_cells += 1;
                        work.selected[i] += 1;
                        first.get_or_insert(i);
                    }
                }
                let Some(i) = first else {
                    work.floor_refusals += 1;
                    break;
                };
                start = i;
            } else {
                let mut best = None;
                let mut best_score = -1.;
                for i in 0..4 {
                    if self.scales[i] <= policy.min_scale {
                        continue;
                    }
                    let v = self.defects[i];
                    let score = match policy.plan {
                        SpatialLocalReplayPlan::SignedAligned if signed.lo > 0. => v.lo.max(0.),
                        SpatialLocalReplayPlan::SignedAligned if signed.hi < 0. => (-v.hi).max(0.),
                        SpatialLocalReplayPlan::SuffixWeighted => {
                            v.mag() / (self.steps[i..].iter().sum::<usize>().max(1) as f64)
                        }
                        _ => v.mag(),
                    };
                    if score > best_score {
                        best_score = score;
                        best = Some(i);
                    }
                }
                let Some(mut i) = best else {
                    work.floor_refusals += 1;
                    break;
                };
                if best_score == 0. {
                    i = (0..4)
                        .filter(|&j| self.scales[j] > policy.min_scale)
                        .max_by(|&a, &b| self.defects[a].mag().total_cmp(&self.defects[b].mag()))
                        .unwrap_or(i);
                }
                let next = (self.scales[i] * 0.25).max(policy.min_scale);
                if next >= self.scales[i] {
                    work.floor_refusals += 1;
                    break;
                }
                self.scales[i] = next;
                work.selected[i] += 1;
                work.refined_cells += 1;
                start = i;
            }
            work.replay_rounds += 1;
            work.prefix_cells_reused += start;
            work.completed = false;
            work.signed_goal_error = None;
            work.max_signed_error_upper = f64::MAX;
            self.last_receipt = None;
            let before = work.solves;
            let complete = self.run_suffix(start, goal, base, &mut work)?;
            work.replayed_cells += work.solves - before;
            if !complete {
                break;
            }
        }
        Ok(work)
    }
}

/// Partition proposal; accepted-step partitions start from an uninterrupted
/// native pilot and introduce no additional hard stops in that pilot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SpatialCompactPartition {
    /// Four prescribed macro intervals.
    Fixed4,
    /// Two groups of actual pilot steps.
    Accepted2,
    /// Four groups of actual pilot steps.
    Accepted4,
    /// Eight groups of actual pilot steps, capped by actual step count.
    Accepted8,
}
/// Choose one or multiple local cells for the next suffix retry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SpatialCompactRefinement {
    /// Tighten one largest defect.
    Single,
    /// Tighten all eligible cells.
    Uniform,
    /// Tighten multiple cells above a budget share.
    MultiMagnitude,
    /// Cost-weighted selection with multiple eligible large defects.
    MultiCost,
}
#[derive(Clone, Copy, Debug, Serialize)]
/// Bounded compact replay policy; the final signed budget is fixed.
pub struct SpatialCompactPolicy {
    /// Unchanged absolute signed terminal goal budget.
    pub budget: f64,
    /// Maximum extra suffix rounds.
    pub max_replays: usize,
    /// Smallest tolerance multiplier.
    pub min_scale: f64,
    /// Initial partition proposal.
    pub partition: SpatialCompactPartition,
    /// Boundary enclosure construction.
    pub dual: SpatialBoundaryDualPlan,
    /// Cell selection proposal.
    pub refinement: SpatialCompactRefinement,
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
/// Actual work, including the initial pilot and all replayed IVPs.
pub struct SpatialCompactWork {
    /// All terminal acceptance conditions met.
    pub accepted: bool,
    /// A complete numerical proposal is available.
    pub completed: bool,
    /// Number of macro cells.
    pub cells: usize,
    /// Actual native IVPs executed.
    pub solves: usize,
    /// Actual initial IVPs, including pilot.
    pub initial_solves: usize,
    /// Actual extra native IVPs, including partial stops.
    pub replayed_cells: usize,
    /// Number of suffix replay rounds attempted.
    pub replay_rounds: usize,
    /// Number of tolerance reductions.
    pub refined_cells: usize,
    /// Preserved cells summed over replay rounds.
    pub prefix_cells_reused: usize,
    /// Accepted native checkpoints, including discarded proposals.
    pub accepted_packets: usize,
    /// Accepted checkpoints of the uninterrupted pilot.
    pub pilot_packets: usize,
    /// New sparse adjoint actions.
    pub adjoint_actions: usize,
    /// Exact goal and boundary time key reused.
    pub dual_cache_hit: bool,
    /// Near-stationary residual route admitted.
    pub stationary_used: bool,
    /// Outward l1 original transpose residual.
    pub stationary_residual: f64,
    /// Actual directed boundary projections.
    pub projections: usize,
    /// No admissible tighter tolerance.
    pub floor_refusals: usize,
    /// Native partial stops.
    pub primal_stops: usize,
    /// Actual accumulated native counters.
    pub counters: WorkCounters,
    /// Actual native stage solves.
    pub stage_solves: usize,
    /// Direct signed centered terminal error, if complete.
    pub signed_goal_error: Option<[f64; 2]>,
    /// Upper magnitude, or MAX on incomplete.
    pub max_signed_error_upper: f64,
    /// Per-cell refinement counts.
    pub selected: [usize; 8],
}
/// A single bounded accepted-checkpoint buffer and one boundary propagator.
/// No full dense stage packets are retained: the integrated residual uses
/// their exact endpoint identity, already established on the native method.
pub struct SpatialCompactReplayWorkspace {
    primal: SpatialPrimalReplayWorkspace,
    capture: rodas5p_integrators::FastCheckpointCapture,
    dual: super::adjoint::SpatialBoundaryDual,
    times: [f64; 9],
    states: Vec<f64>,
    projections: [Iv; 9],
    defects: [Iv; 8],
    scales: [f64; 8],
    steps: [usize; 8],
    pilot_times: Vec<f64>,
    pilot_indices: [usize; 9],
    cells: usize,
    valid_cells: usize,
    has_initial: bool,
    anchor: f64,
    last_receipt: Option<SpatialAdjointWork>,
}
impl SpatialCompactReplayWorkspace {
    /// Allocate fixed checkpoint and boundary storage.
    pub fn new(g: &DiffusionGrid, duration: f64, capacity: usize) -> CoreResult<Self> {
        let n = g.dimension();
        let primal = SpatialPrimalReplayWorkspace::new(g, duration, 1)?;
        Ok(Self {
            primal,
            capture: rodas5p_integrators::FastCheckpointCapture::new(n, capacity)?,
            dual: super::adjoint::SpatialBoundaryDual::new(g)?,
            times: [0.; 9],
            states: vec![0.; 9 * n],
            projections: [Iv::point(0.)?; 9],
            defects: [Iv::point(0.)?; 8],
            scales: [1.; 8],
            steps: [0; 8],
            pilot_times: Vec::with_capacity(capacity + 1),
            pilot_indices: [0; 9],
            cells: 0,
            valid_cells: 0,
            has_initial: false,
            anchor: 0.,
            last_receipt: None,
        })
    }
    /// Valid represented boundary times.
    pub fn times(&self) -> &[f64] {
        if self.has_initial {
            &self.times[..=self.valid_cells]
        } else {
            &[]
        }
    }
    /// Valid physical boundary states in flat storage.
    pub fn states(&self) -> &[f64] {
        if self.has_initial {
            &self.states[..(self.valid_cells + 1) * self.primal.n]
        } else {
            &[]
        }
    }
    /// Common-anchor directed boundary projections.
    pub fn projections(&self) -> &[Iv] {
        if self.has_initial {
            &self.projections[..=self.valid_cells]
        } else {
            &[]
        }
    }
    /// Signed integrated macro-cell defects.
    pub fn defects(&self) -> &[Iv] {
        if self.has_initial {
            &self.defects[..self.valid_cells]
        } else {
            &[]
        }
    }
    /// Current per-cell native tolerance scales.
    pub fn scales(&self) -> &[f64] {
        &self.scales[..self.cells]
    }
    /// Actual uninterrupted pilot checkpoint times.
    pub fn pilot_times(&self) -> &[f64] {
        &self.pilot_times
    }
    /// Indices of selected actual pilot checkpoints.
    pub fn pilot_indices(&self) -> &[usize] {
        if self.pilot_times.is_empty() {
            &[]
        } else {
            &self.pilot_indices[..=self.cells]
        }
    }
    /// One represented constant removed from every projection.
    pub fn anchor(&self) -> f64 {
        self.anchor
    }
    /// Last complete proposal certificate, even when budget-refused.
    pub fn last_receipt(&self) -> Option<SpatialAdjointWork> {
        self.last_receipt
    }
    fn native(
        &mut self,
        start: f64,
        end: f64,
        scale: f64,
        base: &AdaptiveStepConfig,
        work: &mut SpatialCompactWork,
        replay: bool,
    ) -> CoreResult<bool> {
        let mut cfg = base.clone();
        cfg.rtol *= scale;
        cfg.atol *= scale;
        cfg.validate()?;
        self.primal.atol.fill(cfg.atol);
        self.primal.schedule.reset_pair(start, end)?;
        *self.primal.report.lock().unwrap() = self.primal.report_template;
        let r = self.primal.driver.solve_flat_checkpoints_into(
            &self.primal.problem,
            (start, end),
            &self.primal.centered,
            &cfg,
            &self.primal.schedule,
            &self.primal.atol,
            FastArenaPlan::Legacy,
            &mut self.primal.arena,
            &mut self.capture,
        )?;
        work.solves += 1;
        if replay {
            work.replayed_cells += 1;
        } else {
            work.initial_solves += 1;
        }
        work.counters.accumulate(r.receipt.counters);
        work.stage_solves += self.primal.report.lock().unwrap().solves;
        work.accepted_packets += self.capture.len();
        if !r.receipt.success {
            work.primal_stops += 1;
            return Ok(false);
        }
        if self.capture.len() != r.receipt.accepted_steps
            || self.capture.is_empty()
            || self.capture.times()[0] != start
            || self.capture.times()[self.capture.len()] != end
            || self.capture.state(0) != Some(&self.primal.centered[..])
            || self.capture.state(self.capture.len()) != Some(r.last_state)
        {
            return Err(err("checkpoint identity"));
        }
        Ok(true)
    }
    fn suffix(
        &mut self,
        start: usize,
        base: &AdaptiveStepConfig,
        work: &mut SpatialCompactWork,
        replay: bool,
    ) -> CoreResult<bool> {
        let n = self.primal.n;
        self.valid_cells = start;
        for i in start..self.cells {
            self.primal
                .centered
                .copy_from_slice(&self.states[i * n..(i + 1) * n]);
            if !self.native(
                self.times[i],
                self.times[i + 1],
                self.scales[i],
                base,
                work,
                replay,
            )? {
                return Ok(false);
            }
            self.steps[i] = self.capture.len();
            self.states[(i + 1) * n..(i + 2) * n]
                .copy_from_slice(self.capture.state(self.capture.len()).unwrap());
            self.valid_cells = i + 1;
        }
        Ok(true)
    }
    fn project_suffix(&mut self, start: usize, work: &mut SpatialCompactWork) -> CoreResult<()> {
        let n = self.primal.n;
        for i in start..=self.valid_cells {
            self.projections[i] =
                self.dual
                    .project(i, &self.states[i * n..(i + 1) * n], self.anchor)?;
            work.projections += 1;
            if i > 0 {
                self.defects[i - 1] = self.projections[i - 1].sub(self.projections[i])?;
            }
        }
        Ok(())
    }
    /// Preserve prefix states exactly; any modified cell forces suffix replay.
    /// The directed common-anchor terminal difference, not the proposal,
    /// determines acceptance. Caller output stays unchanged on refusal.
    pub fn solve_into(
        &mut self,
        initial: &[f64],
        goal: &[f64],
        base: &AdaptiveStepConfig,
        policy: SpatialCompactPolicy,
        endpoint: &mut [f64],
        out: &mut SpatialAdjointWork,
    ) -> CoreResult<SpatialCompactWork> {
        self.has_initial = false;
        self.valid_cells = 0;
        self.cells = 0;
        self.last_receipt = None;
        self.pilot_times.clear();
        let n = self.primal.n;
        if initial.len() != n
            || goal.len() != n
            || endpoint.len() != n
            || initial.iter().chain(goal).any(|v| !v.is_finite())
            || !policy.budget.is_finite()
            || policy.budget <= 0.
            || !policy.min_scale.is_finite()
            || policy.min_scale <= 0.
            || policy.min_scale > 1.
            || policy.max_replays > 32
        {
            return Err(err("compact input/policy"));
        }
        base.validate()?;
        let mut w = SpatialCompactWork {
            max_signed_error_upper: f64::MAX,
            ..Default::default()
        };
        self.scales = [1.; 8];
        self.steps = [0; 8];
        self.pilot_indices = [0; 9];
        let lo = initial.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = initial.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        self.anchor = 0.5 * lo + 0.5 * hi;
        self.states[..n].copy_from_slice(initial);
        self.times[0] = 0.;
        let duration = self.primal.duration;
        if policy.partition == SpatialCompactPartition::Fixed4 {
            self.cells = 4;
            for i in 1..=4 {
                self.times[i] = duration * (i as f64 * 0.25);
            }
            if self.times[..=4].windows(2).any(|t| t[0] >= t[1]) {
                return Err(err("compact fixed clock"));
            }
            w.cells = 4;
            if !self.suffix(0, base, &mut w, false)? {
                return Ok(w);
            }
        } else {
            let desired = match policy.partition {
                SpatialCompactPartition::Accepted2 => 2,
                SpatialCompactPartition::Accepted8 => 8,
                _ => 4,
            };
            self.primal.centered.copy_from_slice(initial);
            if !self.native(0., duration, 1., base, &mut w, false)? {
                return Ok(w);
            }
            let m = self.capture.len();
            w.pilot_packets = m;
            self.pilot_times.extend_from_slice(self.capture.times());
            self.cells = desired.min(m);
            w.cells = self.cells;
            for i in 0..=self.cells {
                let j = i * m / self.cells;
                self.pilot_indices[i] = j;
                self.times[i] = self.capture.times()[j];
                self.states[i * n..(i + 1) * n].copy_from_slice(self.capture.state(j).unwrap());
                if i > 0 {
                    self.steps[i - 1] = j - self.pilot_indices[i - 1];
                }
            }
            self.valid_cells = self.cells;
        }
        let prep = self
            .dual
            .prepare(goal, &self.times[..=self.cells], policy.dual)?;
        w.adjoint_actions = prep.actions;
        w.dual_cache_hit = prep.cache_hit;
        w.stationary_used = prep.stationary;
        w.stationary_residual = prep.stationary_residual;
        self.project_suffix(0, &mut w)?;
        self.has_initial = true;
        for round in 0..=policy.max_replays {
            let r = self.dual.receipt(
                self.anchor,
                self.projections[0],
                self.projections[self.cells],
            )?;
            let signed = Iv::new(r.signed_goal_error[0], r.signed_goal_error[1])?;
            w.completed = true;
            w.signed_goal_error = Some(signed.as_pair());
            w.max_signed_error_upper = signed.mag();
            self.last_receipt = Some(r);
            if signed.mag() <= policy.budget {
                w.accepted = true;
                endpoint.copy_from_slice(&self.states[self.cells * n..(self.cells + 1) * n]);
                *out = r;
                break;
            }
            if round == policy.max_replays {
                break;
            }
            let mut best = None;
            let mut best_score = -1.;
            for i in 0..self.cells {
                if self.scales[i] <= policy.min_scale {
                    continue;
                }
                let score = if policy.refinement == SpatialCompactRefinement::MultiCost {
                    self.defects[i].mag()
                        / self.steps[i..self.cells].iter().sum::<usize>().max(1) as f64
                } else {
                    self.defects[i].mag()
                };
                if score > best_score {
                    best_score = score;
                    best = Some(i);
                }
            }
            let Some(best) = best else {
                w.floor_refusals += 1;
                break;
            };
            let target = policy.budget / (2. * self.cells as f64);
            let mut first = self.cells;
            for i in 0..self.cells {
                if self.scales[i] <= policy.min_scale {
                    continue;
                }
                let mag = self.defects[i].mag();
                let chosen = match policy.refinement {
                    SpatialCompactRefinement::Single => i == best,
                    SpatialCompactRefinement::Uniform => true,
                    SpatialCompactRefinement::MultiMagnitude => i == best || mag > target,
                    SpatialCompactRefinement::MultiCost => {
                        i == best || mag > target && mag >= 0.125 * self.defects[best].mag()
                    }
                };
                if !chosen {
                    continue;
                }
                let factor = match policy.refinement {
                    SpatialCompactRefinement::Single | SpatialCompactRefinement::Uniform => 0.25,
                    _ => (0.5 * target / mag.max(f64::MIN_POSITIVE)).clamp(0.015625, 0.25),
                };
                let next = (self.scales[i] * factor).max(policy.min_scale);
                if next < self.scales[i] {
                    self.scales[i] = next;
                    w.refined_cells += 1;
                    w.selected[i] += 1;
                    first = first.min(i);
                }
            }
            if first == self.cells {
                w.floor_refusals += 1;
                break;
            }
            w.replay_rounds += 1;
            w.prefix_cells_reused += first;
            w.completed = false;
            w.signed_goal_error = None;
            w.max_signed_error_upper = f64::MAX;
            self.last_receipt = None;
            if !self.suffix(first, base, &mut w, true)? {
                self.project_suffix(first + 1, &mut w)?;
                break;
            }
            self.project_suffix(first + 1, &mut w)?;
        }
        Ok(w)
    }
}
