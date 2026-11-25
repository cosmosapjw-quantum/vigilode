//! Opt-in adaptive shifted spatial stage solve. The caller supplies the
//! original represented W and an untrusted separable predictor. We certify
//! the full original equation, not just the stage mass or spectral model.
use super::{BoxTransform, SeparableMode, SeparableStages};
use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, direct_solve,
    directed::{Interval, add_up, div_up, sub_down},
};
use rodas5p_integrators::retro_portfolio::{Portfolio, Route, Selection};

fn invalid(s: &str) -> CoreError {
    CoreError::InvalidInput(format!("spatial portfolio: {s}"))
}
/// Absolute infinity-norm error for the represented linear system W x=b.
/// A strict row dominance lower bound is sufficient; otherwise refuse,
/// since the spectral model alone does not prove the original inverse gain.
/// Both transforms and direct fallback share the same independent proof.
/// This is a stage primitive, not a new facade IVP driver.
pub fn adaptive_stage(
    portfolio: &mut Portfolio,
    stages: &SeparableStages,
    z: f64,
    w: &DenseMatrix,
    b: &[f64],
    tol: f64,
) -> CoreResult<Selection<Vec<f64>>> {
    rodas5p_integrators::affine_event_certificate::check_affine_event_arithmetic()?;
    let n = w.nrows();
    if n == 0
        || n > 512
        || w.ncols() != n
        || b.len() != n
        || stages.dimension() != n
        || !z.is_finite()
        || !matches!(stages.mode(), SeparableMode::Verified { .. })
        || w.as_slice().iter().chain(b).any(|v| !v.is_finite())
    {
        return Err(invalid("source, dimension, or verified-mode contract"));
    }
    let mut gap = f64::INFINITY;
    for i in 0..n {
        let mut row = w[(i, i)].abs();
        for j in 0..n {
            if j != i {
                row = sub_down(row, w[(i, j)].abs())?;
            }
        }
        gap = gap.min(row);
    }
    if gap <= 0. || !gap.is_finite() {
        return Err(invalid("original row dominance not certified"));
    }
    let routes = if n < 8 {
        [
            Route::SpatialDirect,
            Route::SpatialPaired,
            Route::SpatialComplex,
        ]
    } else {
        [
            Route::SpatialPaired,
            Route::SpatialComplex,
            Route::SpatialDirect,
        ]
    };
    portfolio.choose(
        "spatial-represented-stage",
        &[
            n as u64,
            z.to_bits(),
            gap.to_bits(),
            stages.linear_rate().to_bits(),
        ],
        &routes,
        tol,
        |r| {
            let x = match r {
                Route::SpatialDirect => direct_solve(w, b)?,
                Route::SpatialPaired | Route::SpatialComplex => {
                    let mut x = b.to_vec();
                    let s = stages
                        .clone()
                        .with_box_transform(if r == Route::SpatialPaired {
                            BoxTransform::PairedReal
                        } else {
                            BoxTransform::Complex
                        });
                    s.solve_stage(z, &mut x)
                        .map_err(|e| CoreError::LinearSolve(e.to_string()))?;
                    x
                }
                _ => return Err(invalid("spatial route")),
            };
            let mut residual = 0.0_f64;
            for (i, &bi) in b.iter().enumerate() {
                let r = w
                    .row(i)
                    .iter()
                    .zip(&x)
                    .try_fold(Interval::point(bi)?, |r, (&a, &v)| {
                        r.sub(Interval::point(a)?.scale(v)?)
                    })?;
                residual = residual.max(r.mag());
            }
            Ok((x, div_up(add_up(residual, 0.)?, gap)?))
        },
    )
}

/// Replay-safe endpoint IVP request. The verifier must certify the ORIGINAL
/// physical problem at the requested final time, not the local LTE or stage
/// residual. No automatic inference of replay safety from a callback type.
pub struct EndpointRequest<'a> {
    /// Original problem (not the predictor's surrogate).
    pub problem: &'a crate::Problem,
    /// Requested exact represented time interval.
    pub span: (f64, f64),
    /// Initial state shared by every attempt.
    pub initial: &'a [f64],
    /// Native solver options, including local tolerance proposals.
    pub options: &'a crate::Options,
    /// Verified spectral stage declaration.
    pub stages: &'a SeparableStages,
    /// Explicit authorization that callbacks can be replayed from the initial state.
    pub replay_safe: bool,
    /// Target absolute physical endpoint bound supplied to the verifier.
    pub absolute_tolerance: f64,
}
/// Actual protected RODAS5P versus both Verified separable stage clients.
/// Requires caller-owned endpoint verification and explicit replay permission.
/// No dense, event, mass, or opaque-callback certification is implied.
pub fn adaptive_endpoint(
    portfolio: &mut Portfolio,
    req: EndpointRequest<'_>,
    mut verify: impl FnMut(&crate::Solution) -> CoreResult<f64>,
) -> CoreResult<Selection<crate::Solution>> {
    let EndpointRequest {
        problem,
        span,
        initial,
        options,
        stages,
        replay_safe,
        absolute_tolerance,
    } = req;
    if !replay_safe
        || options.dense_output
        || !problem.events.is_empty()
        || problem.mass.is_some()
        || !matches!(stages.mode(), SeparableMode::Verified { .. })
        || problem.dimension() != stages.dimension()
        || initial.len() != problem.dimension()
        || !span.0.is_finite()
        || !span.1.is_finite()
        || span.1 <= span.0
    {
        return Err(invalid("endpoint/replay/verified contract"));
    }
    // Caller controls source identity. History is only a heuristic; every
    // trial has an independent endpoint check, even on a history hit.
    let routes = [
        Route::SpatialNative,
        Route::SpatialIvPPaired,
        Route::SpatialIvPComplex,
    ];
    portfolio.choose(
        "spatial-ivp-endpoint",
        &[
            problem.dimension() as u64,
            span.0.to_bits(),
            span.1.to_bits(),
            options.rtol.to_bits(),
            options.h0.to_bits(),
        ],
        &routes,
        absolute_tolerance,
        |r| {
            let solution = match r {
                Route::SpatialNative => crate::Rodas5p::solve(problem, span, initial, options),
                Route::SpatialIvPPaired | Route::SpatialIvPComplex => {
                    let s = stages
                        .clone()
                        .with_box_transform(if r == Route::SpatialIvPPaired {
                            BoxTransform::PairedReal
                        } else {
                            BoxTransform::Complex
                        });
                    crate::VigilODE::solve_separable(problem, span, initial, options, &s)
                }
                _ => return Err(invalid("endpoint route")),
            }
            .map_err(|e| CoreError::NonlinearSolve(e.to_string()))?;
            if !solution.is_success()
                || solution.t.as_slice() != [span.0, span.1]
                || solution.y.len() != 2
                || solution
                    .y
                    .iter()
                    .any(|y| y.len() != initial.len() || y.iter().any(|v| !v.is_finite()))
                || solution.y[0] != initial
            {
                return Err(invalid("native endpoint output incomplete"));
            }
            let bound = verify(&solution)?;
            Ok((solution, bound))
        },
    )
}
