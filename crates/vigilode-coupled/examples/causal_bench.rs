//! Same-binary raw-U stage comparison, with oracle and JSON outside timers.
use rodas5p_core::{DenseMatrix, LuFactorization, rodas5p_coefficients};
use rodas5p_integrators::{OdeProblem, ParallelExecution};
use serde_json::json;
use std::{sync::Arc, time::Instant};
use vigilode_coupled::{
    Attempt, CausalSchedule, Config, coupled_step_causal, coupled_step_residual_first,
};

#[derive(Clone)]
struct Model {
    n: usize,
    eps: f64,
    omega: f64,
}
impl Model {
    fn rhs(&self, y: &[f64]) -> Vec<f64> {
        if self.omega != 0.0 {
            (0..self.n)
                .map(|i| {
                    if i % 2 == 0 {
                        -0.1 * y[i] - self.omega * y[i + 1]
                    } else {
                        self.omega * y[i - 1] - 0.1 * y[i]
                    }
                })
                .collect()
        } else {
            (0..self.n)
                .map(|i| [-1.0, -20.0, -100.0][i % 3] * y[i] + self.eps * y[i] * y[i])
                .collect()
        }
    }
    fn jvp(&self, y: &[f64], v: &[f64]) -> Vec<f64> {
        if self.omega != 0.0 {
            self.rhs(v)
        } else {
            (0..self.n)
                .map(|i| ([-1.0, -20.0, -100.0][i % 3] + 2.0 * self.eps * y[i]) * v[i])
                .collect()
        }
    }
    fn problem(&self) -> OdeProblem {
        let a = self.clone();
        let b = self.clone();
        OdeProblem::new(
            "causal-bench",
            self.n,
            Arc::new(move |_, y, f| {
                f.copy_from_slice(&a.rhs(y));
                Ok(())
            }),
            None,
            None,
            Some(Arc::new(move |_, y, v, f| {
                f.copy_from_slice(&b.jvp(y, v));
                Ok(())
            })),
            None,
            true,
            None,
            None,
        )
        .unwrap()
    }
    // Independent full-space dense raw-U elimination. Restricted small n.
    fn reference(&self, y: &[f64], h: f64) -> Vec<f64> {
        let c = rodas5p_coefficients().unwrap();
        let mut w = DenseMatrix::zeros(self.n, self.n);
        for k in 0..self.n {
            let mut v = vec![0.0; self.n];
            v[k] = 1.0;
            let j = self.jvp(y, &v);
            for i in 0..self.n {
                w[(i, k)] = if i == k { 1.0 } else { 0.0 } - h * c.gamma * j[i];
            }
        }
        let lu = LuFactorization::new(&w).unwrap();
        let mut u = vec![vec![0.0; self.n]; 8];
        for i in 0..8 {
            let mut state = y.to_vec();
            let mut mix = vec![0.0; self.n];
            for (j, previous_stage) in u.iter().take(i).enumerate() {
                for k in 0..self.n {
                    state[k] += c.a[(i, j)] * previous_stage[k];
                    mix[k] += c.c_matrix[(i, j)] * previous_stage[k];
                }
            }
            let f = self.rhs(&state);
            let rhs: Vec<_> = (0..self.n)
                .map(|k| h * c.gamma * f[k] + c.gamma * mix[k])
                .collect();
            u[i] = lu.solve(&rhs).unwrap();
        }
        (0..self.n)
            .map(|k| y[k] + (0..8).map(|i| c.b_code[i] * u[i][k]).sum::<f64>())
            .collect()
    }
}
fn run(p: &OdeProblem, y: &[f64], h: f64, c: &Config, e: &ParallelExecution, m: &str) -> Attempt {
    match m {
        "residual-first" => coupled_step_residual_first(p, 0.0, y, h, c, e),
        "causal-forward" => coupled_step_causal(p, 0.0, y, h, c, e, CausalSchedule::Forward),
        _ => coupled_step_causal(p, 0.0, y, h, c, e, CausalSchedule::BatchedPicard),
    }
}
fn main() {
    let config = Config::default();
    let e = ParallelExecution::sequential();
    let mut rows = Vec::new();
    for n in [6, 96] {
        for (kind, eps, omega, h) in [
            ("affine", 0.0, 0.0, 0.01),
            ("weak-nonlinear", 1.0, 0.0, 0.01),
            ("strong-nonlinear", 10.0, 0.0, 0.01),
            ("stiff-step", 1.0, 0.0, 0.1),
            ("oscillator", 0.0, 1000.0, 0.0001),
            ("underresolved-oscillator", 0.0, 1000.0, 0.01),
        ] {
            let model = Model { n, eps, omega };
            let p = model.problem();
            let y = vec![0.1; n];
            let reference = model.reference(&y, h);
            for method in ["residual-first", "causal-forward", "causal-picard"] {
                let sample = run(&p, &y, h, &config, &e, method);
                let err = sample.candidate.as_ref().map(|c| {
                    c.y_new
                        .iter()
                        .zip(&reference)
                        .map(|(a, b)| (a - b).abs())
                        .fold(0.0, f64::max)
                });
                let mut times = Vec::new();
                let loops = 30;
                for _ in 0..5 {
                    let start = Instant::now();
                    for _ in 0..loops {
                        std::hint::black_box(run(&p, &y, h, &config, &e, method));
                    }
                    times.push(start.elapsed().as_secs_f64() / loops as f64);
                }
                times.sort_by(f64::total_cmp);
                rows.push(json!({"kind":kind,"dimension":n,"epsilon":eps,"omega":omega,"h":h,"method":method,"threads":1,"success":sample.candidate.is_some(),"failure":sample.failure,"max_abs_raw_target_error":err,"embedded_wrms":sample.candidate.as_ref().map(|c|c.embedded_wrms),"work":sample.work,"median_seconds":times[2],"repeat_seconds":times,"loops_per_repeat":loops}));
            }
        }
    }
    println!("{}",serde_json::to_string_pretty(&json!({"schema":"vigilode.causal-comparison.v1","scope":"complete candidate step including basis and full residual check; shared projected and original-space tolerances; physical-step embedded rejection retained","config":config,"rows":rows})).unwrap());
}
