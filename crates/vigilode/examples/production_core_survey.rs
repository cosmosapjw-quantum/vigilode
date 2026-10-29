//! Deterministic endpoint correctness survey of the conservative GeneralOde API.
//! No wall-clock benchmark, external comparison, global-error certificate or
//! unrestricted production qualification is inferred from these small systems.
use serde_json::json;
use vigilode::{
    Atol, DerivativeFallback, GeneralOde, GeneralOdePolicy, GeneralResult, Jacobian, Options,
    Problem, ProblemRegime,
};

struct Case {
    id: &'static str,
    problem: Problem,
    initial: Vec<f64>,
    end: f64,
    regime: ProblemRegime,
    reference: Box<dyn Fn(f64) -> Vec<f64>>,
    description: &'static str,
}

fn linear_two(a: f64, b: f64, c: f64, d: f64) -> Problem {
    Problem::new(2, move |_, y, out| {
        out[0] = a * y[0] + b * y[1];
        out[1] = c * y[0] + d * y[1];
        Ok(())
    })
    .jacobian(Jacobian::dense(move |_, _, j| {
        j.copy_from_slice(&[a, b, c, d]);
        Ok(())
    }))
    .autonomous()
}

fn cases() -> Vec<Case> {
    let mut cases = vec![
        Case {
            id: "scalar_decay",
            regime: ProblemRegime::Nonstiff,
            problem: Problem::new(1, |_, y, out| {
                out[0] = -y[0];
                Ok(())
            })
            .jacobian(Jacobian::dense(|_, _, j| {
                j[0] = -1.0;
                Ok(())
            }))
            .autonomous(),
            initial: vec![1.0],
            end: 2.0,
            reference: Box::new(|t| vec![(-t).exp()]),
            description: "y'=-y; y(0)=1; exact exp(-t)",
        },
        Case {
            id: "nonautonomous_tracking_cos",
            regime: ProblemRegime::Nonstiff,
            problem: Problem::new(1, |t, y, out| {
                out[0] = -5.0 * (y[0] - t.cos()) - t.sin();
                Ok(())
            })
            .jacobian(Jacobian::dense(|_, _, j| {
                j[0] = -5.0;
                Ok(())
            }))
            .time_derivative(|t, _, ft| {
                ft[0] = -5.0 * t.sin() - t.cos();
                Ok(())
            }),
            initial: vec![1.0],
            end: 2.0,
            reference: Box::new(|t| vec![t.cos()]),
            description: "y'=-5(y-cos(t))-sin(t); y(0)=1; exact cos(t)",
        },
        Case {
            id: "stiff_diagonal",
            regime: ProblemRegime::Stiff,
            problem: linear_two(-1.0, 0.0, 0.0, -1000.0),
            initial: vec![1.0, 1.0],
            end: 2.0,
            reference: Box::new(|t| vec![(-t).exp(), (-1000.0 * t).exp()]),
            description: "diag(-1,-1000); y(0)=(1,1); analytic exponential; fast mode underflows at endpoint",
        },
        Case {
            id: "nonnormal_stable_jordan",
            regime: ProblemRegime::Stiff,
            problem: linear_two(-2.0, 30.0, 0.0, -2.0),
            initial: vec![0.0, 1.0],
            end: 2.0,
            reference: Box::new(|t| vec![30.0 * t * (-2.0 * t).exp(), (-2.0 * t).exp()]),
            description: "A=[-2,30;0,-2]; y(0)=(0,1); (30t,1)exp(-2t); implicit declared for route coverage, not a stiffness diagnosis",
        },
        Case {
            id: "stiff_nonlinear_tracking_cos",
            regime: ProblemRegime::Stiff,
            problem: Problem::new(1, |t, y, out| {
                let delta = y[0] - t.cos();
                out[0] = -1000.0 * delta - 10.0 * delta * delta * delta - t.sin();
                Ok(())
            })
            .jacobian(Jacobian::dense(|t, y, j| {
                let delta = y[0] - t.cos();
                j[0] = -1000.0 - 30.0 * delta * delta;
                Ok(())
            }))
            .time_derivative(|t, y, ft| {
                let delta = y[0] - t.cos();
                ft[0] = -(1000.0 + 30.0 * delta * delta) * t.sin() - t.cos();
                Ok(())
            }),
            initial: vec![1.0],
            end: 2.0,
            reference: Box::new(|t| vec![t.cos()]),
            description: "delta=y-cos(t); y'=-1000delta-10delta^3-sin(t); exact cos(t); manufactured exact orbit alone does not test arbitrary nonlinear excursions",
        },
        Case {
            id: "nonstiff_riccati",
            regime: ProblemRegime::Nonstiff,
            problem: Problem::new(1, |_, y, out| {
                out[0] = -y[0] * y[0];
                Ok(())
            })
            .jacobian(Jacobian::dense(|_, y, j| {
                j[0] = -2.0 * y[0];
                Ok(())
            }))
            .autonomous(),
            initial: vec![1.0],
            end: 2.0,
            reference: Box::new(|t| vec![1.0 / (1.0 + t)]),
            description: "y'=-y^2; y(0)=1; exact 1/(1+t)",
        },
    ];
    for (id, damping, description) in [
        (
            "rapid_rotation",
            0.0_f64,
            "A=[0,-100;100,0]; endpoint at200 radians; no continuous phase claim",
        ),
        (
            "damped_rapid_rotation",
            0.25_f64,
            "A=[-0.25,-100;100,-0.25]; endpoint at200 radians; exact exp(-0.25t)(cos(100t),sin(100t))",
        ),
    ] {
        let omega = 100.0_f64;
        cases.push(Case {
            id,
            problem: linear_two(-damping, -omega, omega, -damping),
            initial: vec![1.0, 0.0],
            end: 2.0,
            regime: ProblemRegime::Stiff,
            reference: Box::new(move |t| {
                let amplitude = (-damping * t).exp();
                vec![amplitude * (omega * t).cos(), amplitude * (omega * t).sin()]
            }),
            description,
        });
    }
    cases
}

fn main() {
    let tolerances = [1e-4_f64, 1e-6, 1e-8];
    println!(
        "{}",
        json!({
            "kind": "plan", "schema": "vigilode.production-core-survey.v1",
            "cases": 8, "tolerances": tolerances, "atol_rule": "rtol*1e-3",
            "max_attempts": 100000, "initial_step": 1e-3,
            "endpoint_acceptance": "max_abs_error <=100*(atol+rtol*max_abs_reference_endpoint)",
            "reference": "analytic formulas evaluated in binary64; no rigorous floating-point enclosure",
            "timing": "NOT_MEASURED", "external_comparisons": "NOT_RUN", "threads": 1,
            "limitations": ["endpoint accuracy only", "small manufactured systems", "no universal production or performance claim", "no NCP"]
        })
    );
    let mut passed = 0_usize;
    let mut total = 0_usize;
    let mut trends = Vec::new();
    for case in cases() {
        let mut errors = Vec::new();
        let reference = (case.reference)(case.end);
        let reference_norm = reference.iter().fold(0.0_f64, |a, x| a.max(x.abs()));
        for rtol in tolerances {
            total += 1;
            let atol = rtol * 1e-3;
            let cap = 100.0 * (atol + rtol * reference_norm);
            let options = Options {
                rtol,
                atol: Atol::Scalar(atol),
                h0: 1e-3,
                max_steps: 100000,
                ..Options::default()
            };
            let policy = GeneralOdePolicy {
                regime: case.regime,
                derivatives: if case.regime == ProblemRegime::Stiff {
                    DerivativeFallback::ProvidedOnly
                } else {
                    DerivativeFallback::Forbidden
                },
                ..GeneralOdePolicy::default()
            };
            match GeneralOde::solve(
                &case.problem,
                (0.0, case.end),
                &case.initial,
                &options,
                &policy,
            ) {
                Ok(solution) => {
                    let success = solution.is_success();
                    let final_state = solution.final_state().map(|y| y.to_vec());
                    let finite = final_state.as_ref().is_some_and(|y| {
                        y.len() == reference.len() && y.iter().all(|x| x.is_finite())
                    });
                    let error = if finite {
                        final_state.as_ref().map(|y| {
                            y.iter()
                                .zip(&reference)
                                .map(|(actual, exact)| (actual - exact).abs())
                                .fold(0.0_f64, f64::max)
                        })
                    } else {
                        None
                    };
                    let pass = success && error.is_some_and(|value| value <= cap);
                    if pass {
                        passed += 1;
                    }
                    errors.push(error);
                    let diagnostics = match &solution.result {
                        GeneralResult::Explicit(s) => json!({
                            "final_time": s.final_time, "attempts": s.stats.attempts,
                            "accepted": s.stats.accepted_steps, "rejected": s.stats.rejected_steps,
                            "counters": s.stats.counters, "stop_reason": s.stop_reason,
                            "callback_failure": s.callback_failure, "message": s.message
                        }),
                        GeneralResult::Implicit(s) => json!({
                            "last_output_time": s.t.last(), "attempts": s.stats.attempts,
                            "accepted": s.stats.accepted_steps, "rejected": s.stats.rejected_steps,
                            "counters": s.stats.counters, "stop": s.stats.stop,
                            "callbacks": s.stats.callbacks, "message": s.stats.driver_message
                        }),
                    };
                    println!(
                        "{}",
                        json!({
                            "kind": "row", "case": case.id, "description": case.description,
                            "rtol": rtol, "atol": atol, "end": case.end, "policy": policy,
                            "selection": solution.selection, "status": if pass { "PASS_ENDPOINT" } else { "FAIL" },
                            "solver_success": success, "finite_state": finite,
                            "max_abs_error": error, "predeclared_error_cap": cap,
                            "reference_endpoint": reference, "actual_endpoint_or_partial": final_state,
                            "diagnostics": diagnostics
                        })
                    );
                }
                Err(error) => {
                    errors.push(None);
                    println!(
                        "{}",
                        json!({
                            "kind": "row", "case": case.id, "description": case.description,
                            "rtol": rtol, "atol": atol, "end": case.end, "policy": policy,
                            "status": "ERROR", "error": format!("{error:?}"),
                            "predeclared_error_cap": cap, "reference_endpoint": reference,
                            "max_abs_error": null, "counters": null,
                            "note": "initialization/driver error has no GeneralSolution; error is preserved without success fabrication"
                        })
                    );
                }
            }
        }
        let roundoff_floor = 64.0 * f64::EPSILON * (1.0 + reference_norm);
        let nonincreasing_with_slack = errors.windows(2).all(|pair| match (pair[0], pair[1]) {
            (Some(loose), Some(tight)) => tight <= 1.1 * loose + roundoff_floor,
            _ => false,
        });
        trends.push(json!({ "case": case.id, "endpoint_errors_loose_to_tight": errors,
            "nonincreasing_with_10pct_slack_and_roundoff_floor": nonincreasing_with_slack,
            "roundoff_floor": roundoff_floor,
            "interpretation": "diagnostic only; adaptive endpoint cancellation can make error nonmonotone" }));
    }
    println!(
        "{}",
        json!({
            "kind": "summary", "rows": total, "passed": passed, "failed_or_error": total-passed,
            "status": if passed == total { "PASS_SCOPED_ENDPOINT_SURVEY" } else { "FAIL_PRESERVED" },
            "tolerance_trends": trends, "timing": "NOT_MEASURED", "external_comparisons": "NOT_RUN",
            "production_qualification": "NOT_ESTABLISHED_BY_THIS_SURVEY"
        })
    );
    if passed != total {
        std::process::exit(1);
    }
}
