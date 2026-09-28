//! E-02 / E-03 experiment binary: re-run the 18 n=96 ScientificCorpusV2.1
//! calibration rows through the same public integrator entry points the
//! canonical campaign uses (`execute_arm` in
//! crates/rodas5p-fair-ab/src/scientific_validity_v2_campaign.rs, replicated
//! verbatim for the single-segment calibration families), plus extra dense-path
//! policies (E-03a) and a union-schedule dense run that captures the exact
//! accepted-step states (E-03c attribution).
//!
//! `run_scientific_validity_v2_case` cannot be used directly: it refuses any
//! reference bundle whose `implementation_revision` differs from the compiled
//! revision (campaign was ab8fbcd, this tree is b3e8165).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use rodas5p_core::{InitialGuess, LinearMethod, LinearSolverConfig, PreconditionerKind, WorkCounters, sha256_hex};
use rodas5p_fair_ab::{
    GlobalErrorMetrics, NumericalReferenceBundleV2, classify_output_policy_dominance,
    classify_reference_dominance, load_numerical_reference_v2,
};
use rodas5p_integrators::{
    AdaptiveRunDiagnostics, AdaptiveStepConfig, ControllerKind, OutputSamplingPlan, OutputSchedule,
    ScientificCaseSpec, ScientificCorpusV2, ScientificProblemCase,
    integrate_sequential_matrix_free_adaptive_dense_observed,
    integrate_sequential_matrix_free_adaptive_observed,
};
use serde::Serialize;

#[derive(Parser, Debug)]
struct Args {
    /// Path to a v2 reference manifest whose `artifacts/` dir holds the n=96 artifacts.
    #[arg(long)]
    manifest: PathBuf,
    /// Output JSON path.
    #[arg(long)]
    out: PathBuf,
    /// Only run these families (comma separated family strings); default all six.
    #[arg(long)]
    families: Option<String>,
    /// Only run these rtols (comma separated); default 1e-4,1e-6,1e-8.
    #[arg(long)]
    rtols: Option<String>,
    /// Skip the extra E-03 policies (only clipped_base + dense_base).
    #[arg(long, default_value_t = false)]
    e02_only: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Mode {
    Clipped,
    Dense,
}

impl Mode {
    fn tag(self) -> u8 {
        match self {
            Self::Clipped => 0,
            Self::Dense => 1,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct Policy {
    name: String,
    mode: Mode,
    initial_step: f64,
    max_step: f64,
    /// Extra output times merged into the grid (dense only). Used to capture
    /// accepted-step states exactly (collector returns y_new at theta==1).
    #[serde(skip)]
    extra_times: Vec<f64>,
}

#[derive(Clone, Debug, Serialize)]
struct CountersOut {
    rhs_calls: u64,
    rhs_evaluations: u64,
    ft_calls: u64,
    jvp_calls: u64,
    linear_solves: u64,
    linear_iterations: u64,
    linear_matvecs: u64,
    forced_stage_solves: u64,
    accepted_steps: u64,
    rejected_steps: u64,
    local_error_failures: u64,
    linear_solve_failures: u64,
}

fn counters_out(c: &WorkCounters) -> CountersOut {
    CountersOut {
        rhs_calls: c.rhs_calls,
        rhs_evaluations: c.rhs_evaluations,
        ft_calls: c.ft_calls,
        jvp_calls: c.jvp_calls,
        linear_solves: c.linear_solves,
        linear_iterations: c.linear_iterations,
        linear_matvecs: c.linear_matvecs,
        forced_stage_solves: c.forced_stage_solves,
        accepted_steps: c.accepted_steps,
        rejected_steps: c.rejected_steps,
        local_error_failures: c.local_error_failures,
        linear_solve_failures: c.linear_solve_failures,
    }
}

#[derive(Clone, Debug, Serialize)]
struct ArmOut {
    policy: Policy,
    status: String,
    message: String,
    wall_seconds: f64,
    internal_steps: usize,
    output_clipped_steps: usize,
    counters: CountersOut,
    diagnostics: AdaptiveRunDiagnostics,
    /// Campaign-identical output checksum over the 101-point grid subset
    /// (mode tag, status tag, times, states), comparable with the committed
    /// clipped/dense_output_checksum_sha256 fields.
    grid_output_checksum_sha256: String,
    metrics: Option<GlobalErrorMetrics>,
    /// Max accepted embedded error norm (solver's own control quantity, 1.0 == at tolerance).
    max_accepted_error_norm: Option<f64>,
    mean_accepted_error_norm: Option<f64>,
    /// Output times/states on the 101 grid.
    grid_times: Vec<f64>,
    grid_states: Vec<Vec<f64>>,
    /// Extra sampled times (accepted-step endpoints) and their exact states, if requested.
    extra_times: Vec<f64>,
    extra_states: Vec<Vec<f64>>,
}

#[derive(Clone, Debug, Serialize)]
struct PairOut {
    left: String,
    right: String,
    gap_wrms: f64,
    denominator_policy: String,
    denominator_max_grid_wrms: f64,
    ratio: f64,
    output_policy_dominance: String,
}

#[derive(Clone, Debug, Serialize)]
struct RowOut {
    case_id: String,
    family: String,
    dimension: usize,
    rtol: f64,
    atol: f64,
    t_span: (f64, f64),
    reference_checksum_sha256: String,
    reference_uncertainty_wrms: f64,
    base_initial_step: f64,
    base_max_step: f64,
    arms: Vec<ArmOut>,
    pairs: Vec<PairOut>,
    /// Campaign-identical E-02 verdict for (clipped_base, dense_base).
    e02_status: String,
    e02_output_policy_discrepancy_wrms: Option<f64>,
    error: Option<String>,
}

#[derive(Serialize)]
struct Report {
    compiled_revision: String,
    source_dirty_at_build: bool,
    manifest: String,
    manifest_sha256: String,
    rows: Vec<RowOut>,
}

fn linear_config() -> LinearSolverConfig {
    // verbatim from linear_config() in scientific_validity_v2_campaign.rs
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        rtol: 1.0e-10,
        atol: 1.0e-12,
        restart: 32,
        maxiter: 256,
        inner_m: 30,
        outer_k: 8,
        recycle_dim: 8,
        recycle_rank_tol: 1.0e-12,
        preconditioner: PreconditionerKind::None,
        x0_strategy: InitialGuess::Previous,
    }
}

fn adaptive_config(spec: &ScientificCaseSpec, initial_step: f64, max_step: f64, max_attempts: usize) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: spec.atol,
        rtol: spec.rtol,
        initial_step,
        min_step: 1.0e-12,
        max_step,
        max_attempts,
        safety: 0.9,
        min_factor: 0.2,
        max_factor: 5.0,
        reject_max_factor: 0.9,
        controller: ControllerKind::Integral,
    }
}

const MAX_ATTEMPTS: usize = 200_000;

fn output_checksum(mode: Mode, success: bool, times: &[f64], states: &[Vec<f64>]) -> String {
    let mut bytes = b"vigilode-scientific-v2-mode-output-v1\0".to_vec();
    bytes.push(mode.tag());
    bytes.push(if success { 0 } else { 1 });
    bytes.extend_from_slice(&(times.len() as u64).to_le_bytes());
    for (time, state) in times.iter().zip(states) {
        bytes.extend_from_slice(&time.to_bits().to_le_bytes());
        bytes.extend_from_slice(&(state.len() as u64).to_le_bytes());
        for value in state {
            bytes.extend_from_slice(&value.to_bits().to_le_bytes());
        }
    }
    sha256_hex(&bytes)
}

fn time_close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 64.0 * f64::EPSILON * a.abs().max(b.abs()).max(1.0)
}

/// Replicates execute_arm for a single-segment case, with a policy override.
fn execute_arm(case: &ScientificProblemCase, policy: &Policy, bundle: &NumericalReferenceBundleV2) -> anyhow::Result<ArmOut> {
    anyhow::ensure!(case.integration_segments.len() == 1, "calibration case must be single-segment");
    let segment = &case.integration_segments[0];
    let started = Instant::now();
    let linear = linear_config();
    let grid = case.spec.output_times.clone();
    // schedule = grid (+ extra times for dense attribution run)
    let mut sched_times = grid.clone();
    for &t in &policy.extra_times {
        if t <= segment.t_span.0 || t >= segment.t_span.1 { continue; }
        if sched_times.iter().any(|&g| time_close(g, t)) { continue; }
        sched_times.push(t);
    }
    sched_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let schedule = OutputSchedule::new(sched_times.clone())?;
    let adaptive = adaptive_config(&case.spec, policy.initial_step, policy.max_step, MAX_ATTEMPTS);
    let matrix_free_problem = segment.problem.jvp_only_clone()?;
    let state = case.y0.clone();
    let result = match policy.mode {
        Mode::Clipped => integrate_sequential_matrix_free_adaptive_observed(
            &matrix_free_problem, segment.t_span, &state, &linear, &adaptive, &schedule,
        ).map_err(|e| e.to_string()),
        Mode::Dense => {
            let sampling = OutputSamplingPlan::dense(schedule);
            integrate_sequential_matrix_free_adaptive_dense_observed(
                &matrix_free_problem, segment.t_span, &state, &linear, &adaptive, &sampling,
            ).map_err(|e| e.to_string())
        }
    };
    let wall = started.elapsed().as_secs_f64();
    let result = match result {
        Ok(r) => r,
        Err(e) => {
            return Ok(ArmOut {
                policy: policy.clone(), status: "failure".into(), message: format!("integrator error: {e}"),
                wall_seconds: wall, internal_steps: 0, output_clipped_steps: 0,
                counters: counters_out(&WorkCounters::default()), diagnostics: AdaptiveRunDiagnostics::default(),
                grid_output_checksum_sha256: String::new(), metrics: None, max_accepted_error_norm: None,
                mean_accepted_error_norm: None, grid_times: vec![], grid_states: vec![], extra_times: vec![], extra_states: vec![],
            });
        }
    };
    let success = result.observed.success;
    // split outputs into grid subset and extra subset
    let mut grid_times = Vec::new();
    let mut grid_states = Vec::new();
    let mut extra_times = Vec::new();
    let mut extra_states = Vec::new();
    for (t, y) in result.observed.t.iter().zip(&result.observed.y) {
        if grid.iter().any(|&g| g.to_bits() == t.to_bits()) {
            grid_times.push(*t);
            grid_states.push(y.clone());
        } else {
            extra_times.push(*t);
            extra_states.push(y.clone());
        }
    }
    let grid_exact = success && grid_times.len() == grid.len()
        && grid_times.iter().zip(&grid).all(|(a, b)| a.to_bits() == b.to_bits());
    let checksum = output_checksum(policy.mode, success, &grid_times, &grid_states);
    let metrics = if grid_exact { bundle.wrms_basis.metrics(&grid_times, &grid_states).ok() } else { None };
    let d = &result.diagnostics;
    let accepted_norms: Vec<f64> = d.error_norms.iter().zip(&d.failure_kinds).filter(|(_, k)| k.is_none()).map(|(e, _)| *e).collect();
    let max_acc = accepted_norms.iter().cloned().fold(None, |m: Option<f64>, v| Some(m.map_or(v, |m| m.max(v))));
    let mean_acc = if accepted_norms.is_empty() { None } else { Some(accepted_norms.iter().sum::<f64>() / accepted_norms.len() as f64) };
    Ok(ArmOut {
        policy: policy.clone(),
        status: if success { "success".into() } else { "failure".into() },
        message: result.observed.message.clone(),
        wall_seconds: wall,
        internal_steps: result.observed.internal_steps,
        output_clipped_steps: result.observed.output_clipped_steps,
        counters: counters_out(&result.observed.counters),
        diagnostics: result.diagnostics.clone(),
        grid_output_checksum_sha256: checksum,
        metrics,
        max_accepted_error_norm: max_acc,
        mean_accepted_error_norm: mean_acc,
        grid_times,
        grid_states,
        extra_times,
        extra_states,
    })
}

fn accepted_times(t0: f64, tf: f64, sizes: &[f64]) -> Vec<f64> {
    // The integrator advances t = report.t_new; we mirror t + h and clamp the final landing.
    let mut t = t0;
    let mut out = Vec::new();
    for &h in sizes {
        t += h;
        if t > tf { t = tf; }
        out.push(t);
    }
    out
}

fn pair(bundle: &NumericalReferenceBundleV2, arms: &BTreeMap<String, ArmOut>, left: &str, right: &str, denom: &str) -> Option<PairOut> {
    let l = arms.get(left)?;
    let r = arms.get(right)?;
    let d = arms.get(denom)?;
    if l.metrics.is_none() || r.metrics.is_none() || d.metrics.is_none() { return None; }
    let gap = bundle.wrms_basis.discrepancy_wrms(&l.grid_times, &l.grid_states, &r.grid_times, &r.grid_states).ok()?;
    let dm = d.metrics.as_ref()?.max_grid_wrms;
    let dom = classify_output_policy_dominance(gap, dm).ok()?;
    Some(PairOut {
        left: left.into(), right: right.into(), gap_wrms: gap, denominator_policy: denom.into(),
        denominator_max_grid_wrms: dm, ratio: if dm > 0.0 { gap / dm } else { f64::INFINITY },
        output_policy_dominance: format!("{dom:?}"),
    })
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let fam_filter: Option<Vec<String>> = args.families.as_ref().map(|s| s.split(',').map(|x| x.trim().to_string()).collect());
    let rtol_filter: Option<Vec<f64>> = args.rtols.as_ref().map(|s| s.split(',').map(|x| x.trim().parse::<f64>().unwrap()).collect());
    let manifest_bytes = std::fs::read(&args.manifest)?;
    let manifest_sha = sha256_hex(&manifest_bytes);
    let compiled = rodas5p_fair_ab::scientific_validity_v2_compiled_revision().map(|s| s.to_string()).unwrap_or_else(|e| format!("unavailable: {e}"));
    let dirty = rodas5p_fair_ab::scientific_validity_v2_source_dirty_at_build();
    eprintln!("compiled_revision={compiled} dirty={dirty} manifest_sha256={manifest_sha}");

    let specs: Vec<ScientificCaseSpec> = ScientificCorpusV2::calibration_specs().into_iter().filter(|s| s.dimension == 96).collect();
    let mut rows = Vec::new();
    for spec in specs {
        if let Some(f) = &fam_filter { if !f.iter().any(|x| x == spec.family.as_str()) { continue; } }
        if let Some(r) = &rtol_filter { if !r.iter().any(|x| x.to_bits() == spec.rtol.to_bits()) { continue; } }
        let span = spec.t_span.1 - spec.t_span.0;
        let base_h0 = span / 100.0;
        let base_hmax = span;
        let mut row = RowOut {
            case_id: spec.id.clone(), family: spec.family.as_str().to_string(), dimension: spec.dimension, rtol: spec.rtol, atol: spec.atol,
            t_span: spec.t_span, reference_checksum_sha256: String::new(), reference_uncertainty_wrms: f64::NAN,
            base_initial_step: base_h0, base_max_step: base_hmax, arms: vec![], pairs: vec![], e02_status: "not-run".into(),
            e02_output_policy_discrepancy_wrms: None, error: None,
        };
        eprintln!("== {}", spec.id);
        let bundle = match load_numerical_reference_v2(&args.manifest, &spec) {
            Ok(b) => b,
            Err(e) => { row.error = Some(format!("reference load failed: {e}")); rows.push(row); continue; }
        };
        row.reference_checksum_sha256 = bundle.reference_checksum_sha256.clone();
        row.reference_uncertainty_wrms = bundle.error_scale.reference_uncertainty_wrms;
        let case = match spec.build() { Ok(c) => c, Err(e) => { row.error = Some(format!("build failed: {e}")); rows.push(row); continue; } };

        let mut policies = vec![
            Policy { name: "clipped_base".into(), mode: Mode::Clipped, initial_step: base_h0, max_step: base_hmax, extra_times: vec![] },
            Policy { name: "dense_base".into(), mode: Mode::Dense, initial_step: base_h0, max_step: base_hmax, extra_times: vec![] },
        ];
        if !args.e02_only {
            policies.push(Policy { name: "dense_h0x0.7".into(), mode: Mode::Dense, initial_step: 0.7 * base_h0, max_step: base_hmax, extra_times: vec![] });
            policies.push(Policy { name: "dense_maxstep_grid".into(), mode: Mode::Dense, initial_step: base_h0, max_step: base_h0, extra_times: vec![] });
            policies.push(Policy { name: "clipped_h0x0.7".into(), mode: Mode::Clipped, initial_step: 0.7 * base_h0, max_step: base_hmax, extra_times: vec![] });
        }
        let mut arms: BTreeMap<String, ArmOut> = BTreeMap::new();
        for p in &policies {
            match execute_arm(&case, p, &bundle) {
                Ok(a) => { eprintln!("  {}: {} steps={} max_grid_wrms={:?}", p.name, a.status, a.internal_steps, a.metrics.as_ref().map(|m| m.max_grid_wrms)); arms.insert(p.name.clone(), a); }
                Err(e) => { eprintln!("  {}: ERROR {e}", p.name); }
            }
        }
        // E-03c: union-schedule dense run to capture accepted-step states of dense_base.
        if !args.e02_only {
            if let Some(db) = arms.get("dense_base") {
                if db.status == "success" {
                    let acc = accepted_times(spec.t_span.0, spec.t_span.1, &db.diagnostics.accepted_step_sizes);
                    let p = Policy { name: "dense_base_union_accepted".into(), mode: Mode::Dense, initial_step: base_h0, max_step: base_hmax, extra_times: acc };
                    match execute_arm(&case, &p, &bundle) {
                        Ok(a) => {
                            let same_steps = a.diagnostics.accepted_step_sizes.iter().zip(&db.diagnostics.accepted_step_sizes).all(|(x, y)| x.to_bits() == y.to_bits()) && a.diagnostics.accepted_step_sizes.len() == db.diagnostics.accepted_step_sizes.len();
                            eprintln!("  {}: {} steps={} same_step_sequence={} extra={}", p.name, a.status, a.internal_steps, same_steps, a.extra_times.len());
                            arms.insert(p.name.clone(), a);
                        }
                        Err(e) => eprintln!("  union run ERROR {e}"),
                    }
                }
            }
        }
        // E-02 verdict, campaign order: reference dominance on dense max_grid_wrms, then output policy gap.
        if let (Some(c), Some(d)) = (arms.get("clipped_base"), arms.get("dense_base")) {
            if let (Some(_cm), Some(dm)) = (&c.metrics, &d.metrics) {
                let refd = classify_reference_dominance(bundle.error_scale.reference_uncertainty_wrms, dm.max_grid_wrms)?;
                let gap = bundle.wrms_basis.discrepancy_wrms(&c.grid_times, &c.grid_states, &d.grid_times, &d.grid_states)?;
                let st = match refd {
                    rodas5p_fair_ab::ReferenceDominance::Dominated => "reference-dominated".to_string(),
                    rodas5p_fair_ab::ReferenceDominance::Admissible => match classify_output_policy_dominance(gap, dm.max_grid_wrms)? {
                        rodas5p_fair_ab::OutputPolicyDominance::Dominated => "output-policy-dominated".to_string(),
                        rodas5p_fair_ab::OutputPolicyDominance::Admissible => "pass".to_string(),
                    },
                };
                row.e02_status = st;
                row.e02_output_policy_discrepancy_wrms = Some(gap);
            } else {
                row.e02_status = "fail".into();
            }
        }
        let mut pairs = Vec::new();
        for (l, r, d) in [
            ("clipped_base", "dense_base", "dense_base"),
            ("dense_base", "dense_h0x0.7", "dense_base"),
            ("dense_base", "dense_maxstep_grid", "dense_base"),
            ("dense_h0x0.7", "dense_maxstep_grid", "dense_h0x0.7"),
            ("clipped_base", "clipped_h0x0.7", "clipped_base"),
            ("dense_base", "dense_base_union_accepted", "dense_base"),
        ] {
            if let Some(p) = pair(&bundle, &arms, l, r, d) { pairs.push(p); }
        }
        row.pairs = pairs;
        row.arms = arms.into_values().collect();
        rows.push(row);
    }
    let report = Report { compiled_revision: compiled, source_dirty_at_build: dirty, manifest: args.manifest.display().to_string(), manifest_sha256: manifest_sha, rows };
    std::fs::write(&args.out, serde_json::to_vec_pretty(&report)?)?;
    eprintln!("wrote {}", args.out.display());
    Ok(())
}
