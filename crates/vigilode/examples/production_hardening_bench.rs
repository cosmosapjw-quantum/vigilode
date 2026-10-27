//! Paired observation-policy benchmark. Construction and solve are timed;
//! analytic references, checks and JSON serialization are outside the timer.
use std::time::Instant;
use vigilode::{Atol, Dopri5, Options, Problem};

#[derive(Clone)]
struct Model {
    name: String,
    n: usize,
    end: f64,
}
impl Model {
    fn new(name: &str, n: usize) -> Result<Self, String> {
        let end = match name {
            "decay" | "driven" | "damped" | "nonnormal" | "large-vector" => 4.0,
            "logistic" => 10.0,
            "rapid" => 2.0,
            _ => return Err(format!("unknown model: {name}")),
        };
        if n == 0 || (matches!(name, "rapid" | "damped" | "nonnormal") && !n.is_multiple_of(2)) {
            return Err("positive dimension, even for paired systems, is required".into());
        }
        Ok(Self {
            name: name.into(),
            n,
            end,
        })
    }
    fn rate(&self, i: usize) -> f64 {
        match self.name.as_str() {
            "logistic" => 0.25 + 0.1 * (i % 8) as f64,
            "large-vector" => 0.5 + 0.05 * (i % 8) as f64,
            _ => [0.25, 1.0, 3.0, 0.5][i % 4],
        }
    }
    fn initial(&self, i: usize) -> f64 {
        match self.name.as_str() {
            "logistic" => 0.05 + 0.01 * (i % 8) as f64,
            "rapid" | "damped" => {
                if i.is_multiple_of(2) {
                    1.0
                } else {
                    0.0
                }
            }
            "nonnormal" => {
                if i.is_multiple_of(2) {
                    0.0
                } else {
                    1.0
                }
            }
            _ => 1.0 / (1 + i % 8) as f64,
        }
    }
    fn frequency(&self, i: usize) -> f64 {
        if self.name == "rapid" {
            100.0 + 5.0 * ((i / 2) % 4) as f64
        } else {
            10.0 + ((i / 2) % 4) as f64
        }
    }
    fn rhs(&self, t: f64, y: &[f64], out: &mut [f64]) {
        match self.name.as_str() {
            "rapid" | "damped" => {
                let d = if self.name == "damped" { 3.0 } else { 0.0 };
                for i in (0..self.n).step_by(2) {
                    let w = self.frequency(i);
                    out[i] = -d * y[i] - w * y[i + 1];
                    out[i + 1] = w * y[i] - d * y[i + 1];
                }
            }
            "nonnormal" => {
                for i in (0..self.n).step_by(2) {
                    out[i] = -y[i] + 30.0 * y[i + 1];
                    out[i + 1] = -2.0 * y[i + 1];
                }
            }
            "logistic" => {
                for i in 0..self.n {
                    out[i] = self.rate(i) * y[i] * (1.0 - y[i]);
                }
            }
            "driven" => {
                for i in 0..self.n {
                    out[i] = -self.rate(i) * y[i] + t.cos() + self.rate(i) * t.sin();
                }
            }
            _ => {
                for i in 0..self.n {
                    out[i] = -self.rate(i) * y[i];
                }
            }
        }
    }
    fn exact(&self, t: f64, i: usize) -> f64 {
        match self.name.as_str() {
            "logistic" => 1.0 / (1.0 + (1.0 / self.initial(i) - 1.0) * (-self.rate(i) * t).exp()),
            "driven" => t.sin() + self.initial(i) * (-self.rate(i) * t).exp(),
            "rapid" | "damped" => {
                let envelope = if self.name == "damped" {
                    (-3.0 * t).exp()
                } else {
                    1.0
                };
                envelope
                    * if i.is_multiple_of(2) {
                        (self.frequency(i) * t).cos()
                    } else {
                        (self.frequency(i) * t).sin()
                    }
            }
            "nonnormal" => {
                if i.is_multiple_of(2) {
                    30.0 * ((-t).exp() - (-2.0 * t).exp())
                } else {
                    (-2.0 * t).exp()
                }
            }
            _ => self.initial(i) * (-self.rate(i) * t).exp(),
        }
    }
}
fn arg(args: &[String], key: &str, default: &str) -> String {
    args.windows(2)
        .find(|v| v[0] == key)
        .map(|v| v[1].clone())
        .unwrap_or_else(|| default.into())
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let name = arg(&args, "--case", "decay");
    let mode = arg(&args, "--mode", "dense");
    let n: usize = arg(&args, "--dimension", "1").parse().expect("dimension");
    let count: usize = arg(&args, "--observations", "501")
        .parse()
        .expect("observations");
    let rtol: f64 = arg(&args, "--rtol", "1e-6").parse().expect("rtol");
    let source = arg(&args, "--source-commit", "UNRECORDED");
    assert!(count >= 2 && rtol > 0.0 && rtol.is_finite());
    let m = Model::new(&name, n).expect("model");
    let times: Vec<f64> = (0..count)
        .map(|i| m.end * i as f64 / (count - 1) as f64)
        .collect();
    let started = Instant::now();
    let rhs_model = m.clone();
    let problem = Problem::new(n, move |t, y, out| {
        rhs_model.rhs(t, y, out);
        Ok(())
    });
    let y0: Vec<f64> = (0..n).map(|i| m.initial(i)).collect();
    let opts = Options {
        rtol,
        atol: Atol::Scalar(rtol * 1e-3),
        h0: 1e-3,
        max_steps: 100_000,
        ..Options::default()
    };
    let result = match mode.as_str() {
        "landing" => Dopri5::solve_observed(&problem, (0.0, m.end), &y0, &opts, &times),
        "dense" => Dopri5::solve_dense_observed(&problem, (0.0, m.end), &y0, &opts, &times),
        _ => panic!("mode must be landing or dense"),
    };
    let seconds = started.elapsed().as_secs_f64();
    match result {
        Ok(s) => {
            let shape_ok = s.t.len() == s.y.len()
                && s.y
                    .iter()
                    .all(|row| row.len() == n && row.iter().all(|x| x.is_finite()));
            let mut abs = 0.0_f64;
            let mut normalized = 0.0_f64;
            if shape_ok {
                for (&t, row) in s.t.iter().zip(&s.y) {
                    for (i, &value) in row.iter().enumerate() {
                        let exact = m.exact(t, i);
                        let e = (value - exact).abs();
                        abs = abs.max(e);
                        normalized =
                            normalized.max(e / 1.0_f64.max(m.initial(i).abs()).max(exact.abs()));
                    }
                }
            }
            println!(
                "{}",
                serde_json::json!({
                    "schema":"vigilode.production-hardening-run.v1", "source_commit":source,
                    "case":name,"mode":mode,"dimension":n,"rtol":rtol,"atol":rtol*1e-3,
                    "end":m.end,"observations":count,"requested_times":times,
                    "success":s.success && shape_ok && s.final_time==m.end && s.t==times,
                    "message":s.message,"stop_reason":s.stop_reason,
                    "elapsed_seconds":seconds,"max_abs_error":if shape_ok {Some(abs)} else {None},
                    "max_normalized_error":if shape_ok {Some(normalized)} else {None},
                    "times":s.t,"states":s.y,"final_time":s.final_time,"final_state":s.final_state,
                    "work":{"attempts":s.stats.attempts,"accepted_steps":s.stats.accepted_steps,
                        "rejected_steps":s.stats.rejected_steps,"rhs_completed":s.stats.rhs_completed,
                        "fsal_reuses":s.stats.fsal_reuses,"counters":s.stats.counters,
                        "output_clipped_steps":s.stats.output_clipped_steps},
                    "metric":"max_i,t abs(error)/max(1,abs(initial),abs(exact)); independent of rtol",
                    "timing_scope":"problem/initial-state/options/workspace construction and observed solve; excludes schedule construction, references and serialization"
                })
            );
        }
        Err(e) => println!(
            "{}",
            serde_json::json!({
                "schema":"vigilode.production-hardening-run.v1","source_commit":source,
                "case":name,"mode":mode,"dimension":n,"rtol":rtol,"atol":rtol*1e-3,
                "end":m.end,"observations":count,"success":false,"message":e.to_string(),
                "elapsed_seconds":seconds,"times":[],"states":[]
            })
        ),
    }
}
