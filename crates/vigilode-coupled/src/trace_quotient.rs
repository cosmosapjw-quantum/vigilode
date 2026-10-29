//! Polynomial observables of an explicit length-four complex zero scheme.
//!
//! The original equations are `q1*q1 + q2 = a`, `q2*q2 + q1 = b`.
//! Every query here sums over all complex roots with algebraic multiplicity.
//! In particular, the triple root at `a = b = 3/4` still contributes three
//! times. A selected real branch and a real-only sum are different targets.
//!
//! The monic eliminant is `z^4 - 2*a*z^2 + z + a*a - b`, with `z = q1`.
//! Newton sums and fixed quotient identities evaluate the requested output
//! without root enumeration, Jacobians, JVP/AD or Newton iteration. Interval
//! outputs enclose the exact observable for all parameters in the supplied
//! box, under `rodas5p_core::directed`'s arithmetic assumptions. They are
//! point/box field certificates, not time-integration certificates.
//!
//! See `research/production_rvj_nag_20261028/theory/TRACE.md` for the proof,
//! representation contract, physical error composition and semantic controls.

use rodas5p_core::{CoreError, CoreResult, directed::Interval};
use serde::Serialize;

pub const TRACE_TARGET_ID: &str = "rvj-quartic-all-complex-multiplicity-v1";

/// Query semantics are explicit at every public evaluation boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceSemantics {
    AllComplexRootsAlgebraicMultiplicity,
    SelectedReal,
    RealOnly,
}

/// Two implementations of the same exact aggregate vector field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceFieldRoute {
    DirectMoments,
    SharedPolynomial,
}

/// Fixed polynomial queries in the original two variables.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuarticInvariant {
    One,
    Q1,
    Q2,
    Q1Squared,
    Q2Squared,
    Q1Q2,
    Q1SquaredQ2Squared,
    Q1SquaredPlusQ2,
}

/// Validated parameter enclosure. A point denotes the exact represented
/// binary64 real; a caller approximating other reals must supply their bounds.
#[derive(Clone, Copy, Debug)]
pub struct QuarticParameters {
    a: Interval,
    b: Interval,
}

impl QuarticParameters {
    pub fn new(a: Interval, b: Interval) -> CoreResult<Self> {
        // Interval's fields are public, so constructors cannot assume that
        // every incoming instance was created with Interval::new.
        Ok(Self {
            a: Interval::new(a.lo, a.hi)?,
            b: Interval::new(b.lo, b.hi)?,
        })
    }

    pub fn point(a: f64, b: f64) -> CoreResult<Self> {
        Self::new(Interval::point(a)?, Interval::point(b)?)
    }

    pub fn a(self) -> Interval {
        self.a
    }

    pub fn b(self) -> Interval {
        self.b
    }
}

/// Caller-owned counters survive a rejected or partially evaluated request.
/// Arithmetic counts are operations of this provider, not hardware FLOPs:
/// one interval multiplication includes its directed endpoint work.
/// Parameter construction, downstream ODE updates and serialization are
/// outside these counters and must be measured by their caller.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct TraceWork {
    pub requests_attempted: u64,
    pub requests_completed: u64,
    pub requests_rejected: u64,
    pub semantics_rejections: u64,
    pub moment_batches: u64,
    pub shared_field_batches: u64,
    pub invariant_queries: u64,
    pub polynomial_queries: u64,
    pub scalar_additions: u64,
    pub scalar_multiplications: u64,
    pub interval_additions: u64,
    pub interval_multiplications: u64,
    pub root_enumerations: u64,
    pub jacobian_evaluations: u64,
    pub jvp_evaluations: u64,
    pub ad_evaluations: u64,
    pub newton_updates: u64,
}

fn charge(counter: &mut u64) -> CoreResult<()> {
    *counter = counter
        .checked_add(1)
        .ok_or_else(|| CoreError::InvalidInput("trace work counter exhausted".into()))?;
    Ok(())
}

fn request<T>(
    semantics: TraceSemantics,
    work: &mut TraceWork,
    evaluate: impl FnOnce(&mut TraceWork) -> CoreResult<T>,
) -> CoreResult<T> {
    charge(&mut work.requests_attempted)?;
    let result = if semantics == TraceSemantics::AllComplexRootsAlgebraicMultiplicity {
        evaluate(work)
    } else {
        charge(&mut work.semantics_rejections)?;
        Err(CoreError::InvalidInput(
            "quartic trace requires all complex roots with algebraic multiplicity; selected-real and real-only queries have different targets".into(),
        ))
    };
    if result.is_ok() {
        charge(&mut work.requests_completed)?;
    } else {
        charge(&mut work.requests_rejected)?;
    }
    result
}

fn finite(value: f64) -> CoreResult<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(CoreError::NonFinite(
            "quartic trace input or arithmetic result is non-finite".into(),
        ))
    }
}

// A private straight-line expression evaluates either represented candidates
// or real enclosures. The independent test oracle uses integer matrix powers,
// not this expression with a second floating type.
trait Value: Copy {
    fn constant(value: f64) -> Self;
    fn plus(self, other: Self, work: &mut TraceWork) -> CoreResult<Self>;
    fn times(self, other: Self, work: &mut TraceWork) -> CoreResult<Self>;
    fn negative(self) -> Self;
}

impl Value for f64 {
    fn constant(value: f64) -> Self {
        value
    }

    fn plus(self, other: Self, work: &mut TraceWork) -> CoreResult<Self> {
        charge(&mut work.scalar_additions)?;
        finite(self + other)
    }

    fn times(self, other: Self, work: &mut TraceWork) -> CoreResult<Self> {
        charge(&mut work.scalar_multiplications)?;
        finite(self * other)
    }

    fn negative(self) -> Self {
        -self
    }
}

impl Value for Interval {
    fn constant(value: f64) -> Self {
        // All call sites below are fixed finite exactly dyadic constants.
        Self {
            lo: value,
            hi: value,
        }
    }

    fn plus(self, other: Self, work: &mut TraceWork) -> CoreResult<Self> {
        charge(&mut work.interval_additions)?;
        self.add(other)
    }

    fn times(self, other: Self, work: &mut TraceWork) -> CoreResult<Self> {
        charge(&mut work.interval_multiplications)?;
        self.mul(other)
    }

    fn negative(self) -> Self {
        Self {
            lo: -self.hi,
            hi: -self.lo,
        }
    }
}

fn even_moments<V: Value>(a: V, b: V, work: &mut TraceWork) -> CoreResult<[V; 3]> {
    charge(&mut work.moment_batches)?;
    let a2 = a.times(a, work)?;
    let s2 = a.times(V::constant(4.0), work)?;
    let a2_plus_b = a2.plus(b, work)?;
    let s4 = a2_plus_b.times(V::constant(4.0), work)?;
    let a3 = a2.times(a, work)?;
    let ab = a.times(b, work)?;
    let four_a3 = a3.times(V::constant(4.0), work)?;
    let twelve_ab = ab.times(V::constant(12.0), work)?;
    let s6 = four_a3
        .plus(twelve_ab, work)?
        .plus(V::constant(3.0), work)?;
    Ok([s2, s4, s6])
}

fn all_moments<V: Value>(a: V, b: V, work: &mut TraceWork) -> CoreResult<[V; 7]> {
    let [s2, s4, s6] = even_moments(a, b, work)?;
    let s5 = a.times(V::constant(-10.0), work)?;
    Ok([
        V::constant(4.0),
        V::constant(0.0),
        s2,
        V::constant(-3.0),
        s4,
        s5,
        s6,
    ])
}

// A polynomial request only needs the moments in its live coefficient support.
// In particular S3=-3 and S5=-10a do not depend on a^2 or a^3. Keep this
// request-specific circuit separate from the full-moment and field APIs.
fn supported_moments(
    a: Interval,
    b: Interval,
    live: &[bool; 7],
    work: &mut TraceWork,
) -> CoreResult<[Interval; 7]> {
    let mut moments = [Interval::constant(0.0); 7];
    moments[0] = Interval::constant(4.0);
    moments[3] = Interval::constant(-3.0);
    if live[2] || live[4] || live[5] || live[6] {
        charge(&mut work.moment_batches)?;
    }
    if live[2] {
        moments[2] = a.times(Interval::constant(4.0), work)?;
    }
    if live[5] {
        moments[5] = a.times(Interval::constant(-10.0), work)?;
    }
    if live[4] || live[6] {
        let a2 = a.times(a, work)?;
        if live[4] {
            moments[4] = a2.plus(b, work)?.times(Interval::constant(4.0), work)?;
        }
        if live[6] {
            let a3 = a2.times(a, work)?;
            let ab = a.times(b, work)?;
            let four_a3 = a3.times(Interval::constant(4.0), work)?;
            let twelve_ab = ab.times(Interval::constant(12.0), work)?;
            moments[6] = four_a3
                .plus(twelve_ab, work)?
                .plus(Interval::constant(3.0), work)?;
        }
    }
    Ok(moments)
}

fn field<V: Value>(a: V, b: V, route: TraceFieldRoute, work: &mut TraceWork) -> CoreResult<[V; 2]> {
    match route {
        TraceFieldRoute::DirectMoments => {
            // Only the three requested moments are computed; no artificial
            // unused S5 work is charged to the direct baseline.
            let [s2, s4, s6] = even_moments(a, b, work)?;
            let g1_s2 = s2.times(V::constant(1.0 / 32.0), work)?;
            let g1_s4 = s4.times(V::constant(1.0 / 128.0), work)?;
            let g1_s6 = s6.times(V::constant(1.0 / 512.0), work)?;
            let g1 = a
                .negative()
                .plus(g1_s2, work)?
                .plus(g1_s4, work)?
                .plus(g1_s6, work)?;
            let g2_s2 = s2.times(V::constant(1.0 / 64.0), work)?;
            let g2_s4 = s4.times(V::constant(1.0 / 64.0), work)?;
            let g2_s6 = s6.times(V::constant(1.0 / 256.0), work)?;
            let g2 = b
                .negative()
                .plus(g2_s2, work)?
                .plus(g2_s4, work)?
                .plus(g2_s6, work)?;
            Ok([g1, g2])
        }
        TraceFieldRoute::SharedPolynomial => {
            charge(&mut work.shared_field_batches)?;
            // N = (a^2*(a+4) + b*(3*a+4))/128 + 3/512.
            // This is exactly T12, with one shared nonlinear scalar.
            let a2 = a.times(a, work)?;
            let a_plus_four = a.plus(V::constant(4.0), work)?;
            let three_a_plus_four = a
                .times(V::constant(3.0), work)?
                .plus(V::constant(4.0), work)?;
            let first = a2.times(a_plus_four, work)?;
            let second = b.times(three_a_plus_four, work)?;
            let n = first
                .plus(second, work)?
                .times(V::constant(1.0 / 128.0), work)?
                .plus(V::constant(3.0 / 512.0), work)?;
            let linear_one = a.times(V::constant(-7.0 / 8.0), work)?;
            let g1 = linear_one.plus(n, work)?;
            let a_over_sixteen = a.times(V::constant(1.0 / 16.0), work)?;
            let two_n = n.times(V::constant(2.0), work)?;
            let g2 = b.negative().plus(a_over_sixteen, work)?.plus(two_n, work)?;
            Ok([g1, g2])
        }
    }
}

/// Enclosures for `Tr(q1^k)`, `k = 0,...,6`. Multiplicity is retained at
/// every parameter, including the triple-root discriminant.
pub fn quartic_moments(
    semantics: TraceSemantics,
    parameters: QuarticParameters,
    work: &mut TraceWork,
) -> CoreResult<[Interval; 7]> {
    request(semantics, work, |work| {
        all_moments(parameters.a, parameters.b, work)
    })
}

/// Enclose a fixed original-variable polynomial invariant. The result is a
/// trace, never the value on a selected individual root.
pub fn quartic_invariant(
    semantics: TraceSemantics,
    parameters: QuarticParameters,
    observable: QuarticInvariant,
    work: &mut TraceWork,
) -> CoreResult<Interval> {
    request(semantics, work, |work| {
        charge(&mut work.invariant_queries)?;
        match observable {
            QuarticInvariant::One => Ok(Interval::constant(4.0)),
            QuarticInvariant::Q1 | QuarticInvariant::Q2 => Ok(Interval::constant(0.0)),
            QuarticInvariant::Q1Squared | QuarticInvariant::Q1SquaredPlusQ2 => {
                parameters.a.times(Interval::constant(4.0), work)
            }
            QuarticInvariant::Q2Squared => parameters.b.times(Interval::constant(4.0), work),
            QuarticInvariant::Q1Q2 => Ok(Interval::constant(3.0)),
            QuarticInvariant::Q1SquaredQ2Squared => parameters
                .a
                .times(parameters.b, work)?
                .times(Interval::constant(4.0), work)?
                .plus(Interval::constant(3.0), work),
        }
    })
}

/// Enclose `Tr(sum_k coefficients[k]*q1^k)` for degree at most six.
/// Coefficients are real enclosures, including any exact-rational conversion
/// allowance. An empty coefficient slice is the zero polynomial. This fixed
/// surface has no general compiler, root filter or implicit degree fallback.
/// Every coefficient is validated before exact-zero support is removed. Only
/// live moments are evaluated; the identically zero first moment needs no
/// arithmetic. A moment batch is charged when a parameter-dependent moment
/// is constructed, not for the fixed constants S0=4 and S3=-3.
pub fn quartic_polynomial_trace(
    semantics: TraceSemantics,
    parameters: QuarticParameters,
    coefficients: &[Interval],
    work: &mut TraceWork,
) -> CoreResult<Interval> {
    request(semantics, work, |work| {
        charge(&mut work.polynomial_queries)?;
        if coefficients.len() > 7 {
            return Err(CoreError::InvalidInput(
                "quartic polynomial trace supports degrees zero through six".into(),
            ));
        }
        for coefficient in coefficients {
            Interval::new(coefficient.lo, coefficient.hi)?;
        }
        let mut live = [false; 7];
        for (k, coefficient) in coefficients.iter().enumerate() {
            live[k] = k != 1 && (coefficient.lo != 0.0 || coefficient.hi != 0.0);
        }
        let moments = supported_moments(parameters.a, parameters.b, &live, work)?;
        let mut result: Option<Interval> = None;
        for (k, &coefficient) in coefficients.iter().enumerate() {
            if live[k] {
                let term = coefficient.times(moments[k], work)?;
                result = Some(match result {
                    Some(sum) => sum.plus(term, work)?,
                    None => term,
                });
            }
        }
        Ok(result.unwrap_or_else(|| Interval::constant(0.0)))
    })
}

/// Represented candidate of the declared F6 aggregate field at one finite
/// binary64 state. Call `quartic_field_enclosure` for a real error enclosure.
pub fn quartic_field(
    semantics: TraceSemantics,
    state: [f64; 2],
    route: TraceFieldRoute,
    work: &mut TraceWork,
) -> CoreResult<[f64; 2]> {
    request(semantics, work, |work| {
        field(finite(state[0])?, finite(state[1])?, route, work)
    })
}

/// Enclosure of the exact declared F6 aggregate field over a parameter box.
/// All coefficient constants are exactly dyadic. The two routes may return
/// different enclosure widths while enclosing the same mathematical target.
pub fn quartic_field_enclosure(
    semantics: TraceSemantics,
    parameters: QuarticParameters,
    route: TraceFieldRoute,
    work: &mut TraceWork,
) -> CoreResult<[Interval; 2]> {
    request(semantics, work, |work| {
        field(parameters.a, parameters.b, route, work)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: TraceSemantics = TraceSemantics::AllComplexRootsAlgebraicMultiplicity;
    const ROUTES: [TraceFieldRoute; 2] = [
        TraceFieldRoute::DirectMoments,
        TraceFieldRoute::SharedPolynomial,
    ];

    // Independent exact integer oracle. a=aq/4, b=bq/4, and the integer
    // matrix below is 16 times multiplication by z in the quotient basis.
    fn exact_companion_moments(aq: i128, bq: i128) -> [f64; 7] {
        let matrix = [
            [0, 0, 0, 4 * bq - aq * aq],
            [16, 0, 0, -16],
            [0, 16, 0, 8 * aq],
            [0, 0, 16, 0],
        ];
        let mut power = [[0_i128; 4]; 4];
        for (i, row) in power.iter_mut().enumerate() {
            row[i] = 1;
        }
        let mut result = [0.0; 7];
        for (k, item) in result.iter_mut().enumerate() {
            let numerator: i128 = (0..4).map(|i| power[i][i]).sum();
            assert!(numerator.abs() < (1_i128 << 53));
            *item = numerator as f64 / 16_i128.pow(k as u32) as f64;
            let mut next = [[0_i128; 4]; 4];
            for i in 0..4 {
                for j in 0..4 {
                    next[i][j] = (0..4).map(|l| power[i][l] * matrix[l][j]).sum();
                }
            }
            power = next;
        }
        result
    }

    #[test]
    fn moments_enclose_integer_quotient_oracle_at_collisions_and_regular_points() {
        for (aq, bq) in [(2, 3), (3, 3), (-1, -1), (8, 8)] {
            let parameters = QuarticParameters::point(aq as f64 / 4.0, bq as f64 / 4.0).unwrap();
            let exact = exact_companion_moments(aq, bq);
            let mut work = TraceWork::default();
            let enclosed = quartic_moments(ALL, parameters, &mut work).unwrap();
            for (interval, value) in enclosed.iter().zip(exact) {
                assert!(interval.contains(value), "{interval:?} excludes {value}");
            }
            assert_eq!(work.requests_completed, 1);
            assert_eq!(work.root_enumerations, 0);
        }
    }

    #[test]
    fn triple_root_is_length_four_and_invariants_keep_multiplicity() {
        let parameters = QuarticParameters::point(0.75, 0.75).unwrap();
        let mut work = TraceWork::default();
        for (query, expected) in [
            (QuarticInvariant::One, 4.0),
            (QuarticInvariant::Q1, 0.0),
            (QuarticInvariant::Q2, 0.0),
            (QuarticInvariant::Q1Squared, 3.0),
            (QuarticInvariant::Q2Squared, 3.0),
            (QuarticInvariant::Q1Q2, 3.0),
            (QuarticInvariant::Q1SquaredQ2Squared, 21.0 / 4.0),
            (QuarticInvariant::Q1SquaredPlusQ2, 3.0),
        ] {
            assert!(
                quartic_invariant(ALL, parameters, query, &mut work)
                    .unwrap()
                    .contains(expected)
            );
        }
        let one = quartic_invariant(ALL, parameters, QuarticInvariant::One, &mut work).unwrap();
        assert!(!one.contains(2.0)); // Deduplicating the roots changes the query.
    }

    #[test]
    fn rejects_branch_and_real_only_targets_before_arithmetic() {
        let parameters = QuarticParameters::point(2.0, 2.0).unwrap();
        let mut work = TraceWork::default();
        for wrong in [TraceSemantics::SelectedReal, TraceSemantics::RealOnly] {
            assert!(quartic_invariant(wrong, parameters, QuarticInvariant::Q1, &mut work).is_err());
            assert!(quartic_field(wrong, [2.0, 2.0], ROUTES[0], &mut work).is_err());
            assert!(quartic_field_enclosure(wrong, parameters, ROUTES[1], &mut work).is_err());
            assert!(quartic_moments(wrong, parameters, &mut work).is_err());
            assert!(quartic_polynomial_trace(wrong, parameters, &[], &mut work).is_err());
        }
        assert_eq!(work.semantics_rejections, 10);
        assert_eq!(work.requests_rejected, 10);
        assert_eq!(
            work.scalar_multiplications + work.interval_multiplications,
            0
        );
        let trace = quartic_invariant(ALL, parameters, QuarticInvariant::Q1, &mut work).unwrap();
        assert!(trace.contains(0.0));
        assert!(!trace.contains(1.0)); // The regular branch (1,1) has q1=1.
        let complex = QuarticParameters::point(-0.25, -0.25).unwrap();
        let square =
            quartic_invariant(ALL, complex, QuarticInvariant::Q1Squared, &mut work).unwrap();
        assert!(square.contains(-1.0));
        assert!(!square.contains(0.5)); // Real-only roots would give 1/2.
    }

    #[test]
    fn both_fields_enclose_independent_integer_moment_target() {
        for (aq, bq) in [(2, 3), (3, 3), (-1, -1), (8, 8)] {
            let a = aq as f64 / 4.0;
            let b = bq as f64 / 4.0;
            let s = exact_companion_moments(aq, bq);
            // All these small dyadic operations are exactly representable.
            let target = [
                -a + s[2] / 32.0 + s[4] / 128.0 + s[6] / 512.0,
                -b + s[2] / 64.0 + s[4] / 64.0 + s[6] / 256.0,
            ];
            for route in ROUTES {
                let mut work = TraceWork::default();
                let parameters = QuarticParameters::point(a, b).unwrap();
                let result = quartic_field(ALL, [a, b], route, &mut work).unwrap();
                let enclosed = quartic_field_enclosure(ALL, parameters, route, &mut work).unwrap();
                assert_eq!(result, target);
                for (interval, exact) in enclosed.iter().zip(target) {
                    assert!(interval.contains(exact));
                }
                assert_eq!(result[1] - 2.0 * result[0], 29.0 * a / 16.0 - b);
                assert_eq!(work.jacobian_evaluations + work.jvp_evaluations, 0);
                assert_eq!(work.ad_evaluations + work.newton_updates, 0);
            }
        }
    }

    #[test]
    fn parameter_boxes_and_non_dyadic_coefficients_are_enclosed() {
        let parameters = QuarticParameters::new(
            Interval::new(0.5, 1.0).unwrap(),
            Interval::new(0.5, 1.0).unwrap(),
        )
        .unwrap();
        let mut work = TraceWork::default();
        let moments = quartic_moments(ALL, parameters, &mut work).unwrap();
        for (k, low, high) in [(2, 2.0, 4.0), (4, 3.0, 8.0), (6, 6.5, 19.0)] {
            assert!(moments[k].contains(low));
            assert!(moments[k].contains(high));
        }
        let third = Interval::point(1.0)
            .unwrap()
            .div(Interval::point(3.0).unwrap())
            .unwrap();
        assert!(third.lo < third.hi);
        let result = quartic_polynomial_trace(ALL, parameters, &[third], &mut work).unwrap();
        assert!(result.lo < result.hi);
        assert!(result.contains(4.0 / 3.0));
        let triple = QuarticParameters::point(0.75, 0.75).unwrap();
        let coefficients = [5.0, 0.0, -2.0, 1.0].map(|v| Interval::point(v).unwrap());
        assert!(
            quartic_polynomial_trace(ALL, triple, &coefficients, &mut work)
                .unwrap()
                .contains(11.0)
        );
    }

    #[test]
    fn constant_and_linear_traces_do_not_evaluate_unused_huge_moments() {
        let parameters = QuarticParameters::point(f64::MAX, 0.0).unwrap();
        let coefficients = [
            Interval::point(1.0).unwrap(),
            Interval::point(f64::MAX).unwrap(),
        ];
        let mut work = TraceWork::default();
        for length in 1..=2 {
            let result =
                quartic_polynomial_trace(ALL, parameters, &coefficients[..length], &mut work)
                    .unwrap();
            assert_eq!(result.as_pair(), [4.0, 4.0]);
        }
        assert_eq!(work.requests_completed, 2);
        assert_eq!(work.moment_batches, 0);
        // A coefficient multiplying a zero trace still has to denote a valid
        // real interval; query validation must precede the shortcut.
        let malformed = [
            coefficients[0],
            Interval {
                lo: f64::NAN,
                hi: 1.0,
            },
        ];
        assert!(quartic_polynomial_trace(ALL, parameters, &malformed, &mut work).is_err());
        assert_eq!(work.requests_rejected, 1);
    }

    #[test]
    fn live_support_matches_exact_companion_traces_for_every_support_mask() {
        // All values in this small dyadic oracle sum are exactly representable.
        // It includes the triple root and a double real root plus conjugate pair.
        for (aq, bq) in [(2, 3), (3, 3), (-1, -1), (8, 8)] {
            let parameters = QuarticParameters::point(aq as f64 / 4.0, bq as f64 / 4.0).unwrap();
            let moments = exact_companion_moments(aq, bq);
            for mask in 0_u16..128 {
                let coefficients = std::array::from_fn::<_, 7, _>(|k| {
                    let value = if mask & (1 << k) == 0 {
                        0.0
                    } else {
                        let sign = if k % 2 == 0 { 1.0 } else { -1.0 };
                        sign * (k as f64 + 1.0) / 4.0
                    };
                    Interval::point(value).unwrap()
                });
                let expected = coefficients
                    .iter()
                    .zip(moments)
                    .map(|(coefficient, moment)| coefficient.lo * moment)
                    .sum::<f64>();
                let mut work = TraceWork::default();
                let actual =
                    quartic_polynomial_trace(ALL, parameters, &coefficients, &mut work).unwrap();
                assert!(
                    actual.contains(expected),
                    "mask={mask}, ({aq},{bq}): {actual:?}"
                );
                assert_eq!(work.requests_attempted, 1);
                assert_eq!(work.requests_completed, 1);
                assert_eq!(work.root_enumerations, 0);
            }
        }
        let triple = QuarticParameters::point(0.75, 0.75).unwrap();
        let coefficients = [0.0, 0.0, 0.0, 1.0].map(|v| Interval::point(v).unwrap());
        let mut work = TraceWork::default();
        let value = quartic_polynomial_trace(ALL, triple, &coefficients, &mut work).unwrap();
        assert_eq!(value.as_pair(), [-3.0, -3.0]);
        assert!(!value.contains(-13.0 / 4.0)); // Distinct-root cubic sum changes the target.
        assert_eq!(work.moment_batches, 0);
    }

    #[test]
    fn finite_sparse_low_moments_do_not_compute_unused_powers() {
        let large = 2.0_f64.powi(600);
        let square_base = 2.0_f64.powi(400);
        for (degree, a, b, exact) in [
            (2, large, f64::MAX, 4.0 * large),
            (3, f64::MAX, f64::MAX, -3.0),
            (4, square_base, 0.0, 2.0_f64.powi(802)),
            (5, large, f64::MAX, -10.0 * large),
        ] {
            let parameters = QuarticParameters::point(a, b).unwrap();
            let mut coefficients = [Interval::constant(0.0); 7];
            coefficients[degree] = Interval::constant(1.0);
            let mut work = TraceWork::default();
            let value =
                quartic_polynomial_trace(ALL, parameters, &coefficients, &mut work).unwrap();
            assert!(value.contains(exact), "degree {degree}: {value:?}");
            assert_eq!(work.requests_completed, 1);
            assert!(work.interval_multiplications <= 3);
        }
        let parameters = QuarticParameters::point(f64::MAX, f64::MAX).unwrap();
        let mut coefficients = [Interval::constant(-0.0); 7];
        coefficients[0] = Interval::constant(1.0);
        coefficients[1] = Interval::constant(f64::MAX);
        let mut work = TraceWork::default();
        let value = quartic_polynomial_trace(ALL, parameters, &coefficients, &mut work).unwrap();
        assert_eq!(value.as_pair(), [4.0, 4.0]);
        assert_eq!(work.moment_batches, 0);
        assert_eq!(work.interval_multiplications, 1);
        assert_eq!(work.interval_additions, 0);
    }

    #[test]
    fn support_liveness_preserves_interval_uncertainty_and_validates_dead_coefficients() {
        let parameters = QuarticParameters::point(0.75, 0.75).unwrap();
        let mut coefficients = [Interval::constant(0.0); 7];
        coefficients[2] = Interval::new(-1.0, 1.0).unwrap();
        let mut work = TraceWork::default();
        let value = quartic_polynomial_trace(ALL, parameters, &coefficients, &mut work).unwrap();
        assert!(value.contains(-3.0) && value.contains(3.0));
        assert_eq!(work.interval_multiplications, 2);
        for (index, malformed) in [
            (
                1,
                Interval {
                    lo: f64::NAN,
                    hi: 0.0,
                },
            ),
            (6, Interval { lo: 1.0, hi: -1.0 }),
            (
                6,
                Interval {
                    lo: 0.0,
                    hi: f64::INFINITY,
                },
            ),
        ] {
            coefficients[index] = malformed;
            let mut rejected = TraceWork::default();
            assert!(
                quartic_polynomial_trace(ALL, parameters, &coefficients, &mut rejected).is_err()
            );
            assert_eq!(rejected.requests_rejected, 1);
            assert_eq!(rejected.moment_batches, 0);
            assert_eq!(rejected.interval_multiplications, 0);
            coefficients[index] = Interval::constant(0.0);
        }
    }

    #[test]
    fn required_sixth_moment_overflow_is_rejected_and_charged() {
        let parameters = QuarticParameters::point(f64::MAX, 0.0).unwrap();
        let mut coefficients = [Interval::constant(0.0); 7];
        coefficients[6] = Interval::constant(1.0);
        let mut work = TraceWork::default();
        assert!(quartic_polynomial_trace(ALL, parameters, &coefficients, &mut work).is_err());
        assert_eq!(work.requests_attempted, 1);
        assert_eq!(work.requests_rejected, 1);
        assert_eq!(work.requests_completed, 0);
        assert_eq!(work.polynomial_queries, 1);
        assert_eq!(work.moment_batches, 1);
        assert!(work.interval_multiplications > 0);
    }

    #[test]
    fn malformed_nonfinite_degree_and_overflow_inputs_are_rejected_with_work() {
        assert!(
            QuarticParameters::new(
                Interval {
                    lo: f64::NAN,
                    hi: 1.0
                },
                Interval::point(0.0).unwrap(),
            )
            .is_err()
        );
        assert!(
            QuarticParameters::new(Interval { lo: 1.0, hi: 0.0 }, Interval::point(0.0).unwrap(),)
                .is_err()
        );
        let parameters = QuarticParameters::point(0.75, 0.75).unwrap();
        let mut work = TraceWork::default();
        assert!(
            quartic_polynomial_trace(
                ALL,
                parameters,
                &[Interval {
                    lo: 0.0,
                    hi: f64::INFINITY
                }],
                &mut work,
            )
            .is_err()
        );
        assert!(
            quartic_polynomial_trace(
                ALL,
                parameters,
                &[Interval::point(1.0).unwrap(); 8],
                &mut work,
            )
            .is_err()
        );
        assert!(quartic_field(ALL, [f64::NAN, 0.0], ROUTES[0], &mut work).is_err());
        for route in ROUTES {
            assert!(quartic_field(ALL, [f64::MAX, 0.0], route, &mut work).is_err());
            let huge = QuarticParameters::point(f64::MAX, 0.0).unwrap();
            assert!(quartic_field_enclosure(ALL, huge, route, &mut work).is_err());
        }
        assert_eq!(work.requests_attempted, 7);
        assert_eq!(work.requests_rejected, 7);
        assert_eq!(work.requests_completed, 0);
        assert!(work.scalar_multiplications > 0);
        assert!(work.interval_multiplications > 0);
    }

    #[test]
    fn tiny_represented_inputs_preserve_nonzero_moment_enclosure() {
        let tiny = f64::from_bits(1);
        let parameters = QuarticParameters::point(tiny, 0.0).unwrap();
        let mut work = TraceWork::default();
        let result =
            quartic_invariant(ALL, parameters, QuarticInvariant::Q1Squared, &mut work).unwrap();
        assert!(result.contains(f64::from_bits(4)));
        assert!(result.hi > 0.0);
    }

    #[test]
    fn shared_field_removes_arithmetic_without_charging_unused_direct_moments() {
        let mut direct = TraceWork::default();
        let mut shared = TraceWork::default();
        let state = [0.625, 0.875];
        quartic_field(ALL, state, ROUTES[0], &mut direct).unwrap();
        quartic_field(ALL, state, ROUTES[1], &mut shared).unwrap();
        assert_eq!(direct.moment_batches, 1);
        assert_eq!(shared.shared_field_batches, 1);
        assert!(shared.scalar_additions < direct.scalar_additions);
        assert!(shared.scalar_multiplications < direct.scalar_multiplications);
        assert_eq!(direct.root_enumerations + shared.root_enumerations, 0);
    }
}
