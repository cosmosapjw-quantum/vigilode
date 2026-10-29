//! SG01: standalone right-flexible GMRES prototype with a source-stamped,
//! outward original-equation residual gate and a finite-state route language.
//! This is NOT the native VigilODE integration, nor a physical33 provider.
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Init,
    Arnoldi,
    Check,
    Rejected,
    Committed,
    Aborted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    Input,
    SourceChanged,
    Nonfinite,
    Breakdown,
    Exhausted,
    IllegalTransition,
}
impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Failure {}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Work {
    pub columns: u64,
    pub pc_apps: u64,
    pub operator_apps: u64,
    pub candidate_builds: u64,
    pub original_checks: u64,
    pub accepted: u64,
    pub itinerary: String,
    pub outcome: Option<Failure>,
}

fn step(work: &mut Work, from: Route, to: Route) -> Result<Route, Failure> {
    use Route::*;
    let legal = matches!(
        (from, to),
        (Init, Arnoldi)
            | (Init, Aborted)
            | (Arnoldi, Arnoldi)
            | (Arnoldi, Check)
            | (Arnoldi, Aborted)
            | (Check, Committed)
            | (Check, Rejected)
            | (Check, Aborted)
            | (Rejected, Arnoldi)
            | (Rejected, Aborted)
    );
    if !legal {
        return Err(Failure::IllegalTransition);
    }
    let letter = match to {
        Init => 'I',
        Arnoldi => 'K',
        Check => 'Q',
        Rejected => 'R',
        Committed => 'C',
        Aborted => 'X',
    };
    work.itinerary.push(letter);
    Ok(to)
}

#[derive(Clone, Debug)]
pub struct Frozen<'a> {
    pub n: usize,
    pub j: &'a [f64],
    pub sigma: f64,
    pub rhs: &'a [f64],
    pub w: &'a [f64],
    pub epoch: u64,
}
impl Frozen<'_> {
    pub fn validate(&self) -> Result<(), Failure> {
        if self.n == 0
            || self.j.len() != self.n * self.n
            || self.rhs.len() != self.n
            || self.w.len() != self.n
            || !self.sigma.is_finite()
            || self.sigma < 0.0
            || !self.j.iter().chain(self.rhs).all(|v| v.is_finite())
            || !self.w.iter().all(|v| v.is_finite() && *v > 0.0)
        {
            return Err(Failure::Input);
        }
        Ok(())
    }
    pub fn apply_a(&self, x: &[f64], out: &mut [f64]) -> Result<(), Failure> {
        for (i, o) in out.iter_mut().enumerate() {
            let mut sum = 0.0;
            for (j, xj) in x.iter().enumerate() {
                sum += self.j[i * self.n + j] * xj;
            }
            *o = x[i] - self.sigma * sum;
            if !o.is_finite() {
                return Err(Failure::Nonfinite);
            }
        }
        Ok(())
    }
}

// Each floating product/addition is enclosed by its immediate binary64 neighbours.
// Inputs are interpreted as exact represented numbers. Overflow fails closed.
#[derive(Clone, Copy)]
struct Iv {
    lo: f64,
    hi: f64,
}
impl Iv {
    fn point(x: f64) -> Self {
        Self { lo: x, hi: x }
    }
    fn plus(self, b: Self) -> Result<Self, Failure> {
        let (lo, hi) = ((self.lo + b.lo).next_down(), (self.hi + b.hi).next_up());
        if lo.is_finite() && hi.is_finite() {
            Ok(Self { lo, hi })
        } else {
            Err(Failure::Nonfinite)
        }
    }
    fn times(self, x: f64) -> Result<Self, Failure> {
        let (lo, hi) = if x >= 0.0 {
            (self.lo * x, self.hi * x)
        } else {
            (self.hi * x, self.lo * x)
        };
        let (lo, hi) = (lo.next_down(), hi.next_up());
        if lo.is_finite() && hi.is_finite() {
            Ok(Self { lo, hi })
        } else {
            Err(Failure::Nonfinite)
        }
    }
    fn abs_upper(self) -> f64 {
        self.lo.abs().max(self.hi.abs())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Certificate {
    pub rho_upper: f64,
    pub mu_upper: f64,
    pub inverse_upper: f64,
    pub stage_error_upper: f64,
}

/// Exact-real A = I - sigma J for the *represented* sigma,J; not a rounded A matrix.
/// No callback/owner/physical source discrepancy is included in this authority.
pub fn certify(
    p: &Frozen<'_>,
    x: &[f64],
    original_budget: f64,
    stage_budget: f64,
) -> Result<Option<Certificate>, Failure> {
    p.validate()?;
    if x.len() != p.n
        || !x.iter().all(|v| v.is_finite())
        || !original_budget.is_finite()
        || !stage_budget.is_finite()
        || original_budget < 0.0
        || stage_budget < 0.0
    {
        return Err(Failure::Input);
    }
    let mut rho = 0.0_f64;
    for i in 0..p.n {
        // r = b - x + sigma * sum_j J_ij x_j, all in exact represented reals.
        let mut sum = Iv::point(0.0);
        for (k, xk) in x.iter().enumerate() {
            sum = sum.plus(Iv::point(p.j[i * p.n + k]).times(*xk)?)?;
        }
        let r = Iv::point(p.rhs[i])
            .plus(Iv::point(-x[i]))?
            .plus(sum.times(p.sigma)?)?;
        let v = (r.abs_upper() / p.w[i]).next_up();
        if !v.is_finite() {
            return Err(Failure::Nonfinite);
        }
        rho = rho.max(v);
    }
    let mut mu = f64::NEG_INFINITY;
    for i in 0..p.n {
        let mut row = p.j[i * p.n + i];
        for j in 0..p.n {
            if i != j {
                let ratio = (p.w[j] / p.w[i]).next_up();
                let term = (p.j[i * p.n + j].abs() * ratio).next_up();
                row = (row + term).next_up();
            }
        }
        if !row.is_finite() {
            return Err(Failure::Nonfinite);
        }
        mu = mu.max(row);
    }
    // sigma*mu<1 is a sufficient, not necessary, resolvent condition.
    let den = (1.0 - (p.sigma * mu).next_up()).next_down();
    if !den.is_finite() || den <= 0.0 {
        return Ok(None);
    }
    let inverse = (1.0 / den).next_up();
    let stage = (inverse * rho).next_up();
    if !inverse.is_finite() || !stage.is_finite() {
        return Err(Failure::Nonfinite);
    }
    let cert = Certificate {
        rho_upper: rho,
        mu_upper: mu,
        inverse_upper: inverse,
        stage_error_upper: stage,
    };
    if rho <= original_budget && stage <= stage_budget {
        Ok(Some(cert))
    } else {
        Ok(None)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub max_columns: usize,
    pub restart: usize,
    pub projected_trigger: f64,
    pub original_budget: f64,
    pub stage_budget: f64,
    pub every_prefix: bool,
}

fn l2(a: &[f64]) -> f64 {
    a.iter().map(|v| v * v).sum::<f64>().sqrt()
}
fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn same_epoch(p: &Frozen<'_>, epoch: &impl Fn() -> u64) -> Result<(), Failure> {
    if epoch() == p.epoch {
        Ok(())
    } else {
        Err(Failure::SourceChanged)
    }
}

/// Only writes `out` when original *outward* gate passes; partial work remains
/// in `work` on errors. `pc` can change at every column; z_j is stored (FGMRES).
/// The Givens residual is a candidate trigger only, never an acceptance gate.
pub fn solve_into<F, E>(
    p: &Frozen<'_>,
    x0: &[f64],
    out: &mut [f64],
    opt: Options,
    pc: F,
    epoch: E,
    work: &mut Work,
) -> Result<Certificate, Failure>
where
    F: FnMut(&[f64], &mut [f64]) -> Result<(), Failure>,
    E: Fn() -> u64,
{
    solve_inner(p, x0, out, opt, pc, epoch, work, None)
}

#[allow(clippy::too_many_arguments)]
fn solve_inner<F, E>(
    p: &Frozen<'_>,
    x0: &[f64],
    out: &mut [f64],
    opt: Options,
    mut pc: F,
    epoch: E,
    work: &mut Work,
    cache: Option<&SparseFrozen<'_>>,
) -> Result<Certificate, Failure>
where
    F: FnMut(&[f64], &mut [f64]) -> Result<(), Failure>,
    E: Fn() -> u64,
{
    let attempt = (|| {
        p.validate()?;
        if x0.len() != p.n
            || out.len() != p.n
            || !x0.iter().all(|x| x.is_finite())
            || opt.max_columns == 0
            || opt.restart == 0
            || !opt.projected_trigger.is_finite()
            || opt.projected_trigger < 0.0
        {
            return Err(Failure::Input);
        }
        same_epoch(p, &epoch)?;
        let mut route = step(work, Route::Init, Route::Arnoldi)?;
        let n = p.n;
        let mut x = x0.to_vec();
        let mut used = 0;
        while used < opt.max_columns {
            same_epoch(p, &epoch)?;
            let mut ax = vec![0.0; n];
            if let Some(s) = cache {
                s.apply_a(&x, &mut ax)?;
            } else {
                p.apply_a(&x, &mut ax)?;
            }
            work.operator_apps += 1;
            let r: Vec<f64> = p.rhs.iter().zip(ax).map(|(b, a)| b - a).collect();
            let beta = l2(&r);
            if !beta.is_finite() || beta <= f64::MIN_POSITIVE {
                return Err(Failure::Breakdown);
            }
            let steps = (opt.max_columns - used).min(opt.restart).min(n);
            let mut v = vec![r.iter().map(|x| x / beta).collect::<Vec<_>>()];
            let mut z: Vec<Vec<f64>> = Vec::with_capacity(steps);
            let mut h = vec![vec![0.0_f64; steps]; steps + 1];
            let mut g = vec![0.0_f64; steps + 1];
            g[0] = beta;
            let mut cs = vec![0.0; steps];
            let mut sn = vec![0.0; steps];
            let mut last_candidate: Option<Vec<f64>> = None;
            let mut cannot_extend = false;
            for j in 0..steps {
                same_epoch(p, &epoch)?;
                let mut zj = vec![0.0; n];
                pc(&v[j], &mut zj)?;
                work.pc_apps += 1;
                same_epoch(p, &epoch)?;
                if zj.iter().any(|x| !x.is_finite()) {
                    return Err(Failure::Nonfinite);
                }
                let mut w = vec![0.0; n];
                if let Some(s) = cache {
                    s.apply_a(&zj, &mut w)?;
                } else {
                    p.apply_a(&zj, &mut w)?;
                }
                work.operator_apps += 1;
                same_epoch(p, &epoch)?;
                for _ in 0..2 {
                    for i in 0..=j {
                        let hij = dot(&v[i], &w);
                        h[i][j] += hij;
                        for k in 0..n {
                            w[k] -= hij * v[i][k];
                        }
                    }
                }
                let hnext = l2(&w);
                if !hnext.is_finite() {
                    return Err(Failure::Nonfinite);
                }
                h[j + 1][j] = hnext;
                let happy = hnext <= 1e-13 * l2(&zj).max(1.0);
                z.push(zj);
                if !happy {
                    v.push(w.iter().map(|x| x / hnext).collect());
                }
                for i in 0..j {
                    let a = h[i][j];
                    let b = h[i + 1][j];
                    h[i][j] = cs[i] * a + sn[i] * b;
                    h[i + 1][j] = -sn[i] * a + cs[i] * b;
                }
                let ab = h[j][j].hypot(h[j + 1][j]);
                if !ab.is_finite() || ab == 0.0 {
                    return Err(Failure::Breakdown);
                }
                cs[j] = h[j][j] / ab;
                sn[j] = h[j + 1][j] / ab;
                h[j][j] = ab;
                h[j + 1][j] = 0.0;
                let gj = g[j];
                g[j] = cs[j] * gj;
                g[j + 1] = -sn[j] * gj;
                work.columns += 1;
                used += 1;
                route = step(work, route, Route::Arnoldi)?;
                let trigger = g[j + 1].abs() <= opt.projected_trigger || happy || j + 1 == steps;
                if opt.every_prefix || trigger {
                    work.candidate_builds += 1;
                    let mut y = vec![0.0; j + 1];
                    for k in (0..=j).rev() {
                        let mut val = g[k];
                        for q in k + 1..=j {
                            val -= h[k][q] * y[q];
                        }
                        y[k] = val / h[k][k];
                        if !y[k].is_finite() {
                            return Err(Failure::Nonfinite);
                        }
                    }
                    let mut candidate = x.clone();
                    for (zi, yi) in z.iter().zip(y) {
                        for k in 0..n {
                            candidate[k] += zi[k] * yi;
                        }
                    }
                    if candidate.iter().any(|x| !x.is_finite()) {
                        return Err(Failure::Nonfinite);
                    }
                    if !trigger {
                        continue;
                    } // Negative-cost control: build, but never admit early.
                    route = step(work, route, Route::Check)?;
                    same_epoch(p, &epoch)?;
                    work.original_checks += 1;
                    let cert = if let Some(s) = cache {
                        s.certify(&candidate, opt.original_budget, opt.stage_budget)?
                    } else {
                        certify(p, &candidate, opt.original_budget, opt.stage_budget)?
                    };
                    same_epoch(p, &epoch)?;
                    if let Some(cert) = cert {
                        route = step(work, route, Route::Committed)?;
                        same_epoch(p, &epoch)?;
                        out.copy_from_slice(&candidate);
                        work.accepted += 1;
                        debug_assert_eq!(route, Route::Committed);
                        return Ok(cert);
                    }
                    last_candidate = Some(candidate);
                    route = step(work, route, Route::Rejected)?;
                }
                if happy {
                    cannot_extend = true;
                    break;
                }
            }
            if cannot_extend {
                return Err(Failure::Exhausted);
            }
            if let Some(candidate) = last_candidate {
                x = candidate;
            } else {
                return Err(Failure::Breakdown);
            }
            route = step(work, route, Route::Arnoldi)?;
        }
        Err(Failure::Exhausted)
    })();
    if let Err(ref f) = attempt {
        work.outcome = Some(f.clone());
        if !work.itinerary.ends_with('X') {
            work.itinerary.push('X');
        }
    }
    attempt
}

/// Frozen exact structural sparsity for repeated solves with the *same* model,
/// clock, sigma and weighted-infinity norm; no approximated small entries.
/// The Rust lifetime ties the cache to the immutable frozen represented J.
#[derive(Debug)]
pub struct SparseFrozen<'a> {
    model: &'a Frozen<'a>,
    rows: Vec<Vec<(usize, f64)>>,
    mu_up: f64,
}
impl<'a> SparseFrozen<'a> {
    pub fn new(p: &'a Frozen<'a>) -> Result<Self, Failure> {
        p.validate()?;
        let mut rows = Vec::with_capacity(p.n);
        let mut mu_up = f64::NEG_INFINITY;
        for i in 0..p.n {
            let mut row = Vec::new();
            let mut mu_i = p.j[i * p.n + i];
            for j in 0..p.n {
                let a = p.j[i * p.n + j];
                if a != 0.0 {
                    row.push((j, a));
                    if i != j {
                        let ratio = (p.w[j] / p.w[i]).next_up();
                        let term = (a.abs() * ratio).next_up();
                        mu_i = (mu_i + term).next_up();
                    }
                }
            }
            if !mu_i.is_finite() {
                return Err(Failure::Nonfinite);
            }
            mu_up = mu_up.max(mu_i);
            rows.push(row);
        }
        Ok(Self {
            model: p,
            rows,
            mu_up,
        })
    }
    pub fn nnz(&self) -> usize {
        self.rows.iter().map(Vec::len).sum()
    }
    fn apply_a(&self, x: &[f64], out: &mut [f64]) -> Result<(), Failure> {
        for (i, o) in out.iter_mut().enumerate() {
            let mut sum = 0.0;
            for &(j, a) in &self.rows[i] {
                sum += a * x[j];
            }
            *o = x[i] - self.model.sigma * sum;
            if !o.is_finite() {
                return Err(Failure::Nonfinite);
            }
        }
        Ok(())
    }
    pub fn certify(
        &self,
        x: &[f64],
        original_budget: f64,
        stage_budget: f64,
    ) -> Result<Option<Certificate>, Failure> {
        let p = self.model;
        if x.len() != p.n
            || !x.iter().all(|v| v.is_finite())
            || !original_budget.is_finite()
            || !stage_budget.is_finite()
            || original_budget < 0.0
            || stage_budget < 0.0
        {
            return Err(Failure::Input);
        }
        let mut rho = 0.0_f64;
        for i in 0..p.n {
            let mut sum = Iv::point(0.0);
            for &(j, a) in &self.rows[i] {
                sum = sum.plus(Iv::point(a).times(x[j])?)?;
            }
            let r = Iv::point(p.rhs[i])
                .plus(Iv::point(-x[i]))?
                .plus(sum.times(p.sigma)?)?;
            let bound = (r.abs_upper() / p.w[i]).next_up();
            if !bound.is_finite() {
                return Err(Failure::Nonfinite);
            }
            rho = rho.max(bound);
        }
        let den = (1.0 - (p.sigma * self.mu_up).next_up()).next_down();
        if !den.is_finite() || den <= 0.0 {
            return Ok(None);
        }
        let inverse = (1.0 / den).next_up();
        let stage = (inverse * rho).next_up();
        if !inverse.is_finite() || !stage.is_finite() {
            return Err(Failure::Nonfinite);
        }
        let cert = Certificate {
            rho_upper: rho,
            mu_upper: self.mu_up,
            inverse_upper: inverse,
            stage_error_upper: stage,
        };
        if rho <= original_budget && stage <= stage_budget {
            Ok(Some(cert))
        } else {
            Ok(None)
        }
    }
}

pub fn solve_into_sparse<F, E>(
    s: &SparseFrozen<'_>,
    x0: &[f64],
    out: &mut [f64],
    opt: Options,
    pc: F,
    epoch: E,
    work: &mut Work,
) -> Result<Certificate, Failure>
where
    F: FnMut(&[f64], &mut [f64]) -> Result<(), Failure>,
    E: Fn() -> u64,
{
    solve_inner(s.model, x0, out, opt, pc, epoch, work, Some(s))
}

/// Verifier for the regular itinerary language of the solver guard automaton.
/// This encodes algorithmic route switching; it is NOT a Markov partition of
/// the physical ODE flow and implies no dynamical conjugacy or chaos result.
pub fn valid_itinerary(word: &str) -> bool {
    use Route::*;
    let mut state = Init;
    for (i, c) in word.chars().enumerate() {
        let next = match c {
            'K' => Arnoldi,
            'Q' => Check,
            'R' => Rejected,
            'C' => Committed,
            'X' => Aborted,
            _ => return false,
        };
        let legal = matches!(
            (state, next),
            (Init, Arnoldi)
                | (Init, Aborted)
                | (Arnoldi, Arnoldi)
                | (Arnoldi, Check)
                | (Arnoldi, Aborted)
                | (Check, Committed)
                | (Check, Rejected)
                | (Check, Aborted)
                | (Rejected, Arnoldi)
                | (Rejected, Aborted)
        );
        if !legal || (matches!(next, Committed | Aborted) && i + 1 != word.len()) {
            return false;
        }
        state = next;
    }
    matches!(state, Committed | Aborted)
}
