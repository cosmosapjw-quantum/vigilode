//! Bounded local trajectory experiment: complete adaptive solves and exact references.
//! Run one registered configuration per process. Reference evaluation and JSON are
//! outside timers; timings include integration workspace creation, not pool setup.
use rodas5p_core::{LinearMethod, LinearSolverConfig, PreconditionerKind};
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSchedule, ParallelExecution,
    integrate_rodas5p_mf_fast_observed,
};
use serde_json::{Value, json};
use std::{sync::Arc, time::Instant};
use vigilode_coupled::Config;

#[derive(Clone)]
struct Model {
    name: String,
    n: usize,
    end: f64,
}
impl Model {
    fn registered(name: &str) -> Result<Self, String> {
        let (n, end) = match name {
            "few-mode" => (384, 0.5),
            "few-mode-large" => (6144, 0.5),
            "riccati" => (96, 1.0),
            "stiff" => (96, 0.1),
            "nonnormal" => (64, 0.5),
            "oscillatory" => (96, 0.5),
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
                    out[i] = -0.1 * y[i] - 30. * y[i + 1];
                    out[i + 1] = 30. * y[i] - 0.1 * y[i + 1];
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
                            (30. * t).cos()
                        } else {
                            (30. * t).sin()
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

struct Run {
    times: Vec<f64>,
    states: Vec<Vec<f64>>,
    success: bool,
    message: String,
    work: Value,
}
fn sequential(
    p: &OdeProblem,
    m: &Model,
    a: &AdaptiveStepConfig,
    o: &OutputSchedule,
) -> Result<Run, String> {
    let r =
        integrate_rodas5p_mf_fast_observed(p, (0., m.end), &m.initial(), &linear_config(), a, o)
            .map_err(|e| e.to_string())?;
    Ok(Run {
        times: r.observed.t,
        states: r.observed.y,
        success: r.observed.success,
        message: r.observed.message,
        work: json!({"attempts":r.attempts,"accepted_steps":r.accepted_steps,"rejected_steps":r.rejected_steps,
            "state_reuses":r.state_reuses,"counters":r.observed.counters,"driver":r.driver}),
    })
}

// The coupled adapter follows the public API introduced in this change.
fn coupled(
    p: &OdeProblem,
    m: &Model,
    a: &AdaptiveStepConfig,
    o: &OutputSchedule,
    e: &ParallelExecution,
    forced: bool,
    refinement: bool,
) -> Result<Run, String> {
    use vigilode_coupled::{RootErrorPolicy, integrate_coupled_observed};
    let mut cfg = Config {
        initial_basis: 3,
        max_basis: 8,
        max_expansions: 2,
        max_batches: 64,
        max_newton: 8,
        atol: a.atol,
        rtol: a.rtol,
        ..Config::default()
    };
    if m.name == "broad" {
        cfg.initial_basis = 3;
        cfg.max_basis = 4;
        cfg.max_expansions = 1;
    }
    if forced {
        cfg.max_jvp = 0;
    }
    if refinement {
        cfg.full_residual_wrms_limit = 1e-12;
        cfg.projected_relative_tol = 1e-14;
    }
    let r = integrate_coupled_observed(
        p,
        (0., m.end),
        &m.initial(),
        a,
        o,
        &cfg,
        e,
        &linear_config(),
        &RootErrorPolicy::default(),
    )
    .map_err(|e| e.to_string())?;
    Ok(Run {
        times: r.observed.t,
        states: r.observed.y,
        success: r.observed.success,
        message: r.observed.message,
        work: json!({"attempts":r.attempts,"counters":r.observed.counters,"terminal_failure":r.terminal_failure}),
    })
}

fn arg(args: &[String], name: &str, default: &str) -> String {
    args.windows(2)
        .find(|w| w[0] == name)
        .map(|w| w[1].clone())
        .unwrap_or_else(|| default.into())
}
fn main() {
    if let Err(error) = run() {
        println!(
            "{}",
            json!({"schema":"vigilode.local-adaptive-bench.v1","status":"CONFIGURATION_ERROR","error":error})
        );
        std::process::exit(2);
    }
}
fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let name = arg(&args, "--case", "few-mode");
    let mode = arg(&args, "--mode", "coupled");
    if !["sequential", "coupled", "forced-fallback"].contains(&mode.as_str()) {
        return Err("mode must be sequential, coupled, or forced-fallback".into());
    }
    let threads: usize = arg(&args, "--threads", "1")
        .parse()
        .map_err(|_| "invalid thread count")?;
    if ![1, 2].contains(&threads) {
        return Err("registered local threads are 1 and 2".into());
    }
    let rtol: f64 = arg(&args, "--rtol", "1e-5")
        .parse()
        .map_err(|_| "invalid rtol")?;
    if !(rtol.is_finite() && rtol > 0. && rtol <= 0.1) {
        return Err("rtol must be in (0,0.1]".into());
    }
    let repeats: usize = arg(&args, "--repeats", "3")
        .parse()
        .map_err(|_| "invalid repeats")?;
    if !(1..=3).contains(&repeats) {
        return Err("registered repeat range is 1..3".into());
    }
    let refinement = args.iter().any(|x| x == "--refinement");
    let m = Model::registered(&name)?;
    let p = m.problem();
    let maximum: f64 = arg(&args, "--max-step", &m.end.to_string())
        .parse()
        .map_err(|_| "invalid max-step")?;
    if !(maximum > 0. && maximum.is_finite()) {
        return Err("max-step must be positive finite".into());
    }
    let adaptive = AdaptiveStepConfig {
        atol: rtol * 0.01,
        rtol,
        initial_step: if refinement { maximum } else { 0.001 },
        max_step: maximum,
        max_attempts: 5000,
        ..AdaptiveStepConfig::default()
    };
    let output = OutputSchedule::new(if refinement {
        vec![0., m.end]
    } else {
        (0..=4).map(|i| m.end * i as f64 / 4.).collect()
    })
    .map_err(|e| e.to_string())?;
    let execution = if threads == 1 {
        ParallelExecution::sequential()
    } else {
        ParallelExecution::rayon(threads).map_err(|e| e.to_string())?
    };
    let solve = || {
        if mode == "sequential" {
            sequential(&p, &m, &adaptive, &output)
        } else {
            coupled(
                &p,
                &m,
                &adaptive,
                &output,
                &execution,
                mode == "forced-fallback",
                refinement,
            )
        }
    };
    let warmup = match solve() {
        Ok(r) => json!({"success":r.success,"message":r.message}),
        Err(e) => json!({"success":false,"error":e}),
    };
    let mut rows = Vec::new();
    for repeat in 0..repeats {
        let start = Instant::now();
        let result = solve();
        let seconds = start.elapsed().as_secs_f64();
        rows.push(match result{
            Ok(r)=>{let(abs,wrms)=m.errors(&r.times,&r.states,adaptive.atol,adaptive.rtol);
                let reaches_end=r.times.last().copied()==Some(m.end);
                json!({"repeat":repeat,"elapsed_seconds":seconds,"success":r.success&&reaches_end&&abs.is_finite()&&wrms.is_finite(),
                    "message":r.message,"reaches_end":reaches_end,"max_abs_error":abs,"max_reference_scaled_wrms":wrms,
                    "observation_times":r.times,"work":r.work})},
            Err(e)=>json!({"repeat":repeat,"elapsed_seconds":seconds,"success":false,"error":e}),
        });
    }
    println!(
        "{}",
        json!({"schema":"vigilode.local-adaptive-bench.v1","host_kind":"local-sandbox-not-ncp",
        "source_commit":std::env::var("VIGILODE_SOURCE_COMMIT").unwrap_or_else(|_|"UNSET".into()),
        "case":name,"dimension":m.n,"time_interval":[0.,m.end],"mode":mode,"threads":threads,
        "rtol":adaptive.rtol,"atol":adaptive.atol,"initial_step":adaptive.initial_step,"max_step":maximum,
        "refinement":refinement,"reference":"closed-form-analytic","warmup":warmup,"runs":rows,
        "comparison_contract":"raw-U RODAS5P, identical observations; compare costs at achieved error, not tolerance alone",
        "timing_scope":"complete integration and workspaces; excludes pool/model/reference/serialization"})
    );
    Ok(())
}
