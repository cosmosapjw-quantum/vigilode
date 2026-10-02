//! Shared fixtures of the thread-transfer research tests
//! (`docs/reviews/thread_transfer_20261002/NEXT_DEVELOPMENT_DAG.json`).
#![allow(dead_code)] // each test target uses a subset

use rodas5p_core::{CoreResult, directed::Interval, rodas5p_coefficients};
use rodas5p_integrators::{
    InverseWitness, Q2CertificateSource, QuadraticModel, QuadraticStageProblem, StageCertificate,
    StageTarget,
};

pub const R4_DIMENSIONS: [usize; 5] = [1, 2, 4, 8, 16];
pub const ATOL: f64 = 1.0e-8;
pub const RTOL: f64 = 1.0e-6;
pub const INITIAL_RADIUS: f64 = 1.0e-3;

/// The diagonal quadratic fixture of `r4_studies::homotopy_cost_study`,
/// unchanged.
pub struct R4Fixture {
    pub n: usize,
    pub target: StageTarget,
    pub problem: QuadraticStageProblem,
    pub candidate: Vec<Vec<f64>>,
    pub y_hat: Vec<f64>,
    pub e_hat: Vec<f64>,
    pub witness: InverseWitness,
}

pub fn r4_fixture(n: usize) -> CoreResult<R4Fixture> {
    let coeffs = rodas5p_coefficients()?;
    let target = StageTarget::sequential(coeffs)?;
    let a = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| if i == j { -1.0 - i as f64 } else { 0.0 })
                .collect()
        })
        .collect::<Vec<Vec<f64>>>();
    let q = (0..n)
        .map(|i| -0.05 * (1 + i % 3) as f64)
        .collect::<Vec<_>>();
    let model = QuadraticModel::new(format!("diagonal-quadratic-{n}"), a, q)?;
    let y = (0..n).map(|i| 1.0 + 0.1 * i as f64).collect::<Vec<_>>();
    let h = 0.05;
    let problem = model.stage_problem(0.0, &y, h)?;
    let f = (0..n)
        .map(|i| {
            problem.jacobian[i][i] * y[i] - 2.0 * problem.q[i] * y[i] * y[i]
                + problem.q[i] * y[i] * y[i]
        })
        .collect::<Vec<_>>();
    let candidate = vec![f.iter().map(|x| h * x).collect::<Vec<_>>(); 8];
    let y_hat = y.iter().zip(&f).map(|(y, f)| y + h * f).collect::<Vec<_>>();
    let e_hat = vec![0.0; n];
    let witness = InverseWitness::diagonal(&problem, target.gamma)?;
    Ok(R4Fixture {
        n,
        target,
        problem,
        candidate,
        y_hat,
        e_hat,
        witness,
    })
}

fn interval_dot(weights: &[Interval], values: &[Interval]) -> CoreResult<Interval> {
    let mut total = Interval::point(0.0)?;
    for (w, v) in weights.iter().zip(values) {
        total = total.add(w.mul(*v)?)?;
    }
    Ok(total)
}

/// An interval enclosure of the exact root `K*` of the declared stage
/// equations for a diagonal `J`:
/// `(1 - h gamma J_aa) K_ia = h (J y - q y^2)_a + h J_aa sum_{j<i} L*_ij K_ja
/// + h q_a (sum_{j<i} alpha_ij K_ja)^2`, explicit in `K_i` given `K_j`,
/// `j < i`, evaluated in directed interval arithmetic. It is the reference a
/// stage bound must enclose; it is computed without any certificate.
pub fn interval_root(
    target: &StageTarget,
    problem: &QuadraticStageProblem,
) -> CoreResult<Vec<Vec<Interval>>> {
    let n = problem.dimension();
    for a in 0..n {
        for b in 0..n {
            assert!(
                a == b || problem.jacobian[a][b] == 0.0,
                "reference root needs a diagonal J"
            );
        }
    }
    let h = Interval::point(problem.h)?;
    let gamma = Interval::point(target.gamma)?;
    let mut root: Vec<Vec<Interval>> = Vec::with_capacity(target.stages());
    for i in 0..target.stages() {
        let alpha = target.alpha_rows[i]
            .iter()
            .map(|value| Interval::point(*value))
            .collect::<CoreResult<Vec<_>>>()?;
        let mut row = Vec::with_capacity(n);
        for a in 0..n {
            let column = root.iter().map(|stage| stage[a]).collect::<Vec<_>>();
            let delta = interval_dot(&alpha, &column)?;
            let coupled = interval_dot(&target.coupling_rows[i], &column)?;
            let j = Interval::point(problem.jacobian[a][a])?;
            let q = Interval::point(problem.q[a])?;
            let y = Interval::point(problem.y[a])?;
            let base = j.mul(y)?.sub(q.mul(y)?.mul(y)?)?;
            let right = h
                .mul(base)?
                .add(h.mul(j)?.mul(coupled)?)?
                .add(h.mul(q)?.mul(delta.mul(delta)?)?)?;
            let w = Interval::point(1.0)?.sub(h.mul(gamma)?.mul(j)?)?;
            row.push(right.div(w)?);
        }
        root.push(row);
    }
    Ok(root)
}

/// `|K_hat - K*| <= E` for every point of the root enclosure.
pub fn encloses_root(
    certificate: &StageCertificate,
    candidate: &[Vec<f64>],
    root: &[Vec<Interval>],
) -> CoreResult<bool> {
    for (i, stage) in root.iter().enumerate() {
        for (a, value) in stage.iter().enumerate() {
            let distance = Interval::point(candidate[i][a])?.sub(*value)?.mag();
            if distance > certificate.stage_bound[i][a] {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// Writes `value` to the path in `variable`, if set; a relative path is
/// taken from the workspace root (tests run in the crate directory).
pub fn write_output(variable: &str, value: &serde_json::Value) {
    if let Ok(path) = std::env::var(variable) {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        let text = serde_json::to_string_pretty(value).expect("serialize results") + "\n";
        std::fs::write(&path, text)
            .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
        println!("wrote {}", path.display());
    }
}
