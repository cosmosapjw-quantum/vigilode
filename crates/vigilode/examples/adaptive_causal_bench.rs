//! Single-thread diagnostic comparison of identical endpoint-window contracts.
//! Every forced method restarts at the same window boundaries as the adapter.
//! All construction/restart/integration costs are inside the solver timer.
use serde_json::{Value, json};
use std::time::Instant;
use vigilode::{
    Atol, DerivativeFallback, Dopri5, ExplicitStats, Jacobian, Options, Problem, Rodas5p, Stats,
    Status, SwitchingConfig, SwitchingSolver, SwitchingStats,
};

enum RawStats {
    Explicit(ExplicitStats),
    Implicit(Stats),
    Switching(SwitchingStats),
}

#[derive(Clone)]
struct Model {
    name: String,
    n: usize,
    end: f64,
}
impl Model {
    fn new(name: &str, dimension: usize) -> Self {
        let (n, end) = match name {
            "decay" => (1, 4.0),
            "logistic" => (1, 8.0),
            "prothero" => (1, 1.0),
            "robertson" => (3, 1.0),
            "rapid" => (2, 4.0),
            "damped" => (2, 1.0),
            "nonnormal" => (2, 4.0),
            "transition" => (1, 1.0),
            "vector" => (256, 1.0),
            _ => panic!("unknown registered model"),
        };
        Self {
            name: name.into(),
            n: if dimension == 0 { n } else { dimension },
            end,
        }
    }
    fn initial(&self) -> Vec<f64> {
        match self.name.as_str() {
            "robertson" => vec![1.0, 0.0, 0.0],
            "rapid" | "damped" => vec![1.0, 0.0],
            "nonnormal" => vec![0.0, 1.0],
            "logistic" => vec![0.05],
            "transition" => vec![2.0],
            _ => vec![1.0; self.n],
        }
    }
    fn rate(t: f64) -> f64 {
        1.0 + 4999.5 * ((40.0 * (t - 0.3)).tanh() - (40.0 * (t - 0.7)).tanh())
    }
    fn rate_derivative(t: f64) -> f64 {
        let a = (40.0 * (t - 0.3)).tanh();
        let b = (40.0 * (t - 0.7)).tanh();
        4999.5 * 40.0 * (b * b - a * a)
    }
    fn primitive(t: f64) -> f64 {
        fn lc(x: f64) -> f64 {
            let a = x.abs();
            a + (-2.0 * a).exp().ln_1p() - 2.0_f64.ln()
        }
        t + 4999.5 / 40.0 * (lc(40.0 * (t - 0.3)) - lc(40.0 * (t - 0.7)) - lc(-12.0) + lc(-28.0))
    }
    fn rhs(&self, t: f64, y: &[f64], dy: &mut [f64]) {
        match self.name.as_str() {
            "decay" => dy[0] = -y[0],
            "logistic" => dy[0] = y[0] * (1.0 - y[0]),
            "prothero" => dy[0] = -1e5 * (y[0] - t.cos()) - t.sin(),
            "transition" => dy[0] = -Self::rate(t) * (y[0] - t.cos()) - t.sin(),
            "robertson" => {
                dy[0] = -0.04 * y[0] + 1e4 * y[1] * y[2];
                dy[2] = 3e7 * y[1] * y[1];
                dy[1] = -dy[0] - dy[2];
            }
            "rapid" | "damped" => {
                let (g, w) = if self.name == "rapid" {
                    (0.0, 200.0)
                } else {
                    (100.0, 1000.0)
                };
                dy[0] = -g * y[0] - w * y[1];
                dy[1] = w * y[0] - g * y[1];
            }
            "nonnormal" => {
                dy[0] = -y[0] + 100.0 * y[1];
                dy[1] = -2.0 * y[1];
            }
            "vector" => {
                for (i, v) in dy.iter_mut().enumerate() {
                    *v = -[1.0, 10.0, 1000.0][i % 3] * y[i];
                }
            }
            _ => unreachable!(),
        }
    }
    fn jvp(&self, t: f64, y: &[f64], v: &[f64], out: &mut [f64]) {
        match self.name.as_str() {
            "logistic" => out[0] = (1.0 - 2.0 * y[0]) * v[0],
            "prothero" => out[0] = -1e5 * v[0],
            "transition" => out[0] = -Self::rate(t) * v[0],
            "robertson" => {
                out[0] = -0.04 * v[0] + 1e4 * y[2] * v[1] + 1e4 * y[1] * v[2];
                out[2] = 6e7 * y[1] * v[1];
                out[1] = -out[0] - out[2];
            }
            _ => self.rhs(t, v, out),
        }
    }
    fn exact(&self, t: f64) -> Option<Vec<f64>> {
        Some(match self.name.as_str() {
            "decay" => vec![(-t).exp()],
            "logistic" => vec![1.0 / (1.0 + 19.0 * (-t).exp())],
            "prothero" => vec![t.cos()],
            "transition" => vec![t.cos() + (-Self::primitive(t)).exp()],
            "rapid" => vec![(200.0 * t).cos(), (200.0 * t).sin()],
            "damped" => vec![
                (-100.0 * t).exp() * (1000.0 * t).cos(),
                (-100.0 * t).exp() * (1000.0 * t).sin(),
            ],
            "nonnormal" => vec![100.0 * ((-t).exp() - (-2.0 * t).exp()), (-2.0 * t).exp()],
            "vector" => (0..self.n)
                .map(|i| (-[1.0, 10.0, 1000.0][i % 3] * t).exp())
                .collect(),
            "robertson" => return None,
            _ => unreachable!(),
        })
    }
    fn problem(&self, jvp: bool) -> Problem {
        let m = self.clone();
        let mut p = Problem::new(self.n, move |t, y, out| {
            m.rhs(t, y, out);
            Ok(())
        });
        if jvp {
            let m = self.clone();
            p = p.jacobian(Jacobian::jvp(move |t, y, v, out| {
                m.jvp(t, y, v, out);
                Ok(())
            }));
        } else if self.name == "vector" {
            p = p.banded(0, 0);
        }
        if self.name == "prothero" || self.name == "transition" {
            let m = self.clone();
            p.time_derivative(move |t, y, out| {
                out[0] = if m.name == "prothero" {
                    -1e5 * t.sin() - t.cos()
                } else {
                    -Model::rate_derivative(t) * (y[0] - t.cos())
                        - Model::rate(t) * t.sin()
                        - t.cos()
                };
                Ok(())
            })
        } else {
            p.autonomous()
        }
    }
}

fn number(v: &Value, key: &str) -> u64 {
    v.get(key).and_then(Value::as_u64).unwrap_or(0)
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |key: &str, def: &str| {
        args.windows(2)
            .find(|v| v[0] == key)
            .map(|v| v[1].clone())
            .unwrap_or_else(|| def.into())
    };
    let case = arg("--case", "decay");
    let mode = arg("--mode", "switch-jvp");
    let rtol: f64 = arg("--rtol", "1e-6").parse().unwrap();
    let windows: usize = arg("--windows", "16").parse().unwrap();
    let n: usize = arg("--dimension", "0").parse().unwrap();
    let source = arg("--source-commit", "UNRECORDED");
    assert!(windows > 0);
    let m = Model::new(&case, n);
    let requested: Vec<f64> = (0..=windows)
        .map(|i| {
            if i == windows {
                m.end
            } else {
                m.end * i as f64 / windows as f64
            }
        })
        .collect();
    let start = Instant::now();
    let problem = m.problem(mode.ends_with("jvp"));
    let y0 = m.initial();
    let opts = Options {
        rtol,
        atol: Atol::Scalar(rtol * 1e-3),
        h0: 1e-4,
        max_steps: 25_000,
        ..Options::default()
    };
    let mut times = vec![0.0];
    let mut states = vec![y0.clone()];
    let mut success = true;
    let mut message = "reached final time".to_owned();
    let mut stats = vec![];
    if mode == "dopri-whole" || mode == "dopri-whole-dense" {
        let result = if mode == "dopri-whole" {
            Dopri5::solve_observed(&problem, (0.0, m.end), &y0, &opts, &requested)
        } else {
            Dopri5::solve_dense_observed(&problem, (0.0, m.end), &y0, &opts, &requested)
        };
        match result {
            Ok(s) => {
                times = s.t;
                states = s.y;
                success = s.success;
                message = s.message;
                stats.push(RawStats::Explicit(s.stats));
            }
            Err(e) => {
                success = false;
                message = e.to_string();
            }
        }
    } else if mode == "rodas-whole-fd" {
        let dense_opts = Options {
            dense_output: true,
            ..opts.clone()
        };
        match Rodas5p::solve(&problem, (0.0, m.end), &y0, &dense_opts) {
            Ok(s) => {
                success = s.status == Status::Success;
                message = format!("{:?}", s.status);
                times.clear();
                states.clear();
                for &t in &requested {
                    if let Some(y) = s.at(t) {
                        times.push(t);
                        states.push(y);
                    } else {
                        success = false;
                        break;
                    }
                }
                stats.push(RawStats::Implicit(s.stats));
            }
            Err(e) => {
                success = false;
                message = e.to_string();
            }
        }
    } else if mode.starts_with("switch-") {
        let config = SwitchingConfig {
            fallback: if mode == "switch-rhs" {
                DerivativeFallback::Forbidden
            } else if mode == "switch-fd" {
                DerivativeFallback::AllowFiniteDifference
            } else {
                DerivativeFallback::ProvidedOnly
            },
            window_count: windows,
            ..SwitchingConfig::default()
        };
        match SwitchingSolver::solve(&problem, (0.0, m.end), &y0, &opts, &config) {
            Ok(s) => {
                times = s.t;
                states = s.y;
                success = s.success;
                message = s.message;
                stats.push(RawStats::Switching(s.stats));
            }
            Err(e) => {
                success = false;
                message = e.to_string();
            }
        }
    } else {
        let mut state = y0.clone();
        let mut attempts = 0usize;
        for k in 0..windows {
            if attempts >= opts.max_steps {
                success = false;
                message = "global benchmark attempt budget exhausted".into();
                break;
            }
            let local = Options {
                max_steps: opts.max_steps - attempts,
                ..opts.clone()
            };
            if mode == "dopri5" {
                match Dopri5::solve(&problem, (requested[k], requested[k + 1]), &state, &local) {
                    Ok(s) => {
                        attempts += s.stats.attempts;
                        stats.push(RawStats::Explicit(s.stats));
                        if s.final_time > times[times.len() - 1] {
                            times.push(s.final_time);
                            states.push(s.final_state.clone());
                        }
                        state = s.final_state;
                        if !s.success {
                            success = false;
                            message = s.message;
                            break;
                        }
                    }
                    Err(e) => {
                        success = false;
                        message = e.to_string();
                        break;
                    }
                }
            } else if mode == "rodas-fd" || mode == "rodas-jvp" {
                match Rodas5p::solve(&problem, (requested[k], requested[k + 1]), &state, &local) {
                    Ok(s) => {
                        attempts += s.stats.attempts;
                        stats.push(RawStats::Implicit(s.stats));
                        if let (Some(t), Some(y)) = (s.t.last(), s.y.last()) {
                            if *t > times[times.len() - 1] {
                                times.push(*t);
                                states.push(y.clone());
                            }
                            state = y.clone();
                        }
                        if s.status != Status::Success {
                            success = false;
                            message = format!("{:?}", s.status);
                            break;
                        }
                    }
                    Err(e) => {
                        success = false;
                        message = e.to_string();
                        break;
                    }
                }
            } else {
                panic!("unknown benchmark mode");
            }
        }
    }
    let seconds = start.elapsed().as_secs_f64();
    let stats: Vec<Value> = stats
        .into_iter()
        .map(|s| match s {
            RawStats::Explicit(s) => serde_json::to_value(s).unwrap(),
            RawStats::Implicit(s) => serde_json::to_value(s).unwrap(),
            RawStats::Switching(s) => serde_json::to_value(s).unwrap(),
        })
        .collect();
    let mut work = serde_json::Map::new();
    for key in [
        "attempts",
        "accepted_steps",
        "rejected_steps",
        "rhs_calls",
        "jacobian_builds",
        "jvp_vectors",
        "ft_calls",
        "linear_solves",
    ] {
        let sum: u64 = stats
            .iter()
            .map(|s| {
                if s.get(key).is_some() {
                    number(s, key)
                } else {
                    number(&s["counters"], key)
                }
            })
            .sum();
        work.insert(key.into(), json!(sum));
    }
    let reference: Vec<Option<Vec<f64>>> = times.iter().map(|&t| m.exact(t)).collect();
    let shape = times.len() == states.len()
        && states
            .iter()
            .all(|s| s.len() == m.n && s.iter().all(|v| v.is_finite()));
    let reached = times.last().is_some_and(|t| *t == m.end);
    let mut norm = 0.0_f64;
    let mut absolute = 0.0_f64;
    let mut phase = 0.0_f64;
    let mut amplitude = 0.0_f64;
    if shape {
        for (y, r) in states.iter().zip(&reference) {
            if let Some(r) = r {
                for i in 0..m.n {
                    absolute = absolute.max((y[i] - r[i]).abs());
                    norm = norm.max((y[i] - r[i]).abs() / 1.0_f64.max(y0[i].abs()).max(r[i].abs()));
                }
                if case == "rapid" || case == "damped" {
                    let ra = r[0].hypot(r[1]);
                    let ya = y[0].hypot(y[1]);
                    amplitude = amplitude.max((ya - ra).abs());
                    if ra > 1e-10 && ya > 1e-10 {
                        phase = phase.max(
                            (y[1] * r[0] - y[0] * r[1])
                                .atan2(y[0] * r[0] + y[1] * r[1])
                                .abs(),
                        );
                    }
                }
            }
        }
    }
    let conservation = if case == "robertson" {
        Some(
            states
                .iter()
                .map(|y| (y.iter().sum::<f64>() - 1.0).abs())
                .fold(0.0, f64::max),
        )
    } else {
        None
    };
    println!(
        "{}",
        json!({"schema":"vigilode.adaptive-causal-benchmark.v1","source_commit":source,
        "case":case,"mode":mode,"dimension":m.n,"end":m.end,"rtol":rtol,"atol":rtol*1e-3,
        "windows":windows,"success":success&&shape&&reached,"message":message,"elapsed_seconds":seconds,
        "requested_times":requested,"times":times,"states":states,"reference":reference,
        "max_abs_error":if case!="robertson"&&shape {Some(absolute)}else{None},
        "max_normalized_error":if case!="robertson"&&shape {Some(norm)}else{None},
        "max_phase_error_radians":if case=="rapid"||case=="damped"{Some(phase)}else{None},
        "max_amplitude_error":if case=="rapid"||case=="damped"{Some(amplitude)}else{None},
        "max_mass_drift":conservation,"work":work,"raw_stats":stats,
        "timing_scope":"problem/options/state/workspace construction and all endpoint-window restarts/integration; excludes independent reference and JSON",
        "observation_policy":if mode=="switch-rhs" {"one persistent RHS-only session, step-landing observations"}else if mode=="dopri-whole" {"one call, step landing, retainedworkspace"}else if mode=="dopri-whole-dense" {"one call, quartic sampledextension, retainedworkspace"}else if mode=="rodas-whole-fd" {"one call, retaineddenseoutput, WFilteredEnforce"}else{"endpoint-window restarts"},
        "derivative_policy":if mode.starts_with("dopri")||mode=="switch-rhs"{"no derivatives"}else if mode.ends_with("jvp"){"analytic JVP supplied; switching uses it only after explicit cost pressure"}else{"finite-difference Jacobian explicitly authorized; banded for vector"},
        "error_scope":"all returned window endpoints and components; no uniform-time certificate"})
    );
}
