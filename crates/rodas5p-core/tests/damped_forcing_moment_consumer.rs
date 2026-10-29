use rodas5p_core::damped_moments::compute_damped_moment;
use rodas5p_core::damped_moments::{
    DampedForcingMomentRefusal, DampedForcingMomentWork, compute_damped_forcing_moment,
};
use rodas5p_core::directed::{Interval, sub_up};

#[test]
fn forcing_index_mapping_is_q_rd_to_i_dr() {
    let mut work = DampedForcingMomentWork::default();
    let result = compute_damped_forcing_moment(2.0, 1.0, 0, 1, 1.0, &mut work).unwrap();
    let direct = compute_damped_moment(2.0, 1.0, 0, 1).unwrap();
    let swapped = compute_damped_moment(2.0, 1.0, 1, 0).unwrap();

    assert_eq!(result.interval, direct);
    assert!(result.interval.hi < swapped.lo || swapped.hi < result.interval.lo);
    assert_eq!(
        result.width_upper,
        sub_up(direct.hi, direct.lo).expect("outward width")
    );
    assert_eq!(work.validation_attempts, 1);
    assert_eq!(work.kernel_attempts, 1);
    assert_eq!(work.width_checks, 1);
}

#[test]
fn absolute_width_gate_refuses_narrow_limit_and_keeps_attempted_work() {
    let mut work = DampedForcingMomentWork::default();
    let result = compute_damped_forcing_moment(2.0, 1.0, 0, 1, 0.0, &mut work);
    assert!(matches!(
        result,
        Err(DampedForcingMomentRefusal::WidthExceeded { .. })
    ));
    assert_eq!(work.validation_attempts, 1);
    assert_eq!(work.kernel_attempts, 1);
    assert_eq!(work.width_checks, 1);
}

#[test]
fn invalid_parameters_and_orders_refuse_before_kernel_and_retain_validation_work() {
    for (a, h, d, r, max_width) in [
        (f64::NAN, 1.0, 0, 0, 1.0),
        (-1.0, 1.0, 0, 0, 1.0),
        (1.0, f64::INFINITY, 0, 0, 1.0),
        (1.0, 0.0, 0, 0, 1.0),
        (1.0, 1.0, 65, 0, 1.0),
        (1.0, 1.0, 0, 65, 1.0),
        (1.0, 1.0, 0, 0, f64::NAN),
        (1.0, 1.0, 0, 0, -f64::EPSILON),
    ] {
        let mut work = DampedForcingMomentWork::default();
        assert!(matches!(
            compute_damped_forcing_moment(a, h, d, r, max_width, &mut work),
            Err(DampedForcingMomentRefusal::InvalidInput(_))
        ));
        assert_eq!(work.validation_attempts, 1);
        assert_eq!(work.kernel_attempts, 0);
        assert_eq!(work.width_checks, 0);
    }
}

#[test]
fn kernel_refusal_retains_attempt_without_claiming_width_check() {
    let mut work = DampedForcingMomentWork::default();
    assert!(matches!(
        compute_damped_forcing_moment(f64::MAX, 2.0, 0, 0, 1.0, &mut work),
        Err(DampedForcingMomentRefusal::Kernel(_))
    ));
    assert_eq!(work.validation_attempts, 1);
    assert_eq!(work.kernel_attempts, 1);
    assert_eq!(work.width_checks, 0);
}

#[test]
fn zero_damping_confluent_limit_is_admitted_by_the_consumer() {
    let mut work = DampedForcingMomentWork::default();
    let result = compute_damped_forcing_moment(0.0, 0.5, 1, 0, 1.0, &mut work).unwrap();
    // I_{1,0}(0, h) = h^2 / 2 = 1/8.
    assert!(result.interval.contains(0.125));
    assert!(result.width_upper <= 1.0);
}

#[test]
fn subnormal_damping_product_is_not_treated_as_exact_zero() {
    let a = f64::from_bits(1);
    let h = 0.5;
    let z = Interval::point(a)
        .unwrap()
        .mul(Interval::point(h).unwrap())
        .unwrap();
    assert!(z.lo < 0.0);
    assert_eq!(z.hi, f64::from_bits(1));

    let direct = compute_damped_moment(a, h, 0, 0).unwrap();
    assert!(
        direct.lo < 0.5,
        "the tiny positive damping is not exact zero"
    );
    let mut work = DampedForcingMomentWork::default();
    let result = compute_damped_forcing_moment(a, h, 0, 0, 1.0, &mut work).unwrap();
    assert_eq!(result.interval, direct);
    assert!(result.interval.contains(0.5));
    assert_eq!(
        work,
        DampedForcingMomentWork {
            validation_attempts: 1,
            kernel_attempts: 1,
            width_checks: 1,
        }
    );
}

#[test]
fn rounded_product_branch_boundary_matches_frozen_kernel_interval() {
    let z = Interval::point(0.1)
        .unwrap()
        .mul(Interval::point(10.0).unwrap())
        .unwrap();
    assert!(z.lo <= 1.0 && z.hi > 1.0, "product enclosure={z:?}");

    let direct = compute_damped_moment(0.1, 10.0, 0, 0).unwrap();
    let mut work = DampedForcingMomentWork::default();
    let result = compute_damped_forcing_moment(0.1, 10.0, 0, 0, 1.0, &mut work).unwrap();
    assert_eq!(result.interval, direct);
    assert_eq!(direct.lo.to_bits(), 0x4019_48ea_1e23_ea6f);
    assert_eq!(direct.hi.to_bits(), 0x4019_48ea_1e23_ea8c);
    assert_eq!(work.kernel_attempts, 1);
    assert_eq!(work.width_checks, 1);
}

#[test]
fn large_rate_width_refusal_keeps_validation_kernel_and_width_work() {
    let mut work = DampedForcingMomentWork::default();
    let result = compute_damped_forcing_moment(1.0e6, 1.0, 0, 0, 0.0, &mut work);
    assert!(matches!(
        result,
        Err(DampedForcingMomentRefusal::WidthExceeded { .. })
    ));
    assert_eq!(work.validation_attempts, 1);
    assert_eq!(work.kernel_attempts, 1);
    assert_eq!(work.width_checks, 1);
}
