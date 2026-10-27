//! Local CPU campaign executable. Run --list before constructing a campaign.
//! Timed solve includes callback/provider/workspace construction, but excludes
//! model construction, the reusable Rayon pool startup, diagnostics and JSON.
#[path = "local_bottleneck_support/models.rs"]
mod models;
use models::{LAYERS, METHODS, Model};
use rayon::prelude::*;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    io::{self, Write},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};
use vigilode::{
    Atol, CallbackError, DerivativeFallback, Dopri5, Jacobian, Options, Problem, Rodas5p,
    SwitchingConfig, SwitchingSolver,
};

#[derive(Default)]
struct Counters {
    rhs: AtomicU64,
    jvp: AtomicU64,
    jac: AtomicU64,
    ft: AtomicU64,
    columns: AtomicU64,
    rhs_ns: AtomicU64,
    jvp_ns: AtomicU64,
    jac_ns: AtomicU64,
    ft_ns: AtomicU64,
}
impl Counters {
    fn counts(&self) -> Value {
        let g = |a: &AtomicU64| a.load(Ordering::Relaxed);
        json!({"rhs_callbacks":g(&self.rhs),"jvp_callbacks":g(&self.jvp),"dense_jacobian_callbacks":g(&self.jac),"dense_jacobian_columns":g(&self.columns),"time_derivative_callbacks":g(&self.ft)})
    }
    fn phases(&self) -> Value {
        let g = |a: &AtomicU64| a.load(Ordering::Relaxed) as f64 * 1e-9;
        json!({"rhs":g(&self.rhs_ns),"jvp":g(&self.jvp_ns),"dense_jacobian":g(&self.jac_ns),"time_derivative":g(&self.ft_ns),"semantics":"sum of callback wall durations; includes data-parallel scheduling. Across ensemble members these overlap and are not exclusive wall-time fractions."})
    }
}
fn tick(timer: Option<Instant>, target: &AtomicU64) {
    if let Some(t) = timer {
        target.fetch_add(
            t.elapsed().as_nanos().min(u64::MAX as u128) as u64,
            Ordering::Relaxed,
        );
    }
}
fn callback(
    m: &Model,
    t: f64,
    y: &[f64],
    v: Option<&[f64]>,
    td: bool,
    out: &mut [f64],
    parallel: bool,
) -> Result<(), CallbackError> {
    if (m.id == 7 && y[0] <= 0.0)
        || (m.id == 13 && (0..m.nx).any(|i| y[i] <= 0.0 || y[2 * m.nx + i] <= 0.0))
        || (m.id == 16 && y.iter().map(|z| z * z).sum::<f64>() <= 1e-20)
    {
        return Err(CallbackError::Recoverable("model domain boundary".into()));
    }
    m.apply(t, y, v, td, out, parallel);
    if out.iter().any(|x| !x.is_finite()) {
        return Err(CallbackError::Recoverable(
            "nonfinite model evaluation".into(),
        ));
    }
    Ok(())
}
fn problem(
    m: Arc<Model>,
    counts: Arc<Counters>,
    pool: Option<Arc<rayon::ThreadPool>>,
    dense: bool,
    instrument: bool,
) -> Problem {
    let n = m.n;
    let mr = m.clone();
    let cr = counts.clone();
    let pr = pool.clone();
    let mut p = Problem::new(n, move |t, y, out| {
        cr.rhs.fetch_add(1, Ordering::Relaxed);
        let clock = instrument.then(Instant::now);
        let z = if let Some(p) = &pr {
            p.install(|| callback(&mr, t, y, None, false, out, true))
        } else {
            callback(&mr, t, y, None, false, out, false)
        };
        tick(clock, &cr.rhs_ns);
        z
    });
    if dense {
        let mj = m.clone();
        let cj = counts.clone();
        let pj = pool.clone();
        p = p.jacobian(Jacobian::dense(move |t, y, a| {
            cj.jac.fetch_add(1, Ordering::Relaxed);
            let clock = instrument.then(Instant::now);
            let mut v = vec![0.0; n];
            let mut col = vec![0.0; n];
            let mut result = Ok(());
            for j in 0..n {
                v[j] = 1.0;
                cj.columns.fetch_add(1, Ordering::Relaxed);
                result = if let Some(p) = &pj {
                    p.install(|| callback(&mj, t, y, Some(&v), false, &mut col, true))
                } else {
                    callback(&mj, t, y, Some(&v), false, &mut col, false)
                };
                if result.is_err() {
                    break;
                }
                for i in 0..n {
                    a[i * n + j] = col[i]
                }
                v[j] = 0.0;
            }
            tick(clock, &cj.jac_ns);
            result
        }));
    } else {
        let mj = m.clone();
        let cj = counts.clone();
        let pj = pool.clone();
        p = p.jacobian(Jacobian::jvp(move |t, y, v, out| {
            cj.jvp.fetch_add(1, Ordering::Relaxed);
            let clock = instrument.then(Instant::now);
            let z = if let Some(p) = &pj {
                p.install(|| callback(&mj, t, y, Some(v), false, out, true))
            } else {
                callback(&mj, t, y, Some(v), false, out, false)
            };
            tick(clock, &cj.jvp_ns);
            z
        }));
    }
    if m.autonomous() {
        p.autonomous()
    } else {
        p.time_derivative(move |t, y, out| {
            counts.ft.fetch_add(1, Ordering::Relaxed);
            let clock = instrument.then(Instant::now);
            let z = if let Some(p) = &pool {
                p.install(|| callback(&m, t, y, None, true, out, true))
            } else {
                callback(&m, t, y, None, true, out, false)
            };
            tick(clock, &counts.ft_ns);
            z
        })
    }
}
struct Outcome {
    success: bool,
    state: Vec<f64>,
    time: f64,
    stats: Value,
    message: String,
}
fn solve(
    m: &Model,
    method: &str,
    opts: &Options,
    counts: Arc<Counters>,
    pool: Option<Arc<rayon::ThreadPool>>,
    instrument: bool,
) -> Outcome {
    let p = problem(
        Arc::new(m.clone()),
        counts,
        pool,
        method == "rodas5p-dense",
        instrument,
    );
    let y0 = m.initial();
    let result: Result<Outcome, String> = match method {
        "dopri5" => Dopri5::solve(&p, (0.0, m.end), &y0, opts)
            .map(|s| Outcome {
                success: s.success,
                state: s.final_state,
                time: s.final_time,
                stats: json!(s.stats),
                message: s.message,
            })
            .map_err(|e| e.to_string()),
        "rodas5p-mf" | "rodas5p-dense" => Rodas5p::solve(&p, (0.0, m.end), &y0, opts)
            .map(|s| Outcome {
                success: s.is_success(),
                state: s.final_state().unwrap_or(&y0).to_vec(),
                time: s.t.last().copied().unwrap_or(0.0),
                stats: json!(s.stats),
                message: format!("{:?}", s.status),
            })
            .map_err(|e| e.to_string()),
        "switching" => {
            let config = SwitchingConfig {
                fallback: DerivativeFallback::ProvidedOnly,
                window_count: 16,
                max_dense_dimension: 256,
                implicit_pilot_attempt_budget: 32,
                implicit_pilot_limit: 4,
                ..Default::default()
            };
            SwitchingSolver::solve(&p, (0.0, m.end), &y0, opts, &config)
                .map(|s| Outcome {
                    success: s.success,
                    state: s.final_state,
                    time: s.final_time,
                    stats: json!(s.stats),
                    message: s.message,
                })
                .map_err(|e| e.to_string())
        }
        _ => Err("unrecognized method".into()),
    };
    result.unwrap_or_else(|e| Outcome {
        success: false,
        state: y0,
        time: 0.0,
        stats: Value::Null,
        message: e,
    })
}
fn emit(v: &Value) {
    println!("{v}");
    io::stdout().flush().expect("JSONL output");
}

// This is also the complete descriptor accepted by external references. Keep
// --describe and intake on the same path so parameter/grid/endpoint changes
// cannot silently reuse a reference for a different IVP.
fn model_descriptor(m: &Model) -> Value {
    let y = m.initial();
    let v: Vec<f64> = (0..m.n).map(|i| (i as f64 + 0.3).sin()).collect();
    let mut rhs = vec![0.; m.n];
    let mut jvp = rhs.clone();
    m.apply(0.0, &y, None, false, &mut rhs, false);
    m.apply(0.0, &y, Some(&v), false, &mut jvp, false);
    let probes: Vec<Value> = [0.11, 0.73]
        .iter()
        .enumerate()
        .map(|(k, f)| {
            let state: Vec<f64> = y
                .iter()
                .enumerate()
                .map(|(j, z)| z + 0.001 * (k + 1) as f64 * (j + 1) as f64 / (m.n as f64))
                .collect();
            let time = f * m.end;
            let mut out = vec![0.; m.n];
            m.apply(time, &state, None, false, &mut out, false);
            json!({"t":time,"state":state,"rhs":out})
        })
        .collect();
    json!({"kind":"model_contract","schema_version":1,"case_id":m.id,"size":if m.smoke{"smoke"}else{"standard"},"t_start":0.0,"t_end":m.end,"initial_state":y,"parameters":m.parameters(),"rhs_probes":probes,"model_id":format!("vigilode.local-bottleneck.case{}.v1",m.id),"model_contract":m.metadata(),"rhs_initial":rhs,"jvp_direction":v,"jvp_initial":jvp,"exact_final":m.exact(m.end)})
}

const EXTERNAL_REFERENCE_SCHEMA: &str = "vigilode.local-bottleneck.external-reference.v1";
const EXTERNAL_ERROR_LIMIT: f64 = 1e-4;
const EXTERNAL_UNCERTAINTY_LIMIT: f64 = 1e-5;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExternalReference {
    schema: String,
    model_descriptor: Value,
    model_descriptor_sha256: String,
    norm: String,
    reference_state: Vec<f64>,
    uncertainty: f64,
    provenance: Value,
}

struct EndpointReference {
    state: Vec<f64>,
    uncertainty: f64,
    external: bool,
    descriptor_sha256: Option<String>,
    file_sha256: Option<String>,
    provenance: Value,
}

fn valid_sha256(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn builtin_reference(m: &Model) -> Option<EndpointReference> {
    m.exact(m.end)
        .filter(|v| v.len() == m.n && v.iter().all(|x| x.is_finite()))
        .map(|state| EndpointReference {
            state,
            uncertainty: if m.id == 7 && m.end <= 10.0 {
                1e-190
            } else {
                0.0
            },
            external: false,
            descriptor_sha256: None,
            file_sha256: None,
            provenance: Value::Null,
        })
}

fn parse_external_reference(
    bytes: &[u8],
    m: &Model,
    file_sha256: Option<String>,
    error_limit: f64,
) -> Result<EndpointReference, String> {
    let r: ExternalReference = serde_json::from_slice(bytes)
        .map_err(|e| format!("invalid external reference JSON: {e}"))?;
    if r.schema != EXTERNAL_REFERENCE_SCHEMA {
        return Err("unsupported external reference schema".into());
    }
    if r.model_descriptor != model_descriptor(m) {
        return Err("external reference model_descriptor does not match this IVP".into());
    }
    // The controlling runner verifies these hashes against bytes before and
    // after launch. Exact embedded descriptor comparison is performed here;
    // an echoed hash is not a claim of independent SHA computation by Rust.
    if !valid_sha256(&r.model_descriptor_sha256)
        || file_sha256.as_deref().is_some_and(|s| !valid_sha256(s))
    {
        return Err("external reference SHA256 must be 64 lowercase hex digits".into());
    }
    if r.norm != "max_component_absolute" {
        return Err("external reference norm must be max_component_absolute".into());
    }
    if r.reference_state.len() != m.n || r.reference_state.iter().any(|x| !x.is_finite()) {
        return Err("external reference state must be a full finite n-vector".into());
    }
    if !r.uncertainty.is_finite() || !(0.0..=EXTERNAL_UNCERTAINTY_LIMIT).contains(&r.uncertainty) {
        return Err("external reference uncertainty must be finite in [0, 1e-5]".into());
    }
    if !error_limit.is_finite() || error_limit <= 0.0 || error_limit > EXTERNAL_ERROR_LIMIT {
        return Err("external reference error limit must be finite in (0, 1e-4]".into());
    }
    let provenance = r
        .provenance
        .as_object()
        .ok_or("external reference provenance must be an object")?;
    for key in ["source", "method"] {
        if provenance
            .get(key)
            .and_then(Value::as_str)
            .is_none_or(|s| s.trim().is_empty())
        {
            return Err(format!(
                "external reference provenance.{key} must be nonempty"
            ));
        }
    }
    if !provenance
        .get("evidence_sha256")
        .and_then(Value::as_str)
        .is_some_and(valid_sha256)
    {
        return Err("external reference provenance.evidence_sha256 is invalid".into());
    }
    if let Some(builtin) = builtin_reference(m) {
        let agreement = builtin
            .state
            .iter()
            .zip(&r.reference_state)
            .all(|(a, b)| (a - b).abs() <= builtin.uncertainty + r.uncertainty);
        if !agreement {
            return Err("external reference conflicts with built-in endpoint reference".into());
        }
    }
    Ok(EndpointReference {
        state: r.reference_state,
        uncertainty: r.uncertainty,
        external: true,
        descriptor_sha256: Some(r.model_descriptor_sha256),
        file_sha256,
        provenance: r.provenance,
    })
}

struct Assessment {
    success: bool,
    difference: Option<f64>,
    error: Option<f64>,
    maximum_error_member_index: Option<usize>,
    pass: bool,
    status: &'static str,
}

fn assess_members(
    m: &Model,
    results: &[Outcome],
    reference: Option<&EndpointReference>,
    limit: f64,
) -> Assessment {
    let states_valid = !results.is_empty()
        && results
            .iter()
            .all(|r| r.state.len() == m.n && r.state.iter().all(|x| x.is_finite()));
    let success = states_valid && results.iter().all(|r| r.success && r.time == m.end);
    let maximum = reference.filter(|_| states_valid).and_then(|e| {
        let mut maximum = (0.0_f64, 0);
        for (index, r) in results.iter().enumerate() {
            for (a, b) in r.state.iter().zip(&e.state) {
                let difference = (a - b).abs();
                if !difference.is_finite() {
                    return None;
                }
                if difference > maximum.0 {
                    maximum = (difference, index);
                }
            }
        }
        Some(maximum)
    });
    let difference = maximum.map(|(error, _)| error);
    let error = difference.and_then(|v| {
        let error = v + reference.unwrap().uncertainty;
        error.is_finite().then_some(error)
    });
    let pass = success && error.is_some_and(|e| e <= limit);
    let status = if !success {
        "SOLVER_FAILED"
    } else if reference.is_none() {
        "UNVALIDATED_REFERENCE"
    } else if pass {
        "PASS_SCOPED"
    } else {
        "ACCURACY_FAILED"
    };
    Assessment {
        success,
        difference,
        error,
        maximum_error_member_index: maximum.map(|(_, i)| i),
        pass,
        status,
    }
}

fn main() -> Result<(), String> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let has = |key: &str| a.iter().any(|s| s == key);
    let get = |key: &str, default: &str| {
        a.windows(2)
            .find(|s| s[0] == key)
            .map(|s| s[1].clone())
            .unwrap_or_else(|| default.into())
    };
    if has("--list") {
        emit(
            &json!({"kind":"catalog","schema_version":1,"methods":METHODS,"parallel_layers":LAYERS,"cases":(1..=24).map(|i|{let mut v=Model::new(i,false).unwrap().metadata();v.as_object_mut().unwrap().remove("initial_state");v}).collect::<Vec<_>>(),"endpoint_only":true,"note":"Dense method refused above256. Stage parallelism uses separate coupled example. Model variants and reference status must be retained."}),
        );
        return Ok(());
    }
    let id: usize = get("--case", "1").parse().map_err(|_| "invalid case")?;
    let size = get("--size", "smoke");
    if size != "smoke" && size != "standard" {
        return Err("size must be smoke|standard".into());
    }
    let mut m = Model::new(id, size == "smoke")?;
    if let Some(i) = a.iter().position(|s| s == "--end") {
        m.end = a
            .get(i + 1)
            .ok_or("missing --end")?
            .parse()
            .map_err(|_| "invalid --end")?;
        if !m.end.is_finite() || m.end <= 0.0 || (id == 16 && m.end >= 2.5) {
            return Err("end outside model domain".into());
        }
    }
    if has("--describe") {
        emit(&model_descriptor(&m));
        return Ok(());
    }
    let method = get("--method", "dopri5");
    let mut layer = get("--parallel-layer", "serial");
    if layer == "rhs" {
        layer = "data".into()
    }
    if !METHODS.contains(&method.as_str()) || !LAYERS.contains(&layer.as_str()) {
        return Err("unsupported method or parallel layer; run --list".into());
    }
    let threads: usize = get("--threads", "1")
        .parse()
        .map_err(|_| "invalid threads")?;
    if threads == 0 || threads > 128 {
        return Err("threads must be1..128".into());
    }
    if layer == "serial" && threads != 1 {
        return Err("serial layer requires exactly1 thread".into());
    }
    let instrument = has("--instrument");
    let rtol: f64 = get("--rtol", "1e-6").parse().map_err(|_| "invalid rtol")?;
    let atol: f64 = get("--atol", "1e-9").parse().map_err(|_| "invalid atol")?;
    let limit: f64 = get("--error-limit", "1e-4")
        .parse()
        .map_err(|_| "invalid error limit")?;
    if !rtol.is_finite()
        || rtol <= 0.0
        || !atol.is_finite()
        || atol <= 0.0
        || !limit.is_finite()
        || limit <= 0.0
    {
        return Err("tolerances must be finite positive".into());
    }
    let max_steps: usize = get("--max-steps", "100000")
        .parse()
        .map_err(|_| "invalid max steps")?;
    let members = if layer == "ensemble" {
        get("--ensemble-count", if m.smoke { "2" } else { "256" })
            .parse::<usize>()
            .map_err(|_| "invalid ensemble count")?
    } else {
        1
    };
    if members == 0 || members.checked_mul(m.n).is_none_or(|n| n > 16_777_216) {
        return Err("ensemble state budget exceeded".into());
    }
    for flag in ["--reference-json", "--reference-sha256"] {
        if a.iter().filter(|s| s.as_str() == flag).count() > 1 {
            return Err(format!("duplicate {flag}"));
        }
    }
    let reference = if let Some(i) = a.iter().position(|s| s == "--reference-json") {
        let path = a
            .get(i + 1)
            .filter(|s| !s.starts_with("--"))
            .ok_or("missing --reference-json path")?;
        let digest = if let Some(i) = a.iter().position(|s| s == "--reference-sha256") {
            Some(a.get(i + 1).ok_or("missing --reference-sha256")?.clone())
        } else {
            None
        };
        let bytes = std::fs::read(path).map_err(|e| format!("read external reference: {e}"))?;
        Some(parse_external_reference(&bytes, &m, digest, limit)?)
    } else {
        if has("--reference-sha256") {
            return Err("--reference-sha256 requires --reference-json".into());
        }
        builtin_reference(&m)
    };
    if method == "rodas5p-dense" && m.n > 256 {
        emit(
            &json!({"kind":"result","schema_version":1,"case":id,"method":method,"size":size,"threads":threads,"parallel_layer":layer,"status":"REFUSED_DIMENSION","scientific_status":"NOT_RUN","speed_comparison_eligible":false,"elapsed_seconds":0.0,"dimension":m.n,"message":"dense benchmark cap256; use supplied-JVP matrix-free route"}),
        );
        return Ok(());
    }
    let start = Instant::now();
    let pool = if layer != "serial" {
        Some(Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .map_err(|e| e.to_string())?,
        ))
    } else {
        None
    };
    let startup = start.elapsed().as_secs_f64();
    emit(
        &json!({"kind":"header","schema_version":1,"model_contract":m.metadata(),"method":method,"parallel_layer":layer,"threads":threads,"ensemble_count":members,"ensemble_semantics":"independent identical IVPs; throughput control, per-member error test; no combined enlarged WRMS","instrumented":instrument,"rtol":rtol,"atol":atol,"error_limit":limit,"max_steps":max_steps,"pool_startup_seconds":startup,"source_commit":std::env::var("VIGILODE_SOURCE_COMMIT").ok(),"timing_scope":"whole solve incl problem/workspace creation, rejection and probes; model/pool/diagnostics/serialization excluded"}),
    );
    let opts = Options {
        rtol,
        atol: Atol::Scalar(atol),
        h0: 1e-4_f64.min(m.end / 10.0),
        max_steps,
        ..Default::default()
    };
    let counts = Arc::new(Counters::default());
    let started = Instant::now();
    let results: Vec<Outcome> = if layer == "ensemble" {
        pool.as_ref().unwrap().install(|| {
            (0..members)
                .into_par_iter()
                .map(|_| solve(&m, &method, &opts, counts.clone(), None, instrument))
                .collect()
        })
    } else {
        vec![solve(
            &m,
            &method,
            &opts,
            counts.clone(),
            pool.clone(),
            instrument,
        )]
    };
    let elapsed = started.elapsed().as_secs_f64();
    let assessment = assess_members(&m, &results, reference.as_ref(), limit);
    let Assessment {
        success,
        difference: reference_difference,
        error,
        maximum_error_member_index,
        pass,
        status,
    } = assessment;
    let exact = reference.as_ref().map(|r| &r.state);
    let reference_error_bound = reference.as_ref().map_or(0.0, |r| r.uncertainty);
    let reference_kind = if reference.as_ref().is_some_and(|r| r.external) {
        "external-endpoint-reference"
    } else {
        m.exact_kind()
    };
    let first = &results[0];
    let diagnostics = if first.state.len() == m.n {
        m.diagnostics(&first.state)
    } else {
        Value::Null
    };
    emit(
        &json!({"kind":"result","schema_version":1,"case":id,"name":m.name(),"variant":m.variant(),"size":size,"dimension":m.n,"method":method,"threads":threads,"parallel_layer":layer,"ensemble_count":members,"instrumented":instrument,"status":status,"scientific_status":status,"solver_success":success,"speed_comparison_eligible":pass&&!instrument,"elapsed_seconds":elapsed,"throughput_trajectories_per_second":members as f64/elapsed,"actual_error":error,"endpoint_reference_difference":reference_difference,"reference_absolute_bound":reference_error_bound,"maximum_error_member_index":maximum_error_member_index,"error_limit":limit,"reference_kind":reference_kind,"reference_sha256":reference.as_ref().and_then(|r|r.file_sha256.as_ref()),"model_descriptor_sha256":reference.as_ref().and_then(|r|r.descriptor_sha256.as_ref()),"reference_provenance":reference.as_ref().map(|r|&r.provenance),"reference_hash_verification":"controller verifies bytes before and after launch; Rust validates digest syntax and exact embedded descriptor","accuracy_scope":"maximum component absolute endpoint error across every ensemble member; no continuous/phase-global certificate","final_time":first.time,"counters":counts.counts(),"phase_seconds":if instrument{counts.phases()}else{Value::Null},"pool_startup_seconds":startup,"diagnostics":diagnostics,"member_successes":results.iter().filter(|r|r.success).count(),"first_member_stats":first.stats,"message":first.message,"final_state":if has("--emit-state"){json!(first.state)}else{Value::Null},"exact_state":if has("--emit-state"){json!(exact)}else{Value::Null}}),
    );
    Ok(())
}

#[cfg(test)]
#[path = "../../../research/local_frontier_native_diffsol_20261024/test_external_reference.rs"]
mod external_reference_tests;
