//! Source-bound positive-shift transpose resolvent and actual native stage client.
#![allow(missing_docs)]
use super::*;
use rodas5p_core::{
    ApplyCategory, ClosureOperator, CoreError, CoreResult, IdentityPreconditioner, Preconditioner,
    PreconditionerSide, WorkCounters, apply_counted,
    directed::{Interval as Iv, add_up, div_up, mul_up},
};
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSchedule, ReusableFastClientWorkspace, Rodas5pFastResult,
    StageLinearSystem, integrate_rodas5p_fast_client_observed_with_atol,
};
use rodas5p_krylov::{
    CapacityGrowth, GmresCapacity, GmresConfig, GmresMonitorDecision, GmresWorkspace,
    solve_gmres_into, solve_gmres_into_right_flexible_triggered_monitored,
    solve_gmres_into_right_monitored,
};
use serde::Serialize;
use std::sync::{Arc, Mutex};
const Z: Iv = Iv { lo: 0., hi: 0. };
mod residual;
pub use residual::*;
fn err(s: &str) -> CoreError {
    CoreError::LinearSolve(format!("shifted adjoint: {s}"))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ShiftedAdjointRoute {
    Identity,
    Longitude,
    Meridional,
    ShortIdentity,
    EarlyIdentity,
    EarlyLongitude,
    FourierTranspose,
    TriggeredIdentity,
    TriggeredLongitude,
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct ShiftedAdjointReport {
    pub residual_l1: f64,
    pub error_l1: f64,
    pub rhs_l1: f64,
    pub iterations: u64,
    pub matvecs: u64,
    pub pc_apps: u64,
    #[serde(skip_serializing_if = "zero_count")]
    pub screen_checks: u64,
    #[serde(skip_serializing_if = "zero_count")]
    pub directed_checks: u64,
    #[serde(skip_serializing_if = "zero_count")]
    pub prefix_solves: u64,
    #[serde(skip_serializing_if = "zero_count")]
    pub seed_accepts: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub residual_profile: Option<ResidualProfile>,
}
fn zero_count(x: &u64) -> bool {
    *x == 0
}
struct Right<'a>(&'a dyn Preconditioner);
impl Preconditioner for Right<'_> {
    fn dimension(&self) -> usize {
        self.0.dimension()
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        self.0.apply(x, y)
    }
    fn is_identity(&self) -> bool {
        self.0.is_identity()
    }
    fn side(&self) -> PreconditionerSide {
        PreconditionerSide::Right
    }
}
impl ShiftedAdjointRoute {
    fn early(self) -> bool {
        matches!(
            self,
            Self::EarlyIdentity
                | Self::EarlyLongitude
                | Self::TriggeredIdentity
                | Self::TriggeredLongitude
        )
    }
    fn triggered(self) -> bool {
        matches!(self, Self::TriggeredIdentity | Self::TriggeredLongitude)
    }
    fn direct(self) -> bool {
        self == Self::FourierTranspose
    }
}
pub struct ShiftedAdjointWorkspace {
    grid: DiffusionGrid,
    route: ShiftedAdjointRoute,
    line: Option<LongitudeLinePreconditioner>,
    gmres: GmresWorkspace,
    config: GmresConfig,
    capacity: GmresCapacity,
    z: Option<f64>,
    x: Vec<f64>,
    res: Vec<Iv>,
    seed: Vec<f64>,
    probe: Vec<f64>,
    fourier: Option<Box<super::separable::TransposeKernel>>,
    residual: Option<Box<ResidualWorkspace>>,
}
impl ShiftedAdjointWorkspace {
    pub fn new(grid: &DiffusionGrid, route: ShiftedAdjointRoute) -> CoreResult<Self> {
        Self::new_impl(grid, route, None)
    }
    /// Opt-in original-source residual algorithm for direct Fourier stages.
    pub fn new_with_residual(
        grid: &DiffusionGrid,
        route: ShiftedAdjointRoute,
        plan: ResidualPlan,
    ) -> CoreResult<Self> {
        if !route.direct() {
            return Err(err("residual policy requires direct Fourier"));
        }
        Self::new_impl(grid, route, Some(plan))
    }
    fn new_impl(
        grid: &DiffusionGrid,
        route: ShiftedAdjointRoute,
        residual: Option<ResidualPlan>,
    ) -> CoreResult<Self> {
        rodas5p_integrators::affine_event_certificate::check_affine_event_arithmetic()?;
        let n = grid.dimension();
        let columns = (if route == ShiftedAdjointRoute::ShortIdentity {
            8
        } else {
            32
        })
        .min(n);
        let mut gmres = GmresWorkspace::default();
        if !route.direct() {
            gmres = GmresWorkspace::with_ls_workspace();
            gmres.set_givens_least_squares(true);
            gmres.reserve(n, columns)?;
        }
        let fourier = if route.direct() {
            Some(Box::new(super::separable::TransposeKernel::new(grid)))
        } else {
            None
        };
        let line = if !matches!(
            route,
            ShiftedAdjointRoute::Longitude
                | ShiftedAdjointRoute::Meridional
                | ShiftedAdjointRoute::EarlyLongitude
                | ShiftedAdjointRoute::TriggeredLongitude
        ) {
            None
        } else {
            Some(
                grid.longitude_preconditioner(1., SpatialRows::Serial)
                    .map_err(|e| err(&e.to_string()))?,
            )
        };
        Ok(Self {
            grid: grid.clone(),
            route,
            line,
            gmres,
            config: GmresConfig {
                restart: columns,
                max_arnoldi: 512,
                rtol: 1e-12,
                atol: 1e-14,
            },
            capacity: GmresCapacity {
                max_dimension: n,
                max_columns: columns,
                growth: CapacityGrowth::Refuse,
            },
            z: None,
            x: vec![0.; n],
            res: if residual.is_none() {
                vec![Z; n]
            } else {
                Vec::new()
            },
            seed: if route.early() {
                vec![0.; n]
            } else {
                Vec::new()
            },
            probe: if route.early() {
                vec![0.; n]
            } else {
                Vec::new()
            },
            fourier,
            residual: residual
                .map(|p| ResidualWorkspace::new(grid, p).map(Box::new))
                .transpose()?,
        })
    }
    pub fn prepare(&mut self, z: f64) -> CoreResult<()> {
        self.z = None;
        if !z.is_finite() || z <= 0. {
            return Err(err("positive shift required"));
        }
        if let Some(pc) = self.line.as_mut() {
            pc.refactor_shifted(&self.grid, z, self.route == ShiftedAdjointRoute::Meridional)?;
        }
        if let Some(k) = self.fourier.as_mut() {
            k.prepare(z)?;
        }
        if let Some(r) = self.residual.as_mut() {
            r.prepare(z)?;
        }
        self.z = Some(z);
        Ok(())
    }
    fn candidate(&mut self, b: &[f64], c: &mut WorkCounters) -> CoreResult<ShiftedAdjointReport> {
        let z = self.z.ok_or_else(|| err("not prepared"))?;
        let n = self.grid.dimension();
        if b.len() != n || b.iter().any(|x| !x.is_finite()) {
            return Err(err("rhs shape/nonfinite"));
        }
        let g = &self.grid;
        if let Some(k) = self.fourier.as_mut() {
            self.x.copy_from_slice(b);
            let start = self.residual.as_ref().map(|_| std::time::Instant::now());
            k.solve(&mut self.x);
            c.direct_solve_calls += 1;
            if let Some(r) = self.residual.as_mut() {
                let solve_ns = elapsed_ns(start.unwrap());
                let start = std::time::Instant::now();
                let a = r.evaluate(b, &self.x)?;
                let certificate_ns = elapsed_ns(start);
                if !a.admitted {
                    return Err(err("transpose Fourier residual exceeds stage budget"));
                }
                c.linear_solves += 1;
                return Ok(ShiftedAdjointReport {
                    residual_l1: a.residual_l1,
                    error_l1: a.error_l1,
                    rhs_l1: a.rhs_l1,
                    directed_checks: a.interval_checks,
                    residual_profile: Some(ResidualProfile {
                        solve_ns,
                        certificate_ns,
                        fast_checks: a.fast_checks,
                        interval_checks: a.interval_checks,
                        fallbacks: a.fallbacks,
                        fmas: a.fmas,
                        max_rounding: a.rounding_upper,
                    }),
                    ..Default::default()
                });
            }
            let (rho, upper, rhs) = directed_residual(g, z, b, &self.x, &mut self.res)?;
            if rho > add_up(mul_up(1e-10, rhs)?, 1e-13)? {
                return Err(err("transpose Fourier residual exceeds stage budget"));
            }
            c.linear_solves += 1;
            return Ok(ShiftedAdjointReport {
                residual_l1: rho,
                error_l1: upper,
                rhs_l1: rhs,
                directed_checks: 1,
                ..Default::default()
            });
        }
        let op = ClosureOperator::new(n, |v: &[f64], out: &mut [f64]| {
            g.apply_transpose_gather(v, out)
                .map_err(|e| err(&e.to_string()))?;
            for (o, &x) in out.iter_mut().zip(v) {
                *o = z * x - *o;
            }
            Ok(())
        });
        let identity = IdentityPreconditioner::new(n);
        let pc: &dyn Preconditioner = match &self.line {
            Some(l) => l,
            None => &identity,
        };
        if self.route.early() {
            let before = *c;
            let rhs = b.iter().try_fold(0., |s, x| add_up(s, x.abs()))?;
            let allowed = add_up(mul_up(1e-10, rhs)?, 1e-13)?;
            for (x, &v) in self.seed.iter_mut().zip(b) {
                *x = v / z;
            }
            // An approximate inverse-shift seed is only a proposal. Its
            // exact original residual may already meet the target.
            let mut checks = 1u64;
            let mut screens = 0u64;
            let seed = directed_residual(g, z, b, &self.seed, &mut self.res)?;
            if seed.0 <= allowed {
                self.x.copy_from_slice(&self.seed);
                c.linear_solves += 1;
                return Ok(ShiftedAdjointReport {
                    residual_l1: seed.0,
                    error_l1: seed.1,
                    rhs_l1: rhs,
                    directed_checks: checks,
                    seed_accepts: 1,
                    ..Default::default()
                });
            }
            let mut receipt = None;
            let mut monitor = |_column: u64, v: &[f64], c: &mut WorkCounters| {
                screens += 1;
                apply_counted(&op, v, &mut self.probe, c, ApplyCategory::Diagnostic)?;
                let proposal = b
                    .iter()
                    .zip(&self.probe)
                    .map(|(&b, &a)| (b - a).abs())
                    .sum::<f64>();
                // This screen can decline a good candidate, but cannot admit
                // a bad one. The directed original-source gate is authority.
                if proposal <= 0.75 * allowed {
                    checks += 1;
                    let r = directed_residual(g, z, b, v, &mut self.res)?;
                    if r.0 <= allowed {
                        receipt = Some(r);
                        return Ok(GmresMonitorDecision::Accept);
                    }
                }
                Ok(GmresMonitorDecision::Continue)
            };
            let r = if self.route.triggered() {
                let projected = allowed / (2. * (n as f64).sqrt());
                solve_gmres_into_right_flexible_triggered_monitored(
                    &op,
                    &Right(pc),
                    b,
                    &self.seed,
                    &self.config,
                    projected,
                    self.capacity,
                    &mut self.x,
                    &mut self.gmres,
                    c,
                    &mut monitor,
                )?
            } else {
                solve_gmres_into_right_monitored(
                    &op,
                    &Right(pc),
                    b,
                    &self.seed,
                    &self.config,
                    &mut self.x,
                    &mut self.gmres,
                    c,
                    &mut monitor,
                )?
            };
            if !r.accepted {
                return Err(err("monitored original residual target exhausted"));
            }
            let (rho, upper, _) = receipt.ok_or_else(|| err("missing monitored certificate"))?;
            return Ok(ShiftedAdjointReport {
                residual_l1: rho,
                error_l1: upper,
                rhs_l1: rhs,
                iterations: r.iterations,
                matvecs: c.linear_matvecs - before.linear_matvecs,
                pc_apps: c.preconditioner_apps - before.preconditioner_apps,
                screen_checks: screens,
                directed_checks: checks,
                prefix_solves: r.least_squares_solves,
                seed_accepts: 0,
                ..Default::default()
            });
        }
        let r = solve_gmres_into(
            &op,
            pc,
            b,
            None,
            &self.config,
            None,
            &mut self.x,
            &mut self.gmres,
            self.capacity,
            c,
        )?;
        let (rho, upper, rhs) = directed_residual(&self.grid, z, b, &self.x, &mut self.res)?;
        // Relative stage residual gate, not a global time-discretization claim.
        let allowed = add_up(mul_up(1e-10, rhs)?, 1e-13)?;
        if rho > allowed {
            return Err(err("original represented residual exceeds stage budget"));
        }
        Ok(ShiftedAdjointReport {
            residual_l1: rho,
            error_l1: upper,
            rhs_l1: rhs,
            iterations: r.iterations,
            matvecs: r.matvecs,
            pc_apps: r.preconditioner_apps,
            ..Default::default()
        })
    }
    pub fn solve_into(
        &mut self,
        b: &[f64],
        out: &mut [f64],
        c: &mut WorkCounters,
    ) -> CoreResult<ShiftedAdjointReport> {
        if out.len() != self.grid.dimension() {
            return Err(err("output shape"));
        }
        let r = self.candidate(b, c)?;
        out.copy_from_slice(&self.x);
        Ok(r)
    }
}
/// Original represented stencil; no reliance on ideal area similarity.
fn directed_residual(
    g: &DiffusionGrid,
    z: f64,
    b: &[f64],
    x: &[f64],
    r: &mut [Iv],
) -> CoreResult<(f64, f64, f64)> {
    rodas5p_integrators::affine_event_certificate::check_affine_event_arithmetic()?;
    let n = g.dimension();
    if !z.is_finite() || z <= 0. || b.len() != n || x.len() != n || r.len() != n {
        return Err(err("residual shape/clock"));
    }
    let mut rhs = 0.;
    for i in 0..n {
        r[i] = Iv::point(b[i])?.sub(Iv::point(x[i])?.scale(z)?)?;
        rhs = add_up(rhs, b[i].abs())?;
    }
    for (i, &v) in x.iter().enumerate() {
        for (j, a) in g.represented_row(i).unwrap() {
            if i != j && a != 0. {
                let p = Iv::point(v)?.scale(a)?;
                r[i] = r[i].sub(p)?;
                r[j] = r[j].add(p)?;
            }
        }
    }
    let rho = r.iter().try_fold(0., |s, v| add_up(s, v.mag()))?;
    Ok((rho, div_up(rho, z)?, rhs))
}
/// Check a separately proposed shifted solution, e.g. weighted Fourier.
pub fn shifted_adjoint_residual_into(
    grid: &DiffusionGrid,
    z: f64,
    b: &[f64],
    x: &[f64],
    scratch: &mut [Iv],
) -> CoreResult<ShiftedAdjointReport> {
    let (rho, error, rhs) = directed_residual(grid, z, b, x, scratch)?;
    Ok(ShiftedAdjointReport {
        residual_l1: rho,
        error_l1: error,
        rhs_l1: rhs,
        ..Default::default()
    })
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct ShiftedNativeWork {
    pub preparations: usize,
    pub solves: usize,
    pub returned: usize,
    pub max_error_l1: f64,
    pub max_residual_l1: f64,
    #[serde(skip_serializing_if = "zero_count")]
    pub screen_checks: u64,
    #[serde(skip_serializing_if = "zero_count")]
    pub directed_checks: u64,
    #[serde(skip_serializing_if = "zero_count")]
    pub prefix_solves: u64,
    #[serde(skip_serializing_if = "zero_count")]
    pub seed_accepts: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub residual_profile: Option<ResidualProfile>,
}
/// Opt-in profiling, successful returned stage solves only.
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct ResidualProfile {
    pub solve_ns: u64,
    pub certificate_ns: u64,
    pub fast_checks: u64,
    pub interval_checks: u64,
    pub fallbacks: u64,
    pub fmas: u64,
    pub max_rounding: f64,
}
fn elapsed_ns(t: std::time::Instant) -> u64 {
    t.elapsed().as_nanos().min(u64::MAX as u128) as u64
}
impl ResidualProfile {
    fn add(&mut self, r: Self) {
        self.solve_ns = self.solve_ns.saturating_add(r.solve_ns);
        self.certificate_ns = self.certificate_ns.saturating_add(r.certificate_ns);
        self.fast_checks += r.fast_checks;
        self.interval_checks += r.interval_checks;
        self.fallbacks += r.fallbacks;
        self.fmas += r.fmas;
        self.max_rounding = self.max_rounding.max(r.max_rounding);
    }
}
struct Client {
    w: ShiftedAdjointWorkspace,
    report: Arc<Mutex<ShiftedNativeWork>>,
}
impl StageLinearSystem for Client {
    fn dimension(&self) -> usize {
        self.w.grid.dimension()
    }
    fn linearize(
        &mut self,
        _: &OdeProblem,
        _: f64,
        _: &[f64],
        _: &mut WorkCounters,
    ) -> CoreResult<()> {
        self.w.z = None;
        Ok(())
    }
    fn factor(&mut self, z: f64, c: &mut WorkCounters) -> CoreResult<()> {
        self.report.lock().unwrap().preparations += 1;
        if self.w.line.is_some() || self.w.route.direct() {
            c.direct_factorizations += 1;
        }
        self.w.prepare(z)
    }
    fn solve_in_place(&mut self, b: &mut [f64], c: &mut WorkCounters) -> CoreResult<()> {
        self.report.lock().unwrap().solves += 1;
        // The native driver has charged this stage solve before calling us.
        // GMRES counts its own solve too; retain all other work on both Ok/Err.
        let charged = c.linear_solves;
        let result = self.w.candidate(b, c);
        c.linear_solves = charged;
        let r = result?;
        b.copy_from_slice(&self.w.x);
        let mut w = self.report.lock().unwrap();
        w.returned += 1;
        w.max_error_l1 = w.max_error_l1.max(r.error_l1);
        w.max_residual_l1 = w.max_residual_l1.max(r.residual_l1);
        w.screen_checks += r.screen_checks;
        w.directed_checks += r.directed_checks;
        w.prefix_solves += r.prefix_solves;
        w.seed_accepts += r.seed_accepts;
        if let Some(p) = r.residual_profile {
            w.residual_profile
                .get_or_insert_with(Default::default)
                .add(p);
        }
        Ok(())
    }
}
/// Actual native protected RODAS5P adjoint driver, source-bound callback and
/// stage client constructed together. No external mismatched JVP accepted.
pub fn solve_spatial_adjoint_shifted(
    grid: &DiffusionGrid,
    span: (f64, f64),
    initial: &[f64],
    adaptive: &AdaptiveStepConfig,
    route: ShiftedAdjointRoute,
) -> CoreResult<(Rodas5pFastResult, ShiftedNativeWork)> {
    solve_spatial_adjoint_impl(grid, span, initial, adaptive, route, None)
}
pub fn solve_spatial_adjoint_shifted_with_residual(
    grid: &DiffusionGrid,
    span: (f64, f64),
    initial: &[f64],
    adaptive: &AdaptiveStepConfig,
    plan: ResidualPlan,
) -> CoreResult<(Rodas5pFastResult, ShiftedNativeWork)> {
    solve_spatial_adjoint_impl(
        grid,
        span,
        initial,
        adaptive,
        ShiftedAdjointRoute::FourierTranspose,
        Some(plan),
    )
}
fn solve_spatial_adjoint_impl(
    grid: &DiffusionGrid,
    span: (f64, f64),
    initial: &[f64],
    adaptive: &AdaptiveStepConfig,
    route: ShiftedAdjointRoute,
    residual: Option<ResidualPlan>,
) -> CoreResult<(Rodas5pFastResult, ShiftedNativeWork)> {
    let p = spatial_transpose_problem(grid)?;
    let report = Arc::new(Mutex::new(ShiftedNativeWork::default()));
    let client = Client {
        w: ShiftedAdjointWorkspace::new_impl(grid, route, residual)?,
        report: report.clone(),
    };
    let out = OutputSchedule::new(vec![span.0, span.1])?;
    let (r, _partial, _client) = integrate_rodas5p_fast_client_observed_with_atol(
        &p,
        Box::new(client),
        span,
        initial,
        adaptive,
        &out,
        &vec![adaptive.atol; grid.dimension()],
    )?;
    let w = *report.lock().unwrap();
    Ok((r, w))
}

// Keep callback/source identity fixed for all repeated native solves.
fn spatial_transpose_problem(grid: &DiffusionGrid) -> CoreResult<OdeProblem> {
    let g = grid.clone();
    let j = grid.clone();
    OdeProblem::new(
        "represented spatial transpose",
        grid.dimension(),
        Arc::new(move |_, v, o| {
            g.apply_transpose_gather(v, o)
                .map_err(|e| err(&e.to_string()))
        }),
        None,
        None,
        Some(Arc::new(move |_, _, v, o| {
            j.apply_transpose_gather(v, o)
                .map_err(|e| err(&e.to_string()))
        })),
        None,
        true,
        None,
        None,
    )
}
/// Reuse native stage buffers, exact-source Fourier client, schedule and
/// tolerance vector for repeated spatial adjoint IVPs on one grid/span.
/// Each solve still returns an owning observation; no full-driver zeroheap claim.
pub struct SpatialAdjointDriverWorkspace {
    problem: OdeProblem,
    driver: ReusableFastClientWorkspace,
    report: Arc<Mutex<ShiftedNativeWork>>,
    span: (f64, f64),
    schedule: OutputSchedule,
    atol: Vec<f64>,
}
impl SpatialAdjointDriverWorkspace {
    pub fn new(grid: &DiffusionGrid, span: (f64, f64), plan: ResidualPlan) -> CoreResult<Self> {
        let problem = spatial_transpose_problem(grid)?;
        let report = Arc::new(Mutex::new(ShiftedNativeWork::default()));
        let client = Client {
            w: ShiftedAdjointWorkspace::new_with_residual(
                grid,
                ShiftedAdjointRoute::FourierTranspose,
                plan,
            )?,
            report: report.clone(),
        };
        let driver = ReusableFastClientWorkspace::new(Box::new(client))?;
        let schedule = OutputSchedule::new(vec![span.0, span.1])?;
        Ok(Self {
            problem,
            driver,
            report,
            span,
            schedule,
            atol: vec![0.; grid.dimension()],
        })
    }
    pub fn solve_with_schedule(
        &mut self,
        initial: &[f64],
        adaptive: &AdaptiveStepConfig,
        schedule: &OutputSchedule,
    ) -> CoreResult<(
        Rodas5pFastResult,
        Option<rodas5p_integrators::DriverPartial>,
        ShiftedNativeWork,
    )> {
        *self.report.lock().unwrap() = ShiftedNativeWork::default();
        self.atol.fill(adaptive.atol);
        let (r, p) = self.driver.solve(
            &self.problem,
            self.span,
            initial,
            adaptive,
            schedule,
            &self.atol,
        )?;
        Ok((r, p, *self.report.lock().unwrap()))
    }
    pub fn solve_flat_into<'a>(
        &mut self,
        initial: &[f64],
        adaptive: &AdaptiveStepConfig,
        schedule: &OutputSchedule,
        plan: rodas5p_integrators::FastArenaPlan,
        arena: &'a mut rodas5p_integrators::FastFlatOutputArena,
    ) -> CoreResult<(rodas5p_integrators::FastFlatArenaRun<'a>, ShiftedNativeWork)> {
        *self.report.lock().unwrap() = ShiftedNativeWork::default();
        self.atol.fill(adaptive.atol);
        let r = self.driver.solve_flat_into(
            &self.problem,
            self.span,
            initial,
            adaptive,
            schedule,
            &self.atol,
            plan,
            arena,
        )?;
        Ok((r, *self.report.lock().unwrap()))
    }
    pub fn solve_into<'a>(
        &mut self,
        initial: &[f64],
        adaptive: &AdaptiveStepConfig,
        schedule: &OutputSchedule,
        plan: rodas5p_integrators::FastArenaPlan,
        arena: &'a mut rodas5p_integrators::FastOutputArena,
    ) -> CoreResult<(rodas5p_integrators::FastArenaRun<'a>, ShiftedNativeWork)> {
        *self.report.lock().unwrap() = ShiftedNativeWork::default();
        self.atol.fill(adaptive.atol);
        let r = self.driver.solve_into(
            &self.problem,
            self.span,
            initial,
            adaptive,
            schedule,
            &self.atol,
            plan,
            arena,
        )?;
        Ok((r, *self.report.lock().unwrap()))
    }
    pub fn solve(
        &mut self,
        initial: &[f64],
        adaptive: &AdaptiveStepConfig,
    ) -> CoreResult<(Rodas5pFastResult, ShiftedNativeWork)> {
        *self.report.lock().unwrap() = ShiftedNativeWork::default();
        self.atol.fill(adaptive.atol);
        let (r, _partial) = self.driver.solve(
            &self.problem,
            self.span,
            initial,
            adaptive,
            &self.schedule,
            &self.atol,
        )?;
        let w = *self.report.lock().unwrap();
        Ok((r, w))
    }
}
