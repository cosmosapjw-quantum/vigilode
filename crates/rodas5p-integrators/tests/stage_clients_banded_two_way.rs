//! The opt-in two-way banded stage client against the protected banded driver on whole
//! solves (research node SL-C7 of `research/stage_lattice_referee_20261124`).
//!
//! The client reorders the elimination, so a run is not bit-identical to the protected
//! driver. The tests assert what the reordering must preserve:
//!
//! - the serial and the joined execution of the client return the same bits and counters;
//! - against the protected driver the final state differs by at most `1e-2` of one tolerance
//!   unit, and the attempt counts differ by at most two percent (a rounding difference may
//!   move an accept decision);
//! - below the two-front threshold the client delegates to the bit-identical tight kernel.

use std::f64::consts::PI;
use std::sync::Arc;

use rodas5p_core::CoreResult;
use rodas5p_integrators::stage_clients::{BandedTwoWayStageLu, TwoWayExecution};
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, OdeProblem, OutputSchedule, Rodas5pFastResult,
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
    let name = format!("brusselator-1d-{cells}");
    Case {
        problem: OdeProblem::new(
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
        .unwrap(),
        name,
        band: BandedJacobian {
            lower: 2,
            upper: 2,
            fill,
        },
        y0,
        span: (0.0, 10.0),
    }
}

/// Allen-Cahn on an `m x m` grid, zero flux, natural ordering. Band: lower = upper = m.
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
    let name = format!("allen-cahn-2d-{m}");
    Case {
        problem: OdeProblem::new(
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
        .unwrap(),
        name,
        band: BandedJacobian {
            lower: m,
            upper: m,
            fill,
        },
        y0,
        span: (0.0, 2.0),
    }
}

fn config(case: &Case, rtol: f64) -> (AdaptiveStepConfig, OutputSchedule, Vec<f64>) {
    (
        AdaptiveStepConfig {
            atol: rtol,
            rtol,
            initial_step: 1.0e-6,
            min_step: 1.0e-14,
            max_step: case.span.1 - case.span.0,
            max_attempts: 1_000_000,
            ..AdaptiveStepConfig::default()
        },
        OutputSchedule::new(vec![case.span.0, case.span.1]).unwrap(),
        vec![rtol; case.problem.dimension],
    )
}

fn run_client(case: &Case, rtol: f64, execution: TwoWayExecution) -> (Rodas5pFastResult, bool) {
    let (adaptive, output, atol) = config(case, rtol);
    let client = BandedTwoWayStageLu::new(case.problem.dimension, &case.band, execution).unwrap();
    let two_fronts = client.uses_two_fronts();
    let (result, partial, _) = integrate_rodas5p_fast_client_observed_with_atol(
        &case.problem,
        Box::new(client),
        case.span,
        &case.y0,
        &adaptive,
        &output,
        &atol,
    )
    .unwrap();
    assert!(
        partial.is_none(),
        "{}: the client run stopped early",
        case.name
    );
    assert!(result.observed.success, "{}", case.name);
    (result, two_fronts)
}

fn assert_same_bits(a: &Rodas5pFastResult, b: &Rodas5pFastResult, label: &str) {
    assert_eq!(a.attempts, b.attempts, "{label}");
    assert_eq!(a.accepted_steps, b.accepted_steps, "{label}");
    assert_eq!(a.rejected_steps, b.rejected_steps, "{label}");
    assert_eq!(a.observed.counters, b.observed.counters, "{label}");
    assert_eq!(a.observed.t.len(), b.observed.t.len(), "{label}");
    for (ya, yb) in a.observed.y.iter().zip(&b.observed.y) {
        assert!(
            ya.iter().zip(yb).all(|(x, y)| x.to_bits() == y.to_bits()),
            "{label}: states differ"
        );
    }
}

#[test]
fn two_way_client_tracks_the_protected_banded_driver_within_a_fraction_of_the_tolerance() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    for (case, rtol) in [
        (brusselator(100), 1.0e-6),
        (brusselator(250), 1.0e-8),
        (allen_cahn_2d(12), 1.0e-6),
    ] {
        let label = format!("{} rtol {rtol:e}", case.name);
        let (adaptive, output, atol) = config(&case, rtol);
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
        assert!(protected.fast.observed.success, "{label}");

        let (serial, two_fronts) = run_client(&case, rtol, TwoWayExecution::Serial);
        assert!(two_fronts, "{label}");
        let (joined, _) = pool.install(|| run_client(&case, rtol, TwoWayExecution::Join));
        assert_same_bits(&serial, &joined, &label);

        // One factorisation per attempt and eight solves per completed attempt, charged
        // exactly as the reference client charges them.
        let counters = &serial.observed.counters;
        assert_eq!(
            counters.direct_factorizations as usize, serial.attempts,
            "{label}"
        );
        assert!(
            counters.direct_solve_calls as usize <= 8 * serial.attempts,
            "{label}"
        );

        let reference = &protected.fast;
        let drift = (serial.attempts as f64 - reference.attempts as f64).abs();
        assert!(
            drift <= 0.02 * reference.attempts as f64,
            "{label}: {} attempts against {}",
            serial.attempts,
            reference.attempts
        );
        let (yc, yp) = (
            serial.observed.y.last().unwrap(),
            reference.observed.y.last().unwrap(),
        );
        let scaled = yc
            .iter()
            .zip(yp)
            .map(|(c, p)| (c - p).abs() / (rtol + rtol * p.abs()))
            .fold(0.0f64, f64::max);
        assert!(scaled <= 1.0e-2, "{label}: {scaled:e} tolerance units");
    }
}

/// Diagnostic timing of the stage pattern (one factorisation and eight solves) and of whole
/// solves, printed as JSON lines. Not a gate: wall times on a shared host are diagnostic.
///
/// ```text
/// cargo test --locked -p rodas5p-integrators --profile measurement \
///     --test stage_clients_banded_two_way -- --ignored --nocapture measure
/// ```
#[test]
#[ignore = "timing measurement; run explicitly with --ignored --nocapture"]
fn measure_two_way_stage_pattern_and_whole_solves() {
    use std::time::Instant;

    use rodas5p_core::WorkCounters;
    use rodas5p_integrators::stage_clients::BandedTightStageLu;
    use rodas5p_integrators::{
        BandedKernel, BandedStageLu, StageLinearSystem,
        integrate_rodas5p_fast_banded_observed_with_atol_and_kernel,
    };

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    let minimum = |samples: &[f64]| samples.iter().copied().fold(f64::INFINITY, f64::min);
    let median = |samples: &[f64]| {
        let mut sorted = samples.to_vec();
        sorted.sort_by(f64::total_cmp);
        sorted[sorted.len() / 2]
    };

    let cases = [
        brusselator(1_000),
        brusselator(10_000),
        brusselator(100_000),
        allen_cahn_2d(32),
        allen_cahn_2d(64),
    ];
    for case in &cases {
        let n = case.problem.dimension;
        let patterns = (4_000_000 / (n * (case.band.lower + 1))).clamp(3, 400);
        let inv = 100.0;
        let rhs: Vec<f64> = (0..n)
            .map(|i| ((i * 37 % 101) as f64 - 50.0) / 50.0)
            .collect();
        let mut arms: Vec<(&str, Box<dyn StageLinearSystem>, bool)> = vec![
            (
                "protected-kernel-client",
                Box::new(BandedStageLu::new(n, &case.band).unwrap()),
                false,
            ),
            (
                "tight",
                Box::new(BandedTightStageLu::new(n, &case.band).unwrap()),
                false,
            ),
            (
                "two-way-serial",
                Box::new(BandedTwoWayStageLu::new(n, &case.band, TwoWayExecution::Serial).unwrap()),
                false,
            ),
            (
                "two-way-join-2",
                Box::new(BandedTwoWayStageLu::new(n, &case.band, TwoWayExecution::Join).unwrap()),
                true,
            ),
        ];
        let mut counters = WorkCounters::default();
        for (_, client, _) in &mut arms {
            client
                .linearize(&case.problem, 0.0, &case.y0, &mut counters)
                .unwrap();
        }
        let mut samples: Vec<Vec<f64>> = vec![Vec::new(); arms.len()];
        let mut checksum = vec![0.0f64; arms.len()];
        for round in 0..7 {
            for index in 0..arms.len() {
                // Alternate the arm order between rounds.
                let a = if round % 2 == 0 {
                    index
                } else {
                    arms.len() - 1 - index
                };
                let (_, client, pooled) = &mut arms[a];
                let mut work = rhs.clone();
                let mut body = || {
                    let start = Instant::now();
                    for _ in 0..patterns {
                        client.factor(inv, &mut counters).unwrap();
                        for _ in 0..8 {
                            work.copy_from_slice(&rhs);
                            client.solve_in_place(&mut work, &mut counters).unwrap();
                        }
                    }
                    start.elapsed().as_secs_f64() / patterns as f64
                };
                let seconds = if *pooled {
                    pool.install(&mut body)
                } else {
                    body()
                };
                samples[a].push(seconds);
                checksum[a] = work.iter().sum();
            }
        }
        let base = minimum(&samples[0]);
        for (a, (name, _, _)) in arms.iter().enumerate() {
            println!(
                "{{\"kind\":\"stage-pattern\",\"case\":\"{}\",\"n\":{n},\"band\":{},\"arm\":\"{name}\",\"patterns\":{patterns},\"min_us\":{:.3},\"median_us\":{:.3},\"min_ns_per_row\":{:.3},\"ratio_to_protected_min\":{:.4},\"checksum\":{:.17e}}}",
                case.name,
                case.band.lower,
                minimum(&samples[a]) * 1e6,
                median(&samples[a]) * 1e6,
                minimum(&samples[a]) * 1e9 / n as f64,
                minimum(&samples[a]) / base,
                checksum[a],
            );
        }
    }

    // Whole solves on the two smaller line problems and the small grid.
    for (case, rtol) in [
        (brusselator(1_000), 1.0e-6),
        (brusselator(10_000), 1.0e-6),
        (allen_cahn_2d(32), 1.0e-6),
    ] {
        let (adaptive, output, atol) = config(&case, rtol);
        let n = case.problem.dimension;
        let mut walls: Vec<Vec<f64>> = vec![Vec::new(); 4];
        let mut attempts = [0usize; 4];
        let mut finals: Vec<Vec<f64>> = vec![Vec::new(); 4];
        for round in 0..3 {
            for index in 0..4 {
                let a = if round % 2 == 0 { index } else { 3 - index };
                let start = Instant::now();
                let result = match a {
                    0 => {
                        // The kernel the facade selects: Slices from half width 6 on
                        // (`vigilode::BANDED_SLICES_MIN_HALF_WIDTH`), Indexed below.
                        let kernel = if case.band.lower >= 6 && case.band.upper >= 6 {
                            BandedKernel::Slices
                        } else {
                            BandedKernel::Indexed
                        };
                        integrate_rodas5p_fast_banded_observed_with_atol_and_kernel(
                            &case.problem,
                            &case.band,
                            case.span,
                            &case.y0,
                            &adaptive,
                            &output,
                            &atol,
                            kernel,
                        )
                        .unwrap()
                        .fast
                    }
                    1 => {
                        integrate_rodas5p_fast_client_observed_with_atol(
                            &case.problem,
                            Box::new(BandedTightStageLu::new(n, &case.band).unwrap()),
                            case.span,
                            &case.y0,
                            &adaptive,
                            &output,
                            &atol,
                        )
                        .unwrap()
                        .0
                    }
                    2 => run_client(&case, rtol, TwoWayExecution::Serial).0,
                    _ => pool.install(|| run_client(&case, rtol, TwoWayExecution::Join).0),
                };
                walls[a].push(start.elapsed().as_secs_f64());
                attempts[a] = result.attempts;
                finals[a] = result.observed.y.last().unwrap().clone();
            }
        }
        let base = minimum(&walls[0]);
        for (a, name) in [
            "protected-banded-driver-facade-kernel",
            "client-tight",
            "client-two-way-serial",
            "client-two-way-join-2",
        ]
        .iter()
        .enumerate()
        {
            let scaled = finals[a]
                .iter()
                .zip(&finals[0])
                .map(|(c, p)| (c - p).abs() / (rtol + rtol * p.abs()))
                .fold(0.0f64, f64::max);
            println!(
                "{{\"kind\":\"whole-solve\",\"case\":\"{}\",\"n\":{n},\"rtol\":{rtol:e},\"arm\":\"{name}\",\"attempts\":{},\"min_ms\":{:.3},\"median_ms\":{:.3},\"ratio_to_protected_min\":{:.4},\"difference_in_tolerance_units\":{scaled:.3e}}}",
                case.name,
                attempts[a],
                minimum(&walls[a]) * 1e3,
                median(&walls[a]) * 1e3,
                minimum(&walls[a]) / base,
            );
        }
    }
}

#[test]
fn below_the_two_front_threshold_the_client_is_the_tight_kernel_bit_for_bit() {
    let case = brusselator(4);
    assert!(case.problem.dimension < BandedTwoWayStageLu::min_two_way_dimension(2, 2));
    let rtol = 1.0e-6;
    let (adaptive, output, atol) = config(&case, rtol);
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
    let (client, two_fronts) = run_client(&case, rtol, TwoWayExecution::Serial);
    assert!(!two_fronts);
    assert_same_bits(&client, &protected.fast, &case.name);
}
