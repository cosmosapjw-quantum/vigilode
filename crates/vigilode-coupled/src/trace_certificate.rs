//! Outward whole-slab certificate for the fixed manufactured F6 trace.
//! The candidate is the exact-original-slope Hermite reconstruction of the
//! represented nodes, not the RK4 interpolation polynomial.

use rodas5p_core::{
    CoreError, CoreResult,
    directed::{self, Interval},
};
use serde::Serialize;

use crate::trace_quotient::{
    QuarticParameters, TraceFieldRoute, TraceSemantics, TraceWork, quartic_field_enclosure,
};

const ZERO: Interval = Interval { lo: 0.0, hi: 0.0 };
const LAMBDA: f64 = 39.0 / 64.0;
const MAX_DEGREE: usize = 15;
const ALL: TraceSemantics = TraceSemantics::AllComplexRootsAlgebraicMultiplicity;

#[derive(Clone, Copy, Debug)]
pub struct TraceNode {
    pub t: f64,
    pub state: [f64; 2],
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct TraceCertificateWork {
    pub slabs_attempted: u64,
    pub slabs_completed: u64,
    pub slabs_rejected: u64,
    pub endpoint_field_enclosures: u64,
    pub polynomial_products: u64,
    pub bernstein_coefficients: u64,
    pub provider: TraceWork,
}

#[derive(Clone, Debug, Serialize)]
pub struct TraceSlabCertificate {
    pub index: usize,
    pub t0_bits: u64,
    pub t1_bits: u64,
    pub h_bits: u64,
    pub incoming_error_upper: f64,
    pub residual_upper: f64,
    pub whole_error_upper: f64,
    pub endpoint_error_upper: f64,
    pub domain_margin_lower: f64,
    pub residual_bernstein: [[[f64; 2]; 16]; 2],
    pub path_bernstein: [[[f64; 2]; 4]; 2],
    pub physical_budget_met_through_slab: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct TraceCertificateOutcome {
    pub status: &'static str,
    pub path_definition: &'static str,
    pub residual_definition: &'static str,
    pub propagation: &'static str,
    pub lambda_exact: &'static str,
    pub physical_cap_exact: &'static str,
    pub failure: Option<String>,
    pub physical_budget_met: Option<bool>,
    pub whole_path_error_upper: Option<f64>,
    pub endpoint_error_upper: Option<f64>,
    pub records: Vec<TraceSlabCertificate>,
    pub work: TraceCertificateWork,
}

#[derive(Clone, Copy)]
struct Poly {
    c: [Interval; MAX_DEGREE + 1],
    degree: usize,
}

#[derive(Debug)]
enum SlabFailure {
    Arithmetic(CoreError),
    Domain,
}

impl From<CoreError> for SlabFailure {
    fn from(error: CoreError) -> Self {
        Self::Arithmetic(error)
    }
}

impl Poly {
    fn constant(value: Interval) -> Self {
        let mut c = [ZERO; MAX_DEGREE + 1];
        c[0] = value;
        Self { c, degree: 0 }
    }

    fn variable(t0: f64, h: f64) -> CoreResult<Self> {
        let mut result = Self::constant(Interval::point(t0)?);
        result.c[1] = Interval::point(h)?;
        result.degree = 1;
        Ok(result)
    }

    fn add(self, other: Self) -> CoreResult<Self> {
        let mut out = Self::constant(ZERO);
        out.degree = self.degree.max(other.degree);
        for k in 0..=out.degree {
            out.c[k] = self.c[k].add(other.c[k])?;
        }
        Ok(out)
    }

    fn sub(self, other: Self) -> CoreResult<Self> {
        self.add(other.neg())
    }

    fn neg(mut self) -> Self {
        for k in 0..=self.degree {
            self.c[k] = -self.c[k];
        }
        self
    }

    fn scale(mut self, value: f64) -> CoreResult<Self> {
        for k in 0..=self.degree {
            self.c[k] = self.c[k].scale(value)?;
        }
        Ok(self)
    }

    fn mul(self, other: Self, work: &mut TraceCertificateWork) -> CoreResult<Self> {
        if self.degree + other.degree > MAX_DEGREE {
            return Err(CoreError::InvalidInput(
                "trace certificate polynomial degree exceeded".into(),
            ));
        }
        work.polynomial_products += 1;
        let mut out = Self::constant(ZERO);
        out.degree = self.degree + other.degree;
        for i in 0..=self.degree {
            for j in 0..=other.degree {
                out.c[i + j] = out.c[i + j].add(self.c[i].mul(other.c[j])?)?;
            }
        }
        Ok(out)
    }

    fn derivative_over_h(self, h: f64) -> CoreResult<Self> {
        let mut out = Self::constant(ZERO);
        out.degree = self.degree.saturating_sub(1);
        for k in 1..=self.degree {
            out.c[k - 1] = self.c[k].scale(k as f64)?.div(Interval::point(h)?)?;
        }
        Ok(out)
    }
}

fn choose(n: usize, k: usize) -> u32 {
    let k = k.min(n - k);
    let mut value = 1_u32;
    for j in 1..=k {
        value = value * (n - k + j) as u32 / j as u32;
    }
    value
}

fn bernstein(
    poly: Poly,
    degree: usize,
    work: &mut TraceCertificateWork,
) -> CoreResult<[Interval; MAX_DEGREE + 1]> {
    if degree < poly.degree || degree > MAX_DEGREE {
        return Err(CoreError::InvalidInput(
            "trace certificate Bernstein degree mismatch".into(),
        ));
    }
    let mut out = [ZERO; MAX_DEGREE + 1];
    for (i, item) in out.iter_mut().enumerate().take(degree + 1) {
        for k in 0..=i.min(poly.degree) {
            let ratio = Interval::point(choose(i, k) as f64)?
                .div(Interval::point(choose(degree, k) as f64)?)?;
            *item = item.add(poly.c[k].mul(ratio)?)?;
            work.bernstein_coefficients += 1;
        }
    }
    Ok(out)
}

fn exact_reference(t: Interval) -> CoreResult<(Interval, Interval)> {
    let t2 = t.mul(t)?;
    let t4 = t2.mul(t2)?;
    let t5 = t4.mul(t)?;
    let p = Interval::point(3.0 / 4.0)?
        .add(t.scale(1.0 / 8.0)?)?
        .add(t5.scale(1.0 / 64.0)?)?;
    let dp = Interval::point(1.0 / 8.0)?.add(t4.scale(5.0 / 64.0)?)?;
    Ok((p, dp))
}

fn original_slope(
    t: f64,
    y: [f64; 2],
    work: &mut TraceCertificateWork,
) -> CoreResult<[Interval; 2]> {
    let (p, dp) = exact_reference(Interval::point(t)?)?;
    let state = QuarticParameters::point(y[0], y[1])?;
    let control = QuarticParameters::new(p, p)?;
    work.endpoint_field_enclosures += 1;
    let g = quartic_field_enclosure(
        ALL,
        state,
        TraceFieldRoute::DirectMoments,
        &mut work.provider,
    )?;
    work.endpoint_field_enclosures += 1;
    let gp = quartic_field_enclosure(
        ALL,
        control,
        TraceFieldRoute::DirectMoments,
        &mut work.provider,
    )?;
    Ok([g[0].add(dp)?.sub(gp[0])?, g[1].add(dp)?.sub(gp[1])?])
}

fn hermite(y0: f64, y1: f64, s0: Interval, s1: Interval, h: f64) -> CoreResult<Poly> {
    let y0 = Interval::point(y0)?;
    let y1 = Interval::point(y1)?;
    let mut out = Poly::constant(y0);
    out.degree = 3;
    out.c[1] = s0.scale(h)?;
    out.c[2] = y1
        .sub(y0)?
        .scale(3.0)?
        .sub(s0.scale(2.0)?.add(s1)?.scale(h)?)?;
    out.c[3] = y0.sub(y1)?.scale(2.0)?.add(s0.add(s1)?.scale(h)?)?;
    Ok(out)
}

fn original_field_poly(a: Poly, b: Poly, work: &mut TraceCertificateWork) -> CoreResult<[Poly; 2]> {
    // Original moment equations, deliberately separate from the provider's
    // shared N(a,b) circuit and from its interval field implementation.
    let a2 = a.mul(a, work)?;
    let a3 = a2.mul(a, work)?;
    let ab = a.mul(b, work)?;
    let s2 = a.scale(4.0)?;
    let s4 = a2.add(b)?.scale(4.0)?;
    let s6 = a3
        .scale(4.0)?
        .add(ab.scale(12.0)?)?
        .add(Poly::constant(Interval::point(3.0)?))?;
    Ok([
        a.neg()
            .add(s2.scale(1.0 / 32.0)?)?
            .add(s4.scale(1.0 / 128.0)?)?
            .add(s6.scale(1.0 / 512.0)?)?,
        b.neg()
            .add(s2.scale(1.0 / 64.0)?)?
            .add(s4.scale(1.0 / 64.0)?)?
            .add(s6.scale(1.0 / 256.0)?)?,
    ])
}

fn cap_met(value: f64) -> bool {
    if !value.is_finite() || value < 0.0 {
        return false;
    }
    if value == 0.0 {
        return true;
    }
    let bits = value.to_bits();
    let biased = ((bits >> 52) & 0x7ff) as i32;
    let mantissa = if biased == 0 {
        bits & ((1_u64 << 52) - 1)
    } else {
        (1_u64 << 52) | (bits & ((1_u64 << 52) - 1))
    };
    let exponent = if biased == 0 {
        -1074
    } else {
        biased - 1023 - 52
    };
    if exponent >= 0 {
        return false;
    }
    let shift = (-exponent) as u32;
    shift >= 128 || u128::from(mantissa) * 100_000_000_u128 <= (1_u128 << shift)
}

fn propagate(e: f64, delta: f64, h: f64) -> CoreResult<f64> {
    let x = Interval::point(LAMBDA)?.mul(Interval::point(h)?)?;
    if x.lo < 0.0 {
        return Err(CoreError::InvalidInput(
            "trace certificate negative step".into(),
        ));
    }
    let ql = if x.hi >= 2.0 {
        0.0
    } else {
        let half = directed::div_up(x.hi, 2.0)?;
        let numerator = directed::sub_down(1.0, half)?;
        let denominator = directed::add_up(1.0, half)?;
        directed::div_down(numerator, denominator)?.max(0.0)
    };
    let x2lo = directed::mul_down(x.lo, x.lo)?;
    let denominator = directed::add_down(
        directed::add_down(1.0, x.lo)?,
        directed::div_down(x2lo, 2.0)?,
    )?;
    let qu = directed::div_up(1.0, denominator)?.min(1.0);
    if !(0.0 <= ql && ql <= qu && qu <= 1.0) {
        return Err(CoreError::InvalidInput(
            "trace certificate invalid decay enclosure".into(),
        ));
    }
    let d = directed::div_up(delta, LAMBDA)?;
    let affine = |q: f64| -> CoreResult<f64> {
        directed::add_up(
            directed::mul_up(q, e)?,
            directed::mul_up(directed::sub_up(1.0, q)?, d)?,
        )
    };
    Ok(affine(ql)?.max(affine(qu)?))
}

fn certify_slab(
    index: usize,
    nodes: &[TraceNode],
    incoming: f64,
    maximum: f64,
    work: &mut TraceCertificateWork,
) -> Result<(TraceSlabCertificate, f64), SlabFailure> {
    let left = nodes[index];
    let right = nodes[index + 1];
    let h = right.t - left.t;
    let s0 = original_slope(left.t, left.state, work)?;
    let s1 = original_slope(right.t, right.state, work)?;
    let paths = [
        hermite(left.state[0], right.state[0], s0[0], s1[0], h)?,
        hermite(left.state[1], right.state[1], s0[1], s1[1], h)?,
    ];
    let t = Poly::variable(left.t, h)?;
    let t2 = t.mul(t, work)?;
    let t4 = t2.mul(t2, work)?;
    let t5 = t4.mul(t, work)?;
    let p = Poly::constant(Interval::point(3.0 / 4.0)?)
        .add(t.scale(1.0 / 8.0)?)?
        .add(t5.scale(1.0 / 64.0)?)?;
    let dp = Poly::constant(Interval::point(1.0 / 8.0)?).add(t4.scale(5.0 / 64.0)?)?;
    let g = original_field_poly(paths[0], paths[1], work)?;
    let gp = original_field_poly(p, p, work)?;
    let mut delta: f64 = 0.0;
    let mut margin: f64 = f64::INFINITY;
    let mut residual_bernstein = [[[0.0; 2]; 16]; 2];
    let mut path_bernstein = [[[0.0; 2]; 4]; 2];
    for component in 0..2 {
        let residual = paths[component]
            .derivative_over_h(h)?
            .sub(g[component].add(dp)?.sub(gp[component])?)?;
        for (index, item) in bernstein(residual, 15, work)?.iter().enumerate().take(16) {
            delta = delta.max(item.mag());
            residual_bernstein[component][index] = item.as_pair();
        }
        for (index, item) in bernstein(paths[component], 3, work)?
            .iter()
            .enumerate()
            .take(4)
        {
            margin = margin.min(directed::sub_down(item.lo, 0.5)?);
            margin = margin.min(directed::sub_down(1.0, item.hi)?);
            path_bernstein[component][index] = item.as_pair();
        }
    }
    let endpoint = propagate(incoming, delta, h)?;
    let whole = incoming.max(endpoint);
    if margin.partial_cmp(&whole) != Some(std::cmp::Ordering::Greater) {
        return Err(SlabFailure::Domain);
    }
    let maximum = maximum.max(whole);
    Ok((
        TraceSlabCertificate {
            index,
            t0_bits: left.t.to_bits(),
            t1_bits: right.t.to_bits(),
            h_bits: h.to_bits(),
            incoming_error_upper: incoming,
            residual_upper: delta,
            whole_error_upper: whole,
            endpoint_error_upper: endpoint,
            domain_margin_lower: margin,
            residual_bernstein,
            path_bernstein,
            physical_budget_met_through_slab: cap_met(maximum),
        },
        maximum,
    ))
}

/// Certified bounds for a complete fixed-grid path. All returned bounds are
/// upward binary64 enclosures; a false cap flag is an honest physical outcome.
/// Rejection preserves all prior slab records and attempted provider work.
pub fn certify_trace_path(slabs: usize, nodes: &[TraceNode]) -> TraceCertificateOutcome {
    let mut outcome = TraceCertificateOutcome {
        status: "rejected",
        path_definition: "exact-original-slope-hermite-v1",
        residual_definition: "original-S2-S4-S6-controlled-F-Bernstein-degree15",
        propagation: "T19-outward-max-qL-qU; whole=max(incoming,endpoint)",
        lambda_exact: "39/64",
        physical_cap_exact: "1/100000000",
        failure: None,
        physical_budget_met: None,
        whole_path_error_upper: None,
        endpoint_error_upper: None,
        records: Vec::new(),
        work: TraceCertificateWork::default(),
    };
    let validate = || -> CoreResult<()> {
        if ![16, 32, 64, 128, 256].contains(&slabs) || nodes.len() != slabs + 1 {
            return Err(CoreError::InvalidInput(
                "trace certificate unregistered clock or node count".into(),
            ));
        }
        for (index, node) in nodes.iter().enumerate() {
            let expected = -1.0 + (2 * index) as f64 / slabs as f64;
            if node.t.to_bits() != expected.to_bits() || node.state.iter().any(|x| !x.is_finite()) {
                return Err(CoreError::InvalidInput(
                    "trace certificate clock/state mismatch".into(),
                ));
            }
            if node.state.iter().any(|&x| !(0.5..=1.0).contains(&x)) {
                return Err(CoreError::InvalidInput(
                    "trace certificate endpoint outside original domain".into(),
                ));
            }
        }
        if nodes[0].state != [39.0 / 64.0; 2] {
            return Err(CoreError::InvalidInput(
                "trace certificate initial state mismatch".into(),
            ));
        }
        Ok(())
    };
    if let Err(error) = validate() {
        outcome.failure = Some(error.to_string());
        return outcome;
    }
    // The registered grids cap this allocation at 256 records. It happens
    // before any slab arithmetic, including rejected slabs.
    outcome.records = Vec::with_capacity(slabs);
    let mut incoming = 0.0;
    let mut maximum = 0.0;
    let mut failed = None;
    for index in 0..slabs {
        outcome.work.slabs_attempted += 1;
        match certify_slab(index, nodes, incoming, maximum, &mut outcome.work) {
            Ok((record, new_maximum)) => {
                incoming = record.endpoint_error_upper;
                maximum = new_maximum;
                outcome.records.push(record);
                outcome.work.slabs_completed += 1;
            }
            Err(error) => {
                outcome.work.slabs_rejected += 1;
                failed = Some((index, error));
                break;
            }
        }
    }
    if let Some((index, error)) = failed {
        outcome.failure = Some(match error {
            SlabFailure::Domain => {
                format!("trace certificate slab {index} lacks a strict original-domain tube")
            }
            SlabFailure::Arithmetic(error) => error.to_string(),
        });
        outcome.whole_path_error_upper = Some(maximum);
        outcome.endpoint_error_upper = Some(incoming);
        return outcome;
    }
    outcome.status = "completed";
    outcome.physical_budget_met = Some(cap_met(maximum));
    outcome.whole_path_error_upper = Some(maximum);
    outcome.endpoint_error_upper = Some(incoming);
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exact_reference(t: f64) -> f64 {
        0.75 + t / 8.0 + t.powi(5) / 64.0
    }

    fn exact_nodes(slabs: usize) -> Vec<TraceNode> {
        (0..=slabs)
            .map(|index| {
                let t = -1.0 + (2 * index) as f64 / slabs as f64;
                TraceNode {
                    t,
                    state: [exact_reference(t); 2],
                }
            })
            .collect()
    }

    #[test]
    fn physical_cap_uses_exact_rational_comparison_of_binary64_bound() {
        assert!(cap_met(1.0e-8_f64.next_down()));
        assert!(!cap_met(1.0e-8_f64.next_up()));
        assert!(cap_met(0.0));
        assert!(!cap_met(f64::INFINITY));
    }

    #[test]
    fn refused_clock_keeps_attempted_work_zero() {
        let mut nodes = exact_nodes(16);
        nodes[1].t = -0.5;
        let outcome = certify_trace_path(16, &nodes);
        assert_eq!(outcome.status, "rejected");
        assert_eq!(outcome.work.slabs_attempted, 0);
        assert!(outcome.records.is_empty());
    }

    #[test]
    fn wrong_initial_condition_is_refused_before_field_work() {
        let mut nodes = exact_nodes(16);
        nodes[0].state[0] += 1.0 / 128.0;
        let outcome = certify_trace_path(16, &nodes);
        assert_eq!(outcome.status, "rejected");
        assert_eq!(outcome.work.provider.requests_attempted, 0);
    }

    #[test]
    fn path_outside_original_domain_is_refused_with_partial_work() {
        let mut nodes = exact_nodes(16);
        nodes[1].state = [0.0; 2];
        let outcome = certify_trace_path(16, &nodes);
        assert_eq!(outcome.status, "rejected");
        assert_eq!(outcome.work.slabs_attempted, 0);
        assert_eq!(outcome.work.slabs_rejected, 0);
        assert_eq!(outcome.work.provider.requests_attempted, 0);
        assert!(outcome.records.is_empty());
    }

    #[test]
    fn domain_tube_refusal_retains_slab_work() {
        let nodes = exact_nodes(16);
        let mut work = TraceCertificateWork::default();
        let failure = certify_slab(0, &nodes, 1.0, 1.0, &mut work);
        assert!(matches!(failure, Err(SlabFailure::Domain)));
        assert_eq!(work.endpoint_field_enclosures, 4);
        assert!(work.provider.requests_attempted > 0);
    }

    #[test]
    fn oversized_grid_is_refused_before_record_allocation() {
        let outcome = certify_trace_path(usize::MAX, &[]);
        assert_eq!(outcome.status, "rejected");
        assert!(outcome.records.is_empty());
        assert_eq!(outcome.records.capacity(), 0);
    }
}
