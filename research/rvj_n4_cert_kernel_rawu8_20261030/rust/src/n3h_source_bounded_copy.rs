//! Research-only N3 source-owned two-rate rank-2 Volterra windows.
//! Exact-real *analytical* bounds apply to exact Picard and exact accepted carriers.
//! Bound metadata uses upward arithmetic for represented exact-real recurrence;
//! exponential/polynomial/carrier rounding and floating state are NOT certified.
//! Does NOT implement RODAS8, adaptive physical acceptance, or production solver.
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SOURCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct C {
    pub re: f64,
    pub im: f64,
}
impl C {
    fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }
    fn zero() -> Self {
        Self::new(0., 0.)
    }
    fn conj(self) -> Self {
        Self::new(self.re, -self.im)
    }
    fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }
    fn is_finite(self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }
    fn exp(self) -> Self {
        let k = self.re.exp();
        Self::new(k * self.im.cos(), k * self.im.sin())
    }
    fn scale(self, x: f64) -> Self {
        Self::new(self.re * x, self.im * x)
    }
}
use std::ops::{Add, Div, Mul, Sub};
impl Add for C {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        Self::new(self.re + b.re, self.im + b.im)
    }
}
impl Sub for C {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        Self::new(self.re - b.re, self.im - b.im)
    }
}
impl Mul for C {
    type Output = Self;
    fn mul(self, b: Self) -> Self {
        Self::new(
            self.re * b.re - self.im * b.im,
            self.re * b.im + self.im * b.re,
        )
    }
}
impl Div for C {
    type Output = Self;
    fn div(self, b: Self) -> Self {
        let norm = b.re * b.re + b.im * b.im;
        Self::new(
            (self.re * b.re + self.im * b.im) / norm,
            (self.im * b.re - self.re * b.im) / norm,
        )
    }
}
fn poly_mul(a: &[C], b: &[C]) -> Vec<C> {
    let mut r = vec![C::zero(); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            r[i + j] = r[i + j] + (*x * (*y));
        }
    }
    r
}
fn fact(n: usize) -> f64 {
    (1..=n).fold(1., |a, i| a * (i as f64))
}

/// Actual source, whose parameter vector and phi rank are never caller-replaceable in a Step.
#[derive(Debug)]
pub struct Source {
    token: u64,
    n: usize,
    weights: Vec<f64>,
    phi: [Vec<f64>; 2],
    chi: [f64; 2],
    xi: Vec<f64>,
    kappa: Vec<f64>,
    omega: f64,
    horizon: f64,
}
impl Source {
    pub fn toy(n: usize) -> Result<Arc<Self>, String> {
        Self::toy_sign(n, 1.)
    }
    pub fn toy_sign(n: usize, sign: f64) -> Result<Arc<Self>, String> {
        if n < 4 || !sign.is_finite() || sign.abs() != 1. {
            return Err("bad toy size/sign".into());
        }
        let weights = vec![1. / n as f64; n];
        let xi = (0..n)
            .map(|j| 2. * (j as f64) / ((n - 1) as f64) - 1.)
            .collect();
        let kappa = (0..n)
            .map(|j| if j % 2 == 0 { 0.25 } else { 1000. })
            .collect();
        let phi = [
            vec![1.; n],
            (0..n)
                .map(|j| if j % 2 == 0 { sign } else { -sign })
                .collect(),
        ];
        let source = Self {
            token: NEXT_SOURCE.fetch_add(1, Ordering::Relaxed),
            n,
            weights,
            phi,
            chi: [0.05, 0.02],
            xi,
            kappa,
            omega: 0.5,
            horizon: 0.5,
        };
        source.check()?;
        Ok(Arc::new(source))
    }
    fn check(&self) -> Result<(), String> {
        if self.n == 0
            || self.weights.len() != self.n
            || self.xi.len() != self.n
            || self.kappa.len() != self.n
            || self.phi.iter().any(|p| p.len() != self.n)
        {
            return Err("invalid dimensions".into());
        }
        if !(self.horizon > 0. && self.omega.is_finite() && self.chi.iter().all(|c| c.is_finite()))
        {
            return Err("invalid time/coefficients".into());
        }
        if self.kappa.iter().any(|k| !k.is_finite() || *k < 0.)
            || self.weights.iter().any(|w| !w.is_finite() || *w <= 0.)
            || (self.weights.iter().sum::<f64>() - 1.).abs() > 1e-14
        {
            return Err("bad measure or decay".into());
        }
        if self.xi.iter().any(|x| !x.is_finite() || x.abs() > 1.)
            || self
                .phi
                .iter()
                .any(|row| row.iter().any(|x| !x.is_finite()))
        {
            return Err("bad phi/xi".into());
        }
        if self.phi.iter().any(|row| {
            row.iter()
                .zip(&self.weights)
                .map(|(p, w)| w * p * p)
                .sum::<f64>()
                > 1. + 1e-14
        }) {
            return Err("weighted phi exceeds one".into());
        }
        Ok(())
    }
    pub fn n(&self) -> usize {
        self.n
    }
    /// Non-authoritative dimensionless stiffness telemetry for the current window.
    pub fn window_mode_counts(&self, h: f64) -> Result<(usize, usize), String> {
        if !h.is_finite() || h <= 0. {
            return Err("bad detector time width".into());
        }
        let fast = self.kappa.iter().filter(|k| **k * h >= 10.).count();
        Ok((self.n - fast, fast))
    }
    pub fn token(&self) -> u64 {
        self.token
    }
    fn initial(&self) -> Vec<C> {
        (0..self.n)
            .map(|j| C::new(0.02 + self.xi[j] / 500., ((j % 3) as f64 - 1.) / 200.))
            .collect()
    }
    #[cfg(test)]
    fn energy(&self, y: &[C]) -> f64 {
        y.iter()
            .zip(&self.weights)
            .map(|(z, w)| w * (z.re * z.re + z.im * z.im))
            .sum()
    }
    // Upward enclosure for exact energy of represented binary64 inputs/weights.
    fn energy_upper(&self, y: &[C]) -> f64 {
        up_sum_all((0..self.n).map(|j| {
            let r = y[j].re.abs();
            let i = y[j].im.abs();
            up_product(self.weights[j], up_sum(up_product(r, r), up_product(i, i)))
        }))
    }
    #[allow(clippy::needless_range_loop)] // explicit rank/channel tensor indices
    pub fn rhs(&self, y: &[C]) -> Vec<C> {
        let mut m = [C::zero(); 2];
        for a in 0..2 {
            for (j, z) in y.iter().enumerate() {
                m[a] = m[a] + z.scale(self.weights[j] * self.phi[a][j]);
            }
        }
        (0..self.n)
            .map(|j| {
                let mut u = C::new(-self.kappa[j], self.omega * self.xi[j]) * y[j];
                for a in 0..2 {
                    let rot = C::new(0., self.chi[a] * m[a].abs().powi(2) * self.phi[a][j]);
                    u = u + rot * m[a];
                }
                u
            })
            .collect()
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Work {
    pub attempted: u64,
    pub accepted: u64,
    pub rejected: u64,
    pub pending: u64,
    pub exponential_calls: u64,
    pub moment_calls: u64,
    pub polynomial_multiplies: u64,
}
#[derive(Clone, Debug)]
pub struct Attempt {
    source_token: u64,
    trial_id: u64,
    epoch: u64,
    clock_bits: u64,
    h: f64,
    next: Vec<C>,
    next_error_ideal: Vec<f64>,
    q: f64,
    p: usize,
    work: Work,
}
impl Attempt {
    pub fn projected_ideal_bound(&self) -> f64 {
        self.next_error_ideal.iter().copied().fold(0., f64::max)
    }
    pub fn q(&self) -> f64 {
        self.q
    }
    pub fn order(&self) -> usize {
        self.p
    }
    pub fn h(&self) -> f64 {
        self.h
    }
    pub fn work(&self) -> Work {
        self.work
    }
}
/// Compare the exact-real sum of *represented binary64* nonnegative clocks
/// with a represented source horizon. A nearest-rounded sum can equal the
/// horizon although its exact dyadic sum is greater. Knuth's TwoSum keeps the
/// residual of that represented addition without a blanket time tolerance.
/// Assumes IEEE-754 round-to-nearest and finite inputs, as in the N3R test host.
fn represented_clock_exceeds_horizon(t: f64, h: f64, horizon: f64) -> bool {
    if !(t.is_finite() && h.is_finite() && horizon.is_finite()) {
        return true;
    }
    let rounded = t + h;
    if !rounded.is_finite() || rounded > horizon {
        return true;
    }
    if rounded < horizon {
        return false;
    }
    let shifted = rounded - t;
    let exact_residual = (t - (rounded - shifted)) + (h - shifted);
    exact_residual > 0.0
}

#[derive(Debug)]
pub struct Session {
    source: Arc<Source>,
    time: f64,
    y: Vec<C>,
    ideal_error: Vec<f64>,
    epoch: u64,
    next_trial_id: u64,
    active_trial: Option<u64>,
    work: Work,
}
impl Session {
    pub fn new(source: Arc<Source>) -> Result<Self, String> {
        let y = source.initial();
        Self::from_initial(source, y)
    }
    pub fn from_initial(source: Arc<Source>, y: Vec<C>) -> Result<Self, String> {
        if y.len() != source.n || y.iter().any(|z| !z.is_finite()) || source.energy_upper(&y) > 1. {
            return Err("carrier outside physical energy domain".into());
        }
        let n = source.n;
        Ok(Self {
            source,
            time: 0.,
            y,
            ideal_error: vec![0.; n],
            epoch: 0,
            next_trial_id: 1,
            active_trial: None,
            work: Work::default(),
        })
    }
    pub fn time(&self) -> f64 {
        self.time
    }
    pub fn state(&self) -> &[C] {
        &self.y
    }
    pub fn ideal_bounds(&self) -> &[f64] {
        &self.ideal_error
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    pub fn work(&self) -> Work {
        self.work
    }
    pub fn is_balanced(&self) -> bool {
        self.work.attempted == self.work.accepted + self.work.rejected + self.work.pending
    }
    pub fn abort_pending(&mut self) {
        if self.active_trial.take().is_some() {
            self.work.pending -= 1;
            self.work.rejected += 1;
        }
    }
    /// A source-locked attempt. Any error/rejection preserves the accepted state,
    /// and used work remains charged. The ideal bound excludes floating evaluation error.
    pub fn propose(&mut self, h: f64, p: usize) -> Result<Attempt, String> {
        if self.source.n <= 8 {
            return Err("protected-small-n-baseline".into());
        }
        if !h.is_finite()
            || h <= 0.
            || p > 10
            || represented_clock_exceeds_horizon(self.time, h, self.source.horizon)
        {
            return Err("invalid proposal width/order".into());
        }
        if self.ideal_error.iter().any(|b| !b.is_finite() || *b < 0.) {
            return Err("untrusted inherited error".into());
        }
        self.abort_pending(); // superseded speculation is charged, never silently dropped
        self.work.attempted += 1;
        self.work.pending += 1;
        let trial_id = self.next_trial_id;
        self.next_trial_id = self
            .next_trial_id
            .checked_add(1)
            .ok_or("trial identifier exhausted")?;
        self.active_trial = Some(trial_id);
        let source = &self.source;
        let mut work = Work {
            attempted: 1,
            ..Default::default()
        };
        let (b, q) = match theoretical_window_bound(source, &self.y, &self.ideal_error, h, p) {
            Ok(x) => x,
            Err(e) => {
                self.abort_pending();
                return Err(e);
            }
        };
        let next = one_picard(source, &self.y, h, p, &mut work);
        self.work.exponential_calls += work.exponential_calls;
        self.work.moment_calls += work.moment_calls;
        self.work.polynomial_multiplies += work.polynomial_multiplies;
        if next.iter().any(|z| !z.is_finite()) {
            self.abort_pending();
            return Err("nonfinite proposed endpoint".into());
        }
        Ok(Attempt {
            source_token: source.token,
            trial_id,
            epoch: self.epoch,
            clock_bits: self.time.to_bits(),
            h,
            next,
            next_error_ideal: b,
            q,
            p,
            work,
        })
    }
    /// Bounded adaptive research selector. It never claims actual physical admission;
    /// rejected attempts retain their charged work and cannot mutate y,t,epoch.
    pub fn advance_adaptive(
        &mut self,
        max_h: f64,
        min_h: f64,
        p: usize,
        tol: f64,
    ) -> Result<(f64, &'static str), String> {
        if !max_h.is_finite()
            || !min_h.is_finite()
            || max_h <= 0.
            || min_h <= 0.
            || max_h < min_h
            || !tol.is_finite()
            || tol <= 0.
        {
            return Err("invalid adaptive window policy".into());
        }
        if self.source.n <= 8 {
            return Err("protected-small-n-baseline".into());
        }
        let available = self.source.horizon - self.time;
        if available < min_h {
            return Err("insufficient horizon for minimum width".into());
        }
        let mut h = max_h.min(available);
        loop {
            let trial = self.propose(h, p)?;
            let decision = self.decide(trial, tol)?;
            if decision == "RESEARCH_ACCEPTED_IDEAL_BOUND_ONLY_NOT_PRODUCTION" {
                return Ok((h, decision));
            }
            let next = h * 0.5;
            if next < min_h {
                return Err("adaptive window exhausted without acceptable ideal candidate".into());
            }
            h = next;
        }
    }
    pub fn decide(&mut self, attempt: Attempt, tol: f64) -> Result<&'static str, String> {
        if attempt.source_token != self.source.token
            || self.active_trial != Some(attempt.trial_id)
            || attempt.epoch != self.epoch
            || attempt.clock_bits != self.time.to_bits()
        {
            return Err("stale or substituted source trial".into());
        }
        if !tol.is_finite() || tol <= 0. {
            self.abort_pending();
            return Err("bad tolerance".into());
        }
        if attempt.next_error_ideal.iter().any(|v| !v.is_finite())
            || self.source.energy_upper(&attempt.next) > 1.
            || attempt.projected_ideal_bound() > tol
        {
            self.abort_pending();
            return Ok("RESEARCH_REJECTED_STATE_UNCHANGED_WORK_CHARGED");
        }
        // IMPORTANT: only research transaction; no validated binary-rounding enclosure.
        self.active_trial = None;
        self.work.pending -= 1;
        self.y = attempt.next;
        self.ideal_error = attempt.next_error_ideal;
        self.time += attempt.h;
        self.epoch += 1;
        self.work.accepted += 1;
        Ok("RESEARCH_ACCEPTED_IDEAL_BOUND_ONLY_NOT_PRODUCTION")
    }
}
// Directed one-ulp enclosures for nonnegative operands.  IEEE-754 RN, finite
// inputs, no fast-math/FTZ are required; we deliberately over-enclose even exact
// operations.  A rounded-to-zero product/division of positive reals becomes
// the smallest positive subnormal.  These helpers are NOT an enclosure of the
// complicated trigonometric/exponential kernel in one_picard().
fn up_sum(a: f64, b: f64) -> f64 {
    debug_assert!(a >= 0. && b >= 0.);
    let x = a + b;
    if x == 0. { 0. } else { x.next_up() }
}
fn up_product(a: f64, b: f64) -> f64 {
    debug_assert!(a >= 0. && b >= 0.);
    if a == 0. || b == 0. {
        0.
    } else {
        (a * b).next_up()
    }
}
fn down_product(a: f64, b: f64) -> f64 {
    debug_assert!(a >= 0. && b >= 0.);
    if a == 0. || b == 0. {
        0.
    } else {
        (a * b).next_down().max(0.)
    }
}
fn down_sum(a: f64, b: f64) -> f64 {
    debug_assert!(a >= 0. && b >= 0.);
    (a + b).next_down().max(0.)
}
fn up_quotient(a: f64, b: f64) -> f64 {
    debug_assert!(a >= 0. && b > 0.);
    if a == 0. { 0. } else { (a / b).next_up() }
}
fn up_power(x: f64, n: usize) -> f64 {
    let mut r = 1.;
    for _ in 0..n {
        r = up_product(r, x);
    }
    r
}
fn up_sum_all(xs: impl IntoIterator<Item = f64>) -> f64 {
    xs.into_iter().fold(0., up_sum)
}
fn source_mass_scale_upper(s: &Source) -> f64 {
    // The represented binary64 1/n is not exactly the mathematical 1/n.
    // n=65 and n=257 have a represented sum strictly larger than 1.
    // For phi=±1 this same upper value controls both channel norms.
    let mut out: f64 = 1.;
    for alpha in 0..2 {
        let channel = up_sum_all((0..s.n).map(|j| {
            up_product(
                s.weights[j],
                up_product(s.phi[alpha][j].abs(), s.phi[alpha][j].abs()),
            )
        }));
        out = out.max(channel);
    }
    out
}
/// Outward-rounded *algebraic metadata* for the exact-real N3 source/recurrence.
/// The returned f64 bound encloses the analytical recurrence with the represented
/// source and represented input carrier regarded as exact real numbers.  It is
/// **NOT** a certificate of the floating one_picard() output; exponential,
/// polynomial and endpoint rounding are not included in the underlying `old`.
#[allow(clippy::needless_range_loop)] // weighted rank-2 matrix entries
fn theoretical_window_bound(
    s: &Source,
    y: &[C],
    old: &[f64],
    h: f64,
    p: usize,
) -> Result<(Vec<f64>, f64), String> {
    if !h.is_finite()
        || h <= 0.
        || p > 10
        || old.len() != s.n
        || y.len() != s.n
        || old.iter().any(|x| !x.is_finite() || *x < 0.)
    {
        return Err("invalid source-locked upper-envelope input".into());
    }
    let mass = source_mass_scale_upper(s);
    // (p+1)! is an exactly represented integer for 0<=p<=10.
    let tau = up_quotient(up_power(up_product(s.omega.abs(), h), p + 1), fact(p + 1));
    let e = y
        .iter()
        .map(|x| up_sum(x.re.abs(), x.im.abs()))
        .fold(0., f64::max);
    let a: [f64; 2] = std::array::from_fn(|alpha| {
        up_sum_all((0..s.n).map(|j| {
            up_product(
                up_product(s.weights[j], s.phi[alpha][j].abs()),
                up_sum(y[j].re.abs(), y[j].im.abs()),
            )
        }))
    });
    // Exact energy conservation bounds any live moment by e*mass.  The extra
    // mass factor is needed because represented weights need not sum to one.
    let r = up_product(e, mass)
        .max(up_product(a[0], up_sum(1., tau)))
        .max(up_product(a[1], up_sum(1., tau)));
    if !r.is_finite() || r > 1. {
        return Err("no certified weighted-moment radius".into());
    }
    let qmode: Vec<f64> = (0..s.n)
        .map(|j| {
            if s.kappa[j] == 0. {
                h
            } else {
                h.min(up_quotient(1., s.kappa[j]))
            }
        })
        .collect();
    let mut qrank = [[0.; 2]; 2];
    for alpha in 0..2 {
        for beta in 0..2 {
            qrank[alpha][beta] = up_sum_all((0..s.n).map(|j| {
                up_product(
                    up_product(
                        up_product(s.weights[j], s.phi[alpha][j].abs()),
                        s.phi[beta][j].abs(),
                    ),
                    qmode[j],
                )
            }));
        }
    }
    let e3 = up_product(up_power(e, 3), up_power(mass, 3));
    let b: [f64; 2] = std::array::from_fn(|alpha| {
        up_product(
            e3,
            up_sum_all((0..2).map(|beta| up_product(s.chi[beta].abs(), qrank[alpha][beta]))),
        )
    });
    // The Lipschitz row multiplier 3*mass holds over E<=1. Upper bounds of each
    // represented product/sum are propagated without optimistic rounding.
    let row: Vec<f64> = (0..s.n)
        .map(|j| {
            up_product(
                up_product(3., mass),
                up_sum_all((0..2).map(|alpha| {
                    up_product(
                        up_product(s.chi[alpha].abs(), s.phi[alpha][j].abs()),
                        up_sum_all(
                            (0..s.n).map(|k| up_product(s.weights[k], s.phi[alpha][k].abs())),
                        ),
                    )
                })),
            )
        })
        .collect();
    let q = (0..s.n)
        .map(|j| up_product(row[j], qmode[j]))
        .fold(0., f64::max);
    if !(q.is_finite() && q < 1.) {
        return Err("source upper-bound Volterra contraction unavailable".into());
    }
    let dmax = old.iter().copied().fold(0., f64::max);
    let denominator_lower = (1. - q).next_down();
    if !(denominator_lower > 0. && denominator_lower.is_finite()) {
        return Err("source upper-bound contraction margin nonpositive".into());
    }
    let supdiff = up_quotient(dmax, denominator_lower);
    let next: Vec<f64> = (0..s.n)
        .map(|j| {
            let local = up_product(
                up_product(up_product(3., up_power(r, 2)), qmode[j]),
                up_sum_all((0..2).map(|alpha| {
                    up_product(
                        up_product(s.phi[alpha][j].abs(), s.chi[alpha].abs()),
                        up_sum(b[alpha], up_product(a[alpha], tau)),
                    )
                })),
            );
            // 1/(1+κh)>=exp(-κh) as exact real. Use a LOWER rounded denominator
            // in an upward quotient; otherwise a few-ULP false enclosure is possible.
            let decay_lower = down_sum(1., down_product(s.kappa[j], h));
            let old_transport = up_quotient(old[j], decay_lower);
            let feedback = up_product(up_product(row[j], qmode[j]), supdiff);
            up_sum(local, up_sum(old_transport, feedback))
        })
        .collect();
    if next.iter().any(|x| !x.is_finite() || *x < 0.) {
        return Err("upper envelope overflow/nonfinite".into());
    }
    Ok((next, q))
}

fn monomial_kernel(z: C, kappa_sum: f64, h: f64, degree: usize, work: &mut Work) -> Vec<C> {
    let eta = z + C::new(kappa_sum, 0.);
    let t = eta.scale(h);
    work.moment_calls += 1;
    if t.abs() < 8. {
        // Stable entire phi-family power series for small/moderate |eta*h|.
        let front = z.scale(h).exp();
        work.exponential_calls += 1;
        (0..=degree)
            .map(|ell| {
                let mut term = C::new(1. / ((ell + 1) as f64), 0.);
                let mut sum = term;
                for q in 1..140 {
                    term = term
                        * (t.scale(-1. / (q as f64)))
                        * C::new(((ell + q) as f64) / ((ell + q + 1) as f64), 0.);
                    sum = sum + term;
                    if term.abs() < 1e-18 * sum.abs().max(1e-300) {
                        break;
                    }
                }
                front * sum.scale(h.powi((ell + 1) as i32))
            })
            .collect()
    } else {
        let ez = z.scale(h).exp();
        let ek = (-kappa_sum * h).exp();
        work.exponential_calls += 2;
        let mut out = Vec::with_capacity(degree + 1);
        let mut prev = (ez - C::new(ek, 0.)) / eta;
        out.push(prev);
        for ell in 1..=degree {
            prev = (prev.scale(ell as f64) - C::new(h.powi(ell as i32) * ek, 0.)) / eta;
            out.push(prev)
        }
        out
    }
}
#[allow(clippy::needless_range_loop)] // explicit cubic cluster multi-index
fn one_picard(s: &Source, y: &[C], h: f64, p: usize, work: &mut Work) -> Vec<C> {
    let mut poly = vec![vec![vec![C::zero(); p + 1]; 2]; 2];
    for alpha in 0..2 {
        for j in 0..s.n {
            let c = if s.kappa[j] == 0.25 { 0 } else { 1 };
            let base = y[j].scale(s.weights[j] * s.phi[alpha][j]);
            let freq = C::new(0., s.omega * s.xi[j]);
            let mut power = C::new(1., 0.);
            for ell in 0..=p {
                poly[alpha][c][ell] = poly[alpha][c][ell] + base * power;
                power = power * freq.scale(1. / ((ell + 1) as f64));
            }
        }
    }
    let mut triples = Vec::new();
    for alpha in 0..2 {
        for c in 0..2 {
            for d in 0..2 {
                for e in 0..2 {
                    let tmp = poly_mul(
                        &poly[alpha][c],
                        &poly[alpha][d].iter().map(|v| v.conj()).collect::<Vec<_>>(),
                    );
                    let tmp = poly_mul(&tmp, &poly[alpha][e]);
                    work.polynomial_multiplies += 2;
                    let decay = (if c == 0 { 0.25 } else { 1000. })
                        + (if d == 0 { 0.25 } else { 1000. })
                        + (if e == 0 { 0.25 } else { 1000. });
                    triples.push((alpha, decay, tmp));
                }
            }
        }
    }
    (0..s.n)
        .map(|j| {
            let eig = C::new(-s.kappa[j], s.omega * s.xi[j]);
            let mut ans = eig.scale(h).exp() * y[j];
            work.exponential_calls += 1;
            for (alpha, decay, coeff) in &triples {
                let moments = monomial_kernel(eig, *decay, h, 3 * p, work);
                let mut val = C::zero();
                for (q, c) in coeff.iter().enumerate() {
                    val = val + *c * moments[q];
                }
                ans = ans + C::new(0., s.phi[*alpha][j] * s.chi[*alpha]) * val;
            }
            ans
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn good() -> Session {
        Session::new(Source::toy(12).unwrap()).unwrap()
    }
    #[test]
    fn rust_toolchain_contract_source_and_energy() {
        let s = Source::toy(24).unwrap();
        assert!(s.energy(&s.initial()) < 1.);
        let y = s.initial();
        let f = s.rhs(&y);
        let real = (0..s.n)
            .map(|j| 2. * s.weights[j] * (y[j].re * f[j].re + y[j].im * f[j].im))
            .sum::<f64>();
        let expected = (0..s.n)
            .map(|j| -2. * s.weights[j] * s.kappa[j] * (y[j].re * y[j].re + y[j].im * y[j].im))
            .sum::<f64>();
        assert!(
            (real - expected).abs() < 2e-14,
            "energy {real} != {expected}"
        );
    }
    #[test]
    fn accepted_two_windows_stiff_no_microstep() {
        let mut sess = good();
        for _ in 0..2 {
            let a = sess.propose(0.25, 2).unwrap();
            assert!(a.projected_ideal_bound() < 5e-8);
            assert!(a.q() < 0.1);
            assert_eq!(
                sess.decide(a, 5e-8).unwrap(),
                "RESEARCH_ACCEPTED_IDEAL_BOUND_ONLY_NOT_PRODUCTION"
            );
        }
        assert_eq!(sess.epoch(), 2);
        assert!((sess.time() - 0.5).abs() < 1e-15);
        assert_eq!(sess.work().accepted, 2);
        assert_eq!(sess.work().attempted, 2);
        assert!(sess.work().moment_calls > 0);
        assert!(sess.state().iter().all(|v| v.is_finite()));
    }
    #[test]
    fn malformed_proposal_does_not_change_state() {
        let mut s = good();
        let state = s.state().to_vec();
        for h in [0., -1., f64::INFINITY, f64::NAN, 0.6] {
            assert!(s.propose(h, 2).is_err())
        }
        assert_eq!(s.state(), state.as_slice());
        assert_eq!(s.work().attempted, 0);
    }
    #[test]
    fn rejected_attempt_charged_and_no_mutation() {
        let mut s = good();
        let before = s.state().to_vec();
        let a = s.propose(0.25, 0).unwrap();
        assert!(s.decide(a, 1e-30).unwrap().starts_with("RESEARCH_REJECTED"));
        assert_eq!(s.state(), before.as_slice());
        assert_eq!(s.time(), 0.);
        assert_eq!(s.epoch(), 0);
        assert_eq!(s.work().rejected, 1);
        assert_eq!(s.work().attempted, 1)
    }
    #[test]
    fn replay_and_cross_source_forgery_rejected() {
        let mut a = good();
        let mut b = good();
        let trial = a.propose(0.25, 2).unwrap();
        let y = b.state().to_vec();
        let work = b.work();
        assert!(b.decide(trial, 1.).is_err());
        assert_eq!(b.state(), y.as_slice());
        assert_eq!(b.work(), work);
    }
    #[test]
    fn stale_trial_rejected() {
        let mut s = good();
        let stale = s.propose(0.25, 2).unwrap();
        let trial = s.propose(0.25, 2).unwrap();
        assert!(s.decide(stale, 1.).is_err());
        s.decide(trial, 1.).unwrap();
        assert_eq!(s.epoch(), 1);
        assert_eq!(s.work().rejected, 1);
        assert!(s.is_balanced());
    }
    #[test]
    fn small_n_protected_no_speculative_work() {
        let mut s = Session::new(Source::toy(8).unwrap()).unwrap();
        assert!(s.propose(0.25, 2).unwrap_err().contains("small"));
        assert_eq!(s.work(), Work::default());
    }
    #[test]
    fn odd_initial_modes_stored() {
        let s = good();
        let y = s.state();
        assert_ne!(y[0].im, y[1].im);
        assert_ne!(y[0].re, y[2].re)
    }
    #[test]
    fn frozen_source_no_alias_to_accepted_carrier() {
        let mut s = good();
        let frozen = s.state().to_vec();
        let _ = s.propose(0.25, 2).unwrap();
        assert_eq!(frozen, s.state());
        assert_eq!(s.epoch(), 0)
    }
    #[test]
    fn exact_nonlinear_sign_involution() {
        let u = Source::toy_sign(12, 1.).unwrap();
        let v = Source::toy_sign(12, -1.).unwrap();
        let y = u.initial();
        let fu = u.rhs(&y);
        let fv = v.rhs(&y);
        for (j, k) in fu.iter().zip(fv.iter()) {
            assert!((*j - *k).abs() < 2e-16)
        }
        let mut a = Session::new(u).unwrap();
        let mut b = Session::new(v).unwrap();
        let x = a.propose(0.25, 2).unwrap();
        let y = b.propose(0.25, 2).unwrap();
        a.decide(x, 1.).unwrap();
        b.decide(y, 1.).unwrap();
        for (j, k) in a.state().iter().zip(b.state()) {
            assert!((*j - *k).abs() < 1e-14)
        }
    }
    #[test]
    #[allow(clippy::needless_range_loop)]
    fn zero_coupling_exact_damped_oscillation() {
        let source = Source::toy(12).unwrap();
        let mut test = Source::toy(12).unwrap();
        let v = Arc::get_mut(&mut test).unwrap();
        v.chi = [0., 0.];
        let mut sess = Session::new(test).unwrap();
        let start = sess.state().to_vec();
        let proposal = sess.propose(0.25, 0).unwrap();
        assert_eq!(proposal.projected_ideal_bound(), 0.);
        sess.decide(proposal, 1e-18).unwrap();
        for j in 0..12 {
            let z = C::new(-source.kappa[j], source.omega * source.xi[j])
                .scale(0.25)
                .exp()
                * start[j];
            assert!((z - sess.state()[j]).abs() < 1e-16)
        }
    }
    #[test]
    fn last_time_window_refused() {
        let mut s = good();
        for _ in 0..2 {
            let a = s.propose(0.25, 2).unwrap();
            s.decide(a, 1.).unwrap();
        }
        assert!(s.propose(0.01, 2).is_err());
    }
    #[test]
    fn monotone_work_ledger() {
        let mut s = good();
        let a = s.propose(0.25, 0).unwrap();
        s.decide(a, 1e-30).unwrap();
        let b = s.propose(0.25, 2).unwrap();
        s.decide(b, 1.).unwrap();
        let w = s.work();
        assert_eq!(w.attempted, w.accepted + w.rejected);
        assert_eq!(w.accepted, 1);
        assert_eq!(w.rejected, 1);
    }
    #[test]
    fn infinity_and_nonphysical_carrier_refused() {
        let src = Source::toy(12).unwrap();
        let mut init = src.initial();
        init[3].re = f64::NAN;
        assert!(Session::from_initial(src.clone(), init).is_err());
        let mut large = src.initial();
        large[0].re = 100.;
        assert!(Session::from_initial(src, large).is_err());
    }
    #[test]
    fn pending_is_explicit_and_abort_preserves_work() {
        let mut s = good();
        let prev = s.state().to_vec();
        let _unsubmitted = s.propose(0.25, 2).unwrap();
        assert_eq!(s.work().pending, 1);
        assert!(s.is_balanced());
        s.abort_pending();
        assert_eq!(s.work().pending, 0);
        assert_eq!(s.work().rejected, 1);
        assert_eq!(s.state(), prev.as_slice());
        assert!(s.is_balanced());
    }
    #[test]
    fn invalid_tolerance_rejects_and_charges() {
        let mut s = good();
        let trial = s.propose(0.25, 2).unwrap();
        assert!(s.decide(trial, f64::NAN).is_err());
        assert_eq!(s.work().rejected, 1);
        assert_eq!(s.work().pending, 0);
        assert!(s.is_balanced());
    }
    #[test]
    fn detector_sees_genuine_two_rate_stiff_window() {
        let source = Source::toy(65).unwrap();
        assert_eq!(source.window_mode_counts(0.25).unwrap(), (33, 32));
        assert_eq!(source.window_mode_counts(0.00001).unwrap(), (65, 0));
        assert!(source.window_mode_counts(f64::NAN).is_err());
    }
    #[test]
    fn adaptive_rejection_then_acceptance_carries_attempts() {
        let mut s = good();
        let initial = s.state().to_vec();
        let (h, status) = s.advance_adaptive(0.5, 0.03125, 2, 1e-10).unwrap();
        assert!((0.03125..0.5).contains(&h));
        assert_eq!(status, "RESEARCH_ACCEPTED_IDEAL_BOUND_ONLY_NOT_PRODUCTION");
        assert_eq!(s.time(), h);
        assert_ne!(s.state(), initial.as_slice());
        assert!(s.work().rejected >= 1);
        assert_eq!(s.work().accepted, 1);
        assert!(s.is_balanced());
    }
    #[test]
    fn adaptive_exhaustion_is_fail_closed_with_charged_work() {
        let mut s = good();
        let y = s.state().to_vec();
        assert!(s.advance_adaptive(0.25, 0.125, 2, 1e-25).is_err());
        assert_eq!(s.time(), 0.);
        assert_eq!(s.epoch(), 0);
        assert_eq!(s.state(), y.as_slice());
        assert!(s.work().rejected >= 1);
        assert_eq!(s.work().accepted, 0);
        assert!(s.is_balanced());
    }
    #[test]
    fn odd_n_modes_are_not_deleted() {
        let mut s = Session::new(Source::toy(65).unwrap()).unwrap();
        let a = s.propose(0.25, 2).unwrap();
        assert_eq!(a.next.len(), 65);
        s.decide(a, 1.).unwrap();
        assert_eq!(s.state().len(), 65);
        assert!(s.work().exponential_calls > 0);
    }
}

#[cfg(test)]
mod represented_upper_tests {
    use super::*;

    #[test]
    fn actual_cpu_round_nearest_and_gradual_subnormals() {
        let one = std::hint::black_box(1.0_f64);
        let half_ulp = std::hint::black_box(f64::EPSILON / 2.0);
        let sub = std::hint::black_box(f64::from_bits(1));
        let two = std::hint::black_box(2.0);
        assert_eq!((one + half_ulp).to_bits(), one.to_bits());
        assert_eq!((sub * two).to_bits(), 2);
        assert_eq!((sub * one).to_bits(), 1);
    }
    #[test]
    fn directed_primitives_enclose_nonnegative_underflow_and_zero() {
        assert_eq!(up_sum(0., 0.), 0.);
        assert_eq!(up_product(0., 1.), 0.);
        assert_eq!(up_quotient(0., 1.), 0.);
        assert_eq!(down_product(1e-300, 1e-300), 0.);
        assert_eq!(up_product(1e-300, 1e-300), f64::from_bits(1));
        assert_eq!(up_quotient(f64::from_bits(1), 1e300), f64::from_bits(1));
        assert!(up_sum(0.1, 0.2) > 0.3);
        assert!(down_sum(0.1, 0.2) < 0.1 + 0.2);
    }

    #[test]
    fn represented_weights_not_incorrectly_assumed_exact_1_over_n() {
        for n in [65, 257] {
            let source = Source::toy(n).unwrap();
            assert!(source_mass_scale_upper(&source) > 1.);
            // Native f64 accumulation can round the exact represented weighted
            // energy of n unit-amplitude modes to one.  The corrected upper
            // domain gate MUST refuse it, including without any trial.
            assert!(Session::from_initial(source, vec![C::new(1., 0.); n]).is_err());
        }
        let source = Source::toy(24).unwrap();
        assert!(source_mass_scale_upper(&source) >= 1.);
    }

    #[test]
    fn exact_source_upper_metadata_keeps_research_only_status_and_transactions() {
        for n in [12, 24, 65, 257] {
            let mut session = Session::new(Source::toy(n).unwrap()).unwrap();
            let candidate = session.propose(0.25, 2).unwrap();
            assert!(candidate.q() >= 0.0524 && candidate.q() < 0.0526);
            assert!(candidate.projected_ideal_bound().is_finite());
            let tol = candidate.projected_ideal_bound();
            assert_eq!(
                session.decide(candidate, tol).unwrap(),
                "RESEARCH_ACCEPTED_IDEAL_BOUND_ONLY_NOT_PRODUCTION"
            );
            assert!(session.is_balanced());
        }
    }

    #[test]
    fn reject_tolerance_below_upward_envelope_without_changing_source() {
        let mut session = Session::new(Source::toy(12).unwrap()).unwrap();
        let old = session.state().to_vec();
        let candidate = session.propose(0.25, 2).unwrap();
        let strict = candidate.projected_ideal_bound().next_down();
        assert_eq!(
            session.decide(candidate, strict).unwrap(),
            "RESEARCH_REJECTED_STATE_UNCHANGED_WORK_CHARGED"
        );
        assert_eq!(session.state(), old);
        assert_eq!(session.epoch(), 0);
        assert_eq!(session.work().rejected, 1);
        assert!(session.is_balanced());
    }
}

/// N4 research export only: original N3H monomial_kernel, with exactly the
/// same numerical code path, no modification to N3H original source/archive.
pub fn n4_compare_original_monomial_kernel(a:f64,b:f64,theta:f64,h:f64,r:usize)->(f64,f64){
    let mut work=Work::default();
    let values=monomial_kernel(C::new(-a,theta),b,h,r,&mut work);
    (values[r].re,values[r].im)
}
