//! Single-thread affine-forcing work/precision probe; all methods see identical observations.
use rodas5p_integrators::{
    DampedRotationBlock as Block, DampedRotationFlow as Flow, Dopri5Config, OutputSchedule,
    integrate_affine_lawson_dopri5_observed, integrate_dopri5_observed,
    integrate_lawson_dopri5_observed,
};
use serde_json::json;
use std::time::Instant;
#[derive(Clone, Copy)]
struct Case {
    name: &'static str,
    d: f64,
    w: f64,
    n: usize,
    t: f64,
    varying: bool,
}
fn main() {
    let cases = [
        Case {
            name: "zero-block-constant",
            d: 0.,
            w: 0.,
            n: 2,
            t: 1.,
            varying: false,
        },
        Case {
            name: "slow-constant",
            d: 0.2,
            w: 1.,
            n: 2,
            t: 1.,
            varying: false,
        },
        Case {
            name: "stiff-constant",
            d: 500.,
            w: 500.,
            n: 2,
            t: 1.,
            varying: false,
        },
        Case {
            name: "rapid-constant",
            d: 0.,
            w: 10000.,
            n: 2,
            t: 0.1,
            varying: false,
        },
        Case {
            name: "shared128-constant",
            d: 500.,
            w: 500.,
            n: 128,
            t: 1.,
            varying: false,
        },
        Case {
            name: "stiff-time-forcing-negative-control",
            d: 500.,
            w: 500.,
            n: 2,
            t: 1.,
            varying: true,
        },
    ];
    let mut rows = Vec::new();
    for case in cases {
        let flow = Flow::repeated(
            Block {
                damping: case.d,
                frequency: case.w,
            },
            case.n / 2,
        )
        .unwrap();
        let b: Vec<f64> = (0..case.n)
            .map(|i| {
                if case.varying {
                    0.
                } else if i % 2 == 0 {
                    1.
                } else {
                    -0.5
                }
            })
            .collect();
        let y0: Vec<f64> = (0..case.n)
            .map(|i| if case.varying && i % 2 == 0 { 1. } else { 0. })
            .collect();
        let remaining = |t: f64, _: &[f64], o: &mut [f64]| {
            for i in (0..case.n).step_by(2) {
                if case.varying {
                    let (s, c) = t.sin_cos();
                    o[i] = -s + case.d * c + case.w * s;
                    o[i + 1] = c - case.w * c + case.d * s;
                } else {
                    o[i] = 0.;
                    o[i + 1] = 0.;
                }
            }
            Ok(())
        };
        let original = |t: f64, y: &[f64], o: &mut [f64]| {
            remaining(t, y, o)?;
            for (i, v) in o.iter_mut().enumerate() {
                *v += b[i];
            }
            Ok(())
        };
        let full = |t: f64, y: &[f64], o: &mut [f64]| {
            original(t, y, o)?;
            for i in (0..case.n).step_by(2) {
                o[i] += -case.d * y[i] - case.w * y[i + 1];
                o[i + 1] += case.w * y[i] - case.d * y[i + 1];
            }
            Ok(())
        };
        let times: Vec<f64> = (0..=16).map(|i| case.t * i as f64 / 16.).collect();
        let schedule = OutputSchedule::new(times.clone()).unwrap();
        // Independent analytic oracle: elementary constant-forcing closed form;
        // the varying-forcing control is manufactured to have (cos(t),sin(t)).
        let exact = |t: f64| -> Vec<f64> {
            let pair = if case.varying {
                [t.cos(), t.sin()]
            } else if case.d == 0. && case.w == 0. {
                [t, -0.5 * t]
            } else {
                let u = (-case.d * t).exp() * (case.w * t).cos() - 1.;
                let v = (-case.d * t).exp() * (case.w * t).sin();
                let den = case.d * case.d + case.w * case.w;
                let c = (-case.d * u + case.w * v) / den;
                let s = (-case.w * u - case.d * v) / den;
                [c + 0.5 * s, s - 0.5 * c]
            };
            (0..case.n).map(|i| pair[i % 2]).collect()
        };
        for tol in [1e-6, 1e-8] {
            let mut config = Dopri5Config::default();
            config.adaptive.rtol = tol;
            config.adaptive.atol = tol * 0.01;
            config.adaptive.initial_step = case.t / 16.;
            config.adaptive.max_attempts = 500000;
            for method in [
                "full-rhs-dopri5",
                "original-lawson",
                "affine-lawson",
                "exact-declared-forcing",
            ] {
                for repeat in 0..4 {
                    if method == "exact-declared-forcing" {
                        let start = Instant::now();
                        let mut outputs = Vec::new();
                        let cosine = vec![case.d, 1. - case.w];
                        let sine = vec![case.w - 1., case.d];
                        for &t in &times {
                            let mut y = vec![0.; case.n];
                            if case.varying {
                                flow.harmonic_into(0., t, &y0, 1., &cosine, &sine, &mut y)
                                    .unwrap();
                            } else {
                                flow.affine_into(t, &y0, &b, &mut y).unwrap();
                            }
                            outputs.push(y);
                        }
                        let seconds = start.elapsed().as_secs_f64();
                        let error = times
                            .iter()
                            .zip(&outputs)
                            .flat_map(|(&t, y)| {
                                y.iter()
                                    .zip(exact(t))
                                    .map(|(a, b)| (a - b).abs())
                                    .collect::<Vec<_>>()
                            })
                            .fold(0., f64::max);
                        rows.push(json!({"case":case.name,"dimension":case.n,"damping":case.d,"frequency":case.w,"constant_forcing":!case.varying,"method":method,"tol":tol,"tolerance_used":false,"repeat":repeat,"warmup":repeat==0,"seconds":seconds,"success":true,"maximum_sampled_absolute_error":error,"attempts":null,"accepted":null,"rejected":0,"rhs_calls":0,"quotient_work":null,"direct_output_actions":times.len(),"declared_structure":if case.varying{"single_harmonic"}else{"constant"}}));
                        continue;
                    }
                    let start = Instant::now();
                    let (r, work) = match method {
                        "full-rhs-dopri5" => (
                            integrate_dopri5_observed(&full, (0., case.t), &y0, &config, &schedule)
                                .unwrap(),
                            None,
                        ),
                        "original-lawson" => {
                            let q = integrate_lawson_dopri5_observed(
                                &flow,
                                &original,
                                (0., case.t),
                                &y0,
                                &config,
                                &schedule,
                            )
                            .unwrap();
                            (q.integration, Some(q.quotient_work))
                        }
                        _ => {
                            let q = integrate_affine_lawson_dopri5_observed(
                                &flow,
                                &b,
                                &remaining,
                                (0., case.t),
                                &y0,
                                &config,
                                &schedule,
                            )
                            .unwrap();
                            (q.integration, Some(q.quotient_work))
                        }
                    };
                    let seconds = start.elapsed().as_secs_f64();
                    let error = r
                        .observed
                        .t
                        .iter()
                        .zip(&r.observed.y)
                        .flat_map(|(&t, y)| {
                            y.iter()
                                .zip(exact(t))
                                .map(|(a, b)| (a - b).abs())
                                .collect::<Vec<_>>()
                        })
                        .fold(0., f64::max);
                    rows.push(json!({"case":case.name,"dimension":case.n,"damping":case.d,"frequency":case.w,"constant_forcing":!case.varying,"method":method,"tol":tol,"repeat":repeat,"warmup":repeat==0,"seconds":seconds,"success":r.observed.success,"maximum_sampled_absolute_error":error,"attempts":r.attempts,"accepted":r.accepted_steps,"rejected":r.rejected_steps,"rhs_calls":r.observed.counters.rhs_calls,"quotient_work":work}));
                }
            }
        }
    }
    println!("{}",serde_json::to_string_pretty(&json!({"schema":"vigilode.frontier-affine-bench.v1","threads":1,"observation_count":17,"rows":rows,"scope":"constant declared forcing versus smooth time-forcing negativecontrol; matched measured errors required for speed comparisons"})).unwrap());
}
