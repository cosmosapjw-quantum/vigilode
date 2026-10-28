//! Opt-in certificates for the represented scalar affine equation
//! `x' = -lambda*x + b` and the event `x - threshold` (SYM02).
//!
//! The native adapter consumes an already accepted fast RODAS5P segment.
//! Its exact-real quartic is reconstructed from the stored binary64 `H`,
//! stage rows and endpoint states. Outward intervals enclose its whole-slab
//! defect in the exact-real clock `t1 - t0`. A caller-supplied incoming error
//! is a premise, rather than a conclusion of the adaptive error estimator.
//! Source/model/clock/unit/event identifiers are caller assertions and are
//! carried unchanged. This module does not validate a general RHS callback.
//!
//! Arithmetic assumes IEEE binary64 with round-to-nearest/ties-even and
//! gradual underflow on x86_64. Safe, nonconstant arithmetic probes check
//! those properties at construction and query; no control register is read
//! or changed. The thread must keep its arithmetic environment unchanged
//! during each call. The crate's `forbid(unsafe_code)` is preserved.

use std::hint::black_box;

use rodas5p_core::{
    CoreError, CoreResult,
    directed::{Interval, add_up},
};

/// The clock used by these certificates: the exact real difference of the
/// stored endpoint times, with the same units as the declared affine RHS.
pub const AFFINE_EVENT_CLOCK_ID: &str = "represented-real-segment-v1";

fn invalid(message: &str) -> CoreError {
    CoreError::InvalidInput(format!("affine event certificate: {message}"))
}

/// Check the arithmetic properties required by `rodas5p_core::directed`.
///
/// The two 3/4-ulp sums distinguish nearest from the other three IEEE
/// rounding modes; the half-ulp sums check ties-even. Subnormal input and
/// output products detect DAZ and FTZ. This is an arithmetic probe, not a
/// measurement of MXCSR. It is repeated at every proof-producing call.
pub fn check_affine_event_arithmetic() -> CoreResult<()> {
    if !cfg!(target_arch = "x86_64") {
        return Err(invalid("the arithmetic probe is qualified on x86_64 only"));
    }
    let one = black_box(1.0_f64);
    let minus_one = black_box(-1.0_f64);
    let three_quarters = black_box(0.75 * f64::EPSILON);
    let half = black_box(0.5 * f64::EPSILON);
    let nearest = black_box(one + three_quarters).to_bits() == 1.0_f64.next_up().to_bits()
        && black_box(minus_one - three_quarters).to_bits()
            == (-1.0_f64).next_down().to_bits()
        && black_box(one + half).to_bits() == 1.0_f64.to_bits()
        && black_box(minus_one - half).to_bits() == (-1.0_f64).to_bits();
    let tiny = black_box(f64::from_bits(1));
    let normal = black_box(f64::MIN_POSITIVE);
    let factor = black_box(0.5_f64);
    let gradual = black_box(tiny * one).to_bits() == 1
        && black_box((-tiny) * one).to_bits() == ((1_u64 << 63) | 1)
        && black_box(normal * factor).to_bits() == 0x0008_0000_0000_0000
        && black_box((-normal) * factor).to_bits() == 0x8008_0000_0000_0000;
    if nearest && gradual {
        Ok(())
    } else {
        Err(invalid("requires nearest/ties-even arithmetic and gradual input/output underflow"))
    }
}

/// Immutable source and transaction assertions supplied by the caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AffineEventBinding {
    model_id: String,
    source_id: String,
    clock_id: String,
    unit_id: String,
    event_id: String,
    epoch: u64,
}

impl AffineEventBinding {
    /// All identities must be nonempty; `clock_id` must name this module's
    /// exact-real endpoint-difference clock.
    pub fn new(
        model_id: &str,
        source_id: &str,
        clock_id: &str,
        unit_id: &str,
        event_id: &str,
        epoch: u64,
    ) -> CoreResult<Self> {
        if [model_id, source_id, clock_id, unit_id, event_id]
            .iter()
            .any(|identity| identity.trim().is_empty())
        {
            return Err(invalid("model/source/clock/unit/event identities must be nonempty"));
        }
        if clock_id != AFFINE_EVENT_CLOCK_ID {
            return Err(invalid("a nominal or otherwise different clock cannot be substituted"));
        }
        Ok(Self {
            model_id: model_id.into(),
            source_id: source_id.into(),
            clock_id: clock_id.into(),
            unit_id: unit_id.into(),
            event_id: event_id.into(),
            epoch,
        })
    }

    pub fn model_id(&self) -> &str { &self.model_id }
    pub fn source_id(&self) -> &str { &self.source_id }
    pub fn clock_id(&self) -> &str { &self.clock_id }
    pub fn unit_id(&self) -> &str { &self.unit_id }
    pub fn event_id(&self) -> &str { &self.event_id }
    pub fn epoch(&self) -> u64 { self.epoch }
}

/// Constant parameter boxes for the declared equation `x'=-lambda*x+b`.
/// Every represented equation in these boxes shares the comparison bound.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AffineScalarModel {
    lambda: Interval,
    forcing: Interval,
}

impl AffineScalarModel {
    pub fn new(lambda: Interval, forcing: Interval) -> CoreResult<Self> {
        let lambda = Interval::new(lambda.lo, lambda.hi)?;
        let forcing = Interval::new(forcing.lo, forcing.hi)?;
        if lambda.lo <= 0.0 {
            return Err(invalid("the entire lambda box must be strictly positive"));
        }
        Ok(Self { lambda, forcing })
    }

    pub fn lambda(&self) -> Interval { self.lambda }
    pub fn forcing(&self) -> Interval { self.forcing }
}

/// The full equation, threshold and source/transaction binding.
#[derive(Clone, Debug, PartialEq)]
pub struct AffineEventSpec {
    binding: AffineEventBinding,
    model: AffineScalarModel,
    threshold: f64,
}

impl AffineEventSpec {
    pub fn new(
        binding: AffineEventBinding,
        model: AffineScalarModel,
        threshold: f64,
    ) -> CoreResult<Self> {
        Interval::point(threshold)?;
        Ok(Self { binding, model, threshold })
    }

    pub fn binding(&self) -> &AffineEventBinding { &self.binding }
    pub fn model(&self) -> AffineScalarModel { self.model }
    pub fn threshold(&self) -> f64 { self.threshold }
}

/// A manually supplied represented segment and an actual native accepted
/// segment remain distinguishable in all certificates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AffineSegmentOrigin {
    DeclaredRepresented,
    NativeAcceptedRodas5pFast,
}

/// Immutable binary64 inputs to one scalar RODAS5P dense polynomial.
#[derive(Clone, Debug)]
pub struct RepresentedAffineSegment {
    times: [f64; 2],
    states: [f64; 2],
    dense_h: [[f64; 8]; 3],
    stages: [f64; 8],
    index: usize,
    origin: AffineSegmentOrigin,
}

impl RepresentedAffineSegment {
    /// Declare a represented packet, including synthetic/analytic test
    /// packets. Only the native adapter assigns native-accepted provenance.
    pub fn new(
        times: [f64; 2],
        states: [f64; 2],
        dense_h: [[f64; 8]; 3],
        stages: [f64; 8],
        index: usize,
    ) -> CoreResult<Self> {
        if !(times.iter().chain(states.iter()).all(|v| v.is_finite())
            && dense_h.iter().flatten().chain(stages.iter()).all(|v| v.is_finite())
            && times[0] < times[1])
        {
            return Err(invalid("a packet needs finite data and strictly increasing endpoint times"));
        }
        Ok(Self {
            times, states, dense_h, stages, index,
            origin: AffineSegmentOrigin::DeclaredRepresented,
        })
    }

    pub(crate) fn from_native(
        times: [f64; 2],
        states: [f64; 2],
        dense_h: [[f64; 8]; 3],
        stages: [f64; 8],
        index: usize,
    ) -> CoreResult<Self> {
        let mut packet = Self::new(times, states, dense_h, stages, index)?;
        packet.origin = AffineSegmentOrigin::NativeAcceptedRodas5pFast;
        Ok(packet)
    }

    pub fn times(&self) -> [f64; 2] { self.times }
    pub fn states(&self) -> [f64; 2] { self.states }
    pub fn index(&self) -> usize { self.index }
    pub fn origin(&self) -> AffineSegmentOrigin { self.origin }

    fn same_bits(&self, other: &Self) -> bool {
        self.index == other.index
            && self.origin == other.origin
            && self.times.iter().chain(self.states.iter())
                .chain(self.dense_h.iter().flatten()).chain(self.stages.iter())
                .zip(other.times.iter().chain(other.states.iter())
                    .chain(other.dense_h.iter().flatten()).chain(other.stages.iter()))
                .all(|(a,b)| a.to_bits() == b.to_bits())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AffineEventSign { Positive, Negative, Unresolved }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AffineFlowDirection { StrictlyIncreasing, StrictlyDecreasing, Unresolved }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AffineEventStatus { NoCrossing, UniqueCrossing, AtLeastOneCrossing, Unresolved }

fn sign(range: Interval) -> AffineEventSign {
    if range.lo > 0.0 { AffineEventSign::Positive }
    else if range.hi < 0.0 { AffineEventSign::Negative }
    else { AffineEventSign::Unresolved }
}

fn radius_interval(center: Interval, radius: f64) -> CoreResult<Interval> {
    center.add(Interval::new(-radius, radius)?)
}

fn binomial(n: usize, k: usize) -> f64 {
    // The only callers have 0 <= k <= n <= 4; these integers are exact.
    const C: [[f64; 5]; 5] = [
        [1.0,0.0,0.0,0.0,0.0], [1.0,1.0,0.0,0.0,0.0],
        [1.0,2.0,1.0,0.0,0.0], [1.0,3.0,3.0,1.0,0.0],
        [1.0,4.0,6.0,4.0,1.0],
    ];
    C[n][k]
}

fn bernstein(power: &[Interval; 5]) -> CoreResult<[Interval; 5]> {
    let mut result = [Interval::point(0.0)?; 5];
    for (k, value) in result.iter_mut().enumerate() {
        for (j, coefficient) in power.iter().enumerate().take(k + 1) {
            let ratio = Interval::point(binomial(k,j))?
                .div(Interval::point(binomial(4,j))?)?;
            *value = value.add(coefficient.mul(ratio)?)?;
        }
    }
    Ok(result)
}

fn hull(intervals: &[Interval; 5]) -> CoreResult<Interval> {
    Interval::new(
        intervals.iter().map(|x|x.lo).fold(f64::INFINITY, f64::min),
        intervals.iter().map(|x|x.hi).fold(f64::NEG_INFINITY, f64::max),
    )
}

fn horner(power: &[Interval; 5], theta: Interval) -> CoreResult<Interval> {
    let mut value = power[4];
    for coefficient in power[..4].iter().rev() {
        value = value.mul(theta)?.add(*coefficient)?;
    }
    Ok(value)
}

/// An immutable whole-slab proof for the declared scalar affine model.
/// Query rounding, source identities and caller error premises are explicit.
#[derive(Clone, Debug)]
pub struct AffineEventCertificate {
    segment: RepresentedAffineSegment,
    spec: AffineEventSpec,
    clock_width: Interval,
    power: [Interval; 5],
    residual_bernstein: [Interval; 5],
    polynomial_range: Interval,
    state_tube: Interval,
    flow_range: Interval,
    defect_upper: f64,
    initial_error: f64,
    endpoint_error_upper: f64,
    whole_slab_error_upper: f64,
    endpoint_symbols: [AffineEventSign; 2],
    slab_symbol: AffineEventSign,
    flow_direction: AffineFlowDirection,
    event_status: AffineEventStatus,
}

impl AffineEventCertificate {
    /// `initial_error` must enclose the discrepancy between the exact IVP
    /// initial state and the stored left endpoint. This premise is supplied
    /// by the caller; a local adaptive estimate is not accepted as its proof.
    pub fn certify(
        segment: RepresentedAffineSegment,
        spec: &AffineEventSpec,
        initial_error: f64,
    ) -> CoreResult<Self> {
        check_affine_event_arithmetic()?;
        if !initial_error.is_finite() || initial_error < 0.0 {
            return Err(invalid("incoming error must be finite and nonnegative"));
        }
        let clock_width = Interval::point(segment.times[1])?
            .sub(Interval::point(segment.times[0])?)?;
        if clock_width.lo <= 0.0 {
            return Err(invalid("the enclosed real clock width must be strictly positive"));
        }
        let mut d = [Interval::point(0.0)?; 3];
        for (r, row) in segment.dense_h.iter().enumerate() {
            for (&coefficient, &stage) in row.iter().zip(&segment.stages) {
                d[r] = d[r].add(Interval::point(coefficient)?.mul(Interval::point(stage)?)?)?;
            }
        }
        let y0 = Interval::point(segment.states[0])?;
        let y1 = Interval::point(segment.states[1])?;
        let power = [y0, y1.sub(y0)?.add(d[0])?, d[1].sub(d[0])?,
            d[2].sub(d[1])?, -d[2]];
        let mut residual = [Interval::point(0.0)?; 5];
        for (j, coefficient) in power.iter().enumerate() {
            residual[j] = coefficient.mul(spec.model.lambda)?;
            if j < 4 {
                residual[j] = residual[j]
                    .add(power[j+1].scale((j+1) as f64)?.div(clock_width)?)?;
            }
        }
        residual[0] = residual[0].sub(spec.model.forcing)?;
        let residual_bernstein = bernstein(&residual)?;
        let defect_upper = residual_bernstein.iter().map(Interval::mag).fold(0.0, f64::max);

        // E' = -lambda_lo E + delta. The two rational envelopes from
        // SYM01 cover both signs of E0 - delta/lambda_lo. Interval evaluation
        // includes uncertainty in the exact-real width, and max covers the
        // branch whose exponential inequality has the required direction.
        let z = clock_width.scale(spec.model.lambda.lo)?;
        let force = clock_width.scale(defect_upper)?;
        let e0 = Interval::point(initial_error)?;
        let one = Interval::point(1.0)?;
        let u1 = e0.add(force)?.div(one.add(z)?)?;
        let half_z = z.scale(0.5)?;
        let u2 = one.sub(half_z)?.mul(e0)?.add(force)?.div(one.add(half_z)?)?;
        let endpoint_error_upper = u1.hi.max(u2.hi).max(0.0);
        let whole_slab_error_upper = initial_error.max(endpoint_error_upper);
        let polynomial_range = hull(&bernstein(&power)?)?;
        let state_tube = radius_interval(polynomial_range, whole_slab_error_upper)?;
        let flow_range = (-spec.model.lambda).mul(state_tube)?.add(spec.model.forcing)?;
        let threshold = Interval::point(spec.threshold)?;
        let endpoint_symbols = [
            sign(radius_interval(y0, initial_error)?.sub(threshold)?),
            sign(radius_interval(y1, endpoint_error_upper)?.sub(threshold)?),
        ];
        let slab_symbol = sign(state_tube.sub(threshold)?);
        let flow_direction = if flow_range.hi < 0.0 {
            AffineFlowDirection::StrictlyDecreasing
        } else if flow_range.lo > 0.0 {
            AffineFlowDirection::StrictlyIncreasing
        } else { AffineFlowDirection::Unresolved };
        let opposite = matches!(endpoint_symbols,
            [AffineEventSign::Positive, AffineEventSign::Negative]
            | [AffineEventSign::Negative, AffineEventSign::Positive]);
        let monotone = flow_direction != AffineFlowDirection::Unresolved;
        let event_status = if opposite {
            if monotone { AffineEventStatus::UniqueCrossing }
            else { AffineEventStatus::AtLeastOneCrossing }
        } else if slab_symbol != AffineEventSign::Unresolved
            || (endpoint_symbols[0] == endpoint_symbols[1]
                && endpoint_symbols[0] != AffineEventSign::Unresolved && monotone)
        { AffineEventStatus::NoCrossing }
        else { AffineEventStatus::Unresolved };
        Ok(Self {
            segment, spec: spec.clone(), clock_width, power, residual_bernstein,
            polynomial_range, state_tube, flow_range, defect_upper, initial_error,
            endpoint_error_upper, whole_slab_error_upper, endpoint_symbols,
            slab_symbol, flow_direction, event_status,
        })
    }

    fn check_spec(&self, spec: &AffineEventSpec) -> CoreResult<()> {
        if self.spec != *spec {
            Err(invalid("source/model/clock/unit/event/epoch or parameter binding changed"))
        } else { Ok(()) }
    }

    pub(crate) fn check_segment(&self, packet: &RepresentedAffineSegment) -> CoreResult<()> {
        if self.segment.same_bits(packet) { Ok(()) }
        else { Err(invalid("native segment data or provenance changed")) }
    }

    /// Enclose the error of a supplied binary64 center at the represented
    /// time `t`. The exact-real polynomial at that time is enclosed first;
    /// its certified distance from the actual center includes all clock,
    /// coefficient formation and evaluation rounding. Native callers should
    /// use `FastDenseSegments::query_affine_scalar_certificate`, which also
    /// verifies the current segment's bits and obtains its actual output.
    pub fn query(
        &self,
        spec: &AffineEventSpec,
        t: f64,
        actual_center: f64,
    ) -> CoreResult<AffineEventQuery> {
        check_affine_event_arithmetic()?;
        self.check_spec(spec)?;
        let [t0,t1] = self.segment.times;
        if !(t.is_finite() && t0 <= t && t <= t1 && actual_center.is_finite()) {
            return Err(invalid("query time or center is nonfinite or outside this segment"));
        }
        let (polynomial, state_error) = if t == t0 {
            (Interval::point(self.segment.states[0])?, self.initial_error)
        } else if t == t1 {
            (Interval::point(self.segment.states[1])?, self.endpoint_error_upper)
        } else {
            let theta = Interval::point(t)?.sub(Interval::point(t0)?)?.div(self.clock_width)?;
            // The exact normalized time belongs to [0,1], independently of
            // any outward excess of its arithmetic enclosure.
            let theta = Interval::new(theta.lo.max(0.0), theta.hi.min(1.0))?;
            (horner(&self.power,theta)?, self.whole_slab_error_upper)
        };
        let evaluation_rounding_upper = Interval::point(actual_center)?.sub(polynomial)?.mag();
        let error_upper = add_up(state_error,evaluation_rounding_upper)?;
        if !error_upper.is_finite() {
            return Err(invalid("query error overflow"));
        }
        let state_enclosure = radius_interval(Interval::point(actual_center)?,error_upper)?;
        let symbol = sign(state_enclosure.sub(Interval::point(spec.threshold)?)?);
        Ok(AffineEventQuery {
            time:t, actual_center, represented_polynomial:polynomial,
            evaluation_rounding_upper, error_upper, state_enclosure, symbol,
        })
    }

    /// Carry the proven endpoint radius to one adjacent, unreset segment.
    /// A source/model/event/epoch change, a gap or a changed endpoint refuses
    /// this carry. A reset needs an externally justified new incoming error.
    pub fn carry(&self) -> CoreResult<AffineEventCarry> {
        check_affine_event_arithmetic()?;
        let next_index = self.segment.index.checked_add(1)
            .ok_or_else(||invalid("segment index overflow"))?;
        Ok(AffineEventCarry {
            spec:self.spec.clone(), time:self.segment.times[1],
            state:self.segment.states[1], error:self.endpoint_error_upper,
            next_index, origin:self.segment.origin,
        })
    }

    pub fn binding(&self) -> &AffineEventBinding { &self.spec.binding }
    pub fn segment_index(&self) -> usize { self.segment.index }
    pub fn origin(&self) -> AffineSegmentOrigin { self.segment.origin }
    pub fn times(&self) -> [f64;2] { self.segment.times }
    pub fn clock_width(&self) -> Interval { self.clock_width }
    pub fn power_coefficients(&self) -> &[Interval;5] { &self.power }
    pub fn residual_bernstein(&self) -> &[Interval;5] { &self.residual_bernstein }
    pub fn polynomial_range(&self) -> Interval { self.polynomial_range }
    pub fn state_tube(&self) -> Interval { self.state_tube }
    pub fn flow_range(&self) -> Interval { self.flow_range }
    pub fn defect_upper(&self) -> f64 { self.defect_upper }
    pub fn initial_error(&self) -> f64 { self.initial_error }
    pub fn endpoint_error_upper(&self) -> f64 { self.endpoint_error_upper }
    pub fn whole_slab_error_upper(&self) -> f64 { self.whole_slab_error_upper }
    pub fn endpoint_symbols(&self) -> [AffineEventSign;2] { self.endpoint_symbols }
    pub fn slab_symbol(&self) -> AffineEventSign { self.slab_symbol }
    pub fn flow_direction(&self) -> AffineFlowDirection { self.flow_direction }
    pub fn event_status(&self) -> AffineEventStatus { self.event_status }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AffineEventQuery {
    pub time: f64,
    pub actual_center: f64,
    pub represented_polynomial: Interval,
    pub evaluation_rounding_upper: f64,
    pub error_upper: f64,
    pub state_enclosure: Interval,
    pub symbol: AffineEventSign,
}

#[derive(Clone, Debug)]
pub struct AffineEventCarry {
    spec: AffineEventSpec,
    time: f64,
    state: f64,
    error: f64,
    next_index: usize,
    origin: AffineSegmentOrigin,
}

impl AffineEventCarry {
    pub fn error_upper(&self) -> f64 { self.error }

    pub fn certify_next(
        &self,
        segment: RepresentedAffineSegment,
        spec: &AffineEventSpec,
    ) -> CoreResult<AffineEventCertificate> {
        check_affine_event_arithmetic()?;
        if self.spec != *spec || segment.times[0].to_bits() != self.time.to_bits()
            || segment.states[0].to_bits() != self.state.to_bits()
            || segment.index != self.next_index || segment.origin != self.origin
        {
            return Err(invalid("error carry requires unchanged bindings and adjacent identical endpoint data"));
        }
        AffineEventCertificate::certify(segment,spec,self.error)
    }
}
