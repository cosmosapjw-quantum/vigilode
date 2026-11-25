//! Opt-in source-owned scalar hybrid IVP. Native proposal work is retained
//! and the driver is restarted at each committed synchronization time.
//! Physical and represented reset contracts are deliberately separate.
use crate::{
    AcceptControl, AdaptiveStepConfig, DriverStopReason, FastDenseSegments, FastInteriorControl,
    OdeProblem,
    affine_event_certificate::{
        self as ac, AffineEventBinding, AffineEventCertificate, AffineEventSpec, AffineScalarModel,
    },
    integrate_rodas5p_fast_dense_with_control,
    retro_causal::{self, SlabSpec},
    retro_event_poly::{
        AffineFactor, DeclaredPolynomial, EventTerm, RepresentedPolynomial, RootKind, RootRoute,
    },
    retro_green::RetroTable,
    retro_physical_event::contract_root_once,
    retro_portfolio::{Attempt, Portfolio, Route},
};
use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, WorkCounters,
    directed::{Interval, mul_up, sub_down, sub_up},
    retro_dictionary::RealMode,
};
use serde::Serialize;
use std::sync::Arc;
type Iv = Interval;
fn bad(s: &str) -> CoreError {
    CoreError::InvalidInput(format!("hybrid reset: {s}"))
}
fn pt(x: f64) -> CoreResult<Iv> {
    Iv::point(x)
}
fn ball(x: f64) -> CoreResult<Iv> {
    Iv::new(-x, x)
}
fn sign(x: Iv) -> i8 {
    if x.lo > 0. {
        1
    } else if x.hi < 0. {
        -1
    } else {
        0
    }
}
fn hull(a: Iv, b: Iv) -> CoreResult<Iv> {
    Iv::new(a.lo.min(b.lo), a.hi.max(b.hi))
}
fn intersect(a: Iv, b: Iv) -> CoreResult<Iv> {
    Iv::new(a.lo.max(b.lo), a.hi.min(b.hi))
}
fn radius(v: Iv, c: f64) -> CoreResult<f64> {
    Ok(v.sub(pt(c)?)?.mag())
}
fn center(v: Iv) -> f64 {
    (v.lo * 0.5 + v.hi * 0.5).clamp(v.lo, v.hi)
}

/// R(t,y) = state_scale*y + offset + time_scale*t.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct ResetMap {
    pub state_scale: f64,
    pub offset: f64,
    pub time_scale: f64,
}
impl ResetMap {
    fn finite(self) -> bool {
        [self.state_scale, self.offset, self.time_scale]
            .iter()
            .all(|v| v.is_finite())
    }
    fn enclose(self, t: Iv, y: Iv) -> CoreResult<Iv> {
        y.scale(self.state_scale)?
            .add(pt(self.offset)?)?
            .add(t.scale(self.time_scale)?)
    }
    fn evaluate(self, t: f64, y: f64) -> CoreResult<f64> {
        let v = self
            .time_scale
            .mul_add(t, self.state_scale.mul_add(y, self.offset));
        pt(v)?;
        Ok(v)
    }
}
/// Exactly one threshold and deterministic transition per guarded mode.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Guard {
    pub threshold: f64,
    pub reset: ResetMap,
    pub next: usize,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Mode {
    pub rate: f64,
    pub forcing: f64,
    pub guard: Option<Guard>,
}
#[derive(Clone, Debug, Serialize)]
pub struct HybridSystem {
    modes: Vec<Mode>,
}
impl HybridSystem {
    pub fn new(modes: Vec<Mode>) -> CoreResult<Self> {
        ac::check_affine_event_arithmetic()?;
        if !(1..=32).contains(&modes.len())
            || modes.iter().any(|m| {
                !m.rate.is_finite()
                    || m.rate <= 0.
                    || !m.forcing.is_finite()
                    || m.guard.is_some_and(|g| {
                        !g.threshold.is_finite() || !g.reset.finite() || g.next >= modes.len()
                    })
            })
        {
            return Err(bad("mode graph/source admission"));
        }
        Ok(Self { modes })
    }
    pub fn modes(&self) -> &[Mode] {
        &self.modes
    }
}
/// Physical routes share the same contract; represented is a control only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ResetRoute {
    Represented,
    Carry,
    LegacyFlow,
    DictionaryFlow,
    Hinted,
    Transport,
    Combined,
}
impl ResetRoute {
    fn physical(self) -> bool {
        self != Self::Represented
    }
    fn carry(self) -> bool {
        matches!(self, Self::Carry | Self::Combined)
    }
    fn direct(self) -> bool {
        self.physical() && self != Self::Carry
    }
    fn full(self) -> bool {
        matches!(
            self,
            Self::DictionaryFlow | Self::Hinted | Self::Transport | Self::Combined
        )
    }
    fn hint(self) -> bool {
        matches!(self, Self::Hinted | Self::Combined)
    }
    fn transport(self) -> bool {
        matches!(self, Self::Transport | Self::Combined)
    }
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct ResetPolicy {
    pub time_tol: f64,
    pub state_tol: f64,
    pub min_advance: f64,
    pub max_resets: usize,
}
impl Default for ResetPolicy {
    fn default() -> Self {
        Self {
            time_tol: 1e-6,
            state_tol: 1e-4,
            min_advance: 1e-10,
            max_resets: 32,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum HybridStatus {
    Complete,
    Unresolved,
    ResetLimit,
    DriverFailure,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct ResetWork {
    pub accepted_inspections: u64,
    pub affine_carries: u64,
    pub flow_queries: u64,
    pub gain_queries: u64,
    pub represented_isolations: u64,
    pub root_hints: u64,
    pub root_queries: u64,
    pub contractions: u64,
    pub sync_rectangles: u64,
    pub sync_transport: u64,
}
impl ResetWork {
    fn absorb(&mut self, v: &Self) {
        self.accepted_inspections += v.accepted_inspections;
        self.affine_carries += v.affine_carries;
        self.flow_queries += v.flow_queries;
        self.gain_queries += v.gain_queries;
        self.represented_isolations += v.represented_isolations;
        self.root_hints += v.root_hints;
        self.root_queries += v.root_queries;
        self.contractions += v.contractions;
        self.sync_rectangles += v.sync_rectangles;
        self.sync_transport += v.sync_transport;
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct EndpointProof {
    pub times: [f64; 2],
    pub enclosure: [f64; 2],
    pub sign: i8,
    pub incoming_radius: Option<f64>,
    pub outgoing_radius: Option<f64>,
}
#[derive(Clone, Debug, Serialize)]
pub struct EpochReceipt {
    pub epoch: usize,
    pub mode: usize,
    pub start: f64,
    pub start_center: f64,
    pub start_enclosure: Option<[f64; 2]>,
    pub numerical_end: f64,
    pub committed_end: f64,
    pub committed: bool,
    pub stop_reason: String,
    pub inspected_steps: u64,
    pub counters: WorkCounters,
    pub endpoint_proofs: Vec<EndpointProof>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ResetReceipt {
    pub epoch: usize,
    pub from: usize,
    pub to: usize,
    pub bracket: [f64; 2],
    pub width: f64,
    pub kind: RootKind,
    pub sync_time: f64,
    pub pre_center: f64,
    pub guard_value: f64,
    pub reset_center: f64,
    pub post_enclosure: Option<[f64; 2]>,
    pub post_radius: Option<f64>,
    pub gap_tube: Option<[f64; 2]>,
    pub rectangle_width: Option<f64>,
    pub transported_width: Option<f64>,
}
#[derive(Clone, Debug, Serialize)]
pub struct HybridRun {
    pub route: ResetRoute,
    pub status: HybridStatus,
    pub physical_qualified: bool,
    pub time: f64,
    pub mode: usize,
    pub center: f64,
    pub enclosure: Option<[f64; 2]>,
    pub radius: Option<f64>,
    pub events: Vec<ResetReceipt>,
    pub epochs: Vec<EpochReceipt>,
    pub work: ResetWork,
    pub counters: WorkCounters,
    pub reason: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct HybridSelection {
    pub value: Option<HybridRun>,
    pub attempts: Vec<Attempt>,
    pub charged_native: WorkCounters,
    pub charged_work: ResetWork,
    pub attempted_runs: Vec<HybridRun>,
}
fn slab(m: Mode, end: f64) -> SlabSpec {
    SlabSpec {
        end,
        rate: m.rate,
        forcing: m.forcing,
        forecast: m.rate,
        radius: 0.,
    }
}
fn direct_flow(
    table: &RetroTable,
    m: Mode,
    start: f64,
    t: f64,
    y: f64,
    w: &mut ResetWork,
) -> CoreResult<Iv> {
    w.flow_queries += 1;
    retro_causal::flow(table, slab(m, t), start, t, y)
}
fn flow_box(
    table: &RetroTable,
    m: Mode,
    start: f64,
    t: f64,
    c: f64,
    y: Iv,
    w: &mut ResetWork,
) -> CoreResult<Iv> {
    if t == start {
        return Ok(y);
    }
    let v = direct_flow(table, m, start, t, c, w)?;
    let dt = pt(t)?.sub(pt(start)?)?;
    w.gain_queries += 1;
    let gain = retro_causal::gain(table, m.rate, dt)?;
    // The center+radius arm uses actual causal gain. Monotonicity in the
    // initial state also gives a sharper interval at its two endpoints.
    let radial = v.add(ball(mul_up(gain, radius(y, c)?)?)?)?;
    if y.lo == y.hi {
        return intersect(radial, direct_flow(table, m, start, t, y.lo, w)?);
    }
    let a = direct_flow(table, m, start, t, y.lo, w)?;
    let b = direct_flow(table, m, start, t, y.hi, w)?;
    intersect(radial, Iv::new(a.lo, b.hi)?)
}
fn spec(m: Mode, epoch: usize) -> CoreResult<AffineEventSpec> {
    AffineEventSpec::new(
        AffineEventBinding::new(
            "hybrid-owned-scalar",
            "immutable-mode",
            ac::AFFINE_EVENT_CLOCK_ID,
            "caller-state-time",
            "threshold-reset",
            epoch as u64,
        )?,
        AffineScalarModel::new(pt(m.rate)?, pt(m.forcing)?)?,
        m.guard.map_or(0., |g| g.threshold),
    )
}
fn ode(m: Mode) -> CoreResult<OdeProblem> {
    OdeProblem::new(
        "hybrid-source-owned",
        1,
        Arc::new(move |_, y, f| {
            f[0] = (-m.rate).mul_add(y[0], m.forcing);
            Ok(())
        }),
        None,
        Some(Arc::new(move |_, _| DenseMatrix::new(1, 1, vec![-m.rate]))),
        None,
        None,
        true,
        None,
        None,
    )
}
#[derive(Clone, Copy)]
struct Crossing {
    bracket: [f64; 2],
    kind: RootKind,
}
struct Scan<'a> {
    mode: Mode,
    start: f64,
    center: f64,
    initial: Iv,
    route: ResetRoute,
    table: &'a RetroTable,
    policy: ResetPolicy,
    spec: AffineEventSpec,
    poly: Option<DeclaredPolynomial>,
    initial_sign: i8,
    safe_time: f64,
    safe_box: Iv,
    certs: Vec<AffineEventCertificate>,
    proofs: Vec<EndpointProof>,
    last_box: Iv,
    last_sign: i8,
    crossing: Option<Crossing>,
    failed: Option<String>,
    work: ResetWork,
}
impl<'a> Scan<'a> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        m: Mode,
        start: f64,
        c: f64,
        initial: Iv,
        route: ResetRoute,
        table: &'a RetroTable,
        policy: ResetPolicy,
        epoch: usize,
    ) -> CoreResult<Self> {
        let initial_sign = if let Some(g) = m.guard {
            sign(initial.sub(pt(g.threshold)?)?)
        } else {
            1
        };
        if initial_sign == 0 {
            return Err(bad("initial guard intersects zero; reset not armed"));
        }
        let poly = m
            .guard
            .map(|g| {
                DeclaredPolynomial::new(vec![EventTerm {
                    coefficient: 1.,
                    factors: vec![AffineFactor::component(0, g.threshold)],
                }])
            })
            .transpose()?;
        Ok(Self {
            mode: m,
            start,
            center: c,
            initial,
            route,
            table,
            policy,
            spec: spec(m, epoch)?,
            poly,
            initial_sign,
            safe_time: start,
            safe_box: initial,
            certs: vec![],
            proofs: vec![],
            last_box: initial,
            last_sign: initial_sign,
            crossing: None,
            failed: None,
            work: ResetWork::default(),
        })
    }
    fn state_box(&mut self, s: &FastDenseSegments, t: f64) -> CoreResult<Iv> {
        if t == self.start {
            return Ok(self.initial);
        }
        self.work.root_queries += 1;
        let mut v = None;
        if self.route.carry() {
            let k = s.times()[1..].partition_point(|&end| end < t);
            let c = self
                .certs
                .get(k)
                .ok_or_else(|| bad("missing current affine certificate"))?;
            let q = s.query_affine_scalar_certificate(c, &self.spec, t)?;
            v = Some(q.state_enclosure);
        }
        if self.route.direct() {
            let d = flow_box(
                self.table,
                self.mode,
                self.start,
                t,
                self.center,
                self.initial,
                &mut self.work,
            )?;
            v = Some(if let Some(a) = v { intersect(a, d)? } else { d });
        }
        v.ok_or_else(|| bad("no physical state authority"))
    }
    fn inspect(&mut self, s: &FastDenseSegments) -> AcceptControl {
        self.work.accepted_inspections += 1;
        match self.inspect_impl(s) {
            Ok(stop) => {
                if stop {
                    AcceptControl::Stop
                } else {
                    AcceptControl::Continue
                }
            }
            Err(e) => {
                self.failed = Some(e.to_string());
                AcceptControl::Stop
            }
        }
    }
    fn inspect_impl(&mut self, s: &FastDenseSegments) -> CoreResult<bool> {
        let k = s.len() - 1;
        let t = s.t_new(k);
        if !self.route.physical() {
            if let Some(p) = &self.poly {
                self.work.represented_isolations += 1;
                let report = RepresentedPolynomial::from_segment(s, k, p)?
                    .isolate(RootRoute::DictionaryCompanion, 2f64.powi(-40))?;
                if let Some(r) = report.roots.first() {
                    self.crossing = Some(Crossing {
                        bracket: r.time,
                        kind: r.kind,
                    });
                    return Ok(true);
                }
            }
            return Ok(false);
        }
        let mut incoming_radius = None;
        let mut outgoing_radius = None;
        if self.route.carry() {
            let c = if let Some(prev) = self.certs.last() {
                s.certify_affine_scalar_segment_with_carry(k, &self.spec, &prev.carry()?)?
            } else {
                s.certify_affine_scalar_segment(k, &self.spec, radius(self.initial, self.center)?)?
            };
            incoming_radius = Some(c.initial_error());
            outgoing_radius = Some(c.endpoint_error_upper());
            self.work.affine_carries += 1;
            self.certs.push(c);
        }
        let y = self.state_box(s, t)?;
        self.last_box = y;
        let mut gs = 1;
        if let Some(g) = self.mode.guard {
            gs = sign(y.sub(pt(g.threshold)?)?);
        }
        self.last_sign = gs;
        self.proofs.push(EndpointProof {
            times: [s.t_old(k), t],
            enclosure: y.as_pair(),
            sign: gs,
            incoming_radius,
            outgoing_radius,
        });
        let Some(g) = self.mode.guard else {
            return Ok(false);
        };
        // Every scalar affine solution is monotone or constant. A common
        // strict opposite sign proves exactly one first crossing per
        // initial state, even if some intervening samples were uncertain.
        if gs == self.initial_sign {
            self.safe_time = t;
            self.safe_box = y;
            return Ok(false);
        }
        if gs == 0 {
            return Ok(false);
        }
        let mut lo = self.safe_time;
        let mut hi = t;
        let dy = hull(self.safe_box, y)?
            .scale(-self.mode.rate)?
            .add(pt(self.mode.forcing)?)?;
        let mut hint = None;
        if self.route.hint()
            && let Some(p) = &self.poly
        {
            self.work.represented_isolations += 1;
            if let Ok(r) = RepresentedPolynomial::from_segment(s, k, p)
                .and_then(|p| p.isolate(RootRoute::DictionaryCompanion, 2f64.powi(-40)))
                && let Some(v) = r.roots.first()
            {
                let c = v.time[0] * 0.5 + v.time[1] * 0.5;
                if lo < c && c < hi {
                    self.work.root_hints += 1;
                    hint = Some(c);
                }
            }
        }
        // Use strict signs and physical derivative interval, never the
        // represented hint as authority. Retain bracket across all roots.
        for _ in 0..96 {
            // A tighter internal target reduces propagated uncertainty in
            // repeated events; if unattainable, still allow requested width.
            if sub_up(hi, lo)? <= self.policy.time_tol * 0.0625 {
                break;
            }
            let old = [lo, hi];
            let x = hint.take().unwrap_or(lo + (hi - lo) * 0.5);
            for x in [
                x,
                old[0] + (old[1] - old[0]) * 0.25,
                old[0] + (old[1] - old[0]) * 0.75,
            ] {
                if !(lo < x && x < hi) {
                    continue;
                }
                let f = self.state_box(s, x)?.sub(pt(g.threshold)?)?;
                [lo, hi] =
                    contract_root_once(lo, hi, x, f, dy, self.initial_sign, -self.initial_sign)?;
                if sub_up(hi, lo)? <= self.policy.time_tol * 0.0625 {
                    break;
                }
            }
            if old == [lo, hi] {
                break;
            }
            self.work.contractions += 1;
        }
        self.crossing = Some(Crossing {
            bracket: [lo, hi],
            kind: if self.initial_sign < 0 {
                RootKind::Up
            } else {
                RootKind::Down
            },
        });
        Ok(true)
    }
}
fn synchronize(
    table: &RetroTable,
    g: Guard,
    next: Mode,
    bracket: [f64; 2],
    route: ResetRoute,
    w: &mut ResetWork,
) -> CoreResult<(Iv, Iv, f64, Option<f64>)> {
    let [lo, hi] = bracket;
    let duration = pt(hi)?.sub(Iv::new(lo, hi)?)?;
    let raw = g.reset.enclose(Iv::new(lo, hi)?, pt(g.threshold)?)?;
    let max_dt = duration.hi;
    // Positive affine gain makes the initial box endpoints sufficient.
    // All scalar trajectories are monotone, so hull of start and far end
    // contains every time in the synchronization gap.
    let l = direct_flow(table, next, 0., max_dt, raw.lo, w)?;
    let h = direct_flow(table, next, 0., max_dt, raw.hi, w)?;
    let gap = hull(raw, Iv::new(l.lo, h.hi)?)?;
    w.sync_rectangles += 1;
    let rect_width = sub_up(gap.hi, gap.lo)?;
    if let Some(new_guard) = next.guard
        && sign(gap.sub(pt(new_guard.threshold)?)?) == 0
    {
        return Err(bad(
            "next guard not strictly excluded over synchronization gap",
        ));
    }
    let mut sync = gap;
    let mut tr_width = None;
    if route.transport() {
        // F(tau)=Phi_new(hi-tau,R(tau,q)).
        // F'(tau)=exp(-a*(hi-tau))*(R_t+a*R-b).
        let x = duration.scale(next.rate)?;
        let gain = table.phi(Iv::new(x.lo.max(0.), x.hi)?, 0)?;
        w.gain_queries += 1;
        let deriv = gain.mul(
            raw.scale(next.rate)?
                .sub(pt(next.forcing)?)?
                .add(pt(g.reset.time_scale)?)?,
        )?;
        let anchored = g
            .reset
            .enclose(pt(hi)?, pt(g.threshold)?)?
            .add(Iv::new(-max_dt, 0.)?.mul(deriv)?)?;
        sync = intersect(gap, anchored)?;
        tr_width = Some(sub_up(sync.hi, sync.lo)?);
        w.sync_transport += 1;
    }
    Ok((sync, gap, rect_width, tr_width))
}
impl HybridSystem {
    fn validate(
        &self,
        start_mode: usize,
        y0: f64,
        end: f64,
        c: &AdaptiveStepConfig,
        p: ResetPolicy,
    ) -> CoreResult<()> {
        ac::check_affine_event_arithmetic()?;
        c.validate()?;
        if start_mode >= self.modes.len()
            || !y0.is_finite()
            || !end.is_finite()
            || end <= 0.
            || !p.time_tol.is_finite()
            || p.time_tol <= 0.
            || !p.state_tol.is_finite()
            || p.state_tol <= 0.
            || !p.min_advance.is_finite()
            || p.min_advance <= 0.
            || p.max_resets > 64
            || c.max_attempts > 100000
            || self.modes.iter().any(|m| {
                pt(end)
                    .and_then(|x| x.scale(m.rate))
                    .map_or(true, |v| v.hi > 16384.)
            })
        {
            return Err(bad("hybrid horizon/policy/domain"));
        }
        Ok(())
    }
    /// Transactional prefix: failed epochs do not commit a reset or time
    /// advance. Native attempts remain charged. Exact source owned here.
    pub fn run(
        &self,
        start_mode: usize,
        y0: f64,
        end: f64,
        config: &AdaptiveStepConfig,
        policy: ResetPolicy,
        route: ResetRoute,
    ) -> CoreResult<HybridRun> {
        self.validate(start_mode, y0, end, config, policy)?;
        let table = if route.full() {
            RetroTable::with_dictionary(RealMode::Hybrid)?
        } else {
            RetroTable::new()?
        };
        let mut out = HybridRun {
            route,
            status: HybridStatus::Unresolved,
            physical_qualified: false,
            time: 0.,
            mode: start_mode,
            center: y0,
            enclosure: route.physical().then_some([y0, y0]),
            radius: route.physical().then_some(0.),
            events: vec![],
            epochs: vec![],
            work: ResetWork::default(),
            counters: WorkCounters::default(),
            reason: None,
        };
        while out.time < end {
            let epoch = out.events.len();
            let m = self.modes[out.mode];
            let start = out.time;
            let y = out.center;
            let initial = if let Some([l, h]) = out.enclosure {
                Iv::new(l, h)?
            } else {
                pt(y)?
            };
            let mut scan = match Scan::new(m, start, y, initial, route, &table, policy, epoch) {
                Ok(s) => s,
                Err(e) => {
                    out.reason = Some(e.to_string());
                    return Ok(out);
                }
            };
            let ode = ode(m)?;
            let r = {
                let mut hook = |s: &FastDenseSegments| scan.inspect(s);
                integrate_rodas5p_fast_dense_with_control(
                    &ode,
                    None,
                    (start, end),
                    &[y],
                    config,
                    FastInteriorControl::Off,
                    &[],
                    &mut hook,
                )?
            };
            let counters = r.fast.observed.counters;
            out.counters.accumulate(counters);
            let numerical_end = *r
                .segments
                .times()
                .last()
                .ok_or_else(|| bad("no native start"))?;
            if scan.work.accepted_inspections != r.segments.len() as u64
                || counters.accepted_steps != r.segments.len() as u64
            {
                return Err(bad("native step accounting invariant"));
            }
            let stop_reason = r
                .partial
                .as_ref()
                .map_or("Horizon".into(), |p| format!("{:?}", p.reason));
            let mut receipt = EpochReceipt {
                epoch,
                mode: out.mode,
                start,
                start_center: y,
                start_enclosure: out.enclosure,
                numerical_end,
                committed_end: start,
                committed: false,
                stop_reason,
                inspected_steps: scan.work.accepted_inspections,
                counters,
                endpoint_proofs: std::mem::take(&mut scan.proofs),
            };
            let fail = if let Some(e) = scan.failed.take() {
                Some((HybridStatus::Unresolved, e))
            } else if scan.crossing.is_some()
                && numerical_end < end
                && !r
                    .partial
                    .as_ref()
                    .is_some_and(|p| p.reason == DriverStopReason::ClientStop)
            {
                Some((
                    HybridStatus::DriverFailure,
                    "crossing did not stop native driver".into(),
                ))
            } else if scan.crossing.is_none() && (!r.fast.observed.success || numerical_end != end)
            {
                Some((HybridStatus::DriverFailure, r.fast.observed.message.clone()))
            } else {
                None
            };
            if let Some((status, why)) = fail {
                out.status = status;
                out.reason = Some(why);
                out.work.absorb(&scan.work);
                out.epochs.push(receipt);
                return Ok(out);
            }
            if let Some(cross) = scan.crossing {
                let guard = m.guard.expect("crossing has guard");
                let [lo, hi] = cross.bracket;
                // Atomic prepare: all guards, widths, arithmetic and new
                // initial state proved BEFORE committing time/mode/event.
                let prepared = (|| -> CoreResult<ResetReceipt> {
                    let width = sub_up(hi, lo)?;
                    if !(start < hi && hi <= numerical_end)
                        || lo < start
                        || sub_down(lo, start)? < policy.min_advance
                    {
                        return Err(bad("minimum advance or represented clock"));
                    }
                    if route.physical() && width > policy.time_tol {
                        return Err(bad("physical event bracket too wide"));
                    }
                    let mut pre = [0.];
                    r.segments.evaluate_into(hi, &mut pre)?;
                    let mut reset = ResetReceipt {
                        epoch,
                        from: out.mode,
                        to: guard.next,
                        bracket: [lo, hi],
                        width,
                        kind: cross.kind,
                        sync_time: hi,
                        pre_center: pre[0],
                        guard_value: guard.threshold,
                        reset_center: 0.,
                        post_enclosure: None,
                        post_radius: None,
                        gap_tube: None,
                        rectangle_width: None,
                        transported_width: None,
                    };
                    let next = self.modes[guard.next];
                    if route.physical() {
                        let (iv, gap, rect, trans) =
                            synchronize(&table, guard, next, [lo, hi], route, &mut scan.work)?;
                        let c = center(iv);
                        let rad = radius(iv, c)?;
                        if rad > policy.state_tol {
                            return Err(bad("synchronized state radius too wide"));
                        }
                        reset.reset_center = c;
                        reset.post_enclosure = Some(iv.as_pair());
                        reset.post_radius = Some(rad);
                        reset.gap_tube = Some(gap.as_pair());
                        reset.rectangle_width = Some(rect);
                        reset.transported_width = trans;
                    } else {
                        reset.reset_center = guard.reset.evaluate(hi, pre[0])?;
                        if next
                            .guard
                            .is_some_and(|g| reset.reset_center == g.threshold)
                        {
                            return Err(bad("represented reset lands on next guard"));
                        }
                    }
                    Ok(reset)
                })();
                if epoch >= policy.max_resets {
                    out.status = HybridStatus::ResetLimit;
                    out.reason = Some("reset cap reached".into());
                    out.work.absorb(&scan.work);
                    out.epochs.push(receipt);
                    return Ok(out);
                }
                match prepared {
                    Ok(reset) => {
                        out.time = reset.sync_time;
                        out.mode = reset.to;
                        out.center = reset.reset_center;
                        out.enclosure = reset.post_enclosure;
                        out.radius = reset.post_radius;
                        receipt.committed = true;
                        receipt.committed_end = out.time;
                        out.events.push(reset);
                        out.epochs.push(receipt);
                        out.work.absorb(&scan.work);
                    }
                    Err(e) => {
                        out.reason = Some(e.to_string());
                        out.work.absorb(&scan.work);
                        out.epochs.push(receipt);
                        return Ok(out);
                    }
                }
            } else {
                let final_center = r.segments.state(r.segments.len())[0];
                let (enclosure, rad) = if route.physical() {
                    if m.guard.is_some() && scan.last_sign != scan.initial_sign {
                        out.reason =
                            Some("final guard sign unresolved; cannot exclude root".into());
                        out.work.absorb(&scan.work);
                        out.epochs.push(receipt);
                        return Ok(out);
                    }
                    let rad = radius(scan.last_box, final_center)?;
                    if rad > policy.state_tol {
                        out.reason = Some("final state radius too wide".into());
                        out.work.absorb(&scan.work);
                        out.epochs.push(receipt);
                        return Ok(out);
                    }
                    (Some(scan.last_box.as_pair()), Some(rad))
                } else {
                    (None, None)
                };
                out.time = end;
                out.center = final_center;
                out.enclosure = enclosure;
                out.radius = rad;
                receipt.committed = true;
                receipt.committed_end = end;
                out.epochs.push(receipt);
                out.work.absorb(&scan.work);
            }
        }
        out.status = HybridStatus::Complete;
        out.physical_qualified = route.physical();
        Ok(out)
    }
    /// Same-contract selection over complete physical hybrid runs. All
    /// candidate work, even refused attempts, is retained in the receipt.
    pub fn adaptive(
        &self,
        start_mode: usize,
        y0: f64,
        end: f64,
        config: &AdaptiveStepConfig,
        policy: ResetPolicy,
        portfolio: &mut Portfolio,
    ) -> CoreResult<HybridSelection> {
        self.validate(start_mode, y0, end, config, policy)?;
        let routes = if policy.time_tol >= 1e-6 {
            [
                Route::ResetCarry,
                Route::ResetLegacyFlow,
                Route::ResetDictionaryFlow,
                Route::ResetTransport,
                Route::ResetHinted,
                Route::ResetCombined,
            ]
        } else {
            [
                Route::ResetTransport,
                Route::ResetCombined,
                Route::ResetDictionaryFlow,
                Route::ResetLegacyFlow,
                Route::ResetHinted,
                Route::ResetCarry,
            ]
        };
        let mut charged_native = WorkCounters::default();
        let mut charged_work = ResetWork::default();
        let mut attempted_runs = vec![];
        let chosen = portfolio.choose(
            "hybrid-reset",
            &[
                self.modes.len() as u64,
                policy.time_tol.to_bits(),
                start_mode as u64,
            ],
            &routes,
            policy.state_tol,
            |r| {
                let route = match r {
                    Route::ResetCarry => ResetRoute::Carry,
                    Route::ResetLegacyFlow => ResetRoute::LegacyFlow,
                    Route::ResetDictionaryFlow => ResetRoute::DictionaryFlow,
                    Route::ResetHinted => ResetRoute::Hinted,
                    Route::ResetTransport => ResetRoute::Transport,
                    Route::ResetCombined => ResetRoute::Combined,
                    _ => return Err(bad("route")),
                };
                let run = self.run(start_mode, y0, end, config, policy, route)?;
                charged_native.accumulate(run.counters);
                charged_work.absorb(&run.work);
                let bound = if run.physical_qualified {
                    run.events
                        .iter()
                        .filter_map(|r| r.post_radius)
                        .fold(run.radius.unwrap_or(f64::INFINITY), f64::max)
                } else {
                    f64::INFINITY
                };
                attempted_runs.push(run.clone());
                Ok((run, bound))
            },
        )?;
        Ok(HybridSelection {
            value: chosen.value,
            attempts: chosen.attempts,
            charged_native,
            charged_work,
            attempted_runs,
        })
    }
}
