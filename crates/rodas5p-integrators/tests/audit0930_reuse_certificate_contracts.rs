//! The M08 reuse certificate rejects non-finite intermediates (audit
//! 2026-09-30, AD-03).

use rodas5p_core::{CoreResult, DenseMatrix, DenseOperator, LinearOperator, Preconditioner};
use rodas5p_integrators::audit2_preconditioner_reuse_certificate;

struct TwoByTwo;

impl Preconditioner for TwoByTwo {
    fn dimension(&self) -> usize {
        2
    }

    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        y[0] = 2.0 * x[0] - 2.0 * x[1];
        y[1] = y[0];
        Ok(())
    }
}

struct Nan;

impl Preconditioner for Nan {
    fn dimension(&self) -> usize {
        2
    }

    fn apply(&self, _: &[f64], y: &mut [f64]) -> CoreResult<()> {
        y.fill(f64::NAN);
        Ok(())
    }
}

struct NanOperator;

impl LinearOperator for NanOperator {
    fn dimension(&self) -> usize {
        2
    }

    fn apply(&self, _: &[f64], y: &mut [f64]) -> CoreResult<()> {
        y.fill(f64::NAN);
        Ok(())
    }

    fn token(&self) -> u64 {
        1
    }
}

#[test]
fn reuse_certificate_rejects_nonfinite_intermediates() {
    // Finite W and P with P W = 0 exactly: ||I - P W||_1 = 1, no
    // certificate. In floating point 2e308 - 2e308 = inf - inf = NaN, which
    // f64::max dropped, so the old code certified with epsilon = 0.
    let singular = DenseOperator::new(
        DenseMatrix::from_rows(&[&[1.0e308, 1.0e308], &[1.0e308, 1.0e308]]).unwrap(),
    )
    .unwrap();
    assert!(audit2_preconditioner_reuse_certificate(&singular, &TwoByTwo, false).is_err());
    let identity = DenseOperator::new(DenseMatrix::identity(2)).unwrap();
    assert!(audit2_preconditioner_reuse_certificate(&identity, &Nan, false).is_err());
    assert!(audit2_preconditioner_reuse_certificate(&NanOperator, &TwoByTwo, false).is_err());
    // A finite P W: P = [[2, -2], [2, -2]], W = I, I - P = [[-1, 2], [-2, 3]],
    // column sums 3 and 5, so epsilon = 5.
    let certificate = audit2_preconditioner_reuse_certificate(&identity, &TwoByTwo, false).unwrap();
    assert_eq!(certificate.epsilon_l1, 5.0);
    assert!(!certificate.certified);
}
