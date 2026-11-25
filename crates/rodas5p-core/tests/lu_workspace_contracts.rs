//! Caller-buffer LU solve: pivoted solve accuracy, finite-input rejection and
//! allocation count. Run this binary with --test-threads=1 for the global count.
use rodas5p_core::{DenseMatrix, LuFactorization, LuSolveWorkspace};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct CountAlloc;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for CountAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(p, layout, size) }
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        unsafe { System.dealloc(p, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: CountAlloc = CountAlloc;

#[test]
fn pivoted_lu_workspace_contracts() {
    faer::set_global_parallelism(faer::Par::Seq);
    for n in [1, 2, 7, 32, 65, 96, 192] {
        let mut a = DenseMatrix::zeros(n, n);
        for i in 0..n {
            for j in 0..n {
                a[(i, j)] = 0.01 * (((i * 7 + j * 11) % 17) as f64 - 8.0) / n as f64;
            }
            a[(i, (i + 1) % n)] += 2.0 + i as f64 / n as f64;
        }
        let x: Vec<_> = (0..n).map(|i| (i as f64 + 0.3).sin()).collect();
        let mut b = vec![0.0; n];
        a.matvec_into(&x, &mut b).unwrap();
        let lu = LuFactorization::new(&a).unwrap();
        let mut scratch = LuSolveWorkspace::new(n);
        let reference = lu.solve(&b).unwrap();
        let mut result = b.clone();
        lu.solve_in_place_with_workspace(&mut result, &mut scratch)
            .unwrap();
        for i in 0..n {
            assert!((result[i] - x[i]).abs() < 2e-13, "n={n}, i={i}");
            assert_eq!(result[i].to_bits(), reference[i].to_bits(), "n={n}, i={i}");
        }
        result.copy_from_slice(&b);
        let before = ALLOCS.load(Ordering::Relaxed);
        lu.solve_in_place_with_workspace(&mut result, &mut scratch)
            .unwrap();
        let used = ALLOCS.load(Ordering::Relaxed) - before;
        assert_eq!(used, 0, "n={n}");
        let mut allocated = b.clone();
        lu.solve_in_place(&mut allocated).unwrap();
        assert_eq!(result, allocated);
        // Bad workspace and RHS must fail before overwriting input.
        let mut wrong = LuSolveWorkspace::new(n + 1);
        result.copy_from_slice(&b);
        assert!(
            lu.solve_in_place_with_workspace(&mut result, &mut wrong)
                .is_err()
        );
        assert_eq!(result, b);
        assert!(
            lu.solve_in_place_with_workspace(&mut result[..n - 1], &mut scratch)
                .is_err()
        );
        result[0] = f64::NAN;
        assert!(
            lu.solve_in_place_with_workspace(&mut result, &mut scratch)
                .is_err()
        );
        assert!(result[0].is_nan());
    }
    // Scratch owns no matrix identity: the same buffer is safe with new factors.
    let a = DenseMatrix::from_rows(&[&[2.0, 1.0], &[1.0, 3.0]]).unwrap();
    let c = DenseMatrix::from_rows(&[&[4.0, 1.0], &[2.0, 2.0]]).unwrap();
    let mut work = LuSolveWorkspace::new(2);
    for m in [&a, &c, &a] {
        let lu = LuFactorization::new(m).unwrap();
        let mut b = vec![m[(0, 0)] + 2.0 * m[(0, 1)], m[(1, 0)] + 2.0 * m[(1, 1)]];
        lu.solve_in_place_with_workspace(&mut b, &mut work).unwrap();
        assert!((b[0] - 1.0).abs() < 1e-14 && (b[1] - 2.0).abs() < 1e-14);
    }
}

#[test]
fn preflight_batch_refuses_nonfinite_rhs_at_input_boundary() {
    let lu = LuFactorization::new(&DenseMatrix::identity(2)).unwrap();
    let err = lu
        .solve_rows(&[vec![1., 2.], vec![f64::NAN, 0.]])
        .unwrap_err();
    assert!(
        matches!(err, rodas5p_core::CoreError::NonFinite(_)),
        "{err:?}"
    );
}

#[cfg(feature = "retro-dictionary")]
#[test]
fn preflight_reusable_multi_rhs_has_zero_warm_allocations() {
    use rodas5p_core::LuBatchWorkspace;
    faer::set_global_parallelism(faer::Par::Seq);
    for n in [2, 17, 65, 128] {
        let mut a = DenseMatrix::identity(n);
        for i in 0..n - 1 {
            a[(i, i + 1)] = 0.125;
            a[(i + 1, i)] = -0.25;
        }
        let lu = LuFactorization::new(&a).unwrap();
        for k in [1, 3, 8, 32] {
            let exact = (0..k)
                .map(|j| {
                    (0..n)
                        .map(|i| ((i + 3 * j) % 19) as f64 / 16.)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let b = exact
                .iter()
                .map(|x| a.matvec(x).unwrap())
                .collect::<Vec<_>>();
            let mut w = LuBatchWorkspace::new(n, k).unwrap();
            let mut x = b.clone();
            lu.solve_rows_in_place_with_workspace(&mut x, &mut w)
                .unwrap();
            let prior = lu.solve_rows(&b).unwrap();
            for (x, y) in x.iter().flatten().zip(prior.iter().flatten()) {
                assert!((x - y).abs() < 1e-13);
            }
            for (x, b) in x.iter_mut().zip(&b) {
                x.copy_from_slice(b);
            }
            let before = ALLOCS.load(Ordering::Relaxed);
            lu.solve_rows_in_place_with_workspace(&mut x, &mut w)
                .unwrap();
            let used = ALLOCS.load(Ordering::Relaxed) - before;
            assert_eq!(used, 0, "n={n} k={k}");
            assert!(
                x.iter()
                    .flatten()
                    .zip(exact.iter().flatten())
                    .all(|(a, b)| (a - b).abs() < 1e-13)
            );
            x[0][0] = f64::NAN;
            let snapshot = x.iter().flatten().map(|v| v.to_bits()).collect::<Vec<_>>();
            assert!(
                lu.solve_rows_in_place_with_workspace(&mut x, &mut w)
                    .is_err()
            );
            assert_eq!(
                snapshot,
                x.iter().flatten().map(|v| v.to_bits()).collect::<Vec<_>>()
            );
            println!("WARM_ALLOC n={n} rhs={k} allocations={used}");
        }
    }
    assert!(LuBatchWorkspace::new(0, 1).is_err());
    assert!(LuBatchWorkspace::new(usize::MAX, 2).is_err());
}
