//! The problems of loop node V07 (speculative Jacobian, stage a), shared by
//! `stage_clients_speculative_{contracts,alloc,measure}.rs` through
//! `#[path]`. As a test target of its own this file holds no test.
//!
//! A [`Model`] keeps its callbacks, so the same right-hand side can be built
//! into a plain [`OdeProblem`] (for the protected driver) and into one whose
//! right-hand side is wrapped by a
//! [`StageArgumentRecorder`] (for the speculative client); `OdeProblem`
//! itself keeps its callbacks private.
//!
//! The definitions are those of
//! `research/production_loop_20261116/speculative_jacobian/PREDECLARED.json`
//! and of `reference/models.py` there, written in the same order of
//! operations.
#![allow(dead_code)]

use std::sync::Arc;

use rodas5p_core::{CoreResult, DenseMatrix};
use rodas5p_integrators::stage_clients::DenseLuMode;
use rodas5p_integrators::stage_clients::speculative::{StageArgumentRecorder, StageRhs};
use rodas5p_integrators::{AdaptiveStepConfig, OdeProblem, OutputSchedule};

/// Writes every entry of the model's fixed Jacobian pattern.
pub type JacobianFill = Arc<dyn Fn(f64, &[f64], &mut DenseMatrix) + Send + Sync>;
/// The analytic time derivative of the right-hand side.
pub type PartialT = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
/// A closed-form solution.
pub type Exact = Arc<dyn Fn(f64) -> Vec<f64> + Send + Sync>;

/// The absolute tolerance of a measurement cell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AtolRule {
    /// `atol = rtol * scale`.
    Scale(f64),
    /// The same `atol` at every `rtol`.
    Fixed(f64),
}

/// One problem with its callbacks.
#[derive(Clone)]
pub struct Model {
    pub id: String,
    pub n: usize,
    pub t_span: (f64, f64),
    pub y0: Vec<f64>,
    pub atol: AtolRule,
    pub autonomous: bool,
    pub rhs: StageRhs,
    pub fill: JacobianFill,
    pub partial_t: Option<PartialT>,
    pub exact: Option<Exact>,
}

impl Model {
    /// The absolute tolerance of the cell at `rtol`.
    pub fn atol(&self, rtol: f64) -> f64 {
        match self.atol {
            AtolRule::Scale(scale) => rtol * scale,
            AtolRule::Fixed(atol) => atol,
        }
    }

    /// The problem with `rhs` as its right-hand side: the model's Jacobian
    /// (allocating and in place, the same fill) and its time derivative.
    pub fn build(&self, rhs: StageRhs) -> OdeProblem {
        let n = self.n;
        let fill = Arc::clone(&self.fill);
        let jacobian = Arc::new(move |t: f64, y: &[f64]| {
            let mut j = DenseMatrix::zeros(n, n);
            fill(t, y, &mut j);
            Ok(j)
        });
        let fill = Arc::clone(&self.fill);
        let jacobian_into = Arc::new(move |t: f64, y: &[f64], j: &mut DenseMatrix| {
            fill(t, y, j);
            Ok(())
        });
        OdeProblem::new(
            self.id.clone(),
            n,
            rhs,
            None,
            Some(jacobian),
            None,
            self.partial_t.clone(),
            self.autonomous,
            None,
            None,
        )
        .expect("a valid model")
        .with_jacobian_into(jacobian_into)
    }

    /// The problem as the protected driver integrates it.
    pub fn problem(&self) -> OdeProblem {
        self.build(Arc::clone(&self.rhs))
    }

    /// The problem behind a fresh recorder, and the recorder.
    pub fn recorded_problem(&self) -> (OdeProblem, Arc<StageArgumentRecorder>) {
        let recorder = StageArgumentRecorder::new(self.n).expect("a recorder");
        let problem = self.build(recorder.wrap_rhs(Arc::clone(&self.rhs)));
        (problem, recorder)
    }

    /// The LU the protected dense driver picks for this model.
    pub fn lu_mode(&self) -> DenseLuMode {
        DenseLuMode::protected_choice_at(&self.problem(), self.t_span.0, &self.y0)
            .expect("the first Jacobian")
    }

    /// The Jacobian at `(t, y)`.
    pub fn jacobian(&self, t: f64, y: &[f64]) -> DenseMatrix {
        let mut j = DenseMatrix::zeros(self.n, self.n);
        (self.fill)(t, y, &mut j);
        j
    }

    /// The configuration of an adaptive measurement cell
    /// (`PREDECLARED.json`, `driver_configuration`).
    pub fn adaptive_config(&self, rtol: f64) -> AdaptiveStepConfig {
        AdaptiveStepConfig {
            atol: self.atol(rtol),
            rtol,
            initial_step: 1.0e-6,
            min_step: 1.0e-14,
            max_step: self.t_span.1 - self.t_span.0,
            max_attempts: 1_000_000,
            ..AdaptiveStepConfig::default()
        }
    }

    /// The configuration of a constant-step run with `steps` steps: every
    /// attempt is accepted and every proposal is capped at the step.
    pub fn fixed_step_config(&self, steps: usize) -> AdaptiveStepConfig {
        let h = (self.t_span.1 - self.t_span.0) / steps as f64;
        AdaptiveStepConfig {
            atol: FIXED_STEP_TOLERANCE,
            rtol: FIXED_STEP_TOLERANCE,
            initial_step: h,
            min_step: 1.0e-14,
            max_step: h,
            max_attempts: 1_000_000,
            ..AdaptiveStepConfig::default()
        }
    }

    /// The output schedule of every measurement: the two ends of the span.
    pub fn end_points(&self) -> OutputSchedule {
        OutputSchedule::new(vec![self.t_span.0, self.t_span.1]).expect("two output times")
    }
}

/// `rtol = atol` of the constant-step runs.
pub const FIXED_STEP_TOLERANCE: f64 = 1.0e10;

/// The tolerances of the adaptive set.
pub const RTOLS: [f64; 3] = [1.0e-4, 1.0e-6, 1.0e-8];

/// The step counts of the order set (`h = 1.5 / N`, all dyadic).
pub const ORDER_STEPS: [usize; 16] = [
    6, 8, 12, 16, 24, 32, 48, 64, 96, 128, 192, 256, 384, 512, 768, 1024,
];

fn rhs_fn(f: impl Fn(f64, &[f64], &mut [f64]) + Send + Sync + 'static) -> StageRhs {
    Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        f(t, y, out);
        Ok(())
    })
}

fn fill_fn(f: impl Fn(f64, &[f64], &mut DenseMatrix) + Send + Sync + 'static) -> JacobianFill {
    Arc::new(f)
}

fn autonomous(
    id: &str,
    t_span: (f64, f64),
    y0: Vec<f64>,
    atol: AtolRule,
    rhs: StageRhs,
    fill: JacobianFill,
) -> Model {
    Model {
        id: id.into(),
        n: y0.len(),
        t_span,
        y0,
        atol,
        autonomous: true,
        rhs,
        fill,
        partial_t: None,
        exact: None,
    }
}

pub fn robertson() -> Model {
    autonomous(
        "robertson",
        (0.0, 40.0),
        vec![1.0, 0.0, 0.0],
        AtolRule::Scale(1.0e-4),
        rhs_fn(|_t, y, out| {
            out[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
            out[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
            out[2] = 3.0e7 * y[1] * y[1];
        }),
        fill_fn(|_t, y, j| {
            j[(0, 0)] = -0.04;
            j[(0, 1)] = 1.0e4 * y[2];
            j[(0, 2)] = 1.0e4 * y[1];
            j[(1, 0)] = 0.04;
            j[(1, 1)] = -1.0e4 * y[2] - 6.0e7 * y[1];
            j[(1, 2)] = -1.0e4 * y[1];
            j[(2, 1)] = 6.0e7 * y[1];
        }),
    )
}

pub const VDPOL_EPS: f64 = 1.0e-6;

pub fn vdpol() -> Model {
    let eps = VDPOL_EPS;
    autonomous(
        "vdpol-eps1e-6",
        (0.0, 2.0),
        vec![2.0, 0.0],
        AtolRule::Scale(1.0),
        rhs_fn(move |_t, y, out| {
            out[0] = y[1];
            out[1] = ((1.0 - y[0] * y[0]) * y[1] - y[0]) / eps;
        }),
        fill_fn(move |_t, y, j| {
            j[(0, 1)] = 1.0;
            j[(1, 0)] = (-2.0 * y[0] * y[1] - 1.0) / eps;
            j[(1, 1)] = (1.0 - y[0] * y[0]) / eps;
        }),
    )
}

pub fn oregonator() -> Model {
    const S: f64 = 77.27;
    const Q: f64 = 8.375e-6;
    const W: f64 = 0.161;
    autonomous(
        "oregonator",
        (0.0, 360.0),
        vec![1.0, 2.0, 3.0],
        AtolRule::Scale(1.0),
        rhs_fn(|_t, y, out| {
            out[0] = S * (y[1] + y[0] * (1.0 - Q * y[0] - y[1]));
            out[1] = (y[2] - (1.0 + y[0]) * y[1]) / S;
            out[2] = W * (y[0] - y[2]);
        }),
        fill_fn(|_t, y, j| {
            j[(0, 0)] = S * (1.0 - 2.0 * Q * y[0] - y[1]);
            j[(0, 1)] = S * (1.0 - y[0]);
            j[(1, 0)] = -y[1] / S;
            j[(1, 1)] = -(1.0 + y[0]) / S;
            j[(1, 2)] = 1.0 / S;
            j[(2, 0)] = W;
            j[(2, 2)] = -W;
        }),
    )
}

pub fn hires() -> Model {
    autonomous(
        "hires",
        (0.0, 321.8122),
        vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057],
        AtolRule::Scale(1.0e-4),
        rhs_fn(|_t, y, out| {
            out[0] = -1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007;
            out[1] = 1.71 * y[0] - 8.75 * y[1];
            out[2] = -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4];
            out[3] = 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3];
            out[4] = -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6];
            out[5] = -280.0 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6];
            out[6] = 280.0 * y[5] * y[7] - 1.81 * y[6];
            out[7] = -280.0 * y[5] * y[7] + 1.81 * y[6];
        }),
        fill_fn(|_t, y, j| {
            j[(0, 0)] = -1.71;
            j[(0, 1)] = 0.43;
            j[(0, 2)] = 8.32;
            j[(1, 0)] = 1.71;
            j[(1, 1)] = -8.75;
            j[(2, 2)] = -10.03;
            j[(2, 3)] = 0.43;
            j[(2, 4)] = 0.035;
            j[(3, 1)] = 8.32;
            j[(3, 2)] = 1.71;
            j[(3, 3)] = -1.12;
            j[(4, 4)] = -1.745;
            j[(4, 5)] = 0.43;
            j[(4, 6)] = 0.43;
            j[(5, 3)] = 0.69;
            j[(5, 4)] = 1.71;
            j[(5, 5)] = -280.0 * y[7] - 0.43;
            j[(5, 6)] = 0.69;
            j[(5, 7)] = -280.0 * y[5];
            j[(6, 5)] = 280.0 * y[7];
            j[(6, 6)] = -1.81;
            j[(6, 7)] = 280.0 * y[5];
            j[(7, 5)] = -280.0 * y[7];
            j[(7, 6)] = 1.81;
            j[(7, 7)] = -280.0 * y[5];
        }),
    )
}

const POLLUTION_K: [f64; 25] = [
    0.35, 26.6, 12300.0, 8.6e-4, 8.2e-4, 15000.0, 1.3e-4, 24000.0, 16500.0, 9000.0, 0.022, 12000.0,
    1.88, 16300.0, 4.8e6, 3.5e-4, 0.0175, 1.0e8, 4.44e11, 1240.0, 2.1, 5.78, 0.0474, 1780.0, 3.12,
];

/// Reactants of reaction `k` (0-based species) in the order of the rate
/// product; `usize::MAX` marks a first-order reaction.
const POLLUTION_REACTANTS: [(usize, usize); 25] = [
    (0, usize::MAX),
    (1, 3),
    (4, 1),
    (6, usize::MAX),
    (6, usize::MAX),
    (6, 5),
    (8, usize::MAX),
    (8, 5),
    (10, 1),
    (10, 0),
    (12, usize::MAX),
    (9, 1),
    (13, usize::MAX),
    (0, 5),
    (2, usize::MAX),
    (3, usize::MAX),
    (3, usize::MAX),
    (15, usize::MAX),
    (15, usize::MAX),
    (16, 5),
    (18, usize::MAX),
    (18, usize::MAX),
    (0, 3),
    (18, 0),
    (19, usize::MAX),
];

/// Species `i`: the signed reactions of its right-hand side in summation
/// order.
const POLLUTION_STOICH: [&[(usize, f64)]; 20] = [
    &[
        (0, -1.0),
        (9, -1.0),
        (13, -1.0),
        (22, -1.0),
        (23, -1.0),
        (1, 1.0),
        (2, 1.0),
        (8, 1.0),
        (10, 1.0),
        (11, 1.0),
        (21, 1.0),
        (24, 1.0),
    ],
    &[
        (1, -1.0),
        (2, -1.0),
        (8, -1.0),
        (11, -1.0),
        (0, 1.0),
        (20, 1.0),
    ],
    &[(14, -1.0), (0, 1.0), (16, 1.0), (18, 1.0), (21, 1.0)],
    &[(1, -1.0), (15, -1.0), (16, -1.0), (22, -1.0), (14, 1.0)],
    &[
        (2, -1.0),
        (3, 2.0),
        (5, 1.0),
        (6, 1.0),
        (12, 1.0),
        (19, 1.0),
    ],
    &[
        (5, -1.0),
        (7, -1.0),
        (13, -1.0),
        (19, -1.0),
        (2, 1.0),
        (17, 2.0),
    ],
    &[(3, -1.0), (4, -1.0), (5, -1.0), (12, 1.0)],
    &[(3, 1.0), (4, 1.0), (5, 1.0), (6, 1.0)],
    &[(6, -1.0), (7, -1.0)],
    &[(11, -1.0), (6, 1.0), (8, 1.0)],
    &[(8, -1.0), (9, -1.0), (7, 1.0), (10, 1.0)],
    &[(8, 1.0)],
    &[(10, -1.0), (9, 1.0)],
    &[(12, -1.0), (11, 1.0)],
    &[(13, 1.0)],
    &[(17, -1.0), (18, -1.0), (15, 1.0)],
    &[(19, -1.0)],
    &[(19, 1.0)],
    &[(20, -1.0), (21, -1.0), (23, -1.0), (22, 1.0), (24, 1.0)],
    &[(24, -1.0), (23, 1.0)],
];

pub fn pollution() -> Model {
    let mut y0 = vec![0.0; 20];
    y0[1] = 0.2;
    y0[3] = 0.04;
    y0[6] = 0.1;
    y0[7] = 0.3;
    y0[8] = 0.01;
    y0[16] = 0.007;
    autonomous(
        "pollution",
        (0.0, 60.0),
        y0,
        AtolRule::Scale(1.0e-6),
        rhs_fn(|_t, y, out| {
            let mut r = [0.0; 25];
            for (k, (a, b)) in POLLUTION_REACTANTS.iter().enumerate() {
                let mut value = POLLUTION_K[k] * y[*a];
                if *b != usize::MAX {
                    value *= y[*b];
                }
                r[k] = value;
            }
            for (i, terms) in POLLUTION_STOICH.iter().enumerate() {
                let (reaction, coefficient) = terms[0];
                let mut total = coefficient * r[reaction];
                for (reaction, coefficient) in &terms[1..] {
                    total += coefficient * r[*reaction];
                }
                out[i] = total;
            }
        }),
        fill_fn(|_t, y, j| {
            j.as_mut_slice().fill(0.0);
            for (i, terms) in POLLUTION_STOICH.iter().enumerate() {
                for (reaction, coefficient) in terms.iter() {
                    let (a, b) = POLLUTION_REACTANTS[*reaction];
                    if b == usize::MAX {
                        j[(i, a)] += coefficient * POLLUTION_K[*reaction];
                    } else {
                        j[(i, a)] += coefficient * POLLUTION_K[*reaction] * y[b];
                        j[(i, b)] += coefficient * POLLUTION_K[*reaction] * y[a];
                    }
                }
            }
        }),
    )
}

pub fn e5() -> Model {
    const A: f64 = 7.89e-10;
    const B: f64 = 1.1e7;
    const C: f64 = 1.13e3;
    const M: f64 = 1.0e6;
    let mc = M * C;
    autonomous(
        "e5",
        (0.0, 1.0e13),
        vec![1.76e-3, 0.0, 0.0, 0.0],
        AtolRule::Fixed(1.7e-24),
        rhs_fn(move |_t, y, out| {
            let f1 = -A * y[0] - B * y[0] * y[2];
            let f2 = A * y[0] - mc * y[1] * y[2];
            let f4 = B * y[0] * y[2] - C * y[3];
            out[0] = f1;
            out[1] = f2;
            out[2] = f2 - f4;
            out[3] = f4;
        }),
        fill_fn(move |_t, y, j| {
            j[(0, 0)] = -A - B * y[2];
            j[(0, 2)] = -B * y[0];
            j[(1, 0)] = A;
            j[(1, 1)] = -mc * y[2];
            j[(1, 2)] = -mc * y[1];
            j[(2, 0)] = A - B * y[2];
            j[(2, 1)] = -mc * y[2];
            j[(2, 2)] = -mc * y[1] - B * y[0];
            j[(2, 3)] = C;
            j[(3, 0)] = B * y[2];
            j[(3, 2)] = B * y[0];
            j[(3, 3)] = -C;
        }),
    )
}

pub fn brusselator(cells: usize) -> Model {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    let y0 = (0..cells)
        .flat_map(|i| {
            let x = (i as f64 + 1.0) / (cells as f64 + 1.0);
            [1.0 + (2.0 * std::f64::consts::PI * x).sin(), 3.0]
        })
        .collect();
    autonomous(
        &format!("brusselator-1d-{cells}"),
        (0.0, 10.0),
        y0,
        AtolRule::Scale(1.0),
        rhs_fn(move |_t, y, out| {
            for i in 0..cells {
                let (u, v) = (y[2 * i], y[2 * i + 1]);
                let (ul, vl) = if i == 0 {
                    (1.0, 3.0)
                } else {
                    (y[2 * i - 2], y[2 * i - 1])
                };
                let (ur, vr) = if i + 1 == cells {
                    (1.0, 3.0)
                } else {
                    (y[2 * i + 2], y[2 * i + 3])
                };
                out[2 * i] = 1.0 + u * u * v - 4.0 * u + c * (ul - 2.0 * u + ur);
                out[2 * i + 1] = 3.0 * u - u * u * v + c * (vl - 2.0 * v + vr);
            }
        }),
        fill_fn(move |_t, y, j| {
            for i in 0..cells {
                let (u, v) = (y[2 * i], y[2 * i + 1]);
                let (a, b) = (2 * i, 2 * i + 1);
                j[(a, a)] = 2.0 * u * v - 4.0 - 2.0 * c;
                j[(a, b)] = u * u;
                j[(b, a)] = 3.0 - 2.0 * u * v;
                j[(b, b)] = -u * u - 2.0 * c;
                if i > 0 {
                    j[(a, a - 2)] = c;
                    j[(b, b - 2)] = c;
                }
                if i + 1 < cells {
                    j[(a, a + 2)] = c;
                    j[(b, b + 2)] = c;
                }
            }
        }),
    )
}

/// The stiff rate of the Prothero-Robinson models.
pub const PR_L: f64 = 1.0e6;

/// `phi1, phi1', phi1'', phi2, phi2', phi2''` at `t`:
/// `phi1 = 1 + 0.5 sin(3 t)`, `phi2 = cos(2 t)`.
fn pr_phi(t: f64) -> [f64; 6] {
    [
        1.0 + 0.5 * (3.0 * t).sin(),
        1.5 * (3.0 * t).cos(),
        -4.5 * (3.0 * t).sin(),
        (2.0 * t).cos(),
        -2.0 * (2.0 * t).sin(),
        -4.0 * (2.0 * t).cos(),
    ]
}

/// The nonlinear Prothero-Robinson type problem on `[0, t_end]`:
/// `y1' = phi1' - L (1 + y2^2) (y1 - phi1)`,
/// `y2' = phi2' - (y2 - phi2) - (y1 - phi1) y2`, exact `(phi1, phi2)`.
pub fn pr_nonlinear(t_end: f64) -> Model {
    Model {
        id: "pr-nonlinear".into(),
        n: 2,
        t_span: (0.0, t_end),
        y0: vec![1.0, 1.0],
        atol: AtolRule::Scale(1.0),
        autonomous: false,
        rhs: rhs_fn(|t, y, out| {
            let p = pr_phi(t);
            let d1 = y[0] - p[0];
            out[0] = p[1] - PR_L * (1.0 + y[1] * y[1]) * d1;
            out[1] = p[4] - (y[1] - p[3]) - d1 * y[1];
        }),
        fill: fill_fn(|t, y, j| {
            let d1 = y[0] - pr_phi(t)[0];
            j[(0, 0)] = -PR_L * (1.0 + y[1] * y[1]);
            j[(0, 1)] = -2.0 * PR_L * y[1] * d1;
            j[(1, 0)] = -y[1];
            j[(1, 1)] = -1.0 - d1;
        }),
        partial_t: Some(Arc::new(|t: f64, y: &[f64], out: &mut [f64]| {
            let p = pr_phi(t);
            out[0] = p[2] + PR_L * (1.0 + y[1] * y[1]) * p[1];
            out[1] = p[5] + p[4] + p[1] * y[1];
            Ok(())
        })),
        exact: Some(Arc::new(|t: f64| {
            let p = pr_phi(t);
            vec![p[0], p[3]]
        })),
    }
}

/// The scalar Prothero-Robinson problem `y' = lambda (y - phi1) + phi1'`,
/// `lambda = -L`, exact `phi1`.
pub fn pr_linear() -> Model {
    let lambda = -PR_L;
    Model {
        id: "pr-linear".into(),
        n: 1,
        t_span: (0.0, 1.5),
        y0: vec![1.0],
        atol: AtolRule::Scale(1.0),
        autonomous: false,
        rhs: rhs_fn(move |t, y, out| {
            let p = pr_phi(t);
            out[0] = lambda * (y[0] - p[0]) + p[1];
        }),
        fill: fill_fn(move |_t, _y, j| {
            j[(0, 0)] = lambda;
        }),
        partial_t: Some(Arc::new(move |t: f64, _y: &[f64], out: &mut [f64]| {
            let p = pr_phi(t);
            out[0] = -lambda * p[1] + p[2];
            Ok(())
        })),
        exact: Some(Arc::new(|t: f64| vec![pr_phi(t)[0]])),
    }
}

/// The combustion problem `u_t = u_xx + D (1 + alpha - u) exp(-delta / u)`
/// on `cells` nodes `x_i = i / cells`, zero flux at `x = 0`, `u(1) = 1`.
pub fn combustion(cells: usize, t_end: f64) -> Model {
    const ALPHA: f64 = 1.0;
    const DELTA: f64 = 20.0;
    const R: f64 = 5.0;
    let d = R * DELTA.exp() / (ALPHA * DELTA);
    let inv = cells as f64 * cells as f64;
    autonomous(
        &format!("combustion-{cells}"),
        (0.0, t_end),
        vec![1.0; cells],
        AtolRule::Scale(1.0),
        rhs_fn(move |_t, y, out| {
            for i in 0..cells {
                let left = if i == 0 { y[1] } else { y[i - 1] };
                let right = if i + 1 == cells { 1.0 } else { y[i + 1] };
                out[i] = inv * (left - 2.0 * y[i] + right)
                    + d * (1.0 + ALPHA - y[i]) * (-DELTA / y[i]).exp();
            }
        }),
        fill_fn(move |_t, y, j| {
            for i in 0..cells {
                let e = (-DELTA / y[i]).exp();
                j[(i, i)] =
                    -2.0 * inv + d * e * ((1.0 + ALPHA - y[i]) * DELTA / (y[i] * y[i]) - 1.0);
                if i > 0 {
                    j[(i, i - 1)] = inv;
                }
                if i + 1 < cells {
                    j[(i, i + 1)] = inv;
                }
            }
            j[(0, 1)] = 2.0 * inv;
        }),
    )
}

/// The quasilinear diffusion-reaction problem
/// `u_t = (u^2 u_x)_x + 5 u (1 - u)` on `cells` nodes `x_i = i / cells`,
/// `i = 1..=cells`, `u(0) = 1`, zero flux at the right end.
pub fn nldiff(cells: usize, t_end: f64) -> Model {
    const RATE: f64 = 5.0;
    const U_LEFT: f64 = 1.0;
    const U_INIT: f64 = 0.1;
    let inv = cells as f64 * cells as f64;
    autonomous(
        &format!("nldiff-{cells}"),
        (0.0, t_end),
        vec![U_INIT; cells],
        AtolRule::Scale(1.0),
        rhs_fn(move |_t, y, out| {
            for i in 0..cells {
                let ul = if i == 0 { U_LEFT } else { y[i - 1] };
                let ur = if i + 1 == cells { y[i] } else { y[i + 1] };
                let wm = 0.5 * (ul + y[i]);
                let wp = 0.5 * (y[i] + ur);
                let flux = wp * wp * (ur - y[i]) - wm * wm * (y[i] - ul);
                out[i] = inv * flux + RATE * y[i] * (1.0 - y[i]);
            }
        }),
        fill_fn(move |_t, y, j| {
            for i in 0..cells {
                let ul = if i == 0 { U_LEFT } else { y[i - 1] };
                let ur = if i + 1 == cells { y[i] } else { y[i + 1] };
                let wm = 0.5 * (ul + y[i]);
                let wp = 0.5 * (y[i] + ur);
                let dm = y[i] - ul;
                let dp = ur - y[i];
                // d/du of wp^2 dp - wm^2 dm; at the last node u_{N+1} = u_N,
                // so the right flux vanishes identically.
                let (diagonal, upper) = if i + 1 == cells {
                    (-(wm * dm + wm * wm), 0.0)
                } else {
                    ((wp * dp - wp * wp) - (wm * dm + wm * wm), wp * dp + wp * wp)
                };
                let lower = -(wm * dm - wm * wm);
                j[(i, i)] = inv * diagonal + RATE * (1.0 - 2.0 * y[i]);
                if i > 0 {
                    j[(i, i - 1)] = inv * lower;
                }
                if i + 1 < cells {
                    j[(i, i + 1)] = inv * upper;
                }
            }
        }),
    )
}

/// The Kaps problem with stiffness `1 / eps`, exact `(exp(-2 t), exp(-t))`.
pub fn kaps(eps: f64, id: &str) -> Model {
    let mut model = autonomous(
        id,
        (0.0, 1.5),
        vec![1.0, 1.0],
        AtolRule::Scale(1.0),
        rhs_fn(move |_t, y, out| {
            out[0] = -(2.0 + 1.0 / eps) * y[0] + y[1] * y[1] / eps;
            out[1] = y[0] - y[1] * (1.0 + y[1]);
        }),
        fill_fn(move |_t, y, j| {
            j[(0, 0)] = -(2.0 + 1.0 / eps);
            j[(0, 1)] = 2.0 * y[1] / eps;
            j[(1, 0)] = 1.0;
            j[(1, 1)] = -1.0 - 2.0 * y[1];
        }),
    );
    model.exact = Some(Arc::new(|t: f64| vec![(-2.0 * t).exp(), (-t).exp()]));
    model
}

/// `y1' = 4 y1 (1 - y1)`, `y2' = -2 y1 y2`, `y0 = (0.1, 1)`; exact
/// `y1 = 1 / (1 + 9 exp(-4 t))`, `y2 = ((exp(4 t) + 9) / 10)^(-1/2)`.
pub fn logistic_pair() -> Model {
    const R: f64 = 4.0;
    const S: f64 = 2.0;
    let mut model = autonomous(
        "logistic-pair",
        (0.0, 1.5),
        vec![0.1, 1.0],
        AtolRule::Scale(1.0),
        rhs_fn(|_t, y, out| {
            out[0] = R * y[0] * (1.0 - y[0]);
            out[1] = -S * y[0] * y[1];
        }),
        fill_fn(|_t, y, j| {
            j[(0, 0)] = R * (1.0 - 2.0 * y[0]);
            j[(1, 0)] = -S * y[1];
            j[(1, 1)] = -S * y[0];
        }),
    );
    model.exact = Some(Arc::new(|t: f64| {
        vec![
            1.0 / (1.0 + 9.0 * (-R * t).exp()),
            (((R * t).exp() + 9.0) / 10.0).powf(-S / R),
        ]
    }));
    model
}

/// A deterministic state near `y0` with no zero entry (seed 0, 1, 2, ...
/// give different states): for checks of the model callbacks.
pub fn probe_state(model: &Model, seed: usize) -> Vec<f64> {
    let scale = model.y0.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
    model
        .y0
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let phase = (i + 1 + 7 * seed) as f64;
            v * (1.0 + 0.05 * phase.sin()) + 1.0e-3 * scale * (0.5 + 0.25 * phase.cos())
        })
        .collect()
}

/// The end time of `combustion-100` (`PREDECLARED.json`).
pub const COMBUSTION_T_END: f64 = 0.27;
/// The end time of `nldiff-100` (`PREDECLARED.json`).
pub const NLDIFF_T_END: f64 = 0.3;

/// The adaptive stiff set, in the order of `PREDECLARED.json`.
pub fn adaptive_models() -> Vec<Model> {
    vec![
        robertson(),
        vdpol(),
        oregonator(),
        hires(),
        pollution(),
        e5(),
        brusselator(32),
        brusselator(100),
        pr_nonlinear(10.0),
        combustion(100, COMBUSTION_T_END),
        nldiff(100, NLDIFF_T_END),
    ]
}

/// The fixed-step order set, in the order of `PREDECLARED.json`.
pub fn order_models() -> Vec<Model> {
    vec![
        kaps(1.0, "kaps-eps1"),
        logistic_pair(),
        kaps(1.0e-6, "kaps-eps1e-6"),
        pr_linear(),
        pr_nonlinear(1.5),
    ]
}
