//! Opt-in DAE evolution: actual native mass or Schur reduced integration,
//! branch-verified point projection and a posteriori physical tube proof.
//! The underlying numerical trajectory is not mutated by projection.
#![allow(missing_docs)]
use crate::{
    Atol, CallbackError, Jacobian, Options, Problem, Rodas5p, Solution,
    polynomial_events::initialize_mass_problem, solution::Extension,
};
use rodas5p_core::{
    CoreError, CoreResult,
    directed::{Interval, add_up, mul_up},
};
use rodas5p_integrators::{
    retro_dae_evolution::{self as de, DenseBasis, SlabCertificate, WarpSeed},
    retro_dae_init::{DaeSource, InitPolicy, InitReport, InitRoute, InitWork},
    retro_foundation::QuotientMode,
    retro_portfolio::{Portfolio, Route, Selection},
};
use serde::Serialize;
use std::sync::{Arc, Mutex};
type Iv = Interval;
fn bad(s: impl ToString) -> CoreError {
    CoreError::InvalidInput(format!("DAE client: {}", s.to_string()))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum EvolutionRoute {
    Mass,
    Reduced3,
    Reduced5,
    Prepared5,
    WarpDirect,
    WarpFolded,
    Scalar5,
}
impl EvolutionRoute {
    fn init(self) -> InitRoute {
        match self {
            Self::Prepared5 => InitRoute::PreparedBatch,
            Self::Scalar5 => InitRoute::ScalarRoot,
            _ => InitRoute::Reuse,
        }
    }
    fn nodes(self) -> usize {
        if self == Self::Reduced3 { 3 } else { 5 }
    }
}
/// Absolute physical clock and certified differential-state radius.
/// The caller owns the incoming certificate. None means unresolved, not zero.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct EvolutionEpoch {
    pub start: f64,
    pub end: f64,
    pub incoming_x_error: Option<f64>,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct EvolutionPolicy {
    pub init: InitPolicy,
    pub tube_radius: f64,
    pub global_tol: f64,
    pub green: bool,
}
impl Default for EvolutionPolicy {
    fn default() -> Self {
        Self {
            init: InitPolicy::default(),
            tube_radius: 1e-3,
            global_tol: 1e-3,
            green: true,
        }
    }
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct ProjectionWork {
    pub calls: usize,
    pub refused: usize,
    pub rhs_count: usize,
    pub linear_batches: usize,
    pub prepared_owners: usize,
    pub prepared_support_batches: usize,
    pub root_calls: usize,
    pub dictionary_uses: usize,
    pub horner_uses: usize,
    pub warp_predictions: usize,
}
impl ProjectionWork {
    fn add(&mut self, w: &InitWork, warp: bool) {
        self.calls += 1;
        self.rhs_count += w.rhs_count;
        self.linear_batches += w.linear_batches;
        self.prepared_owners += w.prepared_owners;
        self.prepared_support_batches += w.prepared_support_batches;
        self.root_calls += w.root_calls;
        self.dictionary_uses += w.dictionary_uses;
        self.horner_uses += w.conditioned_horner_uses;
        self.warp_predictions += usize::from(warp);
    }
}
#[derive(Clone)]
struct Engine {
    src: DaeSource,
    branch: Vec<[f64; 2]>,
    seed: Vec<f64>,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    warp: Option<WarpSeed>,
    work: Arc<Mutex<ProjectionWork>>,
}
impl Engine {
    fn project(&self, t: f64, x: &[f64], seed: &[f64]) -> CoreResult<InitReport> {
        let r = de::project(
            &self.src,
            t,
            x,
            seed,
            &self.branch,
            self.policy.init,
            self.route.init(),
            self.warp.as_ref(),
        );
        let mut w = self.work.lock().map_err(|_| bad("work lock"))?;
        if let Ok(v) = &r {
            w.add(&v.work, self.warp.is_some());
        } else {
            w.calls += 1;
            w.refused += 1;
        }
        r
    }
    fn fixed(&self, t: f64, x: &[f64]) -> Result<InitReport, CallbackError> {
        self.project(t, x, &self.seed)
            .map_err(|e| CallbackError::Fatal(e.to_string()))
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Query {
    pub t: f64,
    pub raw: Vec<f64>,
    pub state: Vec<f64>,
    pub raw_constraint_upper: Option<f64>,
    pub projected_constraint_upper: f64,
    pub projection: InitReport,
    pub physical_error_upper: Option<f64>,
}
#[derive(Clone, Debug, Serialize)]
pub struct NativePacket {
    pub basis: DenseBasis,
    pub coefficients: Vec<f64>,
    pub y0: Vec<f64>,
    pub y1: Vec<f64>,
    pub stages: Vec<Vec<f64>>,
}
impl NativePacket {
    fn power(&self) -> CoreResult<Vec<Vec<Iv>>> {
        de::dense_power(&self.y0, &self.y1, &self.stages, self.basis)
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct SlabRecord {
    pub times: [f64; 2],
    pub native: NativePacket,
    pub power: Vec<Vec<[f64; 2]>>,
    pub certificate: Option<SlabCertificate>,
    pub reason: Option<String>,
}
#[derive(Serialize)]
pub struct EvolutionRun {
    pub route: EvolutionRoute,
    pub initial: InitReport,
    /// Nonlegacy epoch authority, retained in the receipt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epoch: Option<EvolutionEpoch>,
    driver: Solution,
    pub accepted: Vec<Query>,
    pub queries: Vec<Query>,
    slabs: Vec<SlabRecord>,
    pub work: ProjectionWork,
    pub warp: Option<WarpSeed>,
    pub pointwise_qualified: bool,
    pub branch_qualified: bool,
    pub global_qualified: bool,
    pub max_physical_error: Option<f64>,
    pub reason: Option<String>,
    #[serde(skip)]
    engine: Engine,
}
fn projected(
    e: &Engine,
    s: &Solution,
    t: f64,
    raw: Vec<f64>,
    slabs: &[SlabRecord],
) -> CoreResult<Query> {
    let nx = e.src.nx();
    let mass = e.route == EvolutionRoute::Mass;
    if raw.len() != if mass { nx + e.src.nz() } else { nx } {
        return Err(bad("raw shape"));
    }
    let x = &raw[..nx];
    let seed = if mass { &raw[nx..] } else { &e.seed };
    let r = e.project(t, x, seed)?;
    let mut state = x.to_vec();
    state.extend_from_slice(&r.z);
    let raw_constraint_upper = if mass {
        Some(e.src.constraint_upper(t, &raw)?)
    } else {
        None
    };
    let mut physical_error_upper = None;
    if let Some(a) = slabs.get(slabs.partition_point(|a| a.times[1] < t))
        && let Some(c) = a.certificate.as_ref().filter(|c| c.global_qualified)
        && t >= a.times[0]
        && t <= a.times[1]
        && x.iter()
            .zip(&c.x_range)
            .all(|(&x, b)| b[0] <= x && x <= b[1])
    {
        let p = a.power[..nx]
            .iter()
            .map(|r| {
                r.iter()
                    .map(|b| Iv::new(b[0], b[1]))
                    .collect::<CoreResult<Vec<_>>>()
            })
            .collect::<CoreResult<Vec<_>>>()?;
        let ex = de::query_x_error(&p, a.times, t, x, c.whole_x_error.expect("qualified"))?;
        physical_error_upper =
            Some(ex.max(add_up(r.state_error_upper, mul_up(c.zx_norm_upper, ex)?)?));
    }
    // Do not permit extrapolation even on a direct accepted-state query.
    if t < s.t[0] || t > *s.t.last().expect("initial") {
        return Err(bad("query outside driver domain"));
    }
    Ok(Query {
        t,
        raw,
        state,
        raw_constraint_upper,
        projected_constraint_upper: r.residual_upper,
        projection: r,
        physical_error_upper,
    })
}
impl EvolutionRun {
    pub fn driver(&self) -> &Solution {
        &self.driver
    }
    pub fn slabs(&self) -> &[SlabRecord] {
        &self.slabs
    }

    /// Fresh original-source projection, retaining no cached algebraic answer.
    /// Physical bound is available only when the containing slab is proved.
    pub fn query(&self, t: f64) -> CoreResult<Query> {
        let y = self
            .driver
            .at(t)
            .ok_or_else(|| bad("query outside driver"))?;
        projected(&self.engine, &self.driver, t, y, &self.slabs)
    }
}
fn packet(s: &Solution, k: usize) -> CoreResult<NativePacket> {
    match s
        .extension
        .as_ref()
        .ok_or_else(|| bad("native dense store missing"))?
    {
        Extension::Sequential(v) => {
            let a = v.get(k).ok_or_else(|| bad("sequential packet index"))?;
            if [a.t_old, a.t_new] != [s.t[k], s.t[k + 1]]
                || a.y_old != s.y[k]
                || a.y_new != s.y[k + 1]
            {
                return Err(bad("sequential packet identity"));
            }
            Ok(NativePacket {
                basis: DenseBasis::SequentialD,
                coefficients: DenseBasis::SequentialD.coefficients()?.as_slice().to_vec(),
                y0: a.y_old.clone(),
                y1: a.y_new.clone(),
                stages: a.stages.clone(),
            })
        }
        Extension::Fast(v) => {
            if [v.t_old(k), v.t_new(k)] != [s.t[k], s.t[k + 1]]
                || v.state(k) != s.y[k]
                || v.state(k + 1) != s.y[k + 1]
            {
                return Err(bad("fast packet identity"));
            }
            let stage = v
                .stage_rows(k)
                .chunks(v.dimension())
                .map(<[f64]>::to_vec)
                .collect::<Vec<_>>();
            Ok(NativePacket {
                basis: DenseBasis::FastH,
                coefficients: DenseBasis::FastH.coefficients()?.as_slice().to_vec(),
                y0: v.state(k).to_vec(),
                y1: v.state(k + 1).to_vec(),
                stages: stage,
            })
        }
    }
}
/// Execute one full numerical candidate plus original source verification.
/// A returned report can be unqualified; no fallback silently changes route.
#[allow(clippy::too_many_arguments)]
pub fn evolve(
    src: &DaeSource,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    end: f64,
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    grid: &[f64],
) -> CoreResult<EvolutionRun> {
    evolve_epoch(
        src,
        x,
        z,
        branch,
        EvolutionEpoch {
            start: 0.,
            end,
            incoming_x_error: Some(0.),
        },
        opts,
        policy,
        route,
        grid,
    )
}
/// Integrate at an absolute physical clock with caller-certified incoming
/// differential error. No event is detected or committed by this API.
/// Unknown incoming radius allows numerical evolution but no global proof.
#[allow(clippy::too_many_arguments)]
pub fn evolve_epoch(
    src: &DaeSource,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    epoch: EvolutionEpoch,
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    grid: &[f64],
) -> CoreResult<EvolutionRun> {
    evolve_epoch_impl(src, x, z, branch, epoch, opts, policy, route, grid, None)
}
#[allow(clippy::too_many_arguments)]
fn evolve_epoch_impl(
    src: &DaeSource,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    epoch: EvolutionEpoch,
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    grid: &[f64],
    mut scan: Option<&mut ThresholdScan>,
) -> CoreResult<EvolutionRun> {
    let EvolutionEpoch {
        start,
        end,
        incoming_x_error,
    } = epoch;
    if !start.is_finite()
        || !end.is_finite()
        || end <= start
        || !(end - start).is_finite()
        || incoming_x_error.is_some_and(|v| !v.is_finite() || v < 0.)
        || !opts.dense_output
        || !matches!(opts.atol, Atol::Scalar(_))
        || !policy.tube_radius.is_finite()
        || policy.tube_radius <= 0.
        || !policy.global_tol.is_finite()
        || policy.global_tol <= 0.
        || grid.len() > 4096
        || grid
            .iter()
            .any(|t| !t.is_finite() || *t < start || *t > end)
        || grid.windows(2).any(|v| v[0] >= v[1])
    {
        return Err(bad("input/clock/dense scalar tolerance contract"));
    }
    let nx = src.nx();
    let initial = src.initialize(start, x, z, branch, policy.init, route.init())?;
    let warp = match route {
        EvolutionRoute::WarpDirect => Some(WarpSeed::new_window(
            src,
            start,
            end,
            QuotientMode::DirectBlock,
        )?),
        EvolutionRoute::WarpFolded => {
            Some(WarpSeed::new_window(src, start, end, QuotientMode::Folded)?)
        }
        _ => None,
    };
    let e = Engine {
        src: src.clone(),
        branch: branch.to_vec(),
        seed: initial.z.clone(),
        policy,
        route,
        warp: warp.clone(),
        work: Arc::new(Mutex::new(ProjectionWork::default())),
    };
    let (p, y) = if route == EvolutionRoute::Mass {
        let a = initialize_mass_problem(src, start, x, z, branch, policy.init, route.init())
            .map_err(bad)?;
        (a.problem, a.initial_state)
    } else {
        let a = e.clone();
        let b = e.clone();
        let c = e.clone();
        let p = Problem::new(nx, move |t, x, out| {
            out.copy_from_slice(&a.fixed(t, x)?.xdot);
            Ok(())
        })
        .jacobian(Jacobian::dense(move |t, x, out| {
            let r = b.fixed(t, x)?;
            for (i, row) in r.reduced_jacobian.iter().enumerate() {
                out[i * nx..(i + 1) * nx].copy_from_slice(row);
            }
            Ok(())
        }))
        .time_derivative(move |t, x, out| {
            let r = c.fixed(t, x)?;
            let mut y = x.to_vec();
            y.extend_from_slice(&r.z);
            let j = c
                .src
                .jacobian_values(t, &y)
                .map_err(|e| CallbackError::Fatal(e.to_string()))?;
            let ft = c
                .src
                .time_values(t, &y)
                .map_err(|e| CallbackError::Fatal(e.to_string()))?;
            let n = y.len();
            for i in 0..nx {
                out[i] =
                    r.zt.iter()
                        .enumerate()
                        .fold(ft[i], |s, (k, &v)| j[i * n + nx + k].mul_add(v, s));
            }
            Ok(())
        });
        (p, x.to_vec())
    };
    let driver = if let Some(scan) = scan.as_deref_mut() {
        let mut inspect = |proposal: crate::solve::DenseProposal<'_>| scan.inspect(&e, proposal);
        Rodas5p::solve_dense_controlled(&p, (start, end), &y, opts, &mut inspect).map_err(bad)?
    } else {
        Rodas5p::solve(&p, (start, end), &y, opts).map_err(bad)?
    };
    let expected_stop = scan
        .as_ref()
        .is_some_and(|v| v.stopped && driver.t.last() == Some(&v.numerical_end));
    let actual_end = if expected_stop {
        *driver.t.last().expect("accepted prefix")
    } else {
        end
    };
    let mut out = EvolutionRun {
        route,
        initial,
        epoch: (start != 0. || incoming_x_error != Some(0.)).then_some(epoch),
        driver,
        accepted: vec![],
        queries: vec![],
        slabs: vec![],
        work: ProjectionWork::default(),
        warp,
        pointwise_qualified: false,
        branch_qualified: false,
        global_qualified: false,
        max_physical_error: None,
        reason: None,
        engine: e.clone(),
    };
    if (!out.driver.is_success() && !expected_stop)
        || out.driver.t.last() != Some(&actual_end)
        || !out.driver.splices.is_empty()
    {
        out.reason = Some(format!(
            "driver incomplete or unsupported splice: {:?}",
            out.driver.status
        ));
        out.work = e.work.lock().map_err(|_| bad("work lock"))?.clone();
        return Ok(out);
    }
    let mut incoming = incoming_x_error;
    for k in 0..out.driver.t.len() - 1 {
        let times = [out.driver.t[k], out.driver.t[k + 1]];
        let native = packet(&out.driver, k)?;
        let mut p = native.power()?;
        if route != EvolutionRoute::Mass {
            let (a, w) = match de::reconstruct(
                src,
                times,
                &p,
                &e.seed,
                branch,
                policy.init,
                route.init(),
                e.warp.as_ref(),
                route.nodes(),
            ) {
                Ok(v) => v,
                Err(err) => {
                    out.slabs.push(SlabRecord {
                        times,
                        native,
                        power: vec![],
                        certificate: None,
                        reason: Some(err.to_string()),
                    });
                    incoming = None;
                    continue;
                }
            };
            for w in &w {
                e.work
                    .lock()
                    .map_err(|_| bad("work lock"))?
                    .add(w, e.warp.is_some());
            }
            p.extend(a);
        }
        let power = p
            .iter()
            .map(|a| a.iter().map(|v| v.as_pair()).collect())
            .collect();
        let r = de::certify_slab(
            src,
            times,
            &p,
            branch,
            policy.tube_radius,
            incoming,
            policy.green,
            policy.init.condition_cap,
        );
        let (certificate, reason) = match r {
            Ok(c) => {
                incoming = if c.global_qualified { c.outgoing } else { None };
                (Some(c), None)
            }
            Err(err) => {
                incoming = None;
                (None, Some(err.to_string()))
            }
        };
        out.slabs.push(SlabRecord {
            times,
            native,
            power,
            certificate,
            reason,
        });
    }
    out.branch_qualified = out.slabs.iter().all(|s| s.certificate.is_some());
    for (&t, y) in out.driver.t.iter().zip(&out.driver.y) {
        match projected(&e, &out.driver, t, y.clone(), &out.slabs) {
            Ok(v) => out.accepted.push(v),
            Err(err) => {
                out.reason = Some(err.to_string());
                break;
            }
        }
    }
    for &t in grid {
        match out.query(t) {
            Ok(v) => out.queries.push(v),
            Err(err) => {
                out.reason = Some(err.to_string());
                break;
            }
        }
    }
    out.pointwise_qualified =
        out.accepted.len() == out.driver.t.len() && out.queries.len() == grid.len();
    if out.branch_qualified
        && out.pointwise_qualified
        && incoming.is_some()
        && out
            .slabs
            .iter()
            .all(|s| s.certificate.as_ref().is_some_and(|c| c.global_qualified))
    {
        let mut mx = 0.0_f64;
        for s in &out.slabs {
            let c = s.certificate.as_ref().unwrap();
            mx = mx
                .max(c.whole_x_error.unwrap())
                .max(c.whole_z_error.unwrap());
        }
        let qs = out
            .accepted
            .iter()
            .chain(&out.queries)
            .map(|q| q.physical_error_upper)
            .collect::<Option<Vec<_>>>();
        if let Some(q) = qs {
            mx = q.into_iter().fold(mx, f64::max);
            out.max_physical_error = Some(mx);
            out.global_qualified = mx <= policy.global_tol;
        }
    }
    out.work = e.work.lock().map_err(|_| bad("work lock"))?.clone();
    Ok(out)
}
/// RETRO measured route choice over the same continuous branch/pointwise
/// contract. Global certificate status is retained separately, not implied.
#[allow(clippy::too_many_arguments)]
pub fn adaptive(
    src: &DaeSource,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    end: f64,
    opts: &Options,
    policy: EvolutionPolicy,
    grid: &[f64],
    portfolio: &mut Portfolio,
) -> CoreResult<Selection<EvolutionRun>> {
    let mut routes = vec![
        Route::EvolutionMass,
        Route::EvolutionReduced3,
        Route::EvolutionReduced5,
        Route::EvolutionPrepared5,
        Route::EvolutionWarpDirect,
        Route::EvolutionWarpFolded,
    ];
    if src.nz() == 1 {
        routes.push(Route::EvolutionScalar5);
    }
    portfolio.choose(
        "dae-evolution",
        &[
            src.nx() as u64,
            src.nz() as u64,
            opts.rtol.to_bits(),
            policy.init.state_tol.to_bits(),
            policy.tube_radius.to_bits(),
            grid.len() as u64,
        ],
        &routes,
        policy.init.residual_tol,
        |r| {
            let route = match r {
                Route::EvolutionMass => EvolutionRoute::Mass,
                Route::EvolutionReduced3 => EvolutionRoute::Reduced3,
                Route::EvolutionReduced5 => EvolutionRoute::Reduced5,
                Route::EvolutionPrepared5 => EvolutionRoute::Prepared5,
                Route::EvolutionWarpDirect => EvolutionRoute::WarpDirect,
                Route::EvolutionWarpFolded => EvolutionRoute::WarpFolded,
                Route::EvolutionScalar5 => EvolutionRoute::Scalar5,
                _ => return Err(bad("route")),
            };
            let v = evolve(src, x, z, branch, end, opts, policy, route, grid)?;
            let b = if v.branch_qualified && v.pointwise_qualified {
                v.accepted
                    .iter()
                    .chain(&v.queries)
                    .map(|q| q.projected_constraint_upper)
                    .fold(0., f64::max)
            } else {
                f64::INFINITY
            };
            Ok((v, b))
        },
    )
}

/// Root-search routes. All require the same source-owned physical tube;
/// a represented-polynomial hint has no physical authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ThresholdRoute {
    Bisection,
    IntervalNewton,
    DictionaryHint,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ThresholdWork {
    pub inspected: usize,
    pub guard_evals: usize,
    pub hint_calls: usize,
    pub hint_dictionary_uses: usize,
    pub projection: ProjectionWork,
    pub certified_slabs: usize,
}
pub use de::{GuardDerivativeRoute, GuardSlabEvidence, GuardValueRoute};
struct DeclaredGuard {
    source: rodas5p_integrators::retro_event_poly::DeclaredPolynomial,
    derivative: GuardDerivativeRoute,
    value: GuardValueRoute,
}
struct ThresholdScan {
    threshold: f64,
    guard: Option<DeclaredGuard>,
    guard_slabs: Vec<GuardSlabEvidence>,
    tolerance: f64,
    route: ThresholdRoute,
    incoming: Option<f64>,
    safe: f64,
    numerical_end: f64,
    root: Option<[f64; 2]>,
    stopped: bool,
    reason: Option<String>,
    work: ThresholdWork,
}
fn proposal_packet(
    proposal: crate::solve::DenseProposal<'_>,
) -> CoreResult<([f64; 2], NativePacket)> {
    use crate::solve::DenseProposal;
    let (times, basis, y0, y1, stages) = match proposal {
        DenseProposal::Sequential(s) => (
            [s.t_old, s.t_new],
            DenseBasis::SequentialD,
            s.y_old.clone(),
            s.y_new.clone(),
            s.stages.clone(),
        ),
        DenseProposal::Fast(s) => {
            let k = s.len() - 1;
            (
                [s.t_old(k), s.t_new(k)],
                DenseBasis::FastH,
                s.state(k).to_vec(),
                s.state(k + 1).to_vec(),
                s.stage_rows(k)
                    .chunks(s.dimension())
                    .map(<[f64]>::to_vec)
                    .collect(),
            )
        }
    };
    Ok((
        times,
        NativePacket {
            basis,
            coefficients: basis.coefficients()?.as_slice().to_vec(),
            y0,
            y1,
            stages,
        },
    ))
}
fn guard_interval(p: &[Iv], times: [f64; 2], t: f64, error: f64, threshold: f64) -> CoreResult<Iv> {
    let u = Iv::point(t)?
        .sub(Iv::point(times[0])?)?
        .div(Iv::point(times[1])?.sub(Iv::point(times[0])?)?)?;
    let u = Iv::new(u.lo.max(0.), u.hi.min(1.))?;
    let x = p
        .iter()
        .rev()
        .try_fold(Iv::point(0.)?, |a, &b| a.mul(u)?.add(b))?;
    x.sub(Iv::point(threshold)?)?.add(Iv::new(-error, error)?)
}
fn diameter(a: f64, b: f64) -> CoreResult<f64> {
    rodas5p_core::directed::sub_up(b, a)
}
impl ThresholdScan {
    fn inspect(
        &mut self,
        e: &Engine,
        proposal: crate::solve::DenseProposal<'_>,
    ) -> rodas5p_integrators::AcceptControl {
        use rodas5p_integrators::AcceptControl;
        self.work.inspected += 1;
        let answer = proposal_packet(proposal).and_then(|(times, native)| {
            self.numerical_end = times[1];
            self.scan(e, times, &native)
        });
        match answer {
            Ok(false) => AcceptControl::Continue,
            Ok(true) => {
                self.stopped = true;
                AcceptControl::Stop
            }
            Err(err) => {
                self.reason = Some(err.to_string());
                self.stopped = true;
                AcceptControl::Stop
            }
        }
    }
    fn scan(&mut self, e: &Engine, times: [f64; 2], native: &NativePacket) -> CoreResult<bool> {
        let mut p = native.power()?;
        if e.route != EvolutionRoute::Mass {
            let (z, w) = de::reconstruct(
                &e.src,
                times,
                &p,
                &e.seed,
                &e.branch,
                e.policy.init,
                e.route.init(),
                e.warp.as_ref(),
                e.route.nodes(),
            )?;
            for q in &w {
                self.work.projection.add(q, e.warp.is_some());
            }
            p.extend(z);
        }
        let c = de::certify_slab(
            &e.src,
            times,
            &p,
            &e.branch,
            e.policy.tube_radius,
            self.incoming,
            e.policy.green,
            e.policy.init.condition_cap,
        )?;
        if !c.global_qualified
            || c.whole_z_error.is_none_or(|v| v > e.policy.global_tol)
            || c.whole_x_error.is_none_or(|v| v > e.policy.global_tol)
        {
            return Err(bad("unqualified physical slab"));
        }
        self.work.certified_slabs += 1;
        let proof = self
            .guard
            .as_ref()
            .map(|g| de::GuardProof::new(&e.src, &g.source, &p, &c, g.derivative, g.value))
            .transpose()?;
        let velocity = if let Some(proof) = &proof {
            let v = proof.evidence.velocity;
            self.guard_slabs.push(proof.evidence.clone());
            Iv::new(v[0], v[1])?
        } else {
            de::threshold_velocity(&e.src, &c)?
        };
        if velocity.lo <= 0. {
            return Err(bad("strict transverse increasing guard not proved"));
        }
        let incoming = self
            .incoming
            .ok_or_else(|| bad("unknown incoming radius"))?;
        let outgoing = c.outgoing.ok_or_else(|| bad("unknown outgoing radius"))?;
        let whole = c.whole_x_error.ok_or_else(|| bad("unknown whole radius"))?;
        let a = times[0];
        let b = times[1];
        let mut g = |t: f64, error: f64| {
            self.work.guard_evals += 1;
            if let Some(proof) = &proof {
                let v = proof.value_at(t, error)?;
                self.guard_slabs
                    .last_mut()
                    .expect("current guard evidence")
                    .evaluations
                    .push(de::GuardPointEvidence {
                        time: t,
                        differential_radius: error,
                        value: v.as_pair(),
                    });
                Ok(v)
            } else {
                guard_interval(&p[0], times, t, error, self.threshold)
            }
        };
        let ga = g(a, incoming)?;
        let gb = g(b, outgoing)?;
        if ga.hi >= 0. {
            return Err(bad("uncertain left guard"));
        }
        if gb.hi < 0. {
            self.safe = b;
            self.incoming = Some(outgoing);
            return Ok(false);
        }
        if gb.lo <= 0. {
            return Err(bad("uncertain right guard; refine before committing"));
        }
        // Unique physical root by strict signs and uniform positive derivative.
        let mut lo = a;
        let mut hi = b;
        let mut hint = None;
        if self.route == ThresholdRoute::DictionaryHint {
            use rodas5p_integrators::retro_event_poly::{RepresentedPolynomial, RootRoute};
            self.work.hint_calls += 1;
            let cp = if let Some(proof) = &proof {
                proof.hint_coefficients()
            } else {
                let mut cp = p[0]
                    .iter()
                    .map(|v| v.lo * 0.5 + v.hi * 0.5)
                    .collect::<Vec<_>>();
                cp[0] -= self.threshold;
                cp
            };
            if let Ok(r) = RepresentedPolynomial::from_power(&cp)
                .and_then(|r| r.isolate(RootRoute::DictionaryCompanion, 2f64.powi(-35)))
            {
                self.work.hint_dictionary_uses += r.work.dictionary_uses;
                if r.roots.len() == 1 {
                    let u = r.roots[0].theta;
                    let t = a + (b - a) * (u[0] * 0.5 + u[1] * 0.5);
                    if t > a && t < b {
                        hint = Some(t);
                    }
                }
            }
        }
        for _ in 0..64 {
            if diameter(lo, hi)? <= self.tolerance {
                self.root = Some([lo, hi]);
                self.safe = lo;
                self.incoming = Some(outgoing);
                return Ok(true);
            }
            let mid = hint.take().unwrap_or(lo * 0.5 + hi * 0.5);
            if !(lo < mid && mid < hi) {
                break;
            }
            let v = g(mid, whole)?;
            let old = (lo, hi);
            if self.route != ThresholdRoute::Bisection {
                let n = Iv::point(mid)?.sub(v.div(velocity)?)?;
                lo = lo.max(n.lo);
                hi = hi.min(n.hi);
                if lo > hi {
                    return Err(bad("inconsistent physical root enclosure"));
                }
            } else if v.hi < 0. {
                lo = mid;
            } else if v.lo > 0. {
                hi = mid;
            } else {
                let left = lo * 0.5 + mid * 0.5;
                let right = mid * 0.5 + hi * 0.5;
                if g(left, whole)?.hi < 0. {
                    lo = left;
                }
                if g(right, whole)?.lo > 0. {
                    hi = right;
                }
            }
            if old == (lo, hi) {
                break;
            }
        }
        if diameter(lo, hi)? <= self.tolerance {
            self.root = Some([lo, hi]);
            self.safe = lo;
            return Ok(true);
        }
        Err(bad("physical root width exceeds tolerance"))
    }
}
/// The numerical continuation is private: only the certified old-mode
/// prefix can be queried. All trial solver work stays in stats.
pub struct ThresholdRun {
    trial: EvolutionRun,
    root: Option<[f64; 2]>,
    committed_until: f64,
    numerical_end: f64,
    reason: Option<String>,
    pub work: ThresholdWork,
    guard_slabs: Vec<GuardSlabEvidence>,
}
impl ThresholdRun {
    pub fn guard_slabs(&self) -> &[GuardSlabEvidence] {
        &self.guard_slabs
    }
    pub fn root(&self) -> Option<[f64; 2]> {
        self.root
    }
    pub fn committed_until(&self) -> f64 {
        self.committed_until
    }
    pub fn numerical_end(&self) -> f64 {
        self.numerical_end
    }
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
    pub fn stats(&self) -> &crate::Stats {
        &self.trial.driver.stats
    }
    pub fn pre_query(&self, t: f64) -> CoreResult<Query> {
        if !t.is_finite() || t > self.committed_until {
            return Err(bad("outside certified pre-event prefix"));
        }
        let q = self.trial.query(t)?;
        if q.physical_error_upper
            .is_none_or(|b| b > self.trial.engine.policy.global_tol)
        {
            return Err(bad("unqualified physical query"));
        }
        Ok(q)
    }
}
/// Single strictly increasing scalar differential threshold. No claim for
/// grazing, opaque or competing guards. The caller supplies initial authority.
#[allow(clippy::too_many_arguments)]
pub fn evolve_to_threshold(
    src: &DaeSource,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    epoch: EvolutionEpoch,
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    threshold: f64,
    tolerance: f64,
    root_route: ThresholdRoute,
) -> CoreResult<ThresholdRun> {
    if src.nx() != 1
        || x.len() != 1
        || !threshold.is_finite()
        || !tolerance.is_finite()
        || tolerance <= 0.
    {
        return Err(bad("scalar threshold contract"));
    }
    let rad = epoch
        .incoming_x_error
        .ok_or_else(|| bad("physical incoming radius required"))?;
    if !rad.is_finite() || rad < 0. || Iv::point(x[0])?.add(Iv::new(-rad, rad)?)?.hi >= threshold {
        return Err(bad("initial guard not strictly below threshold"));
    }
    let mut scan = ThresholdScan {
        threshold,
        guard: None,
        guard_slabs: vec![],
        tolerance,
        route: root_route,
        incoming: Some(rad),
        safe: epoch.start,
        numerical_end: epoch.start,
        root: None,
        stopped: false,
        reason: None,
        work: ThresholdWork::default(),
    };
    let trial = evolve_epoch_impl(
        src,
        x,
        z,
        branch,
        epoch,
        opts,
        policy,
        route,
        &[],
        Some(&mut scan),
    )?;
    if !scan.stopped && (!trial.driver.is_success() || trial.driver.t.last() != Some(&epoch.end)) {
        scan.reason = Some("native driver ended before certified event or horizon".into());
    }
    if scan.root.is_some() && !trial.branch_qualified {
        scan.root = None;
        scan.safe = epoch.start;
        scan.reason = Some("replayed physical branch proof failed".into());
    }
    Ok(ThresholdRun {
        trial,
        root: scan.root,
        committed_until: scan.safe,
        numerical_end: scan.numerical_end,
        reason: scan.reason,
        work: scan.work,
        guard_slabs: scan.guard_slabs,
    })
}

/// Source-owned single polynomial guard H(t,x,z)=0 on the certified branch.
/// Initial physical sign must be negative and all inspected slabs strictly
/// increasing in H. No root can be authorized by a polynomial hint alone.
#[allow(clippy::too_many_arguments)]
pub fn evolve_to_guard(
    src: &DaeSource,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    epoch: EvolutionEpoch,
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    guard: &rodas5p_integrators::retro_event_poly::DeclaredPolynomial,
    tolerance: f64,
    root_route: ThresholdRoute,
    derivative: GuardDerivativeRoute,
    value: GuardValueRoute,
) -> CoreResult<ThresholdRun> {
    guard.validate(src.nx() + src.nz())?;
    if x.len() != src.nx() || !tolerance.is_finite() || tolerance <= 0. {
        return Err(bad("declared guard contract"));
    }
    let rad = epoch
        .incoming_x_error
        .ok_or_else(|| bad("physical incoming radius required"))?;
    if !rad.is_finite() || rad < 0. {
        return Err(bad("invalid incoming radius"));
    }
    let mut scan = ThresholdScan {
        threshold: 0.,
        guard: Some(DeclaredGuard {
            source: guard.clone(),
            derivative,
            value,
        }),
        guard_slabs: vec![],
        tolerance,
        route: root_route,
        incoming: Some(rad),
        safe: epoch.start,
        numerical_end: epoch.start,
        root: None,
        stopped: false,
        reason: None,
        work: ThresholdWork::default(),
    };
    let trial = evolve_epoch_impl(
        src,
        x,
        z,
        branch,
        epoch,
        opts,
        policy,
        route,
        &[],
        Some(&mut scan),
    )?;
    if !scan.stopped && (!trial.driver.is_success() || trial.driver.t.last() != Some(&epoch.end)) {
        scan.reason = Some("native driver ended before certified event or horizon".into());
    }
    if scan.root.is_some() && !trial.branch_qualified {
        scan.root = None;
        scan.safe = epoch.start;
        scan.reason = Some("replayed physical branch proof failed".into());
    }
    Ok(ThresholdRun {
        trial,
        root: scan.root,
        committed_until: scan.safe,
        numerical_end: scan.numerical_end,
        reason: scan.reason,
        work: scan.work,
        guard_slabs: scan.guard_slabs,
    })
}

/// The differential reset restricted to the single threshold is
/// x^+ = offset + time_scale * tau. The new differential source is
/// constructed here, not accepted as an unrelated user callback.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct AffineDaeReset {
    pub rate: f64,
    pub equilibrium: f64,
    pub offset: f64,
    pub time_scale: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum AffineSyncRoute {
    Rectangle,
    GreenMoment,
    CriticalPoint,
}
#[derive(Serialize)]
pub struct AffineRestartRun {
    pub sync_route: AffineSyncRoute,
    pub event_time: [f64; 2],
    pub synchronized_x: [f64; 2],
    pub incoming_radius: f64,
    pub post: EvolutionRun,
}
impl AffineDaeReset {
    fn sync(self, times: [f64; 2], route: AffineSyncRoute) -> CoreResult<Iv> {
        use rodas5p_core::retro_dictionary::RealMode;
        use rodas5p_integrators::retro_green::RetroTable;
        let table = RetroTable::with_dictionary(RealMode::Hybrid)?;
        let s = Iv::point(times[1])?;
        let t = Iv::new(times[0], times[1])?;
        let a = Iv::point(self.rate)?;
        let e = Iv::point(self.equilibrium)?;
        let aa = Iv::point(self.offset)?;
        let b = Iv::point(self.time_scale)?;
        let at = |t: Iv| -> CoreResult<Iv> {
            let dt = s.sub(t)?.mul(a)?;
            let gain = table.phi(Iv::new(dt.lo.max(0.), dt.hi)?, 0)?;
            e.add(gain.mul(aa.add(b.mul(t)?)?.sub(e)?)?)
        };
        match route {
            AffineSyncRoute::Rectangle => at(t),
            AffineSyncRoute::GreenMoment => {
                let dt = s.sub(t)?;
                let u = Iv::new(0., dt.hi)?;
                let au = a.mul(u)?;
                let au = Iv::new(au.lo.max(0.), au.hi)?;
                let ph1 = table.phi(au, 1)?;
                let ph2 = table.phi(au, 2)?;
                let d = ph1.sub(ph2)?;
                let d = Iv::new(d.lo.max(0.), d.hi)?;
                let rs = aa.add(b.mul(s)?)?;
                rs.sub(b.add(a.mul(rs.sub(e)?)?)?.mul(u)?.mul(ph1)?)?
                    .add(a.mul(b)?.mul(u)?.mul(u)?.mul(d)?)
            }
            AffineSyncRoute::CriticalPoint => {
                let l = at(Iv::point(times[0])?)?;
                let r = at(s)?;
                let mut out = Iv::new(l.lo.min(r.lo), l.hi.max(r.hi))?;
                if self.time_scale != 0. {
                    let turn = -(b.add(a.mul(aa.sub(e)?)?)?.div(a.mul(b)?)?);
                    let lo = turn.lo.max(t.lo);
                    let hi = turn.hi.min(t.hi);
                    if lo <= hi {
                        let v = at(Iv::new(lo, hi)?)?;
                        out = Iv::new(out.lo.min(v.lo), out.hi.max(v.hi))?;
                    }
                }
                Ok(out)
            }
        }
    }
}
impl ThresholdRun {
    /// A single mode transition. No second guard is declared in this API.
    /// Do not restart without a physically certified root. Algebraic
    /// projection authority is explicit; no reset of error radius to zero.
    #[allow(clippy::too_many_arguments)]
    pub fn restart_affine(
        &self,
        reset: AffineDaeReset,
        constraints: Vec<rodas5p_integrators::retro_event_poly::DeclaredPolynomial>,
        z_seed: &[f64],
        branch: &[[f64; 2]],
        end: f64,
        opts: &Options,
        policy: EvolutionPolicy,
        route: EvolutionRoute,
        sync_route: AffineSyncRoute,
        grid: &[f64],
    ) -> CoreResult<AffineRestartRun> {
        use rodas5p_integrators::retro_dae_init::InitMode;
        use rodas5p_integrators::retro_event_poly::{AffineFactor, DeclaredPolynomial, EventTerm};
        let root = self
            .root
            .filter(|_| self.reason.is_none())
            .ok_or_else(|| bad("no physical event authority"))?;
        if policy.init.mode != InitMode::ProjectAlgebraic
            || !reset.rate.is_finite()
            || reset.rate <= 0.
            || ![reset.equilibrium, reset.offset, reset.time_scale, end]
                .iter()
                .all(|v| v.is_finite())
            || end <= root[1]
        {
            return Err(bad(
                "affine reset requires explicit algebraic projection and positive rate",
            ));
        }
        let forcing = Iv::point(reset.rate)?.mul(Iv::point(reset.equilibrium)?)?;
        if forcing.lo != forcing.hi {
            return Err(bad("affine source coefficient not exactly represented"));
        }
        let src = DaeSource::new(
            vec![DeclaredPolynomial::new(vec![
                EventTerm {
                    coefficient: -reset.rate,
                    factors: vec![AffineFactor::component(0, 0.)],
                },
                EventTerm {
                    coefficient: forcing.lo,
                    factors: vec![],
                },
            ])?],
            constraints,
        )?;
        let sync = reset.sync(root, sync_route)?;
        let center = sync.lo * 0.5 + sync.hi * 0.5;
        let radius = diameter(sync.lo, center)?.max(diameter(center, sync.hi)?);
        let post = evolve_epoch(
            &src,
            &[center],
            z_seed,
            branch,
            EvolutionEpoch {
                start: root[1],
                end,
                incoming_x_error: Some(radius),
            },
            opts,
            policy,
            route,
            grid,
        )?;
        Ok(AffineRestartRun {
            sync_route,
            event_time: root,
            synchronized_x: sync.as_pair(),
            incoming_radius: radius,
            post,
        })
    }
}
