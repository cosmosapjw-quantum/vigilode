//! Secant stiffness estimate from one DOPRI5 trial (pure, no RHS calls).

use serde::{Deserialize, Serialize};
use std::fmt;

/// Exact coefficients of `Y7 - Y6 = h * sum_j W[j] * k_j` (fifth-order output
/// minus the sixth stage state; both at `t + h`). The weights sum to zero.
pub const DOPRI5_Y7_MINUS_Y6: [f64; 6] = [
    -34913.0 / 12672.0,
    355.0 / 33.0,
    -310624.0 / 36729.0,
    787.0 / 2112.0,
    -3645.0 / 74624.0,
    11.0 / 84.0,
];

/// Magnitude of the point where the DOPRI5 stability region meets the
/// negative real axis: R(x) = -1 at x = -3.306568... (25-digit Wolfram root,
/// rounded down so the threshold below is never above the true boundary).
pub const DOPRI5_REAL_AXIS_BOUNDARY: f64 = 3.306_568;

/// Status code or refusal reason from guarded DOPRI5 secant evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SecantStatus {
    /// Degenerate displacement (||d||_2 < 1e-15 or invalid dimension/step).
    Degenerate,
    /// Divergent slope (||g||_2 / ||d||_2 > 1e15) or nonfinite stage data.
    NonFiniteOrDivergent,
    /// Kink or event boundary detected (step-to-step jump in h*lambda > 1e3 or sudden sign flip with high magnitude).
    KinkDetected,
}

impl fmt::Display for SecantStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Degenerate => write!(f, "degenerate stages or displacement"),
            Self::NonFiniteOrDivergent => write!(f, "nonfinite stages or divergent slope"),
            Self::KinkDetected => write!(f, "kink or event boundary detected"),
        }
    }
}

impl std::error::Error for SecantStatus {}

/// State and thresholds for guarded DOPRI5 secant stiffness evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SecantScreenGuard {
    /// Previous step's computed h*lambda value.
    pub prev_h_lambda: Option<f64>,
    /// Maximum step-to-step jump in h*lambda before flagging a kink (default: 1.0e3).
    pub max_step_jump: f64,
    /// Magnitude threshold for sudden sign flip kink detection (default: 10.0).
    pub sign_flip_threshold: f64,
    /// Degenerate displacement threshold ||d||_2 (default: 1.0e-15).
    pub degenerate_tol: f64,
    /// Divergent slope threshold ||g||_2 / ||d||_2 (default: 1.0e15).
    pub divergent_slope_tol: f64,
}

impl Default for SecantScreenGuard {
    fn default() -> Self {
        Self {
            prev_h_lambda: None,
            max_step_jump: 1.0e3,
            sign_flip_threshold: 10.0,
            degenerate_tol: 1.0e-15,
            divergent_slope_tol: 1.0e15,
        }
    }
}

impl SecantScreenGuard {
    /// Creates a new guard with default thresholds.
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets the history of the guard.
    pub fn reset(&mut self) {
        self.prev_h_lambda = None;
    }
}

/// Guarded secant estimate `h * q` with `q = <d, g> / <d, d>`, `d = Y7 - Y6`,
/// `g = k7 - k6`. Performs zero heap allocations.
pub fn dopri5_secant_h_lambda_guarded(
    h: f64,
    stages: &[f64],
    n: usize,
    guard: &mut SecantScreenGuard,
) -> Result<f64, SecantStatus> {
    if n == 0 || stages.len() < 7 * n || !(h.is_finite() && h > 0.0) {
        return Err(SecantStatus::Degenerate);
    }

    let (mut dd, mut dg, mut gg) = (0.0f64, 0.0f64, 0.0f64);
    for i in 0..n {
        let mut d = 0.0;
        for (j, &w) in DOPRI5_Y7_MINUS_Y6.iter().enumerate() {
            d += w * stages[j * n + i];
        }
        d *= h;
        let g = stages[6 * n + i] - stages[5 * n + i];
        dd += d * d;
        dg += d * g;
        gg += g * g;
    }

    if !(dd.is_finite() && dg.is_finite() && gg.is_finite()) {
        return Err(SecantStatus::NonFiniteOrDivergent);
    }

    let norm_d = dd.sqrt();
    if norm_d < guard.degenerate_tol {
        return Err(SecantStatus::Degenerate);
    }

    let norm_g = gg.sqrt();
    let slope = norm_g / norm_d;
    if !slope.is_finite() || slope > guard.divergent_slope_tol {
        return Err(SecantStatus::NonFiniteOrDivergent);
    }

    let h_lambda = h * dg / dd;
    if !h_lambda.is_finite() {
        return Err(SecantStatus::NonFiniteOrDivergent);
    }

    if let Some(prev) = guard.prev_h_lambda {
        let jump = (h_lambda - prev).abs();
        let sign_flip = (prev < 0.0 && h_lambda > 0.0) || (prev > 0.0 && h_lambda < 0.0);
        let high_mag =
            prev.abs() > guard.sign_flip_threshold || h_lambda.abs() > guard.sign_flip_threshold;
        if jump > guard.max_step_jump || (sign_flip && high_mag) {
            return Err(SecantStatus::KinkDetected);
        }
    }

    guard.prev_h_lambda = Some(h_lambda);
    Ok(h_lambda)
}

/// Authority classification of a stiffness screen or estimate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SecantClassification {
    /// Directional secant is an empirical estimate only, dependent on direction d.
    EstimateOnly,
    /// Screen remains observer-only; decision gate failed off-registry holdout.
    ObserverOnly,
    /// Formally certified spectrum or enclosure.
    Certified,
}

/// Calibration outcome on scalar linear test equation y' = lambda * y.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScalarLinearCalibration {
    /// Nominal test eigenvalue.
    pub lambda: f64,
    /// Test step size.
    pub h: f64,
    /// Secant-recovered h*lambda value.
    pub recovered_h_lambda: f64,
    /// Exact mathematical h*lambda value.
    pub exact_h_lambda: f64,
    /// Relative error between recovered and exact h*lambda.
    pub relative_error: f64,
    /// Whether the stiffness flag was correctly raised.
    pub flags_stiffness: bool,
}

/// Calibration outcome on a nonlinear stiff test problem.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NonlinearStiffCalibration {
    /// Problem identifier (e.g. "robertson", "van_der_pol").
    pub problem: String,
    /// Test step size.
    pub h: f64,
    /// Minimum secant h*lambda observed.
    pub min_h_lambda: f64,
    /// Whether the stiff boundary layer was correctly detected.
    pub detected_stiff_boundary: bool,
}

/// Calibration outcome demonstrating nonnormal directional dependency.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NonnormalLinearCalibration {
    /// Description of the nonnormal system.
    pub matrix_description: String,
    /// True eigenvalues of the matrix.
    pub true_spectrum: [f64; 2],
    /// Directional secant estimates along different initial state directions.
    pub directional_secants: Vec<(Vec<f64>, f64)>,
    /// Whether directional secants depend on direction d.
    pub depends_on_direction: bool,
    /// Whether directional secants lie outside the true spectrum.
    pub lies_outside_spectrum: bool,
    /// Authority classification (strictly EstimateOnly).
    pub classification: SecantClassification,
}

/// Calibration outcome recording off-registry Brusselator performance per L-0213.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OffRegistryBrusselatorCalibration {
    /// Problem identifier.
    pub problem: String,
    /// Dimension of the spatial discretization.
    pub n: usize,
    /// Relative tolerance.
    pub rtol: f64,
    /// Number of integration windows evaluated.
    pub windows_evaluated: usize,
    /// Number of windows agreeing with intervention test.
    pub matching_windows: usize,
    /// Fraction of agreeing windows.
    pub agreement_fraction: f64,
    /// Required gate threshold (0.90).
    pub required_gate_threshold: f64,
    /// Whether the gate was satisfied.
    pub meets_gate: bool,
    /// Authority classification (ObserverOnly).
    pub classification: SecantClassification,
}

/// Calibration report covering four numerical regimes (WP-04).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecantCalibrationReport {
    /// Scalar linear regime calibration.
    pub scalar_linear: ScalarLinearCalibration,
    /// Nonlinear stiff regime calibrations.
    pub nonlinear_stiff: Vec<NonlinearStiffCalibration>,
    /// Nonnormal linear regime calibration.
    pub nonnormal_linear: NonnormalLinearCalibration,
    /// Off-registry Brusselator calibration.
    pub off_registry_brusselator: OffRegistryBrusselatorCalibration,
    /// Overall recommendation regarding default promotion.
    pub overall_recommendation: String,
}

/// Butcher tableau A coefficients for DOPRI5
const DOPRI5_TABLEAU_A: [[f64; 7]; 7] = [
    [0.0; 7],
    [1.0 / 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [3.0 / 40.0, 9.0 / 40.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [44.0 / 45.0, -56.0 / 15.0, 32.0 / 9.0, 0.0, 0.0, 0.0, 0.0],
    [
        19372.0 / 6561.0,
        -25360.0 / 2187.0,
        64448.0 / 6561.0,
        -212.0 / 729.0,
        0.0,
        0.0,
        0.0,
    ],
    [
        9017.0 / 3168.0,
        -355.0 / 33.0,
        46732.0 / 5247.0,
        49.0 / 176.0,
        -5103.0 / 18656.0,
        0.0,
        0.0,
    ],
    [
        35.0 / 384.0,
        0.0,
        500.0 / 1113.0,
        125.0 / 192.0,
        -2187.0 / 6784.0,
        11.0 / 84.0,
        0.0,
    ],
];

/// Simulates 7 DOPRI5 stages for an arbitrary RHS function into a preallocated slice `k`.
fn simulate_dopri5_stages<F>(h: f64, y0: &[f64], k: &mut [f64], rhs: F)
where
    F: Fn(&[f64], &mut [f64]),
{
    let n = y0.len();
    assert!(k.len() >= 7 * n);
    let mut ys = [0.0; 16];
    let mut f_val = [0.0; 16];
    assert!(n <= 16);
    for s in 0..7 {
        ys[..n].copy_from_slice(y0);
        for j in 0..s {
            let a_sj = DOPRI5_TABLEAU_A[s][j];
            if a_sj != 0.0 {
                for i in 0..n {
                    ys[i] += h * a_sj * k[j * n + i];
                }
            }
        }
        rhs(&ys[..n], &mut f_val[..n]);
        for i in 0..n {
            k[s * n + i] = f_val[i];
        }
    }
}

/// Runs screen calibration across four regimes:
/// 1. Scalar linear: flags stiffness correctly and recovers exact h*lambda.
/// 2. Nonlinear stiff (Robertson, Van der Pol): detects stiff boundary transitions.
/// 3. Nonnormal linear (J = [[-1, 10], [0, -2]]): demonstrates directional dependence
///    and that secant is strictly EstimateOnly, not full spectrum.
/// 4. Off-registry Brusselator: records agreement (0.5833 < 0.90 per L-0213) and
///    marks result as ObserverOnly (do NOT widen tolerance or promote to default gate).
pub fn calibrate_secant_screen() -> SecantCalibrationReport {
    // Regime a: Scalar linear (y' = lam * y, lam = -1e4, h = 3.2e-4)
    let lam = -1.0e4;
    let h_scalar = 3.2e-4;
    let mut k_scalar = [0.0; 7];
    simulate_dopri5_stages(h_scalar, &[1.0], &mut k_scalar, |y, out| {
        out[0] = lam * y[0];
    });
    let mut guard = SecantScreenGuard::new();
    let hl_recovered = dopri5_secant_h_lambda_guarded(h_scalar, &k_scalar, 1, &mut guard)
        .expect("scalar linear evaluation");
    let exact_hl = h_scalar * lam;
    let rel_err = (hl_recovered - exact_hl).abs() / exact_hl.abs();
    let flags_stiff = dopri5_stability_limited(hl_recovered, 0.8);
    let scalar_linear = ScalarLinearCalibration {
        lambda: lam,
        h: h_scalar,
        recovered_h_lambda: hl_recovered,
        exact_h_lambda: exact_hl,
        relative_error: rel_err,
        flags_stiffness: flags_stiff,
    };

    // Regime b: Nonlinear stiff (Robertson and Van der Pol)
    // 1. Robertson in active transition: y0 = [0.8, 2e-5, 0.2], h = 1e-3
    let h_rob = 1.0e-3;
    let y0_rob = [0.8, 2.0e-5, 0.2];
    let mut k_rob = [0.0; 21];
    simulate_dopri5_stages(h_rob, &y0_rob, &mut k_rob, |y, out| {
        out[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
        out[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
        out[2] = 3.0e7 * y[1] * y[1];
    });
    guard.reset();
    let hl_rob = dopri5_secant_h_lambda_guarded(h_rob, &k_rob, 3, &mut guard)
        .expect("robertson secant evaluation");

    // 2. Van der Pol (mu = 500, relaxation state [2.0, 0.0], h = 1e-3)
    let h_vdp = 1.0e-3;
    let y0_vdp = [2.0, 0.0];
    let mu = 500.0;
    let mut k_vdp = [0.0; 14];
    simulate_dopri5_stages(h_vdp, &y0_vdp, &mut k_vdp, |y, out| {
        out[0] = y[1];
        out[1] = mu * (1.0 - y[0] * y[0]) * y[1] - y[0];
    });
    guard.reset();
    let hl_vdp = dopri5_secant_h_lambda_guarded(h_vdp, &k_vdp, 2, &mut guard)
        .expect("van der pol secant evaluation");

    let nonlinear_stiff = vec![
        NonlinearStiffCalibration {
            problem: "robertson".into(),
            h: h_rob,
            min_h_lambda: hl_rob,
            detected_stiff_boundary: hl_rob < -1.0,
        },
        NonlinearStiffCalibration {
            problem: "van_der_pol".into(),
            h: h_vdp,
            min_h_lambda: hl_vdp,
            detected_stiff_boundary: hl_vdp < -1.0,
        },
    ];

    // Regime c: Nonnormal linear (J = [[-1, 10], [0, -2]])
    let h_nn = 0.01;
    let j_mat = |y: &[f64], out: &mut [f64]| {
        out[0] = -y[0] + 10.0 * y[1];
        out[1] = -2.0 * y[1];
    };
    let dirs = [[1.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, -0.2]];
    let mut directional_secants = Vec::new();
    let mut min_q = f64::INFINITY;
    let mut max_q = f64::NEG_INFINITY;
    let mut k_nn = [0.0; 14];

    for dir in dirs {
        guard.reset();
        simulate_dopri5_stages(h_nn, &dir, &mut k_nn, j_mat);
        let hl = dopri5_secant_h_lambda_guarded(h_nn, &k_nn, 2, &mut guard)
            .expect("nonnormal secant evaluation");
        let q = hl / h_nn;
        if q < min_q {
            min_q = q;
        }
        if q > max_q {
            max_q = q;
        }
        directional_secants.push((dir.to_vec(), q));
    }

    let true_spectrum = [-2.0, -1.0];
    let depends_on_direction = (max_q - min_q).abs() > 0.5;
    let lies_outside_spectrum = min_q < true_spectrum[0] || max_q > true_spectrum[1];

    let nonnormal_linear = NonnormalLinearCalibration {
        matrix_description: "J = [[-1, 10], [0, -2]], spectrum = [-2, -1]".into(),
        true_spectrum,
        directional_secants,
        depends_on_direction,
        lies_outside_spectrum,
        classification: SecantClassification::EstimateOnly,
    };

    // Regime d: Off-registry Brusselator (recorded per L-0213)
    let off_registry_brusselator = OffRegistryBrusselatorCalibration {
        problem: "brusselator_1d_n64".into(),
        n: 64,
        rtol: 1.0e-6,
        windows_evaluated: 24,
        matching_windows: 14,
        agreement_fraction: 14.0 / 24.0,
        required_gate_threshold: 0.90,
        meets_gate: false,
        classification: SecantClassification::ObserverOnly,
    };

    SecantCalibrationReport {
        scalar_linear,
        nonlinear_stiff,
        nonnormal_linear,
        off_registry_brusselator,
        overall_recommendation: "ObserverOnly: off-registry agreement 0.5833 < 0.90 per L-0213; \
             retain stiffness_gate_fraction = None default; do NOT widen tolerance \
             or promote to default gate"
            .into(),
    }
}

/// Secant estimate `h * q` with `q = <d, g> / <d, d>`, `d = Y7 - Y6`,
/// `g = k7 - k6 = f(t+h, Y7) - f(t+h, Y6)`. `stages` holds the seven stage
/// derivatives of one trial, `k_j = stages[j*n..(j+1)*n]`.
/// Returns `None` when `n == 0`, the slice is too short, `d` vanishes or a
/// value is not finite.
pub fn dopri5_secant_h_lambda(h: f64, stages: &[f64], n: usize) -> Option<f64> {
    if n == 0 || stages.len() < 7 * n || !(h.is_finite() && h > 0.0) {
        return None;
    }
    let (mut dd, mut dg) = (0.0f64, 0.0f64);
    for i in 0..n {
        let mut d = 0.0;
        for (j, &w) in DOPRI5_Y7_MINUS_Y6.iter().enumerate() {
            d += w * stages[j * n + i];
        }
        d *= h;
        let g = stages[6 * n + i] - stages[5 * n + i];
        dd += d * d;
        dg += d * g;
    }
    if !(dd.is_finite() && dg.is_finite()) || dd == 0.0 {
        return None;
    }
    let h_lambda = h * dg / dd;
    h_lambda.is_finite().then_some(h_lambda)
}

/// True when the secant estimate is dissipative and at least `fraction` of the
/// real-axis stability boundary (`fraction = 0.8` in the preregistered gate).
pub fn dopri5_stability_limited(h_lambda: f64, fraction: f64) -> bool {
    h_lambda < 0.0 && -h_lambda >= fraction * DOPRI5_REAL_AXIS_BOUNDARY
}

#[cfg(test)]
mod tests {
    use super::*;
    const A: [[f64; 7]; 7] = [
        [0.0; 7],
        [1.0 / 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [3.0 / 40.0, 9.0 / 40.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [44.0 / 45.0, -56.0 / 15.0, 32.0 / 9.0, 0.0, 0.0, 0.0, 0.0],
        [
            19372.0 / 6561.0,
            -25360.0 / 2187.0,
            64448.0 / 6561.0,
            -212.0 / 729.0,
            0.0,
            0.0,
            0.0,
        ],
        [
            9017.0 / 3168.0,
            -355.0 / 33.0,
            46732.0 / 5247.0,
            49.0 / 176.0,
            -5103.0 / 18656.0,
            0.0,
            0.0,
        ],
        [
            35.0 / 384.0,
            0.0,
            500.0 / 1113.0,
            125.0 / 192.0,
            -2187.0 / 6784.0,
            11.0 / 84.0,
            0.0,
        ],
    ];

    /// Stage derivatives of one DOPRI5 trial for the linear system y' = diag(lam) y.
    fn linear_stages(h: f64, y: &[f64], lam: &[f64]) -> Vec<f64> {
        let n = y.len();
        let mut k = vec![0.0; 7 * n];
        for s in 0..7 {
            for i in 0..n {
                let mut yi = y[i];
                for j in 0..s {
                    yi += h * A[s][j] * k[j * n + i];
                }
                k[s * n + i] = lam[i] * yi;
            }
        }
        k
    }

    #[test]
    fn weights_equal_tableau_difference_and_sum_to_zero() {
        let mut sum = 0.0;
        for j in 0..6 {
            let diff = A[6][j] - A[5][j];
            assert!(
                (DOPRI5_Y7_MINUS_Y6[j] - diff).abs() <= 4.0 * f64::EPSILON * diff.abs().max(1.0)
            );
            sum += DOPRI5_Y7_MINUS_Y6[j];
        }
        assert!(sum.abs() < 1e-14);
    }

    #[test]
    fn scalar_linear_mode_is_recovered() {
        for (h, lam) in [(1e-3, -1.0e4), (3.2e-4, -1.0e4), (0.01, -2.0), (0.01, 3.0)] {
            let k = linear_stages(h, &[1.0], &[lam]);
            let hl = dopri5_secant_h_lambda(h, &k, 1).expect("valid test input");
            assert!(
                (hl - h * lam).abs() <= 1e-9 * (h * lam).abs(),
                "h={h} lam={lam} got {hl}"
            );
        }
        let k2 = linear_stages(1e-3, &[1.0, 2.0], &[-1.0e4, -2.0e4]);
        let hl2 = dopri5_secant_h_lambda(1e-3, &k2, 2).expect("valid 2d");
        assert!(hl2 < -15.0 && hl2 > -20.0);
    }

    #[test]
    fn gate_flags_stiff_forced_regime_and_not_rotation() {
        // stiff-forced: DOPRI5 settles at h*lambda ~ -3.2 .. -3.30.
        let k = linear_stages(3.2e-4, &[1.0], &[-1.0e4]);
        assert!(dopri5_stability_limited(
            dopri5_secant_h_lambda(3.2e-4, &k, 1).expect("valid test input"),
            0.8
        ));
        // rotation y' = [-100 y1, 100 y0]: no real dissipative direction.
        let h = 1.0e-3;
        let n = 2;
        let mut kk = vec![0.0; 7 * n];
        let y = [1.0, 0.0];
        for s in 0..7 {
            let mut ys = y;
            for j in 0..s {
                ys[0] += h * A[s][j] * kk[j * n];
                ys[1] += h * A[s][j] * kk[j * n + 1];
            }
            kk[s * n] = -100.0 * ys[1];
            kk[s * n + 1] = 100.0 * ys[0];
        }
        let hl = dopri5_secant_h_lambda(h, &kk, n).expect("valid test input");
        assert!(hl.abs() < 0.05, "rotation secant {hl}");
        assert!(!dopri5_stability_limited(hl, 0.8));
    }

    #[test]
    fn degenerate_inputs_return_none() {
        assert_eq!(dopri5_secant_h_lambda(1e-3, &[0.0; 7], 1), None);
        assert_eq!(dopri5_secant_h_lambda(1e-3, &[1.0; 6], 1), None);
        assert_eq!(dopri5_secant_h_lambda(0.0, &[1.0; 7], 1), None);
        assert_eq!(dopri5_secant_h_lambda(1e-3, &[1.0; 7], 0), None);
        assert_eq!(dopri5_secant_h_lambda(1e-3, &[1.0; 13], 2), None);
        assert_eq!(
            dopri5_secant_h_lambda(-1e-3, &linear_stages(1e-3, &[1.0], &[-10.0]), 1),
            None
        );
        assert_eq!(
            dopri5_secant_h_lambda(1.0, &[1e200, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0], 1),
            None
        );
        assert!(!dopri5_stability_limited(0.0, 0.0));
        assert!(dopri5_stability_limited(
            -0.8 * DOPRI5_REAL_AXIS_BOUNDARY,
            0.8
        ));
        assert!(!dopri5_stability_limited(f64::NAN, 0.8));
        assert!(!dopri5_stability_limited(-2.0, 0.8));
        assert!(dopri5_stability_limited(-2.7, 0.8));
    }
}
