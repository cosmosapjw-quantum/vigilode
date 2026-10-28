//! Opt-in, fixed-model original-equation RVJ closure certificates.
//!
//! This module admits only the cyclic quadratic model documented in
//! `research/production_rvj_nag_20261028/theory/CLOSURE.md`. It does not
//! change the generic solver dispatcher. The certified dense path uses exact
//! original-field slopes at represented endpoints; it is not a cubic made
//! from rounded integration slopes. Every original slow and algebraic
//! coordinate is covered over each whole physical slab.
//!
//! The caller supplies the valid incoming slow radius. A slab receipt alone
//! does not certify an unrelated initial condition or a discontinuous chain.
//! [`integrate_rk4`] generates the fixed zero-initial-data candidate on `[0,1]`.
//! Arithmetic inherits `rodas5p_core::directed`'s IEEE round-to-nearest,
//! gradual-underflow and compiler-transform assumptions.

use rodas5p_core::{
    CoreError, CoreResult,
    directed::{self, Interval},
};
use serde::Serialize;

pub const MODEL_ID: &str = "rvj-cyclic-quarter-square-v1";
pub const BRANCH_ID: &str = "cyclic-small-real-zero";
pub const PATH_DEFINITION: &str = "exact-original-slope-hermite-v1";
/// Display value only; admission uses the exact rational 1/100000000.
pub const PHYSICAL_CAP: f64 = 1.0e-8;
const ZERO: Interval = Interval { lo: 0.0, hi: 0.0 };
const ONE: Interval = Interval { lo: 1.0, hi: 1.0 };

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(message.into())
}

fn point(x: f64) -> CoreResult<Interval> {
    Interval::point(x)
}

fn ratio(numerator: f64, denominator: f64) -> CoreResult<Interval> {
    point(numerator)?.div(point(denominator)?)
}

fn constant(i: usize) -> f64 {
    1.0 / 32.0 + (i % 3) as f64 / 256.0
}

/// Conservative comparison with the exact rational cap 1/100000000.
/// Comparing only with the rounded `1e-8` literal could admit the next tiny
/// interval above the requested mathematical cap.
pub fn meets_physical_cap_bound(bound: f64) -> bool {
    bound.is_finite()
        && bound >= 0.0
        && directed::mul_up(bound, 100_000_000.0).is_ok_and(|scaled| scaled <= 1.0)
}

/// Counters charge attempted work, including work preceding an error.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct CyclicWork {
    pub candidate_rhs_evaluations: u64,
    pub candidate_cyclic_solves: u64,
    pub interval_slope_evaluations: u64,
    pub interval_cyclic_solves: u64,
    pub endpoint_original_residual_evaluations: u64,
    pub endpoint_reuses: u64,
    pub slab_attempts: u64,
    pub slab_certificates: u64,
    pub power_to_bernstein_transforms: u64,
    pub polynomial_convolution_products: u64,
    pub direct_slow_components: u64,
    pub direct_algebraic_components: u64,
    pub jacobian_callbacks: u64,
    pub jvp_callbacks: u64,
    pub ad_callbacks: u64,
    pub newton_updates: u64,
    pub state_matrix_factorizations: u64,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Certifier {
    HistoricalPower,
    DirectBernstein,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RadiusRecipe {
    Legacy,
    Tight,
}

/// Immutable identity for this fixed autonomous model. Epoch changes
/// invalidate prepared endpoints even though the fixed equations are equal.
#[derive(Clone, Debug)]
pub struct CyclicModel {
    dimension: usize,
    epoch: u64,
}

impl CyclicModel {
    pub fn new(dimension: usize, epoch: u64) -> CoreResult<Self> {
        if !matches!(dimension, 4 | 8 | 16) {
            return Err(invalid("cyclic closure admits only n=4,8,16"));
        }
        Ok(Self { dimension, epoch })
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    fn validate_state(&self, state: &[f64]) -> CoreResult<()> {
        if state.len() != 2 * self.dimension {
            return Err(invalid("cyclic endpoint dimension mismatch"));
        }
        for (i, &value) in state.iter().enumerate() {
            let limit = if i < self.dimension { 0.125 } else { 0.25 };
            if !value.is_finite() || value.abs() > limit {
                return Err(invalid(
                    "cyclic endpoint is nonfinite or outside the fixed domain",
                ));
            }
        }
        Ok(())
    }

    /// Prepare both exact-field slopes and original G for the direct kernel.
    pub fn prepare_endpoint(
        &self,
        state: &[f64],
        work: &mut CyclicWork,
    ) -> CoreResult<PreparedEndpoint> {
        self.prepare(state, true, work)
    }

    /// Historical certifier preparation, without unused endpoint-G work.
    /// Keeping this distinct avoids artificially inflating the comparator.
    pub fn prepare_reference_endpoint(
        &self,
        state: &[f64],
        work: &mut CyclicWork,
    ) -> CoreResult<PreparedEndpoint> {
        self.prepare(state, false, work)
    }

    fn prepare(
        &self,
        state: &[f64],
        include_residual: bool,
        work: &mut CyclicWork,
    ) -> CoreResult<PreparedEndpoint> {
        self.validate_state(state)?;
        let values = state
            .iter()
            .copied()
            .map(point)
            .collect::<CoreResult<Vec<_>>>()?;
        work.interval_slope_evaluations += 1;
        let (slopes, denominator) = interval_lift(&values, self.dimension, work)?;
        let original_residual = if include_residual {
            work.endpoint_original_residual_evaluations += 1;
            let mut residual = Vec::with_capacity(self.dimension);
            for i in 0..self.dimension {
                let q = values[self.dimension + (i + 1) % self.dimension];
                residual.push(
                    values[self.dimension + i]
                        .add(q.mul(q)?.scale(0.25)?)?
                        .sub(values[i])?,
                );
            }
            Some(residual)
        } else {
            None
        };
        Ok(PreparedEndpoint {
            dimension: self.dimension,
            epoch: self.epoch,
            bits: state.iter().map(|x| x.to_bits()).collect(),
            values,
            slopes,
            original_residual,
            denominator,
        })
    }

    /// Explicitly charge a verified reuse. Approximate equality is rejected.
    pub fn reuse_endpoint(
        &self,
        prepared: &PreparedEndpoint,
        state: &[f64],
        work: &mut CyclicWork,
    ) -> CoreResult<()> {
        if !prepared.matches(self, state) {
            return Err(invalid("cyclic endpoint reuse changed bits or model epoch"));
        }
        work.endpoint_reuses += 1;
        Ok(())
    }
}

/// Fields are private so a caller cannot attach an unrelated slope enclosure
/// or claim the analytic endpoint identities for another field.
#[derive(Clone, Debug)]
pub struct PreparedEndpoint {
    dimension: usize,
    epoch: u64,
    bits: Vec<u64>,
    values: Vec<Interval>,
    slopes: Vec<Interval>,
    original_residual: Option<Vec<Interval>>,
    denominator: Interval,
}

impl PreparedEndpoint {
    pub fn matches(&self, model: &CyclicModel, state: &[f64]) -> bool {
        self.dimension == model.dimension
            && self.epoch == model.epoch
            && state.len() == self.bits.len()
            && state
                .iter()
                .zip(&self.bits)
                .all(|(value, bits)| value.to_bits() == *bits)
    }

    pub fn endpoint_bits(&self) -> &[u64] {
        &self.bits
    }

    pub fn circuit_denominator(&self) -> [f64; 2] {
        self.denominator.as_pair()
    }
}

/// The exact represented physical grid t=k/N, 0<=k<=N.
#[derive(Clone, Copy, Debug)]
pub struct DyadicGrid {
    steps: usize,
    h: f64,
}

impl DyadicGrid {
    pub fn new(steps: usize) -> CoreResult<Self> {
        // Every k must convert to binary64 exactly. This is an arithmetic
        // representability limit, not an accuracy or tuning parameter.
        if !steps.is_power_of_two() || steps as u128 > (1_u128 << 53) {
            return Err(invalid("grid requires a power-of-two N <= 2^53"));
        }
        Ok(Self {
            steps,
            h: 1.0 / steps as f64,
        })
    }

    pub fn steps(&self) -> usize {
        self.steps
    }
    pub fn h(&self) -> f64 {
        self.h
    }

    pub fn slab(&self, index: usize) -> CoreResult<SlabClock> {
        if index >= self.steps {
            return Err(invalid("slab index outside physical grid"));
        }
        Ok(SlabClock {
            index,
            t0: index as f64 * self.h,
            t1: (index + 1) as f64 * self.h,
            h: self.h,
        })
    }

    /// Validate all three supplied labels. Agreement of only t1-t0 or a
    /// rounded cumulative time does not establish the physical clock.
    pub fn checked_slab(&self, index: usize, t0: f64, t1: f64, h: f64) -> CoreResult<SlabClock> {
        let expected = self.slab(index)?;
        if t0.to_bits() != expected.t0.to_bits()
            || t1.to_bits() != expected.t1.to_bits()
            || h.to_bits() != expected.h.to_bits()
        {
            return Err(invalid(
                "physical clock labels disagree with the exact dyadic grid",
            ));
        }
        Ok(expected)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SlabClock {
    index: usize,
    t0: f64,
    t1: f64,
    h: f64,
}

impl SlabClock {
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn t0(&self) -> f64 {
        self.t0
    }
    pub fn t1(&self) -> f64 {
        self.t1
    }
    pub fn h(&self) -> f64 {
        self.h
    }
}

fn slow_interval(state: &[Interval], n: usize) -> CoreResult<Vec<Interval>> {
    (0..n)
        .map(|i| {
            state[i]
                .scale(-1.0)?
                .add(
                    state[n + i]
                        .add(state[n + (i + 1) % n])?
                        .scale(1.0 / 16.0)?,
                )?
                .add(point(constant(i))?)
        })
        .collect()
}

fn interval_lift(
    state: &[Interval],
    n: usize,
    work: &mut CyclicWork,
) -> CoreResult<(Vec<Interval>, Interval)> {
    let f = slow_interval(state, n)?;
    let b = (0..n)
        .map(|i| state[n + (i + 1) % n].scale(0.5))
        .collect::<CoreResult<Vec<_>>>()?;
    work.interval_cyclic_solves += 1;
    let mut r = f[n - 1];
    let mut p = b[n - 1].scale(-1.0)?;
    for i in (0..n - 1).rev() {
        r = f[i].sub(b[i].mul(r)?)?;
        p = b[i].scale(-1.0)?.mul(p)?;
    }
    let denominator = ONE.sub(p)?;
    if denominator.lo <= 0.0 {
        return Err(invalid("cyclic circuit denominator not proved positive"));
    }
    let mut v = vec![ZERO; n];
    v[0] = r.div(denominator)?;
    v[n - 1] = f[n - 1].sub(b[n - 1].mul(v[0])?)?;
    for i in (1..n - 1).rev() {
        v[i] = f[i].sub(b[i].mul(v[i + 1])?)?;
    }
    let mut full = f;
    full.extend(v);
    Ok((full, denominator))
}

// Preserve the recurrence's written operation tree and operand order.
#[allow(clippy::assign_op_pattern)]
fn lifted_rhs(model: &CyclicModel, state: &[f64], work: &mut CyclicWork) -> CoreResult<Vec<f64>> {
    work.candidate_rhs_evaluations += 1;
    model.validate_state(state)?;
    let n = model.dimension;
    let f: Vec<f64> = (0..n)
        .map(|i| -state[i] + (state[n + i] + state[n + (i + 1) % n]) / 16.0 + constant(i))
        .collect();
    let b: Vec<f64> = (0..n).map(|i| state[n + (i + 1) % n] * 0.5).collect();
    work.candidate_cyclic_solves += 1;
    let mut r = f[n - 1];
    let mut p = -b[n - 1];
    for i in (0..n - 1).rev() {
        r = f[i] - b[i] * r;
        p = -b[i] * p;
    }
    let denominator = 1.0 - p;
    if !denominator.is_finite() || denominator <= 0.0 {
        return Err(invalid("candidate circuit denominator invalid"));
    }
    let mut v = vec![0.0; n];
    v[0] = r / denominator;
    v[n - 1] = f[n - 1] - b[n - 1] * v[0];
    for i in (1..n - 1).rev() {
        v[i] = f[i] - b[i] * v[i + 1];
    }
    let mut full = f;
    full.extend(v);
    if full.iter().any(|value| !value.is_finite()) {
        return Err(invalid("candidate field produced a nonfinite value"));
    }
    Ok(full)
}

/// Fixed dyadic RK4 candidate, x(0)=q(0)=0 and physical t in `[0,1]`.
/// Returned endpoints require a separate original-equation certificate.
pub fn integrate_rk4(
    model: &CyclicModel,
    grid: &DyadicGrid,
    work: &mut CyclicWork,
) -> CoreResult<Vec<Vec<f64>>> {
    let n2 = 2 * model.dimension;
    let capacity = grid
        .steps
        .checked_add(1)
        .ok_or_else(|| invalid("trajectory length overflow"))?;
    let mut states = Vec::new();
    states
        .try_reserve_exact(capacity)
        .map_err(|_| invalid("trajectory allocation exceeds available resources"))?;
    let mut y = vec![0.0; n2];
    states.push(y.clone());
    let h = grid.h;
    for _ in 0..grid.steps {
        let k1 = lifted_rhs(model, &y, work)?;
        let mut temporary: Vec<f64> = (0..n2).map(|i| y[i] + h * k1[i] / 2.0).collect();
        let k2 = lifted_rhs(model, &temporary, work)?;
        for i in 0..n2 {
            temporary[i] = y[i] + h * k2[i] / 2.0;
        }
        let k3 = lifted_rhs(model, &temporary, work)?;
        for i in 0..n2 {
            temporary[i] = y[i] + h * k3[i];
        }
        let k4 = lifted_rhs(model, &temporary, work)?;
        for i in 0..n2 {
            y[i] += h * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]) / 6.0;
        }
        model.validate_state(&y)?;
        states.push(y.clone());
    }
    Ok(states)
}

#[derive(Clone, Debug, Serialize)]
pub struct CoefficientDiagnostics {
    /// Component, Bernstein coefficient, `[lower, upper]`.
    pub path_bernstein: Vec<[[f64; 2]; 4]>,
    pub slow_bernstein: Vec<[[f64; 2]; 4]>,
    pub algebraic_bernstein: Vec<[[f64; 2]; 7]>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SlabCertificate {
    pub model_id: &'static str,
    pub model_epoch: u64,
    pub branch_id: &'static str,
    pub path_definition: &'static str,
    pub physical_norm: &'static str,
    pub certifier: Certifier,
    pub radius_recipe: RadiusRecipe,
    pub index: usize,
    pub t0: f64,
    pub t1: f64,
    pub h: f64,
    pub e0: f64,
    pub df: f64,
    pub dg: f64,
    pub ew: f64,
    pub ee: f64,
    pub qw: f64,
    pub diagnostics: Option<CoefficientDiagnostics>,
}

impl SlabCertificate {
    pub fn meets_physical_cap(&self) -> bool {
        meets_physical_cap_bound(self.ew.max(self.qw))
    }
}

fn validate_pair(
    model: &CyclicModel,
    left: &PreparedEndpoint,
    right: &PreparedEndpoint,
) -> CoreResult<()> {
    if left.dimension != model.dimension
        || right.dimension != model.dimension
        || left.epoch != model.epoch
        || right.epoch != model.epoch
    {
        return Err(invalid("prepared endpoint dimension or epoch mismatch"));
    }
    Ok(())
}

fn direct_controls(
    left: &PreparedEndpoint,
    right: &PreparedEndpoint,
    h: f64,
) -> CoreResult<Vec<[Interval; 4]>> {
    let h_over_three = point(h)?.div(point(3.0)?)?;
    (0..left.values.len())
        .map(|i| {
            Ok([
                left.values[i],
                left.values[i].add(h_over_three.mul(left.slopes[i])?)?,
                right.values[i].sub(h_over_three.mul(right.slopes[i])?)?,
                right.values[i],
            ])
        })
        .collect()
}

fn check_domain(path: &[[Interval; 4]], n: usize) -> CoreResult<()> {
    for (i, controls) in path.iter().enumerate() {
        let limit = if i < n { 0.125 } else { 0.25 };
        if controls.iter().any(|value| value.mag() > limit) {
            return Err(invalid(
                "whole Hermite control hull leaves the fixed physical domain",
            ));
        }
    }
    Ok(())
}

/// Enclose the exact certified Hermite curve at represented theta in `[0,1]`.
/// The returned intervals enclose the mathematical curve, rather than
/// silently substituting rounded slopes into a different interpolant.
pub fn enclose_dense(
    model: &CyclicModel,
    left: &PreparedEndpoint,
    right: &PreparedEndpoint,
    clock: SlabClock,
    theta: f64,
) -> CoreResult<Vec<Interval>> {
    validate_pair(model, left, right)?;
    if !theta.is_finite() || !(0.0..=1.0).contains(&theta) {
        return Err(invalid("dense theta must lie in [0,1]"));
    }
    let controls = direct_controls(left, right, clock.h)?;
    check_domain(&controls, model.dimension)?;
    if theta == 0.0 {
        return Ok(left.values.clone());
    }
    if theta == 1.0 {
        return Ok(right.values.clone());
    }
    let t = point(theta)?;
    let one_minus_t = ONE.sub(t)?;
    controls
        .into_iter()
        .map(|mut values| {
            for remaining in (1..4).rev() {
                for j in 0..remaining {
                    values[j] = one_minus_t.mul(values[j])?.add(t.mul(values[j + 1])?)?;
                }
            }
            Ok(values[0])
        })
        .collect()
}

struct KernelResult {
    df: f64,
    dg: f64,
    diagnostics: Option<CoefficientDiagnostics>,
}

fn direct_kernel(
    model: &CyclicModel,
    left: &PreparedEndpoint,
    right: &PreparedEndpoint,
    h: f64,
    diagnostics: bool,
    work: &mut CyclicWork,
) -> CoreResult<KernelResult> {
    let n = model.dimension;
    let g0 = left
        .original_residual
        .as_ref()
        .ok_or_else(|| invalid("direct certificate needs original-G endpoint preparation"))?;
    let g1 = right
        .original_residual
        .as_ref()
        .ok_or_else(|| invalid("direct certificate needs original-G endpoint preparation"))?;
    let path = direct_controls(left, right, h)?;
    check_domain(&path, n)?;
    let one_third = ratio(1.0, 3.0)?;
    let two_thirds = ratio(2.0, 3.0)?;
    let mut slow_coefficients = if diagnostics {
        Some(Vec::with_capacity(n))
    } else {
        None
    };
    let mut algebraic_coefficients = if diagnostics {
        Some(Vec::with_capacity(n))
    } else {
        None
    };
    let mut slow_middle: f64 = 0.0;
    let mut endpoint_g: f64 = 0.0;
    let mut interior_g: f64 = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        work.direct_slow_components += 1;
        let d1 = path[i][2].sub(path[i][1])?.scale(3.0)?.div(point(h)?)?;
        let control_field = |k: usize| -> CoreResult<Interval> {
            path[i][k]
                .scale(-1.0)?
                .add(path[n + i][k].add(path[n + j][k])?.scale(1.0 / 16.0)?)?
                .add(point(constant(i))?)
        };
        let r1 = one_third
            .mul(left.slopes[i])?
            .add(two_thirds.mul(d1)?)?
            .sub(control_field(1)?)?;
        let r2 = two_thirds
            .mul(d1)?
            .add(one_third.mul(right.slopes[i])?)?
            .sub(control_field(2)?)?;
        slow_middle = slow_middle.max(r1.mag()).max(r2.mag());
        if let Some(coefficients) = &mut slow_coefficients {
            coefficients.push([ZERO.as_pair(), r1.as_pair(), r2.as_pair(), ZERO.as_pair()]);
        }

        work.direct_algebraic_components += 1;
        let d = [
            path[n + i][0].sub(path[i][0])?,
            path[n + i][1].sub(path[i][1])?,
            path[n + i][2].sub(path[i][2])?,
            path[n + i][3].sub(path[i][3])?,
        ];
        let z = path[n + j];
        let e2 = d[0].add(d[1].scale(3.0)?)?.add(d[2])?.div(point(5.0)?)?;
        let e3 = d[0]
            .add(d[1].scale(9.0)?)?
            .add(d[2].scale(9.0)?)?
            .add(d[3])?
            .div(point(20.0)?)?;
        let e4 = d[1].add(d[2].scale(3.0)?)?.add(d[3])?.div(point(5.0)?)?;
        let s2 = z[0]
            .mul(z[2])?
            .scale(2.0)?
            .add(z[1].mul(z[1])?.scale(3.0)?)?
            .div(point(5.0)?)?;
        let s3 = z[0]
            .mul(z[3])?
            .add(z[1].mul(z[2])?.scale(9.0)?)?
            .div(point(10.0)?)?;
        let s4 = z[1]
            .mul(z[3])?
            .scale(2.0)?
            .add(z[2].mul(z[2])?.scale(3.0)?)?
            .div(point(5.0)?)?;
        let beta2 = e2.add(s2.scale(0.25)?)?;
        let beta3 = e3.add(s3.scale(0.25)?)?;
        let beta4 = e4.add(s4.scale(0.25)?)?;
        endpoint_g = endpoint_g.max(g0[i].mag()).max(g1[i].mag());
        interior_g = interior_g
            .max(beta2.mag())
            .max(beta3.mag())
            .max(beta4.mag());
        if let Some(coefficients) = &mut algebraic_coefficients {
            coefficients.push([
                g0[i].as_pair(),
                g0[i].as_pair(),
                beta2.as_pair(),
                beta3.as_pair(),
                beta4.as_pair(),
                g1[i].as_pair(),
                g1[i].as_pair(),
            ]);
        }
    }
    let df = point(slow_middle)?.scale(0.75)?.hi;
    let dg = endpoint_g.max(
        point(endpoint_g)?
            .scale(7.0 / 32.0)?
            .add(point(interior_g)?.scale(25.0 / 32.0)?)?
            .hi,
    );
    let diagnostics = match (slow_coefficients, algebraic_coefficients) {
        (Some(slow_bernstein), Some(algebraic_bernstein)) => Some(CoefficientDiagnostics {
            path_bernstein: path
                .iter()
                .map(|controls| controls.map(|value| value.as_pair()))
                .collect(),
            slow_bernstein,
            algebraic_bernstein,
        }),
        _ => None,
    };
    Ok(KernelResult {
        df,
        dg,
        diagnostics,
    })
}

type Polynomial = Vec<Interval>;

fn polynomial_add(mut a: Polynomial, b: &[Interval]) -> CoreResult<Polynomial> {
    a.resize(a.len().max(b.len()), ZERO);
    for i in 0..b.len() {
        a[i] = a[i].add(b[i])?;
    }
    Ok(a)
}

fn polynomial_scale(a: &[Interval], factor: Interval) -> CoreResult<Polynomial> {
    a.iter().map(|value| value.mul(factor)).collect()
}

fn polynomial_multiply(
    a: &[Interval],
    b: &[Interval],
    work: &mut CyclicWork,
) -> CoreResult<Polynomial> {
    let mut product = vec![ZERO; a.len() + b.len() - 1];
    for (i, ai) in a.iter().enumerate() {
        for (j, bj) in b.iter().enumerate() {
            work.polynomial_convolution_products += 1;
            product[i + j] = product[i + j].add(ai.mul(*bj)?)?;
        }
    }
    Ok(product)
}

fn polynomial_derivative(a: &[Interval]) -> CoreResult<Polynomial> {
    a.iter()
        .enumerate()
        .skip(1)
        .map(|(i, value)| value.scale(i as f64))
        .collect()
}

fn choose(n: usize, k: usize) -> usize {
    let mut value = 1;
    for j in 1..=k {
        value = value * (n - j + 1) / j;
    }
    value
}

fn power_to_bernstein(power: &[Interval], work: &mut CyclicWork) -> CoreResult<Vec<Interval>> {
    work.power_to_bernstein_transforms += 1;
    let n = power.len() - 1;
    let mut bernstein = Vec::with_capacity(power.len());
    for j in 0..=n {
        let mut coefficient = ZERO;
        for (k, value) in power.iter().enumerate().take(j + 1) {
            coefficient =
                coefficient.add(value.mul(ratio(choose(j, k) as f64, choose(n, k) as f64)?)?)?;
        }
        bernstein.push(coefficient);
    }
    Ok(bernstein)
}

/// Historical algorithm kept intentionally as a native comparator. It uses
/// the same finite intervals, exact slopes and candidate endpoints, but no
/// new endpoint cancellation or direct coefficient formula.
fn historical_kernel(
    model: &CyclicModel,
    left: &PreparedEndpoint,
    right: &PreparedEndpoint,
    h: f64,
    diagnostics: bool,
    work: &mut CyclicWork,
) -> CoreResult<KernelResult> {
    let n = model.dimension;
    let hi = point(h)?;
    let mut power = Vec::with_capacity(2 * n);
    let mut path = Vec::with_capacity(2 * n);
    for i in 0..2 * n {
        let a = left.values[i];
        let b = right.values[i];
        let f0 = left.slopes[i];
        let f1 = right.slopes[i];
        let p = vec![
            a,
            hi.mul(f0)?,
            b.sub(a)?
                .scale(3.0)?
                .sub(hi.mul(f0.scale(2.0)?.add(f1)?)?)?,
            a.sub(b)?.scale(2.0)?.add(hi.mul(f0.add(f1)?)?)?,
        ];
        let bernstein = power_to_bernstein(&p, work)?;
        path.push([bernstein[0], bernstein[1], bernstein[2], bernstein[3]]);
        power.push(p);
    }
    check_domain(&path, n)?;
    let mut df: f64 = 0.0;
    let mut dg: f64 = 0.0;
    let mut slow_coefficients = if diagnostics {
        Some(Vec::with_capacity(n))
    } else {
        None
    };
    let mut algebraic_coefficients = if diagnostics {
        Some(Vec::with_capacity(n))
    } else {
        None
    };
    for i in 0..n {
        let j = (i + 1) % n;
        let square = polynomial_multiply(&power[n + j], &power[n + j], work)?;
        let g = polynomial_add(
            polynomial_add(
                power[n + i].clone(),
                &polynomial_scale(&square, point(0.25)?)?,
            )?,
            &polynomial_scale(&power[i], point(-1.0)?)?,
        )?;
        let qsum = polynomial_add(power[n + i].clone(), &power[n + j])?;
        let field = polynomial_add(
            polynomial_add(
                polynomial_scale(&power[i], point(-1.0)?)?,
                &polynomial_scale(&qsum, point(1.0 / 16.0)?)?,
            )?,
            &[point(constant(i))?],
        )?;
        let derivative = polynomial_scale(&polynomial_derivative(&power[i])?, ONE.div(hi)?)?;
        let residual = polynomial_add(derivative, &polynomial_scale(&field, point(-1.0)?)?)?;
        let gb = power_to_bernstein(&g, work)?;
        let fb = power_to_bernstein(&residual, work)?;
        for value in &gb {
            dg = dg.max(value.mag());
        }
        for value in &fb {
            df = df.max(value.mag());
        }
        if let Some(coefficients) = &mut slow_coefficients {
            coefficients.push([
                fb[0].as_pair(),
                fb[1].as_pair(),
                fb[2].as_pair(),
                fb[3].as_pair(),
            ]);
        }
        if let Some(coefficients) = &mut algebraic_coefficients {
            coefficients.push([
                gb[0].as_pair(),
                gb[1].as_pair(),
                gb[2].as_pair(),
                gb[3].as_pair(),
                gb[4].as_pair(),
                gb[5].as_pair(),
                gb[6].as_pair(),
            ]);
        }
    }
    let diagnostics = match (slow_coefficients, algebraic_coefficients) {
        (Some(slow_bernstein), Some(algebraic_bernstein)) => Some(CoefficientDiagnostics {
            path_bernstein: path
                .iter()
                .map(|controls| controls.map(|value| value.as_pair()))
                .collect(),
            slow_bernstein,
            algebraic_bernstein,
        }),
        _ => None,
    };
    Ok(KernelResult {
        df,
        dg,
        diagnostics,
    })
}

fn physical_radii(
    e0: f64,
    h: f64,
    df: f64,
    dg: f64,
    recipe: RadiusRecipe,
) -> CoreResult<(f64, f64, f64)> {
    if !e0.is_finite() || e0 < 0.0 {
        return Err(invalid(
            "incoming slow radius must be finite and nonnegative",
        ));
    }
    let e = point(e0)?;
    let hi = point(h)?;
    let delta = point(df)?.add(point(dg)?.div(point(7.0)?)?)?;
    let z = hi.mul(ratio(6.0, 7.0)?)?;
    let forcing = hi.mul(delta)?;
    let (ew, ee) = match recipe {
        RadiusRecipe::Legacy => (e.add(forcing)?.hi, e.div(ONE.add(z)?)?.add(forcing)?.hi),
        RadiusRecipe::Tight => {
            let u1 = e.add(forcing)?.div(ONE.add(z)?)?.hi;
            let half_z = z.scale(0.5)?;
            let u2 = ONE
                .sub(half_z)?
                .mul(e)?
                .add(forcing)?
                .div(ONE.add(half_z)?)?
                .hi;
            let endpoint = u1.max(u2);
            (e0.max(endpoint), endpoint)
        }
    };
    let qw = point(ew)?.add(point(dg)?)?.mul(ratio(8.0, 7.0)?)?.hi;
    Ok((ew, ee, qw))
}

/// Certify one exact-field Hermite slab. No candidate generation or cache
/// lookup is hidden in this operation. The incoming radius and path chain
/// remain explicit caller-owned inputs.
#[allow(clippy::too_many_arguments)]
pub fn certify_slab(
    model: &CyclicModel,
    left: &PreparedEndpoint,
    right: &PreparedEndpoint,
    clock: SlabClock,
    incoming_radius: f64,
    certifier: Certifier,
    radius_recipe: RadiusRecipe,
    diagnostics: bool,
    work: &mut CyclicWork,
) -> CoreResult<SlabCertificate> {
    work.slab_attempts += 1;
    validate_pair(model, left, right)?;
    if !incoming_radius.is_finite() || incoming_radius < 0.0 {
        return Err(invalid(
            "incoming slow radius must be finite and nonnegative",
        ));
    }
    let result = match certifier {
        Certifier::HistoricalPower => {
            historical_kernel(model, left, right, clock.h, diagnostics, work)?
        }
        Certifier::DirectBernstein => {
            direct_kernel(model, left, right, clock.h, diagnostics, work)?
        }
    };
    let (ew, ee, qw) = physical_radii(
        incoming_radius,
        clock.h,
        result.df,
        result.dg,
        radius_recipe,
    )?;
    work.slab_certificates += 1;
    Ok(SlabCertificate {
        model_id: MODEL_ID,
        model_epoch: model.epoch,
        branch_id: BRANCH_ID,
        path_definition: PATH_DEFINITION,
        physical_norm: "all-state-infinity",
        certifier,
        radius_recipe,
        index: clock.index,
        t0: clock.t0,
        t1: clock.t1,
        h: clock.h,
        e0: incoming_radius,
        df: result.df,
        dg: result.dg,
        ew,
        ee,
        qw,
        diagnostics: result.diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_domain_and_dimension_inputs_fail_closed() {
        assert!(CyclicModel::new(5, 0).is_err());
        let model = CyclicModel::new(4, 0).unwrap();
        let mut work = CyclicWork::default();
        assert!(model.prepare_endpoint(&[0.0; 7], &mut work).is_err());
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.126] {
            let mut state = [0.0; 8];
            state[0] = bad;
            assert!(model.prepare_endpoint(&state, &mut work).is_err());
        }
        let mut state = [0.0; 8];
        state[4] = 0.251;
        assert!(model.prepare_endpoint(&state, &mut work).is_err());
    }

    #[test]
    fn changed_bits_and_epoch_cannot_reuse_an_endpoint() {
        let model = CyclicModel::new(4, 7).unwrap();
        let changed_model = CyclicModel::new(4, 8).unwrap();
        let mut work = CyclicWork::default();
        let state = [0.0; 8];
        let endpoint = model.prepare_endpoint(&state, &mut work).unwrap();
        model.reuse_endpoint(&endpoint, &state, &mut work).unwrap();
        let mut changed = state;
        changed[0] = f64::from_bits(1);
        assert!(
            model
                .reuse_endpoint(&endpoint, &changed, &mut work)
                .is_err()
        );
        assert!(
            changed_model
                .reuse_endpoint(&endpoint, &state, &mut work)
                .is_err()
        );
        let clock = DyadicGrid::new(16).unwrap().slab(0).unwrap();
        assert!(
            certify_slab(
                &changed_model,
                &endpoint,
                &endpoint,
                clock,
                0.0,
                Certifier::DirectBernstein,
                RadiusRecipe::Legacy,
                false,
                &mut work
            )
            .is_err()
        );
        assert_eq!(work.endpoint_reuses, 1);
    }

    #[test]
    fn all_three_physical_clock_labels_are_checked() {
        assert!(DyadicGrid::new(0).is_err());
        assert!(DyadicGrid::new(3).is_err());
        let grid = DyadicGrid::new(16).unwrap();
        let clock = grid.slab(3).unwrap();
        assert!(
            grid.checked_slab(3, clock.t0.next_up(), clock.t1, clock.h)
                .is_err()
        );
        assert!(
            grid.checked_slab(3, clock.t0, clock.t1.next_up(), clock.h)
                .is_err()
        );
        assert!(
            grid.checked_slab(3, clock.t0, clock.t1, clock.h.next_up())
                .is_err()
        );
        assert!(grid.checked_slab(3, clock.t0, clock.t1, clock.h).is_ok());
        assert!(grid.slab(16).is_err());
    }

    #[test]
    fn endpoint_membership_does_not_replace_whole_path_domain() {
        let model = CyclicModel::new(4, 0).unwrap();
        let mut work = CyclicWork::default();
        let left = model.prepare_endpoint(&[0.0; 8], &mut work).unwrap();
        let mut right_state = [0.0; 8];
        right_state[..4].fill(-0.125);
        let right = model.prepare_endpoint(&right_state, &mut work).unwrap();
        let clock = DyadicGrid::new(1).unwrap().slab(0).unwrap();
        assert!(
            certify_slab(
                &model,
                &left,
                &right,
                clock,
                0.0,
                Certifier::DirectBernstein,
                RadiusRecipe::Legacy,
                false,
                &mut work
            )
            .is_err()
        );
        assert_eq!(work.slab_attempts, 1);
        assert_eq!(work.slab_certificates, 0);
    }

    #[test]
    fn singular_circuit_denominator_is_rejected_independently() {
        let mut state = vec![ZERO; 8];
        for value in &mut state[4..] {
            *value = point(2.0).unwrap();
        }
        // This private-kernel attack is deliberately outside the public Q
        // domain; the public input validation rejects it even earlier.
        let mut work = CyclicWork::default();
        assert!(interval_lift(&state, 4, &mut work).is_err());
    }

    #[test]
    fn zero_initial_radius_does_not_drop_positive_forcing() {
        let (whole, endpoint, _) = physical_radii(0.0, 1.0, 1.0, 0.0, RadiusRecipe::Tight).unwrap();
        let lambda: f64 = 6.0 / 7.0;
        let forced_exact_solution = (1.0 - (-lambda).exp()) / lambda;
        assert!(1.0 / (1.0 + lambda) < forced_exact_solution);
        assert!(endpoint >= forced_exact_solution);
        assert!(whole >= endpoint);
        assert!(physical_radii(f64::NAN, 1.0, 1.0, 0.0, RadiusRecipe::Legacy).is_err());
    }
}
