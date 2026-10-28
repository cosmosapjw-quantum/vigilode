//! Local, reproducible whole-trajectory comparison with analytic references.
//! Timed interval includes problem/workspace construction and integration.
//! Reference evaluation and serialization occur only after the clock stops.
use rodas5p_core::{
    DenseMatrix, LinearMethod, LinearSolverConfig, PreconditionerKind, WorkCounters,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSchedule, integrate_rodas5p_fast_observed,
    integrate_rodas5p_mf_fast_observed,
};
use serde::Serialize;
use std::{sync::Arc, time::Instant};
use vigilode::{Atol, Dopri5, Options, Problem};

#[derive(Clone)]
struct Model {
    name: String,
    n: usize,
    end: f64,
}
impl Model {
    fn new(name: &str, dimension: Option<usize>) -> Result<Self, String> {
        let (n, end) = match name {
            "decay" => (8, 4.),
            "driven" => (8, 4.),
            "logistic" => (8, 10.),
            "harmonic" => (8, 12.),
            "rapid" => (8, 2.),
            "damped" => (8, 4.),
            "mixed-scales" => (12, 4.),
            "nonnormal" => (8, 4.),
            "large-vector" => (4096, 4.),
            "moderate-stiff" => (12, 2.),
            "stiff-control" => (12, 1.),
            _ => return Err(format!("unknown case {name}")),
        };
        let n = dimension.unwrap_or(n);
        if n == 0 || (Self::paired(name) && !n.is_multiple_of(2)) {
            return Err("dimension must be positive and even for paired systems".into());
        }
        Ok(Self {
            name: name.into(),
            n,
            end,
        })
    }
    fn paired(name: &str) -> bool {
        matches!(name, "harmonic" | "rapid" | "damped" | "nonnormal")
    }
    fn rate(&self, i: usize) -> f64 {
        match self.name.as_str() {
            "logistic" => 0.25 + 0.1 * (i % 8) as f64,
            "large-vector" => 0.5 + 0.05 * (i % 8) as f64,
            "moderate-stiff" => [1., 30., 300.][i % 3],
            "stiff-control" => [1., 1_000., 100_000.][i % 3],
            _ => [0.25, 1., 3., 0.5][i % 4],
        }
    }
    fn frequency(&self, i: usize) -> f64 {
        match self.name.as_str() {
            "rapid" => 100. + 5. * ((i / 2) % 4) as f64,
            "damped" => 10. + ((i / 2) % 4) as f64,
            _ => 1. + 0.25 * ((i / 2) % 4) as f64,
        }
    }
    fn initial_value(&self, i: usize) -> f64 {
        match self.name.as_str() {
            "logistic" => 0.05 + 0.01 * (i % 8) as f64,
            "mixed-scales" => 10_f64.powi((i % 12) as i32 - 8),
            "nonnormal" => {
                if i.is_multiple_of(2) {
                    0.
                } else {
                    1.
                }
            }
            "harmonic" | "rapid" | "damped" => {
                if i.is_multiple_of(2) {
                    1.
                } else {
                    0.
                }
            }
            _ => 1. / (1. + (i % 8) as f64),
        }
    }
    fn initial(&self) -> Vec<f64> {
        (0..self.n).map(|i| self.initial_value(i)).collect()
    }
    fn rhs(&self, t: f64, y: &[f64], out: &mut [f64]) {
        match self.name.as_str() {
            "harmonic" | "rapid" | "damped" => {
                let damping = if self.name == "damped" { 3. } else { 0. };
                for i in (0..self.n).step_by(2) {
                    let w = self.frequency(i);
                    out[i] = -damping * y[i] - w * y[i + 1];
                    out[i + 1] = w * y[i] - damping * y[i + 1];
                }
            }
            "nonnormal" => {
                for i in (0..self.n).step_by(2) {
                    out[i] = -y[i] + 30. * y[i + 1];
                    out[i + 1] = -2. * y[i + 1];
                }
            }
            "logistic" => {
                for i in 0..self.n {
                    out[i] = self.rate(i) * y[i] * (1. - y[i]);
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
    fn jvp(&self, y: &[f64], v: &[f64], out: &mut [f64]) {
        if self.name == "logistic" {
            for i in 0..self.n {
                out[i] = self.rate(i) * (1. - 2. * y[i]) * v[i];
            }
        } else if self.name == "driven" {
            for i in 0..self.n {
                out[i] = -self.rate(i) * v[i];
            }
        } else {
            self.rhs(0., v, out);
        }
    }
    fn dense(&self, y: &[f64]) -> DenseMatrix {
        let mut matrix = DenseMatrix::zeros(self.n, self.n);
        let a = matrix.as_mut_slice();
        match self.name.as_str() {
            "harmonic" | "rapid" | "damped" => {
                let damping = if self.name == "damped" { 3. } else { 0. };
                for i in (0..self.n).step_by(2) {
                    let w = self.frequency(i);
                    a[i * self.n + i] = -damping;
                    a[i * self.n + i + 1] = -w;
                    a[(i + 1) * self.n + i] = w;
                    a[(i + 1) * self.n + i + 1] = -damping;
                }
            }
            "nonnormal" => {
                for i in (0..self.n).step_by(2) {
                    a[i * self.n + i] = -1.;
                    a[i * self.n + i + 1] = 30.;
                    a[(i + 1) * self.n + i + 1] = -2.;
                }
            }
            "logistic" => {
                for i in 0..self.n {
                    a[i * self.n + i] = self.rate(i) * (1. - 2. * y[i]);
                }
            }
            _ => {
                for i in 0..self.n {
                    a[i * self.n + i] = -self.rate(i);
                }
            }
        }
        matrix
    }
    fn exact(&self, t: f64) -> Vec<f64> {
        (0..self.n)
            .map(|i| match self.name.as_str() {
                "logistic" => {
                    1. / (1. + (1. / self.initial_value(i) - 1.) * (-self.rate(i) * t).exp())
                }
                "driven" => t.sin() + self.initial_value(i) * (-self.rate(i) * t).exp(),
                "harmonic" | "rapid" | "damped" => {
                    let envelope = if self.name == "damped" {
                        (-3. * t).exp()
                    } else {
                        1.
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
                        30. * ((-t).exp() - (-2. * t).exp())
                    } else {
                        (-2. * t).exp()
                    }
                }
                _ => self.initial_value(i) * (-self.rate(i) * t).exp(),
            })
            .collect()
    }
    fn public_problem(&self) -> Problem {
        let m = self.clone();
        let p = Problem::new(self.n, move |t, y, out| {
            m.rhs(t, y, out);
            Ok(())
        });
        if self.name == "driven" {
            p
        } else {
            p.autonomous()
        }
    }
    fn native_problem(&self, mf: bool) -> OdeProblem {
        let a = self.clone();
        let b = self.clone();
        let c = self.clone();
        let d = self.clone();
        OdeProblem::new(
            self.name.clone(),
            self.n,
            Arc::new(move |t, y, out| {
                a.rhs(t, y, out);
                Ok(())
            }),
            None,
            if mf {
                None
            } else {
                Some(Arc::new(move |_, y| Ok(b.dense(y))))
            },
            if mf {
                Some(Arc::new(move |_, y, v, out| {
                    c.jvp(y, v, out);
                    Ok(())
                }))
            } else {
                None
            },
            if self.name == "driven" {
                Some(Arc::new(move |t, _, out| {
                    for (i, v) in out.iter_mut().enumerate() {
                        *v = -t.sin() + d.rate(i) * t.cos();
                    }
                    Ok(())
                }))
            } else {
                None
            },
            self.name != "driven",
            None,
            None,
        )
        .unwrap()
    }
}

#[derive(Serialize)]
struct Work {
    attempts: usize,
    accepted_steps: usize,
    rejected_steps: usize,
    fsal_reuses: Option<u64>,
    rhs_completed: Option<u64>,
    counters: WorkCounters,
}
struct Run {
    t: Vec<f64>,
    y: Vec<Vec<f64>>,
    success: bool,
    message: String,
    work: Work,
}
fn run(m: &Model, mode: &str, rtol: f64, times: &[f64]) -> Result<Run, String> {
    let y0 = m.initial();
    let opts = Options {
        rtol,
        atol: Atol::Scalar(rtol * 1e-3),
        h0: 1e-3,
        max_steps: 100_000,
        ..Options::default()
    };
    if mode == "dopri5" {
        let s = Dopri5::solve_observed(&m.public_problem(), (0., m.end), &y0, &opts, times)
            .map_err(|e| e.to_string())?;
        Ok(Run {
            t: s.t,
            y: s.y,
            success: s.success,
            message: s.message,
            work: Work {
                attempts: s.stats.attempts,
                accepted_steps: s.stats.accepted_steps,
                rejected_steps: s.stats.rejected_steps,
                fsal_reuses: Some(s.stats.fsal_reuses),
                rhs_completed: Some(s.stats.rhs_completed),
                counters: s.stats.counters,
            },
        })
    } else {
        let adaptive = AdaptiveStepConfig {
            rtol,
            atol: rtol * 1e-3,
            initial_step: opts.h0,
            max_attempts: opts.max_steps,
            ..AdaptiveStepConfig::default()
        };
        let schedule = OutputSchedule::new(times.to_vec()).map_err(|e| e.to_string())?;
        if mode == "rodas-mf" {
            let linear = LinearSolverConfig {
                method: LinearMethod::Gmres,
                preconditioner: PreconditionerKind::None,
                rtol: 1e-12,
                atol: 1e-14,
                maxiter: 256,
                restart: 64,
                ..LinearSolverConfig::default()
            };
            let s = integrate_rodas5p_mf_fast_observed(
                &m.native_problem(true),
                (0., m.end),
                &y0,
                &linear,
                &adaptive,
                &schedule,
            )
            .map_err(|e| e.to_string())?;
            Ok(Run {
                t: s.observed.t,
                y: s.observed.y,
                success: s.observed.success,
                message: s.observed.message,
                work: Work {
                    attempts: s.attempts,
                    accepted_steps: s.accepted_steps,
                    rejected_steps: s.rejected_steps,
                    fsal_reuses: None,
                    rhs_completed: None,
                    counters: s.observed.counters,
                },
            })
        } else if mode == "rodas-dense" {
            let s = integrate_rodas5p_fast_observed(
                &m.native_problem(false),
                (0., m.end),
                &y0,
                &adaptive,
                &schedule,
            )
            .map_err(|e| e.to_string())?;
            Ok(Run {
                t: s.observed.t,
                y: s.observed.y,
                success: s.observed.success,
                message: s.observed.message,
                work: Work {
                    attempts: s.attempts,
                    accepted_steps: s.accepted_steps,
                    rejected_steps: s.rejected_steps,
                    fsal_reuses: None,
                    rhs_completed: None,
                    counters: s.observed.counters,
                },
            })
        } else {
            Err(format!("unknown mode {mode}"))
        }
    }
}

fn arg(args: &[String], key: &str, default: &str) -> String {
    args.windows(2)
        .find(|x| x[0] == key)
        .map(|x| x[1].clone())
        .unwrap_or_else(|| default.into())
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let name = arg(&args, "--case", "decay");
    let mode = arg(&args, "--mode", "dopri5");
    let rtol: f64 = arg(&args, "--rtol", "1e-6").parse().unwrap();
    let n = arg(&args, "--dimension", "0").parse::<usize>().unwrap();
    let m = Model::new(&name, if n == 0 { None } else { Some(n) }).unwrap();
    let times: Vec<f64> = (0..=4).map(|i| m.end * i as f64 / 4.).collect();
    let start = Instant::now();
    let result = run(&m, &mode, rtol, &times);
    let seconds = start.elapsed().as_secs_f64();
    let source = arg(&args, "--source-commit", "UNRECORDED");
    match result {
        Ok(s) => {
            let reference: Vec<Vec<f64>> = s.t.iter().map(|&t| m.exact(t)).collect();
            let mut abs = 0_f64;
            let mut normalized = 0_f64;
            let mut scaled = 0_f64;
            let shape_ok = s.t.len() == s.y.len()
                && s.y
                    .iter()
                    .all(|y| y.len() == m.n && y.iter().all(|v| v.is_finite()));
            if shape_ok {
                for (y, r) in s.y.iter().zip(&reference) {
                    for i in 0..m.n {
                        let e = (y[i] - r[i]).abs();
                        abs = abs.max(e);
                        normalized = normalized
                            .max(e / (1_f64.max(m.initial_value(i).abs()).max(r[i].abs())));
                    }
                    let mut sum = 0.;
                    for i in 0..m.n {
                        let q = (y[i] - r[i]) / (rtol * 1e-3 + rtol * r[i].abs());
                        sum += q * q;
                    }
                    scaled = scaled.max((sum / m.n as f64).sqrt());
                }
            }
            let reached = s.t.last().is_some_and(|&t| t == m.end);
            println!(
                "{}",
                serde_json::json!({"schema":"vigilode.nonstiff-benchmark-run.v1","source_commit":source,
                "case":name,"dimension":m.n,"mode":mode,"rtol":rtol,"atol":rtol*1e-3,"end":m.end,
                "success":s.success&&shape_ok&&reached,"message":s.message,"elapsed_seconds":seconds,
                "max_abs_error":if shape_ok {Some(abs)}else{None},"max_normalized_error":if shape_ok {Some(normalized)}else{None},
                "max_reference_scaled_wrms":if shape_ok {Some(scaled)}else{None},"requested_times":times,
                "times":s.t,"states":s.y,"reference":reference,"work":s.work,
                "error_normalization":"max_i,t |e_i(t)|/max(1,|y_i(0)|,|y_exact_i(t)|); fixed independently of requested tolerance",
                "timing_scope":"problem/workspace construction and complete observed integration; excludes reference and JSON"})
            );
        }
        Err(e) => println!(
            "{}",
            serde_json::json!({"schema":"vigilode.nonstiff-benchmark-run.v1","source_commit":source,
            "case":name,"dimension":m.n,"mode":mode,"rtol":rtol,"atol":rtol*1e-3,"end":m.end,
            "success":false,"message":e,"elapsed_seconds":seconds,"times":[],"states":[],"reference":[]})
        ),
    }
}
