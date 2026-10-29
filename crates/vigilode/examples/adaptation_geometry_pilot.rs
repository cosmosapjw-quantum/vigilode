//! Actual native DOPRI5 solves of the published nonlinear relaxation case.
//! Setup, callback construction, integration, and final reconstruction are timed.
use serde_json::json;
use std::time::Instant;
use vigilode::{Atol, CallbackError, Dopri5, ExponentialCofactor, Options, Problem};

fn physical_problem(eps: f64) -> Problem {
    Problem::new(2, move |t, y, out| {
        let f = -y[1] / (1.0 + t) + 0.2 * y[0];
        out[0] = -(y[0] - y[1].cos()) / eps - y[1].sin() * f;
        out[1] = f;
        Ok(())
    })
}
fn chart_problem(carrier: ExponentialCofactor) -> Problem {
    Problem::new(1, move |t, v, out| {
        let y = carrier
            .physical_slow(t, v[0])
            .map_err(|s| CallbackError::Fatal(s.into()))?;
        out[0] = -y / (1.0 + t) + 0.2 * y.cos();
        Ok(())
    })
}
fn lift(c: ExponentialCofactor, time: f64, v: f64) -> [f64; 2] {
    let y = c.physical_slow(time, v).unwrap();
    [y.cos() + c.value(time).unwrap(), y]
}
fn main() {
    for (eps, end) in [(1e-3, 2.0), (1e-6, 0.01), (1e-6, 2.0), (1e-9, 2.0)] {
        let carrier = ExponentialCofactor::new(0.0, -1.0_f64.cos(), 1.0 / eps, 0.2).unwrap();
        let precise = Options {
            rtol: 1e-12,
            atol: Atol::Scalar(1e-14),
            h0: eps.min(1e-4),
            max_steps: 200000,
            ..Default::default()
        };
        let reference =
            Dopri5::solve(&chart_problem(carrier), (0.0, end), &[1.0], &precise).unwrap();
        assert!(reference.success);
        let native_reference = lift(carrier, end, reference.final_state[0]);
        let (reference, reference_kind, uncertainty) = if eps == 1e-6 && end == 2.0 {
            let published: serde_json::Value = serde_json::from_str(include_str!(
                "../../../research/local_frontier_native_diffsol_20261024/stop_audit_20261025/evidence/case_references/case05/EXTERNAL_REFERENCE_PRECOMMIT.json"
            )).unwrap();
            (
                [
                    published["reference_state"][0].as_f64().unwrap(),
                    published["reference_state"][1].as_f64().unwrap(),
                ],
                "published independent Wolfram WP40/WP50 cofactor reduction",
                Some(1e-12),
            )
        } else {
            (
                native_reference,
                "same-native-method tighter reduced solve",
                None,
            )
        };
        for kind in ["literal", "cofactor"] {
            // Repeated 1e-9 literal full-span failure adds no discrimination.
            if kind == "literal" && eps < 1e-6 {
                continue;
            }
            for repeat in 0..4 {
                let options = Options {
                    rtol: 1e-6,
                    atol: Atol::Scalar(1e-9),
                    h0: 1e-4,
                    max_steps: 200000,
                    ..Default::default()
                };
                let timer = Instant::now();
                let (run, physical) = if kind == "literal" {
                    let run =
                        Dopri5::solve(&physical_problem(eps), (0.0, end), &[0.0, 1.0], &options)
                            .unwrap();
                    let physical = [run.final_state[0], run.final_state[1]];
                    (run, physical)
                } else {
                    let run = Dopri5::solve(&chart_problem(carrier), (0.0, end), &[1.0], &options)
                        .unwrap();
                    let physical = lift(carrier, run.final_time, run.final_state[0]);
                    (run, physical)
                };
                let elapsed = timer.elapsed().as_secs_f64();
                let error = run.success.then(|| {
                    physical
                        .iter()
                        .zip(reference)
                        .map(|(a, b)| (a - b).abs())
                        .fold(0.0_f64, f64::max)
                });
                println!(
                    "{}",
                    json!({"schema":"vigilode.cofactor-pilot.v1","eps":eps,"end":end,"method":kind,"repeat":repeat,"warmup":repeat==0,"elapsed_seconds":elapsed,"success":run.success,"final_time":run.final_time,"physical_state":physical,"reference_state":reference,"reference_kind":reference_kind,"reference_empirical_uncertainty":uncertainty,"native_reference_max_abs_difference":native_reference.iter().zip(reference).map(|(a,b)| (a-b).abs()).fold(0.0_f64, f64::max),"endpoint_max_abs_error":error,"attempts":run.stats.attempts,"accepted":run.stats.accepted_steps,"rejected":run.stats.rejected_steps,"rhs":run.stats.counters.rhs_calls,"jacobians":run.stats.counters.jacobian_builds,"jvp":run.stats.counters.jvp_calls,"newton":run.stats.counters.nonlinear_iterations,"scope":"declared exact cofactor structure; no automatic structure discovery"})
                );
            }
        }
    }
}
