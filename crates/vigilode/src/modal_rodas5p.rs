//! Fixed-step RODAS5P endpoint and signed embedded defect for homogeneous
//! real, nonpositive, independent modes. The caller supplies modal amplitudes
//! and their constant eigenvalues. A physical-space transform and its inverse
//! belong to the caller. The endpoint-only transfer cannot supply stage data
//! or the protected continuous extension.
//!
//! The exact-rational generator in research/modal_transfer_20261124/check_modal.py
//! uses the frozen RODAS5P decimal snapshot (SHA-256 211255f6125b648c6955833c3da537d9c031ec78168b2918563c29fa88c8bce6).
//! It recovers the project raw-increment estimator U_8. Reordering changes
//! floating-point roundoff. Restrict this API to source-declared modal ODEs.

use std::{error::Error, fmt};

const GAMMA: f64 = 0.21193756319429014;
const STEP_NEAR: [f64; 9] = [
    1.0,
    4.718370754707945,
    6.413140534733579,
    -0.03711938290786116,
    -3.194676945691334,
    2.119020688592897,
    0.26447102375230613,
    -2.31733744759843,
    -1.5004946063828076e-16,
];
const ERROR_NEAR: [f64; 9] = [
    0.0,
    7.021296121659102e-15,
    -3.107526549027429e-14,
    -4.1334430160701206e-14,
    2.551947726111874e-14,
    0.2896713839922561,
    -2.0276660636062025,
    -2.31733744759843,
    -1.5004946063828076e-16,
];
const STEP_STIFF: [f64; 9] = [
    0.0,
    -2.653645513455822,
    18.79738187923181,
    -52.46443585520473,
    71.28409563357381,
    -48.131891853487964,
    16.48583315694131,
    -2.3173374475984287,
    -1.5004946063828076e-16,
];
const ERROR_STIFF: [f64; 9] = [
    0.0,
    -2.6070088315906705,
    15.352381605551674,
    -37.65677555389858,
    49.24346279189075,
    -36.208418633937555,
    14.193696069582804,
    -2.317337447598429,
    -1.5004946063828076e-16,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Contract failures for fixed-step modal RODAS5P.
pub enum ModalError {
    /// The requested step is nonfinite or nonpositive.
    InvalidStep,
    /// The supplied rates violate the fixed-step dissipative domain.
    InvalidSpectrum,
    /// An amplitude is nonfinite.
    InvalidState,
    /// State or destination shape differs from the rate vector.
    WrongLength,
    /// A finite amplitude cannot be scaled into a finite result.
    NonfiniteOutput,
}

impl fmt::Display for ModalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidStep => "a modal step requires a finite positive step size",
            Self::InvalidSpectrum => {
                "modal rates must be a nonempty sequence of finite nonpositive eigenvalues"
            }
            Self::InvalidState => "modal amplitudes must be finite",
            Self::WrongLength => "modal state, outputs and rate vector must have the same length",
            Self::NonfiniteOutput => "the scaled modal output exceeds the floating-point range",
        })
    }
}

impl Error for ModalError {}

/// A reusable pair of endpoint/error multipliers for one step size and spectrum.
/// Construction costs O(n). Each subsequent step needs two scalar multiplies
/// per mode. Rebuild when the step size or operator changes.
#[derive(Debug, Clone)]
pub struct ModalRodas5pStep {
    factors: Vec<(f64, f64)>,
    check_output_range: bool,
}

fn estrin(a: &[f64; 9], x: f64) -> f64 {
    let x2 = x * x;
    let x4 = x2 * x2;
    let left = (a[0] + x * a[1]) + x2 * (a[2] + x * a[3]);
    let right = (a[4] + x * a[5]) + x2 * (a[6] + x * a[7]);
    left + x4 * (right + x4 * a[8])
}

fn transfer(h: f64, rate: f64) -> (f64, f64) {
    // Multiply h and rate first when representable to preserve subnormal h.
    // For overflow in h*rate, 1/(1-gamma*z) is still representable in
    // parts of the subnormal range. Factor the reciprocal without forming z.
    let z = h * rate;
    let gamma_z = GAMMA * z;
    if gamma_z >= -1.0 {
        let x = gamma_z / (1.0 - gamma_z);
        (estrin(&STEP_NEAR, x), estrin(&ERROR_NEAR, x))
    } else {
        let p = if z.is_finite() {
            1.0 / (1.0 - gamma_z)
        } else {
            ((1.0 / GAMMA) / h) / (-rate)
        };
        (estrin(&STEP_STIFF, p), estrin(&ERROR_STIFF, p))
    }
}

impl ModalRodas5pStep {
    /// Prepare fixed-step factors for a nonempty set of finite, nonpositive rates.
    pub fn new(h: f64, rates: &[f64]) -> Result<Self, ModalError> {
        if !h.is_finite() || h <= 0.0 {
            return Err(ModalError::InvalidStep);
        }
        if rates.is_empty() || rates.iter().any(|r| !r.is_finite() || *r > 0.0) {
            return Err(ModalError::InvalidSpectrum);
        }
        let factors: Vec<_> = rates.iter().map(|&rate| transfer(h, rate)).collect();
        let check_output_range = factors.iter().any(|&(r, e)| r.abs() > 1.0 || e.abs() > 1.0);
        Ok(Self {
            factors,
            check_output_range,
        })
    }

    /// Apply the multipliers to amplitudes in the same order as the supplied
    /// rates. On error both destination slices retain their previous values.
    pub fn apply_into(
        &self,
        state: &[f64],
        endpoint: &mut [f64],
        embedded_error: &mut [f64],
    ) -> Result<(), ModalError> {
        let n = self.factors.len();
        if state.len() != n || endpoint.len() != n || embedded_error.len() != n {
            return Err(ModalError::WrongLength);
        }
        // Full reduction admits SIMD and preserves transactional validation.
        let finite = state.iter().fold(true, |valid, y| valid & y.is_finite());
        if !finite {
            return Err(ModalError::InvalidState);
        }
        if self.check_output_range
            && state
                .iter()
                .zip(&self.factors)
                .any(|(&y, &(r, e))| !(r * y).is_finite() || !(e * y).is_finite())
        {
            return Err(ModalError::NonfiniteOutput);
        }
        for (((&y, &(r, e)), y_out), e_out) in state
            .iter()
            .zip(&self.factors)
            .zip(endpoint)
            .zip(embedded_error)
        {
            *y_out = r * y;
            *e_out = e * y;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_and_validation_are_transactional() {
        assert_eq!(
            ModalRodas5pStep::new(0.0, &[0.0]).unwrap_err(),
            ModalError::InvalidStep
        );
        for rates in [&[][..], &[1.0], &[f64::NEG_INFINITY], &[f64::NAN]] {
            assert_eq!(
                ModalRodas5pStep::new(1.0, rates).unwrap_err(),
                ModalError::InvalidSpectrum
            );
        }
        let plan = ModalRodas5pStep::new(1.0, &[-0.0, -1.0e300]).unwrap();
        let mut y = [10.0; 2];
        let mut e = [20.0; 2];
        assert!(matches!(
            plan.apply_into(&[1.0, f64::NAN], &mut y, &mut e),
            Err(ModalError::InvalidState)
        ));
        assert_eq!((y, e), ([10.0; 2], [20.0; 2]));
        assert_eq!(
            plan.apply_into(&[], &mut y, &mut e),
            Err(ModalError::WrongLength)
        );
        plan.apply_into(&[3.0, 1.0], &mut y, &mut e).unwrap();
        assert_eq!((y[0], e[0]), (3.0, 0.0));
        assert!(y[1] < 0.0 && y[1].abs() < 1.3e-299);
        let (r, _) = transfer(4.0, -f64::MAX);
        assert!(r.is_finite() && r < 0.0 && r.abs() < 2e-308);
    }

    #[test]
    fn modal_flow_converges_and_embedded_error_has_correct_sign() {
        let (r1, e1) = transfer(0.125, -4.0);
        let (r2, e2) = transfer(0.0625, -4.0);
        let exact = (-4.0_f64).exp();
        assert!((r1.powi(8) - exact).abs() > 40.0 * (r2.powi(16) - exact).abs());
        assert!(e1 < 0.0 && e2 < 0.0 && e1.abs() > 12.0 * e2.abs());
    }
}
