//! Immutable represented packet curves and continuous interval residuals.
//!
//! The rounded binary64 mode coefficients define the represented curve. The
//! exact weighted mean and continuous ODE residual of that frozen curve are
//! then enclosed with outward intervals. This local object does not change
//! packet certificate authority or certify trajectories across windows.

use rodas5p_core::directed::{Interval, add_up, mul_up, sqrt_up};

use super::CollectiveModel;
use super::{
    CompiledPacket, IComplex, PacketCandidate, PacketError, PacketIdentity, PacketWorkCounters,
    c_add, c_mul, c_scale, checked_capacity,
};

#[path = "packet_curve_output.rs"]
mod output;
pub use output::{
    PacketCertifiedView, PacketCurveOutput, PacketCurveOutputView, PacketCurveOutputWorkspace,
    PacketFrozenOutput,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PacketComplexInterval {
    pub re: Interval,
    pub im: Interval,
}

impl From<IComplex> for PacketComplexInterval {
    fn from(value: IComplex) -> Self {
        Self {
            re: value.re,
            im: value.im,
        }
    }
}

impl From<PacketComplexInterval> for IComplex {
    fn from(value: PacketComplexInterval) -> Self {
        Self {
            re: value.re,
            im: value.im,
        }
    }
}

/// A frozen binary64 polynomial per microscopic mode, with outward interval
/// coefficients for its exact weighted mean and exact continuous-model
/// residual. Coefficients are in normalized time `tau=t/h`, and the residual
/// derivative is in physical time.
///
/// The maximum mode degree is `D=p+3d+1`, where p is the linear kernel order
/// and d is the candidate mean degree. Each mode stores D+1 coefficients;
/// each residual stores 3D+1 coefficients. The arrays are private and the
/// type is immutable after construction.
#[derive(Clone, Debug)]
pub struct RepresentedPacketCurve {
    source_identity: PacketIdentity,
    source_candidate_bits: Vec<(u64, u64)>,
    source_candidate_degree: usize,
    source_candidate_sweeps: usize,
    horizon: f64,
    maximum_degree: usize,
    residual_stride: usize,
    mode_coefficients: Vec<(f64, f64)>,
    mean_coefficients: Vec<PacketComplexInterval>,
    residual_coefficients: Vec<PacketComplexInterval>,
    residual_bound: ContinuousPacketResidualBound,
}

/// Coefficient-sum bounds for the represented continuous curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContinuousPacketResidualBound {
    /// Upper bound on `sup_tau ||R(Y(tau))||_w`.
    pub weighted_residual_sup: Interval,
    /// Upper bound on `sup_tau |M_Y(tau)|`.
    pub mean_magnitude_sup: Interval,
}

impl RepresentedPacketCurve {
    /// Freeze the rounded mode reconstruction and enclose all coefficients of
    /// its exact continuous-model residual.
    pub fn freeze(
        candidate: &PacketCandidate,
        packet: &CompiledPacket,
        work: &mut PacketWorkCounters,
    ) -> Result<Self, PacketError> {
        packet.validate_identity(work)?;
        work.identity_comparison_items += packet.identity.scalar_items();
        if !candidate.identity.matches_identity(&packet.identity) {
            return Err(PacketError::CandidatePacketMismatch);
        }
        if packet.model.kappa() != 0.0 {
            return Err(PacketError::UnsupportedDamping(packet.model.kappa()));
        }
        let candidate_len = candidate
            .degree
            .checked_add(1)
            .ok_or(PacketError::SizeOverflow("represented candidate degree"))?;
        if candidate.mean.len() != candidate_len {
            return Err(PacketError::OutwardRefusal(
                "represented candidate shape mismatch",
            ));
        }
        if candidate.degree < packet.order {
            return Err(PacketError::DegreeLessThanOrder {
                degree: candidate.degree,
                order: packet.order,
            });
        }
        if candidate
            .mean
            .iter()
            .any(|&(re, im)| !re.is_finite() || !im.is_finite())
        {
            return Err(PacketError::NonFinite("represented candidate coefficient"));
        }
        if packet.order > (1_u64 << 53) as usize || candidate.degree > (1_u64 << 53) as usize {
            return Err(PacketError::OutwardRefusal(
                "represented polynomial integer exceeds exact binary64 range",
            ));
        }

        let maximum_degree = checked_maximum_degree(packet.order, candidate.degree)?;
        let forcing_len = candidate
            .degree
            .checked_mul(3)
            .and_then(|x| x.checked_add(1))
            .ok_or(PacketError::SizeOverflow(
                "represented forcing coefficients",
            ))?;
        let mode_stride = maximum_degree
            .checked_add(1)
            .ok_or(PacketError::SizeOverflow("represented mode stride"))?;
        let residual_stride = maximum_degree
            .checked_mul(3)
            .and_then(|x| x.checked_add(1))
            .ok_or(PacketError::SizeOverflow("represented residual stride"))?;
        let n = packet.model.num_modes();
        let mode_items = n
            .checked_mul(mode_stride)
            .ok_or(PacketError::SizeOverflow("represented mode coefficients"))?;
        let residual_items = n
            .checked_mul(residual_stride)
            .ok_or(PacketError::SizeOverflow(
                "represented residual coefficients",
            ))?;
        let beta_cols = forcing_len;
        let beta_rows = packet
            .order
            .checked_add(1)
            .ok_or(PacketError::SizeOverflow("represented beta rows"))?;
        let beta_items = beta_rows
            .checked_mul(beta_cols)
            .ok_or(PacketError::SizeOverflow("represented beta table"))?;

        let forcing = rounded_candidate_force(&candidate.mean, packet.model.chi(), work)?;
        if forcing.len() != forcing_len {
            return Err(PacketError::OutwardRefusal(
                "represented forcing shape mismatch",
            ));
        }

        // Beta(l,k)=B(l+1,k+1), generated once by a row recurrence.
        let mut beta = counted_capacity::<f64>(beta_items, "represented beta table", work)?;
        beta.resize(beta_items, 0.0);
        for l in 0..=packet.order {
            work.reconstruction_terms += 1;
            beta[l * beta_cols] = 1.0 / ((l + 1) as f64);
            if !beta[l * beta_cols].is_finite() {
                return Err(PacketError::OutwardRefusal("represented beta overflow"));
            }
            for k in 1..forcing_len {
                work.reconstruction_terms += 2;
                let denominator = l
                    .checked_add(k)
                    .and_then(|x| x.checked_add(1))
                    .ok_or(PacketError::SizeOverflow("represented beta denominator"))?;
                let ratio = (k as f64) / (denominator as f64);
                beta[l * beta_cols + k] = beta[l * beta_cols + k - 1] * ratio;
                if !ratio.is_finite() || !beta[l * beta_cols + k].is_finite() {
                    return Err(PacketError::OutwardRefusal("represented beta overflow"));
                }
            }
        }

        let mut mode_coefficients =
            counted_capacity::<(f64, f64)>(mode_items, "represented mode coefficients", work)?;
        mode_coefficients.resize(mode_items, (0.0, 0.0));
        let basis_len = packet
            .order
            .checked_add(1)
            .ok_or(PacketError::SizeOverflow("represented basis"))?;
        let mut basis = counted_capacity::<(f64, f64)>(basis_len, "represented basis", work)?;
        basis.resize(basis_len, (0.0, 0.0));
        let mut convolution =
            counted_capacity::<(f64, f64)>(mode_stride, "represented convolution", work)?;
        convolution.resize(mode_stride, (0.0, 0.0));

        for j in 0..n {
            convolution.fill((0.0, 0.0));
            let xi = packet.model.xi()[j];
            // This defines rounded curve coefficients only; the residual
            // below uses the exact outward product width*xi.
            work.reconstruction_terms += 2;
            let omega_h = packet.model.width() * xi * packet.horizon;
            if !omega_h.is_finite() {
                return Err(PacketError::OutwardRefusal(
                    "represented width-frequency-horizon overflow",
                ));
            }
            basis[0] = (1.0, 0.0);
            for l in 1..=packet.order {
                work.reconstruction_terms += 2;
                let factor = omega_h / (l as f64);
                basis[l] = (-factor * basis[l - 1].1, factor * basis[l - 1].0);
                if !factor.is_finite() || !basis[l].0.is_finite() || !basis[l].1.is_finite() {
                    return Err(PacketError::OutwardRefusal("represented basis overflow"));
                }
            }

            let base = j * mode_stride;
            let z = packet.model.initial_amplitudes()[j];
            // Fhat_0 is not part of Y_j's constant coefficient.
            mode_coefficients[base] = z;
            for l in 1..=packet.order {
                work.reconstruction_terms += 1;
                mode_coefficients[base + l] = c_mul(basis[l], z);
                if !mode_coefficients[base + l].0.is_finite()
                    || !mode_coefficients[base + l].1.is_finite()
                {
                    return Err(PacketError::OutwardRefusal(
                        "represented linear coefficient overflow",
                    ));
                }
            }

            // Accumulate the beta convolution first. Apply physical h once
            // outside the sum; forcing coefficient zero first contributes to
            // the coefficient of tau^1.
            for l in 0..=packet.order {
                for (k, &force_k) in forcing.iter().enumerate() {
                    let exponent = l
                        .checked_add(k)
                        .and_then(|x| x.checked_add(1))
                        .ok_or(PacketError::SizeOverflow("represented convolution index"))?;
                    if exponent > maximum_degree {
                        return Err(PacketError::OutwardRefusal(
                            "represented convolution exceeds checked degree",
                        ));
                    }
                    work.volterra_terms += 1;
                    let term = c_scale(c_mul(basis[l], force_k), beta[l * beta_cols + k]);
                    convolution[exponent] = c_add(convolution[exponent], term);
                    if !convolution[exponent].0.is_finite() || !convolution[exponent].1.is_finite()
                    {
                        return Err(PacketError::OutwardRefusal(
                            "represented convolution coefficient overflow",
                        ));
                    }
                }
            }
            for m in 1..=maximum_degree {
                work.reconstruction_terms += 2;
                let linear = if m <= packet.order {
                    mode_coefficients[base + m]
                } else {
                    (0.0, 0.0)
                };
                let forced = c_scale(convolution[m], packet.horizon);
                mode_coefficients[base + m] = c_add(linear, forced);
                if !mode_coefficients[base + m].0.is_finite()
                    || !mode_coefficients[base + m].1.is_finite()
                {
                    return Err(PacketError::OutwardRefusal(
                        "represented mode coefficient overflow",
                    ));
                }
            }
        }

        let mut mean_coefficients = counted_capacity::<PacketComplexInterval>(
            mode_stride,
            "represented mean coefficients",
            work,
        )?;
        mean_coefficients.resize(mode_stride, IComplex::point((0.0, 0.0))?.into());
        for k in 0..mode_stride {
            let mut mean = IComplex::point((0.0, 0.0))?;
            for j in 0..n {
                work.certificate_items += 1;
                let weight = Interval::point(packet.model.weights()[j])
                    .map_err(|_| PacketError::OutwardRefusal("represented mean weight"))?;
                let mode = IComplex::point(mode_coefficients[j * mode_stride + k])?;
                mean =
                    interval_complex_add(mean, interval_complex_scale(mode, weight, work)?, work)?;
            }
            mean_coefficients[k] = mean.into();
        }

        let mut mean_i =
            counted_capacity::<IComplex>(mode_stride, "represented mean workspace", work)?;
        mean_i.extend(mean_coefficients.iter().copied().map(IComplex::from));
        let mean_conjugate = conjugate_coefficients(&mean_i, work)?;
        let mean_square = interval_polynomial_product(&mean_i, &mean_conjugate, work)?;
        let mean_square_mean = interval_polynomial_product(&mean_square, &mean_i, work)?;
        let nonlinear =
            rotate_and_scale_i_chi(&mean_square_mean, packet.model.chi(), residual_stride, work)?;

        let mut residual_coefficients = counted_capacity::<PacketComplexInterval>(
            residual_items,
            "represented residual coefficients",
            work,
        )?;
        residual_coefficients.resize(residual_items, IComplex::point((0.0, 0.0))?.into());
        // One outward reciprocal operation; charge before the fallible call.
        work.curve_setup_interval_ops += 1;
        let inv_h = Interval::point(1.0)
            .and_then(|one| one.div(Interval::point(packet.horizon)?))
            .map_err(|_| PacketError::OutwardRefusal("represented physical reciprocal"))?;
        for j in 0..n {
            // One outward frequency product per mode; charge before it can refuse.
            work.curve_setup_interval_ops += 1;
            let omega = Interval::point(packet.model.width())
                .and_then(|width| width.mul(Interval::point(packet.model.xi()[j])?))
                .map_err(|_| PacketError::OutwardRefusal("represented width-frequency"))?;
            let base_y = j * mode_stride;
            let base_r = j * residual_stride;
            for q in 0..residual_stride {
                let derivative = if q < maximum_degree {
                    work.certificate_items += 1;
                    let scale = Interval::point((q + 1) as f64)
                        .and_then(|integer| integer.mul(inv_h))
                        .map_err(|_| PacketError::OutwardRefusal("represented derivative scale"))?;
                    interval_complex_scale(
                        IComplex::point(mode_coefficients[base_y + q + 1])?,
                        scale,
                        work,
                    )?
                } else {
                    IComplex::point((0.0, 0.0))?
                };
                let linear = if q <= maximum_degree {
                    interval_i_omega_y(
                        IComplex::point(mode_coefficients[base_y + q])?,
                        omega,
                        work,
                    )?
                } else {
                    IComplex::point((0.0, 0.0))?
                };
                let rhs = interval_complex_add(linear, nonlinear[q], work)?;
                let residual = interval_complex_sub(derivative, rhs, work)?;
                residual_coefficients[base_r + q] = residual.into();
            }
        }

        let residual_bound = coefficient_sum_bound(
            &mean_coefficients,
            &residual_coefficients,
            n,
            residual_stride,
            packet.model.weights(),
            work,
        )?;
        let mut source_candidate_bits = counted_capacity::<(u64, u64)>(
            candidate.mean.len(),
            "represented source candidate identity",
            work,
        )?;
        for &(re, im) in &candidate.mean {
            work.identity_snapshot_items += 2;
            source_candidate_bits.push((re.to_bits(), im.to_bits()));
        }
        let source_identity = copy_identity(&packet.identity, work)?;
        Ok(Self {
            source_identity,
            source_candidate_bits,
            source_candidate_degree: candidate.degree,
            source_candidate_sweeps: candidate.sweeps,
            horizon: packet.horizon,
            maximum_degree,
            residual_stride,
            mode_coefficients,
            mean_coefficients,
            residual_coefficients,
            residual_bound,
        })
    }

    pub const fn maximum_degree(&self) -> usize {
        self.maximum_degree
    }
    pub const fn horizon(&self) -> f64 {
        self.horizon
    }
    pub fn mode_count(&self) -> usize {
        self.source_identity.model.num_modes()
    }

    pub fn mode_coefficients(&self, mode: usize) -> Option<&[(f64, f64)]> {
        if mode >= self.mode_count() {
            return None;
        }
        let stride = self.maximum_degree + 1;
        Some(&self.mode_coefficients[mode * stride..(mode + 1) * stride])
    }

    pub fn mean_coefficients(&self) -> &[PacketComplexInterval] {
        &self.mean_coefficients
    }
    pub fn residual_stride(&self) -> usize {
        self.residual_stride
    }

    pub const fn residual_bound(&self) -> ContinuousPacketResidualBound {
        self.residual_bound
    }

    pub fn residual_coefficients(&self, mode: usize) -> Option<&[PacketComplexInterval]> {
        if mode >= self.mode_count() {
            return None;
        }
        Some(
            &self.residual_coefficients
                [mode * self.residual_stride..(mode + 1) * self.residual_stride],
        )
    }

    /// Bind this curve to the exact source packet and candidate payload.
    pub fn validate_source(
        &self,
        candidate: &PacketCandidate,
        packet: &CompiledPacket,
        work: &mut PacketWorkCounters,
    ) -> Result<(), PacketError> {
        packet.validate_identity(work)?;
        work.identity_comparison_items += packet.identity.scalar_items();
        let model_payload_items = 4 * packet.model.num_modes() as u64 + 5;
        work.curve_model_revalidation_items += model_payload_items;
        let source_matches_packet = self.source_identity.matches(packet);
        work.curve_model_revalidation_items += model_payload_items;
        let source_matches_descriptor = self.source_identity.matches_identity(&packet.identity);
        work.curve_model_revalidation_items += model_payload_items;
        let candidate_matches_source = candidate.identity.matches_identity(&self.source_identity);
        if !source_matches_packet
            || !source_matches_descriptor
            || !candidate_matches_source
            || candidate.degree != self.source_candidate_degree
            || candidate.sweeps != self.source_candidate_sweeps
            || candidate.mean.len() != self.source_candidate_bits.len()
        {
            return Err(PacketError::CandidatePacketMismatch);
        }
        work.identity_comparison_items += candidate.mean.len() as u64 * 2;
        if candidate
            .mean
            .iter()
            .zip(&self.source_candidate_bits)
            .any(|(&(re, im), &(er, ei))| re.to_bits() != er || im.to_bits() != ei)
        {
            return Err(PacketError::CandidatePacketMismatch);
        }
        Ok(())
    }

    /// Evaluate frozen binary64 mode polynomials into caller storage. This
    /// operation performs no allocation.
    pub fn evaluate_modes_at_into(
        &self,
        time: f64,
        output: &mut [(f64, f64)],
        work: &mut PacketWorkCounters,
    ) -> Result<(), PacketError> {
        self.validate_time(time)?;
        if output.len() != self.mode_count() {
            return Err(PacketError::OutwardRefusal(
                "represented mode output shape mismatch",
            ));
        }
        work.curve_time_normalization_ops += 1;
        let tau = time / self.horizon;
        if !tau.is_finite() {
            return Err(PacketError::OutwardRefusal(
                "represented time normalization",
            ));
        }
        let stride = self.maximum_degree + 1;
        for (mode, result) in output.iter_mut().enumerate() {
            let base = mode * stride;
            let mut value = (0.0, 0.0);
            for q in (0..stride).rev() {
                work.output_horner_terms += 1;
                value = c_add(c_scale(value, tau), self.mode_coefficients[base + q]);
                if !value.0.is_finite() || !value.1.is_finite() {
                    return Err(PacketError::OutwardRefusal(
                        "represented mode evaluation overflow",
                    ));
                }
            }
            *result = value;
            work.output_value_items += 2;
        }
        Ok(())
    }

    /// Outwardly evaluate every frozen mode polynomial at physical time.
    pub fn enclose_modes_at_into(
        &self,
        time: f64,
        output: &mut [PacketComplexInterval],
        work: &mut PacketWorkCounters,
    ) -> Result<(), PacketError> {
        self.validate_time(time)?;
        if output.len() != self.mode_count() {
            return Err(PacketError::OutwardRefusal(
                "represented interval output shape mismatch",
            ));
        }
        work.curve_time_normalization_ops += 1;
        let tau = Interval::point(time)
            .and_then(|t| t.div(Interval::point(self.horizon)?))
            .map_err(|_| PacketError::OutwardRefusal("represented interval time normalization"))?;
        let stride = self.maximum_degree + 1;
        for (mode, result) in output.iter_mut().enumerate() {
            let base = mode * stride;
            let mut value = IComplex::point((0.0, 0.0))?;
            for q in (0..stride).rev() {
                work.certificate_items += 1;
                let scaled = interval_complex_scale(value, tau, work)?;
                value = interval_complex_add(
                    scaled,
                    IComplex::point(self.mode_coefficients[base + q])?,
                    work,
                )?;
            }
            *result = value.into();
        }
        Ok(())
    }

    /// Outwardly evaluate the exact weighted mean polynomial of the frozen
    /// mode coefficients; the candidate mean is not substituted.
    pub fn enclose_mean_at(
        &self,
        time: f64,
        work: &mut PacketWorkCounters,
    ) -> Result<PacketComplexInterval, PacketError> {
        self.validate_time(time)?;
        work.curve_time_normalization_ops += 1;
        let tau = Interval::point(time)
            .and_then(|t| t.div(Interval::point(self.horizon)?))
            .map_err(|_| PacketError::OutwardRefusal("represented mean time normalization"))?;
        let mut value = IComplex::point((0.0, 0.0))?;
        for coefficient in self.mean_coefficients.iter().rev() {
            work.certificate_items += 1;
            let scaled = interval_complex_scale(value, tau, work)?;
            value = interval_complex_add(scaled, (*coefficient).into(), work)?;
        }
        Ok(value.into())
    }

    fn validate_time(&self, time: f64) -> Result<(), PacketError> {
        if !time.is_finite() || time < 0.0 || time > self.horizon {
            return Err(PacketError::OutwardRefusal(
                "represented physical time outside curve horizon",
            ));
        }
        Ok(())
    }
}

fn checked_maximum_degree(order: usize, candidate_degree: usize) -> Result<usize, PacketError> {
    let degree = candidate_degree
        .checked_mul(3)
        .and_then(|triple| order.checked_add(triple))
        .and_then(|sum| sum.checked_add(1))
        .ok_or(PacketError::SizeOverflow("represented maximum degree"))?;
    let degree_u64 = u64::try_from(degree).map_err(|_| {
        PacketError::OutwardRefusal(
            "represented maximum degree exceeds exact binary64 integer range",
        )
    })?;
    if degree_u64 > (1_u64 << 53) {
        return Err(PacketError::OutwardRefusal(
            "represented maximum degree exceeds exact binary64 integer range",
        ));
    }
    Ok(degree)
}

#[cfg(test)]
mod degree_tests {
    use super::checked_maximum_degree;
    use crate::packet_window::PacketError;

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn maximum_degree_guard_checks_binary64_integer_domain_without_allocation() {
        let exact_limit = (1_u64 << 53) as usize;
        assert_eq!(checked_maximum_degree(exact_limit - 1, 0), Ok(exact_limit));
        assert!(matches!(
            checked_maximum_degree(exact_limit, 0),
            Err(PacketError::OutwardRefusal(
                "represented maximum degree exceeds exact binary64 integer range"
            ))
        ));
        assert!(matches!(
            checked_maximum_degree(usize::MAX, 1),
            Err(PacketError::SizeOverflow("represented maximum degree"))
        ));
    }
}

fn rounded_candidate_force(
    candidate: &[(f64, f64)],
    chi: f64,
    work: &mut PacketWorkCounters,
) -> Result<Vec<(f64, f64)>, PacketError> {
    let degree = candidate
        .len()
        .checked_sub(1)
        .ok_or(PacketError::OutwardRefusal("empty represented candidate"))?;
    let square_len = degree
        .checked_mul(2)
        .and_then(|x| x.checked_add(1))
        .ok_or(PacketError::SizeOverflow("represented force square"))?;
    let cube_len = degree
        .checked_mul(3)
        .and_then(|x| x.checked_add(1))
        .ok_or(PacketError::SizeOverflow("represented force cube"))?;
    let mut conjugate =
        counted_capacity::<(f64, f64)>(candidate.len(), "represented candidate conjugate", work)?;
    for &(re, im) in candidate {
        work.reconstruction_terms += 1;
        conjugate.push((re, -im));
    }
    let square = rounded_product(candidate, &conjugate, square_len, work)?;
    let cube = rounded_product(&square, candidate, cube_len, work)?;
    let mut forcing = counted_capacity::<(f64, f64)>(cube_len, "represented forcing", work)?;
    for &(re, im) in &cube {
        work.reconstruction_terms += 2;
        let coefficient = (-chi * im, chi * re);
        if !coefficient.0.is_finite() || !coefficient.1.is_finite() {
            return Err(PacketError::OutwardRefusal("represented forcing overflow"));
        }
        forcing.push(coefficient);
    }
    Ok(forcing)
}

fn rounded_product(
    left: &[(f64, f64)],
    right: &[(f64, f64)],
    output_len: usize,
    work: &mut PacketWorkCounters,
) -> Result<Vec<(f64, f64)>, PacketError> {
    let mut output =
        counted_capacity::<(f64, f64)>(output_len, "represented polynomial product", work)?;
    output.resize(output_len, (0.0, 0.0));
    for (j, &a) in left.iter().enumerate() {
        for (k, &b) in right.iter().enumerate() {
            let index = j
                .checked_add(k)
                .ok_or(PacketError::SizeOverflow("represented product index"))?;
            if index >= output_len {
                return Err(PacketError::OutwardRefusal(
                    "represented product exceeds checked size",
                ));
            }
            work.poly_multiply_terms += 1;
            let product = c_mul(a, b);
            output[index] = c_add(output[index], product);
            if !output[index].0.is_finite() || !output[index].1.is_finite() {
                return Err(PacketError::OutwardRefusal(
                    "represented polynomial product overflow",
                ));
            }
        }
    }
    Ok(output)
}

fn interval_complex_add(
    a: IComplex,
    b: IComplex,
    work: &mut PacketWorkCounters,
) -> Result<IComplex, PacketError> {
    work.certificate_items += 1;
    Ok(IComplex {
        re: a
            .re
            .add(b.re)
            .map_err(|_| PacketError::OutwardRefusal("represented interval complex sum"))?,
        im: a
            .im
            .add(b.im)
            .map_err(|_| PacketError::OutwardRefusal("represented interval complex sum"))?,
    })
}

fn interval_complex_sub(
    a: IComplex,
    b: IComplex,
    work: &mut PacketWorkCounters,
) -> Result<IComplex, PacketError> {
    work.certificate_items += 1;
    Ok(IComplex {
        re: a
            .re
            .sub(b.re)
            .map_err(|_| PacketError::OutwardRefusal("represented interval complex difference"))?,
        im: a
            .im
            .sub(b.im)
            .map_err(|_| PacketError::OutwardRefusal("represented interval complex difference"))?,
    })
}

fn interval_complex_mul(
    a: IComplex,
    b: IComplex,
    work: &mut PacketWorkCounters,
) -> Result<IComplex, PacketError> {
    work.certificate_items += 1;
    a.mul(b)
}

fn interval_complex_scale(
    a: IComplex,
    scale: Interval,
    work: &mut PacketWorkCounters,
) -> Result<IComplex, PacketError> {
    work.certificate_items += 1;
    a.scale(scale)
}

fn interval_i_omega_y(
    y: IComplex,
    omega: Interval,
    work: &mut PacketWorkCounters,
) -> Result<IComplex, PacketError> {
    let scaled = interval_complex_scale(y, omega, work)?;
    Ok(IComplex {
        re: Interval::new(-scaled.im.hi, -scaled.im.lo)
            .map_err(|_| PacketError::OutwardRefusal("represented imaginary rotation"))?,
        im: scaled.re,
    })
}

fn conjugate_coefficients(
    polynomial: &[IComplex],
    work: &mut PacketWorkCounters,
) -> Result<Vec<IComplex>, PacketError> {
    let mut output =
        counted_capacity::<IComplex>(polynomial.len(), "represented mean conjugate", work)?;
    for coefficient in polynomial {
        work.certificate_items += 1;
        output.push(IComplex {
            re: coefficient.re,
            im: Interval::new(-coefficient.im.hi, -coefficient.im.lo)
                .map_err(|_| PacketError::OutwardRefusal("represented mean conjugate"))?,
        });
    }
    Ok(output)
}

fn interval_polynomial_product(
    left: &[IComplex],
    right: &[IComplex],
    work: &mut PacketWorkCounters,
) -> Result<Vec<IComplex>, PacketError> {
    let output_len = left
        .len()
        .checked_add(right.len())
        .and_then(|x| x.checked_sub(1))
        .ok_or(PacketError::SizeOverflow("represented interval product"))?;
    let mut output =
        counted_capacity::<IComplex>(output_len, "represented interval product", work)?;
    output.resize(output_len, IComplex::point((0.0, 0.0))?);
    for (j, &a) in left.iter().enumerate() {
        for (k, &b) in right.iter().enumerate() {
            let index = j.checked_add(k).ok_or(PacketError::SizeOverflow(
                "represented interval product index",
            ))?;
            work.certificate_items += 1;
            let term = interval_complex_mul(a, b, work)?;
            output[index] = interval_complex_add(output[index], term, work)?;
        }
    }
    Ok(output)
}

fn rotate_and_scale_i_chi(
    polynomial: &[IComplex],
    chi: f64,
    output_len: usize,
    work: &mut PacketWorkCounters,
) -> Result<Vec<IComplex>, PacketError> {
    if polynomial.len() > output_len {
        return Err(PacketError::OutwardRefusal(
            "represented nonlinear residual exceeds checked stride",
        ));
    }
    let mut output =
        counted_capacity::<IComplex>(output_len, "represented nonlinear residual", work)?;
    output.resize(output_len, IComplex::point((0.0, 0.0))?);
    let scale = Interval::point(chi)
        .map_err(|_| PacketError::OutwardRefusal("represented nonlinear coupling"))?;
    for (index, &coefficient) in polynomial.iter().enumerate() {
        let scaled = interval_complex_scale(coefficient, scale, work)?;
        work.certificate_items += 1;
        output[index] = IComplex {
            re: Interval::new(-scaled.im.hi, -scaled.im.lo)
                .map_err(|_| PacketError::OutwardRefusal("represented nonlinear rotation"))?,
            im: scaled.re,
        };
    }
    Ok(output)
}

fn counted_capacity<T>(
    len: usize,
    label: &'static str,
    work: &mut PacketWorkCounters,
) -> Result<Vec<T>, PacketError> {
    work.curve_setup_allocation_requests += 1;
    let items = u64::try_from(len).map_err(|_| PacketError::SizeOverflow(label))?;
    work.curve_setup_reserved_items += items;
    checked_capacity(len, label)
}

fn copied_slice<T: Copy>(
    source: &[T],
    label: &'static str,
    work: &mut PacketWorkCounters,
) -> Result<Vec<T>, PacketError> {
    let mut copy = counted_capacity::<T>(source.len(), label, work)?;
    work.identity_snapshot_items +=
        u64::try_from(source.len()).map_err(|_| PacketError::SizeOverflow(label))?;
    copy.extend_from_slice(source);
    Ok(copy)
}

fn copy_identity(
    source: &PacketIdentity,
    work: &mut PacketWorkCounters,
) -> Result<PacketIdentity, PacketError> {
    let source_model = &source.model;
    let xi = copied_slice(source_model.xi(), "represented identity xi", work)?;
    let weights = copied_slice(source_model.weights(), "represented identity weights", work)?;
    let initial = copied_slice(
        source_model.initial_amplitudes(),
        "represented identity initial state",
        work,
    )?;
    work.identity_snapshot_items += 5;
    let modes = u64::try_from(source_model.num_modes())
        .map_err(|_| PacketError::SizeOverflow("represented model constructor accounting"))?;
    let constructor_payload = modes
        .checked_mul(4)
        .and_then(|items| items.checked_add(4))
        .ok_or(PacketError::SizeOverflow(
            "represented model constructor accounting",
        ))?;
    let constructor_arithmetic = modes
        .checked_mul(6)
        .and_then(|ops| ops.checked_add(2))
        .ok_or(PacketError::SizeOverflow(
            "represented model constructor accounting",
        ))?;
    // These account for the constructor's actual payload and arithmetic before
    // its validation can refuse the copied source model.
    work.curve_model_constructor_payload_items += constructor_payload;
    work.curve_model_constructor_arithmetic_ops += constructor_arithmetic;
    let model = CollectiveModel::new(
        xi,
        weights,
        initial,
        source_model.width(),
        source_model.chi(),
        source_model.carrier(),
        source_model.kappa(),
    )
    .map_err(|_| PacketError::OutwardRefusal("represented model identity copy"))?;

    let mu = copied_slice(&source.mu, "represented identity moments", work)?;
    let nu = copied_slice(&source.nu, "represented identity candidate", work)?;
    let free = copied_slice(&source.free, "represented identity free kernel", work)?;
    let kernel = copied_slice(&source.kernel, "represented identity kernel", work)?;
    Ok(PacketIdentity {
        model,
        horizon: source.horizon,
        order: source.order,
        mu,
        nu,
        free,
        kernel,
    })
}

fn complex_magnitude_upper(
    value: PacketComplexInterval,
    work: &mut PacketWorkCounters,
) -> Result<f64, PacketError> {
    let re = value.re.mag();
    let im = value.im.mag();
    work.certificate_items += 1;
    let re2 = mul_up(re, re)
        .map_err(|_| PacketError::OutwardRefusal("represented coefficient magnitude"))?;
    work.certificate_items += 1;
    let im2 = mul_up(im, im)
        .map_err(|_| PacketError::OutwardRefusal("represented coefficient magnitude"))?;
    work.certificate_items += 1;
    let square = add_up(re2, im2)
        .map_err(|_| PacketError::OutwardRefusal("represented coefficient magnitude"))?;
    work.certificate_items += 1;
    sqrt_up(square).map_err(|_| PacketError::OutwardRefusal("represented coefficient magnitude"))
}

fn coefficient_sum_bound(
    mean: &[PacketComplexInterval],
    residual: &[PacketComplexInterval],
    modes: usize,
    residual_stride: usize,
    weights: &[f64],
    work: &mut PacketWorkCounters,
) -> Result<ContinuousPacketResidualBound, PacketError> {
    let expected_residual = modes
        .checked_mul(residual_stride)
        .ok_or(PacketError::SizeOverflow(
            "represented residual bound shape",
        ))?;
    if weights.len() != modes || residual.len() != expected_residual {
        return Err(PacketError::OutwardRefusal(
            "represented residual bound shape mismatch",
        ));
    }

    let mut mean_sup = 0.0;
    for &coefficient in mean {
        let magnitude = complex_magnitude_upper(coefficient, work)?;
        work.certificate_items += 1;
        mean_sup = add_up(mean_sup, magnitude)
            .map_err(|_| PacketError::OutwardRefusal("represented mean coefficient sum"))?;
    }

    let mut weighted_square_sup = 0.0;
    for (mode, &weight) in weights.iter().enumerate() {
        let mut mode_sup = 0.0;
        for &coefficient in &residual[mode * residual_stride..(mode + 1) * residual_stride] {
            let magnitude = complex_magnitude_upper(coefficient, work)?;
            work.certificate_items += 1;
            mode_sup = add_up(mode_sup, magnitude)
                .map_err(|_| PacketError::OutwardRefusal("represented residual coefficient sum"))?;
        }
        work.certificate_items += 1;
        let mode_square = mul_up(mode_sup, mode_sup)
            .map_err(|_| PacketError::OutwardRefusal("represented weighted residual norm"))?;
        work.certificate_items += 1;
        let weighted = mul_up(weight, mode_square)
            .map_err(|_| PacketError::OutwardRefusal("represented weighted residual norm"))?;
        work.certificate_items += 1;
        weighted_square_sup = add_up(weighted_square_sup, weighted)
            .map_err(|_| PacketError::OutwardRefusal("represented weighted residual norm"))?;
    }
    work.certificate_items += 1;
    let residual_sup = sqrt_up(weighted_square_sup)
        .map_err(|_| PacketError::OutwardRefusal("represented weighted residual norm"))?;

    Ok(ContinuousPacketResidualBound {
        weighted_residual_sup: Interval::new(0.0, residual_sup)
            .map_err(|_| PacketError::OutwardRefusal("represented weighted residual bound"))?,
        mean_magnitude_sup: Interval::new(0.0, mean_sup)
            .map_err(|_| PacketError::OutwardRefusal("represented mean magnitude bound"))?,
    })
}
