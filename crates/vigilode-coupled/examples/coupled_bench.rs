//! Same-binary complete-step experiment. All reference construction and JSON
//! serialization are OUTSIDE timers. No trajectory or global-error claim.
use rodas5p_core::{
    CoreResult, DenseMatrix, LinearMethod, LinearSolverConfig, LuFactorization, PreconditionerKind,
    WorkCounters, rodas5p_coefficients,
};
use rodas5p_integrators::{OdeProblem, ParallelExecution, StepResult, sequential_step};
use serde_json::{Value, json};
use std::{sync::Arc, time::Instant};
use vigilode_coupled::{Attempt, Config, SelectedStep, coupled_or_sequential, coupled_step};

#[derive(Clone)]
struct Model {
    name: String,
    n: usize,
    eps: f64,
    kind: u8,
}
impl Model {
    fn new(name: &str, n: usize, eps: f64) -> Result<Self, String> {
        if ![3, 64, 96, 384, 12288].contains(&n)
            || !eps.is_finite()
            || !(0.0..=100.0).contains(&eps)
        {
            return Err("unregistered dimension/epsilon".into());
        }
        let kind = match name {
            "few-mode" => 0,
            "broad" => 1,
            "chain" => 2,
            "diffusion" => 3,
            _ => return Err("unknown model".into()),
        };
        if kind != 0 && n > 384 {
            return Err("large non-diagonal reference not permitted".into());
        }
        Ok(Self {
            name: name.into(),
            n,
            eps,
            kind,
        })
    }
    fn initial(&self) -> Vec<f64> {
        (0..self.n)
            .map(|i| {
                if self.kind == 1 {
                    1.0
                } else if self.kind == 3 {
                    0.1 + 0.01 * (2.0 * std::f64::consts::PI * i as f64 / self.n as f64).sin()
                } else {
                    0.1
                }
            })
            .collect()
    }
    fn rate(&self, i: usize) -> f64 {
        if self.kind == 1 {
            -10f64.powf(4.0 * i as f64 / (self.n - 1) as f64)
        } else {
            [-1.0, -20.0, -100.0][i % 3]
        }
    }
    fn rhs(&self, y: &[f64], f: &mut [f64]) {
        let d = if self.kind == 3 {
            0.002 * (self.n as f64).powi(2)
        } else {
            0.0
        };
        for i in 0..self.n {
            f[i] = self.rate(i) * y[i] + self.eps * y[i] * y[i];
            if self.kind == 2 && i > 0 {
                f[i] += 0.3 * y[i - 1];
            }
            if self.kind == 3 {
                f[i] += d * (y[(i + self.n - 1) % self.n] - 2.0 * y[i] + y[(i + 1) % self.n]);
            }
        }
    }
    fn jvp(&self, y: &[f64], v: &[f64], out: &mut [f64]) {
        let d = if self.kind == 3 {
            0.002 * (self.n as f64).powi(2)
        } else {
            0.0
        };
        for i in 0..self.n {
            out[i] = (self.rate(i) + 2.0 * self.eps * y[i]) * v[i];
            if self.kind == 2 && i > 0 {
                out[i] += 0.3 * v[i - 1];
            }
            if self.kind == 3 {
                out[i] += d * (v[(i + self.n - 1) % self.n] - 2.0 * v[i] + v[(i + 1) % self.n]);
            }
        }
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
    fn reference(&self, h: f64) -> CoreResult<(Vec<f64>, Vec<Vec<f64>>)> {
        let c = rodas5p_coefficients()?;
        let y = self.initial();
        let hg = h * c.gamma;
        let n = self.n;
        // Independent analytic assembly only for the small reference. The candidate
        // never sees this matrix or any reference state. Diagonal cases need no matrix.
        let lu = if self.kind >= 2 {
            let mut w = DenseMatrix::zeros(n, n);
            let d = if self.kind == 3 {
                0.002 * (n as f64).powi(2)
            } else {
                0.0
            };
            for i in 0..n {
                w[(i, i)] = 1.0 - hg * (self.rate(i) + 2.0 * self.eps * y[i] - 2.0 * d);
                if self.kind == 2 && i > 0 {
                    w[(i, i - 1)] = -hg * 0.3;
                }
                if self.kind == 3 {
                    w[(i, (i + n - 1) % n)] = -hg * d;
                    w[(i, (i + 1) % n)] = -hg * d;
                }
            }
            Some(LuFactorization::new(&w)?)
        } else {
            None
        };
        let mut u = vec![vec![0.0; n]; 8];
        for i in 0..8 {
            let mut x = y.clone();
            for j in 0..i {
                for k in 0..n {
                    x[k] += c.a[(i, j)] * u[j][k];
                }
            }
            let mut f = vec![0.0; n];
            self.rhs(&x, &mut f);
            for k in 0..n {
                let mut q = 0.0;
                for j in 0..i {
                    q += c.c_matrix[(i, j)] * u[j][k];
                }
                f[k] = hg * f[k] + c.gamma * q;
            }
            u[i] = if let Some(lu) = &lu {
                lu.solve(&f)?
            } else {
                (0..n)
                    .map(|k| f[k] / (1.0 - hg * (self.rate(k) + 2.0 * self.eps * y[k])))
                    .collect()
            };
        }
        let mut out = y;
        for i in 0..8 {
            for k in 0..n {
                out[k] += c.b_code[i] * u[i][k];
            }
        }
        Ok((out, u))
    }
}
fn linear_config() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        preconditioner: PreconditionerKind::None,
        rtol: 1e-12,
        atol: 1e-14,
        maxiter: 512,
        ..LinearSolverConfig::default()
    }
}
enum Run {
    Coupled(Attempt),
    Sequential(CoreResult<StepResult>, WorkCounters),
    Transactional(SelectedStep),
}
impl Run {
    fn y(&self) -> Option<&[f64]> {
        match self {
            Self::Coupled(a) => a.candidate.as_ref().map(|c| c.y_new.as_slice()),
            Self::Sequential(r, _) => r.as_ref().ok().map(|r| r.y_new.as_slice()),
            Self::Transactional(r) => r
                .candidate_attempt
                .candidate
                .as_ref()
                .map(|c| c.y_new.as_slice())
                .or_else(|| r.fallback.as_ref().map(|s| s.y_new.as_slice())),
        }
    }
    fn error(&self) -> Option<&[f64]> {
        match self {
            Self::Coupled(a) => a.candidate.as_ref().map(|c| c.error_vector.as_slice()),
            Self::Sequential(r, _) => r.as_ref().ok().map(|r| r.error_vector.as_slice()),
            Self::Transactional(r) => r
                .candidate_attempt
                .candidate
                .as_ref()
                .map(|c| c.error_vector.as_slice())
                .or_else(|| r.fallback.as_ref().map(|s| s.error_vector.as_slice())),
        }
    }
    fn data(&self) -> Value {
        match self {
            Self::Coupled(a) => json!({"kind":"coupled","attempt":a}),
            Self::Sequential(r, w) => match r {
                Ok(s) => {
                    json!({"kind":"sequential-native-k","y_new":s.y_new,"error_vector":s.error_vector,"error_norm":s.error_norm,"embedded_test_passes":s.accepted,"work":w})
                }
                Err(e) => json!({"kind":"sequential-native-k","failure":e.to_string(),"work":w}),
            },
            Self::Transactional(a) => json!({"kind":"transactional","result":a}),
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    if a.len() != 8 {
        return Err("usage: coupled_bench MODE MODEL N EPS H ARM REPS WARMUPS".into());
    }
    let model = Model::new(&a[1], a[2].parse()?, a[3].parse()?)?;
    let h: f64 = a[4].parse()?;
    if !h.is_finite() || h <= 0.0 || h > 0.1 {
        return Err("step outside experiment domain".into());
    }
    if a[0] == "reference" {
        let (y, u) = model.reference(h)?;
        println!(
            "{}",
            json!({"case":a[1],"n":model.n,"epsilon":model.eps,"h":h,"y_new":y,"stages_u":u,"target":"independent-raw-u-stage-recurrence","not_a_continuous_time_reference":true})
        );
        return Ok(());
    }
    if a[0] != "run" {
        return Err("unknown mode".into());
    }
    let arm = &a[5];
    let threads = match arm.as_str() {
        "coupled-1" | "sequential-mf" => 1,
        "coupled-2" => 2,
        "coupled-4" | "transactional-4" => 4,
        _ => return Err("unknown arm".into()),
    };
    let reps: usize = a[6].parse()?;
    let warm: usize = a[7].parse()?;
    if reps == 0 || reps > 64 || warm > 4 {
        return Err("invalid repetitions".into());
    }
    let setup = Instant::now();
    let exec = ParallelExecution::rayon(threads)?;
    let pool_setup_ns = setup.elapsed().as_nanos();
    let problem = model.problem();
    let y = model.initial();
    let cfg = Config::default();
    let lin = linear_config();
    let run = || {
        if arm == "sequential-mf" {
            let mut w = WorkCounters::default();
            let r = sequential_step(
                &problem, 0.0, &y, h, &lin, None, cfg.atol, cfg.rtol, false, &mut w,
            );
            Run::Sequential(r, w)
        } else if arm == "transactional-4" {
            Run::Transactional(coupled_or_sequential(
                &problem, 0.0, &y, h, &cfg, &exec, &lin,
            ))
        } else {
            Run::Coupled(coupled_step(&problem, 0.0, &y, h, &cfg, &exec))
        }
    };
    for _ in 0..warm {
        let _warm = run();
    }
    let mut samples = Vec::with_capacity(reps);
    let mut outputs = Vec::with_capacity(reps);
    for _ in 0..reps {
        let start = Instant::now();
        let r = run();
        let ns = start.elapsed().as_nanos();
        samples.push(ns);
        outputs.push(r);
    }
    let first = &outputs[0];
    let complete = outputs.iter().all(|r| {
        r.y()
            .is_some_and(|v| v.len() == model.n && v.iter().all(|x| x.is_finite()))
    });
    let deterministic =
        outputs
            .iter()
            .all(|r| match (first.y(), r.y(), first.error(), r.error()) {
                (Some(x), Some(y), Some(e), Some(f)) => {
                    x.len() == y.len()
                        && x.iter().zip(y).all(|(x, y)| x.to_bits() == y.to_bits())
                        && e.len() == f.len()
                        && e.iter().zip(f).all(|(x, y)| x.to_bits() == y.to_bits())
                }
                (None, None, None, None) => true,
                _ => false,
            });
    println!(
        "{}",
        json!({"schema":"coupled-native-step-benchmark-v1","case":model.name,"n":model.n,"epsilon":model.eps,"h":h,"arm":arm,"threads":threads,"config":cfg,
  "root_completed":complete,"deterministic":deterministic,"elapsed_ns":samples,"warmups":warm,"repetitions":reps,"pool_setup_ns":pool_setup_ns,
  "y_new":first.y(),"error_vector":first.error(),"result":first.data(),"timing_scope":"complete step including setup/basis/root validation; excluding pool setup, JSON, comparisons, result drops",
  "scope":"fixed-h root latency, not adaptive trajectory runtime","state_jacobian_available":false})
    );
    Ok(())
}
