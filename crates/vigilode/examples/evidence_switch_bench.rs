//! Bounded work-selection pilot. All timings exclude model construction and output.
use serde_json::json;
use std::time::Instant;
use vigilode::{
    Atol, DerivativeFallback, Dopri5, Jacobian, Options, Problem, Rodas5p, SwitchingConfig,
    SwitchingSolver,
};
fn problem(case: &str) -> Problem {
    match case {
        "rotation" => Problem::new(2, |_, y, o| {
            o[0] = -100.0 * y[1];
            o[1] = 100.0 * y[0];
            Ok(())
        })
        .jacobian(Jacobian::jvp(|_, _, v, o| {
            o[0] = -100.0 * v[1];
            o[1] = 100.0 * v[0];
            Ok(())
        }))
        .autonomous(),
        "decay" => Problem::new(1, |_, y, o| {
            o[0] = -y[0];
            Ok(())
        })
        .jacobian(Jacobian::dense(|_, _, j| {
            j[0] = -1.0;
            Ok(())
        }))
        .autonomous(),
        "stiff-forced" => Problem::new(1, |t, y, o| {
            o[0] = -10000.0 * (y[0] - t.cos()) - t.sin();
            Ok(())
        })
        .jacobian(Jacobian::dense(|_, _, j| {
            j[0] = -10000.0;
            Ok(())
        }))
        .time_derivative(|t, _, o| {
            o[0] = -10000.0 * t.sin() - t.cos();
            Ok(())
        }),
        _ => panic!("unknown case"),
    }
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let case = args.get(1).map_or("rotation", |s| s.as_str());
    let mode = args.get(2).map_or("evidence", |s| s.as_str());
    let h0: f64 = args.get(3).map_or("1e-7", |s| s.as_str()).parse().unwrap();
    let p = problem(case);
    let y0 = if case == "rotation" {
        vec![1.0, 0.0]
    } else {
        vec![1.0]
    };
    let o = Options {
        rtol: 1e-7,
        atol: Atol::Scalar(1e-10),
        h0,
        max_steps: 20000,
        ..Default::default()
    };
    let base = SwitchingConfig::evidence_driven(DerivativeFallback::ProvidedOnly);
    let start = Instant::now();
    let (success, state, work, seconds, detail) = match mode {
        "dopri" => {
            let r = Dopri5::solve(&p, (0.0, 1.0), &y0, &o).unwrap();
            let s = start.elapsed().as_secs_f64();
            (
                r.success,
                r.final_state.clone(),
                r.stats.counters.rhs_calls,
                s,
                json!(r),
            )
        }
        "rodas" => {
            let r = Rodas5p::solve(&p, (0.0, 1.0), &y0, &o).unwrap();
            let s = start.elapsed().as_secs_f64();
            (
                r.is_success(),
                r.final_state().unwrap().to_vec(),
                r.stats.counters.rhs_calls,
                s,
                json!(r),
            )
        }
        "evidence" | "screened-reset" | "pressure-only" => {
            let mut c = base;
            if mode == "screened-reset" {
                c.carry_explicit_step = false;
            }
            if mode == "pressure-only" {
                c = SwitchingConfig {
                    fallback: DerivativeFallback::ProvidedOnly,
                    ..Default::default()
                };
            }
            let r = SwitchingSolver::solve(&p, (0.0, 1.0), &y0, &o, &c).unwrap();
            let s = start.elapsed().as_secs_f64();
            (
                r.success,
                r.final_state.clone(),
                r.stats.rhs_calls,
                s,
                json!(r),
            )
        }
        _ => panic!("unknown mode"),
    };
    let exact = match case {
        "rotation" => vec![100.0_f64.cos(), 100.0_f64.sin()],
        "decay" => vec![(-1.0_f64).exp()],
        _ => vec![1.0_f64.cos()],
    };
    let error = state
        .iter()
        .zip(exact)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f64, f64::max);
    println!(
        "{}",
        json!({"schema":"vigilode.evidence-switch-pilot.v1","case":case,"mode":mode,"h0":h0,
        "success":success,"endpoint_max_abs_error":error,"accuracy_cap":1e-5,
        "accuracy_pass":success&&error<=1e-5,"rhs_calls":work,"seconds":seconds,"record":detail,
        "timing_scope":"solve including workspace and pilots; excludes model construction and output",
        "interpretation":"short smoke comparison at common error cap, not matched-achieved-error ranking"})
    );
}
