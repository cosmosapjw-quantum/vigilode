//! Native source-scoped R5 experiment. The transformed RODAS5P right-hand
//! sides stay signed and the driver owns all eight stages and step acceptance.
use rodas5p_core::{CoreResult, DenseMatrix, WorkCounters, directed::Interval};
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSchedule, StageLinearSystem,
    integrate_rodas5p_fast_client_observed_with_atol, integrate_rodas5p_fast_observed,
    stage_clients::{DenseLuMode, DenseLuStageClient},
};
use serde::Serialize;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::Instant;

const MAX: usize = 3;
type Row = [f64; MAX];
#[derive(Clone, Copy, Serialize)]
struct Cubic {
    name: &'static str,
    n: usize,
    k: Row,
    g: Row,
    c: [Row; MAX],
    y0: Row,
    panels: usize,
}
impl Cubic {
    fn row_sum(&self, i: usize) -> f64 {
        self.c[i][..self.n].iter().sum()
    }
    fn jac(&self, y: &[f64], i: usize) -> f64 {
        -self.k[i] - 3.0 * self.g[i] * y[i] * y[i] - self.row_sum(i)
    }
    fn problem(self) -> CoreResult<OdeProblem> {
        let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
            for i in 0..self.n {
                out[i] = -self.k[i] * y[i] - self.g[i] * y[i] * y[i] * y[i];
                for j in 0..self.n {
                    out[i] += self.c[i][j] * (y[j] - y[i]);
                }
            }
            Ok(())
        });
        let jac = Arc::new(move |_t: f64, y: &[f64]| {
            let mut mat = DenseMatrix::zeros(self.n, self.n);
            for i in 0..self.n {
                for j in 0..self.n {
                    mat[(i, j)] = if i == j { self.jac(y, i) } else { self.c[i][j] };
                }
            }
            Ok(mat)
        });
        OdeProblem::new(
            self.name,
            self.n,
            rhs,
            None,
            Some(jac),
            None,
            None,
            true,
            None,
            None,
        )
    }
}
fn sources() -> Vec<Cubic> {
    let a = Cubic {
        name: "asymmetric_2",
        n: 2,
        k: [4., 7., 0.],
        g: [20., 40., 0.],
        c: [[0., 12., 0.], [8., 0., 0.], [0.; 3]],
        y0: [0.2, 0.12, 0.],
        panels: 64,
    };
    vec![
        a,
        Cubic {
            name: "asymmetric_2_sparse_output",
            panels: 4,
            ..a
        },
        Cubic {
            name: "asymmetric_2_refined",
            panels: 128,
            ..a
        },
        Cubic {
            name: "coupled_3_cubic",
            n: 3,
            k: [6., 8., 10.],
            g: [60., 100., 30.],
            c: [[0., 30., 4.], [15., 0., 10.], [10., 8., 0.]],
            y0: [0.04, 0.06, 0.02],
            panels: 128,
        },
        Cubic {
            name: "linear_exchange_control",
            n: 2,
            k: [4., 4., 0.],
            g: [0.; 3],
            c: [[0., 6., 0.], [6., 0., 0.], [0.; 3]],
            y0: [0.1, 0.05, 0.],
            panels: 128,
        },
        Cubic {
            name: "strong_exchange_stress",
            c: [[0., 20_000., 0.], [15_000., 0., 0.], [0.; 3]],
            ..a
        },
    ]
}
#[derive(Default, Clone, Serialize)]
struct Counts {
    stage_calls: u64,
    stage_calls_with_negative_rhs: u64,
    source_linearizations: u64,
    candidate_stages: u64,
    sweeps: u64,
    stations: u64,
    interval_residual_checks: u64,
    fallback_stages: u64,
    fallback_factors: u64,
    fallback_jacobian_calls: u64,
    q_rejections: u64,
    exhausted: u64,
    unrepresentable_certificate: u64,
    max_accepted_stage_error_upper: f64,
    max_observed_q: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Arm {
    Protected,
    Direct,
    Continued,
    Capped,
}
struct Client {
    source: Cubic,
    problem: OdeProblem,
    arm: Arm,
    y: Row,
    t: f64,
    shift: f64,
    diag: Row,
    diag_boxes: [Interval; MAX],
    inv_margin: f64,
    q: f64,
    lu: DenseLuStageClient,
    factored: bool,
    has_jac: bool,
    counts: Counts,
    receipt: Arc<Mutex<Counts>>,
}
impl Drop for Client {
    fn drop(&mut self) {
        *self.receipt.lock().unwrap() = self.counts.clone();
    }
}
impl Client {
    fn dense(&mut self, b: &mut [f64], w: &mut WorkCounters) -> CoreResult<()> {
        self.counts.fallback_stages += 1;
        if !self.has_jac {
            self.lu
                .linearize(&self.problem, self.t, &self.y[..self.source.n], w)?;
            self.counts.fallback_jacobian_calls += 1;
            self.has_jac = true;
        }
        if !self.factored {
            self.lu.factor(self.shift, w)?;
            self.counts.fallback_factors += 1;
            self.factored = true;
        }
        self.lu.solve_in_place(b, w)
    }
    fn residual_bound(&self, x: &Row, b: &[f64]) -> CoreResult<f64> {
        let mut upper = 0f64;
        for (i, &bi) in b.iter().enumerate() {
            let mut residual = self.diag_boxes[i]
                .mul(Interval::point(x[i])?)?
                .sub(Interval::point(bi)?)?;
            for (j, &xj) in x[..self.source.n].iter().enumerate() {
                if self.source.c[i][j] != 0. {
                    residual = residual
                        .sub(Interval::point(self.source.c[i][j])?.mul(Interval::point(xj)?)?)?;
                }
            }
            upper = upper.max(residual.mag());
        }
        Ok(Interval::point(upper)?
            .mul(Interval::point(self.inv_margin)?)?
            .hi)
    }
}
impl StageLinearSystem for Client {
    fn dimension(&self) -> usize {
        self.source.n
    }
    fn linearize(
        &mut self,
        _: &OdeProblem,
        t: f64,
        y: &[f64],
        _: &mut WorkCounters,
    ) -> CoreResult<()> {
        self.y[..y.len()].copy_from_slice(y);
        self.t = t;
        self.has_jac = false;
        self.counts.source_linearizations += 1;
        Ok(())
    }
    fn factor(&mut self, alpha: f64, _: &mut WorkCounters) -> CoreResult<()> {
        self.shift = alpha;
        self.factored = false;
        self.q = 0.;
        let p = Interval::point;
        let kmin = self.source.k[..self.source.n]
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        self.inv_margin = p(1.)?.div(p(alpha)?.add(p(kmin)?)?)?.hi;
        for i in 0..self.source.n {
            self.diag[i] = alpha - self.source.jac(&self.y, i);
            let mut d = p(alpha)?.add(p(self.source.k[i])?)?.add(
                p(3.)?
                    .mul(p(self.source.g[i])?)?
                    .mul(p(self.y[i])?)?
                    .mul(p(self.y[i])?)?,
            )?;
            for v in &self.source.c[i][..self.source.n] {
                d = d.add(p(*v)?)?;
            }
            self.diag_boxes[i] = d;
            self.q = self.q.max(self.source.row_sum(i) / self.diag[i]);
        }
        self.counts.max_observed_q = self.counts.max_observed_q.max(self.q);
        Ok(())
    }
    fn solve_in_place(&mut self, b: &mut [f64], work: &mut WorkCounters) -> CoreResult<()> {
        self.counts.stage_calls += 1;
        self.counts.stage_calls_with_negative_rhs += u64::from(b.iter().any(|v| *v < 0.));
        if self.q >= 0.75 {
            self.counts.q_rejections += 1;
            return self.dense(b, work);
        }
        let stations: &[f64] = if self.arm == Arm::Continued {
            &[0., 0.5, 1.]
        } else {
            &[1.]
        };
        let max_sweeps = if self.arm == Arm::Capped { 1 } else { 24 };
        let n = self.source.n;
        let mut x = [0.; MAX];
        for &lambda in stations {
            self.counts.stations += 1;
            let mut converged = false;
            for _ in 0..max_sweeps {
                let prev = x;
                let mut delta = 0f64;
                let mut norm = 0f64;
                for (i, &bi) in b.iter().enumerate() {
                    let sum: f64 = self.source.c[i][..n]
                        .iter()
                        .zip(prev)
                        .map(|(c, p)| c * p)
                        .sum();
                    x[i] = (bi + lambda * sum) / self.diag[i];
                    delta = delta.max((x[i] - prev[i]).abs());
                    norm = norm.max(x[i].abs());
                }
                self.counts.sweeps += 1;
                work.linear_iterations += 1;
                let budget = 2e-15 + 5e-14 * norm;
                if delta > budget || !x[..n].iter().all(|v| v.is_finite()) {
                    continue;
                }
                if lambda == 1. {
                    self.counts.interval_residual_checks += 1;
                    let Ok(bound) = self.residual_bound(&x, b) else {
                        self.counts.unrepresentable_certificate += 1;
                        return self.dense(b, work);
                    };
                    if bound > budget {
                        continue;
                    }
                    self.counts.max_accepted_stage_error_upper =
                        self.counts.max_accepted_stage_error_upper.max(bound);
                    b.copy_from_slice(&x[..n]);
                    self.counts.candidate_stages += 1;
                    return Ok(());
                }
                converged = true;
                break;
            }
            if !converged {
                self.counts.exhausted += 1;
                return self.dense(b, work);
            }
        }
        unreachable!("the terminal station returns a certified stage or its fallback")
    }
}
fn run(source: Cubic, arm: Arm, rtol: f64, initial_step: f64) -> CoreResult<serde_json::Value> {
    let start = Instant::now();
    let p = source.problem()?;
    let grid = (0..=source.panels)
        .map(|i| 0.1 * i as f64 / source.panels as f64)
        .collect::<Vec<_>>();
    let schedule = OutputSchedule::new(grid.clone())?;
    let cfg = AdaptiveStepConfig {
        atol: rtol * 1e-3,
        rtol,
        initial_step,
        max_attempts: 100_000,
        ..Default::default()
    };
    let n = source.n;
    let y0 = &source.y0[..n];
    let receipt = Arc::new(Mutex::new(Counts::default()));
    let run = if arm == Arm::Protected {
        integrate_rodas5p_fast_observed(&p, (0., 0.1), y0, &cfg, &schedule)?
    } else {
        let client = Client {
            source,
            problem: p.clone(),
            arm,
            y: [0.; MAX],
            t: 0.,
            shift: 0.,
            diag: [0.; MAX],
            diag_boxes: [Interval::point(0.)?; MAX],
            inv_margin: 0.,
            q: 0.,
            lu: DenseLuStageClient::new(n, DenseLuMode::InPlace)?,
            factored: false,
            has_jac: false,
            counts: Counts::default(),
            receipt: receipt.clone(),
        };
        let (r, partial, client) = integrate_rodas5p_fast_client_observed_with_atol(
            &p,
            Box::new(client),
            (0., 0.1),
            y0,
            &cfg,
            &schedule,
            &vec![cfg.atol; n],
        )?;
        assert!(partial.is_none());
        drop(client);
        r
    };
    let elapsed = start.elapsed().as_secs_f64();
    assert!(run.observed.success && run.observed.t == grid);
    let counts = receipt.lock().unwrap().clone();
    if arm != Arm::Protected {
        assert_eq!(counts.stage_calls, 8 * run.attempts as u64);
        assert_eq!(
            counts.candidate_stages + counts.fallback_stages,
            counts.stage_calls
        );
        assert!(counts.stage_calls_with_negative_rhs > 0);
    }
    Ok(
        json!({"source":source,"arm":format!("{arm:?}"),"rtol":rtol,"elapsed_seconds":elapsed,
        "accepted":run.accepted_steps,"rejected":run.rejected_steps,"attempts":run.attempts,
        "times":grid,"trajectory":run.observed.y,"core_work":run.observed.counters,"client_work":counts}),
    )
}
fn main() -> CoreResult<()> {
    rodas5p_integrators::affine_event_certificate::check_affine_event_arithmetic()?;
    let mut arms = [Arm::Protected, Arm::Direct, Arm::Continued, Arm::Capped];
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => {}
        [flag] if flag == "--reverse" => arms.reverse(),
        _ => {
            return Err(rodas5p_core::CoreError::InvalidInput(
                "usage: r5_cubic_stages [--reverse]".into(),
            ));
        }
    }
    for source in sources() {
        for tol in [1e-5, 1e-7] {
            for arm in arms {
                println!(
                    "{}",
                    run(
                        source,
                        arm,
                        tol,
                        if source.name == "strong_exchange_stress" {
                            0.01
                        } else {
                            1e-3
                        }
                    )?
                );
            }
        }
    }
    Ok(())
}
