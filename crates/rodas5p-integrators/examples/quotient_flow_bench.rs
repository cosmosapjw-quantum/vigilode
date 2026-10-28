//! Small single-thread declared-structure versus full-RHS work/precision probe.
use rodas5p_integrators::{
    DampedRotationBlock as Block, DampedRotationFlow as Flow, Dopri5Config, OutputSchedule,
    integrate_dopri5_observed, integrate_lawson_dopri5_observed,
};
use serde_json::json;
use std::time::Instant;
#[derive(Clone, Copy)]
struct Case {
    name: &'static str,
    d: f64,
    w: f64,
    kind: u8,
    n: usize,
    t: f64,
}
fn main() {
    let cases = [
        Case {
            name: "slow-control",
            d: 0.0,
            w: 1.0,
            kind: 0,
            n: 2,
            t: 2.0,
        },
        Case {
            name: "rapid-linear",
            d: 0.0,
            w: 10000.0,
            kind: 0,
            n: 2,
            t: 1.0,
        },
        Case {
            name: "stiff-damped-rotation",
            d: 5000.0,
            w: 10000.0,
            kind: 0,
            n: 2,
            t: 0.05,
        },
        Case {
            name: "radial-nonlinear",
            d: 0.0,
            w: 2000.0,
            kind: 1,
            n: 2,
            t: 2.0,
        },
        Case {
            name: "shared-carrier-128",
            d: 0.0,
            w: 2000.0,
            kind: 1,
            n: 128,
            t: 1.0,
        },
        Case {
            name: "noncommuting-residual",
            d: 0.3,
            w: 2.0,
            kind: 2,
            n: 2,
            t: 2.0,
        },
        Case {
            name: "stiff-forcing-control",
            d: 500.0,
            w: 500.0,
            kind: 3,
            n: 2,
            t: 1.0,
        },
    ];
    let mut rows = Vec::new();
    for case in cases {
        for tol in [1e-6, 1e-8] {
            let f = Flow::repeated(
                Block {
                    damping: case.d,
                    frequency: case.w,
                },
                case.n / 2,
            )
            .unwrap();
            let residual = |t: f64, y: &[f64], out: &mut [f64]| {
                for i in (0..y.len()).step_by(2) {
                    let (x, z) = (y[i], y[i + 1]);
                    match case.kind {
                        0 => {
                            out[i] = 0.0;
                            out[i + 1] = 0.0;
                        }
                        1 => {
                            let a = 0.2 * (1.0 - x * x - z * z);
                            out[i] = a * x;
                            out[i + 1] = a * z;
                        }
                        2 => {
                            out[i] = 0.2 * x;
                            out[i + 1] = -0.1 * z;
                        }
                        _ => {
                            let (s, c) = t.sin_cos();
                            out[i] = -s + case.d * c + case.w * s;
                            out[i + 1] = c - case.w * c + case.d * s;
                        }
                    }
                }
                Ok(())
            };
            let full = |t: f64, y: &[f64], out: &mut [f64]| {
                residual(t, y, out)?;
                for i in (0..y.len()).step_by(2) {
                    out[i] += -case.d * y[i] - case.w * y[i + 1];
                    out[i + 1] += case.w * y[i] - case.d * y[i + 1];
                }
                Ok(())
            };
            let exact = |t: f64| -> Vec<f64> {
                let pair = match case.kind {
                    1 => {
                        let r = (1.0 / (1.0 + 24.0 * (-0.4 * t).exp())).sqrt();
                        let (s, c) = (case.w * t).sin_cos();
                        [r * c, r * s]
                    }
                    2 => {
                        let nu = (case.w * case.w - 0.0225).sqrt();
                        let (s, c) = (nu * t).sin_cos();
                        let a = ((-case.d + 0.05) * t).exp();
                        [
                            a * (c + s / nu * (0.15 - case.w * 0.2)),
                            a * (0.2 * c + s / nu * (case.w - 0.15 * 0.2)),
                        ]
                    }
                    3 => [t.cos(), t.sin()],
                    _ => {
                        let a = (-case.d * t).exp();
                        let (s, c) = (case.w * t).sin_cos();
                        [a * c, a * s]
                    }
                };
                (0..case.n).map(|i| pair[i % 2]).collect()
            };
            let mut config = Dopri5Config::default();
            config.adaptive.rtol = tol;
            config.adaptive.atol = tol * 0.01;
            config.adaptive.initial_step = 0.001;
            config.adaptive.max_attempts = 500_000;
            let schedule =
                OutputSchedule::new((0..=10).map(|i| case.t * i as f64 / 10.0).collect()).unwrap();
            for method in ["full-rhs-dopri5", "quotient-lawson-dopri5"] {
                for repeat in 0..4 {
                    let start = Instant::now();
                    let (r, work) = if method == "full-rhs-dopri5" {
                        (
                            integrate_dopri5_observed(
                                &full,
                                (0.0, case.t),
                                &exact(0.0),
                                &config,
                                &schedule,
                            )
                            .unwrap(),
                            None,
                        )
                    } else {
                        let r = integrate_lawson_dopri5_observed(
                            &f,
                            &residual,
                            (0.0, case.t),
                            &exact(0.0),
                            &config,
                            &schedule,
                        )
                        .unwrap();
                        (r.integration, Some(r.quotient_work))
                    };
                    let seconds = start.elapsed().as_secs_f64();
                    let error = r
                        .observed
                        .t
                        .iter()
                        .zip(&r.observed.y)
                        .flat_map(|(&t, y)| {
                            y.iter()
                                .copied()
                                .zip(exact(t))
                                .map(|(x, z)| (x - z).abs())
                                .collect::<Vec<_>>()
                        })
                        .fold(0.0, f64::max);
                    rows.push(json!({"case":case.name,"dimension":case.n,"damping":case.d,"frequency":case.w,"tol":tol,"method":method,"repeat":repeat,"warmup":repeat==0,"seconds":seconds,"success":r.observed.success,"maximum_sampled_absolute_error":error,"attempts":r.attempts,"accepted":r.accepted_steps,"rejected":r.rejected_steps,"rhs_calls":r.observed.counters.rhs_calls,"quotient_work":work}));
                }
            }
        }
    }
    println!("{}",serde_json::to_string_pretty(&json!({"schema":"vigilode.quotient-probe.v1","threads":1,"cases":7,"rows":rows,"scope":"same nominal tolerances; all observed errors measured independently; use matched error bounds for speed claims"})).unwrap());
}
