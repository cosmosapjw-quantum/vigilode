//! Measurement program of loop L1 node V09_LAGGED_LARGE_N (`../PREDECLARED.json`).
//!
//! It runs the protected fast dense driver and the existing lagged-LU driver on the node's
//! problem families and prints one JSON object per line. No number is interpreted here:
//! `../parse_results.py` does that from the raw logs.
//!
//! Subcommands:
//! * `list`: the cells (family, size, rtol);
//! * `selftest [--family F]`: problem checks (Jacobian against central differences, JVP against
//!   the Jacobian, pattern stability of the dense fill, density and the LU the protected policy
//!   picks, spectral identity of acspec);
//! * `rhs0 --family F --size S`: start state and right-hand sides in hex (consistency with the
//!   independent numpy definitions of the reference);
//! * `setup --family F --size S --arm A`: everything of `run` except the integrations (the
//!   baseline that `perf stat` counts are reduced by);
//! * `run --family F --size S --rtol R --arm A [--warmups W] [--reps K] [--state]`: `W + K`
//!   integrations of one arm in this process.

use std::io::Write as _;

use rodas5p_core::{CoreResult, DenseMatrix};
use serde_json::{Value, json};
use v09::harness::{
    Arm, Callbacks, RTOLS, build_problem, cpus_allowed, hex, identity_key, loadavg, problem_json,
    record_json, run_once, unix_time,
};
use v09::problems::{FAMILIES, ProblemDef, definition, dense_from_jvp, sizes};

fn fail(message: &str) -> ! {
    eprintln!("v09-runner: {message}");
    std::process::exit(2)
}

fn emit(value: Value) {
    let mut out = std::io::stdout().lock();
    writeln!(out, "{value}").expect("stdout");
    out.flush().expect("stdout");
}

struct Args {
    family: Option<String>,
    size: Option<usize>,
    rtol: Option<f64>,
    arm: Option<Arm>,
    warmups: usize,
    reps: usize,
    state: bool,
}

fn parse_args(rest: &[String]) -> Args {
    let mut args = Args {
        family: None,
        size: None,
        rtol: None,
        arm: None,
        warmups: 0,
        reps: 1,
        state: false,
    };
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        let mut value = || {
            it.next()
                .unwrap_or_else(|| fail(&format!("{flag} needs a value")))
                .clone()
        };
        match flag.as_str() {
            "--family" => args.family = Some(value()),
            "--size" => {
                let text = value();
                args.size = Some(
                    text.parse()
                        .unwrap_or_else(|_| fail(&format!("not a size: {text}"))),
                );
            }
            "--rtol" => {
                let text = value();
                args.rtol = Some(
                    text.parse()
                        .unwrap_or_else(|_| fail(&format!("not a tolerance: {text}"))),
                );
            }
            "--arm" => {
                let text = value();
                args.arm = Some(Arm::parse(&text).unwrap_or_else(|| fail(&format!("arm {text}"))));
            }
            "--warmups" => {
                let text = value();
                args.warmups = text
                    .parse()
                    .unwrap_or_else(|_| fail(&format!("not a count: {text}")));
            }
            "--reps" => {
                let text = value();
                args.reps = text
                    .parse()
                    .unwrap_or_else(|_| fail(&format!("not a count: {text}")));
            }
            "--state" => args.state = true,
            other => fail(&format!("unknown flag {other}")),
        }
    }
    args
}

fn require_def(args: &Args) -> ProblemDef {
    let family = args
        .family
        .as_deref()
        .unwrap_or_else(|| fail("--family is required"));
    let size = args.size.unwrap_or_else(|| fail("--size is required"));
    if !sizes(family).contains(&size) {
        fail(&format!("{family} has no declared size {size}"));
    }
    definition(family, size).unwrap_or_else(|| fail(&format!("unknown family {family}")))
}

fn header(mode: &str, def: &ProblemDef, extra: Value) -> Value {
    json!({
        "type": "header",
        "mode": mode,
        "problem": problem_json(def),
        "extra": extra,
        "pid": std::process::id(),
        "cpus_allowed_list": cpus_allowed(),
        "loadavg_start": loadavg(),
        "unix_time_start": unix_time(),
    })
}

fn footer() -> Value {
    json!({
        "type": "footer",
        "loadavg_end": loadavg(),
        "unix_time_end": unix_time(),
    })
}

fn command_list() {
    for family in FAMILIES {
        for &size in sizes(family) {
            let def = definition(family, size).expect("declared family");
            for rtol in RTOLS {
                emit(json!({
                    "type": "cell",
                    "family": family,
                    "size": size,
                    "n": def.n,
                    "rtol": rtol,
                    "atol": rtol,
                    "t_span": [def.t_span.0, def.t_span.1],
                    "structure": def.structure,
                }));
            }
        }
    }
}

/// max_ij |a_ij - b_ij| / max_ij |b_ij|.
fn relative_max_difference(a: &DenseMatrix, b: &DenseMatrix) -> f64 {
    let scale = b
        .as_slice()
        .iter()
        .fold(0.0_f64, |m, v| m.max(v.abs()))
        .max(f64::MIN_POSITIVE);
    a.as_slice()
        .iter()
        .zip(b.as_slice())
        .fold(0.0_f64, |m, (x, y)| m.max((x - y).abs()))
        / scale
}

fn selftest_one(def: &ProblemDef) -> CoreResult<Value> {
    let n = def.n;
    let t0 = def.t_span.0;
    // A second state: y0 with a deterministic relative perturbation of 1 %.
    let y1: Vec<f64> = def
        .y0
        .iter()
        .enumerate()
        .map(|(i, v)| v * (1.0 + 0.01 * (i as f64 * 0.7).sin()) + 1.0e-3 * (i as f64).cos())
        .collect();
    let mut jac = DenseMatrix::zeros(n, n);
    (def.dense_fill)(&y1, &mut jac);
    // Central differences of the right-hand side, step 1e-6 max(1, |y_j|).
    let mut fd = DenseMatrix::zeros(n, n);
    let (mut plus, mut minus) = (vec![0.0; n], vec![0.0; n]);
    let mut y = y1.clone();
    for j in 0..n {
        let step = 1.0e-6 * y1[j].abs().max(1.0);
        y[j] = y1[j] + step;
        (def.rhs)(t0, &y, &mut plus)?;
        y[j] = y1[j] - step;
        (def.rhs)(t0, &y, &mut minus)?;
        y[j] = y1[j];
        for i in 0..n {
            fd[(i, j)] = (plus[i] - minus[i]) / (2.0 * step);
        }
    }
    let fd_error = relative_max_difference(&fd, &jac);
    let by_jvp = dense_from_jvp(def, &y1)?;
    let jvp_error = relative_max_difference(&by_jvp, &jac);
    // Pattern stability: a fill over a matrix filled at y0 equals a fill into zeros.
    let mut reused = DenseMatrix::zeros(n, n);
    (def.dense_fill)(&def.y0, &mut reused);
    (def.dense_fill)(&y1, &mut reused);
    let pattern_stable = reused
        .as_slice()
        .iter()
        .zip(jac.as_slice())
        .all(|(a, b)| a.to_bits() == b.to_bits());
    let mut at_y0 = DenseMatrix::zeros(n, n);
    (def.dense_fill)(&def.y0, &mut at_y0);
    let nonzero = at_y0.as_slice().iter().filter(|v| **v != 0.0).count();
    let density = nonzero as f64 / (n * n) as f64;
    // The rule of Workspace::choose_lu (rodas5p_fast.rs): n <= 64 or density <= 0.1 -> in place.
    let expected_lu = if n <= 64 || density <= 0.1 {
        "InPlaceZeroSkipping"
    } else {
        "Faer"
    };
    let mut extra = json!({});
    if def.family == "acspec" {
        // nu D2 cos(x) = -nu cos(x) for the dense Fourier matrix (exact for |k| < n/2).
        let mut probe = vec![0.0; n];
        let mut out = vec![0.0; n];
        let mut zero_state = vec![0.0; n];
        for (j, p) in probe.iter_mut().enumerate() {
            *p = (2.0 * std::f64::consts::PI * j as f64 / n as f64).cos();
        }
        // With u = 0 the JVP is nu D2 v + v.
        zero_state.iter_mut().for_each(|v| *v = 0.0);
        (def.jvp)(t0, &zero_state, &probe, &mut out)?;
        let nu = v09::problems::ACSPEC_NU;
        let error = out
            .iter()
            .zip(&probe)
            .fold(0.0_f64, |m, (o, p)| m.max((o - (p - nu * p)).abs()));
        extra = json!({ "spectral_cos_identity_max_error": error });
    }
    Ok(json!({
        "type": "selftest",
        "problem": problem_json(def),
        "jacobian_vs_central_differences_rel_max": fd_error,
        "jacobian_by_jvp_vs_fill_rel_max": jvp_error,
        "fill_pattern_stable": pattern_stable,
        "jacobian_nonzeros_at_y0": nonzero,
        "jacobian_density_at_y0": density,
        "lu_the_protected_policy_picks": expected_lu,
        "y0_finite": def.y0.iter().all(|v| v.is_finite()),
        "extra": extra,
    }))
}

fn command_selftest(args: &Args) -> CoreResult<()> {
    for family in FAMILIES {
        if args.family.as_deref().is_some_and(|f| f != family) {
            continue;
        }
        for &size in sizes(family) {
            if args.size.is_some_and(|s| s != size) {
                continue;
            }
            let def = definition(family, size).expect("declared family");
            emit(selftest_one(&def)?);
        }
    }
    Ok(())
}

fn command_rhs0(args: &Args) -> CoreResult<()> {
    let def = require_def(args);
    let n = def.n;
    let y1: Vec<f64> = def
        .y0
        .iter()
        .enumerate()
        .map(|(i, v)| v * (1.0 + 0.01 * (i as f64 * 0.7).sin()) + 1.0e-3 * (i as f64).cos())
        .collect();
    let (mut f0, mut f1) = (vec![0.0; n], vec![0.0; n]);
    (def.rhs)(def.t_span.0, &def.y0, &mut f0)?;
    (def.rhs)(def.t_span.0, &y1, &mut f1)?;
    emit(json!({
        "type": "rhs0",
        "problem": problem_json(&def),
        "t0_hex": format!("{:016x}", def.t_span.0.to_bits()),
        "t1_hex": format!("{:016x}", def.t_span.1.to_bits()),
        "y0_hex": hex(&def.y0),
        "f0_hex": hex(&f0),
        "y1_hex": hex(&y1),
        "f1_hex": hex(&f1),
    }));
    Ok(())
}

fn command_setup(args: &Args) -> CoreResult<()> {
    let def = require_def(args);
    let arm = args.arm.unwrap_or_else(|| fail("--arm is required"));
    emit(header(
        "setup",
        &def,
        json!({ "arm": arm.id(), "arm_description": arm.description() }),
    ));
    let callbacks = Callbacks::plain(&def, arm.uses_jvp());
    let problem = build_problem(&def, &callbacks, arm)?;
    std::hint::black_box(&problem);
    emit(footer());
    Ok(())
}

fn command_run(args: &Args) -> CoreResult<()> {
    let def = require_def(args);
    let arm = args.arm.unwrap_or_else(|| fail("--arm is required"));
    let rtol = args.rtol.unwrap_or_else(|| fail("--rtol is required"));
    if args.reps == 0 {
        fail("--reps must be positive");
    }
    emit(header(
        "run",
        &def,
        json!({
            "arm": arm.id(),
            "arm_description": arm.description(),
            "lagged_options": arm.lagged_options().map(|o| format!("{o:?}")),
            "jvp_attached": arm.uses_jvp(),
            "rtol": rtol,
            "atol": rtol,
            "initial_step": v09::harness::INITIAL_STEP,
            "min_step": v09::harness::MIN_STEP,
            "max_attempts": v09::harness::MAX_ATTEMPTS,
            "warmups": args.warmups,
            "reps": args.reps,
        }),
    ));
    let callbacks = Callbacks::plain(&def, arm.uses_jvp());
    let problem = build_problem(&def, &callbacks, arm)?;
    let total = args.warmups + args.reps;
    let mut first_key = None;
    let mut deterministic = true;
    let mut last = None;
    for index in 0..total {
        let record = run_once(arm, &problem, &def, rtol)?;
        let key = identity_key(&record);
        match &first_key {
            None => first_key = Some(key.clone()),
            Some(first) => deterministic &= *first == key,
        }
        emit(json!({
            "type": "integration",
            "index": index,
            "warmup": index < args.warmups,
            "wall_ns": record.wall_ns,
            "y_end_fnv64": key.0,
            "attempts": record.attempts,
            "accepted_steps": record.accepted,
            "direct_factorizations": record.counters.direct_factorizations,
            "loadavg_after": loadavg(),
        }));
        last = Some(record);
    }
    let last = last.expect("at least one integration");
    let mut result = record_json(&last, args.state);
    result["type"] = json!("result");
    result["arm"] = json!(arm.id());
    result["rtol"] = json!(rtol);
    result["deterministic_across_integrations"] = json!(deterministic);
    emit(result);
    emit(footer());
    Ok(())
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let command = argv.get(1).map(String::as_str).unwrap_or("");
    let args = parse_args(argv.get(2..).unwrap_or(&[]));
    let outcome = match command {
        "list" => {
            command_list();
            Ok(())
        }
        "selftest" => command_selftest(&args),
        "rhs0" => command_rhs0(&args),
        "setup" => command_setup(&args),
        "run" => command_run(&args),
        _ => fail("usage: v09-runner list | selftest | rhs0 | setup | run [flags]"),
    };
    if let Err(error) = outcome {
        emit(json!({ "type": "error", "error": error.to_string() }));
        std::process::exit(1);
    }
}
