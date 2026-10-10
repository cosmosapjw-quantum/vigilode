//! Band verification must not accept a wrong band when the products leave the
//! range of a plain sum of squares (wave-1 review S1 on SP03): with entries
//! near 1e200 both norms used to overflow to Inf, with entries near 1e-200
//! both used to underflow to 0, and either comparison accepted a band that
//! drops the subdiagonal.

use std::sync::Arc;

use rodas5p_core::CoreResult;
use rodas5p_integrators::{
    BandedJacobianFn, OdeProblem, StructureDeclaration, StructureError, verify_declared_band,
};

/// `J = s * tridiag(1, -2, 1)`, JVP only.
fn scaled_tridiagonal(n: usize, s: f64) -> OdeProblem {
    let apply = move |v: &[f64], out: &mut [f64]| {
        for i in 0..n {
            let left = if i > 0 { v[i - 1] } else { 0.0 };
            let right = if i + 1 < n { v[i + 1] } else { 0.0 };
            out[i] = s * left - 2.0 * s * v[i] + s * right;
        }
    };
    let rhs = Arc::new(
        move |_t: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
            apply(y, out);
            Ok(())
        },
    );
    let jvp = Arc::new(
        move |_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| -> CoreResult<()> {
            apply(v, out);
            Ok(())
        },
    );
    OdeProblem::new(
        "scaled-tridiagonal",
        n,
        rhs,
        None,
        None,
        Some(jvp),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

/// Band storage `band[i * width + (j + lower - i)]`; `lower = 0` drops the
/// subdiagonal of the true Jacobian.
fn fill(n: usize, lower: usize, s: f64) -> BandedJacobianFn {
    Arc::new(move |_t: f64, _y: &[f64], band: &mut [f64]| {
        let width = lower + 2;
        for i in 0..n {
            band[i * width + lower] = -2.0 * s;
            if lower == 1 && i > 0 {
                band[i * width] = s;
            }
            if i + 1 < n {
                band[i * width + lower + 1] = s;
            }
        }
        Ok(())
    })
}

fn verify(n: usize, lower: usize, s: f64) -> Result<(), StructureError> {
    let p = scaled_tridiagonal(n, s)
        .with_declared_structure(StructureDeclaration::banded(n, lower, 1, fill(n, lower, s)))
        .unwrap();
    verify_declared_band(&p, 0.0, &vec![0.0; n]).map(|_| ())
}

#[test]
fn a_correct_band_is_accepted_at_every_scale() {
    for s in [1e-200, 1e-160, 1.0, 1e160, 1e200] {
        verify(16, 1, s).unwrap_or_else(|e| panic!("scale {s:e}: {e:?}"));
    }
}

#[test]
fn a_narrow_band_is_rejected_at_every_scale() {
    for s in [1e-200, 1e-160, 1.0, 1e160, 1e200] {
        match verify(16, 0, s) {
            Err(StructureError::BandVerificationMismatch { .. }) => {}
            other => panic!("scale {s:e}: narrow band not rejected: {other:?}"),
        }
    }
}

#[test]
fn an_unrepresentable_difference_rejects() {
    // Entries near f64::MAX: the products themselves stay finite only for
    // small |v|, and any non-finite product, difference or norm must reject
    // rather than compare Inf with Inf.
    match verify(16, 0, 1e307) {
        Err(StructureError::BandVerificationMismatch { .. })
        | Err(StructureError::BandVerificationFailed(_)) => {}
        other => panic!("not rejected: {other:?}"),
    }
}
