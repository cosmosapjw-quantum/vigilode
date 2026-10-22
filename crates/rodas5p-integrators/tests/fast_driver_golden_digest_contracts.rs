//! Golden digests of every pre-existing fast-driver entry point, recorded on
//! the tree before library steps L1-L3 (5fd6e459): dense, banded (all
//! kernels), options, dense-output, telemetry, partial-record and single-step
//! paths on non-autonomous problems with analytic, finite-difference and
//! failing `f_t`, including attempt-budget and minimum-step stops. Each line
//! hashes the output-state bits, the WorkCounters JSON, the step statistics,
//! the driver id, the band work and the partial record; the whole text must
//! equal the 5fd6e459 text (2002 lines, FNV-1a 64 below). Set `PROBE_OUT` to
//! write the text for a diff when it does not.

use std::fmt::Write as _;
use std::io::Write as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rodas5p_core::{CoreError, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, BandedKernel, ControllerTelemetry, FastInteriorControl,
    FastLuPolicy, OdeProblem, OutputSchedule, Rodas5pFastOptions, Rodas5pFastResult,
    integrate_rodas5p_fast_banded_dense_observed,
    integrate_rodas5p_fast_banded_dense_observed_with_atol, integrate_rodas5p_fast_banded_observed,
    integrate_rodas5p_fast_banded_observed_with_atol,
    integrate_rodas5p_fast_banded_observed_with_kernel,
    integrate_rodas5p_fast_banded_observed_with_options, integrate_rodas5p_fast_dense_observed,
    integrate_rodas5p_fast_dense_observed_with_atol, integrate_rodas5p_fast_observed,
    integrate_rodas5p_fast_observed_with_atol, integrate_rodas5p_fast_observed_with_options,
    integrate_rodas5p_fast_observed_with_partial, integrate_rodas5p_fast_observed_with_telemetry,
    prothero_robinson_problem, robertson_problem, rodas5p_fast_step,
};

#[path = "b1_problems/mod.rs"]
mod b1_problems;

fn fnv(s: &str) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325_u64;
    for b in s.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn bitsdigest(v: &[Vec<f64>]) -> u64 {
    let mut s = String::new();
    for row in v {
        for x in row {
            let _ = write!(s, "{:016x},", x.to_bits());
        }
        s.push(';');
    }
    fnv(&s)
}

fn fast_line(r: &Rodas5pFastResult) -> String {
    let o = &r.observed;
    let tb: Vec<Vec<f64>> = vec![o.t.clone()];
    format!(
        "t={:x} y={:x} succ={} msg={:?} counters={} internal={} clipped={} att={} acc={} rej={} reuse={} lu={:?} drv={}",
        bitsdigest(&tb),
        bitsdigest(&o.y),
        o.success,
        o.message,
        serde_json::to_string(&o.counters).unwrap(),
        o.internal_steps,
        o.output_clipped_steps,
        r.attempts,
        r.accepted_steps,
        r.rejected_steps,
        r.jacobian_reuses,
        r.lu,
        r.driver
    )
}

fn json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).unwrap()
}

const CELLS: usize = 12;

/// Brusselator plus a forcing `eps sin(w t + i)`; `ft` mode:
/// 0 analytic, 1 FD fallback, 2 analytic but fails (Err NonFinite) on
/// every 5th call, 3 analytic writes NaN on every 4th call, 4 accumulating
/// (`+=`, relies on zeroed out), 5 analytic returning Err(LinearSolve)
/// every 6th call.
fn forced_brusselator(mode: u8) -> (OdeProblem, Vec<f64>, BandedJacobian) {
    let cells = CELLS;
    let (base, y0) = b1_problems::brusselator_problem(cells).unwrap();
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    let (eps, w) = (0.3, 7.0);
    let rhs_base = base.clone();
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let mut wc = WorkCounters::default();
        rhs_base.eval_rhs_into(t, y, out, &mut wc)?;
        for (i, o) in out.iter_mut().enumerate() {
            *o += eps * (w * t + i as f64).sin();
        }
        Ok(())
    });
    let jac_base = base.clone();
    let jacobian = Arc::new(move |t: f64, y: &[f64]| {
        jac_base.dense_jacobian(t, y, &mut WorkCounters::default())
    });
    let calls = Arc::new(AtomicUsize::new(0));
    type PartialT =
        Arc<dyn Fn(f64, &[f64], &mut [f64]) -> rodas5p_core::CoreResult<()> + Send + Sync>;
    let ft: Option<PartialT> = match mode {
        1 => None,
        _ => {
            let calls = calls.clone();
            Some(Arc::new(move |t: f64, _y: &[f64], out: &mut [f64]| {
                let k = calls.fetch_add(1, Ordering::Relaxed) + 1;
                if mode == 2 && k.is_multiple_of(5) {
                    out[0] = 123.0;
                    return Err(CoreError::NonFinite("probe".into()));
                }
                if mode == 5 && k.is_multiple_of(6) {
                    out[1] = -7.0;
                    return Err(CoreError::LinearSolve("probe".into()));
                }
                for (i, o) in out.iter_mut().enumerate() {
                    let v = eps * w * (w * t + i as f64).cos();
                    if mode == 4 {
                        *o += v;
                    } else {
                        *o = v;
                    }
                }
                if mode == 3 && k.is_multiple_of(4) {
                    out[2] = f64::NAN;
                }
                Ok(())
            }))
        }
    };
    let n = 2 * cells;
    let problem = OdeProblem::new(
        format!("forced-brusselator-{mode}"),
        n,
        rhs,
        None,
        Some(jacobian),
        None,
        ft,
        false,
        None,
        None,
    )
    .unwrap();
    let band = BandedJacobian {
        lower: 2,
        upper: 2,
        fill: Arc::new(move |_t, y: &[f64], band: &mut [f64]| {
            let at = |band: &mut [f64], i: usize, j: usize, v: f64| {
                band[i * 5 + (j + 2 - i)] = v;
            };
            for i in 0..cells {
                let (u, v) = (y[2 * i], y[2 * i + 1]);
                let (a, b) = (2 * i, 2 * i + 1);
                at(band, a, a, 2.0 * u * v - 4.0 - 2.0 * c);
                at(band, a, b, u * u);
                at(band, b, a, 3.0 - 2.0 * u * v);
                at(band, b, b, -u * u - 2.0 * c);
                if i > 0 {
                    at(band, a, a - 2, c);
                    at(band, b, b - 2, c);
                }
                if i + 1 < cells {
                    at(band, a, a + 2, c);
                    at(band, b, b + 2, c);
                }
            }
            Ok(())
        }),
    };
    (problem, y0, band)
}

fn autonomous_brusselator() -> (OdeProblem, Vec<f64>, BandedJacobian) {
    let (_, _, band) = forced_brusselator(0);
    let (p, y0) = b1_problems::brusselator_problem(CELLS).unwrap();
    (p, y0, band)
}

fn cfg(rtol: f64, atol: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol,
        rtol,
        initial_step: 1.0e-4,
        max_attempts: 200_000,
        ..AdaptiveStepConfig::default()
    }
}

fn all_options() -> Vec<Rodas5pFastOptions> {
    let mut v = Vec::new();
    for prevalidated_controller in [false, true] {
        for fused_landing in [false, true] {
            for lu_policy in [
                FastLuPolicy::Legacy,
                FastLuPolicy::ColumnExtents,
                FastLuPolicy::ColumnExtentsAbove64,
            ] {
                v.push(Rodas5pFastOptions {
                    prevalidated_controller,
                    fused_landing,
                    lu_policy,
                });
            }
        }
    }
    v
}

fn res<T>(r: rodas5p_core::CoreResult<T>, f: impl FnOnce(T) -> String) -> String {
    match r {
        Ok(v) => f(v),
        Err(e) => format!("ERR {e:?}"),
    }
}

/// Line count and FNV-1a 64 of the probe text on 5fd6e459.
const GOLDEN_LINES: usize = 2002;
const GOLDEN_FNV1A64: u64 = 0x745e_3d9d_b5ee_61d6;

#[test]
fn pre_existing_entry_points_equal_their_5fd6e459_digests() {
    let mut out = String::new();
    let mut line = |name: &str, s: String| {
        let _ = writeln!(out, "{name}\t{s}");
    };
    let schedule = OutputSchedule::new(vec![0.0, 0.25, 0.5, 1.0, 1.5]).unwrap();
    let span = (0.0, 1.5);
    let mut problems: Vec<(String, OdeProblem, Vec<f64>, Option<BandedJacobian>)> = Vec::new();
    for mode in 0..=5u8 {
        let (p, y0, band) = forced_brusselator(mode);
        problems.push((format!("forced{mode}"), p, y0, Some(band)));
    }
    let (p, y0, band) = autonomous_brusselator();
    problems.push(("auto-bruss".into(), p, y0, Some(band)));
    let (p, y0) = prothero_robinson_problem(-1.0e4, 0.5, 0.0);
    problems.push(("pr".into(), p, y0, None));
    let (p, y0) = robertson_problem().unwrap();
    problems.push(("rober".into(), p, y0, None));
    let (p, y0) = b1_problems::hires_problem().unwrap();
    problems.push(("hires".into(), p, y0, None));

    for (name, problem, y0, band) in &problems {
        let n = problem.dimension;
        for rtol in [1.0e-5, 1.0e-8] {
            let a = cfg(rtol, 1.0e-3 * rtol);
            let tag = format!("{name}-r{rtol:e}");
            line(
                &format!("{tag} observed"),
                res(
                    integrate_rodas5p_fast_observed(problem, span, y0, &a, &schedule),
                    |r| fast_line(&r),
                ),
            );
            for (k, o) in all_options().into_iter().enumerate() {
                line(
                    &format!("{tag} options{k}"),
                    res(
                        integrate_rodas5p_fast_observed_with_options(
                            problem, span, y0, &a, &schedule, o,
                        ),
                        |r| fast_line(&r),
                    ),
                );
            }
            for (k, aa) in [
                a.clone(),
                AdaptiveStepConfig {
                    max_attempts: 23,
                    ..a.clone()
                },
                AdaptiveStepConfig {
                    min_step: 0.04,
                    initial_step: 0.05,
                    ..a.clone()
                },
                AdaptiveStepConfig {
                    min_step: 1.0e-3,
                    initial_step: 0.3,
                    rtol: 1.0e-12,
                    atol: 1.0e-14,
                    ..a.clone()
                },
            ]
            .iter()
            .enumerate()
            {
                line(
                    &format!("{tag} partial{k}"),
                    res(
                        integrate_rodas5p_fast_observed_with_partial(
                            problem, span, y0, aa, &schedule,
                        ),
                        |(r, p)| format!("{} partial={}", fast_line(&r), json(&p)),
                    ),
                );
                let atol: Vec<f64> = (0..n).map(|i| aa.atol * (1 + i % 3) as f64).collect();
                line(
                    &format!("{tag} atol{k}"),
                    res(
                        integrate_rodas5p_fast_observed_with_atol(
                            problem, span, y0, aa, &schedule, &atol,
                        ),
                        |(r, p)| format!("{} partial={}", fast_line(&r), json(&p)),
                    ),
                );
                for (ik, ic) in [
                    FastInteriorControl::Off,
                    FastInteriorControl::WFilteredEnforce,
                ]
                .into_iter()
                .enumerate()
                {
                    line(
                        &format!("{tag} dense{k}-{ik}"),
                        res(
                            integrate_rodas5p_fast_dense_observed(problem, span, y0, aa, ic),
                            |r| {
                                format!(
                                    "{} seg={:x} int={:?} partial={} band={:?}",
                                    fast_line(&r.fast),
                                    fnv(&format!("{:?}", r.segments)),
                                    r.interior,
                                    json(&r.partial),
                                    r.banded
                                )
                            },
                        ),
                    );
                    line(
                        &format!("{tag} dense-atol{k}-{ik}"),
                        res(
                            integrate_rodas5p_fast_dense_observed_with_atol(
                                problem, span, y0, aa, ic, &atol,
                            ),
                            |r| {
                                format!(
                                    "{} seg={:x} int={:?} partial={} band={:?}",
                                    fast_line(&r.fast),
                                    fnv(&format!("{:?}", r.segments)),
                                    r.interior,
                                    json(&r.partial),
                                    r.banded
                                )
                            },
                        ),
                    );
                }
                if let Some(band) = band {
                    let b = |r: rodas5p_integrators::Rodas5pFastBandedResult| {
                        format!(
                            "{} work={:?} partial={}",
                            fast_line(&r.fast),
                            r.work,
                            json(&r.partial)
                        )
                    };
                    line(
                        &format!("{tag} banded{k}"),
                        res(
                            integrate_rodas5p_fast_banded_observed(
                                problem, band, span, y0, aa, &schedule,
                            ),
                            b,
                        ),
                    );
                    line(
                        &format!("{tag} banded-atol{k}"),
                        res(
                            integrate_rodas5p_fast_banded_observed_with_atol(
                                problem, band, span, y0, aa, &schedule, &atol,
                            ),
                            b,
                        ),
                    );
                    for kernel in [
                        BandedKernel::Indexed,
                        BandedKernel::Slices,
                        BandedKernel::FixedWidth2,
                        BandedKernel::FixedWidth2ExactCache,
                    ] {
                        line(
                            &format!("{tag} banded-kernel{k}-{kernel:?}"),
                            res(
                                integrate_rodas5p_fast_banded_observed_with_kernel(
                                    problem, band, span, y0, aa, &schedule, kernel,
                                ),
                                b,
                            ),
                        );
                    }
                    for (ok, o) in all_options().into_iter().enumerate() {
                        line(
                            &format!("{tag} banded-options{k}-{ok}"),
                            res(
                                integrate_rodas5p_fast_banded_observed_with_options(
                                    problem, band, span, y0, aa, &schedule, o,
                                ),
                                b,
                            ),
                        );
                    }
                    for (ik, ic) in [
                        FastInteriorControl::Off,
                        FastInteriorControl::WFilteredEnforce,
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let d = |r: rodas5p_integrators::Rodas5pFastDenseResult| {
                            format!(
                                "{} seg={:x} int={:?} partial={} band={:?}",
                                fast_line(&r.fast),
                                fnv(&format!("{:?}", r.segments)),
                                r.interior,
                                json(&r.partial),
                                r.banded
                            )
                        };
                        line(
                            &format!("{tag} banded-dense{k}-{ik}"),
                            res(
                                integrate_rodas5p_fast_banded_dense_observed(
                                    problem, band, span, y0, aa, ic,
                                ),
                                d,
                            ),
                        );
                        line(
                            &format!("{tag} banded-dense-atol{k}-{ik}"),
                            res(
                                integrate_rodas5p_fast_banded_dense_observed_with_atol(
                                    problem, band, span, y0, aa, ic, &atol,
                                ),
                                d,
                            ),
                        );
                    }
                }
            }
            line(
                &format!("{tag} telemetry"),
                res(
                    integrate_rodas5p_fast_observed_with_telemetry(
                        problem,
                        span,
                        y0,
                        &a,
                        &schedule,
                        ControllerTelemetry::with_trace(),
                    ),
                    |(r, t)| format!("{} tel={:x}", fast_line(&r), fnv(&format!("{t:?}"))),
                ),
            );
        }
        // Single steps.
        let mut counters = WorkCounters::default();
        let mut s = String::new();
        for h in [1.0e-4, 1.0e-2, 0.2] {
            match rodas5p_fast_step(problem, 0.1, y0, h, 1.0e-8, 1.0e-6, &mut counters) {
                Ok((y, e, lu)) => {
                    let _ = write!(s, "[{:x} {:016x} {:?}]", bitsdigest(&[y]), e.to_bits(), lu);
                }
                Err(e) => {
                    let _ = write!(s, "[ERR {e:?}]");
                }
            }
        }
        let _ = write!(s, " counters={}", json(&counters));
        line(&format!("{name} step"), s);
    }
    if let Ok(path) = std::env::var("PROBE_OUT") {
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(out.as_bytes()).unwrap();
    }
    assert_eq!(out.lines().count(), GOLDEN_LINES);
    assert_eq!(
        fnv(&out),
        GOLDEN_FNV1A64,
        "the pre-existing entry points left the 5fd6e459 behaviour"
    );
}
