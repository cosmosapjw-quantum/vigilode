//! Problem families of loop node V06 (shared Krylov basis), included by the
//! `stage_clients_krylov_*` test targets with `#[path]`. Every problem is
//! JVP-only, autonomous and allocation-free in its callbacks, and comes with
//! the analytic diagonal of its Jacobian for the Jacobi option.
//!
//! The declared instances are those of
//! `research/production_loop_20261116/shared_krylov/PREDECLARED.json`
//! ([`declared`]); the generators also build the small instances of the
//! contract tests. The second half runs one arm of the node on a family
//! ([`run_arm`]).
#![allow(dead_code)]

use std::f64::consts::PI;
use std::sync::Arc;

use rodas5p_core::{CoreError, CoreResult, WorkCounters};
use rodas5p_integrators::gray_scott::gray_scott_2d;
use rodas5p_integrators::stage_clients::krylov::{
    JacobianDiagonalFn, KrylovPreconditioner, KrylovStageClient, KrylovStats, KrylovStatsHandle,
    PerStageGmresOptions, ResidualAuditClient, ResidualAuditHandle, ResidualAuditReport,
    SharedBasisOptions, StageStoppingRule,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSchedule, RoutingOptions, StageLinearSystem,
    integrate_rodas5p_fast_client_observed_with_atol,
    integrate_rodas5p_mf_fast_observed_gmres_into,
};

/// A problem with its start, span, tolerance ratio and Jacobian diagonal.
pub struct Family {
    pub id: String,
    pub problem: OdeProblem,
    pub y0: Vec<f64>,
    pub span: (f64, f64),
    /// `atol = rtol * atol_over_rtol`.
    pub atol_over_rtol: f64,
    pub diagonal: JacobianDiagonalFn,
}

fn jvp_only(
    name: String,
    n: usize,
    rhs: impl Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync + 'static,
    jvp: impl Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync + 'static,
) -> CoreResult<OdeProblem> {
    OdeProblem::new(
        name,
        n,
        Arc::new(rhs),
        None,
        None,
        Some(Arc::new(jvp)),
        None,
        true,
        None,
        None,
    )
}

/// 1D Brusselator, method of lines, on `[0, length]` with Dirichlet values
/// `u = 1`, `v = 3`, diffusion `alpha = 1/50`, `x_i = (i + 1) dx`,
/// `dx = length / (cells + 1)`, state interleaved `(u_i, v_i)`, start
/// `u = 1 + sin(2 pi x)`, `v = 3`, span `[0, 10]`. With `length = 1` it is
/// the Brusselator of `b1_problems`.
pub fn bruss1d(cells: usize, length: f64) -> CoreResult<Family> {
    if cells == 0 || !(length.is_finite() && length > 0.0) {
        return Err(CoreError::InvalidInput("invalid Brusselator size".into()));
    }
    let n = 2 * cells;
    let dx = length / (cells as f64 + 1.0);
    let c = 0.02 / (dx * dx);
    let rhs = move |_t: f64, y: &[f64], out: &mut [f64]| {
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (ul, vl) = if i == 0 {
                (1.0, 3.0)
            } else {
                (y[2 * i - 2], y[2 * i - 1])
            };
            let (ur, vr) = if i + 1 == cells {
                (1.0, 3.0)
            } else {
                (y[2 * i + 2], y[2 * i + 3])
            };
            out[2 * i] = 1.0 + u * u * v - 4.0 * u + c * (ul - 2.0 * u + ur);
            out[2 * i + 1] = 3.0 * u - u * u * v + c * (vl - 2.0 * v + vr);
        }
        Ok(())
    };
    let jvp = move |_t: f64, y: &[f64], d: &[f64], out: &mut [f64]| {
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (du, dv) = (d[2 * i], d[2 * i + 1]);
            let (dul, dvl) = if i == 0 {
                (0.0, 0.0)
            } else {
                (d[2 * i - 2], d[2 * i - 1])
            };
            let (dur, dvr) = if i + 1 == cells {
                (0.0, 0.0)
            } else {
                (d[2 * i + 2], d[2 * i + 3])
            };
            out[2 * i] = (2.0 * u * v - 4.0) * du + u * u * dv + c * (dul - 2.0 * du + dur);
            out[2 * i + 1] = (3.0 - 2.0 * u * v) * du - u * u * dv + c * (dvl - 2.0 * dv + dvr);
        }
        Ok(())
    };
    let diagonal: JacobianDiagonalFn = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            out[2 * i] = 2.0 * u * v - 4.0 - 2.0 * c;
            out[2 * i + 1] = -u * u - 2.0 * c;
        }
        Ok(())
    });
    let y0 = (0..cells)
        .flat_map(|i| {
            let x = (i as f64 + 1.0) * dx;
            [1.0 + (2.0 * PI * x).sin(), 3.0]
        })
        .collect();
    Ok(Family {
        id: format!("bruss1d-{cells}-L{length}"),
        problem: jvp_only(format!("bruss1d-{cells}-L{length}"), n, rhs, jvp)?,
        y0,
        span: (0.0, 10.0),
        atol_over_rtol: 1.0,
        diagonal,
    })
}

/// The crate's 2D Gray-Scott problem ([`gray_scott_2d`]): periodic unit
/// square, `D_u = 2e-3`, `D_v = 1e-3`, `F = 0.04`, `k = 0.06`, `u` block
/// then `v` block, span `[0, 20]`.
pub fn grayscott2d(grid: usize) -> CoreResult<Family> {
    if grid < 3 {
        return Err(CoreError::InvalidInput(
            "the Gray-Scott diagonal needs a grid of at least 3".into(),
        ));
    }
    let (problem, y0) = gray_scott_2d(grid)?;
    let cells = grid * grid;
    let inv_dx2 = (grid * grid) as f64;
    let diagonal: JacobianDiagonalFn = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        let (u, v) = y.split_at(cells);
        let (out_u, out_v) = out.split_at_mut(cells);
        for i in 0..cells {
            out_u[i] = -4.0 * 2.0e-3 * inv_dx2 - v[i] * v[i] - 0.04;
            out_v[i] = -4.0 * 1.0e-3 * inv_dx2 + 2.0 * u[i] * v[i] - 0.1;
        }
        Ok(())
    });
    Ok(Family {
        id: format!("grayscott2d-{grid}"),
        problem,
        y0,
        span: (0.0, 20.0),
        atol_over_rtol: 1.0,
        diagonal,
    })
}

/// 2D advection-diffusion-reaction on the unit square,
/// `u_t = eps lap(u) - a (u_x + u_y) + g u (u - 1/2)(1 - u)` with
/// `eps = 0.01`, `a = 10`, `g = 100`: first-order upwind advection,
/// second-order diffusion, cell centres `((i + 1/2) dx, (j + 1/2) dx)`,
/// `dx = 1/grid`, index `i + grid j`, zero-gradient boundaries by mirrored
/// ghost cells; start `256 (x y (1 - x)(1 - y))^2 + 0.3`, span `[0, 0.1]`.
pub fn adr2d(grid: usize) -> CoreResult<Family> {
    if grid < 2 {
        return Err(CoreError::InvalidInput("invalid ADR grid".into()));
    }
    const EPS: f64 = 0.01;
    const A: f64 = 10.0;
    const G: f64 = 100.0;
    let n = grid * grid;
    let dx = 1.0 / grid as f64;
    let diffusion = EPS / (dx * dx);
    let advection = A / dx;
    // The linear stencil applied to `w`: neighbours outside the square are
    // the cell itself.
    let stencil = move |w: &[f64], i: usize, j: usize| {
        let k = i + grid * j;
        let centre = w[k];
        let left = if i > 0 { w[k - 1] } else { centre };
        let right = if i + 1 < grid { w[k + 1] } else { centre };
        let down = if j > 0 { w[k - grid] } else { centre };
        let up = if j + 1 < grid { w[k + grid] } else { centre };
        diffusion * (left + right + down + up - 4.0 * centre)
            - advection * ((centre - left) + (centre - down))
    };
    let rhs = move |_t: f64, y: &[f64], out: &mut [f64]| {
        for j in 0..grid {
            for i in 0..grid {
                let u = y[i + grid * j];
                out[i + grid * j] = stencil(y, i, j) + G * u * (u - 0.5) * (1.0 - u);
            }
        }
        Ok(())
    };
    let jvp = move |_t: f64, y: &[f64], d: &[f64], out: &mut [f64]| {
        for j in 0..grid {
            for i in 0..grid {
                let k = i + grid * j;
                let u = y[k];
                out[k] = stencil(d, i, j) + G * (-3.0 * u * u + 3.0 * u - 0.5) * d[k];
            }
        }
        Ok(())
    };
    let diagonal: JacobianDiagonalFn = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        for j in 0..grid {
            for i in 0..grid {
                let k = i + grid * j;
                let u = y[k];
                let neighbours = f64::from(
                    u8::from(i > 0)
                        + u8::from(i + 1 < grid)
                        + u8::from(j > 0)
                        + u8::from(j + 1 < grid),
                );
                let upwind = f64::from(u8::from(i > 0) + u8::from(j > 0));
                out[k] = -diffusion * neighbours - advection * upwind
                    + G * (-3.0 * u * u + 3.0 * u - 0.5);
            }
        }
        Ok(())
    });
    let mut y0 = vec![0.0; n];
    for j in 0..grid {
        for i in 0..grid {
            let (x, y) = ((i as f64 + 0.5) * dx, (j as f64 + 0.5) * dx);
            y0[i + grid * j] = 256.0 * (x * y * (1.0 - x) * (1.0 - y)).powi(2) + 0.3;
        }
    }
    Ok(Family {
        id: format!("adr2d-{grid}"),
        problem: jvp_only(format!("adr2d-{grid}"), n, rhs, jvp)?,
        y0,
        span: (0.0, 0.1),
        atol_over_rtol: 1.0,
        diagonal,
    })
}

/// Robertson kinetics in every cell of `[0, 1]` with diffusion `1e-5` on
/// every species, zero-gradient boundaries, state interleaved
/// `(y1, y2, y3)`, `x_i = (i + 1/2) dx`, `dx = 1/cells`; start
/// `y1 = 1 - sin^2(pi x)/2`, `y2 = 0`, `y3 = sin^2(pi x)/2`, span `[0, 40]`.
pub fn robertson_rd1d(cells: usize) -> CoreResult<Family> {
    if cells < 2 {
        return Err(CoreError::InvalidInput("invalid Robertson size".into()));
    }
    let n = 3 * cells;
    let dx = 1.0 / cells as f64;
    let d = 1.0e-5 / (dx * dx);
    // `d (w_left + w_right - 2 w)` for species `s` of cell `i`.
    let laplace = move |w: &[f64], i: usize, s: usize| {
        let centre = w[3 * i + s];
        let left = if i > 0 { w[3 * i + s - 3] } else { centre };
        let right = if i + 1 < cells {
            w[3 * i + s + 3]
        } else {
            centre
        };
        d * (left + right - 2.0 * centre)
    };
    let rhs = move |_t: f64, y: &[f64], out: &mut [f64]| {
        for i in 0..cells {
            let (y1, y2, y3) = (y[3 * i], y[3 * i + 1], y[3 * i + 2]);
            out[3 * i] = -0.04 * y1 + 1.0e4 * y2 * y3 + laplace(y, i, 0);
            out[3 * i + 1] = 0.04 * y1 - 1.0e4 * y2 * y3 - 3.0e7 * y2 * y2 + laplace(y, i, 1);
            out[3 * i + 2] = 3.0e7 * y2 * y2 + laplace(y, i, 2);
        }
        Ok(())
    };
    let jvp = move |_t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        for i in 0..cells {
            let (y2, y3) = (y[3 * i + 1], y[3 * i + 2]);
            let (v1, v2, v3) = (v[3 * i], v[3 * i + 1], v[3 * i + 2]);
            let exchange = 1.0e4 * (y3 * v2 + y2 * v3);
            out[3 * i] = -0.04 * v1 + exchange + laplace(v, i, 0);
            out[3 * i + 1] = 0.04 * v1 - exchange - 6.0e7 * y2 * v2 + laplace(v, i, 1);
            out[3 * i + 2] = 6.0e7 * y2 * v2 + laplace(v, i, 2);
        }
        Ok(())
    };
    let diagonal: JacobianDiagonalFn = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        for i in 0..cells {
            let neighbours = f64::from(u8::from(i > 0) + u8::from(i + 1 < cells));
            let (y2, y3) = (y[3 * i + 1], y[3 * i + 2]);
            out[3 * i] = -0.04 - d * neighbours;
            out[3 * i + 1] = -1.0e4 * y3 - 6.0e7 * y2 - d * neighbours;
            out[3 * i + 2] = -d * neighbours;
        }
        Ok(())
    });
    let y0 = (0..cells)
        .flat_map(|i| {
            let x = (i as f64 + 0.5) * dx;
            let s = 0.5 * (PI * x).sin().powi(2);
            [1.0 - s, 0.0, s]
        })
        .collect();
    Ok(Family {
        id: format!("robertson_rd1d-{cells}"),
        problem: jvp_only(format!("robertson_rd1d-{cells}"), n, rhs, jvp)?,
        y0,
        span: (0.0, 40.0),
        atol_over_rtol: 1.0e-4,
        diagonal,
    })
}

/// The linear 1D heat equation `u_t = nu u_xx` with zero Dirichlet values on
/// `n` interior points of `[0, 1]`, start `sin(pi x) + sin(7 pi x)/2`, span
/// `[0, 0.05]`. Development and contract tests only.
pub fn heat1d(n: usize, nu: f64) -> CoreResult<Family> {
    if n == 0 {
        return Err(CoreError::InvalidInput("invalid heat size".into()));
    }
    let dx = 1.0 / (n as f64 + 1.0);
    let c = nu / (dx * dx);
    let apply = move |w: &[f64], out: &mut [f64]| {
        for i in 0..n {
            let left = if i == 0 { 0.0 } else { w[i - 1] };
            let right = if i + 1 == n { 0.0 } else { w[i + 1] };
            out[i] = c * (left - 2.0 * w[i] + right);
        }
    };
    let rhs = move |_t: f64, y: &[f64], out: &mut [f64]| {
        apply(y, out);
        Ok(())
    };
    let jvp = move |_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
        apply(v, out);
        Ok(())
    };
    let diagonal: JacobianDiagonalFn = Arc::new(move |_t: f64, _y: &[f64], out: &mut [f64]| {
        out.fill(-2.0 * c);
        Ok(())
    });
    let y0 = (0..n)
        .map(|i| {
            let x = (i as f64 + 1.0) * dx;
            (PI * x).sin() + 0.5 * (7.0 * PI * x).sin()
        })
        .collect();
    Ok(Family {
        id: format!("heat1d-{n}"),
        problem: jvp_only(format!("heat1d-{n}"), n, rhs, jvp)?,
        y0,
        span: (0.0, 0.05),
        atol_over_rtol: 1.0,
        diagonal,
    })
}

/// The instances of `PREDECLARED.json` by id, the exploratory stiff
/// Brusselator, and the development instances (`dev_*`).
pub fn declared(id: &str) -> CoreResult<Family> {
    let mut family = match id {
        "bruss1d" => bruss1d(512, 8.0)?,
        "grayscott2d" => grayscott2d(32)?,
        "adr2d" => adr2d(32)?,
        "robertson_rd1d" => robertson_rd1d(342)?,
        "bruss1d_stiff" => bruss1d(512, 1.0)?,
        "dev_bruss1d" => bruss1d(32, 1.0)?,
        "dev_grayscott2d" => grayscott2d(8)?,
        "dev_adr2d" => adr2d(8)?,
        "dev_robertson_rd1d" => robertson_rd1d(8)?,
        "dev_heat1d" => heat1d(64, 1.0)?,
        _ => {
            return Err(CoreError::InvalidInput(format!("unknown V06 family {id}")));
        }
    };
    family.id = id.to_string();
    Ok(family)
}

/// The arms of `PREDECLARED.json`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arm {
    /// A: the protected matrix-free driver as routed in production.
    Protected,
    /// B0: per-stage GMRES client with the protected kernel's cadence.
    PerStageShipped,
    /// B: per-stage GMRES client.
    PerStage,
    /// C: shared-basis client.
    Shared,
}

impl Arm {
    pub fn label(self) -> &'static str {
        match self {
            Self::Protected => "A",
            Self::PerStageShipped => "B0",
            Self::PerStage => "B",
            Self::Shared => "C",
        }
    }

    pub fn parse(label: &str) -> Option<Self> {
        [
            Self::Protected,
            Self::PerStageShipped,
            Self::PerStage,
            Self::Shared,
        ]
        .into_iter()
        .find(|arm| arm.label() == label)
    }
}

/// One arm's configuration; [`ArmOptions::new`] gives the declared one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArmOptions {
    pub arm: Arm,
    /// Right Jacobi instead of no preconditioner (client arms only).
    pub jacobi: bool,
    /// Wrap the client in the residual audit (client arms only).
    pub audit: bool,
    /// Shared basis: largest dimension.
    pub max_dim: usize,
    /// Shared basis: keep it across an attempt rejected by the estimate.
    pub keep: bool,
}

impl ArmOptions {
    pub fn new(arm: Arm) -> Self {
        Self {
            arm,
            jacobi: false,
            audit: false,
            max_dim: SharedBasisOptions::baseline().max_dim,
            keep: false,
        }
    }
}

/// The result of [`run_arm`].
pub struct ArmRun {
    pub success: bool,
    pub attempts: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub counters: WorkCounters,
    pub t_end: f64,
    pub y_end: Vec<f64>,
    /// Client arms: the client's statistics.
    pub stats: Option<KrylovStats>,
    /// Client arms with the audit: its report.
    pub audit: Option<ResidualAuditReport>,
}

/// The integrator configuration of `PREDECLARED.json`.
pub fn adaptive(family: &Family, rtol: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: rtol * family.atol_over_rtol,
        rtol,
        initial_step: 1.0e-6,
        min_step: 1.0e-14,
        max_step: family.span.1 - family.span.0,
        max_attempts: 1_000_000,
        ..AdaptiveStepConfig::default()
    }
}

/// The client of a client arm with the handles of its statistics and audit.
pub fn build_client(
    family: &Family,
    options: &ArmOptions,
) -> CoreResult<(
    Box<dyn StageLinearSystem>,
    KrylovStatsHandle,
    Option<ResidualAuditHandle>,
)> {
    let n = family.problem.dimension;
    let rule = StageStoppingRule::protected_production()?;
    let preconditioner = if options.jacobi {
        KrylovPreconditioner::RightJacobi(Arc::clone(&family.diagonal))
    } else {
        KrylovPreconditioner::None
    };
    let client = match options.arm {
        Arm::Protected => {
            return Err(CoreError::InvalidInput(
                "arm A is the protected driver, not a client".into(),
            ));
        }
        Arm::PerStageShipped => KrylovStageClient::per_stage(
            n,
            rule,
            PerStageGmresOptions::shipped_cadence(),
            preconditioner,
        )?,
        Arm::PerStage => {
            KrylovStageClient::per_stage(n, rule, PerStageGmresOptions::baseline(), preconditioner)?
        }
        Arm::Shared => KrylovStageClient::shared_basis(
            n,
            rule,
            SharedBasisOptions {
                max_dim: options.max_dim,
                keep_across_rejection: options.keep,
                ..SharedBasisOptions::baseline()
            },
            PerStageGmresOptions::baseline(),
            preconditioner,
        )?,
    };
    let stats = client.stats_handle();
    if options.audit {
        let audited = ResidualAuditClient::new(client, rule);
        let audit = audited.handle();
        Ok((Box::new(audited), stats, Some(audit)))
    } else {
        Ok((Box::new(client), stats, None))
    }
}

/// Run one arm on `family` at `rtol` with the output schedule `[t0, t1]`.
pub fn run_arm(family: &Family, rtol: f64, options: &ArmOptions) -> CoreResult<ArmRun> {
    let config = adaptive(family, rtol);
    let output = OutputSchedule::new(vec![family.span.0, family.span.1])?;
    if options.arm == Arm::Protected {
        let linear = RoutingOptions::default().matrix_free;
        let run = integrate_rodas5p_mf_fast_observed_gmres_into(
            &family.problem,
            family.span,
            &family.y0,
            &linear,
            &config,
            &output,
        )?;
        return Ok(ArmRun {
            success: run.observed.success,
            attempts: run.attempts,
            accepted: run.accepted_steps,
            rejected: run.rejected_steps,
            counters: run.observed.counters,
            t_end: run.observed.t.last().copied().unwrap_or(family.span.0),
            y_end: run.observed.y.last().cloned().unwrap_or_default(),
            stats: None,
            audit: None,
        });
    }
    let (client, stats, audit) = build_client(family, options)?;
    let atol = vec![config.atol; family.problem.dimension];
    let (run, _partial, client) = integrate_rodas5p_fast_client_observed_with_atol(
        &family.problem,
        client,
        family.span,
        &family.y0,
        &config,
        &output,
        &atol,
    )?;
    drop(client);
    Ok(ArmRun {
        success: run.observed.success,
        attempts: run.attempts,
        accepted: run.accepted_steps,
        rejected: run.rejected_steps,
        counters: run.observed.counters,
        t_end: run.observed.t.last().copied().unwrap_or(family.span.0),
        y_end: run.observed.y.last().cloned().unwrap_or_default(),
        stats: Some(stats.snapshot()),
        audit: audit.map(|handle| handle.snapshot()),
    })
}
