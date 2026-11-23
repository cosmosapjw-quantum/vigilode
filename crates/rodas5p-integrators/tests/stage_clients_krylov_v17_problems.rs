//! Problem families and arms of loop node V17 (matrix-free stage solve),
//! included by the `stage_clients_krylov_v17_*` test targets with `#[path]`.
//!
//! The declared instances are those of
//! `research/production_loop_20261116/mf_stage_solve/PREDECLARED.json`
//! ([`declared`]). Every problem is JVP-only, autonomous and
//! allocation-free in its callbacks and comes with the analytic diagonal of
//! its Jacobian. [`run_arm`] runs one arm; with the audit on, every stage
//! solve is checked against the original stage residual with JVPs that are
//! not part of the arm's counters:
//!
//! * client arms (B0, B, C): `ResidualAuditClient`;
//! * arm A (the protected driver): [`ProtectedAudit`], which rebuilds each
//!   stage system from the stage equations in the attempt observer.
#![allow(dead_code)]

#[path = "stage_clients_krylov_problems.rs"]
pub mod v06;

use std::f64::consts::PI;
use std::sync::Arc;

use rodas5p_core::{
    ApplyCategory, CoreError, CoreResult, DenseMatrix, LinearOperator, WorkCounters, apply_counted,
    rodas5p_coefficients, safe_l2,
};
use rodas5p_integrators::stage_clients::krylov::{
    JacobianDiagonalFn, KrylovPreconditioner, KrylovStageClient, KrylovStats, KrylovStatsHandle,
    PerStageGmresOptions, ResidualAuditClient, ResidualAuditHandle, StageStoppingRule,
    StepTiedTolerance,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSchedule, Rodas5pMfFastWorkspace, RoutingOptions,
    StageLinearSystem, integrate_rodas5p_fast_client_observed_with_atol,
    integrate_rodas5p_mf_fast_observed_gmres_into, raw_absolute_residual_budget,
};

pub use v06::Family;

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

/// 3D FitzHugh-Nagumo system with a heterogeneous drive on the unit cube,
/// zero-flux boundaries:
///
/// ```text
/// u_t = Du lap(u) + u - u^3/3 - v + I(x, y, z)
/// v_t = Dv lap(v) + 0.08 (u + 0.7 - 0.8 v)
/// I   = 0.5 + 0.4 cos(pi x) cos(2 pi y) cos(pi z)
/// ```
///
/// `Du = 0.1`, `Dv = 0.02`; `m^3` cell-centred finite volumes with the
/// 7-point Laplacian (a neighbour outside the cube is the cell itself),
/// `u` block then `v` block, index `i + m (j + m k)`; start
/// `u = -1.2 + 0.6 cos(pi x) cos(pi y)`, `v = -0.6 + 0.3 cos(pi z)`. The
/// drive keeps the oscillation spatially structured.
pub fn fhn3d(m: usize) -> CoreResult<Family> {
    if m < 2 {
        return Err(CoreError::InvalidInput(
            "invalid FitzHugh-Nagumo grid".into(),
        ));
    }
    const DU: f64 = 0.1;
    const DV: f64 = 0.02;
    let cells = m * m * m;
    let n = 2 * cells;
    let scale = (m * m) as f64;
    // `sum of the neighbours - 6 w` with mirrored ghost cells, times 1/dx^2.
    let laplace = move |w: &[f64], i: usize, j: usize, k: usize| {
        let c = i + m * (j + m * k);
        let centre = w[c];
        let xm = if i > 0 { w[c - 1] } else { centre };
        let xp = if i + 1 < m { w[c + 1] } else { centre };
        let ym = if j > 0 { w[c - m] } else { centre };
        let yp = if j + 1 < m { w[c + m] } else { centre };
        let zm = if k > 0 { w[c - m * m] } else { centre };
        let zp = if k + 1 < m { w[c + m * m] } else { centre };
        scale * (xm + xp + ym + yp + zm + zp - 6.0 * centre)
    };
    let dx = 1.0 / m as f64;
    let mut drive = vec![0.0; cells];
    let mut y0 = vec![0.0; n];
    for k in 0..m {
        for j in 0..m {
            for i in 0..m {
                let (x, y, z) = (
                    (i as f64 + 0.5) * dx,
                    (j as f64 + 0.5) * dx,
                    (k as f64 + 0.5) * dx,
                );
                let c = i + m * (j + m * k);
                drive[c] = 0.5 + 0.4 * (PI * x).cos() * (2.0 * PI * y).cos() * (PI * z).cos();
                y0[c] = -1.2 + 0.6 * (PI * x).cos() * (PI * y).cos();
                y0[cells + c] = -0.6 + 0.3 * (PI * z).cos();
            }
        }
    }
    let rhs = move |_t: f64, y: &[f64], out: &mut [f64]| {
        let (u, v) = y.split_at(cells);
        let (out_u, out_v) = out.split_at_mut(cells);
        for k in 0..m {
            for j in 0..m {
                for i in 0..m {
                    let c = i + m * (j + m * k);
                    let (uc, vc) = (u[c], v[c]);
                    out_u[c] = DU * laplace(u, i, j, k) + uc - uc * uc * uc / 3.0 - vc + drive[c];
                    out_v[c] = DV * laplace(v, i, j, k) + 0.08 * (uc + 0.7 - 0.8 * vc);
                }
            }
        }
        Ok(())
    };
    let jvp = move |_t: f64, y: &[f64], d: &[f64], out: &mut [f64]| {
        let u = &y[..cells];
        let (du, dv) = d.split_at(cells);
        let (out_u, out_v) = out.split_at_mut(cells);
        for k in 0..m {
            for j in 0..m {
                for i in 0..m {
                    let c = i + m * (j + m * k);
                    out_u[c] = DU * laplace(du, i, j, k) + (1.0 - u[c] * u[c]) * du[c] - dv[c];
                    out_v[c] = DV * laplace(dv, i, j, k) + 0.08 * (du[c] - 0.8 * dv[c]);
                }
            }
        }
        Ok(())
    };
    let diagonal: JacobianDiagonalFn = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        let u = &y[..cells];
        let (out_u, out_v) = out.split_at_mut(cells);
        for k in 0..m {
            for j in 0..m {
                for i in 0..m {
                    let c = i + m * (j + m * k);
                    let neighbours = f64::from(
                        u8::from(i > 0)
                            + u8::from(i + 1 < m)
                            + u8::from(j > 0)
                            + u8::from(j + 1 < m)
                            + u8::from(k > 0)
                            + u8::from(k + 1 < m),
                    );
                    out_u[c] = -DU * scale * neighbours + 1.0 - u[c] * u[c];
                    out_v[c] = -DV * scale * neighbours - 0.064;
                }
            }
        }
        Ok(())
    });
    Ok(Family {
        id: format!("fhn3d-{m}"),
        problem: jvp_only(format!("fhn3d-{m}"), n, rhs, jvp)?,
        y0,
        span: (0.0, 40.0),
        atol_over_rtol: 1.0,
        diagonal,
    })
}

/// CUSP after Hairer and Wanner (Solving ODEs II, section IV.10) with
/// `cells` cells on a periodic ring, state interleaved `(y_i, a_i, b_i)`:
///
/// ```text
/// y_i' = -(y_i^3 + a_i y_i + b_i)/eps + Dc (y_{i-1} - 2 y_i + y_{i+1})
/// a_i' = b_i + 0.07 v_i + Dc (a_{i-1} - 2 a_i + a_{i+1})
/// b_i' = (1 - a_i^2) b_i - a_i - 0.4 y_i + 0.035 v_i + Dc (b_{i-1} - 2 b_i + b_{i+1})
/// v_i = u_i/(u_i + 0.1),  u_i = (y_i - 0.7)(y_i - 1.3)
/// ```
///
/// `eps = 1e-4`, `Dc = cells^2/144`; start `y_i = 0`,
/// `a_i = -2 cos(2 pi i/cells)`, `b_i = 2 sin(2 pi i/cells)`, `i = 1..cells`.
pub fn cusp1d(cells: usize) -> CoreResult<Family> {
    if cells < 3 {
        return Err(CoreError::InvalidInput("invalid CUSP size".into()));
    }
    const INV_EPS: f64 = 1.0e4;
    let n = 3 * cells;
    let dc = (cells * cells) as f64 / 144.0;
    // `Dc (w_left - 2 w + w_right)` for species `s` of cell `i` on the ring.
    let laplace = move |w: &[f64], i: usize, s: usize| {
        let left = if i == 0 { cells - 1 } else { i - 1 };
        let right = if i + 1 == cells { 0 } else { i + 1 };
        dc * (w[3 * left + s] - 2.0 * w[3 * i + s] + w[3 * right + s])
    };
    let rhs = move |_t: f64, w: &[f64], out: &mut [f64]| {
        for i in 0..cells {
            let (y, a, b) = (w[3 * i], w[3 * i + 1], w[3 * i + 2]);
            let u = (y - 0.7) * (y - 1.3);
            let v = u / (u + 0.1);
            out[3 * i] = -(y * y * y + a * y + b) * INV_EPS + laplace(w, i, 0);
            out[3 * i + 1] = b + 0.07 * v + laplace(w, i, 1);
            out[3 * i + 2] = (1.0 - a * a) * b - a - 0.4 * y + 0.035 * v + laplace(w, i, 2);
        }
        Ok(())
    };
    let jvp = move |_t: f64, w: &[f64], d: &[f64], out: &mut [f64]| {
        for i in 0..cells {
            let (y, a, b) = (w[3 * i], w[3 * i + 1], w[3 * i + 2]);
            let (dy, da, db) = (d[3 * i], d[3 * i + 1], d[3 * i + 2]);
            let u = (y - 0.7) * (y - 1.3);
            // dv/dy = (du/dy) 0.1/(u + 0.1)^2.
            let dv = (2.0 * y - 2.0) * 0.1 / ((u + 0.1) * (u + 0.1)) * dy;
            out[3 * i] = -((3.0 * y * y + a) * dy + y * da + db) * INV_EPS + laplace(d, i, 0);
            out[3 * i + 1] = db + 0.07 * dv + laplace(d, i, 1);
            out[3 * i + 2] = -2.0 * a * b * da + (1.0 - a * a) * db - da - 0.4 * dy
                + 0.035 * dv
                + laplace(d, i, 2);
        }
        Ok(())
    };
    let diagonal: JacobianDiagonalFn = Arc::new(move |_t: f64, w: &[f64], out: &mut [f64]| {
        for i in 0..cells {
            let (y, a) = (w[3 * i], w[3 * i + 1]);
            out[3 * i] = -(3.0 * y * y + a) * INV_EPS - 2.0 * dc;
            out[3 * i + 1] = -2.0 * dc;
            out[3 * i + 2] = (1.0 - a * a) - 2.0 * dc;
        }
        Ok(())
    });
    let y0 = (0..cells)
        .flat_map(|i| {
            let angle = 2.0 * PI * (i as f64 + 1.0) / cells as f64;
            [0.0, -2.0 * angle.cos(), 2.0 * angle.sin()]
        })
        .collect();
    Ok(Family {
        id: format!("cusp1d-{cells}"),
        problem: jvp_only(format!("cusp1d-{cells}"), n, rhs, jvp)?,
        y0,
        span: (0.0, 0.7),
        atol_over_rtol: 1.0,
        diagonal,
    })
}

/// 2D viscous Burgers system on the periodic unit square,
/// `u_t = -(u u_x + v u_y) + nu lap(u)`, `v_t = -(u v_x + v v_y) + nu lap(v)`
/// with viscosity `nu`: second-order central first derivatives, 5-point
/// Laplacian, `u` block then `v` block, index `i + m j`, cell centres
/// `((i + 1/2)/m, (j + 1/2)/m)`; start `u = sin(2 pi x) cos(2 pi y) + u_mean`,
/// `v = -cos(2 pi x) sin(2 pi y) + 0.25 sin(2 pi (x + y))`. The declared
/// viscosity and mean are [`BURGERS_NU`] and [`BURGERS_U_MEAN`]: the mean
/// flow carries the pattern across the box four times per unit time.
pub fn burgers2d(m: usize, nu: f64, u_mean: f64) -> CoreResult<Family> {
    if m < 3 || !(nu.is_finite() && nu > 0.0) {
        return Err(CoreError::InvalidInput("invalid Burgers problem".into()));
    }
    let cells = m * m;
    let n = 2 * cells;
    let half_inv_dx = 0.5 * m as f64;
    let diffusion = nu * (m * m) as f64;
    // (d/dx, d/dy, nu lap) of the periodic field `w` at cell `(i, j)`.
    let stencil = move |w: &[f64], i: usize, j: usize| {
        let ip = if i + 1 == m { 0 } else { i + 1 };
        let im = if i == 0 { m - 1 } else { i - 1 };
        let jp = if j + 1 == m { 0 } else { j + 1 };
        let jm = if j == 0 { m - 1 } else { j - 1 };
        let (east, west, north, south) =
            (w[ip + m * j], w[im + m * j], w[i + m * jp], w[i + m * jm]);
        (
            half_inv_dx * (east - west),
            half_inv_dx * (north - south),
            diffusion * (east + west + north + south - 4.0 * w[i + m * j]),
        )
    };
    let rhs = move |_t: f64, y: &[f64], out: &mut [f64]| {
        let (u, v) = y.split_at(cells);
        let (out_u, out_v) = out.split_at_mut(cells);
        for j in 0..m {
            for i in 0..m {
                let c = i + m * j;
                let (ux, uy, ulap) = stencil(u, i, j);
                let (vx, vy, vlap) = stencil(v, i, j);
                out_u[c] = -(u[c] * ux + v[c] * uy) + ulap;
                out_v[c] = -(u[c] * vx + v[c] * vy) + vlap;
            }
        }
        Ok(())
    };
    let jvp = move |_t: f64, y: &[f64], d: &[f64], out: &mut [f64]| {
        let (u, v) = y.split_at(cells);
        let (du, dv) = d.split_at(cells);
        let (out_u, out_v) = out.split_at_mut(cells);
        for j in 0..m {
            for i in 0..m {
                let c = i + m * j;
                let (ux, uy, _) = stencil(u, i, j);
                let (vx, vy, _) = stencil(v, i, j);
                let (dux, duy, dulap) = stencil(du, i, j);
                let (dvx, dvy, dvlap) = stencil(dv, i, j);
                out_u[c] = -(du[c] * ux + u[c] * dux + dv[c] * uy + v[c] * duy) + dulap;
                out_v[c] = -(du[c] * vx + u[c] * dvx + dv[c] * vy + v[c] * dvy) + dvlap;
            }
        }
        Ok(())
    };
    let diagonal: JacobianDiagonalFn = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        let (u, v) = y.split_at(cells);
        let (out_u, out_v) = out.split_at_mut(cells);
        for j in 0..m {
            for i in 0..m {
                let c = i + m * j;
                let (ux, _, _) = stencil(u, i, j);
                let (_, vy, _) = stencil(v, i, j);
                out_u[c] = -ux - 4.0 * diffusion;
                out_v[c] = -vy - 4.0 * diffusion;
            }
        }
        Ok(())
    });
    let dx = 1.0 / m as f64;
    let mut y0 = vec![0.0; n];
    for j in 0..m {
        for i in 0..m {
            let (x, y) = ((i as f64 + 0.5) * dx, (j as f64 + 0.5) * dx);
            y0[i + m * j] = (2.0 * PI * x).sin() * (2.0 * PI * y).cos() + u_mean;
            y0[cells + i + m * j] =
                -(2.0 * PI * x).cos() * (2.0 * PI * y).sin() + 0.25 * (2.0 * PI * (x + y)).sin();
        }
    }
    Ok(Family {
        id: format!("burgers2d-{m}"),
        problem: jvp_only(format!("burgers2d-{m}"), n, rhs, jvp)?,
        y0,
        span: (0.0, 0.5),
        atol_over_rtol: 1.0,
        diagonal,
    })
}

/// The viscosity and the mean of `u` at the start of the declared Burgers
/// family.
pub const BURGERS_NU: f64 = 0.05;
pub const BURGERS_U_MEAN: f64 = 4.0;

/// The time spans of the declared families (`PREDECLARED.json`, amendment
/// A1).
pub const SPAN_FHN3D: (f64, f64) = (0.0, 40.0);
pub const SPAN_CUSP1D: (f64, f64) = (0.0, 0.7);
pub const SPAN_BURGERS2D: (f64, f64) = (0.0, 0.5);
pub const SPAN_BRUSS1D: (f64, f64) = (0.0, 10.0);

/// The declared family ids of the node.
pub const DECLARED: [&str; 4] = ["fhn3d", "cusp1d", "burgers2d", "bruss1d"];

/// The development instances (reduced sizes of the declared families).
pub const DEVELOPMENT: [&str; 4] = ["dev_fhn3d", "dev_cusp1d", "dev_burgers2d", "dev_bruss1d"];

/// The instances of `PREDECLARED.json` by id, the reduced development
/// instances (`dev_*`, `size_*`) and the exploratory stiff Brusselator.
pub fn declared(id: &str) -> CoreResult<Family> {
    let mut family = match id {
        "fhn3d" => fhn3d(10)?,
        "cusp1d" => cusp1d(400)?,
        "burgers2d" => burgers2d(28, BURGERS_NU, BURGERS_U_MEAN)?,
        "bruss1d" => v06::bruss1d(700, 8.0)?,
        "dev_fhn3d" => fhn3d(5)?,
        "dev_cusp1d" => cusp1d(32)?,
        "dev_burgers2d" => burgers2d(8, BURGERS_NU, BURGERS_U_MEAN)?,
        "dev_bruss1d" => v06::bruss1d(48, 8.0)?,
        "size_fhn3d" => fhn3d(6)?,
        "size_cusp1d" => cusp1d(100)?,
        "size_burgers2d" => burgers2d(12, BURGERS_NU, BURGERS_U_MEAN)?,
        "size_bruss1d" => v06::bruss1d(96, 8.0)?,
        // Sizing only (amendment A1): a smaller ring.
        "size_cusp1d_50" => cusp1d(50)?,
        "bruss1d_stiff600" => v06::bruss1d(600, 1.0)?,
        _ => {
            return Err(CoreError::InvalidInput(format!("unknown V17 family {id}")));
        }
    };
    let base = id
        .strip_prefix("dev_")
        .or_else(|| id.strip_prefix("size_"))
        .unwrap_or(id);
    let base = base.strip_suffix("_50").unwrap_or(base);
    family.span = match base {
        "fhn3d" => SPAN_FHN3D,
        "cusp1d" => SPAN_CUSP1D,
        "burgers2d" => SPAN_BURGERS2D,
        "bruss1d" => SPAN_BRUSS1D,
        _ => family.span,
    };
    family.id = id.to_string();
    Ok(family)
}

/// The arms of `PREDECLARED.json`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arm {
    /// A: the protected matrix-free driver as routed in production.
    Protected,
    /// B0 (control): per-stage GMRES client with the shipped cadence.
    PerStageShipped,
    /// B: per-stage GMRES client, residual test after every column.
    PerStage,
    /// C: B with the step-tied linear tolerance.
    StepTied,
}

impl Arm {
    pub const ALL: [Self; 4] = [
        Self::Protected,
        Self::PerStageShipped,
        Self::PerStage,
        Self::StepTied,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Protected => "A",
            Self::PerStageShipped => "B0",
            Self::PerStage => "B",
            Self::StepTied => "C",
        }
    }

    pub fn parse(label: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|arm| arm.label() == label)
    }
}

/// One arm's configuration; [`ArmOptions::new`] gives the declared one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArmOptions {
    pub arm: Arm,
    /// Check every stage solve against the original residual.
    pub audit: bool,
    /// Exploratory: Arnoldi columns per restart pass (`None`: as shipped,
    /// 40, the declared value).
    pub restart: Option<usize>,
    /// Exploratory: right Jacobi (client arms only).
    pub jacobi: bool,
    /// Arm C: the fraction of the local tolerance (declared:
    /// `StepTiedTolerance::KAPPA`).
    pub kappa: f64,
}

impl ArmOptions {
    pub fn new(arm: Arm) -> Self {
        Self {
            arm,
            audit: false,
            restart: None,
            jacobi: false,
            kappa: StepTiedTolerance::KAPPA,
        }
    }

    /// The per-stage GMRES options of a client arm.
    pub fn gmres(&self) -> PerStageGmresOptions {
        let base = if self.arm == Arm::PerStageShipped {
            PerStageGmresOptions::shipped_cadence()
        } else {
            PerStageGmresOptions::baseline()
        };
        PerStageGmresOptions {
            restart: self.restart.unwrap_or(base.restart),
            ..base
        }
    }

    /// The step-tied tolerance of arm C for the step tolerances of `config`.
    pub fn policy(&self, config: &AdaptiveStepConfig) -> CoreResult<Option<StepTiedTolerance>> {
        if self.arm == Arm::StepTied {
            StepTiedTolerance::new(self.kappa, config.rtol, config.atol).map(Some)
        } else {
            Ok(None)
        }
    }
}

/// The outcome of the residual check of one run, whichever arm.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AuditSummary {
    /// Stage solves checked.
    pub checks: u64,
    /// Checks whose residual exceeded the threshold, was not finite or
    /// could not be formed.
    pub failures: u64,
    /// Largest residual/threshold over the checks with a positive
    /// threshold.
    pub max_ratio: f64,
    pub zero_threshold_failures: u64,
    /// 1-based index of the first failed check (0: none).
    pub first_failure: u64,
    /// JVPs and right-hand sides of the check itself; never part of the
    /// arm's counters.
    pub jvp_calls: u64,
    pub rhs_calls: u64,
}

/// Arm A, audit-on run: the Arnoldi columns of every completed stage solve
/// (the driver's solve log).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SolveLogSummary {
    pub solves: u64,
    pub iterations: u64,
    pub max_iterations: u64,
    /// `histogram[p]`: solves that used `p` restart passes of `restart`
    /// columns (rounded up); the last slot collects the rest.
    pub pass_histogram: [u64; 8],
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
    /// With the audit: its outcome.
    pub audit: Option<AuditSummary>,
    /// Arm A with the audit: the solve log.
    pub solve_log: Option<SolveLogSummary>,
}

/// The integrator configuration of `PREDECLARED.json`.
pub fn adaptive(family: &Family, rtol: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: rtol * family.atol_over_rtol,
        rtol,
        initial_step: 1.0e-3,
        min_step: 1.0e-14,
        max_step: family.span.1 - family.span.0,
        max_attempts: 1_000_000,
        ..AdaptiveStepConfig::default()
    }
}

/// The independent residual check of arm A.
///
/// The protected driver does not expose the vector it hands to GMRES, so
/// the check rebuilds every stage system from the stage equations of the
/// driver's U form (`rodas5p_matrix_free_fast.rs`, `attempt_inner`): with
/// the stages `U_j` read from the workspace after the attempt,
///
/// ```text
/// Y_i   = y + sum_j a_ij U_j
/// rhs_i = h gamma f(t + c_i h, Y_i) + sum_j (gamma C_ij) U_j
/// r_i   = rhs_i - (U_i - h gamma J(t, y) U_i)
/// ```
///
/// and tests `||r_i||_2 <= max(gamma atol_lin, rtol_lin ||rhs_i||_2)`. The
/// right-hand sides and JVPs are evaluated here with the audit's own
/// counters. Autonomous problems only.
pub struct ProtectedAudit {
    gamma: f64,
    a: DenseMatrix,
    c_matrix: DenseMatrix,
    c: Vec<f64>,
    rtol_lin: f64,
    /// `gamma atol_lin`, as the driver forms it.
    atol_scaled: f64,
    state: Vec<f64>,
    rhs: Vec<f64>,
    jv: Vec<f64>,
    residual: Vec<f64>,
    counters: WorkCounters,
    /// Entries of the driver's solve log seen so far.
    logged: usize,
    pub summary: AuditSummary,
}

impl ProtectedAudit {
    /// The check for a driver with linear tolerances `rtol_lin`, `atol_lin`.
    pub fn new(n: usize, rtol_lin: f64, atol_lin: f64) -> CoreResult<Self> {
        let coeffs = rodas5p_coefficients()?;
        let atol_scaled = if atol_lin == 0.0 {
            0.0
        } else {
            raw_absolute_residual_budget(atol_lin, coeffs.gamma)?
        };
        Ok(Self {
            gamma: coeffs.gamma,
            a: coeffs.a.clone(),
            c_matrix: coeffs.c_matrix.clone(),
            c: coeffs.c.clone(),
            rtol_lin,
            atol_scaled,
            state: vec![0.0; n],
            rhs: vec![0.0; n],
            jv: vec![0.0; n],
            residual: vec![0.0; n],
            counters: WorkCounters::default(),
            logged: 0,
            summary: AuditSummary::default(),
        })
    }

    /// `(||r_i||, threshold)` of stage `i` of the attempt `(t, y, h)`.
    #[allow(clippy::too_many_arguments)]
    fn stage_residual(
        &mut self,
        problem: &OdeProblem,
        jacobian: &dyn LinearOperator,
        t: f64,
        y: &[f64],
        h: f64,
        i: usize,
        work: &Rodas5pMfFastWorkspace,
    ) -> CoreResult<(f64, f64)> {
        let hg = h * self.gamma;
        if i == 0 {
            problem.eval_rhs_into(t, y, &mut self.rhs, &mut self.counters)?;
        } else {
            self.state.copy_from_slice(y);
            for j in 0..i {
                let aij = self.a[(i, j)];
                if aij != 0.0 {
                    for (x, v) in self.state.iter_mut().zip(work.stage(j)) {
                        *x += aij * v;
                    }
                }
            }
            problem.eval_rhs_into(
                t + self.c[i] * h,
                &self.state,
                &mut self.rhs,
                &mut self.counters,
            )?;
        }
        for x in self.rhs.iter_mut() {
            *x *= hg;
        }
        for j in 0..i {
            let cij = self.c_matrix[(i, j)];
            if cij != 0.0 {
                let weight = self.gamma * cij;
                for (x, v) in self.rhs.iter_mut().zip(work.stage(j)) {
                    *x += weight * v;
                }
            }
        }
        let u = work.stage(i);
        apply_counted(
            jacobian,
            u,
            &mut self.jv,
            &mut self.counters,
            ApplyCategory::Diagnostic,
        )?;
        for (((r, b), x), jx) in self.residual.iter_mut().zip(&self.rhs).zip(u).zip(&self.jv) {
            *r = b - (x - hg * jx);
        }
        let threshold = self.atol_scaled.max(self.rtol_lin * safe_l2(&self.rhs));
        Ok((safe_l2(&self.residual), threshold))
    }

    /// The attempt observer: checks every stage solve the driver completed
    /// in the attempt (the new entries of its solve log).
    pub fn observe(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        h: f64,
        work: &Rodas5pMfFastWorkspace,
    ) {
        let total = work.solve_log().map_or(0, <[_]>::len);
        let completed = total.saturating_sub(self.logged);
        self.logged = total;
        if completed == 0 {
            return;
        }
        let jacobian = problem.linearize_matrix_free(t, y);
        for i in 0..completed {
            self.summary.checks += 1;
            let verdict = match &jacobian {
                Ok(jacobian) if problem.autonomous && i < self.c.len() => {
                    self.stage_residual(problem, jacobian.as_ref(), t, y, h, i, work)
                }
                _ => Err(CoreError::InvalidInput(
                    "the stage system could not be rebuilt".into(),
                )),
            };
            let passed = match verdict {
                Ok((residual_norm, threshold)) => {
                    if threshold > 0.0 {
                        let ratio = residual_norm / threshold;
                        if ratio.is_finite() {
                            self.summary.max_ratio = self.summary.max_ratio.max(ratio);
                        }
                    } else if residual_norm != 0.0 {
                        self.summary.zero_threshold_failures += 1;
                    }
                    residual_norm <= threshold
                }
                Err(_) => false,
            };
            if !passed {
                self.summary.failures += 1;
                if self.summary.first_failure == 0 {
                    self.summary.first_failure = self.summary.checks;
                }
            }
        }
        self.summary.jvp_calls = self.counters.jvp_calls;
        self.summary.rhs_calls = self.counters.rhs_calls;
    }
}

/// The client of a client arm with the handles of its statistics and audit.
pub fn build_client(
    family: &Family,
    config: &AdaptiveStepConfig,
    options: &ArmOptions,
) -> CoreResult<(
    Box<dyn StageLinearSystem>,
    KrylovStatsHandle,
    Option<ResidualAuditHandle>,
)> {
    if options.arm == Arm::Protected {
        return Err(CoreError::InvalidInput(
            "arm A is the protected driver, not a client".into(),
        ));
    }
    let n = family.problem.dimension;
    let rule = StageStoppingRule::protected_production()?;
    let preconditioner = if options.jacobi {
        KrylovPreconditioner::RightJacobi(Arc::clone(&family.diagonal))
    } else {
        KrylovPreconditioner::None
    };
    let policy = options.policy(config)?;
    let mut client = KrylovStageClient::per_stage(n, rule, options.gmres(), preconditioner)?;
    if let Some(policy) = policy {
        client = client.with_step_tied_tolerance(policy);
    }
    let stats = client.stats_handle();
    if options.audit {
        let mut audited = ResidualAuditClient::new(client, rule);
        if let Some(policy) = policy {
            audited = audited.with_step_tied_tolerance(policy);
        }
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
        if options.jacobi {
            return Err(CoreError::InvalidInput(
                "arm A has no preconditioner".into(),
            ));
        }
        let mut linear = RoutingOptions::default().matrix_free;
        if let Some(restart) = options.restart {
            linear.restart = restart;
        }
        let (run, audit, solve_log) = if options.audit {
            // What the shipped entry does (`integrate_mf_fast_inner`), plus
            // the solve log and a non-empty observer.
            let mut work = Rodas5pMfFastWorkspace::new(&family.problem, &linear)?;
            work.set_gmres_into(true);
            work.set_solve_log(true);
            let mut audit =
                ProtectedAudit::new(family.problem.dimension, linear.rtol, linear.atol)?;
            let mut observer =
                |t: f64, y: &[f64], h: f64, _error: Option<f64>, work: &Rodas5pMfFastWorkspace| {
                    audit.observe(&family.problem, t, y, h, work);
                };
            let (run, _statistics, _charges, log) = work.integrate_observed_with_observer(
                &family.problem,
                family.span,
                &family.y0,
                &config,
                &output,
                &mut observer,
            )?;
            let mut summary = SolveLogSummary::default();
            for entry in log.as_deref().unwrap_or_default() {
                summary.solves += 1;
                summary.iterations += entry.iterations;
                summary.max_iterations = summary.max_iterations.max(entry.iterations);
                let passes = entry.iterations.div_ceil(linear.restart.max(1) as u64) as usize;
                summary.pass_histogram[passes.min(7)] += 1;
            }
            (run, Some(audit.summary), Some(summary))
        } else {
            let run = integrate_rodas5p_mf_fast_observed_gmres_into(
                &family.problem,
                family.span,
                &family.y0,
                &linear,
                &config,
                &output,
            )?;
            (run, None, None)
        };
        return Ok(ArmRun {
            success: run.observed.success,
            attempts: run.attempts,
            accepted: run.accepted_steps,
            rejected: run.rejected_steps,
            counters: run.observed.counters,
            t_end: run.observed.t.last().copied().unwrap_or(family.span.0),
            y_end: run.observed.y.last().cloned().unwrap_or_default(),
            stats: None,
            audit,
            solve_log,
        });
    }
    let (client, stats, audit) = build_client(family, &config, options)?;
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
        audit: audit.map(|handle| {
            let report = handle.snapshot();
            AuditSummary {
                checks: report.checks,
                failures: report.failures,
                max_ratio: report.max_ratio,
                zero_threshold_failures: report.zero_threshold_failures,
                first_failure: report.first_failure,
                jvp_calls: report.jvp_calls,
                rhs_calls: 0,
            }
        }),
        solve_log: None,
    })
}
