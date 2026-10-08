//! Cells, references and direct-solve twins of research nodes
//! `research/alg01_coupled_stage_target_20261008` (ALG01) and
//! `research/alg03_stage_budget_guard_20261008` (ALG03).
//!
//! The stress problems transcribe the exploratory pilot
//! (`docs/reviews/20261008_algorithmic_directions/pilot/phase3_code/critic/sprobs.py`,
//! `e05mat.py`, `target/tprobs.py`); the E-05 operator is the construction
//! of `crates/rodas5p-krylov/tests/givens_production_differential_contracts.rs`
//! (same LCG, seed and Householder product). The fixed-step direct twin is
//! the U-form arithmetic of `rodas5p_matrix_free_fast.rs` with dense LU stage
//! solves.
#![allow(dead_code)]

use std::sync::Arc;

use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, InitialGuess, LinearMethod, LinearSolverConfig,
    LuFactorization, WorkCounters, matrix_exp_pade13, rodas5p_coefficients,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSchedule, Rodas5pMfFastResult, Rodas5pMfFastWorkspace,
    integrate_rodas5p_fast_observed, prothero_robinson_problem, robertson_problem,
    semilinear_advection_diffusion_problem,
};
use serde_json::{Value, json};

#[path = "../rnext_common/mod.rs"]
pub mod rnext;

pub use rnext::{adaptive, brusselator, cases, linear_config, write_output};

/// Linear rtol and atol of the production stage target (SPD07).
pub const LINEAR_RTOL: f64 = 1.0e-10;
/// The SPD07 / ALG01 Krylov budget and the ALG03 large budget.
pub const BUDGET: usize = 200;
pub const BIG_BUDGET: usize = 2000;
/// The pilot ladders' Krylov budget (reported supplement only).
pub const PILOT_LADDER_BUDGET: usize = 20_000;
pub const REFERENCE_RTOL: f64 = 1.0e-13;
pub const REFERENCE_RTOL_LOOSE: f64 = 1.0e-12;

/// One problem of a node: the JVP-only problem the matrix-free arms run,
/// the explicit-Jacobian problem of the dense twin and the reference, and an
/// exact endpoint where one exists.
pub struct Problem {
    pub id: String,
    pub problem: OdeProblem,
    pub full: OdeProblem,
    pub exact: Option<Box<dyn Fn(f64) -> Vec<f64>>>,
    /// Label of the exact endpoint's construction.
    pub exact_kind: &'static str,
    pub y0: Vec<f64>,
    pub t_span: (f64, f64),
    pub atol_scale: f64,
    pub max_attempts: usize,
}

impl Problem {
    pub fn span(&self) -> f64 {
        self.t_span.1 - self.t_span.0
    }

    pub fn adaptive(&self, rtol: f64) -> AdaptiveStepConfig {
        AdaptiveStepConfig {
            max_attempts: self.max_attempts,
            ..adaptive(rtol, self.atol_scale, self.span())
        }
    }

    pub fn schedule(&self) -> OutputSchedule {
        OutputSchedule::new(vec![self.t_span.0, self.t_span.1]).unwrap()
    }
}

pub fn hexes(v: &[f64]) -> Vec<String> {
    v.iter().map(|x| format!("{:016x}", x.to_bits())).collect()
}

/// The SPD07 run record (`spd07_mf_step_warm_start.rs::result_json`), so
/// that the C1 rows compare equal to SPD07's `BASE.json`.
pub fn result_json(result: &CoreResult<Rodas5pMfFastResult>) -> Value {
    match result {
        Ok(r) => json!({
            "ok": true,
            "success": r.observed.success,
            "message": r.observed.message,
            "t": hexes(&r.observed.t),
            "y_last": hexes(r.observed.y.last().unwrap()),
            "attempts": r.attempts,
            "accepted": r.accepted_steps,
            "rejected": r.rejected_steps,
            "state_reuses": r.state_reuses,
            "internal_steps": r.observed.internal_steps,
            "output_clipped_steps": r.observed.output_clipped_steps,
            "counters": serde_json::to_value(r.observed.counters).unwrap(),
        }),
        Err(e) => json!({"ok": false, "error": e.to_string()}),
    }
}

/// The SPD07 GMRES configuration (restart 40, linear rtol 1e-10, atol
/// 1e-14, no preconditioner, zero start) with Krylov budget `budget`.
pub fn gmres_config(budget: usize) -> LinearSolverConfig {
    LinearSolverConfig {
        x0_strategy: InitialGuess::Zero,
        maxiter: budget,
        ..linear_config(LinearMethod::Gmres, LINEAR_RTOL)
    }
}

/// The dense fast driver on the explicit-Jacobian problem (the twin).
pub fn dense_run(p: &Problem, adaptive: &AdaptiveStepConfig) -> Value {
    match integrate_rodas5p_fast_observed(&p.full, p.t_span, &p.y0, adaptive, &p.schedule()) {
        Ok(r) => json!({
            "ok": true,
            "success": r.observed.success,
            "message": r.observed.message,
            "t": hexes(&r.observed.t),
            "y_last": hexes(r.observed.y.last().unwrap()),
            "attempts": r.attempts,
            "accepted": r.accepted_steps,
            "rejected": r.rejected_steps,
            "counters": serde_json::to_value(r.observed.counters).unwrap(),
        }),
        Err(e) => json!({"ok": false, "error": e.to_string()}),
    }
}

pub fn twin(p: &Problem, rtol: f64) -> Value {
    dense_run(p, &p.adaptive(rtol))
}

/// The endpoint reference: the exact solution where one exists, else the
/// dense fast driver at rtol 1e-13; the dense run at 1e-12 is kept for the
/// reference uncertainty (and both dense runs are recorded next to an exact
/// endpoint, as a check).
pub fn reference(p: &Problem) -> Value {
    let tf = p.t_span.1;
    let dense = |rtol: f64| {
        dense_run(
            p,
            &AdaptiveStepConfig {
                max_attempts: 1_000_000,
                ..adaptive(rtol, p.atol_scale, p.span())
            },
        )
    };
    let tight = dense(REFERENCE_RTOL);
    let loose = dense(REFERENCE_RTOL_LOOSE);
    match &p.exact {
        Some(exact) => json!({
            "kind": p.exact_kind,
            "y": hexes(&exact(tf)),
            "dense_1e-13": tight,
            "dense_1e-12": loose,
        }),
        None => {
            assert_eq!(tight["success"], true, "reference run of {} failed", p.id);
            json!({
                "kind": "dense-fast-rtol-1e-13",
                "y": tight["y_last"].clone(),
                "y_1e-12": loose["y_last"].clone(),
                "dense_1e-13": tight,
                "dense_1e-12": loose,
            })
        }
    }
}

// ---------------------------------------------------------------- problems

/// The 14 SPD07 cells' problems (`rnext_common::cases` plus
/// brusselator-1d-160), with the explicit-Jacobian twins.
pub fn spd07_problems() -> Vec<Problem> {
    let mut out = Vec::new();
    for c in cases() {
        let full = match c.id {
            "prothero-robinson-forced" => prothero_robinson_problem(-1.0e4, 0.0, 0.0).0,
            "quadratic-4" => quadratic_full(),
            _ => c.full.expect("explicit-Jacobian problem"),
        };
        out.push(Problem {
            id: c.id.to_string(),
            problem: c.problem,
            full,
            exact: c.exact,
            exact_kind: "exact",
            y0: c.y0,
            t_span: c.t_span,
            atol_scale: c.atol_scale,
            max_attempts: 5_000,
        });
    }
    out.push(brusselator_problem(160));
    out
}

pub fn spd07_problem(id: &str) -> Problem {
    spd07_problems().into_iter().find(|p| p.id == id).unwrap()
}

pub fn brusselator_problem(cells: usize) -> Problem {
    let (p, y0) = brusselator(cells).unwrap();
    Problem {
        id: format!("brusselator-1d-{cells}"),
        problem: p.jvp_only_clone().unwrap(),
        full: p,
        exact: None,
        exact_kind: "exact",
        y0,
        t_span: (0.0, 10.0),
        atol_scale: 1.0,
        max_attempts: 5_000,
    }
}

/// `quadratic-4` of `rnext_common` with an explicit Jacobian
/// `A + diag(2 q z)` next to the same right-hand side and JVP.
fn quadratic_full() -> OdeProblem {
    let (model, _) = rnext::quadratic();
    let n = model.a.len();
    let (a, q) = (Arc::new(model.a.clone()), Arc::new(model.q.clone()));
    let (ra, rq) = (a.clone(), q.clone());
    let rhs = Arc::new(move |_t: f64, z: &[f64], out: &mut [f64]| {
        for (i, o) in out.iter_mut().enumerate() {
            let linear = ra[i].iter().zip(z).map(|(x, y)| x * y).sum::<f64>();
            *o = linear + rq[i] * z[i] * z[i];
        }
        Ok(())
    });
    let (ja, jq) = (a.clone(), q.clone());
    let jacobian = Arc::new(move |_t: f64, z: &[f64]| {
        let mut m = DenseMatrix::zeros(n, n);
        for i in 0..n {
            for j in 0..n {
                m[(i, j)] = ja[i][j];
            }
            m[(i, i)] += 2.0 * jq[i] * z[i];
        }
        Ok(m)
    });
    let (va, vq) = (a, q);
    let jvp = Arc::new(move |_t: f64, z: &[f64], v: &[f64], out: &mut [f64]| {
        for (i, o) in out.iter_mut().enumerate() {
            let linear = va[i].iter().zip(v).map(|(x, y)| x * y).sum::<f64>();
            *o = linear + 2.0 * vq[i] * z[i] * v[i];
        }
        Ok(())
    });
    OdeProblem::new(
        "quadratic-4-full",
        n,
        rhs,
        None,
        Some(jacobian),
        Some(jvp),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

/// HIRES or Robertson of `rnext_common` (C2).
pub fn tight_problem(id: &str) -> Problem {
    spd07_problem(id)
}

/// Robertson to `t = 4e10` (ALG03 D2), max attempts 50,000.
pub fn robertson_long() -> Problem {
    let (p, y0) = robertson_problem().unwrap();
    Problem {
        id: "robertson-4e10".into(),
        problem: p.jvp_only_clone().unwrap(),
        full: p,
        exact: None,
        exact_kind: "exact",
        y0,
        t_span: (0.0, 4.0e10),
        atol_scale: 1.0e-4,
        max_attempts: 50_000,
    }
}

/// Row-wise nonzeros of a dense matrix.
fn nonzero_rows(a: &DenseMatrix) -> Vec<Vec<(usize, f64)>> {
    (0..a.nrows())
        .map(|i| {
            (0..a.ncols())
                .filter(|&j| a[(i, j)] != 0.0)
                .map(|j| (j, a[(i, j)]))
                .collect()
        })
        .collect()
}

fn sparse_matvec(rows: &[Vec<(usize, f64)>], v: &[f64], out: &mut [f64]) {
    for (o, row) in out.iter_mut().zip(rows) {
        *o = row.iter().map(|&(j, a)| a * v[j]).sum();
    }
}

/// `y' = A (y - phi(t)) + phi'(t)` with `phi_i(t) = sin(c t + 0.17 i) + off`
/// (pilot `_linear_forced` with `_phi_slow`): explicit Jacobian `A`,
/// `f_t = -A phi' + phi''`.
pub fn linear_forced(
    id: String,
    a: DenseMatrix,
    c: f64,
    off: f64,
    y0: Vec<f64>,
) -> (OdeProblem, OdeProblem) {
    let n = a.nrows();
    let rows = Arc::new(nonzero_rows(&a));
    let phase: Arc<Vec<f64>> = Arc::new((0..n).map(|i| 0.17 * i as f64).collect());
    assert_eq!(y0.len(), n);
    let rhs = {
        let (rows, phase) = (rows.clone(), phase.clone());
        Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
            let d: Vec<f64> = (0..n)
                .map(|i| y[i] - ((c * t + phase[i]).sin() + off))
                .collect();
            sparse_matvec(&rows, &d, out);
            for i in 0..n {
                out[i] += c * (c * t + phase[i]).cos();
            }
            Ok(())
        })
    };
    let jacobian = {
        let a = Arc::new(a);
        Arc::new(move |_t: f64, _y: &[f64]| Ok((*a).clone()))
    };
    let jvp = {
        let rows = rows.clone();
        Arc::new(move |_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
            sparse_matvec(&rows, v, out);
            Ok(())
        })
    };
    let partial_t = {
        let (rows, phase) = (rows.clone(), phase.clone());
        Arc::new(move |t: f64, _y: &[f64], out: &mut [f64]| {
            let dphi: Vec<f64> = (0..n).map(|i| c * (c * t + phase[i]).cos()).collect();
            sparse_matvec(&rows, &dphi, out);
            for i in 0..n {
                out[i] = -out[i] - c * c * (c * t + phase[i]).sin();
            }
            Ok(())
        })
    };
    let full = OdeProblem::new(
        id,
        n,
        rhs,
        None,
        Some(jacobian),
        Some(jvp),
        Some(partial_t),
        false,
        None,
        None,
    )
    .unwrap();
    (full.jvp_only_clone().unwrap(), full)
}

fn phi_slow(n: usize, c: f64, off: f64, t: f64) -> Vec<f64> {
    (0..n)
        .map(|i| (c * t + 0.17 * i as f64).sin() + off)
        .collect()
}

/// The block-diagonal VIG-A02 family (pilot `vigb`): blocks
/// `lam_j [[-2, 2^k], [2^-k, -2]]`, `lam_j` log-spaced in
/// `[lam_lo, lam_hi]`, forcing `phi` with offset 0.5, started on `phi`
/// (exact solution `phi`), `T = 2`, atol = 1e-2 rtol.
pub fn vigb(id: &str, k: i32, nb: usize, lam_lo: f64, lam_hi: f64) -> Problem {
    let n = 2 * nb;
    let mut a = DenseMatrix::zeros(n, n);
    for j in 0..nb {
        let lam =
            (lam_lo.ln() + (lam_hi.ln() - lam_lo.ln()) * j as f64 / (nb.max(2) - 1) as f64).exp();
        let (p, q) = (2 * j, 2 * j + 1);
        a[(p, p)] = -2.0 * lam;
        a[(p, q)] = lam * 2f64.powi(k);
        a[(q, p)] = lam * 2f64.powi(-k);
        a[(q, q)] = -2.0 * lam;
    }
    let y0 = phi_slow(n, 1.0, 0.5, 0.0);
    let (problem, full) = linear_forced(id.into(), a, 1.0, 0.5, y0.clone());
    Problem {
        id: id.into(),
        problem,
        full,
        exact: Some(Box::new(move |t| phi_slow(n, 1.0, 0.5, t))),
        exact_kind: "exact",
        y0,
        t_span: (0.0, 2.0),
        atol_scale: 1.0e-2,
        max_attempts: 5_000,
    }
}

/// Stiff-oscillatory forced blocks (pilot `stosc`, excite = 0): 64 blocks
/// `[[-50, -w_k], [w_k, -50]]`, `w_k = omega (0.1 + 0.9 k / 63)`, forcing
/// `sin(t + 0.17 i)`, started on it (exact solution), `T = 2`, atol = rtol.
pub fn stosc(omega: f64) -> Problem {
    let (sigma, nb) = (50.0, 64usize);
    let n = 2 * nb;
    let mut a = DenseMatrix::zeros(n, n);
    for k in 0..nb {
        let w = omega * (0.1 + 0.9 * k as f64 / (nb - 1) as f64);
        let (p, q) = (2 * k, 2 * k + 1);
        a[(p, p)] = -sigma;
        a[(p, q)] = -w;
        a[(q, p)] = w;
        a[(q, q)] = -sigma;
    }
    let id = format!("stosc-w{omega:e}");
    let y0 = phi_slow(n, 1.0, 0.0, 0.0);
    let (problem, full) = linear_forced(id.clone(), a, 1.0, 0.0, y0.clone());
    Problem {
        id,
        problem,
        full,
        exact: Some(Box::new(move |t| phi_slow(n, 1.0, 0.0, t))),
        exact_kind: "exact",
        y0,
        t_span: (0.0, 2.0),
        atol_scale: 1.0,
        max_attempts: 5_000,
    }
}

/// The E-05 harness's linear congruential generator.
pub struct Lcg(pub u64);

impl Lcg {
    pub fn next_f64(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }

    pub fn uniform(&mut self) -> f64 {
        2.0 * self.next_f64() - 1.0
    }
}

/// Orthogonal Q as the explicit product of Householder reflectors, as in E-05.
fn random_orthogonal(n: usize, rng: &mut Lcg) -> DenseMatrix {
    let mut q = DenseMatrix::identity(n);
    for k in 0..n {
        let mut v: Vec<f64> = (0..n)
            .map(|i| if i < k { 0.0 } else { rng.uniform() })
            .collect();
        let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm == 0.0 {
            continue;
        }
        for x in &mut v {
            *x /= norm;
        }
        let mut qv = vec![0.0; n];
        for (i, value) in qv.iter_mut().enumerate() {
            for (j, vj) in v.iter().enumerate() {
                *value += q[(i, j)] * vj;
            }
        }
        for (i, qvi) in qv.iter().enumerate() {
            for (j, vj) in v.iter().enumerate() {
                q[(i, j)] -= 2.0 * qvi * vj;
            }
        }
    }
    q
}

/// `A = Q (D + s N) Q^T` with `D` log-spaced in `[-dmax, -1]`, as in E-05.
fn build_a(n: usize, s: f64, dmax: f64, rng: &mut Lcg) -> DenseMatrix {
    let q = random_orthogonal(n, rng);
    let mut inner = DenseMatrix::zeros(n, n);
    for i in 0..n {
        inner[(i, i)] = -(10f64).powf((i as f64) / ((n - 1) as f64) * dmax.log10());
        for j in (i + 1)..n {
            inner[(i, j)] = s * rng.uniform();
        }
    }
    q.matmul(&inner).unwrap().matmul(&q.transpose()).unwrap()
}

/// The E-05 unit right-hand side and the operator with scale `s`
/// (E-05 builds s = 0, 1, 10, 100 in that order from one generator).
pub fn e05_operator(s: f64) -> (Vec<f64>, DenseMatrix) {
    let n = 256;
    let mut rng = Lcg(20_260_927);
    let b: Vec<f64> = {
        let v: Vec<f64> = (0..n).map(|_| rng.uniform()).collect();
        let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        v.into_iter().map(|x| x / norm).collect()
    };
    for scale in [0.0, 1.0, 10.0, 100.0] {
        let a = build_a(n, scale, 1.0e4, &mut rng);
        if scale == s {
            return (b, a);
        }
    }
    panic!("E-05 has no operator with s = {s}");
}

/// The E-05 operator forced as in the pilot (`e05`): `phi_i = sin(c t +
/// 0.17 i) + 0.5`, started at `phi(0) + excite b`; exact endpoint
/// `phi(t) + expm(t A) (excite b)` (Pade-13 scaling and squaring);
/// atol = 1e-2 rtol.
pub fn e05(id: &str, s: f64, t_end: f64, c: f64) -> Problem {
    let excite = 0.1;
    let (b, a) = e05_operator(s);
    let n = a.nrows();
    let e: Vec<f64> = b.iter().map(|x| excite * x).collect();
    let y0: Vec<f64> = phi_slow(n, c, 0.5, 0.0)
        .iter()
        .zip(&e)
        .map(|(p, x)| p + x)
        .collect();
    let a_exact = a.clone();
    let (problem, full) = linear_forced(id.into(), a, c, 0.5, y0.clone());
    Problem {
        id: id.into(),
        problem,
        full,
        exact: Some(Box::new(move |t| {
            let m = matrix_exp_pade13(&a_exact.scale(t)).unwrap();
            let decay = m.matvec(&e).unwrap();
            phi_slow(n, c, 0.5, t)
                .iter()
                .zip(&decay)
                .map(|(p, d)| p + d)
                .collect()
        })),
        exact_kind: "exact-expm-pade13",
        y0,
        t_span: (0.0, t_end),
        atol_scale: 1.0e-2,
        max_attempts: 5_000,
    }
}

/// The C3 stress cells' problems.
pub fn stress_problems() -> Vec<Problem> {
    vec![
        vigb("vigb-k10", 10, 32, 1.0, 1.0e4),
        vigb("vigb-k20", 20, 32, 1.0, 1.0e4),
        vigb("vig1b-k20", 20, 1, 1.0e3, 1.0e3),
        e05("e05-s0", 0.0, 2.0, 1.0),
        e05("e05-s1", 1.0, 2.0, 1.0),
    ]
}

// ------------------------------------------------------------ ladders (C4)

/// Diagonal Prothero-Robinson (`inner_forcing_fixed_step_ladder_contracts.rs`,
/// pilot `diag_pr`): `lambda_k = -lambda_max^(k/(n-1))`,
/// `g_k = sin(t + phi_k)`, on [0, 1], exact solution `g`.
pub fn diag_pr(n: usize, lambda_max: f64) -> Problem {
    let lambda: Arc<Vec<f64>> = Arc::new(
        (0..n)
            .map(|k| -(lambda_max.ln() * k as f64 / (n - 1) as f64).exp())
            .collect(),
    );
    let phase: Arc<Vec<f64>> = Arc::new(
        (0..n)
            .map(|k| 2.0 * std::f64::consts::PI * ((k as f64 * 0.618_033_988_749_895) % 1.0))
            .collect(),
    );
    let rhs = {
        let (lambda, phase) = (lambda.clone(), phase.clone());
        Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
            for i in 0..n {
                out[i] = lambda[i] * (y[i] - (t + phase[i]).sin()) + (t + phase[i]).cos();
            }
            Ok(())
        })
    };
    let jacobian = {
        let lambda = lambda.clone();
        Arc::new(move |_t: f64, _y: &[f64]| -> CoreResult<DenseMatrix> {
            let mut matrix = DenseMatrix::zeros(n, n);
            for i in 0..n {
                matrix[(i, i)] = lambda[i];
            }
            Ok(matrix)
        })
    };
    let jvp = {
        let lambda = lambda.clone();
        Arc::new(move |_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
            for i in 0..n {
                out[i] = lambda[i] * v[i];
            }
            Ok(())
        })
    };
    let partial_t = {
        let (lambda, phase) = (lambda.clone(), phase.clone());
        Arc::new(move |t: f64, _y: &[f64], out: &mut [f64]| {
            for i in 0..n {
                out[i] = -lambda[i] * (t + phase[i]).cos() - (t + phase[i]).sin();
            }
            Ok(())
        })
    };
    let exact_phase = phase.clone();
    let exact = move |t: f64| -> Vec<f64> { exact_phase.iter().map(|p| (t + p).sin()).collect() };
    let y0 = exact(0.0);
    let full = OdeProblem::new(
        format!("diagpr{n}"),
        n,
        rhs,
        None,
        Some(jacobian),
        Some(jvp),
        Some(partial_t),
        false,
        None,
        None,
    )
    .unwrap();
    Problem {
        id: format!("diagpr{n}"),
        problem: full.jvp_only_clone().unwrap(),
        full,
        exact: Some(Box::new(exact)),
        exact_kind: "exact",
        y0,
        t_span: (0.0, 1.0),
        atol_scale: 1.0e-2,
        max_attempts: 5_000,
    }
}

/// `semilinear_advection_diffusion_problem(n, d, a, r, nl, 0)` on [0, 1].
pub fn semilin(id: &str, n: usize, d: f64, a: f64, r: f64, nl: f64) -> Problem {
    let (full, y0) = semilinear_advection_diffusion_problem(n, d, a, r, nl, 0.0).unwrap();
    let exact_problem = full.clone();
    Problem {
        id: id.into(),
        problem: full.jvp_only_clone().unwrap(),
        full,
        exact: Some(Box::new(move |t| exact_problem.exact(t).unwrap())),
        exact_kind: "exact",
        y0,
        t_span: (0.0, 1.0),
        atol_scale: 1.0e-2,
        max_attempts: 5_000,
    }
}

/// A ladder: problem, stage-target rtols and rungs `k` (h = span / 2^k).
pub struct Ladder {
    pub id: &'static str,
    pub problem: Problem,
    pub rtols: Vec<f64>,
    pub rungs: Vec<u32>,
}

pub fn ladders() -> Vec<Ladder> {
    vec![
        Ladder {
            id: "diagpr128",
            problem: diag_pr(128, 1.0e6),
            rtols: vec![1.0e-6],
            rungs: vec![3, 4, 5],
        },
        Ladder {
            id: "semilin64",
            problem: semilin("semilin64", 64, 0.05, 0.5, -1.0, 0.5),
            rtols: vec![1.0e-4, 1.0e-6],
            rungs: vec![3, 4, 5, 6, 7, 8],
        },
        Ladder {
            id: "semilin128",
            problem: semilin("semilin128", 128, 0.02, 3.0, -1.0, 10.0),
            rtols: vec![1.0e-6],
            rungs: vec![3, 4, 5],
        },
    ]
}

/// Relative max-norm `max |y - ref| / max |ref|` (the ladder metric).
pub fn relative_max_norm(y: &[f64], reference: &[f64]) -> f64 {
    let scale = reference.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
    y.iter()
        .zip(reference)
        .fold(0.0_f64, |m, (a, b)| m.max((a - b).abs()))
        / scale.max(f64::MIN_POSITIVE)
}

/// One U-form RODAS5P attempt with dense LU stage solves: the arithmetic
/// of `Rodas5pMfFastWorkspace::attempt` (stage right-hand sides, update,
/// embedded error norm) with `(I - h gamma J) U_i = rhs_i` solved by LU.
pub fn lu_u_form_step(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    atol: f64,
    rtol: f64,
) -> CoreResult<(Vec<f64>, f64)> {
    let coeffs = rodas5p_coefficients()?;
    let (n, s, gamma) = (y.len(), coeffs.stages(), coeffs.gamma);
    let mut counters = WorkCounters::default();
    let j = problem.dense_jacobian(t, y, &mut counters)?;
    let hg = h * gamma;
    let mut w = DenseMatrix::zeros(n, n);
    for r in 0..n {
        for c in 0..n {
            w[(r, c)] = -hg * j[(r, c)];
        }
        w[(r, r)] += 1.0;
    }
    let lu = LuFactorization::new(&w)?;
    let f0 = problem.eval_rhs(t, y, &mut counters)?;
    let ft = if problem.autonomous {
        vec![0.0; n]
    } else {
        problem.eval_partial_t(t, y, &mut counters)?
    };
    let mut u: Vec<Vec<f64>> = Vec::with_capacity(s);
    for i in 0..s {
        let mut rhs = if i == 0 {
            f0.clone()
        } else {
            let mut state = y.to_vec();
            for (jj, uj) in u.iter().enumerate() {
                let aij = coeffs.a[(i, jj)];
                if aij != 0.0 {
                    for (x, v) in state.iter_mut().zip(uj) {
                        *x += aij * v;
                    }
                }
            }
            problem.eval_rhs(t + coeffs.c[i] * h, &state, &mut counters)?
        };
        for x in rhs.iter_mut() {
            *x *= hg;
        }
        for (jj, uj) in u.iter().enumerate() {
            let cij = coeffs.c_matrix[(i, jj)];
            if cij != 0.0 {
                let weight = gamma * cij;
                for (x, v) in rhs.iter_mut().zip(uj) {
                    *x += weight * v;
                }
            }
        }
        if !problem.autonomous {
            let g = hg * h * coeffs.gamma_rows[i];
            for (x, v) in rhs.iter_mut().zip(&ft) {
                *x += g * v;
            }
        }
        u.push(lu.solve(&rhs)?);
    }
    let mut y_new = y.to_vec();
    for (jj, uj) in u.iter().enumerate() {
        let bj = coeffs.b_code[jj];
        if bj != 0.0 {
            for (x, v) in y_new.iter_mut().zip(uj) {
                *x += bj * v;
            }
        }
    }
    let mut sum = 0.0;
    for ((e, a), b) in u[s - 1].iter().zip(y).zip(&y_new) {
        let z = e / (atol + rtol * a.abs().max(b.abs()));
        sum += z * z;
    }
    if !y_new.iter().all(|v| v.is_finite()) {
        return Err(CoreError::NonFinite(
            "LU U-form step produced NaN/Inf".into(),
        ));
    }
    Ok((y_new, (sum / n as f64).sqrt()))
}

/// The time of fixed step `step` of `2^k` steps on the span.
fn rung_time(p: &Problem, k: u32, step: usize) -> f64 {
    let steps = 1usize << k;
    if step == steps {
        p.t_span.1
    } else {
        p.t_span.0 + step as f64 * (p.span() / steps as f64)
    }
}

fn ladder_json(
    p: &Problem,
    k: u32,
    y: &[f64],
    errors: &[f64],
    failed: Option<(usize, String)>,
    counters: Option<&WorkCounters>,
) -> Value {
    let exact = (p.exact.as_ref().unwrap())(p.t_span.1);
    let mut v = json!({
        "ok": failed.is_none(),
        "steps": 1usize << k,
        "y_last": hexes(y),
        "max_step_error_estimate": errors.iter().fold(0.0_f64, |m, e| m.max(*e)),
    });
    if let Some((step, error)) = failed {
        v["failed_step"] = json!(step);
        v["error"] = json!(error);
    } else {
        v["rel_max_norm_error"] = json!(relative_max_norm(y, &exact));
    }
    if let Some(c) = counters {
        v["counters"] = serde_json::to_value(c).unwrap();
    }
    v
}

/// The fixed-step direct twin of a rung.
pub fn ladder_lu(p: &Problem, rtol: f64, k: u32) -> Value {
    let atol = p.atol_scale * rtol;
    let mut y = p.y0.clone();
    let mut errors = Vec::new();
    for step in 0..(1usize << k) {
        let (t, t_new) = (rung_time(p, k, step), rung_time(p, k, step + 1));
        match lu_u_form_step(&p.full, t, &y, t_new - t, atol, rtol) {
            Ok((y_new, err)) => {
                y = y_new;
                errors.push(err);
            }
            Err(e) => return ladder_json(p, k, &y, &errors, Some((step, e.to_string())), None),
        }
    }
    ladder_json(p, k, &y, &errors, None, None)
}

/// A rung with the matrix-free U-form workspace (`set_gmres_into(true)`,
/// the SPD07 GMRES configuration with budget `budget`); `setup` prepares the
/// workspace and `after_step(work, err)` runs after every step (every step
/// is accepted).
pub fn ladder_mf(
    p: &Problem,
    rtol: f64,
    k: u32,
    budget: usize,
    setup: &mut dyn FnMut(&mut Rodas5pMfFastWorkspace),
    after_step: &mut dyn FnMut(&mut Rodas5pMfFastWorkspace, f64),
) -> Value {
    ladder_mf_finish(p, rtol, k, budget, setup, after_step, &mut |_, _| {})
}

/// [`ladder_mf`] with `finish(work, record)` called on the rung's record
/// (after the last step or the failed one).
pub fn ladder_mf_finish(
    p: &Problem,
    rtol: f64,
    k: u32,
    budget: usize,
    setup: &mut dyn FnMut(&mut Rodas5pMfFastWorkspace),
    after_step: &mut dyn FnMut(&mut Rodas5pMfFastWorkspace, f64),
    finish: &mut dyn FnMut(&Rodas5pMfFastWorkspace, &mut Value),
) -> Value {
    let atol = p.atol_scale * rtol;
    let mut work = Rodas5pMfFastWorkspace::new(&p.problem, &gmres_config(budget)).unwrap();
    work.set_gmres_into(true);
    setup(&mut work);
    let mut counters = WorkCounters::default();
    let mut y = p.y0.clone();
    let mut errors = Vec::new();
    for step in 0..(1usize << k) {
        let (t, t_new) = (rung_time(p, k, step), rung_time(p, k, step + 1));
        match work.attempt(
            &p.problem,
            t,
            &y,
            t_new - t,
            true,
            None,
            atol,
            rtol,
            &mut counters,
        ) {
            Ok(err) => {
                y.copy_from_slice(work.y_new());
                errors.push(err);
                after_step(&mut work, err);
            }
            Err(e) => {
                let mut record = ladder_json(
                    p,
                    k,
                    &y,
                    &errors,
                    Some((step, e.to_string())),
                    Some(&counters),
                );
                finish(&work, &mut record);
                return record;
            }
        }
    }
    let mut record = ladder_json(p, k, &y, &errors, None, Some(&counters));
    finish(&work, &mut record);
    record
}

/// The C5 / frontier tolerances: 1e-3 to 1e-10 in half decades.
pub fn half_decades() -> Vec<f64> {
    (0..15)
        .map(|k| {
            if k % 2 == 0 {
                format!("1e-{}", 3 + k / 2).parse().unwrap()
            } else {
                10f64.powf(-3.0 - 0.5 * k as f64)
            }
        })
        .collect()
}

/// Brusselator-1d-`cells` whose JVP is a forward difference
/// `(f(t, y + s v) - f(t, y)) / s`, `s = sqrt(eps) (1 + ||y||_2) / ||v||_2`
/// (the in-repo `semilinear_f033_ablation.rs` formula, uncached base RHS;
/// the RHS calls inside the JVP are not counted). Reported variant only.
pub fn brusselator_fd_problem(cells: usize) -> Problem {
    let base = brusselator_problem(cells);
    let f = base.full.clone();
    let n = base.y0.len();
    let rhs = {
        let f = f.clone();
        Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
            let mut c = WorkCounters::default();
            f.eval_rhs_into(t, y, out, &mut c)
        })
    };
    let jvp = {
        let f = f.clone();
        Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
            let mut c = WorkCounters::default();
            let vn = rodas5p_core::safe_l2(v);
            if vn == 0.0 {
                out.fill(0.0);
                return Ok(());
            }
            let s = f64::EPSILON.sqrt() * (1.0 + rodas5p_core::safe_l2(y)) / vn;
            let shifted: Vec<f64> = y.iter().zip(v).map(|(a, b)| a + s * b).collect();
            let f0 = f.eval_rhs(t, y, &mut c)?;
            f.eval_rhs_into(t, &shifted, out, &mut c)?;
            for (o, a) in out.iter_mut().zip(&f0) {
                *o = (*o - a) / s;
            }
            Ok(())
        })
    };
    let problem = OdeProblem::new(
        format!("brusselator-1d-{cells}-fd"),
        n,
        rhs,
        None,
        None,
        Some(jvp),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    Problem {
        id: format!("brusselator-1d-{cells}-fd"),
        problem,
        full: f,
        exact: None,
        exact_kind: "exact",
        y0: base.y0,
        t_span: base.t_span,
        atol_scale: base.atol_scale,
        max_attempts: base.max_attempts,
    }
}
