use rodas5p_core::{WorkCounters, sparse_direct::SparseDirectSolver};

#[test]
fn malformed_csc_is_rejected_without_panicking() {
    for (n, pointers, rows) in [
        (2, vec![0, 1, 2], vec![0, 2]),
        (2, vec![0, 3, 2], vec![0, 1]),
        (2, vec![0, 2, 2], vec![1, 0]),
        (2, vec![0, 2, 2], vec![0, 0]),
        (0, vec![0], vec![]),
        (usize::MAX, vec![], vec![]),
    ] {
        assert!(SparseDirectSolver::try_new_symbolic(n, &pointers, &rows).is_err());
    }
}

#[test]
fn numeric_state_is_invalidated_until_exact_pattern_refactor_succeeds() {
    let mut s = SparseDirectSolver::try_new_symbolic(2, &[0, 1, 2], &[0, 1]).unwrap();
    let mut c = WorkCounters::default();
    assert!(s.solve_in_place(&mut [1., 1.], &mut c).is_err());
    s.refactorize(&[0, 1, 2], &[0, 1], &[2., 4.], &mut c)
        .unwrap();
    let mut x = [2., 8.];
    s.solve_in_place(&mut x, &mut c).unwrap();
    assert_eq!(x, [1., 2.]);
    assert!(
        s.refactorize(&[0, 1, 2], &[1, 0], &[2., 4.], &mut c)
            .is_err()
    );
    assert!(s.solve_in_place(&mut [1., 1.], &mut c).is_err());
    s.refactorize(&[0, 1, 2], &[0, 1], &[2., 4.], &mut c)
        .unwrap();
    assert!(
        s.refactorize(&[0, 1, 2], &[0, 1], &[f64::NAN, 4.], &mut c)
            .is_err()
    );
    assert!(s.solve_in_place(&mut [1., 1.], &mut c).is_err());
    assert_eq!(c.direct_solve_calls, 1);
}

#[test]
fn nonfinite_rhs_is_rejected() {
    let mut s = SparseDirectSolver::try_new_symbolic(1, &[0, 1], &[0]).unwrap();
    let mut c = WorkCounters::default();
    s.refactorize(&[0, 1], &[0], &[2.], &mut c).unwrap();
    assert!(s.solve_in_place(&mut [f64::NAN], &mut c).is_err());
    assert_eq!(c.direct_solve_calls, 0);
}
