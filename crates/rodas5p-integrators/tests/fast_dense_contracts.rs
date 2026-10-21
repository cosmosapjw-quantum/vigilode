//! Contracts of the fast dense and banded drivers with a continuous
//! extension and W-filtered interior control (wave-3 node `p0-fastdense`,
//! plan §5 T3, task brief tests (a), (b), (d), (e), (f)). Test (c) is the
//! lib unit test `rodas5p_fast_dense::tests::stage_rows_match_sequential_stages`.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use rodas5p_core::{CoreError, CoreResult, DenseMatrix};
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, FastInteriorControl, OdeProblem, OutputSchedule,
    RODAS5P_FAST_BANDED_DENSE_DRIVER_ID, RODAS5P_FAST_DENSE_DRIVER_ID, Rodas5pFastDenseResult,
    integrate_rodas5p_fast_banded_dense_observed,
    integrate_rodas5p_fast_banded_dense_observed_with_atol, integrate_rodas5p_fast_banded_observed,
    integrate_rodas5p_fast_dense_observed, integrate_rodas5p_fast_dense_observed_with_atol,
    integrate_rodas5p_fast_observed, prothero_robinson_problem, robertson_problem,
};

const RTOLS: [f64; 3] = [1.0e-4, 1.0e-6, 1.0e-8];

fn bits(values: &[f64]) -> Vec<u64> {
    values.iter().map(|v| v.to_bits()).collect()
}

fn config(rtol: f64, t1: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 1.0e-4 * rtol,
        rtol,
        initial_step: 1.0e-6,
        min_step: 1.0e-14,
        max_step: t1,
        max_attempts: 1_000_000,
        ..AdaptiveStepConfig::default()
    }
}

/// HIRES (Hairer-Wanner II), with an explicit Jacobian.
fn hires() -> (OdeProblem, Vec<f64>) {
    fn fill(y: &[f64], j: &mut DenseMatrix) {
        let entries = [
            (0, 0, -1.71),
            (0, 1, 0.43),
            (0, 2, 8.32),
            (1, 0, 1.71),
            (1, 1, -8.75),
            (2, 2, -10.03),
            (2, 3, 0.43),
            (2, 4, 0.035),
            (3, 1, 8.32),
            (3, 2, 1.71),
            (3, 3, -1.12),
            (4, 4, -1.745),
            (4, 5, 0.43),
            (4, 6, 0.43),
            (5, 3, 0.69),
            (5, 4, 1.71),
            (5, 5, -280.0 * y[7] - 0.43),
            (5, 6, 0.69),
            (5, 7, -280.0 * y[5]),
            (6, 5, 280.0 * y[7]),
            (6, 6, -1.81),
            (6, 7, 280.0 * y[5]),
            (7, 5, -280.0 * y[7]),
            (7, 6, 1.81),
            (7, 7, -280.0 * y[5]),
        ];
        for (a, b, v) in entries {
            j[(a, b)] = v;
        }
    }
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
    let jacobian = Arc::new(|_t: f64, y: &[f64]| {
        let mut j = DenseMatrix::zeros(8, 8);
        fill(y, &mut j);
        Ok(j)
    });
    (
        OdeProblem::new(
            "hires",
            8,
            rhs,
            None,
            Some(jacobian),
            None,
            None,
            true,
            None,
            None,
        )
        .unwrap(),
        vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057],
    )
}

/// The 1-D Brusselator with `cells` cells (n = 2 cells, interleaved u, v)
/// and its (2, 2) band.
fn brusselator(cells: usize) -> (OdeProblem, BandedJacobian, Vec<f64>) {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    let n = 2 * cells;
    let neighbours = move |y: &[f64], i: usize| {
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
        (ul, vl, ur, vr)
    };
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (ul, vl, ur, vr) = neighbours(y, i);
            out[2 * i] = 1.0 + u * u * v - 4.0 * u + c * (ul - 2.0 * u + ur);
            out[2 * i + 1] = 3.0 * u - u * u * v + c * (vl - 2.0 * v + vr);
        }
        Ok(())
    });
    let entries = move |y: &[f64], mut put: Box<dyn FnMut(usize, usize, f64) + '_>| {
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (a, b) = (2 * i, 2 * i + 1);
            put(a, a, 2.0 * u * v - 4.0 - 2.0 * c);
            put(a, b, u * u);
            put(b, a, 3.0 - 2.0 * u * v);
            put(b, b, -u * u - 2.0 * c);
            if i > 0 {
                put(a, a - 2, c);
                put(b, b - 2, c);
            }
            if i + 1 < cells {
                put(a, a + 2, c);
                put(b, b + 2, c);
            }
        }
    };
    let jacobian = Arc::new(move |_t: f64, y: &[f64]| {
        let mut j = DenseMatrix::zeros(n, n);
        entries(y, Box::new(|r, col, v| j[(r, col)] = v));
        Ok(j)
    });
    let (lower, upper) = (2, 2);
    let width = lower + upper + 1;
    let fill = Arc::new(
        move |_t: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
            entries(
                y,
                Box::new(|r, col, v| out[r * width + (col + lower - r)] = v),
            );
            Ok(())
        },
    );
    let y0 = (0..cells)
        .flat_map(|i| {
            let x = (i as f64 + 1.0) / (cells as f64 + 1.0);
            [1.0 + (2.0 * std::f64::consts::PI * x).sin(), 3.0]
        })
        .collect();
    (
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
        )
        .unwrap(),
        BandedJacobian { lower, upper, fill },
        y0,
    )
}

fn span_schedule(t0: f64, t1: f64) -> OutputSchedule {
    OutputSchedule::new(vec![t0, t1]).unwrap()
}

// ------------------------------------------------------------------ (a)

/// Test (a): under `Off` the dense driver is the fast driver bit for bit on
/// `[t0, t1]`: endpoint bits, attempts, accepted, rejected and every
/// counter; the banded twin likewise on Brusselator n = 100.
#[test]
fn off_matches_fast_driver_bitwise() {
    let (robertson, y_rob) = robertson_problem().unwrap();
    let (hires, y_hires) = hires();
    let cases: [(&str, &OdeProblem, &[f64], f64); 2] = [
        ("robertson", &robertson, &y_rob, 40.0),
        ("hires", &hires, &y_hires, 321.8122),
    ];
    for (name, problem, y0, t1) in cases {
        for rtol in RTOLS {
            let adaptive = config(rtol, t1);
            let fast = integrate_rodas5p_fast_observed(
                problem,
                (0.0, t1),
                y0,
                &adaptive,
                &span_schedule(0.0, t1),
            )
            .unwrap();
            let dense = integrate_rodas5p_fast_dense_observed(
                problem,
                (0.0, t1),
                y0,
                &adaptive,
                FastInteriorControl::Off,
            )
            .unwrap();
            let d = &dense.fast;
            assert!(
                fast.observed.success && d.observed.success,
                "{name} {rtol:e}"
            );
            assert_eq!(d.driver, RODAS5P_FAST_DENSE_DRIVER_ID);
            assert_eq!(
                bits(d.observed.y.last().unwrap()),
                bits(fast.observed.y.last().unwrap()),
                "{name} {rtol:e}"
            );
            assert_eq!(d.observed.t, fast.observed.t, "{name} {rtol:e}");
            assert_eq!(
                (d.attempts, d.accepted_steps, d.rejected_steps),
                (fast.attempts, fast.accepted_steps, fast.rejected_steps),
                "{name} {rtol:e}"
            );
            assert_eq!(
                d.observed.counters, fast.observed.counters,
                "{name} {rtol:e}"
            );
            assert_eq!(d.jacobian_reuses, fast.jacobian_reuses);
            assert_eq!(d.lu, fast.lu);
            assert_eq!(dense.segments.len(), fast.accepted_steps);
            assert_eq!(dense.interior.samples, 0);
            assert!(dense.partial.is_none() && dense.banded.is_none());
            let last = dense.segments.state(dense.segments.len());
            assert_eq!(bits(last), bits(fast.observed.y.last().unwrap()));
            eprintln!(
                "(a) {name} rtol {rtol:e}: attempts {}, accepted {}",
                d.attempts, d.accepted_steps
            );
        }
    }
    let (problem, band, y0) = brusselator(50);
    assert_eq!(problem.dimension, 100);
    for rtol in RTOLS {
        let adaptive = config(rtol, 10.0);
        let fast = integrate_rodas5p_fast_banded_observed(
            &problem,
            &band,
            (0.0, 10.0),
            &y0,
            &adaptive,
            &span_schedule(0.0, 10.0),
        )
        .unwrap();
        let dense = integrate_rodas5p_fast_banded_dense_observed(
            &problem,
            &band,
            (0.0, 10.0),
            &y0,
            &adaptive,
            FastInteriorControl::Off,
        )
        .unwrap();
        let d = &dense.fast;
        assert_eq!(d.driver, RODAS5P_FAST_BANDED_DENSE_DRIVER_ID);
        assert_eq!(
            bits(d.observed.y.last().unwrap()),
            bits(fast.fast.observed.y.last().unwrap()),
            "brusselator {rtol:e}"
        );
        assert_eq!(
            (d.attempts, d.accepted_steps, d.rejected_steps),
            (
                fast.fast.attempts,
                fast.fast.accepted_steps,
                fast.fast.rejected_steps
            )
        );
        assert_eq!(d.observed.counters, fast.fast.observed.counters);
        assert_eq!(dense.banded, Some(fast.work));
        assert_eq!(dense.partial, fast.partial);
        eprintln!(
            "(a) brusselator-100 rtol {rtol:e}: attempts {}, accepted {}",
            d.attempts, d.accepted_steps
        );
    }
}

// ------------------------------------------------------------------ (b)

fn assert_endpoints_stored(name: &str, run: &Rodas5pFastDenseResult) {
    let segments = &run.segments;
    let mut out = vec![0.0; segments.dimension()];
    for (k, &t) in segments.times().iter().enumerate() {
        segments.evaluate_into(t, &mut out).unwrap();
        assert_eq!(bits(&out), bits(segments.state(k)), "{name} k = {k}");
    }
    for k in 0..segments.len() {
        assert_eq!(segments.t_old(k), segments.times()[k]);
        assert_eq!(segments.t_new(k), segments.times()[k + 1]);
        assert!(segments.t_old(k) < segments.t_new(k));
    }
    // The final stored state is the driver's endpoint.
    assert_eq!(
        bits(segments.state(segments.len())),
        bits(run.fast.observed.y.last().unwrap()),
        "{name}"
    );
    // Outside the span and with a wrong buffer: typed errors.
    let last = *segments.times().last().unwrap();
    assert!(segments.evaluate_into(last + 1.0, &mut out).is_err());
    assert!(segments.evaluate_into(-1.0, &mut out).is_err());
    assert!(segments.evaluate_into(f64::NAN, &mut out).is_err());
    assert!(segments.evaluate_into(0.0, &mut [0.0; 0]).is_err());
}

/// Test (b): `evaluate_into(t_k)` is the stored state bit for bit, for
/// every k, under both controls.
#[test]
fn segment_endpoints_are_stored_states() {
    let (robertson, y_rob) = robertson_problem().unwrap();
    let (hires, y_hires) = hires();
    for control in [
        FastInteriorControl::Off,
        FastInteriorControl::WFilteredEnforce,
    ] {
        for (name, problem, y0, t1) in [
            ("robertson", &robertson, &y_rob, 40.0),
            ("hires", &hires, &y_hires, 321.8122),
        ] {
            let run = integrate_rodas5p_fast_dense_observed(
                problem,
                (0.0, t1),
                y0,
                &config(1.0e-6, t1),
                control,
            )
            .unwrap();
            assert!(run.fast.observed.success);
            assert_endpoints_stored(name, &run);
        }
    }
    let (problem, band, y0) = brusselator(50);
    let run = integrate_rodas5p_fast_banded_dense_observed(
        &problem,
        &band,
        (0.0, 10.0),
        &y0,
        &config(1.0e-6, 10.0),
        FastInteriorControl::WFilteredEnforce,
    )
    .unwrap();
    assert_endpoints_stored("brusselator", &run);
}

/// An empty span: no step, `evaluate_into(t0) == y0`.
#[test]
fn empty_span_keeps_the_initial_state() {
    let (problem, y0) = robertson_problem().unwrap();
    let run = integrate_rodas5p_fast_dense_observed(
        &problem,
        (2.0, 2.0),
        &y0,
        &config(1.0e-6, 1.0),
        FastInteriorControl::WFilteredEnforce,
    )
    .unwrap();
    assert!(run.fast.observed.success);
    assert_eq!(run.segments.len(), 0);
    assert!(run.segments.is_empty());
    let mut out = vec![0.0; 3];
    run.segments.evaluate_into(2.0, &mut out).unwrap();
    assert_eq!(bits(&out), bits(&y0));
    assert!(run.segments.evaluate_into(2.5, &mut out).is_err());
}

// --------------------------------------------------------------- (d), (e)

const PR_LAMBDA: f64 = -1.0e5;

fn pr_adaptive() -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 1.0e-8,
        rtol: 1.0e-6,
        initial_step: 1.0e-3,
        min_step: 1.0e-12,
        max_step: 1.0,
        max_attempts: 100_000,
        ..AdaptiveStepConfig::default()
    }
}

/// The run and its worst error over 201 evaluation points, in tolerance
/// units `|y - sin t| / (atol + rtol |sin t|)`.
fn pr_run(control: FastInteriorControl) -> (Rodas5pFastDenseResult, f64) {
    let (problem, y0) = prothero_robinson_problem(PR_LAMBDA, 0.0, 0.0);
    let adaptive = pr_adaptive();
    let run = integrate_rodas5p_fast_dense_observed(&problem, (0.0, 2.0), &y0, &adaptive, control)
        .unwrap();
    assert!(run.fast.observed.success, "{}", run.fast.observed.message);
    let mut out = [0.0];
    let mut worst = 0.0_f64;
    for i in 0..=200 {
        let t = i as f64 * 0.01;
        run.segments.evaluate_into(t, &mut out).unwrap();
        let exact = t.sin();
        worst = worst.max((out[0] - exact).abs() / (adaptive.atol + adaptive.rtol * exact.abs()));
    }
    (run, worst)
}

/// Test (d): Prothero-Robinson λ = -1e5 on [0, 2], rtol 1e-6, atol 1e-8:
/// the worst interior error over 201 points is at most 2 tolerance units
/// under `WFilteredEnforce`. The accepted steps are recorded next to the
/// sequential driver's 15 (WFilteredEnforce) and 7 (Off); measured, not
/// gated.
#[test]
fn wfiltered_pr_interior_within_two_units() {
    let (wf, wf_worst) = pr_run(FastInteriorControl::WFilteredEnforce);
    let (off, off_worst) = pr_run(FastInteriorControl::Off);
    eprintln!(
        "(d) PR λ = -1e5 rtol 1e-6: WFilteredEnforce interior {wf_worst:.4} units, accepted {} \
         (attempts {}, rejected {}, interior rejections {}, max estimate {:.4}); \
         Off interior {off_worst:.2} units, accepted {} (attempts {}); sequential: 15 / 7",
        wf.fast.accepted_steps,
        wf.fast.attempts,
        wf.fast.rejected_steps,
        wf.interior.interior_rejections,
        wf.interior.max_estimate,
        off.fast.accepted_steps,
        off.fast.attempts,
    );
    assert!(wf_worst <= 2.0, "interior {wf_worst}");
    assert!(wf.interior.max_estimate <= 1.0);
}

/// Test (e): the counter ledger closes, and the interior estimate rejects
/// embedded-accepted attempts.
///
/// Deviation from the brief, measured: on the *linear* PR (λ = -1e5,
/// rtol 1e-6) WFilteredEnforce rejects no attempt (15 attempts, 15
/// accepted), as the sequential driver does (15 / 15, `dense_wfiltered_
/// contracts.rs`): the controller sees max(embedded, estimate) in the step
/// proposal, so no proposed step exceeds one unit. Interior rejections are
/// gated on the nonlinear PR (μ = 1e5) at the same tolerances instead (2 of
/// 27 attempts when written); the linear run keeps the ledger checks.
#[test]
fn interior_rejection_counter_sum() {
    let check_ledger = |run: &Rodas5pFastDenseResult| {
        let fast = &run.fast;
        let interior = run.interior;
        assert_eq!(fast.accepted_steps + fast.rejected_steps, fast.attempts);
        assert_eq!(
            fast.observed.counters.accepted_steps as usize,
            fast.accepted_steps
        );
        assert_eq!(
            fast.observed.counters.rejected_steps as usize,
            fast.rejected_steps
        );
        // Every interior rejection is a local-error rejection of the run.
        assert!(interior.interior_rejections <= fast.observed.counters.local_error_failures);
        // One sample per attempt the embedded estimate accepted, each with
        // one right-hand side and one back-solve.
        assert_eq!(
            interior.samples,
            fast.accepted_steps as u64 + interior.interior_rejections
        );
        assert_eq!(interior.rhs_calls, interior.samples);
        assert_eq!(interior.back_solves, interior.samples);
        // No Jacobian and no factorization beyond the attempts'.
        let c = &fast.observed.counters;
        assert!(c.direct_factorizations <= fast.attempts as u64);
        assert!(c.jacobian_builds <= fast.attempts as u64);
    };
    let (linear, _) = pr_run(FastInteriorControl::WFilteredEnforce);
    check_ledger(&linear);
    let (problem, y0) = prothero_robinson_problem(PR_LAMBDA, 1.0e5, 0.0);
    let nonlinear = integrate_rodas5p_fast_dense_observed(
        &problem,
        (0.0, 2.0),
        &y0,
        &pr_adaptive(),
        FastInteriorControl::WFilteredEnforce,
    )
    .unwrap();
    assert!(nonlinear.fast.observed.success);
    check_ledger(&nonlinear);
    eprintln!(
        "(e) PR λ = -1e5 rtol 1e-6: linear {} interior rejections in {} attempts; \
         μ = 1e5: {} in {} attempts",
        linear.interior.interior_rejections,
        linear.fast.attempts,
        nonlinear.interior.interior_rejections,
        nonlinear.fast.attempts
    );
    assert!(
        nonlinear.interior.interior_rejections > 0,
        "no interior rejection: {:?}",
        nonlinear.interior
    );
    let (off, _) = pr_run(FastInteriorControl::Off);
    assert!(off.interior.samples == 0 && off.interior.rhs_calls == 0);
}

// ------------------------------------------------------------------ (f)

/// Robertson whose RHS fails (as the facade's fatal latch does: every call
/// after the first failing one fails too, as `NonFinite`) once it is called
/// at a time beyond `latch_after`.
fn latched_robertson(latch_after: f64) -> OdeProblem {
    let latch = Arc::new(AtomicBool::new(false));
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        if t > latch_after {
            latch.store(true, Ordering::SeqCst);
        }
        if latch.load(Ordering::SeqCst) {
            return Err(CoreError::NonFinite("fatal RHS latch".into()));
        }
        out[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
        out[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
        out[2] = 3.0e7 * y[1] * y[1];
        Ok(())
    });
    let jacobian = Arc::new(|_t: f64, y: &[f64]| {
        DenseMatrix::new(
            3,
            3,
            vec![
                -0.04,
                1.0e4 * y[2],
                1.0e4 * y[1],
                0.04,
                -1.0e4 * y[2] - 6.0e7 * y[1],
                -1.0e4 * y[1],
                0.0,
                6.0e7 * y[1],
                0.0,
            ],
        )
    });
    OdeProblem::new(
        "latched-robertson",
        3,
        rhs,
        None,
        Some(jacobian),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

/// Test (f): with a fatal RHS after at least three steps, the segments of
/// the failed run are a bit-identical prefix of the clean run's.
#[test]
fn fatal_rhs_partial_prefix() {
    let y0 = [1.0, 0.0, 0.0];
    let adaptive = config(1.0e-6, 40.0);
    let clean = integrate_rodas5p_fast_dense_observed(
        &latched_robertson(f64::INFINITY),
        (0.0, 40.0),
        &y0,
        &adaptive,
        FastInteriorControl::WFilteredEnforce,
    )
    .unwrap();
    assert!(clean.fast.observed.success && clean.segments.len() > 6);
    // Every call of the first four steps is at a time <= the fourth end.
    let latch_after = clean.segments.t_new(3);
    let failed = integrate_rodas5p_fast_dense_observed(
        &latched_robertson(latch_after),
        (0.0, 40.0),
        &y0,
        &adaptive,
        FastInteriorControl::WFilteredEnforce,
    )
    .unwrap();
    assert!(!failed.fast.observed.success);
    let partial = failed.partial.as_ref().expect("partial record");
    let m = failed.segments.len();
    assert!(m >= 3, "{m} steps");
    assert!(m < clean.segments.len());
    assert_eq!(partial.t, failed.segments.t_new(m - 1));
    assert_eq!(bits(&partial.y), bits(failed.segments.state(m)));
    for k in 0..=m {
        assert_eq!(
            failed.segments.times()[k].to_bits(),
            clean.segments.times()[k].to_bits()
        );
        assert_eq!(
            bits(failed.segments.state(k)),
            bits(clean.segments.state(k))
        );
    }
    for k in 0..m {
        assert_eq!(
            bits(failed.segments.stage_rows(k)),
            bits(clean.segments.stage_rows(k))
        );
    }
    eprintln!("(f) fatal RHS after {latch_after:e}: {m} segments kept");
}

// --------------------------------------------------- R-C1 (p0-atol seam)

fn assert_same_run(a: &Rodas5pFastDenseResult, b: &Rodas5pFastDenseResult, what: &str) {
    assert_eq!(
        (
            a.fast.attempts,
            a.fast.accepted_steps,
            a.fast.rejected_steps
        ),
        (
            b.fast.attempts,
            b.fast.accepted_steps,
            b.fast.rejected_steps
        ),
        "{what}"
    );
    assert_eq!(a.fast.observed.counters, b.fast.observed.counters, "{what}");
    assert_eq!(a.interior, b.interior, "{what}");
    assert_eq!(a.banded, b.banded, "{what}");
    assert_eq!(a.segments, b.segments, "{what}");
}

/// A uniform atol vector equal to the scalar is the scalar run bit for bit
/// under W-filtered dense output (the estimate's scale reads the vector);
/// a wrong length is refused.
#[test]
fn uniform_atol_vector_is_the_scalar_dense_run() {
    let (problem, y0) = hires();
    let adaptive = config(1.0e-6, 321.8122);
    let control = FastInteriorControl::WFilteredEnforce;
    let scalar =
        integrate_rodas5p_fast_dense_observed(&problem, (0.0, 321.8122), &y0, &adaptive, control)
            .unwrap();
    let vector = integrate_rodas5p_fast_dense_observed_with_atol(
        &problem,
        (0.0, 321.8122),
        &y0,
        &adaptive,
        control,
        &[adaptive.atol; 8],
    )
    .unwrap();
    assert!(scalar.interior.samples > 0);
    assert_same_run(&scalar, &vector, "dense");
    assert!(
        integrate_rodas5p_fast_dense_observed_with_atol(
            &problem,
            (0.0, 1.0),
            &y0,
            &adaptive,
            control,
            &[adaptive.atol; 7],
        )
        .is_err()
    );
    // A larger atol on one component changes the run.
    let mut loose = [adaptive.atol; 8];
    loose[6] = 1.0e3 * adaptive.atol;
    let changed = integrate_rodas5p_fast_dense_observed_with_atol(
        &problem,
        (0.0, 321.8122),
        &y0,
        &adaptive,
        control,
        &loose,
    )
    .unwrap();
    assert_ne!(changed.segments, scalar.segments);
    let (problem, band, y0) = brusselator(50);
    let adaptive = config(1.0e-6, 10.0);
    let scalar = integrate_rodas5p_fast_banded_dense_observed(
        &problem,
        &band,
        (0.0, 10.0),
        &y0,
        &adaptive,
        control,
    )
    .unwrap();
    let vector = integrate_rodas5p_fast_banded_dense_observed_with_atol(
        &problem,
        &band,
        (0.0, 10.0),
        &y0,
        &adaptive,
        control,
        &[adaptive.atol; 100],
    )
    .unwrap();
    assert_same_run(&scalar, &vector, "banded");
}
