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
    CandidateRoute, CausalReferenceGuard, CausalSchedule, CausalSwitchPolicy, Config,
    CoupledIntegrationResult, RootErrorPolicy, integrate_causal_observed,
    integrate_coupled_observed, integrate_sequential_raw_observed,
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
            "nonlinear-changing-rank" | "nonlinear-step-regime" => (33, 0.5, 0),
            "meanfield-manufactured" => (65, 0.2, 0),
            _ => return Err(format!("unknown registered case: {name}")),
        };
        Ok(Self {
            name: name.into(),
            n,
            end,
            injected_rhs_iterations,
        })
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
    fn changing_rank(&self) -> bool {
        matches!(
            self.name.as_str(),
            "nonlinear-changing-rank" | "nonlinear-step-regime"
        )
    }
    // C-infinity transition with exactly constant outer intervals. No
    // numerical integral enters the manufactured reference trajectory.
    fn transition(&self, t: f64) -> (f64, f64) {
        let x = (t - 0.1) / 0.2;
        let (value, derivative) = if x <= 0.0 {
            (0.0, 0.0)
        } else if x >= 1.0 {
            (1.0, 0.0)
        } else {
            let a = (-1.0 / x).exp();
            let b = (-1.0 / (1.0 - x)).exp();
            let v = a / (a + b);
            let dv = if v == 0.0 || v == 1.0 {
                0.0
            } else {
                v * (1.0 - v) * (1.0 / (x * x) + 1.0 / ((1.0 - x) * (1.0 - x))) / 0.2
            };
            (v, dv)
        };
        if self.name == "nonlinear-step-regime" {
            (1.0 - value, -derivative)
        } else {
            (value, derivative)
        }
    }
    fn nonlinear_exact(&self, t: f64, i: usize) -> f64 {
        0.1 * (-self.clock_rates(i).0 * t).exp()
    }
    fn phase(&self, t: f64, i: usize) -> f64 {
        0.15 * (i % 7) as f64 + self.phase_speed(i) * t + 0.05 * (self.phase_frequency(i) * t).sin()
    }
    fn phase_speed(&self, i: usize) -> f64 {
        0.5 + 0.025 * (i % 11) as f64
    }
    fn phase_frequency(&self, i: usize) -> f64 {
        2.0 + (i % 5) as f64
    }
    fn phase_derivative(&self, t: f64, i: usize) -> f64 {
        let nu = self.phase_frequency(i);
        self.phase_speed(i) + 0.05 * nu * (nu * t).cos()
    }
    fn phase_curvature(&self, t: f64, i: usize) -> f64 {
        let nu = self.phase_frequency(i);
        -0.05 * nu * nu * (nu * t).sin()
    }
    fn coupling(&self, i: usize, j: usize) -> f64 {
        // A nonuniform, nonseparable pair kernel: no synthetic busy work.
        10.0 / (self.n - 1) as f64 / (1.0 + (i.abs_diff(j) as f64 / 8.0).powi(2))
    }
    fn initial(&self) -> Vec<f64> {
        self.exact(0.0)
    }
    fn exact(&self, t: f64) -> Vec<f64> {
        (0..self.n)
            .map(|i| match self.name.as_str() {
                "nonlinear-changing-rank" | "nonlinear-step-regime" if i == 0 => t,
                "nonlinear-changing-rank" | "nonlinear-step-regime" => self.nonlinear_exact(t, i),
                "meanfield-manufactured" if i == 0 => t,
                "meanfield-manufactured" => self.phase(t, i),
                "nonnormal" if i % 2 == 0 => 5.0 * ((-t).exp() - (-20.0 * t).exp()) / 19.0,
                "nonnormal" => 0.1 * (-20.0 * t).exp(),
                "oscillatory" => {
                    (-0.1 * t).exp()
                        * if i % 2 == 0 {
                            (1000.0 * t).cos()
                        } else {
                            (1000.0 * t).sin()
                        }
                }
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
            })
            .collect()
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
            "nonlinear-changing-rank" | "nonlinear-step-regime" => {
                out[0] = 1.0;
                let (s, _) = self.transition(y[0]);
                for i in 1..self.n {
                    let (a, b) = self.clock_rates(i);
                    let exact = self.nonlinear_exact(y[0], i);
                    let delta = y[i] - exact;
                    out[i] = -(a + b * s) * delta - a * exact + 5.0 * delta * delta;
                }
            }
            "meanfield-manufactured" => {
                out[0] = 1.0;
                for i in 1..self.n {
                    let mut value = self.phase_derivative(y[0], i);
                    for j in 1..self.n {
                        if i != j {
                            value += self.coupling(i, j)
                                * ((y[j] - y[i]).sin()
                                    - (self.phase(y[0], j) - self.phase(y[0], i)).sin());
                        }
                    }
                    out[i] = value;
                }
            }
            "nonnormal" => {
                for i in (0..self.n).step_by(2) {
                    out[i] = -y[i] + 50.0 * y[i + 1];
                    out[i + 1] = -20.0 * y[i + 1];
                }
            }
            "oscillatory" => {
                for i in (0..self.n).step_by(2) {
                    out[i] = -0.1 * y[i] - 1000.0 * y[i + 1];
                    out[i + 1] = 1000.0 * y[i] - 0.1 * y[i + 1];
                }
            }
            "changing-rate" => {
                out[0] = 1.0;
                let decay = (-40.0 * y[0]).exp();
                for i in 1..self.n {
                    let (a, b) = self.clock_rates(i);
                    out[i] = -(a + b * decay) * y[i];
                }
            }
            _ => {
                for i in 0..self.n {
                    out[i] = self.rate(i) * y[i]
                        + if self.name == "riccati" {
                            0.5 * y[i] * y[i]
                        } else {
                            0.0
                        };
                }
            }
        }
    }
    fn jacobian_entry(&self, row: usize, col: usize, y: &[f64]) -> f64 {
        match self.name.as_str() {
            "nonlinear-changing-rank" | "nonlinear-step-regime" if row == 0 => 0.0,
            "nonlinear-changing-rank" | "nonlinear-step-regime" => {
                let (a, b) = self.clock_rates(row);
                let (s, ds) = self.transition(y[0]);
                let exact = self.nonlinear_exact(y[0], row);
                if col == 0 {
                    -b * ds * (y[row] - exact) + (b * s - 10.0 * (y[row] - exact)) * (-a * exact)
                } else if row == col {
                    -(a + b * s) + 10.0 * (y[row] - exact)
                } else {
                    0.0
                }
            }
            "meanfield-manufactured" if row == 0 => 0.0,
            "meanfield-manufactured" if col == 0 => {
                self.phase_curvature(y[0], row)
                    - (1..self.n)
                        .filter(|&j| j != row)
                        .map(|j| {
                            self.coupling(row, j)
                                * (self.phase(y[0], j) - self.phase(y[0], row)).cos()
                                * (self.phase_derivative(y[0], j)
                                    - self.phase_derivative(y[0], row))
                        })
                        .sum::<f64>()
            }
            "meanfield-manufactured" if row == col => -(1..self.n)
                .filter(|&j| j != row)
                .map(|j| self.coupling(row, j) * (y[j] - y[row]).cos())
                .sum::<f64>(),
            "meanfield-manufactured" => self.coupling(row, col) * (y[col] - y[row]).cos(),
            "nonnormal" if row == col => {
                if row.is_multiple_of(2) {
                    -1.0
                } else {
                    -20.0
                }
            }
            "nonnormal" if row.is_multiple_of(2) && col == row + 1 => 50.0,
            "nonnormal" => 0.0,
            "oscillatory" if row == col => -0.1,
            "oscillatory" if row.is_multiple_of(2) && col == row + 1 => -1000.0,
            "oscillatory" if row % 2 == 1 && col + 1 == row => 1000.0,
            "oscillatory" => 0.0,
            "changing-rate" if row == 0 => 0.0,
            "changing-rate" => {
                let (a, b) = self.clock_rates(row);
                let decay = (-40.0 * y[0]).exp();
                if col == 0 {
                    40.0 * b * decay * y[row]
                } else if row == col {
                    -(a + b * decay)
                } else {
                    0.0
                }
            }
            _ if row == col => self.rate(row) + if self.name == "riccati" { y[row] } else { 0.0 },
            _ => 0.0,
        }
    }
    fn jvp(&self, y: &[f64], v: &[f64], out: &mut [f64]) {
        match self.name.as_str() {
            "meanfield-manufactured" => {
                out[0] = 0.0;
                for i in 1..self.n {
                    let mut value = self.phase_curvature(y[0], i) * v[0];
                    for j in 1..self.n {
                        if i != j {
                            value += self.coupling(i, j)
                                * ((y[j] - y[i]).cos() * (v[j] - v[i])
                                    - (self.phase(y[0], j) - self.phase(y[0], i)).cos()
                                        * (self.phase_derivative(y[0], j)
                                            - self.phase_derivative(y[0], i))
                                        * v[0]);
                        }
                    }
                    out[i] = value;
                }
            }
            "changing-rate" | "nonlinear-changing-rank" | "nonlinear-step-regime" => {
                out[0] = 0.0;
                for i in 1..self.n {
                    out[i] =
                        self.jacobian_entry(i, i, y) * v[i] + self.jacobian_entry(i, 0, y) * v[0];
                }
            }
            "nonnormal" | "oscillatory" => {
                for i in (0..self.n).step_by(2) {
                    out[i] = self.jacobian_entry(i, i, y) * v[i]
                        + self.jacobian_entry(i, i + 1, y) * v[i + 1];
                    out[i + 1] = self.jacobian_entry(i + 1, i, y) * v[i]
                        + self.jacobian_entry(i + 1, i + 1, y) * v[i + 1];
                }
            }
            _ => {
                for i in 0..self.n {
                    out[i] = self.jacobian_entry(i, i, y) * v[i];
                }
            }
        }
    }
    fn problem(&self) -> OdeProblem {
        let (rhs, jvp, jac) = (self.clone(), self.clone(), self.clone());
        OdeProblem::new(
            self.name.clone(),
            self.n,
            Arc::new(move |_, y, out| {
                rhs.rhs(y, out);
                Ok(())
            }),
            None,
            Some(Arc::new(move |_, y| {
                let mut j = DenseMatrix::zeros(jac.n, jac.n);
                for row in 0..jac.n {
                    for col in 0..jac.n {
                        j[(row, col)] = jac.jacobian_entry(row, col, y);
                    }
                }
                Ok(j)
            })),
            Some(Arc::new(move |_, y, v, out| {
                jvp.jvp(y, v, out);
                Ok(())
            })),
            None,
            true,
            None,
            None,
        )
        .expect("registered problem")
    }
    fn band(&self) -> Option<BandedJacobian> {
        if self.name == "changing-rate"
            || self.changing_rank()
            || self.name == "meanfield-manufactured"
        {
            return None;
        }
        let lower = usize::from(self.name == "oscillatory");
        let upper = usize::from(self.name == "oscillatory" || self.name == "nonnormal");
        let m = self.clone();
        Some(BandedJacobian {
            lower,
            upper,
            fill: Arc::new(move |_, y, out| {
                for row in 0..m.n {
                    for col in row.saturating_sub(lower)..=(row + upper).min(m.n - 1) {
                        out[row * (lower + upper + 1) + col + lower - row] =
                            m.jacobian_entry(row, col, y);
                    }
                }
                Ok(())
            }),
        })
    }
    fn errors(&self, times: &[f64], states: &[Vec<f64>], a: &AdaptiveStepConfig) -> Value {
        if times.len() != states.len()
            || states
                .iter()
                .any(|s| s.len() != self.n || s.iter().any(|x| !x.is_finite()))
        {
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
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        preconditioner: PreconditionerKind::None,
        rtol: 1e-12,
        atol: 1e-14,
        maxiter: 256,
        restart: 64,
        ..Default::default()
    }
}
#[allow(clippy::too_many_arguments)]
fn solve(
    mode: &str,
    m: &Model,
    p: &OdeProblem,
    band: Option<&BandedJacobian>,
    initial: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    config: &Config,
    execution: &ParallelExecution,
) -> Result<SolveResult, String> {
    let span = (0.0, m.end);
    let linear = linear_config();
    match mode {
        "fixed-causal" => integrate_causal_observed(
            p,
            span,
            initial,
            adaptive,
            output,
            config,
            execution,
            &linear,
            &RootErrorPolicy::Operational,
            CausalSchedule::Forward,
        )
        .map(SolveResult::Coupled),
        "switching-causal"
        | "guarded-switching-causal"
        | "legacy-switching-causal"
        | "switching-picard"
        | "switching-residual-first" => {
            let mut policy = CausalSwitchPolicy {
                candidate: match mode {
                    "switching-picard" => CandidateRoute::Picard,
                    "switching-residual-first" => CandidateRoute::ResidualFirst,
                    _ => CandidateRoute::Forward,
                },
                ..Default::default()
            };
            if mode == "legacy-switching-causal" {
                policy.exploration_overhead_fraction = None;
            }
            if mode == "guarded-switching-causal" {
                policy.reference_guard = Some(CausalReferenceGuard::default());
            }
            integrate_switching_causal_observed(
                p,
                span,
                initial,
                adaptive,
                output,
                config,
                execution,
                &linear,
                &RootErrorPolicy::Operational,
                &policy,
            )
            .map(SolveResult::Coupled)
        }
        "fixed-picard" => integrate_causal_observed(
            p,
            span,
            initial,
            adaptive,
            output,
            config,
            execution,
            &linear,
            &RootErrorPolicy::Operational,
            CausalSchedule::BatchedPicard,
        )
        .map(SolveResult::Coupled),
        "fixed-residual-first" => integrate_coupled_observed(
            p,
            span,
            initial,
            adaptive,
            output,
            config,
            execution,
            &linear,
            &RootErrorPolicy::Operational,
        )
        .map(SolveResult::Coupled),
        "protected-raw" => integrate_sequential_raw_observed(
            p,
            span,
            initial,
            adaptive,
            output,
            config,
            execution,
            &linear,
            &RootErrorPolicy::Operational,
        )
        .map(SolveResult::Coupled),
        "protected-mf" => {
            integrate_rodas5p_mf_fast_observed(p, span, initial, &linear, adaptive, output)
                .map(SolveResult::Mf)
        }
        "protected-dense" => integrate_rodas5p_fast_observed(p, span, initial, adaptive, output)
            .map(SolveResult::Dense),
        "protected-band" => integrate_rodas5p_fast_banded_observed(
            p,
            band.expect("registered band"),
            span,
            initial,
            adaptive,
            output,
        )
        .map(SolveResult::Band),
        _ => unreachable!(),
    }
    .map_err(|e| e.to_string())
}
fn report(
    result: Result<SolveResult, String>,
    m: &Model,
    a: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> Value {
    let r = match result {
        Ok(r) => r,
        Err(e) => return json!({"success":false,"error":e}),
    };
    let (observed, extra) = match r {
        SolveResult::Coupled(r) => {
            let attempts = serde_json::to_value(&r.attempts).unwrap();
            let rows = attempts.as_array().unwrap();
            let skipped = rows
                .iter()
                .filter(|v| v.get("candidate_skipped").and_then(Value::as_bool) == Some(true))
                .count();
            let mut decisions = std::collections::BTreeMap::new();
            for row in rows {
                let key = row
                    .get("switch_decision")
                    .unwrap_or(&Value::Null)
                    .to_string();
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
        SolveResult::Dense(r) => (
            r.observed,
            json!({"driver":r.driver,"attempts":r.attempts,"accepted":r.accepted_steps,"rejected":r.rejected_steps,"lu":format!("{:?}",r.lu)}),
        ),
        SolveResult::Band(r) => (
            r.fast.observed,
            json!({"driver":r.fast.driver,"attempts":r.fast.attempts,"accepted":r.fast.accepted_steps,"rejected":r.fast.rejected_steps,"band_work":r.work}),
        ),
        SolveResult::Mf(r) => (
            r.observed,
            json!({"driver":r.driver,"attempts":r.attempts,"accepted":r.accepted_steps,"rejected":r.rejected_steps,"partial":r.partial}),
        ),
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
    if let Err(e) = run() {
        emit(json!({"kind":"configuration_error","error":e}));
        std::process::exit(2);
    }
}
fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |name: &str, default: &str| {
        args.windows(2)
            .find(|w| w[0] == name)
            .map(|w| w[1].clone())
            .unwrap_or_else(|| default.into())
    };
    let case = value("--case", "few-mode");
    let basis_limit: usize = value("--basis-limit", "8")
        .parse()
        .map_err(|_| "invalid basis limit")?;
    if !(1..=8).contains(&basis_limit) {
        return Err("registered basis limit is 1..8".into());
    }
    let case_id = if basis_limit == 8 {
        case.clone()
    } else {
        format!("{case}-basis{basis_limit}")
    };
    let root_variants = args.iter().any(|s| s == "--root-variants");
    let reference_guard = args.iter().any(|s| s == "--reference-guard");
    let threads: usize = value("--threads", "1")
        .parse()
        .map_err(|_| "invalid threads")?;
    if threads != 1 && !(root_variants && [2, 4, 8, 12].contains(&threads)) {
        return Err(
            "primary experiment requires one thread; root variants permit 1/2/4/8/12".into(),
        );
    }
    let repeats: usize = value("--repeats", "5")
        .parse()
        .map_err(|_| "invalid repeats")?;
    if !(1..=9).contains(&repeats) {
        return Err("repeats must be 1..9".into());
    }
    let m = Model::new(&case)?;
    let p = m.problem();
    let band = m.band();
    let initial = m.initial();
    let adaptive = AdaptiveStepConfig {
        rtol: 1e-6,
        atol: 1e-8,
        initial_step: 1e-3,
        max_step: m.end,
        max_attempts: 5000,
        ..Default::default()
    };
    let output = OutputSchedule::new((0..=16).map(|i| m.end * i as f64 / 16.0).collect())
        .map_err(|e| e.to_string())?;
    let config = Config {
        initial_basis: 3.min(basis_limit),
        max_basis: basis_limit,
        max_expansions: if basis_limit == 1 { 0 } else { 2 },
        max_batches: 64,
        ..Default::default()
    };
    let pool_start = Instant::now();
    let execution = if threads == 1 {
        ParallelExecution::sequential()
    } else {
        ParallelExecution::rayon(threads).map_err(|e| e.to_string())?
    };
    let pool_startup_seconds = pool_start.elapsed().as_secs_f64();
    let mut modes = vec![
        "fixed-causal",
        "switching-causal",
        "legacy-switching-causal",
        "protected-raw",
        "protected-mf",
        "protected-dense",
    ];
    if root_variants {
        modes = vec![
            "fixed-causal",
            "switching-causal",
            "fixed-picard",
            "switching-picard",
            "fixed-residual-first",
            "switching-residual-first",
            "protected-raw",
            "protected-dense",
        ];
    } else if band.is_some() {
        modes.push("protected-band");
    }
    if reference_guard {
        modes.push("guarded-switching-causal");
    }
    emit(
        json!({"kind":"header","schema":"vigilode.candidate-reconsideration-benchmark.v1",
        "case":case_id,"registered_model":case,"basis_limit":basis_limit,"dimension":m.n,"end":m.end,"threads":threads,"repeats":repeats,"modes":modes,
        "campaign":if root_variants {"root-variants"} else {"primary"},
        "pool_startup_seconds":pool_startup_seconds,"reference_guard_enabled":reference_guard,
        "guard_policy":if reference_guard {Some(CausalReferenceGuard::default())} else {None},
        "model_scope":if m.name=="meanfield-manufactured" {"nonlinear pair-coupled phase dynamics with manufactured clock-dependent forcing; forcing charged in RHS"} else if m.changing_rank() {"nonlinear clock-augmented smooth rank/step-regime manufactured holdout"} else {"previous registered analytic case"},
        "rtol":adaptive.rtol,"atol":adaptive.atol,"max_abs_error_cap":ERROR_CAP,"observations":output.times(),
        "source_commit":std::env::var("VIGILODE_SOURCE_COMMIT").unwrap_or_else(|_|"UNSET".into()),
        "injected_rhs_iterations":m.injected_rhs_iterations,"candidate_config":config,
        "switch_policy":CausalSwitchPolicy::default(),"reference":"analytic at physical output landings",
        "timing":"complete solve including setup, probes, rejection, fallback and receipts; excludes model/reference/JSON",
        "order":"cyclic interleaved; one warmup per method"}),
    );
    for mode in &modes {
        let result = solve(
            mode,
            &m,
            &p,
            band.as_ref(),
            &initial,
            &adaptive,
            &output,
            &config,
            &execution,
        );
        emit(
            json!({"kind":"warmup","case":case_id,"mode":mode,"result":report(result,&m,&adaptive,&output)}),
        );
    }
    for repeat in 0..repeats {
        for slot in 0..modes.len() {
            let mode = modes[(slot + repeat) % modes.len()];
            let start = Instant::now();
            let result = solve(
                mode,
                &m,
                &p,
                band.as_ref(),
                &initial,
                &adaptive,
                &output,
                &config,
                &execution,
            );
            let seconds = start.elapsed().as_secs_f64();
            emit(
                json!({"kind":"sample","case":case_id,"mode":mode,"repeat":repeat,"slot":slot,
                "elapsed_seconds":seconds,"result":report(result,&m,&adaptive,&output)}),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manufactured_extensions_match_exact_derivatives_and_off_trajectory_jvp() {
        for name in [
            "nonlinear-changing-rank",
            "nonlinear-step-regime",
            "meanfield-manufactured",
        ] {
            let m = Model::new(name).unwrap();
            for t in [0.0, 0.12, 0.2, 0.35] {
                let exact = m.exact(t);
                let mut f = vec![0.0; m.n];
                m.rhs(&exact, &mut f);
                for i in 0..m.n {
                    let derivative = if i == 0 {
                        1.0
                    } else if m.changing_rank() {
                        -m.clock_rates(i).0 * exact[i]
                    } else {
                        m.phase_derivative(t, i)
                    };
                    assert!(
                        (f[i] - derivative).abs() < 1e-12,
                        "{name} t={t} component={i}"
                    );
                }
                let y: Vec<_> = exact
                    .iter()
                    .enumerate()
                    .map(|(i, &x)| x + 0.003 * (i as f64).sin())
                    .collect();
                let v: Vec<_> = (0..m.n).map(|i| 0.2 * (i as f64 + 0.5).cos()).collect();
                let epsilon = 1e-6;
                let yp: Vec<_> = y.iter().zip(&v).map(|(&y, &v)| y + epsilon * v).collect();
                let ym: Vec<_> = y.iter().zip(&v).map(|(&y, &v)| y - epsilon * v).collect();
                let mut fp = vec![0.0; m.n];
                let mut fm = fp.clone();
                let mut jv = fp.clone();
                m.rhs(&yp, &mut fp);
                m.rhs(&ym, &mut fm);
                m.jvp(&y, &v, &mut jv);
                for i in 0..m.n {
                    assert!(
                        (jv[i] - (fp[i] - fm[i]) / (2.0 * epsilon)).abs() < 2e-7,
                        "{name} JVP at {t} component={i}"
                    );
                    let dense = (0..m.n)
                        .map(|j| m.jacobian_entry(i, j, &y) * v[j])
                        .sum::<f64>();
                    assert!((jv[i] - dense).abs() < 1e-11, "{name} matrix/JVP at {t}");
                }
            }
        }
    }
}
