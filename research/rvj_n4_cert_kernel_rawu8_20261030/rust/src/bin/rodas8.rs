#![allow(clippy::needless_range_loop)] // Original coefficient/tensor index mapping is explicit.
//! Rust1.99.1 independent source-specific RODAS5P eight-stage ORIGINAL RAW-U
//! dense small-system research reference, not the shipped native solver.
//! Diagnostic force-accept is prohibited as a production/adaptive claim.
#[path = "../rodas8_coefficients.rs"]
mod generated_coefficients;
use generated_coefficients::{A, B, C, GAMMA};
use rvj_n4_source_kernel::I;

struct Source {
    n: usize,
    kap: Vec<f64>,
    theta: Vec<f64>,
    phi: [Vec<f64>; 2],
    w: Vec<f64>,
    chi: [f64; 2],
}
impl Source {
    fn new(n: usize) -> Self {
        let xi = (0..n)
            .map(|j| 2. * (j as f64) / (n as f64 - 1.) - 1.)
            .collect::<Vec<_>>();
        Self {
            n,
            kap: (0..n)
                .map(|j| if j % 2 == 0 { 0.25 } else { 1000. })
                .collect(),
            theta: xi.iter().map(|x| 0.5 * x).collect(),
            phi: [
                vec![1.; n],
                (0..n).map(|j| if j % 2 == 0 { 1. } else { -1. }).collect(),
            ],
            w: vec![1. / n as f64; n],
            chi: [0.05, 0.02],
        }
    }
    fn initial(&self) -> Vec<f64> {
        let n = self.n;
        (0..n)
            .flat_map(|j| [0.02 + self.theta[j] / 250., ((j % 3) as f64 - 1.) / 200.])
            .collect()
    }
    fn moments(&self, y: &[f64]) -> [(f64, f64); 2] {
        let mut m = [(0., 0.); 2];
        for a in 0..2 {
            for j in 0..self.n {
                let p = self.w[j] * self.phi[a][j];
                m[a].0 += p * y[2 * j];
                m[a].1 += p * y[2 * j + 1];
            }
        }
        m
    }
    fn rhs(&self, y: &[f64]) -> Vec<f64> {
        let m = self.moments(y);
        let mut f = vec![0.; 2 * self.n];
        for j in 0..self.n {
            f[2 * j] = -self.kap[j] * y[2 * j] - self.theta[j] * y[2 * j + 1];
            f[2 * j + 1] = self.theta[j] * y[2 * j] - self.kap[j] * y[2 * j + 1];
            for a in 0..2 {
                let (x, z) = m[a];
                let p = self.chi[a] * self.phi[a][j] * (x * x + z * z);
                f[2 * j] -= p * z;
                f[2 * j + 1] += p * x;
            }
        }
        f
    }
    fn jac(&self, y: &[f64]) -> Vec<Vec<f64>> {
        let n = self.n;
        let mut j = vec![vec![0.; 2 * n]; 2 * n];
        let m = self.moments(y);
        for r in 0..n {
            j[2 * r][2 * r] = -self.kap[r];
            j[2 * r][2 * r + 1] = -self.theta[r];
            j[2 * r + 1][2 * r] = self.theta[r];
            j[2 * r + 1][2 * r + 1] = -self.kap[r];
            for a in 0..2 {
                let (x, z) = m[a];
                for k in 0..n {
                    let c = self.chi[a] * self.phi[a][r] * self.w[k] * self.phi[a][k];
                    j[2 * r][2 * k] += -2. * c * x * z;
                    j[2 * r][2 * k + 1] += c * (-x * x - 3. * z * z);
                    j[2 * r + 1][2 * k] += c * (3. * x * x + z * z);
                    j[2 * r + 1][2 * k + 1] += 2. * c * x * z;
                }
            }
        }
        j
    }
    fn energy(&self, y: &[f64]) -> f64 {
        (0..self.n)
            .map(|j| self.w[j] * (y[2 * j] * y[2 * j] + y[2 * j + 1] * y[2 * j + 1]))
            .sum()
    }
    fn interval_rhs(&self, y: &[I]) -> Vec<I> {
        let p = |x: f64| I::point(x).unwrap();
        let mut m = [(I::zero(), I::zero()); 2];
        let mut out = vec![I::zero(); 2 * self.n];
        for a in 0..2 {
            for j in 0..self.n {
                let c = p(self.w[j]).mul(p(self.phi[a][j])).unwrap();
                m[a].0 = m[a].0.add(c.mul(y[2 * j]).unwrap()).unwrap();
                m[a].1 = m[a].1.add(c.mul(y[2 * j + 1]).unwrap()).unwrap();
            }
        }
        for j in 0..self.n {
            out[2 * j] = p(-self.kap[j])
                .mul(y[2 * j])
                .unwrap()
                .sub(p(self.theta[j]).mul(y[2 * j + 1]).unwrap())
                .unwrap();
            out[2 * j + 1] = p(self.theta[j])
                .mul(y[2 * j])
                .unwrap()
                .sub(p(self.kap[j]).mul(y[2 * j + 1]).unwrap())
                .unwrap();
            for a in 0..2 {
                let x = m[a].0;
                let z = m[a].1;
                let mag = x.mul(x).unwrap().add(z.mul(z).unwrap()).unwrap();
                let c = p(self.chi[a])
                    .mul(p(self.phi[a][j]))
                    .unwrap()
                    .mul(mag)
                    .unwrap();
                out[2 * j] = out[2 * j].sub(c.mul(z).unwrap()).unwrap();
                out[2 * j + 1] = out[2 * j + 1].add(c.mul(x).unwrap()).unwrap();
            }
        }
        out
    }
    fn interval_jac(&self, y: &[I]) -> Vec<Vec<I>> {
        let p = |x: f64| I::point(x).unwrap();
        let n = self.n;
        let mut m = [(I::zero(), I::zero()); 2];
        for a in 0..2 {
            for j in 0..n {
                let v = p(self.w[j]).mul(p(self.phi[a][j])).unwrap();
                m[a].0 = m[a].0.add(v.mul(y[2 * j]).unwrap()).unwrap();
                m[a].1 = m[a].1.add(v.mul(y[2 * j + 1]).unwrap()).unwrap();
            }
        }
        let mut jac = vec![vec![I::zero(); 2 * n]; 2 * n];
        for j in 0..n {
            jac[2 * j][2 * j] = p(-self.kap[j]);
            jac[2 * j][2 * j + 1] = p(-self.theta[j]);
            jac[2 * j + 1][2 * j] = p(self.theta[j]);
            jac[2 * j + 1][2 * j + 1] = p(-self.kap[j]);
            for a in 0..2 {
                let x = m[a].0;
                let z = m[a].1;
                let xx = x.mul(x).unwrap();
                let zz = z.mul(z).unwrap();
                let xz = x.mul(z).unwrap();
                for k in 0..n {
                    let c = p(self.chi[a])
                        .mul(p(self.phi[a][j]))
                        .unwrap()
                        .mul(p(self.w[k]))
                        .unwrap()
                        .mul(p(self.phi[a][k]))
                        .unwrap();
                    let rr = p(-2.).mul(xz).unwrap();
                    let ri = xx.add(p(3.).mul(zz).unwrap()).unwrap().neg();
                    let ir = p(3.).mul(xx).unwrap().add(zz).unwrap();
                    let ii = p(2.).mul(xz).unwrap();
                    jac[2 * j][2 * k] = jac[2 * j][2 * k].add(c.mul(rr).unwrap()).unwrap();
                    jac[2 * j][2 * k + 1] = jac[2 * j][2 * k + 1].add(c.mul(ri).unwrap()).unwrap();
                    jac[2 * j + 1][2 * k] = jac[2 * j + 1][2 * k].add(c.mul(ir).unwrap()).unwrap();
                    jac[2 * j + 1][2 * k + 1] =
                        jac[2 * j + 1][2 * k + 1].add(c.mul(ii).unwrap()).unwrap();
                }
            }
        }
        jac
    }
}
fn solve(mut mat: Vec<Vec<f64>>, mut b: Vec<f64>) -> Vec<f64> {
    let n = b.len();
    for k in 0..n {
        let mut pivot = k;
        for j in k + 1..n {
            if mat[j][k].abs() > mat[pivot][k].abs() {
                pivot = j;
            }
        }
        if pivot != k {
            mat.swap(k, pivot);
            b.swap(k, pivot);
        }
        let p = mat[k][k];
        assert!(p.is_finite() && p.abs() > 1e-40);
        for j in k + 1..n {
            let c = mat[j][k] / p;
            mat[j][k] = 0.;
            for l in k + 1..n {
                mat[j][l] -= c * mat[k][l];
            }
            b[j] -= c * b[k];
        }
    }
    let mut x = vec![0.; n];
    for k in (0..n).rev() {
        let mut z = b[k];
        for j in k + 1..n {
            z -= mat[k][j] * x[j];
        }
        x[k] = z / mat[k][k];
    }
    x
}

fn audit_original_raw_u_intervals(s: &Source, y: &[f64], h: f64, u: &[Vec<f64>]) -> Vec<f64> {
    let p = |v: f64| I::point(v).unwrap();
    let d = y.len();
    let base = y.iter().copied().map(p).collect::<Vec<_>>();
    let j0 = s.interval_jac(&base);
    let sig = p(h).mul(p(GAMMA)).unwrap();
    let gamma = p(GAMMA);
    let mut res = vec![0_f64; 8];
    for i in 0..8 {
        let mut stage = base.clone();
        let mut mix = vec![I::zero(); d];
        for j in 0..i {
            for k in 0..d {
                stage[k] = stage[k].add(p(A[i][j]).mul(p(u[j][k])).unwrap()).unwrap();
                mix[k] = mix[k].add(p(C[i][j]).mul(p(u[j][k])).unwrap()).unwrap();
            }
        }
        let f = s.interval_rhs(&stage);
        for k in 0..d {
            let mut ju = I::zero();
            for l in 0..d {
                ju = ju.add(j0[k][l].mul(p(u[i][l])).unwrap()).unwrap();
            }
            let r = p(u[i][k])
                .sub(sig.mul(ju).unwrap())
                .unwrap()
                .sub(gamma.mul(mix[k]).unwrap())
                .unwrap()
                .sub(sig.mul(f[k]).unwrap())
                .unwrap();
            res[i] = res[i].max(r.mag());
        }
    }
    res
}

#[derive(Clone, Debug)]
struct ResearchProposal {
    before: Vec<f64>,
    after: Vec<f64>,
    h: f64,
    max_interval_residual: f64,
    embedded_wrms: f64,
    direct_linear_solves: u64,
}

// Single-source, exact-bit carrier, opt-in reference transaction. This is NOT
// native PR131 or adaptive production; work is always charged on rejected steps.
struct ResearchTransaction {
    source: Source,
    accepted: Vec<f64>,
    clock: f64,
    attempts: u64,
    accepts: u64,
    rejects: u64,
    attempted_linear_solves: u64,
    accepted_linear_solves: u64,
    rejected_linear_solves: u64,
}
impl ResearchTransaction {
    fn new(source: Source) -> Self {
        let accepted = source.initial();
        Self {
            source,
            accepted,
            clock: 0.,
            attempts: 0,
            accepts: 0,
            rejects: 0,
            attempted_linear_solves: 0,
            accepted_linear_solves: 0,
            rejected_linear_solves: 0,
        }
    }
    fn decide(&mut self, proposal: ResearchProposal) -> bool {
        self.attempts += 1;
        self.attempted_linear_solves += proposal.direct_linear_solves;
        let same_carrier = proposal.before.len() == self.accepted.len()
            && proposal
                .before
                .iter()
                .zip(&self.accepted)
                .all(|(a, b)| a.to_bits() == b.to_bits());
        let admissible = same_carrier
            && proposal.h.is_finite()
            && proposal.h > 0.
            && proposal.embedded_wrms.is_finite()
            && proposal.embedded_wrms <= 1.
            && proposal.max_interval_residual.is_finite()
            && proposal.max_interval_residual < 1e-9
            && proposal.after.iter().all(|x| x.is_finite());
        if admissible {
            self.accepted = proposal.after;
            self.clock += proposal.h;
            self.accepts += 1;
            self.accepted_linear_solves += proposal.direct_linear_solves;
        } else {
            self.rejects += 1;
            self.rejected_linear_solves += proposal.direct_linear_solves;
        }
        assert_eq!(self.attempts, self.accepts + self.rejects);
        assert_eq!(
            self.attempted_linear_solves,
            self.accepted_linear_solves + self.rejected_linear_solves
        );
        admissible
    }
    fn try_source_owned_reference(&mut self, h: f64) -> bool {
        let p = run_on_source(&self.source, &self.accepted, h);
        self.decide(p)
    }
}

fn run_on_source(s: &Source, y: &[f64], h: f64) -> ResearchProposal {
    let n = s.n;
    let d = y.len();
    let jac = s.jac(y);
    let mut w = vec![vec![0.; d]; d];
    for i in 0..d {
        for j in 0..d {
            w[i][j] = (if i == j { 1. } else { 0. }) - h * GAMMA * jac[i][j];
        }
    }
    let mut u = vec![vec![0.; d]; 8];
    let mut res = vec![0.; 8];
    let mut max_stage_energy = 0.;
    for i in 0..8 {
        let mut state = y.to_vec();
        let mut mix = vec![0.; d];
        for j in 0..i {
            for k in 0..d {
                state[k] += A[i][j] * u[j][k];
                mix[k] += C[i][j] * u[j][k];
            }
        }
        max_stage_energy = f64::max(max_stage_energy, s.energy(&state));
        let f = s.rhs(&state);
        let right = (0..d)
            .map(|k| GAMMA * (mix[k] + h * f[k]))
            .collect::<Vec<_>>();
        u[i] = solve(w.clone(), right);
        let mut r = 0_f64;
        for k in 0..d {
            let ju = (0..d).map(|j| jac[k][j] * u[i][j]).sum::<f64>();
            r = r.max((u[i][k] - h * GAMMA * ju - GAMMA * mix[k] - h * GAMMA * f[k]).abs());
        }
        res[i] = r;
    }
    let mut out = y.to_vec();
    for i in 0..8 {
        for k in 0..d {
            out[k] += B[i] * u[i][k];
        }
    }
    let envelope = audit_original_raw_u_intervals(s, y, h, &u);
    assert!(envelope.iter().all(|x| x.is_finite() && *x < 1e-9));
    let e0 = s.energy(y);
    let e1 = s.energy(&out);
    let embedded = (0..d)
        .map(|k| (u[7][k] / (1e-10 + 1e-7 * y[k].abs().max(out[k].abs()))).powi(2))
        .sum::<f64>()
        / (d as f64);
    let embedded = embedded.sqrt();
    println!(
        "N4_RUST_RAWU8 n={n} h={h:.6} max_rho={:.17e} embedded_wrms={embedded:.12e} e0={e0:.12e} e1={e1:.12e} stage_energy={max_stage_energy:.12e} interval_upper={envelope:?} residuals={res:?} output6={:?}",
        res.iter().copied().fold(0., f64::max),
        &out[..6]
    );
    assert!(res.iter().all(|x| *x < 1e-9));
    assert!(e1 < e0);
    ResearchProposal {
        before: y.to_vec(),
        after: out,
        h,
        max_interval_residual: envelope.iter().copied().fold(0., f64::max),
        embedded_wrms: embedded,
        direct_linear_solves: 8,
    }
}
fn run(n: usize, h: f64) -> ResearchProposal {
    let s = Source::new(n);
    let y = s.initial();
    run_on_source(&s, &y, h)
}
fn main() {
    for n in [12, 24] {
        for h in [0.001, 0.25] {
            let _ = run(n, h);
        }
    }
    let mut transaction = ResearchTransaction::new(Source::new(12));
    let diagnostic_refused = transaction.try_source_owned_reference(0.25);
    let small_step_admitted = transaction.try_source_owned_reference(1e-8);
    println!(
        "N4_RESEARCH_TRANSACTION rejected_big={} accepted_small={} attempts={} accepted={} rejected={} charged_solves={} accepted_solves={} rejected_solves={}",
        !diagnostic_refused,
        small_step_admitted,
        transaction.attempts,
        transaction.accepts,
        transaction.rejects,
        transaction.attempted_linear_solves,
        transaction.accepted_linear_solves,
        transaction.rejected_linear_solves
    );
    assert!(!diagnostic_refused && small_step_admitted);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_rawu_small_rank2_test() {
        let _ = run(12, 0.001);
        let _ = run(12, 0.25);
    }
    #[test]
    fn diagnostic_small_residual_never_overrides_rejected_embedded_error() {
        let source = Source::new(12);
        let mut tx = ResearchTransaction::new(source);
        let before = tx.accepted.clone();
        assert!(!tx.try_source_owned_reference(0.25));
        assert_eq!(tx.clock, 0.);
        assert!(
            before
                .iter()
                .zip(&tx.accepted)
                .all(|(x, y)| x.to_bits() == y.to_bits())
        );
        assert_eq!((tx.attempts, tx.accepts, tx.rejects), (1, 0, 1));
        assert_eq!(
            (tx.attempted_linear_solves, tx.rejected_linear_solves),
            (8, 8)
        );
    }
    #[test]
    fn source_owned_reference_accepts_only_a_small_adaptive_step_and_charges_both() {
        let mut tx = ResearchTransaction::new(Source::new(12));
        assert!(!tx.try_source_owned_reference(0.25));
        let before = tx.accepted.clone();
        assert!(tx.try_source_owned_reference(1e-8));
        assert!(tx.clock > 0.);
        assert!(
            tx.accepted
                .iter()
                .zip(before.iter())
                .any(|(x, y)| x.to_bits() != y.to_bits())
        );
        assert_eq!((tx.attempts, tx.accepts, tx.rejects), (2, 1, 1));
        assert_eq!(
            (
                tx.attempted_linear_solves,
                tx.accepted_linear_solves,
                tx.rejected_linear_solves
            ),
            (16, 8, 8)
        );
    }
    #[test]
    fn original_rawu_two_rate_rank2_n24() {
        let _ = run(24, 0.001);
        let _ = run(24, 0.25);
    }
}
