use diffsol::{
    DenseMatrix, FaerLU, FaerMat, OdeBuilder, OdeSolverMethod, OdeSolverStopReason, Vector,
};
use serde::Serialize;
use serde_json::Value;
use std::env;
use std::fs;
use std::time::Instant;

#[derive(Serialize)]
struct Output {
    kind: &'static str,
    schema_version: u32,
    case: Option<u64>,
    method: &'static str,
    status: &'static str,
    failure_kind: Option<&'static str>,
    error: Option<String>,
    final_state: Option<Vec<f64>>,
    final_time: Option<f64>,
    solver_statistics: Option<Value>,
    elapsed_seconds: Option<f64>,
    actual_error: Option<f64>,
    error_limit: f64,
    reference_kind: &'static str,
    model_match: Option<Value>,
}

fn number(v: &Value, key: &str) -> Result<f64, String> {
    v.get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("missing/non-numeric parameter {key}"))
}
fn arr(v: &Value, key: &str) -> Result<Vec<f64>, String> {
    v.get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("missing array {key}"))?
        .iter()
        .map(|x| {
            x.as_f64()
                .ok_or_else(|| format!("non-numeric item in {key}"))
        })
        .collect()
}
fn close(a: f64, b: f64) -> bool {
    a.is_finite() && (a - b).abs() <= 32.0 * f64::EPSILON * a.abs().max(b.abs()).max(1.0)
}

trait Components {
    fn get(&self, i: usize) -> f64;
    fn set(&mut self, i: usize, x: f64);
}
impl Components for Vec<f64> {
    fn get(&self, i: usize) -> f64 {
        self[i]
    }
    fn set(&mut self, i: usize, x: f64) {
        self[i] = x;
    }
}
impl Components for diffsol::FaerVec<f64> {
    fn get(&self, i: usize) -> f64 {
        self.get_index(i)
    }
    fn set(&mut self, i: usize, x: f64) {
        self.set_index(i, x);
    }
}
fn at<V: Components>(v: &V, i: usize) -> f64 {
    v.get(i)
}
fn put<V: Components>(v: &mut V, i: usize, x: f64) {
    v.set(i, x);
}

fn rhs<V: Components>(case: u64, p: &[f64], t: f64, y: &V, o: &mut V) {
    match case {
        1 => {
            let a = p[0] * at(y, 0);
            let b = p[1] * at(y, 1) * at(y, 2);
            let c = p[2] * at(y, 1) * at(y, 1);
            put(o, 0, -a + b);
            put(o, 1, a - b - c);
            put(o, 2, c);
        }
        10 => {
            let m = [at(y, 3), at(y, 4), at(y, 5)];
            let g = (1.0 + m[0] * m[0] + m[1] * m[1] + m[2] * m[2]).sqrt();
            put(o, 0, m[0] / g);
            put(o, 1, m[1] / g);
            put(o, 2, m[2] / g);
            put(o, 3, p[0] * m[1] / g);
            put(o, 4, -p[0] * m[0] / g);
            put(o, 5, 0.0);
        }
        15 => {
            let a = 2.0 * p[2] - p[1];
            let b = p[0] * p[0] * (2.0 * p[1] * t).exp() + p[2] * p[2] - p[2] * p[1];
            put(o, 0, at(y, 1));
            put(o, 1, -a * at(y, 1) - b * at(y, 0));
        }
        16 => {
            let x = at(y, 0);
            let z = at(y, 1);
            let r2 = x * x + z * z;
            let radial = -p[0] / (r2 * r2);
            let w = p[1] / r2.powf(0.75);
            put(o, 0, radial * x - w * z);
            put(o, 1, radial * z + w * x);
        }
        _ => unreachable!(),
    }
}

fn jvp<V: Components>(case: u64, p: &[f64], t: f64, y: &V, v: &V, o: &mut V) {
    match case {
        1 => {
            let cross = at(y, 2) * at(v, 1) + at(y, 1) * at(v, 2);
            put(o, 0, -p[0] * at(v, 0) + p[1] * cross);
            put(
                o,
                1,
                p[0] * at(v, 0) - p[1] * cross - 2.0 * p[2] * at(y, 1) * at(v, 1),
            );
            put(o, 2, 2.0 * p[2] * at(y, 1) * at(v, 1));
        }
        10 => {
            let m = [at(y, 3), at(y, 4), at(y, 5)];
            let dm = [at(v, 3), at(v, 4), at(v, 5)];
            let g = (1.0 + m.iter().map(|x| x * x).sum::<f64>()).sqrt();
            let dot = m.iter().zip(dm).map(|(x, z)| x * z).sum::<f64>();
            for i in 0..3 {
                put(o, i, dm[i] / g - m[i] * dot / (g * g * g));
            }
            put(o, 3, p[0] * (dm[1] / g - m[1] * dot / (g * g * g)));
            put(o, 4, -p[0] * (dm[0] / g - m[0] * dot / (g * g * g)));
            put(o, 5, 0.0);
        }
        15 => {
            let a = 2.0 * p[2] - p[1];
            let b = p[0] * p[0] * (2.0 * p[1] * t).exp() + p[2] * p[2] - p[2] * p[1];
            put(o, 0, at(v, 1));
            put(o, 1, -b * at(v, 0) - a * at(v, 1));
        }
        16 => {
            let x = at(y, 0);
            let z = at(y, 1);
            let vx = at(v, 0);
            let vz = at(v, 1);
            let r2 = x * x + z * z;
            let dot = x * vx + z * vz;
            let radial = -p[0] / (r2 * r2);
            let w = p[1] / r2.powf(0.75);
            let dr = 4.0 * p[0] * dot / (r2 * r2 * r2);
            let dw = -1.5 * p[1] * dot / r2.powf(1.75);
            put(o, 0, radial * vx + dr * x - w * vz - dw * z);
            put(o, 1, radial * vz + dr * z + w * vx + dw * x);
        }
        _ => unreachable!(),
    }
}

fn expected_initial(case: u64) -> &'static [f64] {
    match case {
        1 => &[1., 0., 0.],
        10 => &[0., 0., 0., 1., 0., 0.2],
        15 => &[1., -0.8],
        16 => &[1., 0.],
        _ => &[],
    }
}

fn model(contract: &Value) -> Result<(u64, Vec<f64>, Vec<f64>, f64, Value), String> {
    let case = contract
        .get("case_id")
        .and_then(Value::as_u64)
        .ok_or("missing case_id")?;
    if ![1, 10, 15, 16].contains(&case) {
        return Err("unsupported case; only 1,10,15,16 are declared".into());
    }
    let start = contract
        .get("t_start")
        .and_then(Value::as_f64)
        .ok_or("missing t_start")?;
    let end = contract
        .get("t_end")
        .and_then(Value::as_f64)
        .ok_or("missing t_end")?;
    if start != 0.0 || !end.is_finite() || end <= 0.0 {
        return Err("contract requires finite positive t_end and t_start=0".into());
    }
    let y0 = arr(contract, "initial_state")?;
    if y0.len() != expected_initial(case).len()
        || !y0
            .iter()
            .zip(expected_initial(case))
            .all(|(x, y)| close(*x, *y))
    {
        return Err("initial state differs from frozen descriptor".into());
    }
    let pars = contract.get("parameters").ok_or("missing parameters")?;
    let p = match case {
        1 => {
            let x = vec![
                number(pars, "k1")?,
                number(pars, "k2")?,
                number(pars, "k3")?,
            ];
            if !close(x[0], 0.04) || !close(x[1], 1e4) || !close(x[2], 3e7) {
                return Err("case 1 parameters differ from frozen Robertson descriptor".into());
            }
            x
        }
        10 => {
            if arr(pars, "B")? != [0., 0., 1.]
                || arr(pars, "E")? != [0., 0., 0.]
                || number(pars, "c")? != 1.0
                || arr(pars, "initial_momentum")? != [1., 0., 0.2]
            {
                return Err(
                    "case 10 descriptor differs from supported uniform-field initial condition"
                        .into(),
                );
            }
            let w = number(pars, "omega_c")?;
            if !close(w, 500.) {
                return Err("case 10 omega_c differs from frozen descriptor".into());
            }
            vec![w]
        }
        15 => {
            let x = vec![
                number(pars, "omega0")?,
                number(pars, "beta")?,
                number(pars, "gamma")?,
            ];
            if !close(x[0], 1.) || !close(x[1], 1.2) || !close(x[2], 0.8) {
                return Err("case 15 parameters differ from frozen descriptor".into());
            }
            x
        }
        16 => {
            let x = vec![
                number(pars, "alpha")?,
                number(pars, "omega0")?,
                number(pars, "r0")?,
            ];
            if !close(x[0], 0.1)
                || !close(x[1], 1.)
                || !close(x[2], 1.)
                || !close(number(pars, "collision_time")?, 2.5)
                || end >= 2.5
            {
                return Err(
                    "case 16 parameters/span differ from frozen pre-collision descriptor".into(),
                );
            }
            x
        }
        _ => unreachable!(),
    };
    let probes = contract
        .get("rhs_probes")
        .and_then(Value::as_array)
        .ok_or("missing rhs_probes")?;
    if probes.len() < 2 {
        return Err("at least two RHS probes required".into());
    }
    let mut max_scaled = 0.0_f64;
    for (i, probe) in probes.iter().enumerate() {
        let t = probe
            .get("t")
            .and_then(Value::as_f64)
            .ok_or("probe missing t")?;
        let state = arr(probe, "state")?;
        let expected = arr(probe, "rhs")?;
        if state.len() != y0.len() || expected.len() != y0.len() {
            return Err("probe dimension mismatch".into());
        }
        let mut actual = vec![0.; y0.len()];
        rhs(case, &p, t, &state, &mut actual);
        for (a, e) in actual.iter().zip(expected) {
            let d = (a - e).abs() / 1.0_f64.max(a.abs()).max(e.abs());
            max_scaled = max_scaled.max(d);
            if !d.is_finite() || d > 128.0 * f64::EPSILON {
                return Err(format!("RHS probe {i} mismatch: scaled difference {d:e}"));
            }
        }
    }
    let evidence = serde_json::json!({"status":"MATCHED_AT_DECLARED_PROBES","probe_count":probes.len(),"max_scaled_probe_difference":max_scaled,"scope":"descriptor parameters and initial state plus supplied RHS probes; not a formal equivalence proof"});
    Ok((case, p, y0, end, evidence))
}

fn exact(case: u64, p: &[f64], t: f64) -> Option<Vec<f64>> {
    match case {
        1 => None,
        10 => {
            let g = 2.04_f64.sqrt();
            let phase = p[0] * t / g;
            Some(vec![
                phase.sin() / p[0],
                (phase.cos() - 1.0) / p[0],
                0.2 * t / g,
                phase.cos(),
                -phase.sin(),
                0.2,
            ])
        }
        15 => {
            let w = p[0] * (p[1] * t).exp();
            let phase = p[0] / p[1] * (p[1] * t).exp_m1();
            let a = (-p[2] * t).exp();
            Some(vec![
                a * phase.cos(),
                a * (-p[2] * phase.cos() - w * phase.sin()),
            ])
        }
        16 => {
            let r = (1.0 - 4.0 * p[0] * t).powf(0.25);
            let phase = p[1] * (1.0 - r.powf(2.5)) / (2.5 * p[0]);
            Some(vec![r * phase.cos(), r * phase.sin()])
        }
        _ => None,
    }
}

fn execute(c: &Value, rtol: f64, atol: f64, error_limit: f64) -> Output {
    let parsed = model(c);
    let (case, p, y0, end, match_evidence) = match parsed {
        Ok(x) => x,
        Err(e) => {
            return Output {
                kind: "result",
                schema_version: 1,
                case: c.get("case_id").and_then(Value::as_u64),
                method: "diffsol-bdf",
                status: "MODEL_REJECTED",
                failure_kind: Some("contract"),
                error: Some(e),
                final_state: None,
                final_time: None,
                solver_statistics: None,
                elapsed_seconds: None,
                actual_error: None,
                error_limit,
                reference_kind: "none",
                model_match: None,
            };
        }
    };
    let began = Instant::now();
    let result = (|| -> Result<(Vec<f64>, f64, Value, f64), String> {
        let dim = y0.len();
        let problem = OdeBuilder::<FaerMat<f64>>::new()
            .p(p.clone())
            .rtol(rtol)
            .atol(vec![atol; dim])
            .rhs_implicit(
                {
                    let pp = p.clone();
                    move |y, _pars, t, out| rhs(case, &pp, t, y, out)
                },
                {
                    let pp = p.clone();
                    move |y, _pars, t, v, out| jvp(case, &pp, t, y, v, out)
                },
            )
            .init(
                move |_pars, _t, y| {
                    for (i, src) in y0.iter().enumerate() {
                        y.set_index(i, *src);
                    }
                },
                dim,
            )
            .build()
            .map_err(|e| e.to_string())?;
        let mut solver = problem.bdf::<FaerLU<f64>>().map_err(|e| e.to_string())?;
        let (matrix, reason) = solver.solve_dense(&[end]).map_err(|e| e.to_string())?;
        let elapsed = began.elapsed().as_secs_f64();
        if !matches!(reason, OdeSolverStopReason::TstopReached) {
            return Err(format!("solver stopped before endpoint: {reason:?}"));
        }
        let state = (0..dim).map(|i| matrix.get_index(i, 0)).collect::<Vec<_>>();
        let stats = serde_json::to_value(solver.get_statistics()).unwrap_or(Value::Null);
        let final_time = solver.state().t; // Diffsol accepted state time, checked below against requested endpoint.
        Ok((state, final_time, stats, elapsed))
    })();
    match result {
        Ok((state, t, stats, elapsed)) => {
            let reference = exact(case, &p, end);
            let error = reference.as_ref().map(|r| {
                state
                    .iter()
                    .zip(r)
                    .map(|(x, y)| (x - y).abs())
                    .fold(0.0, f64::max)
            });
            let finite = state.iter().all(|x| x.is_finite()) && t.is_finite();
            let reached = (t - end).abs() <= 16.0 * f64::EPSILON * end.abs().max(1.0);
            let status = if !finite || !reached {
                "SOLVER_FAILED"
            } else if error.is_none() {
                "UNVALIDATED_REFERENCE"
            } else if error.unwrap() <= error_limit {
                "PASS_SCOPED"
            } else {
                "ACCURACY_FAILED"
            };
            Output {
                kind: "result",
                schema_version: 1,
                case: Some(case),
                method: "diffsol-bdf",
                status,
                failure_kind: if status == "SOLVER_FAILED" {
                    Some("solver")
                } else {
                    None
                },
                error: if !reached {
                    Some(format!(
                        "final time {t} did not reach requested endpoint {end}"
                    ))
                } else {
                    None
                },
                final_state: Some(state),
                final_time: Some(t),
                solver_statistics: Some(stats),
                elapsed_seconds: Some(elapsed),
                actual_error: error,
                error_limit,
                reference_kind: if case == 1 {
                    "none"
                } else {
                    "analytic endpoint"
                },
                model_match: Some(match_evidence),
            }
        }
        Err(e) => Output {
            kind: "result",
            schema_version: 1,
            case: Some(case),
            method: "diffsol-bdf",
            status: "SOLVER_FAILED",
            failure_kind: Some("solver"),
            error: Some(e),
            final_state: None,
            final_time: None,
            solver_statistics: None,
            elapsed_seconds: Some(began.elapsed().as_secs_f64()),
            actual_error: None,
            error_limit,
            reference_kind: if case == 1 {
                "none"
            } else {
                "analytic endpoint"
            },
            model_match: Some(match_evidence),
        },
    }
}

fn main() {
    let mut contract = None;
    let mut rtol: Option<f64> = None;
    let mut atol: Option<f64> = None;
    let mut error_limit: Option<f64> = None;
    let mut args = env::args().skip(1);
    while let Some(k) = args.next() {
        let v = args.next().unwrap_or_default();
        match k.as_str() {
            "--contract" => contract = Some(v),
            "--rtol" => rtol = v.parse().ok(),
            "--atol" => atol = v.parse().ok(),
            "--error-limit" => error_limit = v.parse().ok(),
            _ => {
                eprintln!("unknown option {k}");
                std::process::exit(2)
            }
        }
    }
    let (Some(path), Some(rtol), Some(atol), Some(error_limit)) =
        (contract, rtol, atol, error_limit)
    else {
        eprintln!(
            "usage: diffsol-local-frontier --contract JSON --rtol X --atol X --error-limit X"
        );
        std::process::exit(2)
    };
    let out = if [rtol, atol, error_limit]
        .iter()
        .all(|x| x.is_finite() && *x > 0.0)
    {
        match fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|s| serde_json::from_str::<Value>(&s).map_err(|e| e.to_string()))
        {
            Ok(c) => execute(&c, rtol, atol, error_limit),
            Err(e) => Output {
                kind: "result",
                schema_version: 1,
                case: None,
                method: "diffsol-bdf",
                status: "CONTRACT_READ_FAILED",
                failure_kind: Some("contract"),
                error: Some(e),
                final_state: None,
                final_time: None,
                solver_statistics: None,
                elapsed_seconds: None,
                actual_error: None,
                error_limit,
                reference_kind: "none",
                model_match: None,
            },
        }
    } else {
        Output {
            kind: "result",
            schema_version: 1,
            case: None,
            method: "diffsol-bdf",
            status: "INVALID_TOLERANCE",
            failure_kind: Some("arguments"),
            error: Some("rtol, atol and error-limit must be finite and positive".into()),
            final_state: None,
            final_time: None,
            solver_statistics: None,
            elapsed_seconds: None,
            actual_error: None,
            error_limit,
            reference_kind: "none",
            model_match: None,
        }
    };
    println!("{}", serde_json::to_string(&out).unwrap());
}
