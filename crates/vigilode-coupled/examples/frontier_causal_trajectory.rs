//! Full accepted-trajectory causal schedule experiment. JSON/reference work is outside timers.
//! All three methods use the same physical output landings, root screen and fallback.
use rodas5p_core::{LinearMethod, LinearSolverConfig, PreconditionerKind};
use rodas5p_integrators::{AdaptiveStepConfig, OdeProblem, OutputSchedule, ParallelExecution};
use serde_json::json;
use std::{sync::Arc, time::Instant};
use vigilode_coupled::{
    CausalSchedule, Config, RootErrorPolicy, integrate_causal_observed, integrate_coupled_observed,
};
#[derive(Clone)]
struct Model {
    name: String,
    n: usize,
    end: f64,
}
impl Model {
    fn registered(name: &str) -> Result<Self, String> {
        let (n, end) = match name {
            "few-mode" => (96, 0.5),
            "few-mode-large" => (384, 0.5),
            "riccati" => (96, 1.0),
            "stiff" => (96, 0.1),
            "nonnormal" => (64, 0.5),
            "oscillatory" => (96, 0.02),
            "broad" => (64, 0.1),
            _ => return Err(format!("unregistered case {name}")),
        };
        Ok(Self {
            name: name.into(),
            n,
            end,
        })
    }
    fn rate(&self, i: usize) -> f64 {
        match self.name.as_str() {
            "riccati" => [-1., -2., -3.][i % 3],
            "stiff" => [-1., -100., -10000.][i % 3],
            "broad" => -10_f64.powf(4. * i as f64 / (self.n - 1) as f64),
            _ => [-1., -20., -100.][i % 3],
        }
    }
    fn initial(&self) -> Vec<f64> {
        (0..self.n)
            .map(|i| match self.name.as_str() {
                "nonnormal" => {
                    if i % 2 == 0 {
                        0.
                    } else {
                        0.1
                    }
                }
                "oscillatory" => {
                    if i % 2 == 0 {
                        1.
                    } else {
                        0.
                    }
                }
                _ => 0.1,
            })
            .collect()
    }
    fn rhs(&self, y: &[f64], out: &mut [f64]) {
        match self.name.as_str() {
            "nonnormal" => {
                for i in (0..self.n).step_by(2) {
                    out[i] = -y[i] + 50. * y[i + 1];
                    out[i + 1] = -20. * y[i + 1];
                }
            }
            "oscillatory" => {
                for i in (0..self.n).step_by(2) {
                    out[i] = -0.1 * y[i] - 1000. * y[i + 1];
                    out[i + 1] = 1000. * y[i] - 0.1 * y[i + 1];
                }
            }
            _ => {
                for i in 0..self.n {
                    out[i] = self.rate(i) * y[i]
                        + if self.name == "riccati" {
                            0.5 * y[i] * y[i]
                        } else {
                            0.
                        };
                }
            }
        }
    }
    fn jvp(&self, y: &[f64], v: &[f64], out: &mut [f64]) {
        if self.name == "riccati" {
            for i in 0..self.n {
                out[i] = (self.rate(i) + y[i]) * v[i];
            }
        } else {
            self.rhs(v, out);
        }
    }
    fn exact(&self, t: f64) -> Vec<f64> {
        (0..self.n)
            .map(|i| match self.name.as_str() {
                "nonnormal" => {
                    if i % 2 == 0 {
                        5. * ((-t).exp() - (-20. * t).exp()) / 19.
                    } else {
                        0.1 * (-20. * t).exp()
                    }
                }
                "oscillatory" => {
                    (-0.1 * t).exp()
                        * if i % 2 == 0 {
                            (1000. * t).cos()
                        } else {
                            (1000. * t).sin()
                        }
                }
                "riccati" => {
                    let a = self.rate(i);
                    let e = (a * t).exp();
                    0.1 * e / (1. + 0.05 / a * (1. - e))
                }
                _ => 0.1 * (self.rate(i) * t).exp(),
            })
            .collect()
    }
    fn problem(&self) -> OdeProblem {
        let a = self.clone();
        let b = self.clone();
        OdeProblem::new(
            self.name.clone(),
            self.n,
            Arc::new(move |_, y, out| {
                a.rhs(y, out);
                Ok(())
            }),
            None,
            None,
            Some(Arc::new(move |_, y, v, out| {
                b.jvp(y, v, out);
                Ok(())
            })),
            None,
            true,
            None,
            None,
        )
        .unwrap()
    }
    fn errors(&self, t: &[f64], y: &[Vec<f64>], atol: f64, rtol: f64) -> (f64, f64) {
        if t.len() != y.len()
            || y.iter()
                .any(|row| row.len() != self.n || row.iter().any(|v| !v.is_finite()))
        {
            return (f64::INFINITY, f64::INFINITY);
        }
        let mut abs: f64 = 0.;
        let mut wrms: f64 = 0.;
        for (&time, state) in t.iter().zip(y) {
            let exact = self.exact(time);
            let mut sum = 0.;
            for (&actual, &expected) in state.iter().zip(&exact) {
                let err = (actual - expected).abs();
                abs = abs.max(err);
                sum += (err / (atol + rtol * expected.abs())).powi(2);
            }
            wrms = wrms.max((sum / self.n as f64).sqrt());
        }
        (abs, wrms)
    }
}

fn linear_config() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        preconditioner: PreconditionerKind::None,
        rtol: 1e-12,
        atol: 1e-14,
        maxiter: 256,
        restart: 64,
        ..LinearSolverConfig::default()
    }
}

fn arg(args: &[String], name: &str, default: &str) -> String {
    args.windows(2)
        .find(|w| w[0] == name)
        .map(|w| w[1].clone())
        .unwrap_or_else(|| default.into())
}
fn main() {
    if let Err(e) = run() {
        println!(
            "{}",
            json!({"schema":"vigilode.frontier-causal-trajectory.v1","status":"CONFIGURATION_ERROR","error":e})
        );
        std::process::exit(2);
    }
}
fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let name = arg(&args, "--case", "riccati");
    let mode = arg(&args, "--mode", "causal-forward");
    if !["residual-first", "causal-forward", "causal-picard"].contains(&mode.as_str()) {
        return Err("unknown method".into());
    }
    let threads: usize = arg(&args, "--threads", "1")
        .parse()
        .map_err(|_| "invalid threads")?;
    if ![1, 2].contains(&threads) {
        return Err("threads must be 1 or 2".into());
    }
    let rtol: f64 = arg(&args, "--rtol", "1e-6")
        .parse()
        .map_err(|_| "invalid tolerance")?;
    if !(rtol > 0. && rtol <= 0.01 && rtol.is_finite()) {
        return Err("invalid tolerance".into());
    }
    let repeats: usize = arg(&args, "--repeats", "3")
        .parse()
        .map_err(|_| "invalid repeats")?;
    if !(1..=5).contains(&repeats) {
        return Err("repeats must be 1..5".into());
    }
    let m = Model::registered(&name)?;
    let p = m.problem();
    let adaptive = AdaptiveStepConfig {
        atol: rtol * 0.01,
        rtol,
        initial_step: 0.001,
        max_step: m.end,
        max_attempts: 5000,
        ..Default::default()
    };
    let output = OutputSchedule::new((0..=16).map(|i| m.end * i as f64 / 16.).collect())
        .map_err(|e| e.to_string())?;
    let execution = if threads == 1 {
        ParallelExecution::sequential()
    } else {
        ParallelExecution::rayon(threads).map_err(|e| e.to_string())?
    };
    let cfg = Config {
        initial_basis: 3,
        max_basis: if name == "broad" { 4 } else { 8 },
        max_expansions: if name == "broad" { 1 } else { 2 },
        max_batches: 64,
        ..Default::default()
    };
    let solve = || match mode.as_str() {
        "residual-first" => integrate_coupled_observed(
            &p,
            (0., m.end),
            &m.initial(),
            &adaptive,
            &output,
            &cfg,
            &execution,
            &linear_config(),
            &RootErrorPolicy::Operational,
        ),
        _ => integrate_causal_observed(
            &p,
            (0., m.end),
            &m.initial(),
            &adaptive,
            &output,
            &cfg,
            &execution,
            &linear_config(),
            &RootErrorPolicy::Operational,
            if mode == "causal-forward" {
                CausalSchedule::Forward
            } else {
                CausalSchedule::BatchedPicard
            },
        ),
    };
    let warmup = match solve() {
        Ok(r) => json!({"success":r.observed.success,"message":r.observed.message}),
        Err(e) => json!({"success":false,"message":e.to_string()}),
    };
    let mut rows = Vec::new();
    for repeat in 0..repeats {
        let begin = Instant::now();
        let result = solve();
        let elapsed = begin.elapsed().as_secs_f64();
        rows.push(match result {
            Err(e)=>json!({"repeat":repeat,"success":false,"elapsed_seconds":elapsed,"error":e.to_string()}),
            Ok(r)=>{
                let (abs,wrms)=m.errors(&r.observed.t,&r.observed.y,adaptive.atol,adaptive.rtol);
                let sum=|f:fn(&vigilode_coupled::CoupledTrajectoryAttempt)->u64|r.attempts.iter().map(f).sum::<u64>();
                let totals=json!({"stage_jvp":sum(|a|a.candidate_work.stage_jvp),"basis_jvp":sum(|a|a.candidate_work.basis_jvp),"validation_jvp":sum(|a|a.candidate_work.validation_jvp),"newton_updates":sum(|a|a.candidate_work.newton_updates),"picard_updates":sum(|a|a.candidate_work.causal_picard_updates),"basis_expansions":sum(|a|a.candidate_work.basis_expansions),"candidate_factorizations":sum(|a|a.candidate_work.small_factorizations),"candidate_rhs":sum(|a|a.candidate_work.rhs_attempted),"candidate_jvp":sum(|a|a.candidate_work.jvp_attempted),"fallback_attempts":r.attempts.iter().filter(|a|a.fallback_attempted).count(),"maximum_basis":r.attempts.iter().map(|a|a.candidate_work.maximum_basis).max()});
                let reaches_end=r.last_t==m.end && r.observed.t==output.times();
                json!({"repeat":repeat,"success":r.observed.success&&reaches_end&&abs.is_finite(),"elapsed_seconds":elapsed,"message":r.observed.message,"reaches_end":reaches_end,"max_abs_error":abs,"max_reference_scaled_wrms":wrms,"observation_times":r.observed.t,"observation_states":r.observed.y,"counters":r.observed.counters,"candidate_totals":totals,"attempts":r.attempts,"terminal_failure":r.terminal_failure})
            }
        });
    }
    println!(
        "{}",
        json!({"schema":"vigilode.frontier-causal-trajectory.v1","source_commit":std::env::var("VIGILODE_SOURCE_COMMIT").unwrap_or_else(|_|"UNSET".into()),"host_kind":"local-sandbox-not-ncp","case":name,"mode":mode,"dimension":m.n,"threads":threads,"rtol":rtol,"atol":adaptive.atol,"config":cfg,"time_interval":[0.,m.end],"output_count":17,"initial_step":adaptive.initial_step,"max_step":adaptive.max_step,"max_attempts":adaptive.max_attempts,"policy":"Operational; residual screen is not a global error certificate","reference":"independent analytic solution at all 17 physical output landings","timing_scope":"complete adaptive integration including allocation, rejected attempts and fallback; excludes threadpool/model/reference/serialization","comparison_contract":"same achieved error cap; never infer speed from identical nominal tolerances alone","warmup":warmup,"runs":rows})
    );
    Ok(())
}
