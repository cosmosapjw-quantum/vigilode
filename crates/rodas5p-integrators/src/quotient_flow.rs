//! Declared constant damped-rotation quotient actions and RHS-only Lawson RK.
//!
//! The caller supplies `A = diag([-d,-w;w,-d])` and the **residual** `N` in
//! `y' = A y + N(t,y)`. No Jacobian or derivative is inferred from callbacks.
//! The identity `(A+dI)^2 = -w^2 I` removes state-size linear algebra. Lawson's
//! transformation is implemented with forward semigroup actions only; global
//! inverse damping is never formed. Classical order five holds for fixed A
//! and a smooth residual; it is not a stiffness-uniform nonlinear order claim.
use crate::adaptive::adaptive_next_step_after_attempt_prevalidated;
use crate::dopri5_dense::Dopri5OutputSession;
use crate::{
    AdaptiveControllerState, AffineErrorNorm, DOPRI5_A, DOPRI5_C, Dopri5Config,
    Dopri5IntegrationResult, Dopri5StopReason, OutputSchedule,
};
use rodas5p_core::{CoreError, CoreResult, WorkCounters, scaled_error_component};
use serde::Serialize;
use std::collections::BTreeMap;

const EW: [f64; 7] = [
    71.0 / 57600.0,
    0.0,
    -71.0 / 16695.0,
    71.0 / 1920.0,
    -17253.0 / 339200.0,
    22.0 / 525.0,
    -1.0 / 40.0,
];

/// Constant block `[-damping,-frequency;frequency,-damping]`.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct DampedRotationBlock {
    pub damping: f64,
    pub frequency: f64,
}

#[derive(Clone, Copy, Debug, Default)]
struct Factor {
    c: f64,
    s: f64,
    scale_half: f64,
}
impl Factor {
    fn new(b: DampedRotationBlock, dt: f64) -> CoreResult<Self> {
        Self::new_scaled(b, dt, 1.0)
    }
    fn new_scaled(b: DampedRotationBlock, h: f64, fraction: f64) -> CoreResult<Self> {
        if fraction == 0.0 {
            return Ok(Self {
                c: 1.0,
                s: 0.0,
                scale_half: 1.0,
            });
        }
        let full_damping = b.damping * h;
        let exponent = if full_damping.is_finite() {
            -full_damping * fraction
        } else {
            -b.damping * (h * fraction)
        };
        // sqrt(2)*MAX*exp(-1500) is below half the least subnormal.
        // Only this stronger condition permits skipping a possibly huge phase.
        if exponent < -1500.0 {
            return Ok(Self {
                c: 0.0,
                s: 0.0,
                scale_half: 1.0,
            });
        }
        let full_phase = b.frequency * h;
        let phase = if full_phase.is_finite() {
            full_phase * fraction
        } else {
            b.frequency * (h * fraction)
        };
        if !phase.is_finite() {
            return Err(CoreError::NonFinite(
                "declared-flow phase product overflow".into(),
            ));
        }
        let (s, c) = phase.sin_cos();
        if exponent < -700.0 {
            // Do not round an underflowing coefficient before multiplying a
            // large input: exp(-1000)*1e308 is representable. Splitting also
            // avoids the poor relative precision of subnormal coefficients.
            Ok(Self {
                c,
                s,
                scale_half: (0.5 * exponent).exp(),
            })
        } else {
            let damping = exponent.exp();
            Ok(Self {
                c: damping * c,
                s: damping * s,
                scale_half: 1.0,
            })
        }
    }
    #[inline]
    fn action(self, x: f64, y: f64) -> (f64, f64) {
        let (x, y) = if self.scale_half == 1.0 {
            (x, y)
        } else {
            (
                (x * self.scale_half) * self.scale_half,
                (y * self.scale_half) * self.scale_half,
            )
        };
        (self.c * x - self.s * y, self.s * x + self.c * y)
    }
}

/// Matrix-free flow for an even-dimensional declared block operator.
/// Equal blocks share their transcendental coefficients. Construction is
/// O(n log g), flow actions O(n), and each RK coefficient cache uses O(g)
/// storage, where g is the number of distinct declared blocks.
#[derive(Clone, Debug)]
pub struct DampedRotationFlow {
    groups: Vec<DampedRotationBlock>,
    indices: Vec<usize>,
}
impl DampedRotationFlow {
    pub fn new(blocks: &[DampedRotationBlock]) -> CoreResult<Self> {
        if blocks.is_empty() || blocks.len().checked_mul(2).is_none() {
            return Err(CoreError::Dimension(
                "declared flow requires nonempty block pairs".into(),
            ));
        }
        let mut lookup = BTreeMap::new();
        let mut groups = Vec::new();
        let mut indices = Vec::with_capacity(blocks.len());
        for &b in blocks {
            if !b.damping.is_finite() || b.damping < 0.0 || !b.frequency.is_finite() {
                return Err(CoreError::InvalidInput(
                    "declared flow needs finite damping >= 0 and frequency".into(),
                ));
            }
            let b = DampedRotationBlock {
                damping: if b.damping == 0.0 { 0.0 } else { b.damping },
                frequency: if b.frequency == 0.0 { 0.0 } else { b.frequency },
            };
            let key = (b.damping.to_bits(), b.frequency.to_bits());
            let id = *lookup.entry(key).or_insert_with(|| {
                groups.push(b);
                groups.len() - 1
            });
            indices.push(id);
        }
        Ok(Self { groups, indices })
    }
    pub fn repeated(block: DampedRotationBlock, count: usize) -> CoreResult<Self> {
        if count == 0 || count.checked_mul(2).is_none() {
            return Err(CoreError::Dimension(
                "declared flow block count overflow/zero".into(),
            ));
        }
        let mut result = Self::new(&[block])?;
        result.indices.resize(count, 0);
        Ok(result)
    }
    pub fn dimension(&self) -> usize {
        self.indices.len() * 2
    }
    pub fn distinct_blocks(&self) -> usize {
        self.groups.len()
    }
    fn validate_vectors(&self, input: &[f64], output: &[f64]) -> CoreResult<()> {
        if input.len() != self.dimension() || output.len() != self.dimension() {
            return Err(CoreError::Dimension(
                "declared flow vector dimensions differ".into(),
            ));
        }
        if input.iter().any(|x| !x.is_finite()) {
            return Err(CoreError::NonFinite(
                "declared flow input must be finite".into(),
            ));
        }
        Ok(())
    }
    /// `exp(dt*A) input`, evaluated without matrices or operator probes.
    /// Output is scratch on failure. Binary64 trigonometric rounding is not
    /// enclosed; phase accuracy depends on the represented `frequency*dt`.
    pub fn apply_into(&self, dt: f64, input: &[f64], output: &mut [f64]) -> CoreResult<()> {
        self.validate_vectors(input, output)?;
        if !dt.is_finite() || dt < 0.0 {
            return Err(CoreError::InvalidInput(
                "declared flow requires finite dt >= 0".into(),
            ));
        }
        // Public action avoids a heap cache. The RK workspace below caches all
        // shared factors once per attempted step.
        for (k, &g) in self.indices.iter().enumerate() {
            let f = Factor::new(self.groups[g], dt)?;
            let (x, y) = f.action(input[2 * k], input[2 * k + 1]);
            output[2 * k] = x;
            output[2 * k + 1] = y;
        }
        if output.iter().any(|x| !x.is_finite()) {
            return Err(CoreError::NonFinite("declared flow action overflow".into()));
        }
        Ok(())
    }
    /// Exact-algebra action of the constant-forcing problem `y' = A*y + b`:
    /// `exp(dt*A)*input + dt*phi_1(dt*A)*constant_forcing`.
    ///
    /// This accepts a declared constant forcing, not a sampled nonlinear or
    /// time-dependent RHS. Singular zero blocks are supported. No matrix,
    /// derivative, iteration, or heap allocation is used. Repeated calls with
    /// shared blocks can use [`DampedRotationAffineWorkspace`] to cache factors.
    /// Output is scratch on error; binary64 roundoff is not enclosed.
    pub fn affine_into(
        &self,
        dt: f64,
        input: &[f64],
        constant_forcing: &[f64],
        output: &mut [f64],
    ) -> CoreResult<()> {
        self.validate_vectors(input, output)?;
        self.validate_vectors(constant_forcing, output)?;
        validate_affine_dt(dt)?;
        for (k, &g) in self.indices.iter().enumerate() {
            let factor = AffineFactor::new(self.groups[g], dt)?;
            let (x, y) = factor.action(
                input[2 * k],
                input[2 * k + 1],
                constant_forcing[2 * k],
                constant_forcing[2 * k + 1],
            );
            output[2 * k] = x;
            output[2 * k + 1] = y;
        }
        validate_affine_output(output)
    }
    /// Exact-algebra endpoint for `y'=A*y+c*cos(nu*t)+d*sin(nu*t)`.
    /// The coefficients and frequency are declared constants. Resonance is
    /// regular: shifted frequencies `omega +/- nu` use the phi_1 zero chart.
    /// No residual callback, derivatives, matrices, or iterations are used.
    /// The phase and shifted frequencies must remain finite in binary64.
    #[allow(clippy::too_many_arguments)]
    pub fn harmonic_into(
        &self,
        t: f64,
        dt: f64,
        input: &[f64],
        frequency: f64,
        cosine_coeff: &[f64],
        sine_coeff: &[f64],
        output: &mut [f64],
    ) -> CoreResult<()> {
        self.validate_vectors(input, output)?;
        self.validate_vectors(cosine_coeff, output)?;
        self.validate_vectors(sine_coeff, output)?;
        validate_affine_dt(dt)?;
        let phase = frequency * (t + dt);
        if !t.is_finite() || !frequency.is_finite() || !phase.is_finite() {
            return Err(CoreError::InvalidInput(
                "harmonic flow needs finite time/frequency/endpoint phase".into(),
            ));
        }
        if frequency == 0.0 {
            return self.affine_into(dt, input, cosine_coeff, output);
        }
        let (sin, cos) = phase.sin_cos();
        for (k, &g) in self.indices.iter().enumerate() {
            let block = self.groups[g];
            let plus = block.frequency - frequency;
            let minus = block.frequency + frequency;
            if !plus.is_finite() || !minus.is_finite() {
                return Err(CoreError::NonFinite(
                    "harmonic shifted frequency overflow".into(),
                ));
            }
            let fp = AffineFactor::new(
                DampedRotationBlock {
                    damping: block.damping,
                    frequency: plus,
                },
                dt,
            )?;
            let fm = AffineFactor::new(
                DampedRotationBlock {
                    damping: block.damping,
                    frequency: minus,
                },
                dt,
            )?;
            let (cx, cy, dx, dy) = (
                cosine_coeff[2 * k],
                cosine_coeff[2 * k + 1],
                sine_coeff[2 * k],
                sine_coeff[2 * k + 1],
            );
            // Halving must occur in exponent-separated form: a subnormal
            // coefficient can have a representable integral after large dt.
            let (px, py) = fp.forcing_scaled(
                Scaled::new(cx).add(Scaled::new(dy)).mul(0.5),
                Scaled::new(cy).add(Scaled::new(-dx)).mul(0.5),
            );
            let (mx, my) = fm.forcing_scaled(
                Scaled::new(cx).add(Scaled::new(-dy)).mul(0.5),
                Scaled::new(cy).add(Scaled::new(dx)).mul(0.5),
            );
            let homogeneous = AffineFactor::new(block, dt)?;
            let (hx, hy) = homogeneous.action_scaled(input[2 * k], input[2 * k + 1], 0., 0.);
            output[2 * k] = hx
                .add(px.mul(cos))
                .add(py.mul(-sin))
                .add(mx.mul(cos))
                .add(my.mul(sin))
                .value();
            output[2 * k + 1] = hy
                .add(px.mul(sin))
                .add(py.mul(cos))
                .add(mx.mul(-sin))
                .add(my.mul(cos))
                .value();
        }
        validate_affine_output(output)
    }
    /// Exact-algebra action of `(I-shift*A)^-1`. For shift>=0,d>=0 the
    /// determinant is `(1+shift*d)^2+(shift*w)^2 >= 1` in exact arithmetic.
    /// Scaling avoids squaring large coefficients and overflowing shift*A.
    pub fn shifted_inverse_into(
        &self,
        shift: f64,
        input: &[f64],
        output: &mut [f64],
    ) -> CoreResult<()> {
        self.validate_vectors(input, output)?;
        if !shift.is_finite() || shift < 0.0 {
            return Err(CoreError::InvalidInput(
                "quotient inverse requires finite shift >= 0".into(),
            ));
        }
        if shift == 0.0 {
            output.copy_from_slice(input);
            return Ok(());
        }
        for (k, &g) in self.indices.iter().enumerate() {
            let b = self.groups[g];
            let x = input[2 * k];
            let y = input[2 * k + 1];
            let a = 1.0 + shift * b.damping;
            let v = shift * b.frequency;
            let (ar, br, xr, yr) = if a.is_finite() && v.is_finite() {
                let m = a.abs().max(v.abs());
                (a / m, v / m, x / m, y / m)
            } else {
                let m = b.damping.max(b.frequency.abs());
                (
                    (1.0 / shift) / m + b.damping / m,
                    b.frequency / m,
                    (x / shift) / m,
                    (y / shift) / m,
                )
            };
            let denominator = ar * ar + br * br;
            let ca = ar / denominator;
            let cb = br / denominator;
            output[2 * k] = ca * xr - cb * yr;
            output[2 * k + 1] = cb * xr + ca * yr;
        }
        if output.iter().any(|x| !x.is_finite()) {
            return Err(CoreError::NonFinite(
                "quotient inverse action overflow".into(),
            ));
        }
        Ok(())
    }
}

// A finite mantissa plus an unrestricted binary exponent. Keeping the scale
// separate prevents avoidable overflow/underflow before cancellation or a
// subsequent multiplication by a very large/small forcing component.
#[derive(Clone, Copy, Debug, Default)]
struct Scaled {
    mantissa: f64,
    exponent: i32,
}
impl Scaled {
    fn new(x: f64) -> Self {
        if x == 0.0 {
            return Self::default();
        }
        let bits = x.to_bits();
        let biased = ((bits >> 52) & 0x7ff) as i32;
        if biased == 0 {
            let mut value = Self::new(x * 18_014_398_509_481_984.0); // 2^54
            value.exponent -= 54;
            return value;
        }
        Self {
            mantissa: f64::from_bits((bits & 0x800f_ffff_ffff_ffff) | (1023u64 << 52)),
            exponent: biased - 1023,
        }
    }
    fn mul(self, x: f64) -> Self {
        let rhs = Self::new(x);
        let mut value = Self::new(self.mantissa * rhs.mantissa);
        value.exponent += self.exponent + rhs.exponent;
        value
    }
    fn div(self, x: f64) -> Self {
        let rhs = Self::new(x);
        let mut value = Self::new(self.mantissa / rhs.mantissa);
        value.exponent += self.exponent - rhs.exponent;
        value
    }
    fn add(self, rhs: Self) -> Self {
        if self.mantissa == 0.0 {
            return rhs;
        }
        if rhs.mantissa == 0.0 {
            return self;
        }
        let (large, small) = if self.exponent >= rhs.exponent {
            (self, rhs)
        } else {
            (rhs, self)
        };
        let mut value =
            Self::new(large.mantissa + scale_pow2(small.mantissa, small.exponent - large.exponent));
        value.exponent += large.exponent;
        value
    }
    fn value(self) -> f64 {
        scale_pow2(self.mantissa, self.exponent)
    }
}
fn scale_pow2(mut x: f64, mut exponent: i32) -> f64 {
    if x == 0.0 {
        return x;
    }
    while exponent > 512 {
        x *= 1.3407807929942597e154; // 2^512
        exponent -= 512;
    }
    while exponent < -512 {
        x *= 7.458340731200207e-155; // 2^-512
        exponent += 512;
    }
    x * 2.0_f64.powi(exponent)
}
fn complex_scaled(c: f64, s: f64, x: f64, y: f64) -> (Scaled, Scaled) {
    (
        Scaled::new(x).mul(c).add(Scaled::new(y).mul(-s)),
        Scaled::new(x).mul(s).add(Scaled::new(y).mul(c)),
    )
}
#[derive(Clone, Copy, Debug, Default)]
struct AffineFactor {
    homogeneous: Factor,
    c: f64,
    s: f64,
    scale: f64,
    divide: bool,
}
impl AffineFactor {
    fn new(block: DampedRotationBlock, h: f64) -> CoreResult<Self> {
        let homogeneous = Factor::new(block, h)?;
        let x = -block.damping * h;
        let y = block.frequency * h;
        if x.abs().max(y.abs()) <= 0.25 {
            // phi_1(z)=sum z^k/(k+1)!, including z=0. With |z|<=sqrt(2)/4,
            // 18 terms leave an absolute series tail below 2e-27.
            let (mut c, mut s, mut tr, mut ti) = (1.0, 0.0, 1.0, 0.0);
            for k in 1..=18 {
                let denominator = (k + 1) as f64;
                (tr, ti) = (
                    (tr * x - ti * y) / denominator,
                    (tr * y + ti * x) / denominator,
                );
                c += tr;
                s += ti;
            }
            return Ok(Self {
                homogeneous,
                c,
                s,
                scale: h,
                divide: false,
            });
        }
        let (u, v) = if x < -1500.0 {
            (-1.0, 0.0)
        } else {
            // Factor::new already checked a represented finite phase. Using
            // sin(theta/2)^2 retains exp(z)-1 near an exact rotation period.
            let (sin, cos) = y.sin_cos();
            (
                x.exp_m1() * cos - 2.0 * (0.5 * y).sin().powi(2),
                x.exp() * sin,
            )
        };
        let scale = block.damping.max(block.frequency.abs());
        let ar = -block.damping / scale;
        let ai = block.frequency / scale;
        let denominator = ar * ar + ai * ai;
        Ok(Self {
            homogeneous,
            c: (u * ar + v * ai) / denominator,
            s: (v * ar - u * ai) / denominator,
            scale,
            divide: true,
        })
    }
    fn action(self, x: f64, y: f64, bx: f64, by: f64) -> (f64, f64) {
        let (x, y) = self.action_scaled(x, y, bx, by);
        (x.value(), y.value())
    }
    fn action_scaled(self, x: f64, y: f64, bx: f64, by: f64) -> (Scaled, Scaled) {
        let (hx, hy) = complex_scaled(self.homogeneous.c, self.homogeneous.s, x, y);
        let hx = hx
            .mul(self.homogeneous.scale_half)
            .mul(self.homogeneous.scale_half);
        let hy = hy
            .mul(self.homogeneous.scale_half)
            .mul(self.homogeneous.scale_half);
        let (fx, fy) = self.forcing_scaled(Scaled::new(bx), Scaled::new(by));
        (hx.add(fx), hy.add(fy))
    }
    fn forcing_scaled(self, bx: Scaled, by: Scaled) -> (Scaled, Scaled) {
        let fx = bx.mul(self.c).add(by.mul(-self.s));
        let fy = bx.mul(self.s).add(by.mul(self.c));
        if self.divide {
            (fx.div(self.scale), fy.div(self.scale))
        } else {
            (fx.mul(self.scale), fy.mul(self.scale))
        }
    }
}
fn validate_affine_dt(dt: f64) -> CoreResult<()> {
    if !dt.is_finite() || dt < 0.0 {
        Err(CoreError::InvalidInput(
            "affine flow requires finite dt >= 0".into(),
        ))
    } else {
        Ok(())
    }
}
fn validate_affine_output(output: &[f64]) -> CoreResult<()> {
    if output.iter().any(|x| !x.is_finite()) {
        Err(CoreError::NonFinite(
            "declared affine flow action overflow".into(),
        ))
    } else {
        Ok(())
    }
}

/// Cached exact constant-forcing actions for a declared damped-rotation flow.
/// Construction allocates O(n+g) storage. Calls allocate no heap memory and
/// cost O(n+g), or O(n) when dt is unchanged; g is the distinct block count.
/// Different forcing vectors can reuse the same coefficient cache. There is
/// no inferred Jacobian, RHS callback, linear solve or nonlinear iteration.
#[derive(Clone, Debug)]
pub struct DampedRotationAffineWorkspace {
    flow: DampedRotationFlow,
    factors: Vec<AffineFactor>,
    cached_dt: Option<f64>,
    work: QuotientWork,
}
impl DampedRotationAffineWorkspace {
    pub fn new(flow: DampedRotationFlow) -> CoreResult<Self> {
        let factors = vec![AffineFactor::default(); flow.distinct_blocks()];
        let work = QuotientWork {
            workspace_bytes: factors.len() * std::mem::size_of::<AffineFactor>()
                + flow.indices.len() * std::mem::size_of::<usize>()
                + flow.groups.len() * std::mem::size_of::<DampedRotationBlock>(),
            distinct_blocks: flow.distinct_blocks(),
            ..Default::default()
        };
        Ok(Self {
            flow,
            factors,
            cached_dt: None,
            work,
        })
    }
    pub fn work(&self) -> &QuotientWork {
        &self.work
    }
    pub fn apply_into(
        &mut self,
        dt: f64,
        input: &[f64],
        constant_forcing: &[f64],
        output: &mut [f64],
    ) -> CoreResult<()> {
        self.flow.validate_vectors(input, output)?;
        self.flow.validate_vectors(constant_forcing, output)?;
        validate_affine_dt(dt)?;
        if self.cached_dt != Some(dt) {
            // A failed refresh must not leave an apparently valid old cache.
            self.cached_dt = None;
            for (factor, block) in self.factors.iter_mut().zip(&self.flow.groups) {
                *factor = AffineFactor::new(*block, dt)?;
                self.work.coefficient_pairs = self.work.coefficient_pairs.saturating_add(1);
            }
            self.cached_dt = Some(dt);
            self.work.coefficient_refreshes = self.work.coefficient_refreshes.saturating_add(1);
        }
        for (k, &group) in self.flow.indices.iter().enumerate() {
            let (x, y) = self.factors[group].action(
                input[2 * k],
                input[2 * k + 1],
                constant_forcing[2 * k],
                constant_forcing[2 * k + 1],
            );
            output[2 * k] = x;
            output[2 * k + 1] = y;
            self.work.applied_block_actions = self.work.applied_block_actions.saturating_add(1);
        }
        validate_affine_output(output)
    }
}

/// Additional algebraic work. RHS counts live in integration counters.
#[derive(Clone, Debug, Default, Serialize)]
pub struct QuotientWork {
    pub coefficient_refreshes: u64,
    pub coefficient_pairs: u64,
    pub applied_block_actions: u64,
    /// Principal workspace vectors/cache, excludes outputs and allocator metadata.
    pub workspace_bytes: usize,
    pub distinct_blocks: usize,
}

/// Reusable trial storage. Successful trials allocate no heap memory.
#[derive(Debug)]
pub struct LawsonDopri5Workspace {
    flow: DampedRotationFlow,
    stages: Vec<f64>,
    stage_state: Vec<f64>,
    factors: Vec<[Factor; 28]>,
    affine_forcing: Option<Vec<f64>>,
    affine_factors: Vec<[AffineFactor; 7]>,
    first_valid: bool,
}
#[inline]
fn slot(i: usize, j: usize) -> usize {
    i * (i + 1) / 2 + j
}
struct Failure {
    error: CoreError,
    reason: Dopri5StopReason,
    retryable: bool,
}
impl Failure {
    fn numerical(error: CoreError) -> Self {
        Self {
            error,
            reason: Dopri5StopReason::NonFinite,
            retryable: true,
        }
    }
}
impl LawsonDopri5Workspace {
    pub fn new(flow: DampedRotationFlow) -> CoreResult<Self> {
        let n = flow.dimension();
        let len = n
            .checked_mul(7)
            .ok_or_else(|| CoreError::Dimension("Lawson workspace overflow".into()))?;
        let factors = vec![[Factor::default(); 28]; flow.distinct_blocks()];
        Ok(Self {
            flow,
            stages: vec![0.0; len],
            stage_state: vec![0.0; n],
            factors,
            affine_forcing: None,
            affine_factors: Vec::new(),
            first_valid: false,
        })
    }
    /// Lawson integration of `A*y + constant_forcing + residual(t,y)`.
    /// The residual callback must exclude both declared affine terms.
    pub fn new_affine(flow: DampedRotationFlow, constant_forcing: &[f64]) -> CoreResult<Self> {
        flow.validate_vectors(constant_forcing, constant_forcing)?;
        let groups = flow.distinct_blocks();
        let mut workspace = Self::new(flow)?;
        workspace.affine_forcing = Some(constant_forcing.to_vec());
        workspace.affine_factors = vec![[AffineFactor::default(); 7]; groups];
        Ok(workspace)
    }
    pub fn workspace_bytes(&self) -> usize {
        (self.stages.len()
            + self.stage_state.len()
            + self.affine_forcing.as_ref().map_or(0, Vec::len))
            * std::mem::size_of::<f64>()
            + self.affine_factors.len() * 7 * std::mem::size_of::<AffineFactor>()
            + self.factors.len() * 28 * std::mem::size_of::<Factor>()
            + self.flow.indices.len() * std::mem::size_of::<usize>()
            + self.flow.groups.len() * std::mem::size_of::<DampedRotationBlock>()
    }
    /// Uncommitted trial. `residual` is N, not the full RHS A*y+N.
    /// No FSAL data crosses this public call. Outputs are scratch on error.
    #[allow(clippy::too_many_arguments)]
    pub fn step_into<F: Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + ?Sized>(
        &mut self,
        residual: &F,
        t: f64,
        y: &[f64],
        h: f64,
        endpoint: &mut [f64],
        error: &mut [f64],
        counters: &mut WorkCounters,
        work: &mut QuotientWork,
    ) -> CoreResult<()> {
        self.flow.validate_vectors(y, endpoint)?;
        if error.len() != y.len()
            || !t.is_finite()
            || !h.is_finite()
            || h <= 0.0
            || !(t + h).is_finite()
            || t + h <= t
        {
            return Err(CoreError::InvalidInput(
                "Lawson trial needs matching buffers and a represented forward interval".into(),
            ));
        }
        self.first_valid = false;
        let mut completed = 0;
        self.trial(
            residual,
            t,
            y,
            h,
            endpoint,
            error,
            counters,
            work,
            &mut completed,
            false,
        )
        .map_err(|e| e.error)
    }
    #[allow(clippy::too_many_arguments)]
    fn trial<F: Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + ?Sized>(
        &mut self,
        residual: &F,
        t: f64,
        y: &[f64],
        h: f64,
        endpoint: &mut [f64],
        error: &mut [f64],
        counters: &mut WorkCounters,
        work: &mut QuotientWork,
        completed: &mut u64,
        recover: bool,
    ) -> Result<(), Failure> {
        let n = y.len();
        for (g, &b) in self.flow.groups.iter().enumerate() {
            for i in 0..7 {
                for j in 0..=i {
                    self.factors[g][slot(i, j)] =
                        Factor::new_scaled(b, h, DOPRI5_C[i] - DOPRI5_C[j])
                            .map_err(Failure::numerical)?;
                    work.coefficient_pairs = work.coefficient_pairs.saturating_add(1);
                }
            }
        }
        if self.affine_forcing.is_some() {
            for (g, &block) in self.flow.groups.iter().enumerate() {
                for (i, &fraction) in DOPRI5_C.iter().enumerate() {
                    self.affine_factors[g][i] =
                        AffineFactor::new(block, h * fraction).map_err(Failure::numerical)?;
                    work.coefficient_pairs = work.coefficient_pairs.saturating_add(1);
                }
            }
        }
        work.coefficient_refreshes = work.coefficient_refreshes.saturating_add(1);
        if !self.first_valid {
            evaluate(
                residual,
                t,
                y,
                &mut self.stages[..n],
                counters,
                completed,
                recover,
            )?;
            self.first_valid = true;
        }
        for i in 1..7 {
            for (k, &g) in self.flow.indices.iter().enumerate() {
                let (x, z) = if let Some(forcing) = &self.affine_forcing {
                    self.affine_factors[g][i].action(
                        y[2 * k],
                        y[2 * k + 1],
                        forcing[2 * k],
                        forcing[2 * k + 1],
                    )
                } else {
                    self.factors[g][slot(i, 0)].action(y[2 * k], y[2 * k + 1])
                };
                self.stage_state[2 * k] = x;
                self.stage_state[2 * k + 1] = z;
            }
            work.applied_block_actions = work
                .applied_block_actions
                .saturating_add(self.flow.indices.len() as u64);
            for j in 0..i {
                let a = DOPRI5_A[i][j];
                if a != 0.0 {
                    for (k, &g) in self.flow.indices.iter().enumerate() {
                        // Scale by h first, retaining increments on tiny subnormal h.
                        // Any nonfinite intermediate triggers a smaller-step retry.
                        let (x, z) = self.factors[g][slot(i, j)].action(
                            h * self.stages[j * n + 2 * k],
                            h * self.stages[j * n + 2 * k + 1],
                        );
                        self.stage_state[2 * k] += a * x;
                        self.stage_state[2 * k + 1] += a * z;
                    }
                    work.applied_block_actions = work
                        .applied_block_actions
                        .saturating_add(self.flow.indices.len() as u64);
                }
            }
            if self.stage_state.iter().any(|x| !x.is_finite()) {
                return Err(Failure::numerical(CoreError::NonFinite(
                    "Lawson stage overflow".into(),
                )));
            }
            let ti = if i >= 5 { t + h } else { t + DOPRI5_C[i] * h };
            evaluate(
                residual,
                ti,
                &self.stage_state,
                &mut self.stages[i * n..(i + 1) * n],
                counters,
                completed,
                recover,
            )?;
        }
        endpoint.copy_from_slice(&self.stage_state);
        error.fill(0.0);
        for (j, &a) in EW.iter().enumerate() {
            if a != 0.0 {
                for (k, &g) in self.flow.indices.iter().enumerate() {
                    let (x, z) = self.factors[g][slot(6, j)].action(
                        h * self.stages[j * n + 2 * k],
                        h * self.stages[j * n + 2 * k + 1],
                    );
                    error[2 * k] += a * x;
                    error[2 * k + 1] += a * z;
                }
                work.applied_block_actions = work
                    .applied_block_actions
                    .saturating_add(self.flow.indices.len() as u64);
            }
        }
        if error.iter().any(|x| !x.is_finite()) {
            return Err(Failure::numerical(CoreError::NonFinite(
                "Lawson embedded error overflow".into(),
            )));
        }
        Ok(())
    }
}
fn evaluate<F: Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + ?Sized>(
    f: &F,
    t: f64,
    y: &[f64],
    out: &mut [f64],
    counters: &mut WorkCounters,
    completed: &mut u64,
    recover: bool,
) -> Result<(), Failure> {
    out.fill(f64::NAN);
    counters.rhs_calls = counters.rhs_calls.saturating_add(1);
    counters.rhs_evaluations = counters.rhs_evaluations.saturating_add(1);
    if let Err(error) = f(t, y, out) {
        let retryable = recover && matches!(&error, CoreError::NonFinite(_));
        return Err(Failure {
            error,
            reason: Dopri5StopReason::Callback,
            retryable,
        });
    }
    if out.iter().any(|x| !x.is_finite()) {
        return Err(Failure {
            error: CoreError::NonFinite("Lawson residual must write every finite component".into()),
            reason: Dopri5StopReason::NonFinite,
            retryable: false,
        });
    }
    *completed = completed.saturating_add(1);
    Ok(())
}
fn norm(y: &[f64], z: &[f64], e: &[f64], config: &Dopri5Config) -> CoreResult<f64> {
    let mut max = 0.0_f64;
    let mut sum = 0.0_f64;
    for i in 0..y.len() {
        let scale = config.atol.as_ref().map_or(config.adaptive.atol, |a| a[i])
            + config.adaptive.rtol * y[i].abs().max(z[i].abs());
        if !scale.is_finite() {
            return Err(CoreError::InvalidInput(
                "Lawson tolerance scale overflow".into(),
            ));
        }
        let x = scaled_error_component(e[i], scale).abs();
        if !x.is_finite() {
            return Ok(f64::INFINITY);
        }
        if x > max {
            sum = if max == 0.0 {
                1.0
            } else {
                1.0 + sum * (max / x).powi(2)
            };
            max = x;
        } else if max > 0.0 {
            sum += (x / max).powi(2);
        }
    }
    Ok(max * (sum / y.len() as f64).sqrt())
}

#[derive(Clone, Debug)]
pub struct LawsonDopri5Result {
    pub integration: Dopri5IntegrationResult,
    pub quotient_work: QuotientWork,
}
/// Adaptive physical-coordinate Lawson-DOPRI with endpoint observations.
/// No state matrix, nonlinear solve, Jacobian, AD, JVP or finite-difference
/// probe is evaluated. Smooth fixed-A order is inherited from DOPRI5.
/// The nonlinear residual can retain stiffness or fast forcing, which the
/// embedded estimate and caller accuracy study must resolve separately.
pub fn integrate_lawson_dopri5_observed<
    F: Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + ?Sized,
>(
    flow: &DampedRotationFlow,
    residual: &F,
    t_span: (f64, f64),
    y0: &[f64],
    config: &Dopri5Config,
    output: &OutputSchedule,
) -> CoreResult<LawsonDopri5Result> {
    integrate_impl(flow, residual, t_span, y0, config, output, None, None)
}
/// Lawson-DOPRI with an exactly integrated declared constant affine forcing.
/// The callback is the remaining residual in `y'=A*y+b+N(t,y)`. For `N=0`,
/// the endpoint is the exact affine action up to floating-point evaluation.
/// Smooth fixed-A,b classical order is five; no uniform stiff-order claim is
/// made for general nonlinear/time-dependent N. Observations clip endpoints.
#[allow(clippy::too_many_arguments)]
pub fn integrate_affine_lawson_dopri5_observed<
    F: Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + ?Sized,
>(
    flow: &DampedRotationFlow,
    constant_forcing: &[f64],
    residual: &F,
    t_span: (f64, f64),
    y0: &[f64],
    config: &Dopri5Config,
    output: &OutputSchedule,
) -> CoreResult<LawsonDopri5Result> {
    integrate_impl(
        flow,
        residual,
        t_span,
        y0,
        config,
        output,
        None,
        Some(constant_forcing),
    )
}
/// Constant-affine Lawson integration with a supplied physical error norm.
#[allow(clippy::too_many_arguments)]
pub fn integrate_affine_lawson_dopri5_observed_with_norm<
    F: Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + ?Sized,
>(
    flow: &DampedRotationFlow,
    constant_forcing: &[f64],
    residual: &F,
    t_span: (f64, f64),
    y0: &[f64],
    config: &Dopri5Config,
    output: &OutputSchedule,
    physical_norm: &AffineErrorNorm,
) -> CoreResult<LawsonDopri5Result> {
    integrate_impl(
        flow,
        residual,
        t_span,
        y0,
        config,
        output,
        Some(physical_norm),
        Some(constant_forcing),
    )
}
/// Same method with the existing physical-space affine error norm.
pub fn integrate_lawson_dopri5_observed_with_norm<
    F: Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + ?Sized,
>(
    flow: &DampedRotationFlow,
    residual: &F,
    t_span: (f64, f64),
    y0: &[f64],
    config: &Dopri5Config,
    output: &OutputSchedule,
    physical_norm: &AffineErrorNorm,
) -> CoreResult<LawsonDopri5Result> {
    integrate_impl(
        flow,
        residual,
        t_span,
        y0,
        config,
        output,
        Some(physical_norm),
        None,
    )
}
fn integrate_impl<F: Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + ?Sized>(
    flow: &DampedRotationFlow,
    residual: &F,
    t_span: (f64, f64),
    y0: &[f64],
    config: &Dopri5Config,
    output: &OutputSchedule,
    physical_norm: Option<&AffineErrorNorm>,
    constant_forcing: Option<&[f64]>,
) -> CoreResult<LawsonDopri5Result> {
    flow.validate_vectors(y0, y0)?;
    let mut adaptive = config.adaptive.clone();
    if let Some(a) = &config.atol {
        if a.len() != y0.len() || a.iter().any(|x| !x.is_finite() || *x < 0.0) {
            return Err(CoreError::InvalidInput(
                "Lawson absolute tolerances invalid".into(),
            ));
        }
        adaptive.atol = a.iter().copied().fold(0.0, f64::max);
    }
    adaptive.validate()?;
    let mut output = Dopri5OutputSession::new(&adaptive, output, t_span, y0, false)?;
    let mut workspace = if let Some(forcing) = constant_forcing {
        LawsonDopri5Workspace::new_affine(flow.clone(), forcing)?
    } else {
        LawsonDopri5Workspace::new(flow.clone())?
    };
    let mut work = QuotientWork {
        workspace_bytes: workspace.workspace_bytes(),
        distinct_blocks: flow.distinct_blocks(),
        ..Default::default()
    };
    let mut state = y0.to_vec();
    let mut endpoint = vec![0.0; y0.len()];
    let mut error = vec![0.0; y0.len()];
    if let Some(p) = physical_norm {
        p.validate_dimension(y0.len())?;
        p.evaluate(y0, y0, &error)?;
    }
    let mut counters = WorkCounters::default();
    let mut controller = AdaptiveControllerState::default();
    let mut t = t_span.0;
    let mut h = adaptive.initial_step;
    let (mut attempts, mut accepted, mut rejected, mut completed, mut reuses) =
        (0usize, 0usize, 0usize, 0u64, 0u64);
    let mut stop = None;
    let mut message = "Lawson-DOPRI5 reached the final time".to_string();
    while t < t_span.1 {
        if attempts == adaptive.max_attempts {
            stop = Some(Dopri5StopReason::AttemptBudget);
            message = "Lawson attempt budget exhausted".into();
            break;
        }
        let trial = match output.trial(t, h, &controller) {
            Ok(Some(v)) => v,
            Ok(None) => {
                stop = Some(Dopri5StopReason::StepUnderflow);
                message = "Lawson step cannot advance".into();
                break;
            }
            Err(e) => {
                stop = Some(Dopri5StopReason::StepUnderflow);
                message = e.to_string();
                break;
            }
        };
        attempts += 1;
        if !config.fsal {
            workspace.first_valid = false;
        }
        if workspace.first_valid {
            reuses = reuses.saturating_add(1);
        }
        let result = workspace.trial(
            residual,
            t,
            &state,
            trial.h,
            &mut endpoint,
            &mut error,
            &mut counters,
            &mut work,
            &mut completed,
            config.recover_nonfinite_rhs,
        );
        let e = match result {
            Ok(()) => match physical_norm.map_or_else(
                || norm(&state, &endpoint, &error, config),
                |p| p.evaluate(&state, &endpoint, &error),
            ) {
                Ok(v) => v,
                Err(e) => {
                    rejected += 1;
                    counters.rejected_steps += 1;
                    stop = Some(Dopri5StopReason::Controller);
                    message = e.to_string();
                    break;
                }
            },
            Err(e) => {
                if !e.retryable {
                    rejected += 1;
                    counters.rejected_steps += 1;
                    stop = Some(e.reason);
                    message = e.error.to_string();
                    break;
                }
                counters.nonfinite_step_failures += 1;
                f64::INFINITY
            }
        };
        let accept = e <= 1.0;
        if accept {
            if let Err(e) = output.accept(
                t,
                &state,
                t + trial.h,
                &endpoint,
                &workspace.stages,
                trial.clipped,
            ) {
                rejected += 1;
                counters.rejected_steps += 1;
                stop = Some(Dopri5StopReason::Output);
                message = e.to_string();
                break;
            }
            t += trial.h;
            state.copy_from_slice(&endpoint);
            accepted += 1;
            counters.accepted_steps += 1;
            if config.fsal {
                workspace.stages.copy_within(6 * y0.len()..7 * y0.len(), 0);
                workspace.first_valid = true;
            }
        } else {
            rejected += 1;
            counters.rejected_steps += 1;
            counters.local_error_failures += 1;
        }
        if t == t_span.1 {
            break;
        }
        match adaptive_next_step_after_attempt_prevalidated(
            &mut controller,
            &adaptive,
            trial.requested_h,
            trial.h,
            e,
            5,
            accept,
            trial.clipped,
        ) {
            Ok(next) => h = next,
            Err(e) => {
                stop = Some(Dopri5StopReason::Controller);
                message = e.to_string();
                break;
            }
        }
    }
    let observed = output.finish(t == t_span.1 && stop.is_none(), message, counters, accepted)?;
    Ok(LawsonDopri5Result {
        integration: Dopri5IntegrationResult {
            observed,
            final_time: t,
            final_state: state,
            attempts,
            accepted_steps: accepted,
            rejected_steps: rejected,
            rhs_completed: completed,
            fsal_reuses: reuses,
            stop_reason: stop,
        },
        quotient_work: work,
    })
}
