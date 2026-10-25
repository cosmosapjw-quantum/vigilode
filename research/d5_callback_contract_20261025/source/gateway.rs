//! Opt-in strict full-write callback boundary; no solver or retry loop.
//! The native previous-matrix/fixed-sparsity Jacobian contract is unchanged.
//! A probe is a caller assertion: honest epoch/environment reporting, no epoch
//! ABA, and finite callback/probe/destructor return remain explicit assumptions.
//! Owned scratch isolates returned values, not captures or external effects.
#![forbid(unsafe_code)]

use std::panic::{catch_unwind, AssertUnwindSafe};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallbackFault { Recoverable, Fatal }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    InvalidDomain,
    OutsideDomain,
    InvalidDifferenceStep,
    RecoverableCallback,
    FatalCallback,
    CallbackPanic,
    ProbePanic,
    EpochMismatch,
    Environment,
    NonFiniteOutput,
    CounterOverflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeWitness { pub epoch: u64, pub environment_ok: bool }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counters { pub rhs: u64, pub jac: u64, pub pairs: u64 }

/// A validated finite inclusive box. Its fields cannot be changed after creation.
#[derive(Clone, Copy, Debug)]
pub struct BoxDomain<const N: usize> {
    t_lo: f64,
    t_hi: f64,
    y_lo: [f64; N],
    y_hi: [f64; N],
}

// Binary64 numeric order for finite values, independent of FP control modes.
// Signed zeros compare equal; the query bits themselves are never rewritten.
fn finite_key(value: f64) -> u64 {
    let bits = value.to_bits();
    let bits = if bits << 1 == 0 { 0 } else { bits };
    if bits >> 63 == 1 { !bits } else { bits ^ (1_u64 << 63) }
}

impl<const N: usize> BoxDomain<N> {
    pub fn new(t_lo: f64, t_hi: f64, y_lo: [f64; N], y_hi: [f64; N]) -> Result<Self, Fault> {
        if N == 0 || !t_lo.is_finite() || !t_hi.is_finite() || finite_key(t_lo) > finite_key(t_hi)
            || (0..N).any(|i| !y_lo[i].is_finite() || !y_hi[i].is_finite() || finite_key(y_lo[i]) > finite_key(y_hi[i]))
        { return Err(Fault::InvalidDomain); }
        Ok(Self { t_lo, t_hi, y_lo, y_hi })
    }

    fn check(&self, t: f64, y: &[f64; N]) -> Result<(), Fault> {
        if !t.is_finite() || finite_key(t) < finite_key(self.t_lo) || finite_key(t) > finite_key(self.t_hi)
            || (0..N).any(|i| !y[i].is_finite() || finite_key(y[i]) < finite_key(self.y_lo[i]) || finite_key(y[i]) > finite_key(self.y_hi[i]))
        { return Err(Fault::OutsideDomain); }
        Ok(())
    }
}

/// Prepared point values, not a Jacobian-consistency or dense-domain certificate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Prepared<const N: usize> {
    pub t: f64,
    pub y: [f64; N],
    pub epoch: u64,
    pub rhs: [f64; N],
    pub jacobian: [[f64; N]; N],
}

#[derive(Debug)]
pub struct Gate<const N: usize> {
    domain: BoxDomain<N>,
    expected_epoch: u64,
    counters: Counters,
    terminal: Option<Fault>,
}

impl<const N: usize> Gate<N> {
    pub fn new(domain: BoxDomain<N>, expected_epoch: u64) -> Self {
        Self { domain, expected_epoch, counters: Counters::default(), terminal: None }
    }
    pub fn counters(&self) -> Counters { self.counters }
    pub fn terminal(&self) -> Option<Fault> { self.terminal }

    fn ready(&self) -> Result<(), Fault> {
        match self.terminal { Some(fault) => Err(fault), None => Ok(()) }
    }

    fn poison<T>(&mut self, fault: Fault) -> Result<T, Fault> {
        Err(*self.terminal.get_or_insert(fault))
    }

    fn witness<W: FnMut() -> RuntimeWitness>(&mut self, probe: &mut W) -> Result<(), Fault> {
        let observed = match catch_unwind(AssertUnwindSafe(probe)) {
            Ok(value) => value,
            Err(_) => return self.poison(Fault::ProbePanic),
        };
        if observed.epoch != self.expected_epoch { return self.poison(Fault::EpochMismatch); }
        if !observed.environment_ok { return self.poison(Fault::Environment); }
        Ok(())
    }

    fn before<W: FnMut() -> RuntimeWitness>(
        &mut self, t: f64, y: &[f64; N], probe: &mut W, rhs: bool,
    ) -> Result<(), Fault> {
        self.ready()?;
        self.domain.check(t, y)?;
        self.witness(probe)?;
        let count = if rhs { &mut self.counters.rhs } else { &mut self.counters.jac };
        match count.checked_add(1) {
            Some(next) => { *count = next; Ok(()) }
            None => self.poison(Fault::CounterOverflow),
        }
    }

    // A post witness is checked even after callback Err or a handled unwind.
    // Its fault takes precedence over callback status and finite-output checks.
    fn after<W: FnMut() -> RuntimeWitness>(
        &mut self, result: std::thread::Result<Result<(), CallbackFault>>, probe: &mut W,
    ) -> Result<(), Fault> {
        self.witness(probe)?;
        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(CallbackFault::Recoverable)) => Err(Fault::RecoverableCallback),
            Ok(Err(CallbackFault::Fatal)) => self.poison(Fault::FatalCallback),
            Err(_) => self.poison(Fault::CallbackPanic),
        }
    }

    pub fn eval_rhs<W, F>(
        &mut self, t: f64, y: &[f64; N], probe: &mut W, rhs: &mut F,
    ) -> Result<[f64; N], Fault>
    where W: FnMut() -> RuntimeWitness,
          F: FnMut(f64, &[f64; N], &mut [f64; N]) -> Result<(), CallbackFault>,
    {
        self.before(t, y, probe, true)?;
        let mut scratch = [f64::NAN; N];
        let result = catch_unwind(AssertUnwindSafe(|| rhs(t, y, &mut scratch)));
        self.after(result, probe)?;
        if scratch.iter().any(|value| !value.is_finite()) {
            return self.poison(Fault::NonFiniteOutput);
        }
        Ok(scratch)
    }

    pub fn eval_jacobian<W, J>(
        &mut self, t: f64, y: &[f64; N], probe: &mut W, jacobian: &mut J,
    ) -> Result<[[f64; N]; N], Fault>
    where W: FnMut() -> RuntimeWitness,
          J: FnMut(f64, &[f64; N], &mut [[f64; N]; N]) -> Result<(), CallbackFault>,
    {
        self.before(t, y, probe, false)?;
        let mut scratch = [[f64::NAN; N]; N];
        let result = catch_unwind(AssertUnwindSafe(|| jacobian(t, y, &mut scratch)));
        self.after(result, probe)?;
        if scratch.iter().flatten().any(|value| !value.is_finite()) {
            return self.poison(Fault::NonFiniteOutput);
        }
        Ok(scratch)
    }

    pub fn prepare_pair<W, F, J>(
        &mut self, t: f64, y: &[f64; N], probe: &mut W, rhs: &mut F, jacobian: &mut J,
    ) -> Result<Prepared<N>, Fault>
    where W: FnMut() -> RuntimeWitness,
          F: FnMut(f64, &[f64; N], &mut [f64; N]) -> Result<(), CallbackFault>,
          J: FnMut(f64, &[f64; N], &mut [[f64; N]; N]) -> Result<(), CallbackFault>,
    {
        let rhs = self.eval_rhs(t, y, probe, rhs)?;
        let jacobian = self.eval_jacobian(t, y, probe, jacobian)?;
        let Some(next) = self.counters.pairs.checked_add(1) else {
            return self.poison(Fault::CounterOverflow);
        };
        self.counters.pairs = next;
        Ok(Prepared { t, y: *y, epoch: self.expected_epoch, rhs, jacobian })
    }

    /// Guarded point estimate only; it supplies no truncation/rounding enclosure.
    /// Both represented probe times are preflighted before either RHS dispatch.
    pub fn estimate_partial_t_centered<W, F>(
        &mut self, t: f64, y: &[f64; N], eps: f64, probe: &mut W, rhs: &mut F,
    ) -> Result<[f64; N], Fault>
    where W: FnMut() -> RuntimeWitness,
          F: FnMut(f64, &[f64; N], &mut [f64; N]) -> Result<(), CallbackFault>,
    {
        self.ready()?;
        self.domain.check(t, y)?;
        if !eps.is_finite() || eps <= 0.0 { return Err(Fault::InvalidDifferenceStep); }
        // The represented stencil itself must use an admitted FP environment.
        self.witness(probe)?;
        let t_minus = t - eps;
        let t_plus = t + eps;
        let denominator = t_plus - t_minus;
        if !t_minus.is_finite() || !t_plus.is_finite() || !(t_minus < t && t < t_plus)
            || !denominator.is_finite() || denominator <= 0.0
        { return Err(Fault::InvalidDifferenceStep); }
        self.domain.check(t_minus, y)?;
        self.domain.check(t_plus, y)?;
        let minus = self.eval_rhs(t_minus, y, probe, rhs)?;
        let plus = self.eval_rhs(t_plus, y, probe, rhs)?;
        let estimate = std::array::from_fn(|i| (plus[i] - minus[i]) / denominator);
        if estimate.iter().any(|value| !value.is_finite()) {
            return self.poison(Fault::NonFiniteOutput);
        }
        Ok(estimate)
    }

    #[cfg(test)]
    pub(crate) fn seed_counters_for_test(&mut self, counters: Counters) {
        self.counters = counters;
    }
}
