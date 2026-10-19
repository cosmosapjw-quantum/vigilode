//! Local T1 error transport for the exact curve frozen in `RepresentedPacketCurve`.
//! This object owns its sole curve; no candidate or external model may be rebound.

use rodas5p_core::directed::{Interval, add_up, div_up, exp_interval, mul_up, sqrt_down, sqrt_up};

use super::{PacketComplexInterval, RepresentedPacketCurve};
use crate::packet_window::{PacketError, PacketWorkCounters};

/// An owner of one frozen represented curve and its T1 output construction.
pub struct PacketCurveOutput {
    curve: RepresentedPacketCurve,
}

/// Owns one represented curve and a copied, immutable initial mode tube.
/// A query can only use the uncertainty supplied at construction.
///
/// ```compile_fail
/// use rodas5p_integrators::packet_window::PacketFrozenOutput;
/// fn alter(owner: &mut PacketFrozenOutput) { owner.initial_radii[0] = 0.0; }
/// ```
pub struct PacketFrozenOutput {
    output: PacketCurveOutput,
    initial_radii: Vec<f64>,
}

/// Immutable enclosure returned only from a frozen local packet owner.
/// This certifies inclusion at the requested time; it does not assert that
/// any requested application tolerance has been met.
///
/// ```compile_fail
/// use rodas5p_integrators::packet_window::PacketCertifiedView;
/// fn alter(view: &mut PacketCertifiedView<'_>) { view.inner = panic!(); }
/// ```
/// ```compile_fail
/// use rodas5p_integrators::packet_window::PacketCertifiedView;
/// fn alter(view: &mut PacketCertifiedView<'_>) { view.weighted_center_displacement = 0.0; }
/// ```
pub struct PacketCertifiedView<'a> {
    inner: PacketCurveOutputView<'a>,
}

impl PacketCertifiedView<'_> {
    pub fn time(&self) -> f64 {
        self.inner.time()
    }
    pub fn modes(&self) -> &[(f64, f64)] {
        self.inner.modes()
    }
    pub fn mode_radii(&self) -> &[f64] {
        self.inner.mode_radii()
    }
    pub fn mean(&self) -> (f64, f64) {
        self.inner.mean()
    }
    pub fn mean_radius(&self) -> f64 {
        self.inner.mean_radius()
    }
    /// Base T1 radius `B_w` around the exact frozen polynomial. It excludes
    /// displacement from evaluating the returned binary64 centers; use the
    /// mode radii for returned-center enclosures. It is not a weighted ball
    /// around `modes()`.
    pub fn weighted_radius(&self) -> f64 {
        self.inner.weighted_radius()
    }
    /// Same-query outward weighted displacement from the exact represented
    /// curve to the returned binary64 mode centers.
    pub fn weighted_center_displacement(&self) -> f64 {
        self.inner.weighted_center_displacement()
    }
}

impl PacketFrozenOutput {
    /// Copy the caller's uncertainty into the owner. The represented curve
    /// already owns its source model, coefficients, horizon, and residual.
    pub fn new(
        curve: RepresentedPacketCurve,
        initial_radii: &[f64],
        work: &mut PacketWorkCounters,
    ) -> Result<Self, PacketError> {
        let n = curve.mode_count();
        if initial_radii.len() != n {
            return Err(PacketError::OutwardRefusal(
                "frozen output initial radius shape mismatch",
            ));
        }
        if initial_radii.iter().any(|&x| !x.is_finite() || x < 0.0) {
            return Err(PacketError::OutwardRefusal("frozen output initial radius"));
        }
        if n > isize::MAX as usize / std::mem::size_of::<f64>() {
            return Err(PacketError::SizeOverflow("frozen output initial radii"));
        }
        work.curve_frozen_allocation_requests += 1;
        let mut frozen = Vec::new();
        frozen
            .try_reserve_exact(n)
            .map_err(|_| PacketError::SizeOverflow("frozen output initial radii"))?;
        for &radius in initial_radii {
            work.curve_frozen_radius_items += 1;
            frozen.push(radius);
        }
        Ok(Self {
            output: PacketCurveOutput::new(curve),
            initial_radii: frozen,
        })
    }

    pub fn workspace(
        &self,
        work: &mut PacketWorkCounters,
    ) -> Result<PacketCurveOutputWorkspace, PacketError> {
        self.output.workspace(work)
    }

    /// Evaluate the frozen curve and initial tube at one physical time.
    pub fn enclose_at<'a>(
        &'a self,
        time: f64,
        workspace: &'a mut PacketCurveOutputWorkspace,
        work: &mut PacketWorkCounters,
    ) -> Result<PacketCertifiedView<'a>, PacketError> {
        work.curve_frozen_query_attempts += 1;
        let result = self
            .output
            .enclose_at(time, &self.initial_radii, workspace, work);
        if result.is_err() {
            work.curve_frozen_query_refusals += 1;
        }
        result.map(|inner| PacketCertifiedView { inner })
    }
}

/// Caller-owned storage. Allocate once before evaluating requested times.
pub struct PacketCurveOutputWorkspace {
    centers: Vec<(f64, f64)>,
    boxes: Vec<PacketComplexInterval>,
    radii: Vec<f64>,
    mean_center: (f64, f64),
    mean_radius: f64,
    weighted_radius: f64,
    weighted_center_displacement: f64,
    time: f64,
}

/// Borrowed output, valid only while its owner and completed workspace live.
/// Its fields are private; callers receive immutable borrowed slices.
///
/// ```compile_fail
/// use rodas5p_integrators::packet_window::PacketCurveOutputView;
/// fn alter(view: &mut PacketCurveOutputView<'_>) { view.mean_radius = 0.0; }
/// ```
/// ```compile_fail
/// use rodas5p_integrators::packet_window::PacketCurveOutputView;
/// fn replace(view: &mut PacketCurveOutputView<'_>) { view.modes = &[]; }
/// ```
/// ```compile_fail
/// use rodas5p_integrators::packet_window::PacketCurveOutputView;
/// fn alter(view: &mut PacketCurveOutputView<'_>) { view.weighted_center_displacement = 0.0; }
/// ```
pub struct PacketCurveOutputView<'a> {
    time: f64,
    modes: &'a [(f64, f64)],
    mode_radii: &'a [f64],
    mean: (f64, f64),
    mean_radius: f64,
    weighted_radius: f64,
    weighted_center_displacement: f64,
    _curve: &'a PacketCurveOutput,
}

impl PacketCurveOutputView<'_> {
    pub fn time(&self) -> f64 {
        self.time
    }
    pub fn modes(&self) -> &[(f64, f64)] {
        self.modes
    }
    pub fn mode_radii(&self) -> &[f64] {
        self.mode_radii
    }
    pub fn mean(&self) -> (f64, f64) {
        self.mean
    }
    pub fn mean_radius(&self) -> f64 {
        self.mean_radius
    }
    /// Base T1 radius `B_w` around the exact frozen polynomial. It excludes
    /// displacement from evaluating the returned binary64 centers; use the
    /// mode radii for returned-center enclosures. It is not a weighted ball
    /// around `modes()`.
    pub fn weighted_radius(&self) -> f64 {
        self.weighted_radius
    }
    /// Same-query outward weighted displacement from the exact represented
    /// curve to the returned binary64 mode centers.
    pub fn weighted_center_displacement(&self) -> f64 {
        self.weighted_center_displacement
    }
}

impl PacketCurveOutput {
    pub fn new(curve: RepresentedPacketCurve) -> Self {
        Self { curve }
    }

    pub fn workspace(
        &self,
        work: &mut PacketWorkCounters,
    ) -> Result<PacketCurveOutputWorkspace, PacketError> {
        let n = self.curve.mode_count();
        Ok(PacketCurveOutputWorkspace {
            centers: reserve(n, (0.0, 0.0), work)?,
            boxes: reserve(
                n,
                PacketComplexInterval {
                    re: Interval::point(0.0)
                        .map_err(|_| PacketError::OutwardRefusal("represented output workspace"))?,
                    im: Interval::point(0.0)
                        .map_err(|_| PacketError::OutwardRefusal("represented output workspace"))?,
                },
                work,
            )?,
            radii: reserve(n, 0.0, work)?,
            mean_center: (0.0, 0.0),
            mean_radius: 0.0,
            weighted_radius: 0.0,
            weighted_center_displacement: 0.0,
            time: 0.0,
        })
    }

    /// Enclose the true kappa=0 flow at an exact requested binary64 physical
    /// time. `initial_radii[j]` is a modulus radius around the curve's frozen
    /// initial amplitude. Every failure is a typed refusal and charges work.
    pub fn enclose_at<'a>(
        &'a self,
        time: f64,
        initial_radii: &[f64],
        workspace: &'a mut PacketCurveOutputWorkspace,
        work: &mut PacketWorkCounters,
    ) -> Result<PacketCurveOutputView<'a>, PacketError> {
        let n = self.curve.mode_count();
        if initial_radii.len() != n
            || workspace.centers.len() != n
            || workspace.boxes.len() != n
            || workspace.radii.len() != n
        {
            return Err(PacketError::OutwardRefusal(
                "represented output shape mismatch",
            ));
        }
        if !time.is_finite() || time < 0.0 || time > self.curve.horizon() {
            return Err(PacketError::OutwardRefusal(
                "represented output physical time",
            ));
        }
        let model = &self.curve.source_identity.model;
        let weights = model.weights();
        let initial = model.initial_amplitudes();
        let mut s = 0.0;
        let mut energy = 0.0;
        let mut initial_error_square = 0.0;
        for j in 0..n {
            let w = weights[j];
            let eta = initial_radii[j];
            if !w.is_finite() || w <= 0.0 || !eta.is_finite() || eta < 0.0 {
                return Err(PacketError::OutwardRefusal(
                    "represented output initial tube",
                ));
            }
            s = add(s, w, work, "represented output weight sum")?;
            let z_abs = magnitude(initial[j].0, initial[j].1, work)?;
            let amplitude = add(z_abs, eta, work, "represented output initial amplitude")?;
            let amplitude_square = mul(amplitude, amplitude, work, "represented output energy")?;
            let weighted_energy = mul(w, amplitude_square, work, "represented output energy")?;
            energy = add(energy, weighted_energy, work, "represented output energy")?;
            let eta_square = mul(eta, eta, work, "represented output initial error")?;
            let weighted_error = mul(w, eta_square, work, "represented output initial error")?;
            initial_error_square = add(
                initial_error_square,
                weighted_error,
                work,
                "represented output initial error",
            )?;
        }
        let e0 = sqrt_upper(
            initial_error_square,
            work,
            "represented output initial error",
        )?;
        let m_y = self.curve.residual_bound.mean_magnitude_sup.hi;
        let rho = self.curve.residual_bound.weighted_residual_sup.hi;
        let se = mul(s, energy, work, "represented output gain")?;
        let my2 = mul(m_y, m_y, work, "represented output gain")?;
        let schi = mul(s, model.chi().abs(), work, "represented output gain")?;
        let lambda = mul(schi, se.max(my2), work, "represented output gain")?;
        let exponent = mul(lambda, time, work, "represented output gain")?;
        work.curve_output_certificate_ops += 1; // one exp_interval kernel
        let gain = exp_interval(exponent)
            .map_err(|_| PacketError::OutwardRefusal("represented output exponential gain"))?
            .hi;
        let residual_increment = mul(time, rho, work, "represented output residual")?;
        let seeded_radius = add(e0, residual_increment, work, "represented output radius")?;
        let weighted_radius = mul(gain, seeded_radius, work, "represented output radius")?;

        // Evaluate the same stored curve in binary64 and exact-real interval
        // arithmetic. The resulting displacement encloses evaluation rounding.
        self.curve
            .evaluate_modes_at_into(time, &mut workspace.centers, work)?;
        self.curve
            .enclose_modes_at_into(time, &mut workspace.boxes, work)?;
        let mut weighted_displacement_square = 0.0;
        for j in 0..n {
            work.curve_output_certificate_ops += 1; // sqrt_down
            let denominator = sqrt_down(weights[j])
                .map_err(|_| PacketError::OutwardRefusal("represented output weight root"))?;
            if denominator <= 0.0 {
                return Err(PacketError::OutwardRefusal(
                    "represented output weight root vanished",
                ));
            }
            let flow = div(
                weighted_radius,
                denominator,
                work,
                "represented output mode radius",
            )?;
            let delta = displacement(workspace.centers[j], workspace.boxes[j], work)?;
            workspace.radii[j] = add(flow, delta, work, "represented output mode radius")?;
            let delta_square = mul(
                delta,
                delta,
                work,
                "represented output weighted displacement",
            )?;
            let weighted_delta = mul(
                weights[j],
                delta_square,
                work,
                "represented output weighted displacement",
            )?;
            weighted_displacement_square = add(
                weighted_displacement_square,
                weighted_delta,
                work,
                "represented output weighted displacement",
            )?;
        }
        let weighted_center_displacement = sqrt_upper(
            weighted_displacement_square,
            work,
            "represented output weighted displacement",
        )?;

        let mut mean = (0.0, 0.0);
        for (j, &(re, im)) in workspace.centers.iter().enumerate() {
            work.curve_output_certificate_ops += 1;
            let real_term = weights[j] * re;
            if !real_term.is_finite() {
                return Err(PacketError::OutwardRefusal(
                    "represented output mean center",
                ));
            }
            work.curve_output_certificate_ops += 1;
            mean.0 += real_term;
            if !mean.0.is_finite() {
                return Err(PacketError::OutwardRefusal(
                    "represented output mean center",
                ));
            }
            work.curve_output_certificate_ops += 1;
            let imaginary_term = weights[j] * im;
            if !imaginary_term.is_finite() {
                return Err(PacketError::OutwardRefusal(
                    "represented output mean center",
                ));
            }
            work.curve_output_certificate_ops += 1;
            mean.1 += imaginary_term;
            if !mean.0.is_finite() || !mean.1.is_finite() {
                return Err(PacketError::OutwardRefusal(
                    "represented output mean center",
                ));
            }
        }
        let mean_box = self.curve.enclose_mean_at(time, work)?;
        let mean_delta = displacement(mean, mean_box, work)?;
        let root_s = sqrt_upper(s, work, "represented output weight root")?;
        let mean_flow = mul(
            root_s,
            weighted_radius,
            work,
            "represented output mean radius",
        )?;
        let mean_radius = add(
            mean_flow,
            mean_delta,
            work,
            "represented output mean radius",
        )?;
        workspace.mean_center = mean;
        workspace.mean_radius = mean_radius;
        workspace.weighted_radius = weighted_radius;
        workspace.weighted_center_displacement = weighted_center_displacement;
        workspace.time = time;
        Ok(PacketCurveOutputView {
            time: workspace.time,
            modes: &workspace.centers,
            mode_radii: &workspace.radii,
            mean: workspace.mean_center,
            mean_radius: workspace.mean_radius,
            weighted_radius: workspace.weighted_radius,
            weighted_center_displacement: workspace.weighted_center_displacement,
            _curve: self,
        })
    }
}

fn reserve<T: Clone>(
    n: usize,
    value: T,
    work: &mut PacketWorkCounters,
) -> Result<Vec<T>, PacketError> {
    if n > isize::MAX as usize / std::mem::size_of::<T>().max(1) {
        return Err(PacketError::SizeOverflow("represented output workspace"));
    }
    work.curve_output_allocation_requests += 1;
    let mut values = Vec::new();
    values
        .try_reserve_exact(n)
        .map_err(|_| PacketError::SizeOverflow("represented output workspace"))?;
    values.resize(n, value);
    Ok(values)
}

// Each wrapper charges one attempted directed operation before a possible
// overflow/refusal. Constructor shape checks and finite-input validation are
// excluded; underlying curve evaluation retains its existing counters.
fn add(
    a: f64,
    b: f64,
    work: &mut PacketWorkCounters,
    reason: &'static str,
) -> Result<f64, PacketError> {
    work.curve_output_certificate_ops += 1;
    add_up(a, b).map_err(|_| PacketError::OutwardRefusal(reason))
}
fn mul(
    a: f64,
    b: f64,
    work: &mut PacketWorkCounters,
    reason: &'static str,
) -> Result<f64, PacketError> {
    work.curve_output_certificate_ops += 1;
    mul_up(a, b).map_err(|_| PacketError::OutwardRefusal(reason))
}
fn div(
    a: f64,
    b: f64,
    work: &mut PacketWorkCounters,
    reason: &'static str,
) -> Result<f64, PacketError> {
    work.curve_output_certificate_ops += 1;
    div_up(a, b).map_err(|_| PacketError::OutwardRefusal(reason))
}
fn sqrt_upper(
    a: f64,
    work: &mut PacketWorkCounters,
    reason: &'static str,
) -> Result<f64, PacketError> {
    work.curve_output_certificate_ops += 1;
    sqrt_up(a).map_err(|_| PacketError::OutwardRefusal(reason))
}

fn magnitude(re: f64, im: f64, work: &mut PacketWorkCounters) -> Result<f64, PacketError> {
    let re2 = mul(re, re, work, "represented output magnitude")?;
    let im2 = mul(im, im, work, "represented output magnitude")?;
    let square = add(re2, im2, work, "represented output magnitude")?;
    sqrt_upper(square, work, "represented output magnitude")
}

fn displacement(
    center: (f64, f64),
    box_value: PacketComplexInterval,
    work: &mut PacketWorkCounters,
) -> Result<f64, PacketError> {
    work.curve_output_certificate_ops += 1;
    let point_re = Interval::point(center.0)
        .map_err(|_| PacketError::OutwardRefusal("represented output center"))?;
    work.curve_output_certificate_ops += 1;
    let point_im = Interval::point(center.1)
        .map_err(|_| PacketError::OutwardRefusal("represented output center"))?;
    work.curve_output_certificate_ops += 1;
    let re = box_value
        .re
        .sub(point_re)
        .map_err(|_| PacketError::OutwardRefusal("represented output displacement"))?;
    work.curve_output_certificate_ops += 1; // Interval::mag
    let re = re.mag();
    work.curve_output_certificate_ops += 1;
    let im = box_value
        .im
        .sub(point_im)
        .map_err(|_| PacketError::OutwardRefusal("represented output displacement"))?;
    work.curve_output_certificate_ops += 1; // Interval::mag
    let im = im.mag();
    let re2 = mul(re, re, work, "represented output displacement")?;
    let im2 = mul(im, im, work, "represented output displacement")?;
    let square = add(re2, im2, work, "represented output displacement")?;
    sqrt_upper(square, work, "represented output displacement")
}
