#![allow(clippy::needless_range_loop)]
//! N4C experimental, source-owned rank-2 client of the *actual* native
//! sequential_step from the 2026-10-29 offline snapshot of rodas5p-integrators.
//! Does not assert it is PR131's newer native binary; no production admission.
use rodas5p_core::directed::Interval as I;
use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, LinearSolverConfig, WorkCounters, load_rodas5p_coefficients,
};
use rodas5p_integrators::{OdeProblem, sequential_step};
use std::ops::Neg;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
#[allow(dead_code)] // Exact original tableau includes the unused dense-reference B vector.
#[path = "n4c_original_tableau.rs"]
mod original;
use original::{A, C, GAMMA};
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
        let mut m = [(I::point(0.).unwrap(), I::point(0.).unwrap()); 2];
        let mut out = vec![I::point(0.).unwrap(); 2 * self.n];
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
        let mut m = [(I::point(0.).unwrap(), I::point(0.).unwrap()); 2];
        for a in 0..2 {
            for j in 0..n {
                let v = p(self.w[j]).mul(p(self.phi[a][j])).unwrap();
                m[a].0 = m[a].0.add(v.mul(y[2 * j]).unwrap()).unwrap();
                m[a].1 = m[a].1.add(v.mul(y[2 * j + 1]).unwrap()).unwrap();
            }
        }
        let mut jac = vec![vec![I::point(0.).unwrap(); 2 * n]; 2 * n];
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
static TOKENS: AtomicU64 = AtomicU64::new(1);
fn exact_clock_exceeds(t: f64, h: f64, horizon: f64) -> bool {
    if !t.is_finite() || !h.is_finite() || !horizon.is_finite() || t < 0. || h <= 0. {
        return true;
    }
    let sum = t + h;
    if !sum.is_finite() {
        return true;
    }
    let bb = sum - t;
    let residual = (t - (sum - bb)) + (h - bb);
    sum > horizon || (sum == horizon && residual > 0.)
}
struct SourceOwner {
    token: u64,
    source: Source,
    epoch: Arc<AtomicU64>,
    rhs_calls: Arc<AtomicU64>,
    jac_calls: Arc<AtomicU64>,
}
impl SourceOwner {
    fn new(n: usize, sign: f64) -> Self {
        let mut source = Source::new(n);
        for v in &mut source.phi[1] {
            *v *= sign;
        }
        Self {
            token: TOKENS.fetch_add(1, Ordering::SeqCst),
            source,
            epoch: Arc::new(AtomicU64::new(0)),
            rhs_calls: Arc::new(AtomicU64::new(0)),
            jac_calls: Arc::new(AtomicU64::new(0)),
        }
    }
    fn validate(&self, t: f64, y: &[f64]) -> CoreResult<()> {
        if !t.is_finite()
            || t < 0.
            || t > 0.5
            || y.len() != 2 * self.source.n
            || y.iter().any(|x| !x.is_finite() || x.abs() > 1e6)
        {
            return Err(CoreError::InvalidInput("N4C SOURCE_CLOCK_SHAPE_BOX".into()));
        }
        Ok(())
    }
    fn callback_rhs(&self, t: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
        self.validate(t, y)?;
        if out.len() != y.len() {
            return Err(CoreError::Dimension("N4C RHS OUTPUT SHAPE".into()));
        }
        let f = self.source.rhs(y);
        if f.iter().any(|z| !z.is_finite()) {
            return Err(CoreError::NonFinite("N4C rhs".into()));
        }
        out.copy_from_slice(&f);
        self.rhs_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn callback_jac(&self, t: f64, y: &[f64]) -> CoreResult<DenseMatrix> {
        self.validate(t, y)?;
        let data = self.source.jac(y);
        self.jac_calls.fetch_add(1, Ordering::SeqCst);
        DenseMatrix::from_vec_rows(data)
    }
}
struct NativeSourceBundle {
    owner: Arc<SourceOwner>,
    problem: OdeProblem,
}
impl NativeSourceBundle {
    fn new(n: usize, sign: f64) -> CoreResult<Self> {
        let owner = Arc::new(SourceOwner::new(n, sign));
        let r = Arc::clone(&owner);
        let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| r.callback_rhs(t, y, out));
        let j = Arc::clone(&owner);
        let jac = Arc::new(move |t: f64, y: &[f64]| j.callback_jac(t, y));
        let e = Arc::clone(&owner);
        let problem = OdeProblem::new(
            "N4C-native-rank2-private-callback",
            2 * n,
            rhs,
            None,
            Some(jac),
            None,
            None,
            true,
            None,
            None,
        )?
        .with_model_epoch(Arc::new(move || e.epoch.load(Ordering::SeqCst)));
        Ok(Self { owner, problem })
    }
}
#[derive(Debug)]
struct Trial {
    id: u64,
    owner_token: u64,
    epoch: u64,
    t_bits: u64,
    initial_bits: Vec<u64>,
    output: Vec<f64>,
    output_bits: Vec<u64>,
    h: f64,
    native_accepted: bool,
    wrms: f64,
    residuals: [f64; 8],
    work: WorkCounters,
}
#[derive(Clone, Debug)]
struct PendingReceipt {
    id: u64,
    linear: u64,
    h_bits: u64,
    native_accepted: bool,
    wrms_bits: u64,
    residual_bits: [u64; 8],
    output_bits: Vec<u64>,
}
struct NativeSession {
    bundle: NativeSourceBundle,
    y: Vec<f64>,
    time: f64,
    epoch: u64,
    next_id: u64,
    pending: Option<PendingReceipt>, // (trial id, fully charged number of linear solves)
    attempted: u64,
    accepted: u64,
    rejected: u64,
    total_linear: u64,
    accepted_linear: u64,
    rejected_linear: u64,
}
impl NativeSession {
    fn new(n: usize, sign: f64) -> Self {
        let bundle = NativeSourceBundle::new(n, sign).unwrap();
        let y = bundle.owner.source.initial();
        Self {
            bundle,
            y,
            time: 0.,
            epoch: 0,
            next_id: 1,
            pending: None,
            attempted: 0,
            accepted: 0,
            rejected: 0,
            total_linear: 0,
            accepted_linear: 0,
            rejected_linear: 0,
        }
    }
    fn balanced(&self) -> bool {
        self.attempted == self.accepted + self.rejected + u64::from(self.pending.is_some())
            && self.total_linear
                == self.accepted_linear
                    + self.rejected_linear
                    + self.pending.as_ref().map_or(0, |x| x.linear)
    }
    fn abort_pending(&mut self) {
        if let Some(receipt) = self.pending.take() {
            self.rejected += 1;
            self.rejected_linear += receipt.linear;
        }
        assert!(self.balanced());
    }
    fn propose(&mut self, h: f64) -> CoreResult<Trial> {
        if self.pending.is_some() {
            return Err(CoreError::InvalidInput("N4C PENDING_TRANSACTION".into()));
        }
        if self.bundle.owner.source.n <= 8 || exact_clock_exceeds(self.time, h, 0.5) {
            return Err(CoreError::InvalidInput("N4C SMALL_N_OR_HORIZON".into()));
        }
        if self.bundle.owner.epoch.load(Ordering::SeqCst) != self.epoch {
            return Err(CoreError::InvalidInput("N4C STALE_SOURCE_EPOCH".into()));
        }
        let id = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or_else(|| CoreError::InvalidInput("N4C TRIAL_ID_OVERFLOW".into()))?;
        let mut work = WorkCounters::default();
        // force_accept=false is non-negotiable: the native solver owns the embedded time-error decision.
        let outcome = sequential_step(
            &self.bundle.problem,
            self.time,
            &self.y,
            h,
            &LinearSolverConfig::default(),
            None,
            1e-10,
            1e-7,
            false,
            &mut work,
        );
        let audit = outcome.and_then(|r| {
            let residuals = self.raw_u_residuals(&r.stages, h)?;
            Ok((r, residuals))
        });
        // All completed OR failed native work must be charged exactly once.
        self.attempted += 1;
        self.total_linear += work.linear_solves;
        match audit {
            Ok((r, residuals)) => {
                self.pending = Some(PendingReceipt {
                    id,
                    linear: work.linear_solves,
                    h_bits: h.to_bits(),
                    native_accepted: r.accepted,
                    wrms_bits: r.error_norm.to_bits(),
                    residual_bits: residuals.map(f64::to_bits),
                    output_bits: r.y_new.iter().map(|v| v.to_bits()).collect(),
                });
                assert!(self.balanced());
                let output_bits = r.y_new.iter().map(|v| v.to_bits()).collect();
                Ok(Trial {
                    id,
                    owner_token: self.bundle.owner.token,
                    epoch: self.epoch,
                    t_bits: self.time.to_bits(),
                    initial_bits: self.y.iter().map(|v| v.to_bits()).collect(),
                    output: r.y_new,
                    output_bits,
                    h,
                    native_accepted: r.accepted,
                    wrms: r.error_norm,
                    residuals,
                    work,
                })
            }
            Err(e) => {
                self.rejected += 1;
                self.rejected_linear += work.linear_solves;
                assert!(self.balanced());
                Err(e)
            }
        }
    }
    fn raw_u_residuals(&self, k_stage: &[Vec<f64>], h: f64) -> CoreResult<[f64; 8]> {
        if k_stage.len() != 8 || k_stage.iter().any(|s| s.len() != self.y.len()) {
            return Err(CoreError::Dimension(
                "N4C original eight-stage shape".into(),
            ));
        }
        let coeff = load_rodas5p_coefficients()?;
        let d = self.y.len();
        let p = |x: f64| I::point(x);
        let mut u = vec![vec![I::point(0.)?; d]; 8];
        for i in 0..8 {
            for j in 0..=i {
                for q in 0..d {
                    u[i][q] =
                        u[i][q].add(p(coeff.gamma_matrix[(i, j)])?.mul(p(k_stage[j][q])?)?)?;
                }
            }
        }
        let y = self
            .y
            .iter()
            .map(|x| p(*x))
            .collect::<CoreResult<Vec<_>>>()?;
        let jac = self.bundle.owner.source.interval_jac(&y);
        let mut out = [0.0_f64; 8];
        for i in 0..8 {
            let mut yi = y.clone();
            let mut mix = vec![I::point(0.)?; d];
            for q in 0..d {
                for prev in 0..i {
                    yi[q] = yi[q].add(p(A[i][prev])?.mul(u[prev][q])?)?;
                    mix[q] = mix[q].add(p(C[i][prev])?.mul(u[prev][q])?)?;
                }
            }
            let f = self.bundle.owner.source.interval_rhs(&yi);
            for q in 0..d {
                let mut ju = I::point(0.)?;
                for l in 0..d {
                    ju = ju.add(jac[q][l].mul(u[i][l])?)?;
                }
                let r = u[i][q]
                    .sub(p(h)?.mul(p(GAMMA)?)?.mul(ju)?)?
                    .sub(p(GAMMA)?.mul(mix[q])?)?
                    .sub(p(h)?.mul(p(GAMMA)?)?.mul(f[q])?)?;
                out[i] = out[i].max(r.mag());
            }
        }
        Ok(out)
    }
    fn decide(&mut self, trial: Trial) -> Result<bool, String> {
        // No work is charged on a foreign/replayed trial, and the existing pending receipt remains intact.
        if trial.owner_token != self.bundle.owner.token
            || !self.pending.as_ref().is_some_and(|receipt| {
                receipt.id == trial.id
                    && receipt.linear == trial.work.linear_solves
                    && receipt.h_bits == trial.h.to_bits()
                    && receipt.native_accepted == trial.native_accepted
                    && receipt.wrms_bits == trial.wrms.to_bits()
                    && receipt.residual_bits == trial.residuals.map(f64::to_bits)
                    && receipt.output_bits == trial.output_bits
            })
            || trial.epoch != self.epoch
            || trial.t_bits != self.time.to_bits()
            || trial.initial_bits != self.y.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
            || trial.output_bits != trial.output.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
            || exact_clock_exceeds(self.time, trial.h, 0.5)
            || self.bundle.owner.epoch.load(Ordering::SeqCst) != self.epoch
        {
            return Err("N4C STALE_FOREIGN_OR_SUBSTITUTED_TRIAL".into());
        }
        let receipt = self.pending.take().unwrap();
        let work = receipt.linear;
        assert_eq!(receipt.id, trial.id);
        let rawok = trial.residuals.iter().all(|r| r.is_finite() && *r < 1e-8);
        let e0 = self.bundle.owner.source.energy(&self.y);
        let e1 = self.bundle.owner.source.energy(&trial.output);
        let good = trial.native_accepted
            && trial.wrms.is_finite()
            && trial.wrms <= 1.0
            && rawok
            && e1.is_finite()
            && e1 <= e0
            && trial.output.iter().all(|x| x.is_finite());
        if good {
            self.y = trial.output;
            self.time += trial.h;
            self.accepted += 1;
            self.accepted_linear += work;
            self.epoch += 1;
            self.bundle.owner.epoch.store(self.epoch, Ordering::SeqCst);
        } else {
            self.rejected += 1;
            self.rejected_linear += work;
        }
        assert!(self.balanced());
        Ok(good)
    }
}
#[test]
fn native_stage_rawu_and_rejected_large_window() {
    let mut session = NativeSession::new(12, 1.);
    let epoch = session.bundle.owner.epoch.load(Ordering::SeqCst);
    let original = session.y.clone();
    let trial = session.propose(0.25).unwrap();
    println!(
        "N4C_NATIVE n12_h025 error={} residuals={:?} work_rhs={} work_jac={} direct={} linear={}",
        trial.wrms,
        trial.residuals,
        trial.work.rhs_calls,
        trial.work.jacobian_builds,
        trial.work.direct_solve_calls,
        trial.work.linear_solves
    );
    println!(
        "N4C_NATIVE_OUTPUT n12_h025 output6={:?}",
        &trial.output[..6]
    );
    assert!(!trial.native_accepted);
    assert!(trial.residuals.iter().all(|r| *r < 1e-8));
    assert!(!session.decide(trial).unwrap());
    assert_eq!(session.epoch, epoch);
    assert_eq!(session.time.to_bits(), 0_f64.to_bits());
    assert!(
        session
            .y
            .iter()
            .zip(&original)
            .all(|(a, b)| a.to_bits() == b.to_bits())
    );
}
#[test]
fn native_small_step_accept_and_ledger() {
    let mut session = NativeSession::new(12, 1.);
    let rejected = session.propose(0.25).unwrap();
    assert!(!session.decide(rejected).unwrap());
    let accepted = session.propose(1e-8).unwrap();
    println!(
        "N4C_NATIVE n12_h1e-8 error={} residuals={:?} linear={}",
        accepted.wrms, accepted.residuals, accepted.work.linear_solves
    );
    println!(
        "N4C_NATIVE_OUTPUT n12_h1e-8 output6={:?}",
        &accepted.output[..6]
    );
    assert!(session.decide(accepted).unwrap());
    assert_eq!(
        (session.attempted, session.accepted, session.rejected),
        (2, 1, 1)
    );
    assert_eq!(session.total_linear, 16);
    assert_eq!(session.accepted_linear + session.rejected_linear, 16);
    assert_eq!(session.bundle.owner.epoch.load(Ordering::SeqCst), 1);
    assert_eq!(session.time.to_bits(), 1e-8_f64.to_bits());
    assert!(session.balanced());
}
#[test]
fn original_involution_rhs_jac_and_source_binding() {
    let a = NativeSession::new(12, 1.);
    let b = NativeSession::new(12, -1.);
    let y = a.y.clone();
    let fa = a.bundle.owner.source.rhs(&y);
    let fb = b.bundle.owner.source.rhs(&y);
    let ja = a.bundle.owner.source.jac(&y);
    let jb = b.bundle.owner.source.jac(&y);
    assert!(fa.iter().zip(&fb).all(|(x, y)| x.to_bits() == y.to_bits()));
    assert!(
        ja.iter()
            .flatten()
            .zip(jb.iter().flatten())
            .all(|(x, y)| x.to_bits() == y.to_bits())
    );
    assert_eq!(a.bundle.problem.model_epoch(), Some(0));
    assert_ne!(a.bundle.owner.token, b.bundle.owner.token);
}
#[test]
fn no_small_n_or_exact_horizon_overrun() {
    let mut s = NativeSession::new(8, 1.);
    assert!(s.propose(0.25).is_err());
    let mut bigger = NativeSession::new(12, 1.);
    bigger.time = 0.25;
    assert!(
        bigger
            .propose(f64::from_bits(0.25_f64.to_bits() + 1))
            .is_err()
    );
}

#[test]
fn reject_foreign_and_replayed_candidate_without_extra_charge() {
    let mut a = NativeSession::new(12, 1.);
    let mut b = NativeSession::new(12, 1.);
    let trial_a = a.propose(1e-8).unwrap();
    let trial_b = b.propose(1e-8).unwrap();
    assert!(a.decide(trial_b).is_err());
    assert_eq!(a.pending.as_ref().map(|x| x.id), Some(trial_a.id));
    assert!(a.decide(trial_a).unwrap());
    assert_eq!((a.attempted, a.accepted, a.rejected), (1, 1, 0));
    assert!(a.balanced());
    b.abort_pending();
    assert_eq!((b.attempted, b.accepted, b.rejected), (1, 0, 1));
    assert!(b.balanced());
}
#[test]
fn pending_triggers_abort_and_preserves_state() {
    let mut session = NativeSession::new(12, 1.);
    let y = session.y.clone();
    let epoch = session.epoch;
    let trial = session.propose(1e-8).unwrap();
    assert!(session.propose(1e-8).is_err());
    assert!(session.balanced());
    session.abort_pending();
    assert!(session.decide(trial).is_err());
    assert_eq!(
        (session.attempted, session.accepted, session.rejected),
        (1, 0, 1)
    );
    assert_eq!(session.epoch, epoch);
    assert_eq!(session.time, 0.);
    assert_eq!(session.total_linear, 8);
    assert!(
        session
            .y
            .iter()
            .zip(y.iter())
            .all(|(x, z)| x.to_bits() == z.to_bits())
    );
}
#[test]
fn native_rank2_n24_original_eight_stage_refusal_and_small_step() {
    let mut s = NativeSession::new(24, 1.);
    let r = s.propose(0.001).unwrap();
    println!(
        "N4C_NATIVE n24_h0.001 wrms={:.9e} residuals={:?} callbacks_rhs={} jac={} native_direct={}",
        r.wrms, r.residuals, r.work.rhs_calls, r.work.jacobian_builds, r.work.direct_solve_calls
    );
    println!("N4C_NATIVE_OUTPUT n24_h0.001 output6={:?}", &r.output[..6]);
    assert_eq!(r.work.linear_solves, 8);
    assert!(r.residuals.iter().all(|v| *v < 1e-9));
    let _ = s.decide(r).unwrap();
    assert!(s.balanced());
}
#[test]
fn native_raw_u_fault_injected_refusal() {
    let mut s = NativeSession::new(12, 1.);
    let mut r = s.propose(1e-8).unwrap();
    r.residuals[3] = 1.0; // Simulated false stage certificate, cannot override native acceptance.
    assert!(s.decide(r).is_err()); // Replacing the measured interval is a forged receipt.
    s.abort_pending();
    assert_eq!(s.epoch, 0);
    assert_eq!(s.time, 0.);
    assert_eq!((s.accepted_linear, s.rejected_linear), (0, 8));
}
#[test]
fn native_source_immutable_probe_does_not_advance_epoch() {
    let mut s = NativeSession::new(12, 1.);
    let before = s.bundle.owner.epoch.load(Ordering::SeqCst);
    let r = s.propose(1e-8).unwrap();
    assert_eq!(s.bundle.owner.epoch.load(Ordering::SeqCst), before);
    assert_eq!(s.bundle.problem.model_epoch(), Some(before));
    assert!(r.work.rhs_calls >= 8);
    assert_eq!(r.work.jacobian_builds, 1);
    s.abort_pending();
    assert_eq!(s.bundle.owner.epoch.load(Ordering::SeqCst), before);
}

#[test]
fn physical_energy_balance_and_jacobian_direction() {
    for n in [12, 24, 65] {
        let s = Source::new(n);
        let y = s.initial();
        let f = s.rhs(&y);
        let physical_derivative = (0..n)
            .map(|j| 2. * s.w[j] * (y[2 * j] * f[2 * j] + y[2 * j + 1] * f[2 * j + 1]))
            .sum::<f64>();
        let negative_damping = -2.
            * (0..n)
                .map(|j| s.w[j] * s.kap[j] * (y[2 * j] * y[2 * j] + y[2 * j + 1] * y[2 * j + 1]))
                .sum::<f64>();
        assert!(
            (physical_derivative - negative_damping).abs()
                <= 3e-13 * negative_damping.abs().max(1.)
        );
        let jac = s.jac(&y);
        let v = (0..2 * n)
            .map(|k| 0.025 + ((k * 7) % 17) as f64 / 500.)
            .collect::<Vec<_>>();
        let eps = 1e-7;
        let mut yp = y.clone();
        let mut ym = y.clone();
        for k in 0..2 * n {
            yp[k] += eps * v[k];
            ym[k] -= eps * v[k];
        }
        let fp = s.rhs(&yp);
        let fm = s.rhs(&ym);
        let maxerr = (0..2 * n)
            .map(|i| {
                let jv = (0..2 * n).map(|k| jac[i][k] * v[k]).sum::<f64>();
                ((fp[i] - fm[i]) / (2. * eps) - jv).abs()
            })
            .fold(0.0_f64, f64::max);
        println!(
            "N4C_RANK2_SOURCE n={} Eprime={:.9e} physical={:.9e} jac_direction_max={:.9e}",
            n, physical_derivative, negative_damping, maxerr
        );
        assert!(maxerr < 1e-6);
    }
}
#[test]
fn rank2_n65_original_raw_u_stage_audit_no_production_admission() {
    let mut s = NativeSession::new(65, 1.);
    let trial = s.propose(0.001).unwrap();
    println!(
        "N4C_NATIVE n65_h0.001 embedded={} stage_residual_max={} factorisations={} linear={} rhs={}",
        trial.wrms,
        trial.residuals.iter().copied().fold(0., f64::max),
        trial.work.direct_factorizations,
        trial.work.linear_solves,
        trial.work.rhs_calls
    );
    assert_eq!(trial.work.linear_solves, 8);
    assert_eq!(trial.residuals.len(), 8);
    assert!(trial.residuals.iter().all(|x| x.is_finite() && *x < 1e-8));
    let _ = s.decide(trial).unwrap();
    assert!(s.balanced());
}
#[test]
fn tampered_native_endpoint_and_epoch_refused_before_commit() {
    let mut session = NativeSession::new(12, 1.);
    let mut trial = session.propose(1e-8).unwrap();
    trial.output[0] += 1.0;
    assert!(session.decide(trial).is_err());
    assert!(session.balanced());
    session.abort_pending();
    assert_eq!(session.accepted, 0);
    assert_eq!(session.rejected, 1);
    assert_eq!(session.epoch, 0);
    assert_eq!(session.time, 0.);
    let _ = session.propose(1e-8).unwrap();
    session.bundle.owner.epoch.store(100, Ordering::SeqCst);
    let trial_id = session.pending.as_ref().unwrap().id;
    assert!(session.pending.is_some() && trial_id > 0);
    session.abort_pending();
    assert!(session.propose(1e-8).is_err());
}

#[test]
fn forged_h_native_acceptance_and_stage_residuals_rejected_without_false_commit() {
    let mut source = NativeSession::new(12, 1.);
    let mut trial = source.propose(1e-8).unwrap();
    trial.h = 0.25; // try to commit a small accepted state as a 0.25-wide physical step
    assert!(source.decide(trial).is_err());
    source.abort_pending();
    assert_eq!((source.accepted, source.rejected), (0, 1));
    assert!(source.balanced());

    let mut source = NativeSession::new(12, 1.);
    let mut trial = source.propose(0.25).unwrap();
    trial.native_accepted = true;
    trial.wrms = 0.; // forcing the diagnostic output through the time-error gate
    assert!(source.decide(trial).is_err());
    source.abort_pending();
    assert_eq!((source.accepted, source.rejected), (0, 1));
    assert!(source.balanced());

    let mut source = NativeSession::new(12, 1.);
    let mut trial = source.propose(1e-8).unwrap();
    trial.residuals = [0.; 8]; // replacing true computed residuals, not a valid source receipt
    assert!(source.decide(trial).is_err());
    source.abort_pending();
    assert!(source.balanced());
}
