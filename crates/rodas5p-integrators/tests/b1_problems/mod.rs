//! Problems and configuration shared by the B1 lagged-driver contract
//! tests; copied from `b1_lagged_contracts.rs` (itself copied from
//! `rodas5p-cli/src/stiff_benchmark.rs`, no new dependency).
#![allow(dead_code)]

use std::sync::Arc;

use rodas5p_core::{CoreResult, DenseMatrix};
use rodas5p_integrators::{AdaptiveStepConfig, OdeProblem, OutputSchedule, robertson_problem};

/// HIRES.
pub fn hires_problem() -> CoreResult<(OdeProblem, Vec<f64>)> {
    hires_problem_with_jvp(false)
}

/// HIRES, optionally with a JVP `J(y) v` (the explicit Jacobian times `v`).
pub fn hires_problem_with_jvp(with_jvp: bool) -> CoreResult<(OdeProblem, Vec<f64>)> {
    let rhs = Arc::new(|_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = -1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007;
        out[1] = 1.71 * y[0] - 8.75 * y[1];
        out[2] = -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4];
        out[3] = 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3];
        out[4] = -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6];
        out[5] = -280.0 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6];
        out[6] = 280.0 * y[5] * y[7] - 1.81 * y[6];
        out[7] = -280.0 * y[5] * y[7] + 1.81 * y[6];
        Ok(())
    });
    fn hires_fill(y: &[f64], j: &mut DenseMatrix) {
        j[(0, 0)] = -1.71;
        j[(0, 1)] = 0.43;
        j[(0, 2)] = 8.32;
        j[(1, 0)] = 1.71;
        j[(1, 1)] = -8.75;
        j[(2, 2)] = -10.03;
        j[(2, 3)] = 0.43;
        j[(2, 4)] = 0.035;
        j[(3, 1)] = 8.32;
        j[(3, 2)] = 1.71;
        j[(3, 3)] = -1.12;
        j[(4, 4)] = -1.745;
        j[(4, 5)] = 0.43;
        j[(4, 6)] = 0.43;
        j[(5, 3)] = 0.69;
        j[(5, 4)] = 1.71;
        j[(5, 5)] = -280.0 * y[7] - 0.43;
        j[(5, 6)] = 0.69;
        j[(5, 7)] = -280.0 * y[5];
        j[(6, 5)] = 280.0 * y[7];
        j[(6, 6)] = -1.81;
        j[(6, 7)] = 280.0 * y[5];
        j[(7, 5)] = -280.0 * y[7];
        j[(7, 6)] = 1.81;
        j[(7, 7)] = -280.0 * y[5];
    }
    let jacobian = Arc::new(|_t: f64, y: &[f64]| {
        let mut j = DenseMatrix::zeros(8, 8);
        hires_fill(y, &mut j);
        Ok(j)
    });
    let jacobian_into = Arc::new(|_t: f64, y: &[f64], j: &mut DenseMatrix| {
        hires_fill(y, j);
        Ok(())
    });
    Ok((
        OdeProblem::new(
            "hires",
            8,
            rhs,
            None,
            Some(jacobian),
            with_jvp.then(|| {
                Arc::new(|_t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
                    let mut j = DenseMatrix::zeros(8, 8);
                    hires_fill(y, &mut j);
                    out.copy_from_slice(&j.matvec(v)?);
                    Ok(())
                })
                    as Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>
            }),
            None,
            true,
            None,
            None,
        )?
        .with_jacobian_into(jacobian_into),
        vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057],
    ))
}

/// The 1D Brusselator, copied from
/// `rodas5p-cli/src/stiff_benchmark.rs::brusselator_problem`.
pub fn brusselator_problem(cells: usize) -> CoreResult<(OdeProblem, Vec<f64>)> {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
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
    });
    let n = 2 * cells;
    let fill = move |y: &[f64], j: &mut DenseMatrix| {
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (a, b) = (2 * i, 2 * i + 1);
            j[(a, a)] = 2.0 * u * v - 4.0 - 2.0 * c;
            j[(a, b)] = u * u;
            j[(b, a)] = 3.0 - 2.0 * u * v;
            j[(b, b)] = -u * u - 2.0 * c;
            if i > 0 {
                j[(a, a - 2)] = c;
                j[(b, b - 2)] = c;
            }
            if i + 1 < cells {
                j[(a, a + 2)] = c;
                j[(b, b + 2)] = c;
            }
        }
    };
    let jacobian = Arc::new(move |_t: f64, y: &[f64]| {
        let mut j = DenseMatrix::zeros(n, n);
        fill(y, &mut j);
        Ok(j)
    });
    let jacobian_into = Arc::new(move |_t: f64, y: &[f64], j: &mut DenseMatrix| {
        fill(y, j);
        Ok(())
    });
    let y0 = (0..cells)
        .flat_map(|i| {
            let x = (i as f64 + 1.0) / (cells as f64 + 1.0);
            [1.0 + (2.0 * std::f64::consts::PI * x).sin(), 3.0]
        })
        .collect();
    Ok((
        OdeProblem::new(
            format!("brusselator-1d-{cells}"),
            n,
            rhs,
            None,
            Some(jacobian),
            None,
            None,
            true,
            None,
            None,
        )?
        .with_jacobian_into(jacobian_into),
        y0,
    ))
}

/// Name, problem, y0, span and atol scale (atol = rtol * scale), as in
/// `stiff_benchmark.rs::default_problems`.
pub struct Case {
    pub id: &'static str,
    pub problem: OdeProblem,
    pub y0: Vec<f64>,
    pub span: (f64, f64),
    pub atol_scale: f64,
}

pub fn robertson_case() -> Case {
    let (problem, y0) = robertson_problem().unwrap();
    Case {
        id: "robertson",
        problem,
        y0,
        span: (0.0, 40.0),
        atol_scale: 1.0e-4,
    }
}

pub fn hires_case() -> Case {
    let (problem, y0) = hires_problem().unwrap();
    Case {
        id: "hires",
        problem,
        y0,
        span: (0.0, 321.8122),
        atol_scale: 1.0e-4,
    }
}

pub fn brusselator_case() -> Case {
    let (problem, y0) = brusselator_problem(50).unwrap();
    Case {
        id: "brusselator-1d-50 (n = 100)",
        problem,
        y0,
        span: (0.0, 10.0),
        atol_scale: 1.0,
    }
}

/// `stiff_benchmark.rs::adaptive_config`.
pub fn config(case: &Case, rtol: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: rtol * case.atol_scale,
        rtol,
        initial_step: 1.0e-6,
        min_step: 1.0e-14,
        max_step: case.span.1 - case.span.0,
        max_attempts: 1_000_000,
        ..AdaptiveStepConfig::default()
    }
}

pub fn schedule(span: (f64, f64), points: usize) -> OutputSchedule {
    let times = (0..=points)
        .map(|k| {
            if k == points {
                span.1
            } else {
                span.0 + (span.1 - span.0) * k as f64 / points as f64
            }
        })
        .collect();
    OutputSchedule::new(times).unwrap()
}

/// Endpoint error: max abs and max in tolerance units.
pub fn endpoint_errors(y: &[f64], reference: &[f64], adaptive: &AdaptiveStepConfig) -> (f64, f64) {
    let mut max_abs = 0.0_f64;
    let mut tol_units = 0.0_f64;
    for (a, r) in y.iter().zip(reference) {
        let e = (a - r).abs();
        max_abs = max_abs.max(e);
        tol_units = tol_units.max(e / (adaptive.atol + adaptive.rtol * r.abs()));
    }
    (max_abs, tol_units)
}
