//! Opt-in typed research portfolio. Cost observations never authorize an
//! answer: every returned candidate passes the adapter's original contract.
//! History is process-local and scoped by family/shape/tolerance, not a
//! certificate cache. No default solver or callback replay is changed.
use crate::{
    AdaptiveStepConfig, OdeProblem, OutputSchedule, ParallelExecution,
    fourier_path_candidate as fft, fourier_path_certificate as fc, integrate_rodas5p_fast_observed,
    retro_causal as ac, retro_green as green, retro_nonlinear as nl, retro_temporal as tm,
};
use rodas5p_core::warp_cubic::{CubicCoefficients, CubicConfig, CubicPencil};
use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, direct_solve,
    directed::{Interval, add_up, div_down, mul_up, sqrt_up},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::Instant,
};
fn invalid(s: &str) -> CoreError {
    CoreError::InvalidInput(format!("retro portfolio: {s}"))
}
const HISTORY_KEYS: usize = 128;
const MAX_ROUTES: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum Route {
    CubicFolded,
    CubicBlock,
    FourierDirect,
    FourierLegacy,
    FftFull,
    FftCrt,
    TemporalSequential,
    TemporalScan,
    TemporalPadded,
    TemporalCrt,
    NonlinearSequential,
    NonlinearChord,
    Jet2,
    Jet4,
    Jet6,
    JetMulti,
    AffineForward,
    AffineUniform,
    AffineRetro,
    Green,
    GreenHalf,
    GreenQuarter,
    NativeRodas,
    SpatialDirect,
    SpatialPaired,
    SpatialComplex,
    SpatialNative,
    SpatialIvPPaired,
    SpatialIvPComplex,
    #[cfg(feature = "retro-dictionary")]
    GreenFullFast,
    #[cfg(feature = "retro-dictionary")]
    GreenFullDirected,
    #[cfg(feature = "retro-dictionary")]
    GreenFullHybrid,
    #[cfg(feature = "retro-dictionary")]
    PreparedSerial,
    #[cfg(feature = "retro-dictionary")]
    PreparedFaerBatch,
    #[cfg(feature = "retro-dictionary")]
    PreparedRayon,
    #[cfg(feature = "retro-dictionary")]
    AperiodicSequential,
    #[cfg(feature = "retro-dictionary")]
    AperiodicScan,
    #[cfg(feature = "retro-dictionary")]
    QuotientDirect,
    #[cfg(feature = "retro-dictionary")]
    QuotientFolded,
    #[cfg(feature = "polynomial-events")]
    EventSturm,
    #[cfg(feature = "polynomial-events")]
    EventBernstein,
    #[cfg(feature = "polynomial-events")]
    EventDictionary,
    #[cfg(feature = "polynomial-events")]
    EventCompanion,
    #[cfg(feature = "polynomial-events")]
    EventDictionaryCompanion,
    #[cfg(feature = "polynomial-events")]
    PhysicalUnity,
    #[cfg(feature = "polynomial-events")]
    PhysicalAffine,
    #[cfg(feature = "polynomial-events")]
    PhysicalGreen,
    #[cfg(feature = "polynomial-events")]
    PhysicalWeighted,
    #[cfg(feature = "polynomial-events")]
    PhysicalHint,
    #[cfg(feature = "polynomial-events")]
    PhysicalSource,
    #[cfg(feature = "polynomial-events")]
    ResetCarry,
    #[cfg(feature = "polynomial-events")]
    ResetLegacyFlow,
    #[cfg(feature = "polynomial-events")]
    ResetDictionaryFlow,
    #[cfg(feature = "polynomial-events")]
    ResetHinted,
    #[cfg(feature = "polynomial-events")]
    ResetTransport,
    #[cfg(feature = "polynomial-events")]
    ResetCombined,
    #[cfg(feature = "polynomial-events")]
    DaeDirect,
    #[cfg(feature = "polynomial-events")]
    DaeReuse,
    #[cfg(feature = "polynomial-events")]
    DaeBatch,
    #[cfg(feature = "polynomial-events")]
    DaePreparedSerial,
    #[cfg(feature = "polynomial-events")]
    DaePreparedBatch,
    #[cfg(feature = "polynomial-events")]
    DaePreparedRayon,
    #[cfg(feature = "polynomial-events")]
    DaeScalarRoot,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum AttemptStatus {
    Qualified,
    Rejected,
    Error,
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct Attempt {
    pub route: Route,
    pub status: AttemptStatus,
    pub elapsed_ns: u64,
    pub bound: Option<f64>,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct Selection<T> {
    pub value: Option<T>,
    pub selected: Option<Route>,
    pub attempts: Vec<Attempt>,
}
impl<T> Selection<T> {
    pub fn total_elapsed_ns(&self) -> u128 {
        self.attempts.iter().map(|a| a.elapsed_ns as u128).sum()
    }
}
#[derive(Clone, Debug, Default)]
struct Cost {
    trials: u64,
    last_failed: bool,
    samples: [u64; 5],
    sample_count: usize,
    next_sample: usize,
}
impl Cost {
    fn score(&self) -> Option<u64> {
        if self.last_failed || self.sample_count == 0 {
            return None;
        }
        let mut v = self.samples;
        v[..self.sample_count].sort_unstable();
        Some(v[self.sample_count / 2])
    }
    fn update(&mut self, ns: u64, success: bool) {
        self.trials = self.trials.saturating_add(1);
        self.last_failed = !success;
        if success {
            self.samples[self.next_sample] = ns;
            self.sample_count = (self.sample_count + 1).min(5);
            self.next_sample = (self.next_sample + 1) % 5;
        }
    }
}
#[derive(Default)]
struct History {
    calls: u64,
    costs: BTreeMap<Route, Cost>,
}
/// Bounded empirical cost selector, with periodic exploration. Not a
/// statistical model of the ODE, not a proof of optimality, no cached answer.
pub struct Portfolio {
    explore_every: u64,
    max_attempts: usize,
    history: BTreeMap<(&'static str, Vec<u64>), History>,
}
impl Portfolio {
    pub fn new(explore_every: usize, max_attempts: usize) -> CoreResult<Self> {
        if explore_every == 0 || !(1..=MAX_ROUTES).contains(&max_attempts) {
            return Err(invalid("policy bounds"));
        }
        Ok(Self {
            explore_every: explore_every as u64,
            max_attempts,
            history: BTreeMap::new(),
        })
    }
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
    /// Adapter boundary: callback must bind the original problem and return
    /// a valid independent upper bound for the SAME requested contract.
    /// This generic method does not establish the caller's mathematical
    /// bound. Built-in typed adapters below do. All errors/rejections cost
    /// time and are kept in the receipt. No callback side effects are undone.
    pub fn choose<T>(
        &mut self,
        family: &'static str,
        signature: &[u64],
        routes: &[Route],
        tol: f64,
        mut candidate: impl FnMut(Route) -> CoreResult<(T, f64)>,
    ) -> CoreResult<Selection<T>> {
        if !tol.is_finite()
            || tol <= 0.
            || routes.is_empty()
            || routes.len() > MAX_ROUTES
            || routes.iter().collect::<BTreeSet<_>>().len() != routes.len()
            || signature.len() > 64
        {
            return Err(invalid("selection contract"));
        }
        let mut sig = signature.to_vec();
        sig.push(tol.to_bits());
        let key = (family, sig);
        if self.history.len() >= HISTORY_KEYS && !self.history.contains_key(&key) {
            self.history.clear();
        }
        let h = self.history.entry(key).or_default();
        let explore = h.calls.is_multiple_of(self.explore_every);
        h.calls = h.calls.saturating_add(1);
        let mut order = routes.to_vec();
        // Cold ordering is adapter-defined. On exploration choose the least
        // tried route, including failed ones; otherwise rank recent success
        // by median of at most five timings. Failure is not permanent ban.
        if explore {
            order.sort_by_key(|r| h.costs.get(r).map_or(0, |c| c.trials));
        } else {
            order.sort_by_key(|r| match h.costs.get(r) {
                Some(c) if c.score().is_some() => (0, c.score().unwrap()),
                Some(c) if c.last_failed => (2, 0),
                _ => (1, 0),
            });
        }
        let mut attempts = Vec::new();
        for r in order.into_iter().take(self.max_attempts) {
            let start = Instant::now();
            let answer = candidate(r);
            let ns = start.elapsed().as_nanos().clamp(1, u64::MAX as u128) as u64;
            let (value, bound, reason, status) = match answer {
                Ok((v, b)) if b.is_finite() && b >= 0. && b <= tol => {
                    (Some(v), Some(b), None, AttemptStatus::Qualified)
                }
                Ok((_, b)) => (
                    None,
                    b.is_finite().then_some(b),
                    Some("original bound exceeds contract or is invalid".into()),
                    AttemptStatus::Rejected,
                ),
                Err(e) => (None, None, Some(e.to_string()), AttemptStatus::Error),
            };
            h.costs
                .entry(r)
                .or_default()
                .update(ns, status == AttemptStatus::Qualified);
            attempts.push(Attempt {
                route: r,
                status,
                elapsed_ns: ns,
                bound,
                reason,
            });
            if value.is_some() {
                return Ok(Selection {
                    value,
                    selected: Some(r),
                    attempts,
                });
            }
        }
        Ok(Selection {
            value: None,
            selected: None,
            attempts,
        })
    }
    pub fn cubic(
        &mut self,
        w: &DenseMatrix,
        d: &DenseMatrix,
        p: [f64; 3],
        b: &CubicCoefficients,
        tol: f64,
    ) -> CoreResult<Selection<CubicCoefficients>> {
        crate::affine_event_certificate::check_affine_event_arithmetic()?;
        let pencil = CubicPencil::new(w, d, p)?;
        let n = w.nrows();
        if b.iter()
            .any(|x| x.len() != n || x.iter().any(|v| !v.is_finite()))
        {
            return Err(invalid("cubic RHS"));
        }
        let mut routes = vec![Route::CubicFolded];
        if n <= 256 {
            routes.push(Route::CubicBlock);
        }
        self.choose(
            "cubic",
            &[
                n as u64,
                p[0].to_bits(),
                p[1].to_bits(),
                p[2].to_bits(),
                pencil.contraction_upper().to_bits(),
            ],
            &routes,
            tol,
            |r| {
                let x = if r == Route::CubicFolded {
                    pencil
                        .prepare()?
                        .solve(
                            b,
                            CubicConfig {
                                absolute_tolerance: tol,
                                max_updates: 64,
                            },
                        )?
                        .coefficients()
                        .clone()
                } else {
                    let mut a = DenseMatrix::zeros(3 * n, 3 * n);
                    for i in 0..n {
                        for j in 0..n {
                            for k in 0..3 {
                                a[(k * n + i, k * n + j)] = w[(i, j)];
                                if k < 2 {
                                    a[((k + 1) * n + i, k * n + j)] = d[(i, j)];
                                }
                            }
                            for k in 0..3 {
                                a[(k * n + i, 2 * n + j)] -= p[k] * d[(i, j)];
                            }
                        }
                    }
                    let flat = direct_solve(&a, &b.iter().flatten().copied().collect::<Vec<_>>())?;
                    std::array::from_fn(|j| flat[j * n..(j + 1) * n].to_vec())
                };
                let c = pencil.certify(b, &x, tol)?;
                Ok((x, c.error_upper()))
            },
        )
    }
    pub fn fourier(
        &mut self,
        model: &fc::FourierModel,
        t_end: f64,
        tol: f64,
        cap: f64,
    ) -> CoreResult<Selection<(Vec<fc::StepRecord>, fc::DriverCounts)>> {
        if !t_end.is_finite()
            || t_end <= 0.
            || !cap.is_finite()
            || cap <= 0.
            || !model.omega.is_finite()
            || !model.g.is_finite()
            || !model.eps.is_finite()
            || !matches!(model.sigma, -1 | 1)
        {
            return Err(invalid("Fourier domain"));
        }
        let routes = [
            Route::FourierDirect,
            Route::FftFull,
            Route::FftCrt,
            Route::FourierLegacy,
        ];
        self.choose(
            "fourier",
            &[
                model.omega.to_bits(),
                model.g.to_bits(),
                model.eps.to_bits(),
                model.sigma as u64,
                t_end.to_bits(),
                cap.to_bits(),
            ],
            &routes,
            tol,
            |r| {
                let mut old = fft::FftWork::default();
                let mut warp = fft::warp::Work::default();
                let mut rhs = |path: &[fc::FourierPath; 2],
                               model: &fc::FourierModel,
                               phase: &fc::FourierPath,
                               h: f64| {
                    match r {
                        Route::FourierLegacy => {
                            fft::fft_rhs_h(path, model, phase, h, None, false, &mut old)
                        }
                        Route::FftFull => fft::warp::rhs_h(
                            path,
                            model,
                            phase,
                            h,
                            fft::warp::Mode::Full,
                            &mut warp,
                        ),
                        Route::FftCrt => {
                            fft::warp::rhs_h(path, model, phase, h, fft::warp::Mode::Crt, &mut warp)
                        }
                        _ => Ok(fc::predictor_rhs_h(path, model, phase, h, 1.)),
                    }
                };
                let run = fc::run_driver(
                    model,
                    t_end,
                    tol,
                    cap,
                    if r == Route::FourierDirect {
                        None
                    } else {
                        Some(&mut rhs)
                    },
                )?;
                if run.0.last().is_none_or(|x| x.state.t() != t_end) {
                    return Err(invalid("incomplete Fourier path"));
                }
                let bound = run.0.iter().map(|x| x.physical_error).fold(0., f64::max);
                Ok((run, bound))
            },
        )
    }
    pub fn temporal(
        &mut self,
        pattern: &[DenseMatrix],
        y: &[f64],
        b: &[Vec<f64>],
        alpha: f64,
        exec: &ParallelExecution,
        tol: f64,
    ) -> CoreResult<Selection<tm::Report>> {
        if pattern.is_empty()
            || b.is_empty()
            || !b.len().is_multiple_of(pattern.len())
            || !alpha.is_finite()
            || !(1. / 4096. ..=1.).contains(&alpha)
        {
            return Err(invalid("temporal shape or alpha"));
        }
        let m = b.len() / pattern.len();
        let mut routes = vec![Route::TemporalSequential];
        if (2..=4096).contains(&m) && m.is_power_of_two() {
            routes.extend([
                Route::TemporalScan,
                Route::TemporalPadded,
                Route::TemporalCrt,
            ]);
        }
        self.choose(
            "temporal",
            &[
                y.len() as u64,
                pattern.len() as u64,
                m as u64,
                alpha.to_bits(),
                exec.threads() as u64,
            ],
            &routes,
            tol,
            |r| {
                let report = if r == Route::TemporalSequential {
                    tm::solve_sequential(pattern, y, b, tol)?
                } else {
                    let table = tm::TemporalTable::new(m, alpha)?;
                    let p = tm::Prepared::new(&table, pattern)?;
                    p.solve(
                        y,
                        b,
                        match r {
                            Route::TemporalScan => tm::Mode::Scan,
                            Route::TemporalPadded => tm::Mode::Padded,
                            _ => tm::Mode::Crt,
                        },
                        exec,
                        tol,
                    )?
                };
                let bound = report.error_upper;
                Ok((report, bound))
            },
        )
    }
    pub fn nonlinear(
        &mut self,
        w: &nl::Window<'_>,
        tol: f64,
        forecast: Option<&[f64]>,
    ) -> CoreResult<Selection<nl::Report>> {
        let routes = [
            Route::NonlinearSequential,
            Route::Jet4,
            Route::NonlinearChord,
            Route::Jet2,
            Route::Jet6,
            Route::JetMulti,
        ];
        self.choose(
            "nonlinear-discrete",
            &window_key(w, forecast.is_some()),
            &routes,
            tol,
            |r| {
                let v = nonlinear_candidate(w, r, tol, forecast)?;
                let bound = v.certificate.error_upper;
                Ok((v, bound))
            },
        )
    }
    /// All requested physical slab endpoint/dense bounds are rechecked by
    /// native affine controller. Normalize each target error by its own
    /// tolerance; never compare endpoint-only and dense-window requests.
    pub fn affine(
        &mut self,
        table: &green::RetroTable,
        slabs: &[ac::SlabSpec],
        targets: &[ac::Target],
        cap: f64,
        y0: f64,
    ) -> CoreResult<Selection<ac::Run>> {
        let p = ac::plan(table, slabs, targets, ac::Mode::Retro, cap)?;
        let routes = if p.forecast_fallback || p.uniform_fallback {
            [
                Route::AffineForward,
                Route::AffineUniform,
                Route::AffineRetro,
            ]
        } else if slabs.len() > 1 {
            [
                Route::AffineRetro,
                Route::AffineUniform,
                Route::AffineForward,
            ]
        } else {
            [
                Route::AffineForward,
                Route::AffineUniform,
                Route::AffineRetro,
            ]
        };
        // Scope timing to exact source/target metadata using a compact
        // deterministic digest, not the evolving state. Digest is cost only.
        let mut bits = vec![slabs.len() as u64, targets.len() as u64, cap.to_bits()];
        for s in slabs {
            bits.extend([
                s.end.to_bits(),
                s.rate.to_bits(),
                s.forcing.to_bits(),
                s.forecast.to_bits(),
                s.radius.to_bits(),
            ]);
        }
        for t in targets {
            bits.extend([
                t.slab as u64,
                match t.kind {
                    ac::OutputKind::Endpoint => 0,
                    ac::OutputKind::Dense => 1,
                },
                t.tolerance.to_bits(),
            ]);
        }
        let bytes: Vec<_> = bits.into_iter().flat_map(u64::to_le_bytes).collect();
        let digest = rodas5p_core::sha256_bytes(&bytes);
        let mut key = digest
            .chunks_exact(8)
            .map(|s| u64::from_le_bytes(s.try_into().unwrap()))
            .collect::<Vec<_>>();
        key.push(y0.to_bits());
        self.choose("affine-target-ratio", &key, &routes, 1., |r| {
            let mode = match r {
                Route::AffineForward => ac::Mode::Forward,
                Route::AffineUniform => ac::Mode::Uniform,
                _ => ac::Mode::Retro,
            };
            let run = ac::integrate(table, slabs, targets, mode, cap, y0)?;
            let mut bound = 0.0_f64;
            for t in targets {
                let s = &run.slabs[t.slab];
                let b = match t.kind {
                    ac::OutputKind::Endpoint => s.endpoint_upper(),
                    ac::OutputKind::Dense => s
                        .dense_upper()
                        .ok_or_else(|| invalid("missing dense bound"))?,
                };
                // Directed ratio; equality may be widened, conservatively.
                bound = bound.max(rodas5p_core::directed::div_up(b, t.tolerance)?);
            }
            Ok((run, bound))
        })
    }
    pub fn green_endpoint(
        &mut self,
        table: &green::RetroTable,
        model: &green::Model,
        y0: &[f64],
        t_end: f64,
        max_h: f64,
        tol: f64,
    ) -> CoreResult<Selection<green::Trajectory>> {
        if !max_h.is_finite() || max_h <= 0. || !t_end.is_finite() || t_end <= 0. {
            return Err(invalid("Green clock"));
        }
        self.choose(
            "green-endpoint",
            &[
                model.dimension() as u64,
                model.log_norm_upper().to_bits(),
                t_end.to_bits(),
                max_h.to_bits(),
            ],
            &[Route::Green, Route::GreenHalf, Route::GreenQuarter],
            tol,
            |r| {
                let h = max_h
                    * match r {
                        Route::GreenHalf => 0.5,
                        Route::GreenQuarter => 0.25,
                        _ => 1.,
                    };
                let run = green::integrate(table, model, y0, t_end, h, tol)?;
                let e = run.error_upper();
                Ok((run, e))
            },
        )
    }
    /// Compare legacy and complete table providers under the same original
    /// Green endpoint certificate. A failed candidate cannot authorize output.
    #[cfg(feature = "retro-dictionary")]
    pub fn green_dictionary_endpoint(
        &mut self,
        model: &green::Model,
        y0: &[f64],
        t_end: f64,
        max_h: f64,
        tol: f64,
    ) -> CoreResult<Selection<green::Trajectory>> {
        use rodas5p_core::retro_dictionary::RealMode;
        self.choose(
            "green-dictionary",
            &[
                model.dimension() as u64,
                model.log_norm_upper().to_bits(),
                t_end.to_bits(),
                max_h.to_bits(),
            ],
            &[
                Route::Green,
                Route::GreenFullFast,
                Route::GreenFullDirected,
                Route::GreenFullHybrid,
                Route::GreenHalf,
                Route::GreenQuarter,
            ],
            tol,
            |r| {
                let table = match r {
                    Route::GreenFullFast => {
                        green::RetroTable::with_dictionary(RealMode::TableFast)?
                    }
                    Route::GreenFullDirected => {
                        green::RetroTable::with_dictionary(RealMode::TableDirected)?
                    }
                    Route::GreenFullHybrid => green::RetroTable::with_dictionary(RealMode::Hybrid)?,
                    _ => green::RetroTable::new()?,
                };
                let h = max_h
                    * match r {
                        Route::GreenHalf => 0.5,
                        Route::GreenQuarter => 0.25,
                        _ => 1.,
                    };
                let v = green::integrate(&table, model, y0, t_end, h, tol)?;
                let e = v.error_upper();
                Ok((v, e))
            },
        )
    }
    /// Physical all-node absolute Euclidean error on a fixed exact mesh.
    /// Native RODAS and one-stage paths use the same independent ideal
    /// Hermite residual certificate. Compatible Green paths use their own
    /// certified propagation; no discrete tolerance is interpreted as LTE.
    pub fn physical_window(
        &mut self,
        w: &nl::Window<'_>,
        tol: f64,
    ) -> CoreResult<Selection<PhysicalWindow>> {
        let times = exact_times(w)?;
        let green_model = w.compatible_green().ok();
        let mut routes = vec![
            Route::NativeRodas,
            Route::NonlinearSequential,
            Route::Jet4,
            Route::NonlinearChord,
            Route::Jet2,
            Route::Jet6,
            Route::JetMulti,
        ];
        if green_model.is_some() {
            routes.insert(1, Route::Green);
        }
        self.choose(
            "physical-all-nodes",
            &window_key(w, false),
            &routes,
            tol,
            |r| {
                let (values, bound) = if r == Route::Green {
                    let model = green_model
                        .as_ref()
                        .ok_or_else(|| invalid("Green source"))?;
                    let table = green::RetroTable::new()?;
                    let weight = sqrt_up(w.dimension() as f64)?;
                    let local = div_down(div_down(tol, weight)?, w.steps() as f64)?;
                    let mut y = w.initial().to_vec();
                    let mut values = Vec::new();
                    let mut bound = 0.;
                    for _ in 0..w.steps() {
                        let run = green::integrate(&table, model, &y, w.h(), w.h(), local)?;
                        bound = add_up(bound, mul_up(weight, run.error_upper())?)?;
                        y = run.endpoint().to_vec();
                        values.extend(&y);
                    }
                    (values, bound)
                } else {
                    let values = if r == Route::NativeRodas {
                        native_values(w, &times, tol)?
                    } else {
                        nonlinear_candidate(w, r, div_down(tol, 64.)?, None)?.values
                    };
                    let e = w.continuous_certificate(&values)?.error_upper;
                    (values, e)
                };
                Ok((
                    PhysicalWindow {
                        values,
                        error_upper: bound,
                    },
                    bound,
                ))
            },
        )
    }
}
#[derive(Clone, Debug)]
pub struct PhysicalWindow {
    pub values: Vec<f64>,
    pub error_upper: f64,
}
fn window_key(w: &nl::Window<'_>, forecast: bool) -> [u64; 5] {
    [
        w.dimension() as u64,
        w.steps() as u64,
        w.h().to_bits(),
        w.theta().to_bits(),
        forecast as u64,
    ]
}
fn nonlinear_candidate(
    w: &nl::Window<'_>,
    r: Route,
    tol: f64,
    forecast: Option<&[f64]>,
) -> CoreResult<nl::Report> {
    let (axes, degree, mode) = match r {
        Route::NonlinearSequential => (1, 1, nl::Mode::Sequential),
        Route::NonlinearChord => (1, 1, nl::Mode::Chord),
        Route::Jet2 => (1, 2, nl::Mode::Jet),
        Route::Jet4 => (1, 4, nl::Mode::Jet),
        Route::Jet6 => (1, 6, nl::Mode::Jet),
        Route::JetMulti => (3, 4, nl::Mode::Jet),
        _ => return Err(invalid("nonlinear route")),
    };
    w.solve(&nl::JetTable::new(axes, degree)?, mode, tol, forecast)
}
fn exact_times(w: &nl::Window<'_>) -> CoreResult<Vec<f64>> {
    (0..=w.steps())
        .map(|i| {
            let x = Interval::point(w.h())?.scale(i as f64)?;
            if x.lo != x.hi {
                return Err(invalid("physical mesh not exactly represented"));
            }
            Ok(x.lo)
        })
        .collect()
}
fn native_values(w: &nl::Window<'_>, times: &[f64], tol: f64) -> CoreResult<Vec<f64>> {
    let model = Arc::new(w.model().clone());
    let jac = model.clone();
    let p = OdeProblem::new(
        "retro-portfolio-native",
        w.dimension(),
        Arc::new(move |_, y, out| {
            out.copy_from_slice(&model.rhs(y)?);
            Ok(())
        }),
        None,
        Some(Arc::new(move |_, y| jac.jacobian(y))),
        None,
        None,
        true,
        None,
        None,
    )?;
    let local = div_down(div_down(tol, 64.)?, w.steps() as f64)?;
    let cfg = AdaptiveStepConfig {
        atol: local,
        rtol: local,
        initial_step: w.h().min(0.001),
        min_step: w.h().min(1e-14),
        max_step: w.h(),
        max_attempts: 100000,
        ..AdaptiveStepConfig::default()
    };
    let output = OutputSchedule::new(times.to_vec())?;
    let run = integrate_rodas5p_fast_observed(
        &p,
        (0., *times.last().unwrap()),
        w.initial(),
        &cfg,
        &output,
    )?;
    if !run.observed.success || run.observed.t != times || run.observed.y.len() != times.len() {
        return Err(invalid("native incomplete physical mesh"));
    }
    Ok(run.observed.y.into_iter().skip(1).flatten().collect())
}
