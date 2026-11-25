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
    if !end.is_finite()
        || end <= 0.
        || !opts.dense_output
        || !matches!(opts.atol, Atol::Scalar(_))
        || !policy.tube_radius.is_finite()
        || policy.tube_radius <= 0.
        || !policy.global_tol.is_finite()
        || policy.global_tol <= 0.
        || grid.len() > 4096
        || grid.iter().any(|t| !t.is_finite() || *t < 0. || *t > end)
        || grid.windows(2).any(|v| v[0] >= v[1])
    {
        return Err(bad("input/clock/dense scalar tolerance contract"));
    }
    let nx = src.nx();
    let initial = src.initialize(0., x, z, branch, policy.init, route.init())?;
    let warp = match route {
        EvolutionRoute::WarpDirect => Some(WarpSeed::new(src, end, QuotientMode::DirectBlock)?),
        EvolutionRoute::WarpFolded => Some(WarpSeed::new(src, end, QuotientMode::Folded)?),
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
        let a = initialize_mass_problem(src, 0., x, z, branch, policy.init, route.init())
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
    let driver = Rodas5p::solve(&p, (0., end), &y, opts).map_err(bad)?;
    let mut out = EvolutionRun {
        route,
        initial,
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
    if !out.driver.is_success()
        || out.driver.t.last() != Some(&end)
        || !out.driver.splices.is_empty()
    {
        out.reason = Some(format!(
            "driver incomplete or unsupported splice: {:?}",
            out.driver.status
        ));
        out.work = e.work.lock().map_err(|_| bad("work lock"))?.clone();
        return Ok(out);
    }
    let mut incoming = Some(0.);
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
