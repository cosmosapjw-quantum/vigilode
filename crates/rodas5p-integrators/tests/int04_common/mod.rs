//! Charts, families and references of research node
//! `research/int04_stage_chart_20261003` (integrated DAG node INT-04).
#![allow(dead_code)]

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, LuFactorization};
use rodas5p_integrators::{InverseWitness, QuadraticStageProblem, StageChart, StageTarget};

/// `K_0 = Z_0`, `K_i = Z_i + beta Z_(i-1)^2` componentwise.
pub struct Triangular {
    pub s: usize,
    pub n: usize,
    pub beta: f64,
}

impl StageChart for Triangular {
    fn name(&self) -> &str {
        "polynomial-triangular"
    }
    fn contains(&self, z: &[f64]) -> bool {
        z.len() == self.s * self.n
    }
    fn forward(&self, z: &[f64]) -> CoreResult<Vec<f64>> {
        let n = self.n;
        Ok((0..z.len())
            .map(|p| {
                if p < n {
                    z[p]
                } else {
                    z[p] + self.beta * z[p - n] * z[p - n]
                }
            })
            .collect())
    }
    fn inverse(&self, k: &[f64]) -> CoreResult<Vec<f64>> {
        let n = self.n;
        let mut z = vec![0.0; k.len()];
        for p in 0..k.len() {
            z[p] = if p < n {
                k[p]
            } else {
                k[p] - self.beta * z[p - n] * z[p - n]
            };
        }
        Ok(z)
    }
    fn jvp(&self, z: &[f64], dz: &[f64]) -> CoreResult<Vec<f64>> {
        let n = self.n;
        Ok((0..z.len())
            .map(|p| {
                if p < n {
                    dz[p]
                } else {
                    dz[p] + 2.0 * self.beta * z[p - n] * dz[p - n]
                }
            })
            .collect())
    }
}

/// `K = sigma Z` with `sigma_(i,a) = 2^-(i mod 4) (1 + a/n)`.
pub struct Scaling {
    pub s: usize,
    pub n: usize,
}

impl Scaling {
    fn sigma(&self, p: usize) -> f64 {
        let (i, a) = (p / self.n, p % self.n);
        (0.5_f64).powi((i % 4) as i32) * (1.0 + a as f64 / self.n as f64)
    }
}

impl StageChart for Scaling {
    fn name(&self) -> &str {
        "scaling"
    }
    fn contains(&self, z: &[f64]) -> bool {
        z.len() == self.s * self.n
    }
    fn forward(&self, z: &[f64]) -> CoreResult<Vec<f64>> {
        Ok(z.iter()
            .enumerate()
            .map(|(p, v)| self.sigma(p) * v)
            .collect())
    }
    fn inverse(&self, k: &[f64]) -> CoreResult<Vec<f64>> {
        Ok(k.iter()
            .enumerate()
            .map(|(p, v)| v / self.sigma(p))
            .collect())
    }
    fn jvp(&self, _z: &[f64], dz: &[f64]) -> CoreResult<Vec<f64>> {
        self.forward(dz)
    }
}

/// `K = Z^3` componentwise: singular Jacobian at 0.
pub struct Cube;

impl StageChart for Cube {
    fn name(&self) -> &str {
        "cube"
    }
    fn contains(&self, _z: &[f64]) -> bool {
        true
    }
    fn forward(&self, z: &[f64]) -> CoreResult<Vec<f64>> {
        Ok(z.iter().map(|v| v * v * v).collect())
    }
    fn inverse(&self, k: &[f64]) -> CoreResult<Vec<f64>> {
        Ok(k.iter().map(|v| v.cbrt()).collect())
    }
    fn jvp(&self, z: &[f64], dz: &[f64]) -> CoreResult<Vec<f64>> {
        Ok(z.iter().zip(dz).map(|(v, d)| 3.0 * v * v * d).collect())
    }
}

/// `K = Z^2` on `Z > 0`.
pub struct PositiveSquare;

impl StageChart for PositiveSquare {
    fn name(&self) -> &str {
        "positive-square"
    }
    fn contains(&self, z: &[f64]) -> bool {
        z.iter().all(|v| *v > 0.0)
    }
    fn forward(&self, z: &[f64]) -> CoreResult<Vec<f64>> {
        Ok(z.iter().map(|v| v * v).collect())
    }
    fn inverse(&self, k: &[f64]) -> CoreResult<Vec<f64>> {
        if k.iter().any(|v| *v <= 0.0) {
            return Err(CoreError::InvalidInput("outside the chart's image".into()));
        }
        Ok(k.iter().map(|v| v.sqrt()).collect())
    }
    fn jvp(&self, z: &[f64], dz: &[f64]) -> CoreResult<Vec<f64>> {
        Ok(z.iter().zip(dz).map(|(v, d)| 2.0 * v * d).collect())
    }
}

/// `(label, problem)`: R4 diagonal n in {1, 4, 16} and the 2x2 non-diagonal,
/// each at h in {0.05, 0.5}.
pub fn families() -> Vec<(String, QuadraticStageProblem)> {
    let mut out = Vec::new();
    for h in [0.05, 0.5] {
        for n in [1usize, 4, 16] {
            let y: Vec<f64> = (0..n).map(|i| 1.0 + 0.1 * i as f64).collect();
            let q: Vec<f64> = (0..n).map(|i| -0.05 * (1 + i % 3) as f64).collect();
            let jacobian = (0..n)
                .map(|a| {
                    (0..n)
                        .map(|b| {
                            if a == b {
                                -1.0 - a as f64 + 2.0 * q[a] * y[a]
                            } else {
                                0.0
                            }
                        })
                        .collect()
                })
                .collect();
            out.push((
                format!("r4-diagonal-{n}-h{h}"),
                QuadraticStageProblem { jacobian, y, h, q },
            ));
        }
        out.push((
            format!("nondiagonal-2-h{h}"),
            QuadraticStageProblem {
                jacobian: vec![vec![-2.0, 1.0], vec![0.5, -3.0]],
                y: vec![1.0, -0.5],
                h,
                q: vec![-0.1, 0.05],
            },
        ));
    }
    out
}

/// The witness the certificate uses: diagonal or the exact small one.
pub fn witness(problem: &QuadraticStageProblem, gamma: f64) -> InverseWitness {
    InverseWitness::diagonal(problem, gamma)
        .unwrap_or_else(|_| InverseWitness::small(problem, gamma).unwrap())
}

/// The direct sequential root of the binary64 residual, with `W` built
/// from `gamma_w` (the target's gamma for the reference; a perturbed one for
/// an approximate-W predictor). Flattened stage-major.
pub fn sequential_root(
    target: &StageTarget,
    problem: &QuadraticStageProblem,
    gamma_w: f64,
) -> Vec<f64> {
    let (s, n, h) = (target.stages(), problem.dimension(), problem.h);
    let mut w = DenseMatrix::zeros(n, n);
    for a in 0..n {
        for b in 0..n {
            w[(a, b)] = if a == b { 1.0 } else { 0.0 } - h * gamma_w * problem.jacobian[a][b];
        }
    }
    let lu = LuFactorization::new(&w).unwrap();
    let base: Vec<f64> = (0..n)
        .map(|a| {
            let jy: f64 = (0..n).map(|b| problem.jacobian[a][b] * problem.y[b]).sum();
            h * (jy - problem.q[a] * problem.y[a] * problem.y[a])
        })
        .collect();
    let mut k = vec![0.0; s * n];
    for i in 0..s {
        let mut coupled = vec![0.0; n];
        let mut delta = vec![0.0; n];
        for j in 0..i {
            let l = 0.5 * (target.coupling_rows[i][j].lo + target.coupling_rows[i][j].hi);
            for a in 0..n {
                coupled[a] += l * k[j * n + a];
                delta[a] += target.alpha_rows[i][j] * k[j * n + a];
            }
        }
        let rhs: Vec<f64> = (0..n)
            .map(|a| {
                let jc: f64 = (0..n).map(|b| problem.jacobian[a][b] * coupled[b]).sum();
                base[a] + h * jc + h * problem.q[a] * delta[a] * delta[a]
            })
            .collect();
        let stage = lu.solve(&rhs).unwrap();
        k[i * n..(i + 1) * n].copy_from_slice(&stage);
    }
    k
}

/// `(y_hat, e_hat)` of stages under the target's `b` and `btilde`.
pub fn projections(
    target: &StageTarget,
    problem: &QuadraticStageProblem,
    k: &[f64],
) -> (Vec<f64>, Vec<f64>) {
    let n = problem.dimension();
    let mut y_hat = problem.y.clone();
    let mut e_hat = vec![0.0; n];
    for i in 0..target.stages() {
        for a in 0..n {
            y_hat[a] += target.b[i] * k[i * n + a];
            e_hat[a] += target.btilde[i] * k[i * n + a];
        }
    }
    (y_hat, e_hat)
}

pub fn stages(k: &[f64], n: usize) -> Vec<Vec<f64>> {
    k.chunks(n).map(<[f64]>::to_vec).collect()
}
