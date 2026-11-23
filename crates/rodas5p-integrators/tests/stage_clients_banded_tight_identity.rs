//! The opt-in banded stage client equals the protected banded driver bit for bit (loop L1 node
//! V21, stage b). Cells: `research/production_loop_20261116/banded_solve/PREDECLARED_V21B.json`.
//!
//! Every test runs [`integrate_rodas5p_fast_banded_observed_with_atol`] (the protected banded
//! driver with its default kernel) and [`integrate_rodas5p_fast_client_observed_with_atol`]
//! with [`BandedTightStageLu`] on the same problem and compares every returned output time and
//! state bit for bit, the attempt, accepted, rejected and reuse counts, and the whole
//! [`rodas5p_core::WorkCounters`] value.

use std::f64::consts::PI;
use std::sync::Arc;

use rodas5p_core::CoreResult;
use rodas5p_integrators::stage_clients::BandedTightStageLu;
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, OdeProblem, OutputSchedule,
    integrate_rodas5p_fast_banded_observed_with_atol,
    integrate_rodas5p_fast_client_observed_with_atol,
};

type Rhs = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
type Jvp = Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
type BandFill = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

struct Case {
    name: String,
    problem: OdeProblem,
    band: BandedJacobian,
    y0: Vec<f64>,
    span: (f64, f64),
}

#[allow(clippy::too_many_arguments)]
fn case(
    name: String,
    n: usize,
    rhs: Rhs,
    jvp: Jvp,
    band: usize,
    fill: BandFill,
    y0: Vec<f64>,
    span: (f64, f64),
) -> Case {
    let problem = OdeProblem::new(
        name.clone(),
        n,
        rhs,
        None,
        None,
        Some(jvp),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    Case {
        name,
        problem,
        band: BandedJacobian {
            lower: band,
            upper: band,
            fill,
        },
        y0,
        span,
    }
}

/// The one-dimensional Brusselator of loop node V04: species interleaved `(u_i, v_i)`,
/// Dirichlet `u = 1`, `v = 3`, `c = (cells + 1)^2 / 50`. Band: lower = upper = 2.
fn brusselator(cells: usize) -> Case {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    let n = 2 * cells;
    let rhs: Rhs = Arc::new(move |_t, y, out| {
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
    let fill: BandFill = Arc::new(move |_t, y, band| {
        // Row r, column q at r * 5 + (q + 2 - r).
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (da, db) = (2 * i * 5 + 2, (2 * i + 1) * 5 + 2);
            band[da] = 2.0 * u * v - 4.0 - 2.0 * c;
            band[da + 1] = u * u;
            band[db - 1] = 3.0 - 2.0 * u * v;
            band[db] = -u * u - 2.0 * c;
            if i > 0 {
                band[da - 2] = c;
                band[db - 2] = c;
            }
            if i + 1 < cells {
                band[da + 2] = c;
                band[db + 2] = c;
            }
        }
        Ok(())
    });
    let jvp: Jvp = Arc::new(move |_t, y, w, out| {
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (a, b) = (2 * i, 2 * i + 1);
            let (wl_u, wl_v) = if i == 0 {
                (0.0, 0.0)
            } else {
                (w[a - 2], w[b - 2])
            };
            let (wr_u, wr_v) = if i + 1 == cells {
                (0.0, 0.0)
            } else {
                (w[a + 2], w[b + 2])
            };
            out[a] = (2.0 * u * v - 4.0 - 2.0 * c) * w[a] + u * u * w[b] + c * (wl_u + wr_u);
            out[b] = (3.0 - 2.0 * u * v) * w[a] + (-u * u - 2.0 * c) * w[b] + c * (wl_v + wr_v);
        }
        Ok(())
    });
    let y0 = (0..cells)
        .flat_map(|i| {
            let x = (i as f64 + 1.0) / (cells as f64 + 1.0);
            [1.0 + (2.0 * PI * x).sin(), 3.0]
        })
        .collect();
    case(
        format!("brusselator-1d-{cells}"),
        n,
        rhs,
        jvp,
        2,
        fill,
        y0,
        (0.0, 10.0),
    )
}

/// FitzHugh-Nagumo on a line, species interleaved, zero-flux ends (the second two-species
/// family of PREDECLARED_V21B.json). Band: lower = upper = 2.
fn fitzhugh_nagumo(cells: usize) -> Case {
    let scale = (cells as f64 + 1.0).powi(2);
    let (du, dv) = (1.0e-3 * scale, 1.0e-4 * scale);
    let n = 2 * cells;
    let rhs: Rhs = Arc::new(move |_t, y, out| {
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (ul, vl) = if i == 0 {
                (u, v)
            } else {
                (y[2 * i - 2], y[2 * i - 1])
            };
            let (ur, vr) = if i + 1 == cells {
                (u, v)
            } else {
                (y[2 * i + 2], y[2 * i + 3])
            };
            out[2 * i] = du * (ul - 2.0 * u + ur) + u - u * u * u / 3.0 - v;
            out[2 * i + 1] = dv * (vl - 2.0 * v + vr) + 0.08 * (u + 0.7 - 0.8 * v);
        }
        Ok(())
    });
    let degree = move |i: usize| f64::from(u8::from(i > 0) + u8::from(i + 1 < cells));
    let fill: BandFill = Arc::new(move |_t, y, band| {
        for i in 0..cells {
            let u = y[2 * i];
            let (da, db) = (2 * i * 5 + 2, (2 * i + 1) * 5 + 2);
            band[da] = -du * degree(i) + 1.0 - u * u;
            band[da + 1] = -1.0;
            band[db - 1] = 0.08;
            band[db] = -dv * degree(i) - 0.064;
            if i > 0 {
                band[da - 2] = du;
                band[db - 2] = dv;
            }
            if i + 1 < cells {
                band[da + 2] = du;
                band[db + 2] = dv;
            }
        }
        Ok(())
    });
    let jvp: Jvp = Arc::new(move |_t, y, w, out| {
        for i in 0..cells {
            let u = y[2 * i];
            let (a, b) = (2 * i, 2 * i + 1);
            let (wl_u, wl_v) = if i == 0 {
                (0.0, 0.0)
            } else {
                (w[a - 2], w[b - 2])
            };
            let (wr_u, wr_v) = if i + 1 == cells {
                (0.0, 0.0)
            } else {
                (w[a + 2], w[b + 2])
            };
            out[a] = (-du * degree(i) + 1.0 - u * u) * w[a] - w[b] + du * (wl_u + wr_u);
            out[b] = 0.08 * w[a] + (-dv * degree(i) - 0.064) * w[b] + dv * (wl_v + wr_v);
        }
        Ok(())
    });
    let y0 = (0..cells)
        .flat_map(|i| {
            let x = (i as f64 + 1.0) / (cells as f64 + 1.0);
            [-1.2 + 2.4 * (-100.0 * (x - 0.3).powi(2)).exp(), -0.6]
        })
        .collect();
    case(
        format!("fitzhugh-nagumo-1d-{cells}"),
        n,
        rhs,
        jvp,
        2,
        fill,
        y0,
        (0.0, 5.0),
    )
}

/// Allen-Cahn on an `m x m` grid, zero flux, natural ordering (the wide-band problem of
/// PREDECLARED_V21B.json). Band: lower = upper = m, served by the slice loops of the client.
fn allen_cahn_2d(m: usize) -> Case {
    let s = 0.01 * (m * m) as f64;
    let n = m * m;
    let neighbours = move |k: usize| {
        let (ix, iy) = (k % m, k / m);
        [
            (ix > 0).then(|| k - 1),
            (ix + 1 < m).then(|| k + 1),
            (iy > 0).then(|| k - m),
            (iy + 1 < m).then(|| k + m),
        ]
    };
    let rhs: Rhs = Arc::new(move |_t, y, out| {
        for k in 0..n {
            let u = y[k];
            let mut lap = 0.0;
            for nb in neighbours(k).into_iter().flatten() {
                lap += y[nb] - u;
            }
            out[k] = s * lap + u - u * u * u;
        }
        Ok(())
    });
    let width = 2 * m + 1;
    let fill: BandFill = Arc::new(move |_t, y, band| {
        // Row r, column q at r * (2 m + 1) + (q + m - r).
        for k in 0..n {
            let u = y[k];
            let mut degree = 0.0;
            for nb in neighbours(k).into_iter().flatten() {
                band[k * width + (nb + m - k)] = s;
                degree += 1.0;
            }
            band[k * width + m] = -s * degree + 1.0 - 3.0 * u * u;
        }
        Ok(())
    });
    let jvp: Jvp = Arc::new(move |_t, y, w, out| {
        for k in 0..n {
            let u = y[k];
            let mut lap = 0.0;
            for nb in neighbours(k).into_iter().flatten() {
                lap += w[nb] - w[k];
            }
            out[k] = s * lap + (1.0 - 3.0 * u * u) * w[k];
        }
        Ok(())
    });
    let y0 = (0..n)
        .map(|k| {
            let (x, y) = (
                ((k % m) as f64 + 0.5) / m as f64,
                ((k / m) as f64 + 0.5) / m as f64,
            );
            0.5 * (2.0 * PI * x).sin() * (2.0 * PI * y).cos()
        })
        .collect();
    case(
        format!("allen-cahn-2d-{m}"),
        n,
        rhs,
        jvp,
        m,
        fill,
        y0,
        (0.0, 2.0),
    )
}

/// Runs both drivers on `case` at `rtol = atol` and asserts bit identity. Returns the number
/// of attempts.
fn assert_identical(case: &Case, rtol: f64, fixed_offsets: bool) -> usize {
    let n = case.problem.dimension;
    let adaptive = AdaptiveStepConfig {
        atol: rtol,
        rtol,
        initial_step: 1.0e-6,
        min_step: 1.0e-14,
        max_step: case.span.1 - case.span.0,
        max_attempts: 1_000_000,
        ..AdaptiveStepConfig::default()
    };
    let output = OutputSchedule::new(vec![case.span.0, case.span.1]).unwrap();
    let atol = vec![rtol; n];
    let label = format!("{} rtol {rtol:e}", case.name);

    let protected = integrate_rodas5p_fast_banded_observed_with_atol(
        &case.problem,
        &case.band,
        case.span,
        &case.y0,
        &adaptive,
        &output,
        &atol,
    )
    .unwrap();
    let client = BandedTightStageLu::new(n, &case.band).unwrap();
    assert_eq!(client.uses_fixed_offsets(), fixed_offsets, "{label}");
    let (result, partial, _client) = integrate_rodas5p_fast_client_observed_with_atol(
        &case.problem,
        Box::new(client),
        case.span,
        &case.y0,
        &adaptive,
        &output,
        &atol,
    )
    .unwrap();

    let reference = &protected.fast;
    assert!(
        reference.observed.success && protected.partial.is_none(),
        "{label}"
    );
    assert!(result.observed.success && partial.is_none(), "{label}");
    assert_eq!(
        reference.observed.t.len(),
        result.observed.t.len(),
        "{label}"
    );
    for (a, b) in reference.observed.t.iter().zip(&result.observed.t) {
        assert_eq!(a.to_bits(), b.to_bits(), "{label}: output time");
    }
    for (ya, yb) in reference.observed.y.iter().zip(&result.observed.y) {
        assert_eq!(ya.len(), yb.len(), "{label}");
        for (i, (a, b)) in ya.iter().zip(yb).enumerate() {
            assert_eq!(a.to_bits(), b.to_bits(), "{label}: state component {i}");
        }
    }
    assert_eq!(reference.attempts, result.attempts, "{label}: attempts");
    assert_eq!(
        reference.accepted_steps, result.accepted_steps,
        "{label}: accepted"
    );
    assert_eq!(
        reference.rejected_steps, result.rejected_steps,
        "{label}: rejected"
    );
    assert_eq!(
        reference.jacobian_reuses, result.jacobian_reuses,
        "{label}: reuses"
    );
    assert_eq!(
        reference.observed.counters, result.observed.counters,
        "{label}: work counters"
    );
    assert_eq!(
        reference.observed.internal_steps, result.observed.internal_steps,
        "{label}: internal steps"
    );
    println!(
        "V21B_IDENTITY case={} rtol={rtol:e} n={n} band={} fixed_offsets={fixed_offsets} attempts={} accepted={} rejected={} factorizations={} solves={} identical=1",
        case.name,
        case.band.lower,
        result.attempts,
        result.accepted_steps,
        result.rejected_steps,
        result.observed.counters.direct_factorizations,
        result.observed.counters.direct_solve_calls,
    );
    result.attempts
}

#[test]
fn brusselator_1000_and_10000_equal_the_protected_banded_driver() {
    for cells in [500, 5000] {
        let case = brusselator(cells);
        for rtol in [1.0e-4, 1.0e-6, 1.0e-8] {
            assert!(assert_identical(&case, rtol, true) > 10);
        }
    }
}

#[test]
#[ignore = "long: n = 100000; run with --include-ignored for the evidence log"]
fn brusselator_100000_equals_the_protected_banded_driver() {
    let case = brusselator(50_000);
    for rtol in [1.0e-4, 1.0e-6, 1.0e-8] {
        assert!(assert_identical(&case, rtol, true) > 10);
    }
}

#[test]
fn fitzhugh_nagumo_10000_equals_the_protected_banded_driver() {
    let case = fitzhugh_nagumo(5000);
    for rtol in [1.0e-4, 1.0e-6, 1.0e-8] {
        assert!(assert_identical(&case, rtol, true) > 10);
    }
}

#[test]
fn wide_band_allen_cahn_equals_the_protected_banded_driver() {
    let case = allen_cahn_2d(32);
    for rtol in [1.0e-4, 1.0e-6] {
        assert!(assert_identical(&case, rtol, false) > 10);
    }
}

#[test]
fn smallest_sizes_equal_the_protected_banded_driver() {
    // n = 4: the general path; n = 6: the smallest size of the fixed-offset path.
    assert!(assert_identical(&brusselator(2), 1.0e-6, false) > 10);
    assert!(assert_identical(&brusselator(3), 1.0e-6, true) > 10);
}

#[test]
fn a_singular_matrix_is_the_same_linear_solve_error() {
    use rodas5p_core::{CoreError, WorkCounters};
    use rodas5p_integrators::{BandedStageLu, StageLinearSystem};
    // J = I / (h gamma) on the band: W = 0, the first pivot is zero.
    let n = 12;
    let fill: BandFill = Arc::new(move |_t, _y, band| {
        for i in 0..n {
            band[i * 5 + 2] = 4.0;
        }
        Ok(())
    });
    let band = BandedJacobian {
        lower: 2,
        upper: 2,
        fill,
    };
    let problem = brusselator(6).problem;
    let y = vec![0.0; n];
    let mut messages = Vec::new();
    let mut clients: Vec<Box<dyn StageLinearSystem>> = vec![
        Box::new(BandedStageLu::new(n, &band).unwrap()),
        Box::new(BandedTightStageLu::new(n, &band).unwrap()),
    ];
    for client in &mut clients {
        let mut counters = WorkCounters::default();
        client.linearize(&problem, 0.0, &y, &mut counters).unwrap();
        match client.factor(4.0, &mut counters) {
            Err(CoreError::LinearSolve(message)) => messages.push(message),
            other => panic!("expected a LinearSolve error, got {other:?}"),
        }
        assert_eq!(counters.direct_factorizations, 1);
    }
    assert_eq!(messages[0], messages[1]);
}
