use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use rodas5p_core::directed::Interval;
use rodas5p_core::{CoreError, CoreResult, WorkCounters};
use rodas5p_integrators::{
    BoundPr101Problem, CollectiveModel, ContinuousResidualProvider, DenseOutputError,
    DenseSegmentWork, OdeProblem, Pr101Error, Rodas5pQuarticSegment, StepResult,
};

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static TEST_LOCK: Mutex<()> = Mutex::new(());

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

fn lock_tests() -> std::sync::MutexGuard<'static, ()> {
    TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn calibrate_allocator() -> usize {
    let model = CollectiveModel::new(
        vec![0.0],
        vec![1.0],
        vec![(0.125, -0.0625)],
        0.5,
        0.0,
        0.0,
        0.0,
    )
    .unwrap();
    let before = ALLOCATIONS.load(Ordering::SeqCst);
    let cloned = std::hint::black_box(model.clone());
    std::hint::black_box(cloned.xi().as_ptr());
    std::hint::black_box(cloned.weights().as_ptr());
    std::hint::black_box(cloned.initial_amplitudes().as_ptr());
    let observed = ALLOCATIONS.load(Ordering::SeqCst) - before;
    assert!(
        observed >= 3,
        "ALLOCATION_CALIBRATION_FAILED: library model clone with three Vec buffers measured {observed} allocations"
    );
    observed
}

fn linear_segment(t_old: f64, h: f64) -> Rodas5pQuarticSegment {
    let step = StepResult {
        t_old,
        t_new: t_old + h,
        y_old: vec![t_old.sin()],
        y_new: vec![t_old.sin() + h],
        h,
        stages: vec![vec![0.0]; 8],
        error_vector: vec![0.0],
        error_norm: 0.0,
        accepted: true,
        method: "PR101 fixture".into(),
        used_fallback: false,
        certificate: None,
        counters: WorkCounters::default(),
    };
    Rodas5pQuarticSegment::from_step(&step, &mut DenseSegmentWork::default()).unwrap()
}

fn near_one_quartic_segment() -> (Rodas5pQuarticSegment, f64, f64) {
    let t_old: f64 = 1023.0 / 1024.0;
    let h = 2.0_f64.powi(-12);
    let y_old = t_old.sin();
    let stages = (0..8)
        .map(|index| vec![(index + 1) as f64 * 2.0_f64.powi(-32)])
        .collect();
    let step = StepResult {
        t_old,
        t_new: t_old + h,
        y_old: vec![y_old],
        y_new: vec![y_old + h],
        h,
        stages,
        error_vector: vec![0.0],
        error_norm: 0.0,
        accepted: true,
        method: "PR101 near-one quartic fixture".into(),
        used_fallback: false,
        certificate: None,
        counters: WorkCounters::default(),
    };
    (
        Rodas5pQuarticSegment::from_step(&step, &mut DenseSegmentWork::default()).unwrap(),
        t_old,
        h,
    )
}

fn print_interval_list(label: &str, intervals: &[Interval]) {
    print!("{label}=");
    for (index, interval) in intervals.iter().enumerate() {
        if index != 0 {
            print!(";");
        }
        print!("{:.17e},{:.17e}", interval.lo, interval.hi);
    }
    println!();
}

#[test]
fn pr101_exact_dyadic_linear_segment_encloses_endpoints_and_midpoint() {
    let _lock = lock_tests();
    let owner = BoundPr101Problem::new().unwrap();
    assert_eq!(owner.dimension(), 1);
    let h = f64::from_bits(0x3eb0_0000_0000_0000); // exactly 2^-20
    let segment = linear_segment(0.0, h);
    let mut work = DenseSegmentWork::default();
    let certificate = owner
        .verify_segment(&segment, 0.0, 1.0e-10, &mut work)
        .unwrap();
    println!("PR101_FRACTION_BOUND={:.17e}", certificate.residual_upper());
    assert!(certificate.log_norm_upper() < -999_999.0);
    assert!(certificate.residual_upper().is_finite());
    assert!(certificate.required_tube_radius() <= certificate.tube_radius());

    for time in [0.0, 0.5 * h, h] {
        let enclosure = owner
            .output_enclosure(&segment, &certificate, time, &mut work)
            .unwrap();
        let exact = time.sin();
        assert!((enclosure.center() - exact).abs() <= enclosure.radius());
        assert!(enclosure.within_normalized_tolerance());
    }
    assert!(work.provider_operations > 0);
}

#[test]
fn pr101_near_one_quartic_emits_fraction_oracle_fixture() {
    let _lock = lock_tests();
    let owner = BoundPr101Problem::new().unwrap();
    let (segment, t_old, h) = near_one_quartic_segment();
    let mut work = DenseSegmentWork::default();
    let certificate = owner
        .verify_segment(&segment, 1.0e-15, 1.0e-3, &mut work)
        .unwrap();
    println!();
    let q = certificate.quartic_coefficient_intervals();
    let q_intervals = q.as_slice();
    print_interval_list("PR101_NEAR1_Q", q_intervals);
    print_interval_list(
        "PR101_NEAR1_RPOLY",
        certificate.residual_coefficient_intervals(),
    );
    println!("PR101_NEAR1_A={t_old:.17e}");
    println!("PR101_NEAR1_H={h:.17e}");
    println!(
        "PR101_NEAR1_DELTA={:.17e},{:.17e}",
        certificate.delta_interval().lo,
        certificate.delta_interval().hi
    );
    println!("PR101_NEAR1_RESIDUAL={:.17e}", certificate.residual_upper());

    let time = t_old + h;
    let mut state = [0.0];
    segment
        .evaluate_center_at_into(time, &mut state, &mut work)
        .unwrap();
    let mut rhs = [Interval::point(0.0).unwrap()];
    owner
        .enclose_rhs(
            Interval::point(time).unwrap(),
            &[Interval::point(state[0]).unwrap()],
            &mut rhs,
            &mut work,
        )
        .unwrap();
    println!("PR101_NEAR1_T={time:.17e}");
    println!("PR101_NEAR1_Y={:.17e}", state[0]);
    println!("PR101_NEAR1_RHS={:.17e},{:.17e}", rhs[0].lo, rhs[0].hi);

    let output = owner
        .output_enclosure(&segment, &certificate, time, &mut work)
        .unwrap();
    println!(
        "PR101_NEAR1_OUTPUT={:.17e},{:.17e}",
        output.center(),
        output.radius()
    );
    assert!(
        certificate
            .quartic_coefficient_intervals()
            .iter()
            .skip(2)
            .all(|coefficient| coefficient.lo != 0.0 || coefficient.hi != 0.0)
    );
    assert!(work.provider_operations > 0);
}

#[test]
fn pr101_certificate_mismatch_and_rejected_tube_preserve_work() {
    let _lock = lock_tests();
    let owner = BoundPr101Problem::new().unwrap();
    let (segment, t_old, h) = near_one_quartic_segment();
    let mut work = DenseSegmentWork::default();
    let certificate = owner
        .verify_segment(&segment, 1.0e-15, 1.0e-3, &mut work)
        .unwrap();
    let certificate_snapshot = certificate;
    let output_before_rejection = owner
        .output_enclosure(&segment, &certificate, t_old, &mut work)
        .unwrap();
    let different = linear_segment(t_old, h);
    let before_mismatch = work.total_operations();
    assert!(matches!(
        owner.output_enclosure(&different, &certificate, t_old, &mut work),
        Err(Pr101Error::SegmentCertificateMismatch)
    ));
    assert!(work.total_operations() > before_mismatch);

    let before_rejection = work.total_operations();
    assert!(matches!(
        owner.verify_segment(&segment, 1.0e-15, 0.0, &mut work),
        Err(Pr101Error::TubeNotVerified { .. })
    ));
    assert!(work.total_operations() > before_rejection);
    assert_eq!(certificate, certificate_snapshot);
    let output_after_rejection = owner
        .output_enclosure(&segment, &certificate, t_old, &mut work)
        .unwrap();
    assert_eq!(output_after_rejection, output_before_rejection);
}

#[test]
fn pr101_full_tube_and_inherited_initial_radius_are_included() {
    let _lock = lock_tests();
    let owner = BoundPr101Problem::new().unwrap();
    let h = f64::from_bits(0x3eb0_0000_0000_0000);
    let segment = linear_segment(0.0, h);
    let mut work = DenseSegmentWork::default();
    let inherited = 1.0e-12;
    let certificate = owner
        .verify_segment(&segment, inherited, 2.0e-12, &mut work)
        .unwrap();
    assert!(certificate.endpoint_error_radius_upper() > 0.0);
    let at_end = owner
        .output_enclosure(&segment, &certificate, h, &mut work)
        .unwrap();
    assert!(at_end.radius() >= certificate.endpoint_error_radius_upper());
    assert!(at_end.radius() < inherited);

    let point_mu = owner
        .one_sided_log_norm_upper(
            Interval::point(0.0).unwrap(),
            &[Interval::point(0.0).unwrap()],
            &mut work,
        )
        .unwrap();
    let tube_mu = owner
        .one_sided_log_norm_upper(
            Interval::point(0.0).unwrap(),
            &[Interval::new(-1.0e-6, 1.0e-6).unwrap()],
            &mut work,
        )
        .unwrap();
    assert!(tube_mu > point_mu);
}

#[test]
fn pr101_refuses_invalid_radius_domain_nan_and_overflow_with_work() {
    let _lock = lock_tests();
    let owner = BoundPr101Problem::new().unwrap();
    let mut work = DenseSegmentWork::default();
    let segment = linear_segment(0.0, 2.0_f64.powi(-20));
    assert!(matches!(
        owner.verify_segment(&segment, f64::NAN, 1.0e-10, &mut work),
        Err(Pr101Error::InvalidRadius)
    ));
    assert!(work.validation_operations > 0);

    let outside = linear_segment(0.75, 0.5);
    assert!(matches!(
        owner.verify_segment(&outside, 0.0, 1.0, &mut work),
        Err(Pr101Error::OutsideDomain)
    ));

    let zero = Interval::point(0.0).unwrap();
    let mut rhs = [zero];
    let nan = Interval {
        lo: f64::NAN,
        hi: 0.0,
    };
    let nan_result = owner.enclose_rhs(Interval::point(0.0).unwrap(), &[nan], &mut rhs, &mut work);
    assert!(nan_result.is_err());
    assert!(work.provider_operations > 0);

    let wrong_shape = owner.enclose_rhs(Interval::point(0.0).unwrap(), &[], &mut rhs, &mut work);
    assert!(wrong_shape.is_err());
    assert!(work.validation_operations > 0);

    let huge = Interval::new(-1.0e308, 1.0e308).unwrap();
    let overflow = owner.enclose_rhs(
        Interval::new(0.0, 1.0).unwrap(),
        &[huge],
        &mut rhs,
        &mut work,
    );
    assert!(overflow.is_err());
    assert!(work.provider_operations > 0);

    let tiny = f64::from_bits(1);
    owner
        .enclose_rhs(
            Interval::point(0.0).unwrap(),
            &[Interval::point(tiny).unwrap()],
            &mut rhs,
            &mut work,
        )
        .unwrap();
    assert!(rhs[0].lo.is_finite() && rhs[0].hi.is_finite());
}

#[test]
fn pr101_provider_work_counts_reached_operations_and_preserves_output_on_overflow() {
    let _lock = lock_tests();
    let owner = BoundPr101Problem::new().unwrap();
    let time = Interval::point(0.0).unwrap();
    let mut rhs = [Interval::point(17.0).unwrap()];
    let mut work = DenseSegmentWork::default();
    work.charge_provider_operations(13);

    let before_success = work.provider_operations;
    owner
        .enclose_rhs(time, &[Interval::point(0.0).unwrap()], &mut rhs, &mut work)
        .unwrap();
    let success_delta = work.provider_operations - before_success;
    println!("A05_RHS_SUCCESS_PROVIDER_DELTA={success_delta}");
    assert_eq!(success_delta, 707);

    let previous_output = rhs[0];
    let before_failure = work.provider_operations;
    let overflow = owner.enclose_rhs(
        time,
        &[Interval::point(1.0e155).unwrap()],
        &mut rhs,
        &mut work,
    );
    assert!(overflow.is_err());
    let failure_delta = work.provider_operations - before_failure;
    println!("A05_RHS_QUADRATIC_OVERFLOW_PROVIDER_DELTA={failure_delta}");
    assert_eq!(failure_delta, 704);
    assert_eq!(rhs[0], previous_output);
    assert!(failure_delta > 0);
    assert_eq!(work.provider_operations, before_failure + failure_delta);
}

#[test]
fn generic_problem_with_matching_metadata_cannot_request_pr101_authority() {
    let _lock = lock_tests();
    let rhs = std::sync::Arc::new(|_t, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
        if y.len() != 1 || out.len() != 1 {
            return Err(CoreError::Dimension("test shape".into()));
        }
        out[0] = 0.0;
        Ok(())
    });
    let jvp = std::sync::Arc::new(
        |_t, _y: &[f64], _v: &[f64], out: &mut [f64]| -> CoreResult<()> {
            if out.len() != 1 {
                return Err(CoreError::Dimension("test shape".into()));
            }
            out[0] = 0.0;
            Ok(())
        },
    );
    let arbitrary = OdeProblem::new(
        "pr101-sealed",
        1,
        rhs,
        None,
        None,
        Some(jvp),
        None,
        false,
        None,
        None,
    )
    .unwrap();
    assert!(matches!(
        arbitrary.request_continuous_residual_authority(),
        Err(DenseOutputError::UnsupportedUnboundModel)
    ));
}

#[test]
fn pr101_warmed_provider_operations_make_no_heap_allocations() {
    let _lock = lock_tests();
    let calibration_allocations = calibrate_allocator();
    println!("ALLOCATION_CALIBRATION clone_allocations={calibration_allocations}");
    let owner = BoundPr101Problem::new().unwrap();
    let time = Interval::new(0.0, 0.5).unwrap();
    let state = [Interval::new(-0.1, 0.1).unwrap()];
    let mut rhs = [Interval::point(0.0).unwrap()];
    let mut work = DenseSegmentWork::default();
    owner
        .enclose_rhs(time, &state, &mut rhs, &mut work)
        .unwrap();
    owner
        .one_sided_log_norm_upper(time, &state, &mut work)
        .unwrap();

    let before = ALLOCATIONS.load(Ordering::SeqCst);
    for _ in 0..32 {
        owner
            .enclose_rhs(time, &state, &mut rhs, &mut work)
            .unwrap();
        std::hint::black_box(rhs);
        std::hint::black_box(
            owner
                .one_sided_log_norm_upper(time, &state, &mut work)
                .unwrap(),
        );
    }
    assert_eq!(ALLOCATIONS.load(Ordering::SeqCst) - before, 0);
}

#[test]
fn pr101_warmed_segment_verification_and_output_make_no_allocations() {
    let _lock = lock_tests();
    let calibration_allocations = calibrate_allocator();
    println!("ALLOCATION_CALIBRATION clone_allocations={calibration_allocations}");
    let owner = BoundPr101Problem::new().unwrap();
    let (segment, _t_old, h) = near_one_quartic_segment();
    let mut work = DenseSegmentWork::default();
    let certificate = owner
        .verify_segment(&segment, 1.0e-15, 1.0e-3, &mut work)
        .unwrap();
    owner
        .output_enclosure(&segment, &certificate, h + 1023.0 / 1024.0, &mut work)
        .unwrap();

    let before = ALLOCATIONS.load(Ordering::SeqCst);
    for _ in 0..16 {
        let certificate = owner
            .verify_segment(&segment, 1.0e-15, 1.0e-3, &mut work)
            .unwrap();
        std::hint::black_box(certificate);
        let output = owner
            .output_enclosure(&segment, &certificate, h + 1023.0 / 1024.0, &mut work)
            .unwrap();
        std::hint::black_box(output);
    }
    assert_eq!(ALLOCATIONS.load(Ordering::SeqCst) - before, 0);
}
