use rodas5p_core::bordered::BorderedBandedPreconditioner;
use rodas5p_core::low_rank_bordered::{LowRankApplyWorkspace, LowRankBorderedPreconditioner};
use rodas5p_core::{DenseMatrix, Preconditioner, PreconditionerSide, direct_solve};

#[test]
fn original_dimension_matches_independent_dense_solve_for_multiple_shapes() {
    let mut pc = LowRankBorderedPreconditioner::new(12, 3, 4, 1, 1).unwrap();
    let mut ws = LowRankApplyWorkspace::new(12, 3, 4).unwrap();
    for (n, m, r) in [(12, 3, 4), (3, 0, 2), (1, 1, 1), (8, 2, 0)] {
        let mut a = vec![0.0; 3 * n];
        let b: Vec<_> = (0..n * m).map(|i| (i as f64 - 4.0) / 97.0).collect();
        let c: Vec<_> = (0..m * n).map(|i| (3.0 - i as f64) / 113.0).collect();
        let mut d = vec![0.0; m * m];
        let u: Vec<_> = (0..n * r).map(|i| ((i * 7) % 11) as f64 / 29.0).collect();
        let v: Vec<_> = (0..n * r)
            .map(|i| (((i * 3) % 13) as f64 - 6.0) / 37.0)
            .collect();
        let mut matrix = DenseMatrix::zeros(n + m, n + m);
        for i in 0..n {
            a[3 * i + 1] = 4.0;
            matrix[(i, i)] = 4.0;
            if i > 0 {
                a[3 * i] = -0.5;
                matrix[(i, i - 1)] = -0.5;
            }
            if i + 1 < n {
                a[3 * i + 2] = 0.25;
                matrix[(i, i + 1)] = 0.25;
            }
            for j in 0..n {
                for k in 0..r {
                    matrix[(i, j)] += u[i * r + k] * v[j * r + k];
                }
            }
            for j in 0..m {
                matrix[(i, n + j)] = b[i * m + j];
                matrix[(n + j, i)] = c[j * n + i];
            }
        }
        for i in 0..m {
            for j in 0..m {
                d[i * m + j] = if i == j { 3.0 } else { 0.125 };
                matrix[(n + i, n + j)] = d[i * m + j];
            }
        }
        pc.refactor(n, m, r, &a, &b, &c, &d, &u, &v).unwrap();
        assert_eq!(pc.dimension(), n + m);
        assert_eq!(pc.side(), PreconditionerSide::Right);
        assert!(pc.exact_identity().is_none());
        let rhs: Vec<_> = (0..n + m).map(|i| (i as f64 - 3.0) / 8.0).collect();
        let expected = direct_solve(&matrix, &rhs).unwrap();
        let mut actual = vec![0.0; n + m];
        pc.apply_with_workspace(&rhs, &mut actual, &mut ws).unwrap();
        assert!(
            actual
                .iter()
                .zip(&expected)
                .all(|(x, y)| (x - y).abs() < 2e-13)
        );
        let reconstructed = matrix.matvec(&actual).unwrap();
        assert!(
            reconstructed
                .iter()
                .zip(&rhs)
                .all(|(x, y)| (x - y).abs() < 2e-13)
        );
        let mut trait_out = vec![0.0; n + m];
        pc.apply(&rhs, &mut trait_out).unwrap();
        assert_eq!(trait_out, actual);
    }
}

#[test]
fn rank_zero_is_the_existing_bordered_inverse() {
    let mut new = LowRankBorderedPreconditioner::new(2, 1, 0, 0, 0).unwrap();
    let mut old = BorderedBandedPreconditioner::new(2, 1, 0, 0).unwrap();
    let (a, b, c, d) = ([2.0, 3.0], [1.0, 0.5], [-0.25, 0.75], [4.0]);
    new.refactor(2, 1, 0, &a, &b, &c, &d, &[], &[]).unwrap();
    old.refactor(2, 1, &a, &b, &c, &d).unwrap();
    let mut x = [0.0; 3];
    let mut y = [0.0; 3];
    new.apply(&[1.0, 2.0, 3.0], &mut x).unwrap();
    old.apply(&[1.0, 2.0, 3.0], &mut y).unwrap();
    assert_eq!(x, y);
}

#[test]
fn bad_refactors_invalidate_previously_good_factors() {
    let mut pc = LowRankBorderedPreconditioner::new(2, 1, 1, 0, 0).unwrap();
    for failure in 0..4 {
        pc.refactor(1, 0, 1, &[1.0], &[], &[], &[], &[1.0], &[1.0])
            .unwrap();
        let result = match failure {
            0 => pc.refactor(1, 0, 1, &[1.0], &[], &[], &[], &[1.0], &[-1.0]), // singular M=0
            1 => pc.refactor(1, 0, 1, &[1.0], &[], &[], &[], &[f64::NAN], &[1.0]),
            2 => pc.refactor(3, 0, 0, &[1.0; 3], &[], &[], &[], &[], &[]),
            _ => pc.refactor(1, 0, 1, &[1.0], &[], &[], &[], &[], &[1.0]),
        };
        assert!(result.is_err());
        assert!(!pc.is_valid());
        assert_eq!(pc.dimension(), 0);
        let mut output = [19.0];
        assert!(pc.apply(&[1.0], &mut output).is_err());
        assert_eq!(output, [19.0]);
    }
}

#[test]
fn application_failures_are_atomic_and_do_not_invalidate_factors() {
    let mut pc = LowRankBorderedPreconditioner::new(1, 0, 1, 0, 0).unwrap();
    pc.refactor(1, 0, 1, &[1.0], &[], &[], &[], &[1.0], &[1.0])
        .unwrap();
    let mut output = [19.0];
    let mut undersized = LowRankApplyWorkspace::new(1, 0, 0).unwrap();
    assert!(
        pc.apply_with_workspace(&[1.0], &mut output, &mut undersized)
            .is_err()
    );
    assert_eq!(output, [19.0]);
    assert!(pc.apply(&[f64::INFINITY], &mut output).is_err());
    assert_eq!(output, [19.0]);
    assert!(pc.apply(&[], &mut output).is_err());
    assert_eq!(output, [19.0]);
    pc.refactor(1, 0, 0, &[f64::MIN_POSITIVE], &[], &[], &[], &[], &[])
        .unwrap();
    assert!(pc.apply(&[f64::MAX], &mut output).is_err());
    assert_eq!(output, [19.0]);
    assert!(pc.is_valid());
    pc.apply(&[f64::MIN_POSITIVE], &mut output).unwrap();
    assert_eq!(output, [1.0]);
}

#[test]
fn impossible_capacities_fail_without_arithmetic_panic() {
    assert!(LowRankBorderedPreconditioner::new(1, usize::MAX, 1, 0, 0).is_err());
    assert!(LowRankBorderedPreconditioner::new(usize::MAX, 0, 0, 0, 0).is_err());
    assert!(LowRankApplyWorkspace::new(usize::MAX, 1, 0).is_err());
}

#[test]
fn separate_workspaces_support_shared_parallel_factors() {
    let mut pc = LowRankBorderedPreconditioner::new(2, 0, 1, 0, 0).unwrap();
    pc.refactor(
        2,
        0,
        1,
        &[2.0, 2.0],
        &[],
        &[],
        &[],
        &[1.0, 1.0],
        &[1.0, 1.0],
    )
    .unwrap();
    std::thread::scope(|scope| {
        for scale in [1.0, 2.0, 3.0, 4.0] {
            let factors = &pc;
            scope.spawn(move || {
                let mut ws = LowRankApplyWorkspace::new(2, 0, 1).unwrap();
                let mut out = [0.0; 2];
                factors
                    .apply_with_workspace(&[4.0 * scale; 2], &mut out, &mut ws)
                    .unwrap();
                assert!(out.iter().all(|x| (*x - scale).abs() < 1e-14));
            });
        }
    });
}
