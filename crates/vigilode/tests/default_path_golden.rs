//! Bitwise oracle of the default-facing entry points (research loop L1, node
//! V03G).
//!
//! Every case calls one public entry point of `vigilode` and records the
//! status, the final time, the number of outputs, the final state as the hex
//! of `f64::to_bits`, a digest of every output row, every work counter the
//! result type exposes (the serde record of the result, with every `f64`
//! replaced by its bit pattern) and, under dense output, the bits of five
//! interpolated states at fixed fractions of the span. The decision grid
//! enumerates `GeneralOde::select` and `Rodas5p::capabilities` without
//! entering a callback.
//!
//! The records are compared byte for byte with the files of
//! `tests/golden/default_path/`: no tolerance is applied anywhere. With
//! `VIGILODE_BLESS_DEFAULT_PATH=1` the files are written instead of compared.
//!
//! A mismatch is reported as three lists of JSON paths: changed, removed and
//! added. `VIGILODE_DEFAULT_PATH_ALLOW_ADDED_KEYS=1` accepts a run whose only
//! difference is added keys (a record that gained a field); a changed or a
//! removed value always fails.
//!
//! Scope: only routes a default user reaches. Opt-in surfaces
//! (`SwitchingSolver` with derivative permission, `ProductionConfig::
//! selective_jvp`, `AutomaticOde::solve_adaptive`, `PeerOde`, sessions,
//! charts, `Dopri5Ensemble` and the execution policy, the last two of which
//! use worker threads or wall time) are not part of this oracle.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{Debug, Display},
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use serde::Serialize;
use serde_json::{Map, Value, json};
use vigilode::{
    Atol, AutomaticOde, AutomaticOdePolicy, CallbackError, DerivativeFallback, Direction, Dopri5,
    ExplicitSolution, ExplicitSolveError, GeneralOde, GeneralOdePolicy, GeneralResult,
    GeneralSolution, GeneralSolveError, InteriorError, Jacobian, Options, Problem, ProblemRegime,
    ProductionConfig, ProductionSolver, Rodas5p, Solution, SolveError, SwitchingConfig,
    SwitchingSolution, Tier,
};

// ------------------------------------------------------------------ format

/// Schema tag of the golden files.
const SCHEMA: &str = "vigilode.default-path-golden.v1";
/// Set to `1` to write the golden files instead of comparing with them.
const BLESS_ENV: &str = "VIGILODE_BLESS_DEFAULT_PATH";
/// Set to `1` to accept a run whose only difference is added keys.
const ADDITIVE_ENV: &str = "VIGILODE_DEFAULT_PATH_ALLOW_ADDED_KEYS";
/// The golden files of this oracle, one per test.
const GOLDEN_STEMS: [&str; 9] = [
    "decision_grid",
    "dopri5",
    "general_ode",
    "production_automatic",
    "rodas5p_brusselator",
    "rodas5p_contracts",
    "rodas5p_hires",
    "rodas5p_robertson",
    "rodas5p_vanderpol",
];
/// Relative tolerances of the numerical cases.
const RTOLS: [f64; 3] = [1.0e-4, 1.0e-6, 1.0e-8];
/// Fractions of the span at which a dense-output case is interpolated.
const DENSE_FRACTIONS: [f64; 5] = [0.1, 0.3, 0.5, 0.7, 0.9];
/// A float array of the serde record longer than this is stored as a digest.
const LONG_FLOAT_ARRAY: usize = 16;
/// Paths listed per kind in a mismatch report.
const REPORTED_PATHS: usize = 40;

fn env_is_one(name: &str) -> bool {
    std::env::var(name).as_deref() == Ok("1")
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/default_path")
}

/// FNV-1a over the little-endian bytes of 64-bit words. Each byte update is a
/// bijection of the state, so a change of one word always changes the digest.
struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn word(&mut self, word: u64) {
        for byte in word.to_le_bytes() {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    fn floats(&mut self, values: &[f64]) {
        self.word(values.len() as u64);
        for value in values {
            self.word(value.to_bits());
        }
    }

    fn hex(&self) -> String {
        format!("{:016x}", self.0)
    }
}

fn hex(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

fn hex_array(values: &[f64]) -> Value {
    Value::Array(values.iter().map(|v| Value::String(hex(*v))).collect())
}

/// The bit pattern of a float, followed by its shortest decimal form for the
/// reader (the comparison is a string comparison, so the bits decide).
fn float_text(value: f64) -> String {
    format!("f64:{}|{value:e}", hex(value))
}

/// Replace every float of a serde tree by its bit pattern. A long array of
/// floats (an echoed initial state or tolerance vector) becomes its digest.
fn bitify(value: Value) -> Value {
    match value {
        Value::Number(number) if number.is_f64() => {
            Value::String(float_text(number.as_f64().expect("an f64 number")))
        }
        Value::Array(items) => {
            let floats: Option<Vec<f64>> = items
                .iter()
                .map(|item| match item {
                    Value::Number(number) if number.is_f64() => number.as_f64(),
                    _ => None,
                })
                .collect();
            match floats {
                Some(floats) if floats.len() > LONG_FLOAT_ARRAY => {
                    let mut digest = Fnv::new();
                    digest.floats(&floats);
                    json!({ "f64_array_len": floats.len(), "fnv1a64": digest.hex() })
                }
                _ => Value::Array(items.into_iter().map(bitify).collect()),
            }
        }
        Value::Object(map) => Value::Object(map.into_iter().map(|(k, v)| (k, bitify(v))).collect()),
        other => other,
    }
}

/// The serde record of a value with every float as its bit pattern.
fn tree<T: Serialize>(value: &T) -> Value {
    bitify(serde_json::to_value(value).expect("the record serializes"))
}

/// The serde record of a result without its output table (`t`, `y`), which
/// the case records as a digest instead.
fn record_without_outputs<T: Serialize>(value: &T) -> Value {
    let mut record = serde_json::to_value(value).expect("the record serializes");
    if let Value::Object(map) = &mut record {
        map.remove("t");
        map.remove("y");
    }
    bitify(record)
}

fn quote(text: &str) -> String {
    serde_json::to_string(text).expect("a string serializes")
}

/// Deterministic JSON: sorted keys, one key per line, arrays of scalars on
/// one line. A raw float is refused: every float is a bit-pattern string.
fn write_canonical(value: &Value, indent: usize, out: &mut String) {
    let pad = |out: &mut String, level: usize| out.push_str(&"  ".repeat(level));
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
        Value::Number(number) => {
            assert!(
                !number.is_f64(),
                "a raw float {number} reached the golden writer"
            );
            out.push_str(&number.to_string());
        }
        Value::String(text) => out.push_str(&quote(text)),
        Value::Array(items) if items.iter().any(|v| v.is_object() || v.is_array()) => {
            out.push_str("[\n");
            for (index, item) in items.iter().enumerate() {
                pad(out, indent + 1);
                write_canonical(item, indent + 1, out);
                out.push_str(if index + 1 < items.len() { ",\n" } else { "\n" });
            }
            pad(out, indent);
            out.push(']');
        }
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                write_canonical(item, indent, out);
            }
            out.push(']');
        }
        Value::Object(map) if map.is_empty() => out.push_str("{}"),
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push_str("{\n");
            for (index, key) in keys.iter().enumerate() {
                pad(out, indent + 1);
                out.push_str(&quote(key));
                out.push_str(": ");
                write_canonical(&map[*key], indent + 1, out);
                out.push_str(if index + 1 < keys.len() { ",\n" } else { "\n" });
            }
            pad(out, indent);
            out.push('}');
        }
    }
}

/// Sorted-key JSON without whitespace: the identity of a grid outcome.
fn compact(value: &Value) -> String {
    match value {
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(compact).collect();
            format!("[{}]", parts.join(","))
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .iter()
                .map(|key| format!("{}:{}", quote(key), compact(&map[*key])))
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        scalar => {
            let mut out = String::new();
            write_canonical(scalar, 0, &mut out);
            out
        }
    }
}

#[derive(Default)]
struct Difference {
    changed: Vec<String>,
    removed: Vec<String>,
    added: Vec<String>,
}

fn difference(path: &str, expected: &Value, actual: &Value, out: &mut Difference) {
    match (expected, actual) {
        (Value::Object(e), Value::Object(a)) => {
            let keys: BTreeSet<&String> = e.keys().chain(a.keys()).collect();
            for key in keys {
                let child = format!("{path}/{key}");
                match (e.get(key), a.get(key)) {
                    (Some(e), Some(a)) => difference(&child, e, a, out),
                    (Some(_), None) => out.removed.push(child),
                    (None, Some(_)) => out.added.push(child),
                    (None, None) => unreachable!("the key comes from one of the maps"),
                }
            }
        }
        (Value::Array(e), Value::Array(a)) if e.len() == a.len() => {
            for (index, (e, a)) in e.iter().zip(a).enumerate() {
                difference(&format!("{path}/{index}"), e, a, out);
            }
        }
        (e, a) if e == a => {}
        (e, a) => {
            let short = |value: &Value| {
                let text = compact(value);
                if text.chars().count() > 96 {
                    let head: String = text.chars().take(96).collect();
                    format!("{head}... ({} bytes)", text.len())
                } else {
                    text
                }
            };
            out.changed
                .push(format!("{path}: golden {} / run {}", short(e), short(a)));
        }
    }
}

fn listed(title: &str, paths: &[String]) -> String {
    let mut text = format!("{title}: {}\n", paths.len());
    for path in paths.iter().take(REPORTED_PATHS) {
        text.push_str("  ");
        text.push_str(path);
        text.push('\n');
    }
    if paths.len() > REPORTED_PATHS {
        text.push_str(&format!("  ... {} more\n", paths.len() - REPORTED_PATHS));
    }
    text
}

/// A decoded report of the difference between a golden document and a run.
type Explain = fn(&Value, &Value) -> String;

/// Compare a document with its golden file byte for byte, or write the file
/// under `VIGILODE_BLESS_DEFAULT_PATH=1`. `explain` adds a decoded report to
/// a mismatch (the decision grid's entry-level comparison).
fn check_golden(stem: &str, document: &Value, explain: Option<Explain>) {
    assert!(GOLDEN_STEMS.contains(&stem), "unlisted golden file {stem}");
    let mut text = String::new();
    write_canonical(document, 0, &mut text);
    text.push('\n');
    let path = golden_dir().join(format!("{stem}.json"));
    if env_is_one(BLESS_ENV) {
        fs::create_dir_all(golden_dir()).expect("the golden directory can be created");
        fs::write(&path, &text).expect("the golden file can be written");
        println!("blessed {} ({} bytes)", path.display(), text.len());
        return;
    }
    let expected = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "golden file {} cannot be read ({error}); write it once with {BLESS_ENV}=1",
            path.display()
        )
    });
    if expected == text {
        println!("golden {stem}: identical ({} bytes)", text.len());
        return;
    }
    let expected_tree: Value = serde_json::from_str(&expected)
        .unwrap_or_else(|error| panic!("golden file {} is not JSON: {error}", path.display()));
    let mut diff = Difference::default();
    difference("", &expected_tree, document, &mut diff);
    let mut report = format!("golden {stem}: the run differs from {}\n", path.display());
    report.push_str(&listed("changed", &diff.changed));
    report.push_str(&listed("removed", &diff.removed));
    report.push_str(&listed("added", &diff.added));
    if let Some(explain) = explain {
        report.push_str(&explain(&expected_tree, document));
    }
    let additive = diff.changed.is_empty() && diff.removed.is_empty() && !diff.added.is_empty();
    if additive && env_is_one(ADDITIVE_ENV) {
        println!("{report}accepted: only added keys, and {ADDITIVE_ENV}=1");
        return;
    }
    panic!("{report}");
}

/// The cases of one golden file, keyed by case id.
struct Golden {
    stem: &'static str,
    cases: BTreeMap<String, Value>,
}

impl Golden {
    fn new(stem: &'static str) -> Self {
        Self {
            stem,
            cases: BTreeMap::new(),
        }
    }

    fn add(&mut self, id: String, record: Value) {
        assert!(
            self.cases.insert(id.clone(), record).is_none(),
            "duplicate case id {id}"
        );
    }

    fn finish(self) {
        let document = json!({
            "schema": SCHEMA,
            "golden": self.stem,
            "n_cases": self.cases.len(),
            "cases": Value::Object(self.cases.into_iter().collect()),
        });
        check_golden(self.stem, &document, None);
    }
}

fn case_id(problem: &str, variant: &str, rtol: f64) -> String {
    format!("{problem}/{variant}/rtol={rtol:e}")
}

// ----------------------------------------------------------------- records

type Fields = Map<String, Value>;

fn put(fields: &mut Fields, key: &str, value: Value) {
    fields.insert(key.to_owned(), value);
}

fn input_fields(fields: &mut Fields, entry_point: &str, span: (f64, f64), y0: &[f64]) {
    put(fields, "entry_point", json!(entry_point));
    put(
        fields,
        "span",
        json!([float_text(span.0), float_text(span.1)]),
    );
    let mut digest = Fnv::new();
    digest.floats(y0);
    put(
        fields,
        "y0",
        json!({ "len": y0.len(), "fnv1a64": digest.hex() }),
    );
}

fn error_fields<E: Debug + Display>(fields: &mut Fields, error: &E) {
    put(fields, "outcome", json!("error"));
    put(fields, "error_debug", json!(format!("{error:?}")));
    put(fields, "error_display", json!(error.to_string()));
}

/// Final time, number of outputs, final state bits and the digest of every
/// output row.
fn output_fields(
    fields: &mut Fields,
    t: &[f64],
    y: &[Vec<f64>],
    final_time: Option<f64>,
    final_state: Option<&[f64]>,
) {
    put(fields, "final_time", json!(final_time.map(float_text)));
    put(fields, "n_outputs", json!(t.len()));
    put(
        fields,
        "final_state_bits",
        json!(final_state.map(hex_array)),
    );
    let mut times = Fnv::new();
    times.floats(t);
    let mut states = Fnv::new();
    states.word(y.len() as u64);
    for row in y {
        states.floats(row);
    }
    put(
        fields,
        "outputs_digest",
        json!({
            "t_fnv1a64": times.hex(),
            "y_fnv1a64": states.hex(),
            "y_rows": y.len(),
            "y_values": y.iter().map(Vec::len).sum::<usize>(),
        }),
    );
}

fn implicit_fields(fields: &mut Fields, result: Result<Solution, SolveError>, span: (f64, f64)) {
    let solution = match result {
        Ok(solution) => solution,
        Err(error) => return error_fields(fields, &error),
    };
    put(fields, "outcome", json!("solution"));
    put(fields, "status", tree(&solution.status));
    output_fields(
        fields,
        &solution.t,
        &solution.y,
        solution.t.last().copied(),
        solution.final_state(),
    );
    if solution.stats.options.dense_output {
        let samples: Vec<Value> = DENSE_FRACTIONS
            .iter()
            .map(|&fraction| {
                let t = span.0 + fraction * (span.1 - span.0);
                json!({
                    "fraction": float_text(fraction),
                    "t": float_text(t),
                    "state_bits": solution.at(t).map(|state| hex_array(&state)),
                })
            })
            .collect();
        put(fields, "dense_samples", Value::Array(samples));
    }
    put(fields, "record", record_without_outputs(&solution));
}

fn explicit_fields(fields: &mut Fields, result: Result<ExplicitSolution, ExplicitSolveError>) {
    let solution = match result {
        Ok(solution) => solution,
        Err(error) => return error_fields(fields, &error),
    };
    put(fields, "outcome", json!("solution"));
    put(
        fields,
        "status",
        json!({
            "success": solution.success,
            "stop_reason": tree(&solution.stop_reason),
        }),
    );
    output_fields(
        fields,
        &solution.t,
        &solution.y,
        Some(solution.final_time),
        Some(&solution.final_state),
    );
    put(fields, "record", record_without_outputs(&solution));
}

fn switching_fields<E: Debug + Display>(fields: &mut Fields, result: Result<SwitchingSolution, E>) {
    let solution = match result {
        Ok(solution) => solution,
        Err(error) => return error_fields(fields, &error),
    };
    put(fields, "outcome", json!("solution"));
    put(fields, "status", json!({ "success": solution.success }));
    output_fields(
        fields,
        &solution.t,
        &solution.y,
        Some(solution.final_time),
        Some(&solution.final_state),
    );
    put(fields, "record", record_without_outputs(&solution));
}

fn general_fields(
    fields: &mut Fields,
    result: Result<GeneralSolution, GeneralSolveError>,
    span: (f64, f64),
) {
    let solution = match result {
        Ok(solution) => solution,
        Err(error) => return error_fields(fields, &error),
    };
    put(fields, "selection", tree(&solution.selection));
    match solution.result {
        GeneralResult::Explicit(explicit) => explicit_fields(fields, Ok(explicit)),
        GeneralResult::Implicit(implicit) => implicit_fields(fields, Ok(implicit), span),
    }
}

fn rodas_case(problem: &Problem, span: (f64, f64), y0: &[f64], opts: &Options) -> Value {
    let mut fields = Fields::new();
    input_fields(&mut fields, "Rodas5p::solve", span, y0);
    let capabilities = match Rodas5p::capabilities(problem, opts) {
        Ok(capabilities) => tree(&capabilities),
        Err(refusal) => json!({ "refused": tree(&refusal) }),
    };
    put(&mut fields, "capabilities", capabilities);
    implicit_fields(&mut fields, Rodas5p::solve(problem, span, y0, opts), span);
    Value::Object(fields)
}

fn dopri5_case(problem: &Problem, span: (f64, f64), y0: &[f64], opts: &Options) -> Value {
    let mut fields = Fields::new();
    input_fields(&mut fields, "Dopri5::solve", span, y0);
    explicit_fields(&mut fields, Dopri5::solve(problem, span, y0, opts));
    Value::Object(fields)
}

fn general_case(
    problem: &Problem,
    span: (f64, f64),
    y0: &[f64],
    opts: &Options,
    policy: &GeneralOdePolicy,
) -> Value {
    let mut fields = Fields::new();
    input_fields(&mut fields, "GeneralOde::solve", span, y0);
    put(&mut fields, "policy", tree(policy));
    general_fields(
        &mut fields,
        GeneralOde::solve(problem, span, y0, opts, policy),
        span,
    );
    Value::Object(fields)
}

/// `ProductionSolver::solve` under the RHS-only configuration with the
/// default switching schedule (`ProductionConfig` has no `Default`; this is
/// its derivative-free factory with `SwitchingConfig::default().window_count`).
fn production_case(problem: &Problem, span: (f64, f64), y0: &[f64], opts: &Options) -> Value {
    let config = ProductionConfig::rhs_only(opts.clone(), SwitchingConfig::default().window_count);
    let mut fields = Fields::new();
    input_fields(&mut fields, "ProductionSolver::solve", span, y0);
    put(&mut fields, "config", tree(&config));
    switching_fields(
        &mut fields,
        ProductionSolver::solve(problem, span, y0, &config),
    );
    Value::Object(fields)
}

fn automatic_case(problem: &Problem, span: (f64, f64), y0: &[f64], opts: &Options) -> Value {
    let policy = AutomaticOdePolicy::default();
    let mut fields = Fields::new();
    input_fields(&mut fields, "AutomaticOde::solve", span, y0);
    put(
        &mut fields,
        "policy",
        json!({
            "derivatives": tree(&policy.derivatives),
            "max_dense_dimension": policy.max_dense_dimension,
            "allow_matrix_free": policy.allow_matrix_free,
            "replay_safe_callbacks": policy.replay_safe_callbacks,
        }),
    );
    switching_fields(
        &mut fields,
        AutomaticOde::solve(problem, span, y0, opts, &policy),
    );
    Value::Object(fields)
}

// ---------------------------------------------------------------- problems

type VectorFn = fn(f64, &[f64], &mut [f64]) -> Result<(), CallbackError>;

fn options(rtol: f64, atol: f64) -> Options {
    Options {
        rtol,
        atol: Atol::Scalar(atol),
        ..Options::default()
    }
}

fn dense_output(opts: &Options, interior_error: InteriorError) -> Options {
    Options {
        dense_output: true,
        interior_error,
        ..opts.clone()
    }
}

/// A Jacobian-vector product formed from a dense Jacobian callback, row by
/// row in index order.
fn jvp_from_dense(n: usize, jacobian: VectorFn) -> Jacobian {
    Jacobian::jvp(move |t, y, v, out| {
        let mut matrix = vec![0.0; n * n];
        jacobian(t, y, &mut matrix)?;
        for (row, slot) in out.iter_mut().enumerate() {
            let mut sum = 0.0;
            for (column, direction) in v.iter().enumerate() {
                sum += matrix[row * n + column] * direction;
            }
            *slot = sum;
        }
        Ok(())
    })
}

#[derive(Clone, Copy)]
enum Provider {
    Dense,
    FiniteDifference,
    Jvp,
}

fn with_provider(problem: Problem, jacobian: VectorFn, provider: Provider) -> Problem {
    let n = problem.dimension();
    match provider {
        Provider::Dense => problem.jacobian(Jacobian::dense(jacobian)),
        Provider::FiniteDifference => problem.jacobian(Jacobian::FiniteDifference),
        Provider::Jvp => problem.jacobian(jvp_from_dense(n, jacobian)),
    }
}

const ROBERTSON_SPAN: (f64, f64) = (0.0, 40.0);
const ROBERTSON_Y0: [f64; 3] = [1.0, 0.0, 0.0];

fn robertson_rhs(_t: f64, y: &[f64], dy: &mut [f64]) -> Result<(), CallbackError> {
    dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
    dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
    dy[2] = 3.0e7 * y[1] * y[1];
    Ok(())
}

fn robertson_jacobian(_t: f64, y: &[f64], jac: &mut [f64]) -> Result<(), CallbackError> {
    jac.copy_from_slice(&[
        -0.04,
        1.0e4 * y[2],
        1.0e4 * y[1],
        0.04,
        -1.0e4 * y[2] - 6.0e7 * y[1],
        -1.0e4 * y[1],
        0.0,
        6.0e7 * y[1],
        0.0,
    ]);
    Ok(())
}

/// Robertson without a declaration of its time dependence.
fn robertson_undeclared(provider: Provider) -> Problem {
    with_provider(Problem::new(3, robertson_rhs), robertson_jacobian, provider)
}

fn robertson(provider: Provider) -> Problem {
    robertson_undeclared(provider).autonomous()
}

fn robertson_dae_rhs(_t: f64, y: &[f64], dy: &mut [f64]) -> Result<(), CallbackError> {
    dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
    dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
    dy[2] = y[0] + y[1] + y[2] - 1.0;
    Ok(())
}

fn robertson_dae_jacobian(_t: f64, y: &[f64], jac: &mut [f64]) -> Result<(), CallbackError> {
    jac.copy_from_slice(&[
        -0.04,
        1.0e4 * y[2],
        1.0e4 * y[1],
        0.04,
        -1.0e4 * y[2] - 6.0e7 * y[1],
        -1.0e4 * y[1],
        1.0,
        1.0,
        1.0,
    ]);
    Ok(())
}

/// Robertson as an index-1 DAE with the mass matrix diag(1, 1, 0).
fn robertson_dae() -> Problem {
    Problem::new(3, robertson_dae_rhs)
        .jacobian(Jacobian::dense(robertson_dae_jacobian))
        .mass(vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0])
        .autonomous()
}

const HIRES_SPAN: (f64, f64) = (0.0, 321.8122);
const HIRES_Y0: [f64; 8] = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057];

fn hires_rhs(_t: f64, y: &[f64], dy: &mut [f64]) -> Result<(), CallbackError> {
    dy[0] = -1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007;
    dy[1] = 1.71 * y[0] - 8.75 * y[1];
    dy[2] = -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4];
    dy[3] = 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3];
    dy[4] = -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6];
    dy[5] = -280.0 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6];
    dy[6] = 280.0 * y[5] * y[7] - 1.81 * y[6];
    dy[7] = -280.0 * y[5] * y[7] + 1.81 * y[6];
    Ok(())
}

fn hires_jacobian(_t: f64, y: &[f64], jac: &mut [f64]) -> Result<(), CallbackError> {
    let mut set = |row: usize, column: usize, value: f64| jac[row * 8 + column] = value;
    set(0, 0, -1.71);
    set(0, 1, 0.43);
    set(0, 2, 8.32);
    set(1, 0, 1.71);
    set(1, 1, -8.75);
    set(2, 2, -10.03);
    set(2, 3, 0.43);
    set(2, 4, 0.035);
    set(3, 1, 8.32);
    set(3, 2, 1.71);
    set(3, 3, -1.12);
    set(4, 4, -1.745);
    set(4, 5, 0.43);
    set(4, 6, 0.43);
    set(5, 3, 0.69);
    set(5, 4, 1.71);
    set(5, 5, -280.0 * y[7] - 0.43);
    set(5, 6, 0.69);
    set(5, 7, -280.0 * y[5]);
    set(6, 5, 280.0 * y[7]);
    set(6, 6, -1.81);
    set(6, 7, 280.0 * y[5]);
    set(7, 5, -280.0 * y[7]);
    set(7, 6, 1.81);
    set(7, 7, -280.0 * y[5]);
    Ok(())
}

fn hires(provider: Provider) -> Problem {
    with_provider(Problem::new(8, hires_rhs), hires_jacobian, provider).autonomous()
}

const VANDERPOL_MU: f64 = 1000.0;
const VANDERPOL_SPAN: (f64, f64) = (0.0, 900.0);
const VANDERPOL_Y0: [f64; 2] = [2.0, 0.0];

fn vanderpol_rhs(_t: f64, y: &[f64], dy: &mut [f64]) -> Result<(), CallbackError> {
    dy[0] = y[1];
    dy[1] = VANDERPOL_MU * (1.0 - y[0] * y[0]) * y[1] - y[0];
    Ok(())
}

fn vanderpol_jacobian(_t: f64, y: &[f64], jac: &mut [f64]) -> Result<(), CallbackError> {
    jac.copy_from_slice(&[
        0.0,
        1.0,
        -2.0 * VANDERPOL_MU * y[0] * y[1] - 1.0,
        VANDERPOL_MU * (1.0 - y[0] * y[0]),
    ]);
    Ok(())
}

fn vanderpol(provider: Provider) -> Problem {
    with_provider(Problem::new(2, vanderpol_rhs), vanderpol_jacobian, provider).autonomous()
}

const BRUSSELATOR_CELLS: usize = 32;
const BRUSSELATOR_N: usize = 2 * BRUSSELATOR_CELLS;
const BRUSSELATOR_A: f64 = 1.0;
const BRUSSELATOR_B: f64 = 3.0;
const BRUSSELATOR_SPAN: (f64, f64) = (0.0, 10.0);

fn brusselator_alpha() -> f64 {
    let h = 1.0 / (BRUSSELATOR_CELLS as f64 + 1.0);
    0.02 / (h * h)
}

fn brusselator_y0() -> Vec<f64> {
    (0..BRUSSELATOR_CELLS)
        .flat_map(|cell| {
            let x = (cell as f64 + 1.0) / (BRUSSELATOR_CELLS as f64 + 1.0);
            [1.0 + (2.0 * std::f64::consts::PI * x).sin(), 3.0]
        })
        .collect()
}

/// 1D Brusselator, `u` and `v` interleaved, Dirichlet boundaries `u = A`,
/// `v = B / A`: the Jacobian has the band (2, 2).
fn brusselator_rhs(_t: f64, y: &[f64], dy: &mut [f64]) -> Result<(), CallbackError> {
    let (a, b, alpha) = (BRUSSELATOR_A, BRUSSELATOR_B, brusselator_alpha());
    for cell in 0..BRUSSELATOR_CELLS {
        let (u, v) = (y[2 * cell], y[2 * cell + 1]);
        let (u_left, v_left) = if cell == 0 {
            (a, b / a)
        } else {
            (y[2 * cell - 2], y[2 * cell - 1])
        };
        let (u_right, v_right) = if cell + 1 == BRUSSELATOR_CELLS {
            (a, b / a)
        } else {
            (y[2 * cell + 2], y[2 * cell + 3])
        };
        dy[2 * cell] = a + u * u * v - (b + 1.0) * u + alpha * (u_left - 2.0 * u + u_right);
        dy[2 * cell + 1] = b * u - u * u * v + alpha * (v_left - 2.0 * v + v_right);
    }
    Ok(())
}

fn brusselator_entries(y: &[f64], mut set: impl FnMut(usize, usize, f64)) {
    let (b, alpha) = (BRUSSELATOR_B, brusselator_alpha());
    for cell in 0..BRUSSELATOR_CELLS {
        let (u, v) = (y[2 * cell], y[2 * cell + 1]);
        let (row_u, row_v) = (2 * cell, 2 * cell + 1);
        set(row_u, row_u, 2.0 * u * v - (b + 1.0) - 2.0 * alpha);
        set(row_u, row_v, u * u);
        set(row_v, row_u, b - 2.0 * u * v);
        set(row_v, row_v, -u * u - 2.0 * alpha);
        if cell > 0 {
            set(row_u, row_u - 2, alpha);
            set(row_v, row_v - 2, alpha);
        }
        if cell + 1 < BRUSSELATOR_CELLS {
            set(row_u, row_u + 2, alpha);
            set(row_v, row_v + 2, alpha);
        }
    }
}

fn brusselator_dense(_t: f64, y: &[f64], jac: &mut [f64]) -> Result<(), CallbackError> {
    brusselator_entries(y, |row, column, value| {
        jac[row * BRUSSELATOR_N + column] = value;
    });
    Ok(())
}

fn brusselator_band(_t: f64, y: &[f64], band: &mut [f64]) -> Result<(), CallbackError> {
    brusselator_entries(y, |row, column, value| {
        band[row * 5 + (column + 2 - row)] = value;
    });
    Ok(())
}

#[derive(Clone, Copy)]
enum BandProvider {
    /// `Jacobian::Banded` with the band (2, 2).
    Banded,
    /// Finite differences with the declared band (2, 2).
    FiniteDifference,
    /// A dense callback with the declared band (2, 2).
    DenseDeclared,
}

fn brusselator(provider: Provider) -> Problem {
    with_provider(
        Problem::new(BRUSSELATOR_N, brusselator_rhs),
        brusselator_dense,
        provider,
    )
    .autonomous()
}

fn brusselator_banded(provider: BandProvider) -> Problem {
    let problem = Problem::new(BRUSSELATOR_N, brusselator_rhs).autonomous();
    match provider {
        BandProvider::Banded => problem.jacobian(Jacobian::banded(2, 2, brusselator_band)),
        BandProvider::FiniteDifference => problem.banded(2, 2),
        BandProvider::DenseDeclared => problem
            .jacobian(Jacobian::dense(brusselator_dense))
            .banded(2, 2),
    }
}

const PROTHERO_ROBINSON_LAMBDA: f64 = 1.0e4;
const PROTHERO_ROBINSON_SPAN: (f64, f64) = (0.0, 2.0);

/// Prothero-Robinson `y' = -lambda (y - cos t) - sin t`: a right-hand side
/// that depends on `t`, with or without its explicit time derivative.
fn prothero_robinson(explicit_time_derivative: bool) -> Problem {
    let problem = Problem::new(1, |t, y, dy| {
        dy[0] = -PROTHERO_ROBINSON_LAMBDA * (y[0] - t.cos()) - t.sin();
        Ok(())
    })
    .jacobian(Jacobian::dense(|_t, _y, jac| {
        jac[0] = -PROTHERO_ROBINSON_LAMBDA;
        Ok(())
    }));
    if explicit_time_derivative {
        problem.time_derivative(|t, _y, out| {
            out[0] = -PROTHERO_ROBINSON_LAMBDA * t.sin() - t.cos();
            Ok(())
        })
    } else {
        problem
    }
}

const ROTATION_SPAN: (f64, f64) = (0.0, 10.0);
const ROTATION_Y0: [f64; 2] = [1.0, 0.0];

fn rotation() -> Problem {
    Problem::new(2, |_t, y, dy| {
        dy[0] = -y[1];
        dy[1] = y[0];
        Ok(())
    })
}

const LOTKA_VOLTERRA_SPAN: (f64, f64) = (0.0, 10.0);
const LOTKA_VOLTERRA_Y0: [f64; 2] = [10.0, 5.0];

fn lotka_volterra() -> Problem {
    Problem::new(2, |_t, y, dy| {
        dy[0] = 1.5 * y[0] - y[0] * y[1];
        dy[1] = -3.0 * y[1] + y[0] * y[1];
        Ok(())
    })
}

const FORCED_SPAN: (f64, f64) = (0.0, 10.0);
const FORCED_Y0: [f64; 2] = [0.5, 0.0];

/// A forced, weakly damped nonlinear oscillator (the right-hand side depends
/// on `t`).
fn forced_oscillator() -> Problem {
    Problem::new(2, |t, y, dy| {
        dy[0] = y[1];
        dy[1] = -y[0] - 0.1 * y[1] - 0.2 * y[0] * y[0] * y[0] + 0.5 * (1.3 * t).cos();
        Ok(())
    })
}

/// The span on which the explicit method is run on Robertson.
const ROBERTSON_EXPLICIT_SPAN: (f64, f64) = (0.0, 4.0);
/// The span on which the explicit method is run on the Brusselator.
const BRUSSELATOR_EXPLICIT_SPAN: (f64, f64) = (0.0, 1.0);

// ------------------------------------------------------------- Rodas5p::solve

#[test]
fn rodas5p_robertson() {
    let mut golden = Golden::new("rodas5p_robertson");
    for rtol in RTOLS {
        let atol = rtol * 1.0e-4;
        let base = options(rtol, atol);
        let vector = Options {
            atol: Atol::Vector(vec![atol, 0.1 * atol, atol]),
            ..base.clone()
        };
        let mut run = |variant: &str, problem: Problem, opts: &Options| {
            golden.add(
                case_id("robertson", variant, rtol),
                rodas_case(&problem, ROBERTSON_SPAN, &ROBERTSON_Y0, opts),
            );
        };
        // Dense fast route.
        run("dense-jac", robertson(Provider::Dense), &base);
        run("fd-jac", robertson(Provider::FiniteDifference), &base);
        run(
            "dense-jac/time-central-difference",
            robertson_undeclared(Provider::Dense),
            &base,
        );
        run(
            "dense-jac/time-derivative-supplied",
            robertson_undeclared(Provider::Dense).time_derivative(|_t, _y, out| {
                out.fill(0.0);
                Ok(())
            }),
            &base,
        );
        run("dense-jac/vector-atol", robertson(Provider::Dense), &vector);
        run(
            "fd-jac/tier-performance",
            robertson(Provider::FiniteDifference),
            &Options {
                tier: Tier::Performance,
                ..base.clone()
            },
        );
        // Matrix-free route.
        run("jvp", robertson(Provider::Jvp), &base);
        // Fast dense route (continuous extension).
        run(
            "dense-jac/dense-output",
            robertson(Provider::Dense),
            &dense_output(&base, InteriorError::WFilteredEnforce),
        );
        run(
            "dense-jac/dense-output-interior-off",
            robertson(Provider::Dense),
            &dense_output(&base, InteriorError::Off),
        );
        run(
            "fd-jac/dense-output",
            robertson(Provider::FiniteDifference),
            &dense_output(&base, InteriorError::WFilteredEnforce),
        );
        run(
            "dense-jac/vector-atol/dense-output",
            robertson(Provider::Dense),
            &dense_output(&vector, InteriorError::WFilteredEnforce),
        );
        // Sequential dense route.
        run(
            "dense-jac/dense-output-interior-enforce",
            robertson(Provider::Dense),
            &dense_output(&base, InteriorError::Enforce),
        );
        run("mass-dae", robertson_dae(), &base);
        run(
            "mass-dae/dense-output",
            robertson_dae(),
            &dense_output(&base, InteriorError::WFilteredEnforce),
        );
    }
    golden.finish();
}

#[test]
fn rodas5p_hires() {
    let mut golden = Golden::new("rodas5p_hires");
    for rtol in RTOLS {
        let base = options(rtol, rtol * 1.0e-2);
        let mut run = |variant: &str, provider: Provider, opts: &Options| {
            golden.add(
                case_id("hires", variant, rtol),
                rodas_case(&hires(provider), HIRES_SPAN, &HIRES_Y0, opts),
            );
        };
        run("dense-jac", Provider::Dense, &base);
        run("fd-jac", Provider::FiniteDifference, &base);
        run("jvp", Provider::Jvp, &base);
        run(
            "dense-jac/dense-output",
            Provider::Dense,
            &dense_output(&base, InteriorError::WFilteredEnforce),
        );
        run(
            "fd-jac/dense-output",
            Provider::FiniteDifference,
            &dense_output(&base, InteriorError::WFilteredEnforce),
        );
        run(
            "fd-jac/dense-output-interior-enforce",
            Provider::FiniteDifference,
            &dense_output(&base, InteriorError::Enforce),
        );
    }
    golden.finish();
}

#[test]
fn rodas5p_vanderpol() {
    let mut golden = Golden::new("rodas5p_vanderpol");
    for rtol in RTOLS {
        let base = options(rtol, rtol);
        let mut run = |variant: &str, provider: Provider, opts: &Options| {
            golden.add(
                case_id("vanderpol-mu1000", variant, rtol),
                rodas_case(&vanderpol(provider), VANDERPOL_SPAN, &VANDERPOL_Y0, opts),
            );
        };
        run("dense-jac", Provider::Dense, &base);
        run("fd-jac", Provider::FiniteDifference, &base);
        run("jvp", Provider::Jvp, &base);
        run(
            "dense-jac/dense-output",
            Provider::Dense,
            &dense_output(&base, InteriorError::WFilteredEnforce),
        );
        run(
            "fd-jac/dense-output-interior-off",
            Provider::FiniteDifference,
            &dense_output(&base, InteriorError::Off),
        );
        run(
            "dense-jac/dense-output-interior-enforce",
            Provider::Dense,
            &dense_output(&base, InteriorError::Enforce),
        );
    }
    golden.finish();
}

#[test]
fn rodas5p_brusselator() {
    let mut golden = Golden::new("rodas5p_brusselator");
    let y0 = brusselator_y0();
    for rtol in RTOLS {
        let atol = rtol * 1.0e-2;
        let base = options(rtol, atol);
        let vector = Options {
            atol: Atol::Vector(
                (0..BRUSSELATOR_N)
                    .map(|index| atol * (1.0 + (index % 3) as f64))
                    .collect(),
            ),
            ..base.clone()
        };
        let mut run = |variant: &str, problem: Problem, opts: &Options| {
            golden.add(
                case_id("brusselator-n64", variant, rtol),
                rodas_case(&problem, BRUSSELATOR_SPAN, &y0, opts),
            );
        };
        // Dense fast route.
        run("dense-jac", brusselator(Provider::Dense), &base);
        run("fd-jac", brusselator(Provider::FiniteDifference), &base);
        // Banded fast route: the band provider, a finite-difference band and
        // the router's verified declaration on a dense callback.
        run(
            "banded-jac",
            brusselator_banded(BandProvider::Banded),
            &base,
        );
        run(
            "banded-jac/declared-band",
            brusselator_banded(BandProvider::Banded).banded(2, 2),
            &base,
        );
        run(
            "fd-banded",
            brusselator_banded(BandProvider::FiniteDifference),
            &base,
        );
        run(
            "dense-jac-declared-band",
            brusselator_banded(BandProvider::DenseDeclared),
            &base,
        );
        run(
            "banded-jac/vector-atol",
            brusselator_banded(BandProvider::Banded),
            &vector,
        );
        // Matrix-free route.
        run("jvp", brusselator(Provider::Jvp), &base);
        // Fast dense and fast banded dense routes.
        run(
            "dense-jac/dense-output",
            brusselator(Provider::Dense),
            &dense_output(&base, InteriorError::WFilteredEnforce),
        );
        run(
            "banded-jac/dense-output",
            brusselator_banded(BandProvider::Banded),
            &dense_output(&base, InteriorError::WFilteredEnforce),
        );
        run(
            "fd-banded/dense-output",
            brusselator_banded(BandProvider::FiniteDifference),
            &dense_output(&base, InteriorError::WFilteredEnforce),
        );
        run(
            "fd-banded/dense-output-interior-off",
            brusselator_banded(BandProvider::FiniteDifference),
            &dense_output(&base, InteriorError::Off),
        );
        run(
            "fd-banded/vector-atol/dense-output",
            brusselator_banded(BandProvider::FiniteDifference),
            &dense_output(&vector, InteriorError::WFilteredEnforce),
        );
        // Sequential dense route: a band assembled into a dense matrix.
        run(
            "banded-jac/dense-output-interior-enforce",
            brusselator_banded(BandProvider::Banded),
            &dense_output(&base, InteriorError::Enforce),
        );
        let mut identity = vec![0.0; BRUSSELATOR_N * BRUSSELATOR_N];
        for index in 0..BRUSSELATOR_N {
            identity[index * BRUSSELATOR_N + index] = 1.0;
        }
        run(
            "fd-banded/mass-identity",
            brusselator_banded(BandProvider::FiniteDifference).mass(identity),
            &base,
        );
    }
    golden.finish();
}

/// Time-dependent right-hand sides, events, a failed run and the refusals of
/// `Rodas5p::solve`.
#[test]
fn rodas5p_contracts() {
    let mut golden = Golden::new("rodas5p_contracts");
    for rtol in RTOLS {
        let base = options(rtol, rtol * 1.0e-2);
        let mut run = |variant: &str, explicit_time_derivative: bool, opts: &Options| {
            golden.add(
                case_id("prothero-robinson", variant, rtol),
                rodas_case(
                    &prothero_robinson(explicit_time_derivative),
                    PROTHERO_ROBINSON_SPAN,
                    &[1.0],
                    opts,
                ),
            );
        };
        run("time-central-difference", false, &base);
        run("time-derivative-supplied", true, &base);
        run(
            "time-derivative-supplied/dense-output",
            true,
            &dense_output(&base, InteriorError::WFilteredEnforce),
        );
        run(
            "time-central-difference/dense-output-interior-enforce",
            false,
            &dense_output(&base, InteriorError::Enforce),
        );
    }

    let rtol = 1.0e-6;
    let base = options(rtol, 1.0e-10);
    let dense = dense_output(&base, InteriorError::WFilteredEnforce);
    let y0 = brusselator_y0();
    let mut run =
        |variant: &str, problem: Problem, span: (f64, f64), y0: &[f64], opts: &Options| {
            golden.add(
                case_id("contract", variant, opts.rtol),
                rodas_case(&problem, span, y0, opts),
            );
        };

    // Events located on the continuous extension.
    run(
        "robertson/event",
        robertson(Provider::Dense).event(|_t, y| y[0] - 0.9, Direction::Down),
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &dense,
    );
    run(
        "robertson/event-and-terminal-event",
        robertson(Provider::Dense)
            .event(|_t, y| y[0] - 0.9, Direction::Down)
            .terminal_event(|_t, y| y[2] - 0.25, Direction::Up),
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &dense,
    );
    // A run that stops early keeps its partial outputs and work.
    run(
        "hires/attempt-budget",
        hires(Provider::FiniteDifference),
        HIRES_SPAN,
        &HIRES_Y0,
        &Options {
            max_steps: 25,
            ..base.clone()
        },
    );
    run(
        "robertson/attempt-budget/dense-output",
        robertson(Provider::Dense),
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &Options {
            max_steps: 12,
            ..dense.clone()
        },
    );
    run(
        "robertson/zero-length-span",
        robertson(Provider::Dense),
        (0.0, 0.0),
        &ROBERTSON_Y0,
        &base,
    );
    // Refusals.
    run(
        "refused/jvp-with-dense-output",
        robertson(Provider::Jvp),
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &dense,
    );
    run(
        "refused/jvp-with-vector-atol",
        robertson(Provider::Jvp),
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &Options {
            atol: Atol::Vector(vec![1.0e-10; 3]),
            ..base.clone()
        },
    );
    run(
        "refused/mass-with-vector-atol",
        robertson_dae(),
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &Options {
            atol: Atol::Vector(vec![1.0e-10; 3]),
            ..base.clone()
        },
    );
    run(
        "refused/jvp-with-declared-band",
        brusselator(Provider::Jvp).banded(2, 2),
        BRUSSELATOR_SPAN,
        &y0,
        &base,
    );
    run(
        "refused/dense-declared-band-with-dense-output",
        brusselator_banded(BandProvider::DenseDeclared),
        BRUSSELATOR_SPAN,
        &y0,
        &dense,
    );
    run(
        "refused/events-without-dense-output",
        robertson(Provider::Dense).event(|_t, y| y[0] - 0.9, Direction::Down),
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &base,
    );
    run(
        "refused/declared-band-too-narrow",
        brusselator(Provider::Dense).banded(1, 1),
        BRUSSELATOR_SPAN,
        &y0,
        &base,
    );
    run(
        "refused/initial-state-length",
        robertson(Provider::Dense),
        ROBERTSON_SPAN,
        &[1.0, 0.0],
        &base,
    );
    run(
        "refused/negative-rtol",
        robertson(Provider::Dense),
        ROBERTSON_SPAN,
        &ROBERTSON_Y0,
        &options(-1.0e-6, 1.0e-10),
    );
    golden.finish();
}

// -------------------------------------------------------------- Dopri5::solve

#[test]
fn dopri5() {
    let mut golden = Golden::new("dopri5");
    let bruss_y0 = brusselator_y0();
    for rtol in RTOLS {
        let base = options(rtol, rtol * 1.0e-3);
        let mut run = |name: &str, problem: Problem, span: (f64, f64), y0: &[f64]| {
            golden.add(
                case_id(name, "rhs-only", rtol),
                dopri5_case(&problem, span, y0, &base),
            );
        };
        run("rotation", rotation(), ROTATION_SPAN, &ROTATION_Y0);
        run(
            "lotka-volterra",
            lotka_volterra(),
            LOTKA_VOLTERRA_SPAN,
            &LOTKA_VOLTERRA_Y0,
        );
        run(
            "forced-oscillator",
            forced_oscillator(),
            FORCED_SPAN,
            &FORCED_Y0,
        );
        run(
            "brusselator-n64",
            brusselator(Provider::FiniteDifference),
            BRUSSELATOR_EXPLICIT_SPAN,
            &bruss_y0,
        );
        // A stiff problem on the explicit method: stability-limited steps.
        run(
            "robertson",
            robertson(Provider::Dense),
            ROBERTSON_EXPLICIT_SPAN,
            &ROBERTSON_Y0,
        );
    }

    let rtol = 1.0e-6;
    let base = options(rtol, 1.0e-9);
    let mut run = |variant: &str, problem: Problem, opts: &Options| {
        golden.add(
            case_id("lotka-volterra", variant, opts.rtol),
            dopri5_case(&problem, LOTKA_VOLTERRA_SPAN, &LOTKA_VOLTERRA_Y0, opts),
        );
    };
    run(
        "vector-atol",
        lotka_volterra(),
        &Options {
            atol: Atol::Vector(vec![1.0e-9, 1.0e-7]),
            ..base.clone()
        },
    );
    run(
        "attempt-budget",
        lotka_volterra(),
        &Options {
            max_steps: 10,
            ..base.clone()
        },
    );
    run(
        "refused/dense-output",
        lotka_volterra(),
        &dense_output(&base, InteriorError::WFilteredEnforce),
    );
    run(
        "refused/mass",
        lotka_volterra().mass(vec![1.0, 0.0, 0.0, 1.0]),
        &base,
    );
    run(
        "refused/events",
        lotka_volterra().event(|_t, y| y[0] - 1.0, Direction::Either),
        &base,
    );
    golden.finish();
}

// ----------------------------------------------------------- GeneralOde::solve

fn stiff_policy(derivatives: DerivativeFallback) -> GeneralOdePolicy {
    GeneralOdePolicy {
        regime: ProblemRegime::Stiff,
        derivatives,
        ..GeneralOdePolicy::default()
    }
}

#[test]
fn general_ode() {
    let mut golden = Golden::new("general_ode");
    let bruss_y0 = brusselator_y0();
    let default_policy = GeneralOdePolicy::default();
    let provided = stiff_policy(DerivativeFallback::ProvidedOnly);
    let finite_difference = stiff_policy(DerivativeFallback::AllowFiniteDifference);
    let matrix_free = GeneralOdePolicy {
        allow_matrix_free: true,
        ..provided
    };
    for rtol in RTOLS {
        let mut run = |name: &str,
                       variant: &str,
                       problem: Problem,
                       span: (f64, f64),
                       y0: &[f64],
                       atol: f64,
                       policy: &GeneralOdePolicy| {
            golden.add(
                case_id(name, variant, rtol),
                general_case(&problem, span, y0, &options(rtol, atol), policy),
            );
        };
        // The default policy: RHS-only, whatever the problem declares.
        run(
            "rotation",
            "default-policy",
            rotation(),
            ROTATION_SPAN,
            &ROTATION_Y0,
            rtol * 1.0e-3,
            &default_policy,
        );
        run(
            "lotka-volterra",
            "default-policy",
            lotka_volterra(),
            LOTKA_VOLTERRA_SPAN,
            &LOTKA_VOLTERRA_Y0,
            rtol * 1.0e-3,
            &default_policy,
        );
        run(
            "forced-oscillator",
            "default-policy",
            forced_oscillator(),
            FORCED_SPAN,
            &FORCED_Y0,
            rtol * 1.0e-3,
            &default_policy,
        );
        run(
            "robertson",
            "default-policy/dense-jac-declared",
            robertson(Provider::Dense),
            ROBERTSON_EXPLICIT_SPAN,
            &ROBERTSON_Y0,
            rtol * 1.0e-4,
            &default_policy,
        );
        run(
            "rotation",
            "nonstiff-policy",
            rotation(),
            ROTATION_SPAN,
            &ROTATION_Y0,
            rtol * 1.0e-3,
            &GeneralOdePolicy {
                regime: ProblemRegime::Nonstiff,
                ..default_policy
            },
        );
        // A stiff policy that permits supplied derivatives.
        run(
            "robertson",
            "stiff-provided-only/dense-jac",
            robertson(Provider::Dense),
            ROBERTSON_SPAN,
            &ROBERTSON_Y0,
            rtol * 1.0e-4,
            &provided,
        );
        run(
            "hires",
            "stiff-provided-only/dense-jac",
            hires(Provider::Dense),
            HIRES_SPAN,
            &HIRES_Y0,
            rtol * 1.0e-2,
            &provided,
        );
        run(
            "vanderpol-mu1000",
            "stiff-provided-only/dense-jac",
            vanderpol(Provider::Dense),
            VANDERPOL_SPAN,
            &VANDERPOL_Y0,
            rtol,
            &provided,
        );
        run(
            "brusselator-n64",
            "stiff-provided-only/banded-jac",
            brusselator_banded(BandProvider::Banded),
            BRUSSELATOR_SPAN,
            &bruss_y0,
            rtol * 1.0e-2,
            &provided,
        );
        run(
            "brusselator-n64",
            "stiff-provided-only/dense-jac",
            brusselator(Provider::Dense),
            BRUSSELATOR_SPAN,
            &bruss_y0,
            rtol * 1.0e-2,
            &provided,
        );
        run(
            "brusselator-n64",
            "stiff-provided-only-matrix-free/jvp",
            brusselator(Provider::Jvp),
            BRUSSELATOR_SPAN,
            &bruss_y0,
            rtol * 1.0e-2,
            &matrix_free,
        );
        // A stiff policy that also permits finite differences.
        run(
            "hires",
            "stiff-allow-finite-difference/fd-jac",
            hires(Provider::FiniteDifference),
            HIRES_SPAN,
            &HIRES_Y0,
            rtol * 1.0e-2,
            &finite_difference,
        );
        run(
            "brusselator-n64",
            "stiff-allow-finite-difference/fd-banded",
            brusselator_banded(BandProvider::FiniteDifference),
            BRUSSELATOR_SPAN,
            &bruss_y0,
            rtol * 1.0e-2,
            &finite_difference,
        );
    }

    let rtol = 1.0e-6;
    let base = options(rtol, 1.0e-10);
    let mut refuse =
        |variant: &str, problem: Problem, opts: &Options, policy: &GeneralOdePolicy| {
            golden.add(
                case_id("refused", variant, opts.rtol),
                general_case(&problem, ROBERTSON_SPAN, &ROBERTSON_Y0, opts, policy),
            );
        };
    refuse(
        "stiff-derivatives-forbidden",
        robertson(Provider::Dense),
        &base,
        &stiff_policy(DerivativeFallback::Forbidden),
    );
    refuse(
        "stiff-provided-only/fd-jac",
        robertson(Provider::FiniteDifference),
        &base,
        &provided,
    );
    refuse(
        "stiff-provided-only/time-derivative-missing",
        robertson_undeclared(Provider::Dense),
        &base,
        &provided,
    );
    refuse(
        "stiff-provided-only/jvp-without-matrix-free",
        robertson(Provider::Jvp),
        &base,
        &provided,
    );
    refuse(
        "stiff-provided-only/dense-dimension-limit",
        robertson(Provider::Dense),
        &base,
        &GeneralOdePolicy {
            max_dense_dimension: 2,
            ..provided
        },
    );
    refuse(
        "default-policy/dense-output",
        robertson(Provider::Dense),
        &dense_output(&base, InteriorError::WFilteredEnforce),
        &default_policy,
    );
    refuse(
        "default-policy/mass",
        robertson_dae(),
        &base,
        &default_policy,
    );
    refuse(
        "default-policy/events",
        robertson(Provider::Dense).event(|_t, y| y[0] - 0.9, Direction::Down),
        &base,
        &default_policy,
    );
    golden.finish();
}

// ------------------------------------- ProductionSolver and AutomaticOde::solve

#[test]
fn production_automatic() {
    let mut golden = Golden::new("production_automatic");
    let bruss_y0 = brusselator_y0();
    for rtol in RTOLS {
        let mut run = |name: &str, problem: Problem, span: (f64, f64), y0: &[f64], atol: f64| {
            let opts = options(rtol, atol);
            golden.add(
                case_id(name, "production-rhs-only", rtol),
                production_case(&problem, span, y0, &opts),
            );
            golden.add(
                case_id(name, "automatic-default-policy", rtol),
                automatic_case(&problem, span, y0, &opts),
            );
        };
        run(
            "rotation",
            rotation(),
            ROTATION_SPAN,
            &ROTATION_Y0,
            rtol * 1.0e-3,
        );
        run(
            "lotka-volterra",
            lotka_volterra(),
            LOTKA_VOLTERRA_SPAN,
            &LOTKA_VOLTERRA_Y0,
            rtol * 1.0e-3,
        );
        run(
            "forced-oscillator",
            forced_oscillator(),
            FORCED_SPAN,
            &FORCED_Y0,
            rtol * 1.0e-3,
        );
        run(
            "brusselator-n64",
            brusselator_banded(BandProvider::Banded),
            BRUSSELATOR_EXPLICIT_SPAN,
            &bruss_y0,
            rtol * 1.0e-2,
        );
        // Stiff problems stay on the RHS-only method: no derivative is
        // authorized by either default.
        run(
            "robertson",
            robertson(Provider::Dense),
            ROBERTSON_EXPLICIT_SPAN,
            &ROBERTSON_Y0,
            rtol * 1.0e-4,
        );
        run(
            "hires",
            hires(Provider::Jvp),
            (0.0, 5.0),
            &HIRES_Y0,
            rtol * 1.0e-2,
        );
    }

    let rtol = 1.0e-6;
    let base = options(rtol, 1.0e-9);
    let mut run = |variant: &str, problem: Problem, opts: &Options| {
        golden.add(
            case_id(
                "lotka-volterra",
                &format!("production-rhs-only/{variant}"),
                rtol,
            ),
            production_case(&problem, LOTKA_VOLTERRA_SPAN, &LOTKA_VOLTERRA_Y0, opts),
        );
        golden.add(
            case_id(
                "lotka-volterra",
                &format!("automatic-default-policy/{variant}"),
                rtol,
            ),
            automatic_case(&problem, LOTKA_VOLTERRA_SPAN, &LOTKA_VOLTERRA_Y0, opts),
        );
    };
    run(
        "attempt-budget",
        lotka_volterra(),
        &Options {
            max_steps: 40,
            ..base.clone()
        },
    );
    run(
        "refused/dense-output",
        lotka_volterra(),
        &dense_output(&base, InteriorError::WFilteredEnforce),
    );
    run(
        "refused/mass",
        lotka_volterra().mass(vec![1.0, 0.0, 0.0, 1.0]),
        &base,
    );
    run(
        "refused/events",
        lotka_volterra().event(|_t, y| y[0] - 1.0, Direction::Either),
        &base,
    );
    golden.finish();
}

// ------------------------------------------------------------- decision grid

const GRID_DIMENSIONS: [usize; 4] = [8, 64, 65, 500];
const GRID_DENSE_CEILINGS: [usize; 2] = [64, 1000];
const CODE_ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

#[derive(Clone, Copy)]
enum GridJacobian {
    Dense,
    Banded,
    Jvp,
    FiniteDifference,
}

#[derive(Clone, Copy)]
struct Features {
    jacobian: GridJacobian,
    autonomous: bool,
    time_derivative: bool,
    mass: bool,
    events: bool,
    band: bool,
}

impl Features {
    fn all() -> Vec<Self> {
        let mut all = Vec::new();
        for jacobian in [
            GridJacobian::Dense,
            GridJacobian::Banded,
            GridJacobian::Jvp,
            GridJacobian::FiniteDifference,
        ] {
            for autonomous in [false, true] {
                for time_derivative in [false, true] {
                    for mass in [false, true] {
                        for events in [false, true] {
                            for band in [false, true] {
                                all.push(Self {
                                    jacobian,
                                    autonomous,
                                    time_derivative,
                                    mass,
                                    events,
                                    band,
                                });
                            }
                        }
                    }
                }
            }
        }
        all
    }

    fn key(&self) -> String {
        let jacobian = match self.jacobian {
            GridJacobian::Dense => "dense",
            GridJacobian::Banded => "banded",
            GridJacobian::Jvp => "jvp",
            GridJacobian::FiniteDifference => "finite-difference",
        };
        format!(
            "jacobian={jacobian} autonomous={} time_derivative={} mass={} events={} declared_band={}",
            u8::from(self.autonomous),
            u8::from(self.time_derivative),
            u8::from(self.mass),
            u8::from(self.events),
            u8::from(self.band),
        )
    }

    /// The problem with these features. Every callback counts its entries.
    fn problem(&self, n: usize, entries: &Arc<AtomicUsize>) -> Problem {
        let count = |entries: &Arc<AtomicUsize>| {
            let entries = entries.clone();
            move || {
                entries.fetch_add(1, Ordering::Relaxed);
            }
        };
        let rhs = count(entries);
        let mut problem = Problem::new(n, move |_t, _y, _dy| {
            rhs();
            Ok(())
        });
        problem = match self.jacobian {
            GridJacobian::Dense => {
                let entered = count(entries);
                problem.jacobian(Jacobian::dense(move |_t, _y, _jac| {
                    entered();
                    Ok(())
                }))
            }
            GridJacobian::Banded => {
                let entered = count(entries);
                problem.jacobian(Jacobian::banded(1, 1, move |_t, _y, _band| {
                    entered();
                    Ok(())
                }))
            }
            GridJacobian::Jvp => {
                let entered = count(entries);
                problem.jacobian(Jacobian::jvp(move |_t, _y, _v, _out| {
                    entered();
                    Ok(())
                }))
            }
            GridJacobian::FiniteDifference => problem.jacobian(Jacobian::FiniteDifference),
        };
        if self.autonomous {
            problem = problem.autonomous();
        }
        if self.time_derivative {
            let entered = count(entries);
            problem = problem.time_derivative(move |_t, _y, _out| {
                entered();
                Ok(())
            });
        }
        if self.mass {
            let mut identity = vec![0.0; n * n];
            for index in 0..n {
                identity[index * n + index] = 1.0;
            }
            problem = problem.mass(identity);
        }
        if self.events {
            let entered = count(entries);
            problem = problem.event(
                move |_t, _y| {
                    entered();
                    0.0
                },
                Direction::Either,
            );
        }
        if self.band {
            problem = problem.banded(1, 1);
        }
        problem
    }
}

#[derive(Clone, Copy)]
struct GridOptions {
    dense_output: bool,
    interior_error: InteriorError,
    vector_atol: bool,
}

impl GridOptions {
    fn all() -> Vec<Self> {
        let mut all = Vec::new();
        for dense_output in [false, true] {
            for interior_error in [
                InteriorError::WFilteredEnforce,
                InteriorError::Enforce,
                InteriorError::Off,
            ] {
                for vector_atol in [false, true] {
                    all.push(Self {
                        dense_output,
                        interior_error,
                        vector_atol,
                    });
                }
            }
        }
        all
    }

    fn key(&self) -> String {
        let interior = match self.interior_error {
            InteriorError::WFilteredEnforce => "w-filtered-enforce",
            InteriorError::Enforce => "enforce",
            InteriorError::Off => "off",
            _ => unreachable!("the grid enumerates the three known modes"),
        };
        format!(
            "dense_output={} interior_error={interior} atol={}",
            u8::from(self.dense_output),
            if self.vector_atol { "vector" } else { "scalar" },
        )
    }

    fn options(&self, n: usize, tier: Tier) -> Options {
        Options {
            atol: if self.vector_atol {
                Atol::Vector(vec![1.0e-9; n])
            } else {
                Atol::Scalar(1.0e-9)
            },
            dense_output: self.dense_output,
            interior_error: self.interior_error,
            tier,
            ..Options::default()
        }
    }
}

fn grid_policies() -> Vec<GeneralOdePolicy> {
    let mut all = Vec::new();
    for regime in [
        ProblemRegime::Unknown,
        ProblemRegime::Nonstiff,
        ProblemRegime::Stiff,
    ] {
        for derivatives in [
            DerivativeFallback::Forbidden,
            DerivativeFallback::ProvidedOnly,
            DerivativeFallback::AllowFiniteDifference,
        ] {
            for allow_matrix_free in [false, true] {
                for max_dense_dimension in GRID_DENSE_CEILINGS {
                    all.push(GeneralOdePolicy {
                        regime,
                        derivatives,
                        max_dense_dimension,
                        allow_matrix_free,
                    });
                }
            }
        }
    }
    all
}

/// One table of the grid: per (features, options) row, one outcome per
/// column, stored as an index into the table's own legend.
#[derive(Default)]
struct GridTable {
    outcomes: BTreeMap<String, (usize, Value)>,
    rows: BTreeMap<(String, String), Vec<usize>>,
}

impl GridTable {
    fn record(&mut self, row: (String, String), columns: usize, column: usize, outcome: Value) {
        let next = self.outcomes.len();
        let provisional = self
            .outcomes
            .entry(compact(&outcome))
            .or_insert((next, outcome))
            .0;
        let cells = self
            .rows
            .entry(row)
            .or_insert_with(|| vec![usize::MAX; columns]);
        assert_eq!(cells[column], usize::MAX, "a grid cell was recorded twice");
        cells[column] = provisional;
    }

    fn entries(&self) -> usize {
        self.rows.values().map(Vec::len).sum()
    }

    /// The table as JSON: the legend in the order of the outcomes' compact
    /// text, and one fixed-width code per cell.
    fn into_value(self, axes: Value) -> Value {
        let width = if self.outcomes.len() <= CODE_ALPHABET.len() {
            1
        } else {
            2
        };
        assert!(
            self.outcomes.len() <= CODE_ALPHABET.len() * CODE_ALPHABET.len(),
            "more distinct outcomes than two-character codes"
        );
        let code = |index: usize| -> String {
            let digit = |d: usize| char::from(CODE_ALPHABET[d]);
            if width == 1 {
                digit(index).to_string()
            } else {
                [
                    digit(index / CODE_ALPHABET.len()),
                    digit(index % CODE_ALPHABET.len()),
                ]
                .iter()
                .collect()
            }
        };
        let entries = self.entries();
        let mut final_code = vec![String::new(); self.outcomes.len()];
        let mut legend = Map::new();
        for (rank, (provisional, outcome)) in self.outcomes.into_values().enumerate() {
            final_code[provisional] = code(rank);
            legend.insert(code(rank), outcome);
        }
        let mut rows: BTreeMap<String, Map<String, Value>> = BTreeMap::new();
        for ((features, options), cells) in self.rows {
            let text: String = cells
                .iter()
                .map(|&cell| {
                    assert_ne!(cell, usize::MAX, "a grid cell was not recorded");
                    final_code[cell].as_str()
                })
                .collect();
            rows.entry(features)
                .or_default()
                .insert(options, Value::String(text));
        }
        json!({
            "axes": axes,
            "code_width": width,
            "n_entries": entries,
            "legend": Value::Object(legend),
            "rows": rows,
        })
    }
}

/// Decode one table of a grid document into `coordinate -> outcome`.
fn decode_grid(document: &Value, table: &str) -> BTreeMap<String, String> {
    let table = &document[table];
    let width = table["code_width"].as_u64().unwrap_or(1).max(1) as usize;
    let mut decoded = BTreeMap::new();
    let (Some(legend), Some(rows)) = (table["legend"].as_object(), table["rows"].as_object())
    else {
        return decoded;
    };
    for (features, by_options) in rows {
        let Some(by_options) = by_options.as_object() else {
            continue;
        };
        for (options, cells) in by_options {
            let cells = cells.as_str().unwrap_or_default().as_bytes();
            for (column, code) in cells.chunks(width).enumerate() {
                let code = String::from_utf8_lossy(code).into_owned();
                let outcome = legend
                    .get(&code)
                    .map_or_else(|| format!("<code {code} not in legend>"), compact);
                decoded.insert(format!("{features} | {options} | column {column}"), outcome);
            }
        }
    }
    decoded
}

/// The entry-level comparison of two grid documents (codes are decoded with
/// each document's own legend, so a renumbered legend is not a difference).
fn explain_grid(expected: &Value, actual: &Value) -> String {
    let mut report = String::new();
    for table in ["select", "capabilities"] {
        let (expected, actual) = (decode_grid(expected, table), decode_grid(actual, table));
        let coordinates: BTreeSet<&String> = expected.keys().chain(actual.keys()).collect();
        let mut differing = Vec::new();
        for coordinate in coordinates {
            let (e, a) = (expected.get(coordinate), actual.get(coordinate));
            if e != a {
                differing.push(format!(
                    "{coordinate}: golden {} / run {}",
                    e.map_or("<absent>", String::as_str),
                    a.map_or("<absent>", String::as_str),
                ));
            }
        }
        report.push_str(&listed(
            &format!(
                "decoded {table} decisions that differ (of {} golden, {} run)",
                expected.len(),
                actual.len()
            ),
            &differing,
        ));
    }
    report
}

/// `GeneralOde::select` over every feature, option, dimension and policy
/// combination, and `Rodas5p::capabilities` over every feature, option,
/// dimension and tier combination. No callback may be entered.
#[test]
fn decision_grid() {
    let entries = Arc::new(AtomicUsize::new(0));
    let features = Features::all();
    let grid_options = GridOptions::all();
    let policies = grid_policies();
    let tiers = [Tier::Protected, Tier::Performance];
    let select_columns = GRID_DIMENSIONS.len() * policies.len();
    let capability_columns = GRID_DIMENSIONS.len() * tiers.len();
    let mut select = GridTable::default();
    let mut capabilities = GridTable::default();

    for feature in &features {
        for (dimension_index, &n) in GRID_DIMENSIONS.iter().enumerate() {
            let problem = feature.problem(n, &entries);
            for grid_option in &grid_options {
                let row = (feature.key(), grid_option.key());
                let opts = grid_option.options(n, Tier::Protected);
                for (policy_index, policy) in policies.iter().enumerate() {
                    let outcome = match GeneralOde::select(&problem, &opts, policy) {
                        Ok(selection) => json!({ "selected": tree(&selection) }),
                        Err(refusal) => json!({ "refused": tree(&refusal) }),
                    };
                    select.record(
                        row.clone(),
                        select_columns,
                        dimension_index * policies.len() + policy_index,
                        outcome,
                    );
                }
                for (tier_index, &tier) in tiers.iter().enumerate() {
                    let opts = grid_option.options(n, tier);
                    let outcome = match Rodas5p::capabilities(&problem, &opts) {
                        Ok(supported) => json!({ "supported": tree(&supported) }),
                        Err(refusal) => json!({ "refused": tree(&refusal) }),
                    };
                    capabilities.record(
                        row.clone(),
                        capability_columns,
                        dimension_index * tiers.len() + tier_index,
                        outcome,
                    );
                }
            }
        }
    }

    let callback_entries = entries.load(Ordering::Relaxed);
    assert_eq!(callback_entries, 0, "a selection query entered a callback");
    let rows = features.len() * grid_options.len();
    assert_eq!(select.rows.len(), rows);
    assert_eq!(select.entries(), rows * select_columns);
    assert_eq!(capabilities.entries(), rows * capability_columns);

    let dimensions = json!(GRID_DIMENSIONS);
    let n_select_entries = select.entries();
    let n_capability_entries = capabilities.entries();
    let document = json!({
        "schema": SCHEMA,
        "golden": "decision_grid",
        "callback_entries": callback_entries,
        "n_rows": rows,
        "n_select_entries": n_select_entries,
        "n_capability_entries": n_capability_entries,
        "select": select.into_value(json!({
            "entry_point": "GeneralOde::select",
            "column": "dimension_index * n_policies + policy_index",
            "dimensions": dimensions,
            "policies": policies.iter().map(tree).collect::<Vec<_>>(),
            "declared_band": [1, 1],
        })),
        "capabilities": capabilities.into_value(json!({
            "entry_point": "Rodas5p::capabilities",
            "column": "dimension_index * n_tiers + tier_index",
            "dimensions": dimensions,
            "tiers": tiers.iter().map(tree).collect::<Vec<_>>(),
            "declared_band": [1, 1],
        })),
    });
    check_golden("decision_grid", &document, Some(explain_grid));
}

// ------------------------------------------------------------------ hygiene

/// The golden directory holds exactly the files of this oracle.
#[test]
fn golden_directory_is_exactly_the_listed_files() {
    if env_is_one(BLESS_ENV) {
        // The other tests are writing the files in parallel.
        return;
    }
    let found: BTreeSet<String> = fs::read_dir(golden_dir())
        .expect("the golden directory exists; write it once with the bless variable")
        .map(|entry| {
            entry
                .expect("a directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    let expected: BTreeSet<String> = GOLDEN_STEMS
        .iter()
        .map(|stem| format!("{stem}.json"))
        .collect();
    assert_eq!(found, expected);
}

/// The canonical writer and the digest are what the comparison rests on.
#[test]
fn format_helpers_are_bit_faithful() {
    // A float and its neighbour, and both zeros, have different texts.
    let x = 0.1_f64;
    let neighbour = f64::from_bits(x.to_bits() + 1);
    assert_ne!(float_text(x), float_text(neighbour));
    assert_ne!(float_text(0.0), float_text(-0.0));
    assert_eq!(float_text(1.0), "f64:3ff0000000000000|1e0");
    // The digest separates a one-bit change and a moved row boundary.
    let digest = |rows: &[&[f64]]| {
        let mut fnv = Fnv::new();
        for row in rows {
            fnv.floats(row);
        }
        fnv.hex()
    };
    assert_ne!(digest(&[&[x, 1.0]]), digest(&[&[neighbour, 1.0]]));
    assert_ne!(digest(&[&[x, 1.0]]), digest(&[&[x], &[1.0]]));
    // Keys are written sorted; a float record keeps its bits.
    let mut text = String::new();
    write_canonical(&tree(&json!({ "b": 0.5, "a": [1, 2] })), 0, &mut text);
    assert_eq!(
        text,
        "{\n  \"a\": [1, 2],\n  \"b\": \"f64:3fe0000000000000|5e-1\"\n}"
    );
    // The difference report separates changed, removed and added paths.
    let mut diff = Difference::default();
    difference(
        "",
        &json!({ "kept": 1, "changed": 1, "removed": 1 }),
        &json!({ "kept": 1, "changed": 2, "added": 1 }),
        &mut diff,
    );
    assert_eq!(diff.changed, ["/changed: golden 1 / run 2"]);
    assert_eq!(diff.removed, ["/removed"]);
    assert_eq!(diff.added, ["/added"]);
}
