//! Source-bound positive-shift transpose resolvent and actual native stage client.
#![allow(missing_docs)]
use super::*;
use rodas5p_core::{
    ClosureOperator, CoreError, CoreResult, IdentityPreconditioner, Preconditioner, WorkCounters,
    directed::{Interval as Iv, add_up, div_up, mul_up},
};
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSchedule, Rodas5pFastResult, StageLinearSystem,
    integrate_rodas5p_fast_client_observed_with_atol,
};
use rodas5p_krylov::{
    CapacityGrowth, GmresCapacity, GmresConfig, GmresWorkspace, solve_gmres_into,
};
use serde::Serialize;
use std::sync::{Arc, Mutex};
const Z: Iv = Iv { lo: 0., hi: 0. };
fn err(s: &str) -> CoreError {
    CoreError::LinearSolve(format!("shifted adjoint: {s}"))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ShiftedAdjointRoute {
    Identity,
    Longitude,
    Meridional,
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct ShiftedAdjointReport {
    pub residual_l1: f64,
    pub error_l1: f64,
    pub rhs_l1: f64,
    pub iterations: u64,
    pub matvecs: u64,
    pub pc_apps: u64,
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
}
impl ShiftedAdjointWorkspace {
    pub fn new(grid: &DiffusionGrid, route: ShiftedAdjointRoute) -> CoreResult<Self> {
        rodas5p_integrators::affine_event_certificate::check_affine_event_arithmetic()?;
        let n = grid.dimension();
        let columns = 32.min(n);
        let mut gmres = GmresWorkspace::with_ls_workspace();
        gmres.set_givens_least_squares(true);
        gmres.reserve(n, columns)?;
        let line = if route == ShiftedAdjointRoute::Identity {
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
            res: vec![Z; n],
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
        if self.w.line.is_some() {
            c.direct_factorizations += 1;
        }
        self.w.prepare(z)
    }
    fn solve_in_place(&mut self, b: &mut [f64], c: &mut WorkCounters) -> CoreResult<()> {
        self.report.lock().unwrap().solves += 1;
        let r = self.w.candidate(b, c)?;
        b.copy_from_slice(&self.w.x);
        let mut w = self.report.lock().unwrap();
        w.returned += 1;
        w.max_error_l1 = w.max_error_l1.max(r.error_l1);
        w.max_residual_l1 = w.max_residual_l1.max(r.residual_l1);
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
    let g = grid.clone();
    let j = grid.clone();
    let p = OdeProblem::new(
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
    )?;
    let report = Arc::new(Mutex::new(ShiftedNativeWork::default()));
    let client = Client {
        w: ShiftedAdjointWorkspace::new(grid, route)?,
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
