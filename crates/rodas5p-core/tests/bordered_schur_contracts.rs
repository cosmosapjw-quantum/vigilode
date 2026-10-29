use rodas5p_core::bordered::BorderedBandedPreconditioner;
use rodas5p_core::{DenseMatrix, Preconditioner, PreconditionerSide, direct_solve};

fn fixture(
    n: usize,
    m: usize,
    lower: usize,
    upper: usize,
) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, DenseMatrix) {
    let stride = lower + upper + 1;
    let mut a = vec![0.0; n * stride];
    let mut b = vec![0.0; n * m];
    let mut c = vec![0.0; m * n];
    let mut d = vec![0.0; m * m];
    let mut full = DenseMatrix::zeros(n + m, n + m);
    for i in 0..n {
        for j in i.saturating_sub(lower)..n.min(i + upper + 1) {
            let v = if i == j {
                4.0 + (lower + upper) as f64
            } else {
                ((i + 2 * j) % 5) as f64 * 0.125 - 0.25
            };
            a[i * stride + j + lower - i] = v;
            full[(i, j)] = v;
        }
    }
    for i in 0..n {
        for j in 0..m {
            b[i * m + j] = ((i + j) % 3) as f64 * 0.125;
            c[j * n + i] = -(((2 * i + j) % 5) as f64) * 0.0625 / n as f64;
            full[(i, n + j)] = b[i * m + j];
            full[(n + j, i)] = c[j * n + i];
        }
    }
    for i in 0..m {
        for j in 0..m {
            d[i * m + j] = if i == j { 3.0 } else { 0.125 };
            full[(n + i, n + j)] = d[i * m + j];
        }
    }
    (a, b, c, d, full)
}

#[test]
fn bordered_solves_match_independent_dense_oracle_at_changing_dimensions() {
    for (lower, upper) in [(0, 0), (1, 1), (2, 1), (1, 3)] {
        let mut pc = BorderedBandedPreconditioner::new(33, 4, lower, upper).unwrap();
        let slots = pc.allocated_f64_slots();
        for (n, m) in [(33, 4), (5, 2), (1, 0), (17, 3), (33, 4)] {
            let (a, b, c, d, full) = fixture(n, m, lower, upper);
            pc.refactor(n, m, &a, &b, &c, &d).unwrap();
            assert_eq!(pc.dimension(), n + m);
            assert_eq!(pc.side(), PreconditionerSide::Right);
            let rhs: Vec<f64> = (0..n + m).map(|i| ((i * 7) % 13) as f64 - 4.0).collect();
            let mut out = vec![0.0; n + m];
            pc.apply(&rhs, &mut out).unwrap();
            let dense = direct_solve(&full, &rhs).unwrap();
            let residual = full.matvec(&out).unwrap();
            for i in 0..n + m {
                assert!((out[i] - dense[i]).abs() < 2e-13);
                assert!((residual[i] - rhs[i]).abs() < 2e-13);
            }
            assert_eq!(slots, pc.allocated_f64_slots());
        }
    }
}

#[test]
fn schur_pivoting_and_failure_invalidation() {
    let mut pc = BorderedBandedPreconditioner::new(2, 2, 0, 0).unwrap();
    pc.refactor(
        1,
        2,
        &[2.0],
        &[0.0, 0.0],
        &[0.0, 0.0],
        &[0.0, 1.0, 1.0, 1.0],
    )
    .unwrap();
    let mut y = [9.0; 3];
    pc.apply(&[4.0, 3.0, 5.0], &mut y).unwrap();
    assert_eq!(y, [2.0, 2.0, 3.0]);
    // Full M is singular although its phase block is well conditioned.
    assert!(pc.refactor(1, 1, &[2.0], &[2.0], &[3.0], &[3.0]).is_err());
    assert!(!pc.is_valid());
    assert_eq!(pc.active_shape(), None);
    let mut sentinel = [7.0; 2];
    assert!(pc.apply(&[1.0; 2], &mut sentinel).is_err());
    assert_eq!(sentinel, [7.0; 2]);
    pc.refactor(1, 0, &[2.0], &[], &[], &[]).unwrap();
    assert!(pc.is_valid());
}

#[test]
fn shape_nonfinite_dominance_and_arithmetic_failures_are_explicit() {
    assert!(BorderedBandedPreconditioner::new(0, 1, 0, 0).is_err());
    assert!(BorderedBandedPreconditioner::new(usize::MAX, 2, 0, 0).is_err());
    assert!(BorderedBandedPreconditioner::new(usize::MAX / 8, 0, 0, 0).is_err());
    let mut pc = BorderedBandedPreconditioner::new(3, 1, 1, 1).unwrap();
    assert!(
        pc.refactor(2, 0, &[0.0, 1.0, 1.0, 1.0, 1.0, 0.0], &[], &[], &[])
            .is_err()
    );
    assert!(
        pc.refactor(1, 0, &[0.0, f64::NAN, 0.0], &[], &[], &[])
            .is_err()
    );
    assert!(pc.refactor(4, 0, &[], &[], &[], &[]).is_err());
    pc.refactor(1, 0, &[0.0, 2.0, 0.0], &[], &[], &[]).unwrap();
    let mut y = [11.0];
    assert!(pc.apply(&[f64::NAN], &mut y).is_err());
    assert_eq!(y, [11.0]);
    assert!(pc.apply(&[], &mut y).is_err());
    assert_eq!(y, [11.0]);
    assert!(
        pc.refactor(
            1,
            1,
            &[0.0, f64::MIN_POSITIVE, 0.0],
            &[f64::MAX],
            &[1.0],
            &[1.0]
        )
        .is_err()
    );
    assert!(!pc.is_valid());
    pc.refactor(1, 0, &[0.0, f64::MIN_POSITIVE, 0.0], &[], &[], &[])
        .unwrap();
    assert!(pc.apply(&[f64::MAX], &mut y).is_err());
}
