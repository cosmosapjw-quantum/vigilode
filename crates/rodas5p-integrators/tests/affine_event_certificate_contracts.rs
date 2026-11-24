//! SYM02 scalar-affine native/represented contract tests.
//! Synthetic packets have explicit DeclaredRepresented provenance. Native
//! fixtures run only the small scalar affine equation stated in the test.

use rodas5p_core::{DenseMatrix, directed::Interval};
use rodas5p_integrators::{
    AdaptiveStepConfig, FastInteriorControl, OdeProblem,
    affine_event_certificate::{
        AFFINE_EVENT_CLOCK_ID, AffineEventBinding, AffineEventCertificate, AffineEventSign,
        AffineEventSpec, AffineEventStatus, AffineFlowDirection, AffineScalarModel,
        AffineSegmentOrigin, RepresentedAffineSegment, check_affine_event_arithmetic,
    },
    integrate_rodas5p_fast_dense_observed,
};
use std::sync::Arc;

fn spec(source: &str, epoch: u64, threshold: f64) -> AffineEventSpec {
    AffineEventSpec::new(
        AffineEventBinding::new(
            "scalar-affine-lambda1-b0",
            source,
            AFFINE_EVENT_CLOCK_ID,
            "dimensionless-state;unit-time",
            "x-threshold",
            epoch,
        )
        .unwrap(),
        AffineScalarModel::new(Interval::point(1.0).unwrap(), Interval::point(0.0).unwrap())
            .unwrap(),
        threshold,
    )
    .unwrap()
}

fn packet(
    times: [f64; 2],
    states: [f64; 2],
    d: [f64; 3],
    index: usize,
) -> RepresentedAffineSegment {
    let mut h = [[0.0; 8]; 3];
    for (r, row) in h.iter_mut().enumerate() {
        row[r] = 1.0;
    }
    let mut u = [0.0; 8];
    u[..3].copy_from_slice(&d);
    RepresentedAffineSegment::new(times, states, h, u, index).unwrap()
}

fn example() -> (AffineEventCertificate, AffineEventSpec) {
    let s = spec("declared-pr100-taylor-polynomial", 0, 15.0 / 16.0);
    let p = packet(
        [0.0, 1.0 / 8.0],
        [1.0, 113.0 / 128.0],
        [-1.0 / 128.0, 0.0, 0.0],
        0,
    );
    (AffineEventCertificate::certify(p, &s, 0.0).unwrap(), s)
}

#[test]
fn pr100_exact_example_gets_outward_native_formula_enclosure() {
    let (c, s) = example();
    assert_eq!(c.origin(), AffineSegmentOrigin::DeclaredRepresented);
    assert!(c.defect_upper() >= 1.0 / 128.0);
    assert!(c.defect_upper() < (1.0 / 128.0) * 1.000000000001);
    assert!(c.endpoint_error_upper() >= 1.0 / 1088.0);
    assert!(c.endpoint_error_upper() < (1.0 / 1088.0) * 1.000000000001);
    assert_eq!(
        c.endpoint_symbols(),
        [AffineEventSign::Positive, AffineEventSign::Negative]
    );
    assert_eq!(c.flow_direction(), AffineFlowDirection::StrictlyDecreasing);
    assert_eq!(c.event_status(), AffineEventStatus::UniqueCrossing);
    for (t, expected) in [
        (1.0 / 16.0, AffineEventSign::Positive),
        (1.0 / 15.0, AffineEventSign::Negative),
        (33.0 / 512.0, AffineEventSign::Unresolved),
    ] {
        let value = 1.0 - t + 0.5 * t * t;
        assert_eq!(c.query(&s, t, value).unwrap().symbol, expected);
    }
}

#[test]
fn endpoint_copy_and_supplied_center_rounding_are_separate() {
    let (c, s) = example();
    let left = c.query(&s, 0.0, 1.0).unwrap();
    assert_eq!(left.error_upper, 0.0);
    assert_eq!(left.evaluation_rounding_upper, 0.0);
    let right = c.query(&s, 1.0 / 8.0, 113.0 / 128.0).unwrap();
    assert_eq!(right.evaluation_rounding_upper, 0.0);
    assert_eq!(right.error_upper, c.endpoint_error_upper());
    // A changed center is never granted an endpoint-copy zero radius.
    let changed = c.query(&s, 0.0, 1.0_f64.next_up()).unwrap();
    assert!(changed.evaluation_rounding_upper >= f64::EPSILON);
    assert!(changed.state_enclosure.contains(1.0));
    let t = 1.0 / 16.0;
    let shifted = c.query(&s, t, 1.0).unwrap();
    assert!(shifted.evaluation_rounding_upper > 0.05);
    assert!(shifted.state_enclosure.contains((-t).exp()));
}

#[test]
fn quartic_last_row_is_retained_in_residual_and_query() {
    let s = spec("declared-quartic", 0, 2.0);
    let c = AffineEventCertificate::certify(
        packet([0.0, 1.0], [1.0, 1.0], [0.0, 0.0, 1.0], 0),
        &s,
        0.0,
    )
    .unwrap();
    assert!(c.power_coefficients()[4].contains(-1.0));
    assert!(c.power_coefficients()[3].contains(1.0));
    assert!(
        c.query(&s, 0.5, 1.0625)
            .unwrap()
            .represented_polynomial
            .contains(1.0625)
    );
    assert!(c.defect_upper() >= 1.0);
}

#[test]
fn exact_clock_difference_encloses_rounded_subtraction_residue() {
    let s = spec("declared-wide-clock", 0, 2.0);
    let c =
        AffineEventCertificate::certify(packet([-1.0e16, 1.0], [0.0, 0.0], [0.0; 3], 0), &s, 0.0)
            .unwrap();
    assert_eq!(1.0 - (-1.0e16), 1.0e16);
    assert_eq!(c.clock_width().lo, 1.0e16);
    assert_eq!(c.clock_width().hi, 1.0e16_f64.next_up());
    assert_eq!(c.defect_upper(), 0.0);
    let large = 1.0e12_f64;
    let c = AffineEventCertificate::certify(
        packet([large, large.next_up()], [1.0, 1.0], [0.0; 3], 0),
        &s,
        0.0,
    )
    .unwrap();
    assert!(c.clock_width().contains(large.next_up() - large));
    assert!(!c.clock_width().contains(1.0e-4));
}

#[test]
fn equality_grazing_and_endpoint_only_evidence_remain_unresolved() {
    let s = spec("declared-equilibrium", 0, 0.0);
    let c = AffineEventCertificate::certify(packet([0.0, 1.0], [0.0, 0.0], [0.0; 3], 0), &s, 0.0)
        .unwrap();
    assert_eq!(c.endpoint_symbols(), [AffineEventSign::Unresolved; 2]);
    assert_eq!(c.flow_direction(), AffineFlowDirection::Unresolved);
    assert_eq!(c.event_status(), AffineEventStatus::Unresolved);
    let s = spec("declared-hidden-interior", 0, 0.5);
    let c = AffineEventCertificate::certify(
        packet([0.0, 1.0], [1.0, 1.0], [-4.0, 0.0, 0.0], 0),
        &s,
        0.0,
    )
    .unwrap();
    assert!(c.defect_upper() > 1.0);
    assert_eq!(c.event_status(), AffineEventStatus::Unresolved);
}

#[test]
fn invalid_parameters_and_public_interval_literals_are_refused() {
    assert!(
        AffineScalarModel::new(
            Interval {
                lo: 1.0,
                hi: f64::NAN
            },
            Interval { lo: 0.0, hi: 0.0 }
        )
        .is_err()
    );
    assert!(
        AffineScalarModel::new(Interval { lo: 0.0, hi: 1.0 }, Interval { lo: 0.0, hi: 0.0 })
            .is_err()
    );
    assert!(
        AffineScalarModel::new(Interval { lo: 2.0, hi: 1.0 }, Interval { lo: 0.0, hi: 0.0 })
            .is_err()
    );
    assert!(AffineEventBinding::new("m", "", "bad", "u", "e", 0).is_err());
    assert!(AffineEventBinding::new("m", "source", "nominal-accumulated-v0", "u", "e", 0).is_err());
    let (c, s) = example();
    assert!(c.query(&s, f64::NAN, 1.0).is_err());
    assert!(c.query(&s, 0.2, 1.0).is_err());
    assert!(c.query(&s, 0.05, f64::INFINITY).is_err());
    for error in [-1.0, f64::NAN, f64::INFINITY] {
        assert!(
            AffineEventCertificate::certify(packet([0.0, 1.0], [1.0, 0.5], [0.0; 3], 0), &s, error)
                .is_err()
        );
    }
}

#[test]
fn changed_source_epoch_event_and_model_invalidate_queries() {
    let (c, s) = example();
    for changed in [
        spec("other-source", 0, 15.0 / 16.0),
        spec(s.binding().source_id(), 1, 15.0 / 16.0),
        spec(s.binding().source_id(), 0, 0.9),
    ] {
        assert!(c.query(&changed, 0.05, 0.95).is_err());
    }
    let changed = AffineEventSpec::new(
        s.binding().clone(),
        AffineScalarModel::new(Interval::point(2.0).unwrap(), Interval::point(0.0).unwrap())
            .unwrap(),
        s.threshold(),
    )
    .unwrap();
    assert!(c.query(&changed, 0.05, 0.95).is_err());
    for (model, units, event) in [
        ("other-model", s.binding().unit_id(), s.binding().event_id()),
        (
            s.binding().model_id(),
            "other-state-or-time-unit",
            s.binding().event_id(),
        ),
        (
            s.binding().model_id(),
            s.binding().unit_id(),
            "other-event-callback",
        ),
    ] {
        let binding = AffineEventBinding::new(
            model,
            s.binding().source_id(),
            AFFINE_EVENT_CLOCK_ID,
            units,
            event,
            s.binding().epoch(),
        )
        .unwrap();
        let changed = AffineEventSpec::new(binding, s.model(), s.threshold()).unwrap();
        assert!(c.query(&changed, 0.05, 0.95).is_err());
    }
}

#[test]
fn interior_normalization_underflow_does_not_change_time_membership() {
    let s = AffineEventSpec::new(
        spec("normalization-underflow", 0, 1.0).binding().clone(),
        AffineScalarModel::new(
            Interval::point(1.0e-200).unwrap(),
            Interval::point(0.0).unwrap(),
        )
        .unwrap(),
        1.0,
    )
    .unwrap();
    let c =
        AffineEventCertificate::certify(packet([0.0, 1.0e200], [0.0, 0.0], [0.0; 3], 0), &s, 0.0)
            .unwrap();
    let t = f64::from_bits(1);
    assert!(t > 0.0 && t < 1.0e200);
    assert_eq!(t / 1.0e200, 0.0);
    let q = c.query(&s, t, 0.0).unwrap();
    assert_eq!(q.time.to_bits(), t.to_bits());
    assert_eq!(q.symbol, AffineEventSign::Negative);
    assert!(c.query(&s, -t, 0.0).is_err());
}

#[test]
fn error_carry_requires_exact_adjacency_state_and_binding() {
    let (c, s) = example();
    let carry = c.carry().unwrap();
    let next = packet([0.125, 0.25], [113.0 / 128.0, 0.8], [0.0; 3], 1);
    let next_certificate = carry.certify_next(next, &s).unwrap();
    assert_eq!(next_certificate.initial_error(), c.endpoint_error_upper());
    for next in [
        packet(
            [0.125_f64.next_up(), 0.25],
            [113.0 / 128.0, 0.8],
            [0.0; 3],
            1,
        ),
        packet([0.125, 0.25], [0.8, 0.8], [0.0; 3], 1),
        packet([0.125, 0.25], [113.0 / 128.0, 0.8], [0.0; 3], 2),
    ] {
        assert!(carry.certify_next(next, &s).is_err());
    }
    let reset = spec(s.binding().source_id(), 1, s.threshold());
    assert!(
        carry
            .certify_next(
                packet([0.125, 0.25], [113.0 / 128.0, 0.8], [0.0; 3], 1),
                &reset
            )
            .is_err()
    );
}

#[test]
fn parameter_boxes_and_nonzero_incoming_error_are_included() {
    let s = AffineEventSpec::new(
        spec("parameter-box", 0, 0.5).binding().clone(),
        AffineScalarModel::new(
            Interval::new(0.9, 1.1).unwrap(),
            Interval::new(-0.01, 0.01).unwrap(),
        )
        .unwrap(),
        0.5,
    )
    .unwrap();
    let c = AffineEventCertificate::certify(
        packet(
            [0.0, 0.125],
            [1.0, 113.0 / 128.0],
            [-1.0 / 128.0, 0.0, 0.0],
            0,
        ),
        &s,
        0.01,
    )
    .unwrap();
    assert!(c.whole_slab_error_upper() >= 0.01);
    assert!(c.defect_upper() >= 0.1);
    for lambda in [0.9_f64, 1.0, 1.1] {
        for forcing in [-0.01, 0.0, 0.01] {
            for x0 in [0.99, 1.0, 1.01] {
                let t = 0.0625;
                let exact = forcing / lambda + (x0 - forcing / lambda) * (-lambda * t).exp();
                assert!(
                    c.query(&s, t, 1.0 - t + 0.5 * t * t)
                        .unwrap()
                        .state_enclosure
                        .contains(exact)
                );
            }
        }
    }
}

#[test]
fn subnormal_inputs_stay_enclosed_and_overflow_refuses() {
    let tiny = f64::from_bits(1);
    let base = spec("subnormal", 0, 0.0);
    let s = AffineEventSpec::new(
        base.binding().clone(),
        AffineScalarModel::new(
            Interval::point(1.0).unwrap(),
            Interval::point(tiny).unwrap(),
        )
        .unwrap(),
        0.0,
    )
    .unwrap();
    let c = AffineEventCertificate::certify(packet([0.0, 1.0], [tiny, tiny], [0.0; 3], 0), &s, 0.0)
        .unwrap();
    assert!(
        c.query(&s, 0.5, tiny)
            .unwrap()
            .state_enclosure
            .contains(tiny)
    );
    let mut h = [[0.0; 8]; 3];
    h[0][0] = f64::MAX;
    let mut u = [0.0; 8];
    u[0] = 2.0;
    let p = RepresentedAffineSegment::new([0.0, 1.0], [1.0, 1.0], h, u, 0).unwrap();
    assert!(AffineEventCertificate::certify(p, &s, 0.0).is_err());
}

fn scalar_native_run(dimension: usize) -> rodas5p_integrators::Rodas5pFastDenseResult {
    let rhs = Arc::new(|_t: f64, y: &[f64], out: &mut [f64]| {
        for (value, &state) in out.iter_mut().zip(y) {
            *value = -state;
        }
        Ok(())
    });
    let jacobian = Arc::new(move |_t: f64, _y: &[f64]| {
        let mut j = DenseMatrix::zeros(dimension, dimension);
        for k in 0..dimension {
            j[(k, k)] = -1.0;
        }
        Ok(j)
    });
    let problem = OdeProblem::new(
        "SYM02-declared-native-affine",
        dimension,
        rhs,
        None,
        Some(jacobian),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let config = AdaptiveStepConfig {
        atol: 1.0e-9,
        rtol: 1.0e-6,
        initial_step: 0.0625,
        min_step: 1.0e-10,
        max_step: 0.0625,
        max_attempts: 128,
        ..AdaptiveStepConfig::default()
    };
    let result = integrate_rodas5p_fast_dense_observed(
        &problem,
        (0.0, 0.125),
        &vec![1.0; dimension],
        &config,
        FastInteriorControl::Off,
    )
    .unwrap();
    assert!(result.partial.is_none());
    result
}

#[test]
fn actual_native_accepted_segment_adapter_and_error_carry() {
    let result = scalar_native_run(1);
    let s = spec("native-fast-rodas-affine-source", 0, 0.9375);
    assert!(result.segments.len() >= 2);
    let mut certificate = result
        .segments
        .certify_affine_scalar_segment(0, &s, 0.0)
        .unwrap();
    assert_eq!(
        certificate.origin(),
        AffineSegmentOrigin::NativeAcceptedRodas5pFast
    );
    for k in 0..result.segments.len() {
        if k > 0 {
            certificate = result
                .segments
                .certify_affine_scalar_segment_with_carry(k, &s, &certificate.carry().unwrap())
                .unwrap();
        }
        let [left, right] = certificate.times();
        for t in [left, left + 0.5 * (right - left), right] {
            let query = result
                .segments
                .query_affine_scalar_certificate(&certificate, &s, t)
                .unwrap();
            let analytic = (-t).exp();
            // exp() is a diagnostic; the exact-rational oracle is a
            // separate independent check of all outward algebraic bounds.
            assert!(
                (query.actual_center - analytic).abs() <= query.error_upper + 4.0 * f64::EPSILON
            );
            let mut actual = [0.0];
            result.segments.evaluate_into(t, &mut actual).unwrap();
            assert_eq!(query.actual_center.to_bits(), actual[0].to_bits());
        }
    }
    assert!(
        result
            .segments
            .certify_affine_scalar_segment(result.segments.len(), &s, 0.0)
            .is_err()
    );
    let other = scalar_native_run(2);
    assert!(
        other
            .segments
            .certify_affine_scalar_segment(0, &s, 0.0)
            .is_err()
    );
}

#[cfg(target_arch = "x86_64")]
#[test]
fn arithmetic_probe_rejects_all_other_rounding_modes_and_ftz_daz() {
    // This test crate may manipulate the caller-owned thread environment;
    // the production crate preserves forbid(unsafe_code). Always restore it.
    struct Restore(u32);
    impl Drop for Restore {
        fn drop(&mut self) {
            // SAFETY: a valid saved MXCSR image, with reserved bits unchanged.
            unsafe {
                core::arch::asm!("ldmxcsr [{}]",in(reg)&self.0,options(nostack,preserves_flags));
            }
        }
    }
    let mut old = 0u32;
    // SAFETY: the pointer addresses a writable aligned 32-bit stack value.
    unsafe {
        core::arch::asm!("stmxcsr [{}]",in(reg)&mut old,options(nostack,preserves_flags));
    }
    let restore = Restore(old);
    let clean = old & !(0x6000 | 0x8040);
    let (certificate, specification) = example();
    for mask in [0u32, 0x2000, 0x4000, 0x6000, 0x8000, 0x40, 0x8040] {
        let value = clean | mask;
        // SAFETY: only rounding and FTZ/DAZ flags of a saved valid image vary.
        unsafe {
            core::arch::asm!("ldmxcsr [{}]",in(reg)&value,options(nostack,preserves_flags));
        }
        let accepted = check_affine_event_arithmetic().is_ok();
        let query_accepted = certificate.query(&specification, 0.0, 1.0).is_ok();
        // Restore before assertions, including their formatting/cleanup.
        unsafe {
            core::arch::asm!("ldmxcsr [{}]",in(reg)&clean,options(nostack,preserves_flags));
        }
        assert_eq!(accepted, mask == 0, "MXCSR mode {mask:#x}");
        assert_eq!(
            query_accepted,
            mask == 0,
            "query under MXCSR mode {mask:#x}"
        );
    }
    drop(restore);
}
