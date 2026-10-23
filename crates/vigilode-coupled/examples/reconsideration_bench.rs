//! Interleaved, streaming, analytic-trajectory experiment for candidate recovery.
//! The selector sees neither case names nor analytic solutions.
use rodas5p_core::{DenseMatrix, LinearMethod, LinearSolverConfig, PreconditionerKind};
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, OdeProblem, OutputSchedule, ParallelExecution,
    Rodas5pFastBandedResult, Rodas5pFastResult, Rodas5pMfFastResult,
    integrate_rodas5p_fast_banded_observed, integrate_rodas5p_fast_observed,
    integrate_rodas5p_mf_fast_observed,
};
use serde_json::{Value, json};
use std::{hint::black_box, io::Write, sync::Arc, time::Instant};
use vigilode_coupled::{
    CandidateRoute, CausalSchedule, CausalSwitchPolicy, Config, CoupledIntegrationResult, RootErrorPolicy,
    integrate_causal_observed, integrate_coupled_observed, integrate_sequential_raw_observed,
    integrate_switching_causal_observed,
};

const ERROR_CAP: f64 = 1e-5;

#[derive(Clone)]
struct Model {
    name: String,
    n: usize,
    end: f64,
    injected_rhs_iterations: usize,
}
impl Model {
    fn new(name: &str) -> Result<Self, String> {
        let (n, end, injected_rhs_iterations) = match name {
            "few-mode" => (64, 0.2, 0),
            "expensive-few-mode" => (64, 0.2, 2048),
            "broad" => (48, 0.1, 0),
            "riccati" => (32, 1.0, 0),
            "nonnormal" => (32, 0.2, 0),
            "oscillatory" => (32, 0.02, 0),
            "changing-rate" => (33, 0.5, 0),
            _ => return Err(format!("unknown registered case: {name}")),
        };
        Ok(Self { name: name.into(), n, end, injected_rhs_iterations })
    }
    fn rate(&self, i: usize) -> f64 {
        match self.name.as_str() {
            "broad" => -10_f64.powf(4.0 * i as f64 / (self.n - 1) as f64),
            "riccati" => [-1.0, -2.0, -3.0][i % 3],
            _ => [-1.0, -20.0, -100.0][i % 3],
        }
    }
    fn clock_rates(&self, i: usize) -> (f64, f64) {
        (1.0 + (i % 3) as f64, 200.0 * i as f64 / (self.n - 1) as f64)
    }
    fn initial(&self) -> Vec<f64> { self.exact(0.0) }
    fn exact(&self, t: f64) -> Vec<f64> {
        (0..self.n).map(|i| match self.name.as_str() {
            "nonnormal" if i % 2 == 0 => 5.0 * ((-t).exp() - (-20.0 * t).exp()) / 19.0,
            "nonnormal" => 0.1 * (-20.0 * t).exp(),
            "oscillatory" => (-0.1 * t).exp() * if i % 2 == 0 {
                (1000.0 * t).cos()
            } else { (1000.0 * t).sin() },
            "riccati" => {
                let a = self.rate(i);
                let e = (a * t).exp();
                0.1 * e / (1.0 + 0.05 / a * (1.0 - e))
            }
            "changing-rate" if i == 0 => t,
            "changing-rate" => {
                let (a, b) = self.clock_rates(i);
                0.1 * (-a * t - b * (-(-40.0 * t).exp_m1()) / 40.0).exp()
            }
            _ => 0.1 * (self.rate(i) * t).exp(),
        }).collect()
    }
    fn rhs(&self, y: &[f64], out: &mut [f64]) {
        // Synthetic callback-cost control: the mathematical RHS is unchanged.
        // black_box prevents optimization from deleting the deliberate work.
        if self.injected_rhs_iterations != 0 {
            let mut value = black_box(y[0]);
            for k in 0..self.injected_rhs_iterations {
                value = black_box((value + (k as f64 + 1.0) * 1e-6).sin());
            }
            black_box(value);
        }
        match self.name.as_str() {
            "nonnormal" => for i in (0..self.n).step_by(2) {
                out[i] = -y[i] + 50.0 * y[i + 1];
                out[i + 1] = -20.0 * y[i + 1];
            },
            "oscillatory" => for i in (0..self.n).step_by(2) {
                out[i] = -0.1 * y[i] - 1000.0 * y[i + 1];
                out[i + 1] = 1000.0 * y[i] - 0.1 * y[i + 1];
            },
            "changing-rate" => {
                out[0] = 1.0;
                let decay = (-40.0 * y[0]).exp();
                for i in 1..self.n {
                    let (a, b) = self.clock_rates(i);
                    out[i] = -(a + b * decay) * y[i];
                }
            }
            _ => for i in 0..self.n {
                out[i] = self.rate(i) * y[i] + if self.name == "riccati" {
                    0.5 * y[i] * y[i]
                } else { 0.0 };
            },
        }
    }
    fn jacobian_entry(&self, row: usize, col: usize, y: &[f64]) -> f64 {
        match self.name.as_str() {
            "nonnormal" if row == col => if row % 2 == 0 { -1.0 } else { -20.0 },
            "nonnormal" if row % 2 == 0 && col == row + 1 => 50.0,
            "nonnormal" => 0.0,
            "oscillatory" if row == col => -0.1,
            "oscillatory" if row % 2 == 0 && col == row + 1 => -1000.0,
            "oscillatory" if row % 2 == 1 && col + 1 == row => 1000.0,
            "oscillatory" => 0.0,
            "changing-rate" if row == 0 => 0.0,
            "changing-rate" => {
                let (a, b) = self.clock_rates(row);
                let decay = (-40.0 * y[0]).exp();
                if col == 0 { 40.0 * b * decay * y[row] }
                else if row == col { -(a + b * decay) } else { 0.0 }
            }
            _ if row == col => self.rate(row) + if self.name == "riccati" { y[row] } else { 0.0 },
            _ => 0.0,
        }
    }
    fn jvp(&self, y: &[f64], v: &[f64], out: &mut [f64]) {
        match self.name.as_str() {
            "changing-rate" => {
                out[0] = 0.0;
                for i in 1..self.n {
                    out[i] = self.jacobian_entry(i, i, y) * v[i]
                        + self.jacobian_entry(i, 0, y) * v[0];
                }
            }
            "nonnormal" | "oscillatory" => for i in (0..self.n).step_by(2) {
                out[i] = self.jacobian_entry(i, i, y) * v[i]
                    + self.jacobian_entry(i, i + 1, y) * v[i + 1];
                out[i + 1] = self.jacobian_entry(i + 1, i, y) * v[i]
                    + self.jacobian_entry(i + 1, i + 1, y) * v[i + 1];
            },
            _ => for i in 0..self.n { out[i] = self.jacobian_entry(i, i, y) * v[i]; },
        }
    }
    fn problem(&self) -> OdeProblem {
        let (rhs, jvp, jac) = (self.clone(), self.clone(), self.clone());
        OdeProblem::new(self.name.clone(), self.n,
            Arc::new(move |_, y, out| { rhs.rhs(y, out); Ok(()) }), None,
            Some(Arc::new(move |_, y| {
                let mut j = DenseMatrix::zeros(jac.n, jac.n);
                for row in 0..jac.n { for col in 0..jac.n {
                    j[(row, col)] = jac.jacobian_entry(row, col, y);
                }}
                Ok(j)
            })),
            Some(Arc::new(move |_, y, v, out| { jvp.jvp(y, v, out); Ok(()) })),
            None, true, None, None).expect("registered problem")
    }
    fn band(&self) -> Option<BandedJacobian> {
        if self.name == "changing-rate" { return None; }
        let lower = usize::from(self.name == "oscillatory");
        let upper = usize::from(self.name == "oscillatory" || self.name == "nonnormal");
        let m = self.clone();
        Some(BandedJacobian { lower, upper, fill: Arc::new(move |_, y, out| {
            for row in 0..m.n {
                for col in row.saturating_sub(lower)..=(row + upper).min(m.n - 1) {
                    out[row * (lower + upper + 1) + col + lower - row] = m.jacobian_entry(row, col, y);
                }
            }
            Ok(())
        }) })
    }
    fn errors(&self, times: &[f64], states: &[Vec<f64>], a: &AdaptiveStepConfig) -> Value {
        if times.len() != states.len() || states.iter().any(|s| s.len() != self.n || s.iter().any(|x| !x.is_finite())) {
            return json!({"finite":false,"error_pass":false});
        }
        let mut maximum: f64 = 0.0;
        let mut max_scaled: f64 = 0.0;
        let mut sampled = Vec::new();
        for (&t, state) in times.iter().zip(states) {
            let reference = self.exact(t);
            let mut abs: f64 = 0.0;
            let mut sum = 0.0;
            for (&actual, &expected) in state.iter().zip(&reference) {
                let e = (actual - expected).abs();
                abs = abs.max(e);
                sum += (e / (a.atol + a.rtol * expected.abs())).powi(2);
            }
            let scaled = (sum / self.n as f64).sqrt();
            maximum = maximum.max(abs);
            max_scaled = max_scaled.max(scaled);
            sampled.push(json!({"t":t,"max_abs_error":abs,"scaled_wrms":scaled}));
        }
        json!({"finite":maximum.is_finite()&&max_scaled.is_finite(),
            "error_pass":maximum<=ERROR_CAP&&max_scaled.is_finite(),
            "max_abs_error":maximum,"max_scaled_wrms":max_scaled,"sampled_errors":sampled})
    }
}

enum SolveResult {
    Coupled(CoupledIntegrationResult),
    Dense(Rodas5pFastResult),
    Band(Rodas5pFastBandedResult),
    Mf(Rodas5pMfFastResult),
}
fn linear_config() -> LinearSolverConfig {
    LinearSolverConfig { method: LinearMethod::Gmres, preconditioner: PreconditionerKind::None,
        rtol: 1e-12, atol: 1e-14, maxiter: 256, restart: 64, ..Default::default() }
}
#[allow(clippy::too_many_arguments)]
fn solve(mode: &str, m: &Model, p: &OdeProblem, band: Option<&BandedJacobian>,
    initial: &[f64], adaptive: &AdaptiveStepConfig, output: &OutputSchedule,
    config: &Config, execution: &ParallelExecution) -> Result<SolveResult, String> {
    let span = (0.0, m.end);
    let linear = linear_config();
    match mode {
        "fixed-causal" => integrate_causal_observed(p, span, initial, adaptive, output, config, execution,
            &linear, &RootErrorPolicy::Operational, CausalSchedule::Forward).map(SolveResult::Coupled),
        "switching-causal" | "legacy-switching-causal" | "switching-picard" | "switching-residual-first" => {
            let mut policy = CausalSwitchPolicy { candidate: match mode {
                "switching-picard" => CandidateRoute::Picard,
                "switching-residual-first" => CandidateRoute::ResidualFirst,
                _ => CandidateRoute::Forward,
            }, ..Default::default() };
            if mode == "legacy-switching-causal" { policy.exploration_overhead_fraction = None; }
            integrate_switching_causal_observed(p, span, initial, adaptive, output, config,
                execution, &linear, &RootErrorPolicy::Operational, &policy).map(SolveResult::Coupled)
        }
        "fixed-picard" => integrate_causal_observed(p, span, initial, adaptive, output, config, execution,
            &linear, &RootErrorPolicy::Operational, CausalSchedule::BatchedPicard).map(SolveResult::Coupled),
        "fixed-residual-first" => integrate_coupled_observed(p, span, initial, adaptive, output, config,
            execution, &linear, &RootErrorPolicy::Operational).map(SolveResult::Coupled),
        "protected-raw" => integrate_sequential_raw_observed(p, span, initial, adaptive, output, config,
            execution, &linear, &RootErrorPolicy::Operational).map(SolveResult::Coupled),
        "protected-mf" => integrate_rodas5p_mf_fast_observed(p, span, initial, &linear, adaptive, output).map(SolveResult::Mf),
        "protected-dense" => integrate_rodas5p_fast_observed(p, span, initial, adaptive, output).map(SolveResult::Dense),
        "protected-band" => integrate_rodas5p_fast_banded_observed(p, band.expect("registered band"), span,
            initial, adaptive, output).map(SolveResult::Band),
        _ => unreachable!(),
    }.map_err(|e| e.to_string())
}
fn report(result: Result<SolveResult, String>, m: &Model, a: &AdaptiveStepConfig, output: &OutputSchedule) -> Value {
    let r = match result { Ok(r) => r, Err(e) => return json!({"success":false,"error":e}) };
    let (observed, extra) = match r {
        SolveResult::Coupled(r) => {
            let attempts = serde_json::to_value(&r.attempts).unwrap();
            let rows = attempts.as_array().unwrap();
            let skipped = rows.iter().filter(|v| v.get("candidate_skipped").and_then(Value::as_bool)==Some(true)).count();
            let mut decisions = std::collections::BTreeMap::new();
            for row in rows {
                let key = row.get("switch_decision").unwrap_or(&Value::Null).to_string();
                *decisions.entry(key).or_insert(0_usize) += 1;
            }
            let extra = json!({"driver":r.driver,"attempts":rows.len(),"accepted":r.attempts.iter().filter(|a|a.accepted).count(),
                "candidate_skipped":skipped,"candidate_evaluations":rows.len()-skipped,
                "fallback_attempts":r.attempts.iter().filter(|a|a.fallback_attempted).count(),
                "candidate_failures":r.attempts.iter().filter(|a|a.candidate_failure.is_some()).count(),
                "switch_decisions":decisions,"terminal_failure":r.terminal_failure,
                "candidate_rhs":r.attempts.iter().map(|a|a.candidate_work.rhs_attempted).sum::<u64>(),
                "candidate_jvp":r.attempts.iter().map(|a|a.candidate_work.jvp_attempted).sum::<u64>()});
            (r.observed, extra)
        }
        SolveResult::Dense(r) => (r.observed, json!({"driver":r.driver,"attempts":r.attempts,"accepted":r.accepted_steps,"rejected":r.rejected_steps,"lu":format!("{:?}",r.lu)})),
        SolveResult::Band(r) => (r.fast.observed, json!({"driver":r.fast.driver,"attempts":r.fast.attempts,"accepted":r.fast.accepted_steps,"rejected":r.fast.rejected_steps,"band_work":r.work})),
        SolveResult::Mf(r) => (r.observed, json!({"driver":r.driver,"attempts":r.attempts,"accepted":r.accepted_steps,"rejected":r.rejected_steps,"partial":r.partial})),
    };
    let errors = m.errors(&observed.t, &observed.y, a);
    let complete = observed.t == output.times();
    json!({"success":observed.success&&complete&&errors["error_pass"]==true,
        "solver_success":observed.success,"complete":complete,"message":observed.message,
        "errors":errors,"work":extra,"counters":observed.counters})
}
fn emit(v: Value) {
    println!("{v}");
    std::io::stdout().flush().expect("benchmark receipt output");
}
fn main() {
    if let Err(e) = run() { emit(json!({"kind":"configuration_error","error":e})); std::process::exit(2); }
}
fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |name: &str, default: &str| args.windows(2).find(|w|w[0]==name).map(|w|w[1].clone()).unwrap_or_else(||default.into());
    let case = value("--case", "few-mode");
    let basis_limit: usize = value("--basis-limit", "8").parse().map_err(|_|"invalid basis limit")?;
    if !(1..=8).contains(&basis_limit) { return Err("registered basis limit is 1..8".into()); }
    let case_id = if basis_limit == 8 { case.clone() } else { format!("{case}-basis{basis_limit}") };
    let root_variants = args.iter().any(|s| s == "--root-variants");
    let threads: usize = value("--threads", "1").parse().map_err(|_|"invalid threads")?;
    if threads != 1 && !(root_variants && threads == 2) { return Err("primary experiment requires one thread; root variants permit one or two".into()); }
    let repeats: usize = value("--repeats", "5").parse().map_err(|_|"invalid repeats")?;
    if !(1..=9).contains(&repeats) { return Err("repeats must be 1..9".into()); }
    let m = Model::new(&case)?;
    let p = m.problem();
    let band = m.band();
    let initial = m.initial();
    let adaptive = AdaptiveStepConfig { rtol:1e-6, atol:1e-8, initial_step:1e-3,
        max_step:m.end, max_attempts:5000, ..Default::default() };
    let output = OutputSchedule::new((0..=16).map(|i| m.end*i as f64/16.0).collect()).map_err(|e|e.to_string())?;
    let config = Config { initial_basis:3.min(basis_limit), max_basis:basis_limit,
        max_expansions:if basis_limit == 1 {0} else {2}, max_batches:64, ..Default::default() };
    let execution = if threads == 1 { ParallelExecution::sequential() }
        else { ParallelExecution::rayon(threads).map_err(|e|e.to_string())? };
    let mut modes = vec!["fixed-causal", "switching-causal", "legacy-switching-causal", "protected-raw", "protected-mf", "protected-dense"];
    if root_variants {
        modes = vec!["fixed-causal", "switching-causal", "fixed-picard", "switching-picard",
            "fixed-residual-first", "switching-residual-first", "protected-raw", "protected-dense"];
    } else if band.is_some() { modes.push("protected-band"); }
    emit(json!({"kind":"header","schema":"vigilode.candidate-reconsideration-benchmark.v1",
        "case":case_id,"registered_model":case,"basis_limit":basis_limit,"dimension":m.n,"end":m.end,"threads":threads,"repeats":repeats,"modes":modes,
        "campaign":if root_variants {"root-variants"} else {"primary"},
        "rtol":adaptive.rtol,"atol":adaptive.atol,"max_abs_error_cap":ERROR_CAP,"observations":output.times(),
        "source_commit":std::env::var("VIGILODE_SOURCE_COMMIT").unwrap_or_else(|_|"UNSET".into()),
        "injected_rhs_iterations":m.injected_rhs_iterations,"candidate_config":config,
        "switch_policy":CausalSwitchPolicy::default(),"reference":"analytic at physical output landings",
        "timing":"complete solve including setup, probes, rejection, fallback and receipts; excludes model/reference/JSON",
        "order":"cyclic interleaved; one warmup per method"}));
    for mode in &modes {
        let result = solve(mode, &m, &p, band.as_ref(), &initial, &adaptive, &output, &config, &execution);
        emit(json!({"kind":"warmup","case":case_id,"mode":mode,"result":report(result,&m,&adaptive,&output)}));
    }
    for repeat in 0..repeats {
        for slot in 0..modes.len() {
            let mode = modes[(slot+repeat)%modes.len()];
            let start = Instant::now();
            let result = solve(mode, &m, &p, band.as_ref(), &initial, &adaptive, &output, &config, &execution);
            let seconds = start.elapsed().as_secs_f64();
            emit(json!({"kind":"sample","case":case_id,"mode":mode,"repeat":repeat,"slot":slot,
                "elapsed_seconds":seconds,"result":report(result,&m,&adaptive,&output)}));
        }
    }
    Ok(())
}
