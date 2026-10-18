use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use rodas5p_core::directed::Interval;
use rodas5p_integrators::collective_model::CollectiveModel;
use rodas5p_integrators::{
    ContinuousResidualProvider, DenseOutputError, DenseSegmentWork, OdeProblem,
};

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn lock_tests() -> std::sync::MutexGuard<'static, ()> {
    TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn model(xi: Vec<f64>, weights: Vec<f64>, width: f64, chi: f64, kappa: f64) -> CollectiveModel {
    let initial = vec![(0.0, 0.0); xi.len()];
    CollectiveModel::new(xi, weights, initial, width, chi, 0.0, kappa).unwrap()
}

#[derive(Clone, Copy, Debug)]
struct Dyadic {
    numerator: i128,
    exponent: i32,
}

impl Dyadic {
    fn new(numerator: i128, exponent: i32) -> Self {
        if numerator == 0 {
            return Self {
                numerator: 0,
                exponent: 0,
            };
        }
        let trailing_zeros = numerator.unsigned_abs().trailing_zeros();
        Self {
            numerator: numerator >> trailing_zeros,
            exponent: exponent + trailing_zeros as i32,
        }
    }

    fn from_f64(value: f64) -> Self {
        let bits = value.to_bits();
        let sign = if bits >> 63 == 0 { 1_i128 } else { -1_i128 };
        let exponent_bits = ((bits >> 52) & 0x7ff) as i32;
        let fraction = (bits & ((1_u64 << 52) - 1)) as i128;
        if exponent_bits == 0 {
            Self::new(sign * fraction, -1074)
        } else {
            Self::new(
                sign * ((1_i128 << 52) + fraction),
                exponent_bits - 1023 - 52,
            )
        }
    }

    fn add(self, rhs: Self) -> Self {
        let exponent = self.exponent.min(rhs.exponent);
        let left_shift = u32::try_from(self.exponent - exponent).unwrap();
        let right_shift = u32::try_from(rhs.exponent - exponent).unwrap();
        Self::new(
            (self.numerator << left_shift) + (rhs.numerator << right_shift),
            exponent,
        )
    }

    fn neg(self) -> Self {
        Self::new(-self.numerator, self.exponent)
    }

    fn sub(self, rhs: Self) -> Self {
        self.add(rhs.neg())
    }

    fn mul(self, rhs: Self) -> Self {
        Self::new(self.numerator * rhs.numerator, self.exponent + rhs.exponent)
    }

    fn cmp(self, rhs: Self) -> std::cmp::Ordering {
        let exponent = self.exponent.min(rhs.exponent);
        let left = self.numerator << u32::try_from(self.exponent - exponent).unwrap();
        let right = rhs.numerator << u32::try_from(rhs.exponent - exponent).unwrap();
        left.cmp(&right)
    }
}

#[test]
fn generic_same_name_and_dimension_problem_is_unbound() {
    let _lock = lock_tests();
    let owner = model(vec![0.0], vec![1.0], 0.0, 0.0, 0.0)
        .to_bound_continuous_problem()
        .unwrap();
    let generic_rhs = std::sync::Arc::new(|_t, y: &[f64], out: &mut [f64]| {
        out.copy_from_slice(y);
        Ok(())
    });
    let generic_jvp = std::sync::Arc::new(|_t, _y: &[f64], v: &[f64], out: &mut [f64]| {
        out.copy_from_slice(v);
        Ok(())
    });
    let generic = OdeProblem::new(
        "collective-cubic-1",
        2,
        generic_rhs,
        None,
        None,
        Some(generic_jvp),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    assert!(matches!(
        generic.request_continuous_residual_authority(),
        Err(DenseOutputError::UnsupportedUnboundModel)
    ));
    assert_eq!(owner.dimension(), 2);
}

#[test]
fn fraction_dyadic_nonuniform_weights_rhs_is_outward_enclosed() {
    let _lock = lock_tests();
    let model = model(vec![-0.5, 0.25], vec![0.25, 0.75], 0.5, 0.75, 0.125);
    let owner = model.to_bound_continuous_problem().unwrap();
    let state_f64 = [0.5, -0.25, -0.125, 0.375];
    let state = state_f64.map(|value| Interval::point(value).unwrap());
    let mut rhs = [Interval::point(0.0).unwrap(); 4];
    let mut work = DenseSegmentWork::default();
    owner
        .enclose_rhs(
            Interval::new(-1.0, 2.0).unwrap(),
            &state,
            &mut rhs,
            &mut work,
        )
        .unwrap();

    // Exact Fraction arithmetic over the dyadic binary64 inputs.
    let weights = [Dyadic::from_f64(0.25), Dyadic::from_f64(0.75)];
    let ur = [
        Dyadic::from_f64(state_f64[0]),
        Dyadic::from_f64(state_f64[2]),
    ];
    let vi = [
        Dyadic::from_f64(state_f64[1]),
        Dyadic::from_f64(state_f64[3]),
    ];
    let mut mean_re = Dyadic::new(0, 0);
    let mut mean_im = Dyadic::new(0, 0);
    for j in 0..2 {
        mean_re = mean_re.add(weights[j].mul(ur[j]));
        mean_im = mean_im.add(weights[j].mul(vi[j]));
    }
    let mean_sq = mean_re.mul(mean_re).add(mean_im.mul(mean_im));
    let chi = Dyadic::from_f64(0.75);
    let kappa = Dyadic::from_f64(0.125);
    let width = Dyadic::from_f64(0.5);
    let xi = [Dyadic::from_f64(-0.5), Dyadic::from_f64(0.25)];
    let mut exact_rhs = [Dyadic::new(0, 0); 4];
    for j in 0..2 {
        let omega = width.mul(xi[j]);
        exact_rhs[2 * j] = kappa
            .neg()
            .mul(ur[j])
            .sub(omega.mul(vi[j]))
            .sub(chi.mul(mean_sq).mul(mean_im));
        exact_rhs[2 * j + 1] = kappa
            .neg()
            .mul(vi[j])
            .add(omega.mul(ur[j]))
            .add(chi.mul(mean_sq).mul(mean_re));
    }
    for (enclosure, exact) in rhs.into_iter().zip(exact_rhs) {
        assert_ne!(
            Dyadic::from_f64(enclosure.lo).cmp(exact),
            std::cmp::Ordering::Greater
        );
        assert_ne!(
            Dyadic::from_f64(enclosure.hi).cmp(exact),
            std::cmp::Ordering::Less
        );
    }
    assert!(work.provider_operations > 0);
}

#[test]
fn euclidean_log_norm_bounds_two_mode_jacobian_and_uses_full_tube() {
    let _lock = lock_tests();
    let collective = model(vec![-0.5, 0.25], vec![0.25, 0.75], 0.7, 0.6, 0.2);
    let owner = collective.to_bound_continuous_problem().unwrap();
    let y = [0.25, -0.125, -0.125, 0.25];
    let point_tube = y.map(|value| Interval::point(value).unwrap());
    let mut work = DenseSegmentWork::default();
    let mu = owner
        .one_sided_log_norm_upper(Interval::point(0.0).unwrap(), &point_tube, &mut work)
        .unwrap();
    assert_eq!(work.provider_operations, 33);

    let mean_re = 0.25 * y[0] + 0.75 * y[2];
    let mean_im = 0.25 * y[1] + 0.75 * y[3];
    let u2 = mean_re * mean_re;
    let v2 = mean_im * mean_im;
    let uv = mean_re * mean_im;
    let chi = 0.6;
    let kappa = 0.2;
    let weights = [0.25, 0.75];
    let xi = [-0.5, 0.25];
    let width = 0.7;
    let mut jacobian = [[0.0; 4]; 4];
    for j in 0..2 {
        let omega = width * xi[j];
        jacobian[2 * j][2 * j] -= kappa;
        jacobian[2 * j][2 * j + 1] -= omega;
        jacobian[2 * j + 1][2 * j] += omega;
        jacobian[2 * j + 1][2 * j + 1] -= kappa;
        for k in 0..2 {
            let w = weights[k];
            jacobian[2 * j][2 * k] += -2.0 * chi * uv * w;
            jacobian[2 * j][2 * k + 1] += -chi * (u2 + 3.0 * v2) * w;
            jacobian[2 * j + 1][2 * k] += chi * (3.0 * u2 + v2) * w;
            jacobian[2 * j + 1][2 * k + 1] += 2.0 * chi * uv * w;
        }
    }
    let mut symmetric = [[0.0; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            symmetric[i][j] = 0.5 * (jacobian[i][j] + jacobian[j][i]);
        }
    }
    assert!(largest_symmetric_eigenvalue(symmetric) <= mu + 1.0e-12);

    let curve_only = [Interval::point(0.0).unwrap(); 4];
    let full_tube = [Interval::new(-1.0, 1.0).unwrap(); 4];
    let mu_curve = owner
        .one_sided_log_norm_upper(Interval::point(0.0).unwrap(), &curve_only, &mut work)
        .unwrap();
    let mu_full = owner
        .one_sided_log_norm_upper(Interval::point(0.0).unwrap(), &full_tube, &mut work)
        .unwrap();
    assert!(mu_full > mu_curve);

    let undamped = model(vec![0.0], vec![1.0], 0.0, 0.0, 0.375)
        .to_bound_continuous_problem()
        .unwrap();
    assert_eq!(
        undamped
            .one_sided_log_norm_upper(
                Interval::point(0.0).unwrap(),
                &[Interval::new(-1.0, 1.0).unwrap(); 2],
                &mut work,
            )
            .unwrap(),
        -0.375
    );
}

fn largest_symmetric_eigenvalue(mut matrix: [[f64; 4]; 4]) -> f64 {
    for _ in 0..64 {
        let mut p = 0;
        let mut q = 1;
        let mut largest = 0.0_f64;
        for i in 0..4 {
            for j in i + 1..4 {
                if matrix[i][j].abs() > largest {
                    largest = matrix[i][j].abs();
                    p = i;
                    q = j;
                }
            }
        }
        if largest < 1.0e-15 {
            break;
        }
        let angle = 0.5 * (2.0 * matrix[p][q]).atan2(matrix[q][q] - matrix[p][p]);
        let c = angle.cos();
        let s = angle.sin();
        let app = c * c * matrix[p][p] - 2.0 * s * c * matrix[p][q] + s * s * matrix[q][q];
        let aqq = s * s * matrix[p][p] + 2.0 * s * c * matrix[p][q] + c * c * matrix[q][q];
        for k in 0..4 {
            if k != p && k != q {
                let mkp = c * matrix[k][p] - s * matrix[k][q];
                let mkq = s * matrix[k][p] + c * matrix[k][q];
                matrix[k][p] = mkp;
                matrix[p][k] = mkp;
                matrix[k][q] = mkq;
                matrix[q][k] = mkq;
            }
        }
        matrix[p][p] = app;
        matrix[q][q] = aqq;
        matrix[p][q] = 0.0;
        matrix[q][p] = 0.0;
    }
    (0..4)
        .map(|i| matrix[i][i])
        .fold(f64::NEG_INFINITY, f64::max)
}

#[test]
fn provider_fails_closed_on_bad_shape_nan_and_overflow_and_charges_attempts() {
    let _lock = lock_tests();
    let owner = model(vec![0.0], vec![1.0], 0.0, 1.0, 0.0)
        .to_bound_continuous_problem()
        .unwrap();
    let mut work = DenseSegmentWork::default();
    let zero = Interval::point(0.0).unwrap();
    let mut rhs = [zero; 2];
    let shape = owner.enclose_rhs(Interval::point(0.0).unwrap(), &[zero], &mut rhs, &mut work);
    assert!(matches!(shape, Err(DenseOutputError::Core(_))));
    assert!(work.validation_operations > 0);

    let nan_box = [
        Interval {
            lo: f64::NAN,
            hi: 0.0,
        },
        zero,
    ];
    let nan = owner.enclose_rhs(Interval::point(0.0).unwrap(), &nan_box, &mut rhs, &mut work);
    assert!(matches!(nan, Err(DenseOutputError::Core(_))));
    assert!(work.provider_operations > 0);

    let huge_model = model(vec![0.0], vec![1.0], 0.0, 1.0e308, 0.0)
        .to_bound_continuous_problem()
        .unwrap();
    let huge = [Interval::new(-1.0e308, 1.0e308).unwrap(); 2];
    let mut large_work = DenseSegmentWork::default();
    let overflow =
        huge_model.one_sided_log_norm_upper(Interval::point(0.0).unwrap(), &huge, &mut large_work);
    assert!(matches!(overflow, Err(DenseOutputError::Core(_))));
    assert!(large_work.provider_operations > 0);

    let tiny = f64::from_bits(1);
    let mut tiny_rhs = [zero; 2];
    owner
        .enclose_rhs(
            Interval::point(0.0).unwrap(),
            &[
                Interval::point(tiny).unwrap(),
                Interval::point(-tiny).unwrap(),
            ],
            &mut tiny_rhs,
            &mut work,
        )
        .unwrap();
    assert!(
        tiny_rhs
            .iter()
            .all(|entry| entry.lo.is_finite() && entry.hi.is_finite())
    );
}

#[test]
fn warmed_interval_rhs_and_log_norm_make_no_heap_allocations() {
    let _lock = lock_tests();
    let owner = model(vec![-0.5, 0.25], vec![0.25, 0.75], 0.5, 0.75, 0.125)
        .to_bound_continuous_problem()
        .unwrap();
    let state = [
        Interval::new(-0.5, 0.5).unwrap(),
        Interval::new(-0.25, 0.25).unwrap(),
        Interval::new(-0.5, 0.5).unwrap(),
        Interval::new(-0.25, 0.25).unwrap(),
    ];
    let mut rhs = [Interval::point(0.0).unwrap(); 4];
    let time = Interval::new(0.0, 1.0).unwrap();
    let mut work = DenseSegmentWork::default();
    owner
        .enclose_rhs(time, &state, &mut rhs, &mut work)
        .unwrap();
    owner
        .one_sided_log_norm_upper(time, &state, &mut work)
        .unwrap();

    let before_rhs = ALLOCATIONS.load(Ordering::SeqCst);
    for _ in 0..64 {
        owner
            .enclose_rhs(time, &state, &mut rhs, &mut work)
            .unwrap();
        std::hint::black_box(rhs);
    }
    let rhs_allocations = ALLOCATIONS.load(Ordering::SeqCst) - before_rhs;
    let before_norm = ALLOCATIONS.load(Ordering::SeqCst);
    for _ in 0..64 {
        std::hint::black_box(
            owner
                .one_sided_log_norm_upper(time, &state, &mut work)
                .unwrap(),
        );
    }
    let norm_allocations = ALLOCATIONS.load(Ordering::SeqCst) - before_norm;
    assert_eq!((rhs_allocations, norm_allocations), (0, 0));
}
