//! The nine triage problems on the Diffsol side.
//!
//! Every equation, parameter, initial state, time span and `atol_scale` below is
//! transcribed from the Rust triage registry (`crates/rodas5p-cli/src/triage.rs`,
//! `PROBLEMS`) and the builders it calls; the source of each one is named on the
//! model. The right-hand sides keep the operation order of the Rust closures, so
//! `f` agrees bit for bit (checked by `diffsol-triage parity`).
//!
//! Diffsol takes the Jacobian as a Jacobian-vector product `J v`
//! (`OdeBuilder::rhs_implicit`). Where the Rust problem has an analytic JVP it is
//! copied; where it has only an analytic dense Jacobian (HIRES, Brusselator) the
//! JVP is `sum_k J_ik v_k` over the same entry expressions, so Diffsol's dense
//! assembly `J e_j` (`NonLinearOpJacobian::_default_jacobian_inplace`,
//! `use_coloring = false`) reproduces every entry exactly. Oregonator and Pollution
//! are JVP-only in Rust; their JVPs are copied.

use std::f64::consts::PI;

pub type RhsFn = Box<dyn Fn(f64, &[f64], &mut [f64])>;
pub type JvpFn = Box<dyn Fn(f64, &[f64], &[f64], &mut [f64])>;

pub struct Model {
    pub id: &'static str,
    pub tier: &'static str,
    pub t_span: (f64, f64),
    pub atol_scale: f64,
    pub autonomous: bool,
    /// What the Rust registry declares (`triage-list` `jacobian`).
    pub rust_jacobian: &'static str,
    /// How the J v closure given to Diffsol was obtained.
    pub jvp_origin: &'static str,
    pub source: &'static str,
    pub y0: Vec<f64>,
    pub rhs: RhsFn,
    pub jvp: JvpFn,
}

impl Model {
    pub fn dimension(&self) -> usize {
        self.y0.len()
    }
}

pub const PROBLEM_IDS: [&str; 9] = [
    "robertson",
    "hires",
    "van-der-pol-mu1000",
    "oregonator",
    "pollution",
    "prothero-robinson-nl",
    "scalar-linear",
    "brusselator-1d-50",
    "brusselator-1d-250",
];

pub fn model(id: &str) -> Option<Model> {
    match id {
        "robertson" => Some(robertson()),
        "hires" => Some(hires()),
        "van-der-pol-mu1000" => Some(van_der_pol(1.0e3)),
        "oregonator" => Some(oregonator()),
        "pollution" => Some(pollution()),
        "prothero-robinson-nl" => Some(prothero_robinson(-1.0e6, 10.0, 0.0)),
        "scalar-linear" => Some(scalar_linear(-1.0e4, 1.0)),
        "brusselator-1d-50" => Some(brusselator("brusselator-1d-50", 50)),
        "brusselator-1d-250" => Some(brusselator("brusselator-1d-250", 250)),
        _ => None,
    }
}

/// `atol_scale = spec.atol / spec.rtol` of the first `ScientificCorpusV2`
/// holdout spec (`rtol = CORPUS_RTOLS[0] = 1e-4`, `atol = 0.01 rtol`), as in
/// `triage.rs::corpus`: 0.010000000000000002.
fn corpus_v2_atol_scale() -> f64 {
    let rtol = 1.0e-4;
    let atol = 0.01 * rtol;
    atol / rtol
}

/// `rodas5p_integrators::problems::robertson_problem`; t_span and atol_scale of
/// `stiff_benchmark::default_problems` (triage.rs: `(0, 40)`, `1e-4`).
fn robertson() -> Model {
    Model {
        id: "robertson",
        tier: "G",
        t_span: (0.0, 40.0),
        atol_scale: 1.0e-4,
        autonomous: true,
        rust_jacobian: "analytic",
        jvp_origin: "copy of the Rust analytic JVP (problems.rs::robertson_problem)",
        source: "rodas5p_integrators::problems::robertson_problem; t_span [0, 40], atol_scale 1e-4 (triage.rs PROBLEMS)",
        y0: vec![1.0, 0.0, 0.0],
        rhs: Box::new(|_t, y, out| {
            out[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
            out[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
            out[2] = 3.0e7 * y[1] * y[1];
        }),
        jvp: Box::new(|_t, y, v, out| {
            out[0] = -0.04 * v[0] + 1.0e4 * y[2] * v[1] + 1.0e4 * y[1] * v[2];
            out[1] = 0.04 * v[0] + (-1.0e4 * y[2] - 6.0e7 * y[1]) * v[1] - 1.0e4 * y[1] * v[2];
            out[2] = 6.0e7 * y[1] * v[1];
        }),
    }
}

/// `rodas5p-cli stiff_benchmark::hires_problem`; t_span `(0, 321.8122)`,
/// atol_scale `1e-4` (triage.rs PROBLEMS).
fn hires() -> Model {
    Model {
        id: "hires",
        tier: "G",
        t_span: (0.0, 321.8122),
        atol_scale: 1.0e-4,
        autonomous: true,
        rust_jacobian: "analytic",
        jvp_origin: "sum_k J_ik v_k over the entries of stiff_benchmark.rs::hires_fill (Rust has no JVP)",
        source: "rodas5p-cli stiff_benchmark::hires_problem; t_span [0, 321.8122], atol_scale 1e-4 (triage.rs PROBLEMS)",
        y0: vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057],
        rhs: Box::new(|_t, y, out| {
            out[0] = -1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007;
            out[1] = 1.71 * y[0] - 8.75 * y[1];
            out[2] = -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4];
            out[3] = 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3];
            out[4] = -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6];
            out[5] = -280.0 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6];
            out[6] = 280.0 * y[5] * y[7] - 1.81 * y[6];
            out[7] = -280.0 * y[5] * y[7] + 1.81 * y[6];
        }),
        jvp: Box::new(|_t, y, v, out| {
            // Entries of hires_fill; structurally zero entries contribute nothing.
            let j55 = -280.0 * y[7] - 0.43;
            let j57 = -280.0 * y[5];
            let j65 = 280.0 * y[7];
            let j67 = 280.0 * y[5];
            let j75 = -280.0 * y[7];
            let j77 = -280.0 * y[5];
            out[0] = -1.71 * v[0] + 0.43 * v[1] + 8.32 * v[2];
            out[1] = 1.71 * v[0] + -8.75 * v[1];
            out[2] = -10.03 * v[2] + 0.43 * v[3] + 0.035 * v[4];
            out[3] = 8.32 * v[1] + 1.71 * v[2] + -1.12 * v[3];
            out[4] = -1.745 * v[4] + 0.43 * v[5] + 0.43 * v[6];
            out[5] = 0.69 * v[3] + 1.71 * v[4] + j55 * v[5] + 0.69 * v[6] + j57 * v[7];
            out[6] = j65 * v[5] + -1.81 * v[6] + j67 * v[7];
            out[7] = j75 * v[5] + 1.81 * v[6] + j77 * v[7];
        }),
    }
}

/// `rodas5p_integrators::problems::stiff_van_der_pol_problem(1e3)`; t_span
/// `(0, 2000)`, atol_scale `1` (triage.rs PROBLEMS).
fn van_der_pol(mu: f64) -> Model {
    Model {
        id: "van-der-pol-mu1000",
        tier: "G",
        t_span: (0.0, 2000.0),
        atol_scale: 1.0,
        autonomous: true,
        rust_jacobian: "analytic",
        jvp_origin: "copy of the Rust analytic JVP (problems.rs::stiff_van_der_pol_problem)",
        source: "rodas5p_integrators::problems::stiff_van_der_pol_problem(1e3); t_span [0, 2000], atol_scale 1 (triage.rs PROBLEMS)",
        y0: vec![2.0, 0.0],
        rhs: Box::new(move |_t, y, out| {
            out[0] = y[1];
            out[1] = mu * (1.0 - y[0] * y[0]) * y[1] - y[0];
        }),
        jvp: Box::new(move |_t, y, v, out| {
            out[0] = v[1];
            out[1] = (-2.0 * mu * y[0] * y[1] - 1.0) * v[0] + mu * (1.0 - y[0] * y[0]) * v[1];
        }),
    }
}

/// `ScientificCorpusV2` holdout family `oregonator`
/// (`scientific_corpus_v2.rs::oregonator_holdout`, JVP only); t_span `(0, 360)`.
fn oregonator() -> Model {
    const S: f64 = 77.27;
    const Q: f64 = 8.375e-6;
    const W: f64 = 0.161;
    Model {
        id: "oregonator",
        tier: "G",
        t_span: (0.0, 360.0),
        atol_scale: corpus_v2_atol_scale(),
        autonomous: true,
        rust_jacobian: "jvp",
        jvp_origin: "copy of the Rust JVP (scientific_corpus_v2.rs::oregonator_holdout); Rust has no Jacobian",
        source: "rodas5p_integrators::ScientificCorpusV2::holdout_specs() family oregonator (oregonator_holdout, JVP only); t_span [0, 360], atol_scale = spec.atol / spec.rtol",
        y0: vec![1.0, 2.0, 3.0],
        rhs: Box::new(|_t, y, out| {
            out[0] = S * (y[1] + y[0] * (1.0 - Q * y[0] - y[1]));
            out[1] = (y[2] - (1.0 + y[0]) * y[1]) / S;
            out[2] = W * (y[0] - y[2]);
        }),
        jvp: Box::new(|_t, y, v, out| {
            out[0] = S * ((1.0 - 2.0 * Q * y[0] - y[1]) * v[0] + (1.0 - y[0]) * v[1]);
            out[1] = (-y[1] * v[0] - (1.0 + y[0]) * v[1] + v[2]) / S;
            out[2] = W * (v[0] - v[2]);
        }),
    }
}

const POLLUTION_K: [f64; 25] = [
    0.35, 26.6, 12_300.0, 8.6e-4, 8.2e-4, 15_000.0, 1.3e-4, 24_000.0, 16_500.0, 9_000.0, 0.022,
    12_000.0, 1.88, 16_300.0, 4.8e6, 3.5e-4, 0.0175, 1.0e8, 4.44e11, 1_240.0, 2.1, 5.78, 0.0474,
    1_780.0, 3.12,
];

fn pollution_rates(y: &[f64]) -> [f64; 25] {
    let k = &POLLUTION_K;
    [
        k[0] * y[0],
        k[1] * y[1] * y[3],
        k[2] * y[4] * y[1],
        k[3] * y[6],
        k[4] * y[6],
        k[5] * y[6] * y[5],
        k[6] * y[8],
        k[7] * y[8] * y[5],
        k[8] * y[10] * y[1],
        k[9] * y[10] * y[0],
        k[10] * y[12],
        k[11] * y[9] * y[1],
        k[12] * y[13],
        k[13] * y[0] * y[5],
        k[14] * y[2],
        k[15] * y[3],
        k[16] * y[3],
        k[17] * y[15],
        k[18] * y[15],
        k[19] * y[16] * y[5],
        k[20] * y[18],
        k[21] * y[18],
        k[22] * y[0] * y[3],
        k[23] * y[18] * y[0],
        k[24] * y[19],
    ]
}

fn pollution_rate_derivatives(y: &[f64], v: &[f64]) -> [f64; 25] {
    let k = &POLLUTION_K;
    [
        k[0] * v[0],
        k[1] * (v[1] * y[3] + y[1] * v[3]),
        k[2] * (v[4] * y[1] + y[4] * v[1]),
        k[3] * v[6],
        k[4] * v[6],
        k[5] * (v[6] * y[5] + y[6] * v[5]),
        k[6] * v[8],
        k[7] * (v[8] * y[5] + y[8] * v[5]),
        k[8] * (v[10] * y[1] + y[10] * v[1]),
        k[9] * (v[10] * y[0] + y[10] * v[0]),
        k[10] * v[12],
        k[11] * (v[9] * y[1] + y[9] * v[1]),
        k[12] * v[13],
        k[13] * (v[0] * y[5] + y[0] * v[5]),
        k[14] * v[2],
        k[15] * v[3],
        k[16] * v[3],
        k[17] * v[15],
        k[18] * v[15],
        k[19] * (v[16] * y[5] + y[16] * v[5]),
        k[20] * v[18],
        k[21] * v[18],
        k[22] * (v[0] * y[3] + y[0] * v[3]),
        k[23] * (v[18] * y[0] + y[18] * v[0]),
        k[24] * v[19],
    ]
}

fn assemble_pollution(r: &[f64; 25], out: &mut [f64]) {
    out[0] =
        -r[0] - r[9] - r[13] - r[22] - r[23] + r[1] + r[2] + r[8] + r[10] + r[11] + r[21] + r[24];
    out[1] = -r[1] - r[2] - r[8] - r[11] + r[0] + r[20];
    out[2] = -r[14] + r[0] + r[16] + r[18] + r[21];
    out[3] = -r[1] - r[15] - r[16] - r[22] + r[14];
    out[4] = -r[2] + 2.0 * r[3] + r[5] + r[6] + r[12] + r[19];
    out[5] = -r[5] - r[7] - r[13] - r[19] + r[2] + 2.0 * r[17];
    out[6] = -r[3] - r[4] - r[5] + r[12];
    out[7] = r[3] + r[4] + r[5] + r[6];
    out[8] = -r[6] - r[7];
    out[9] = -r[11] + r[6] + r[8];
    out[10] = -r[8] - r[9] + r[7] + r[10];
    out[11] = r[8];
    out[12] = -r[10] + r[9];
    out[13] = -r[12] + r[11];
    out[14] = r[13];
    out[15] = -r[17] - r[18] + r[15];
    out[16] = -r[19];
    out[17] = r[19];
    out[18] = -r[20] - r[21] - r[23] + r[22] + r[24];
    out[19] = -r[24] + r[23];
}

/// `ScientificCorpusV2` holdout family `pollution`
/// (`scientific_corpus_v2.rs::pollution_holdout`, JVP only); t_span `(0, 60)`.
fn pollution() -> Model {
    Model {
        id: "pollution",
        tier: "G",
        t_span: (0.0, 60.0),
        atol_scale: corpus_v2_atol_scale(),
        autonomous: true,
        rust_jacobian: "jvp",
        jvp_origin: "copy of the Rust JVP (scientific_corpus_v2.rs::pollution_holdout); Rust has no Jacobian",
        source: "rodas5p_integrators::ScientificCorpusV2::holdout_specs() family pollution (pollution_holdout, JVP only); t_span [0, 60], atol_scale = spec.atol / spec.rtol",
        y0: vec![
            0.0, 0.2, 0.0, 0.04, 0.0, 0.0, 0.1, 0.3, 0.01, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.007, 0.0, 0.0, 0.0,
        ],
        rhs: Box::new(|_t, y, out| assemble_pollution(&pollution_rates(y), out)),
        jvp: Box::new(|_t, y, v, out| assemble_pollution(&pollution_rate_derivatives(y, v), out)),
    }
}

/// `rodas5p_integrators::problems::prothero_robinson_problem(-1e6, 10, 0)`:
/// `y' = lambda (y - sin t) + cos t + mu (y - sin t)^2`; t_span `(0, 1)`,
/// atol_scale `1e-3` (triage.rs PROBLEMS).
fn prothero_robinson(lambda: f64, mu: f64, t0: f64) -> Model {
    Model {
        id: "prothero-robinson-nl",
        tier: "G",
        t_span: (0.0, 1.0),
        atol_scale: 1.0e-3,
        autonomous: false,
        rust_jacobian: "analytic",
        jvp_origin: "copy of the Rust analytic JVP (problems.rs::prothero_robinson_problem)",
        source: "rodas5p_integrators::problems::prothero_robinson_problem(-1e6, 10, 0); t_span [0, 1], atol_scale 1e-3 (triage.rs PROBLEMS); Diffsol does not use the Rust df/dt callback",
        y0: vec![t0.sin()],
        rhs: Box::new(move |t, y, out| {
            let d = y[0] - t.sin();
            out[0] = lambda * d + t.cos() + mu * d * d;
        }),
        jvp: Box::new(move |t, y, v, out| {
            let d = y[0] - t.sin();
            out[0] = (lambda + 2.0 * mu * d) * v[0];
        }),
    }
}

/// `rodas5p_integrators::problems::scalar_linear_problem(-1e4, 1)`; t_span
/// `(0, 1e-2)`, atol_scale `1e-3` (triage.rs PROBLEMS).
fn scalar_linear(lambda: f64, y0: f64) -> Model {
    Model {
        id: "scalar-linear",
        tier: "G",
        t_span: (0.0, 1.0e-2),
        atol_scale: 1.0e-3,
        autonomous: true,
        rust_jacobian: "analytic",
        jvp_origin: "copy of the Rust analytic JVP (problems.rs::scalar_linear_problem)",
        source: "rodas5p_integrators::problems::scalar_linear_problem(-1e4, 1); t_span [0, 1e-2], atol_scale 1e-3 (triage.rs PROBLEMS)",
        y0: vec![y0],
        rhs: Box::new(move |_t, y, out| {
            out[0] = lambda * y[0];
        }),
        jvp: Box::new(move |_t, _y, v, out| {
            out[0] = lambda * v[0];
        }),
    }
}

/// `rodas5p-cli stiff_benchmark::brusselator_problem(cells)`: the 1D Brusselator
/// of Hairer-Wanner II IV.1, interleaved `u_1, v_1, ...`, Dirichlet values
/// `(u, v) = (1, 3)` at both ends; t_span `(0, 10)`, atol_scale `1`.
fn brusselator(id: &'static str, cells: usize) -> Model {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    let y0 = (0..cells)
        .flat_map(|i| {
            let x = (i as f64 + 1.0) / (cells as f64 + 1.0);
            [1.0 + (2.0 * PI * x).sin(), 3.0]
        })
        .collect();
    Model {
        id,
        tier: "P",
        t_span: (0.0, 10.0),
        atol_scale: 1.0,
        autonomous: true,
        rust_jacobian: "analytic",
        jvp_origin: "sum_k J_ik v_k over the entries of stiff_benchmark.rs::brusselator_problem `fill` (Rust has no JVP)",
        source: "rodas5p-cli stiff_benchmark::brusselator_problem(cells); t_span [0, 10], atol_scale 1 (triage.rs brusselator_1d)",
        y0,
        rhs: Box::new(move |_t, y, out| {
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
        jvp: Box::new(move |_t, y, w, out| {
            for i in 0..cells {
                let (u, v) = (y[2 * i], y[2 * i + 1]);
                let (a, b) = (2 * i, 2 * i + 1);
                let jaa = 2.0 * u * v - 4.0 - 2.0 * c;
                let jab = u * u;
                let jba = 3.0 - 2.0 * u * v;
                let jbb = -u * u - 2.0 * c;
                let mut ra = jaa * w[a] + jab * w[b];
                let mut rb = jba * w[a] + jbb * w[b];
                if i > 0 {
                    ra += c * w[a - 2];
                    rb += c * w[b - 2];
                }
                if i + 1 < cells {
                    ra += c * w[a + 2];
                    rb += c * w[b + 2];
                }
                out[a] = ra;
                out[b] = rb;
            }
        }),
    }
}
