//! Work counters and resource tracking for the causal packet integrator (LP02).
//!
//! Tracks selected arithmetic terms and binary64 payload movement in the causal
//! packet compiler, Picard waveform generator, certificate evaluator, and mode
//! reconstructor. This is not a complete setup/runtime cost model.
//!
//! Note: Online Jacobian builds, JVPs, VJPs, Krylov iterations, and linear
//! solves are strictly zero by design.

use serde::{Deserialize, Serialize};

/// Work counters recording operations performed by the causal packet pipeline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PacketWorkCounters {
    /// Binary64 payload items copied into packet/candidate identity snapshots.
    pub identity_snapshot_items: u64,
    /// Binary64 payload items compared while validating identities; metadata
    /// length/order checks are not included in this count.
    pub identity_comparison_items: u64,
    /// Binary64 payload items copied while cloning the model into a packet.
    pub model_copy_items: u64,
    /// Stored weight/support scalar inputs reduced while constructing a packet
    /// model-bounds snapshot.
    pub model_bounds_snapshot_items: u64,
    /// Stored weight/support scalar inputs re-read while validating the
    /// model-bounds snapshot before certificate or output use.
    pub model_bounds_comparison_items: u64,
    /// Binary64 payload items copied while restricting a candidate for output.
    pub output_copy_items: u64,
    /// Binary64 scalar values written into returned reconstructed modes.
    pub output_value_items: u64,
    /// Scalar multiplications used to restrict the candidate polynomial.
    pub output_restriction_multiplies: u64,
    /// Scalar coefficient/mode items visited by certificate setup and bounds.
    pub certificate_items: u64,
    /// Directed spectral-power recurrence products in outward moment replay.
    /// A failed product remains charged as attempted work.
    pub outward_spectral_power_terms: u64,
    /// Interval coefficient-ratio and accumulation terms in an adaptive
    /// Bernstein transform. Zero when the cheap coefficient gate passes.
    pub bernstein_transform_terms: u64,
    /// Complex control-point averages in dyadic de Casteljau subdivision.
    pub bernstein_refinement_terms: u64,
    /// Scalar arithmetic terms used to build reconstruction polynomials.
    pub reconstruction_terms: u64,
    /// Number of scalar spectral/weighted moment accumulations.
    pub moment_terms: u64,
    /// Number of scalar complex product terms in polynomial multiplications.
    pub poly_multiply_terms: u64,
    /// Number of causal Volterra convolution terms evaluated.
    pub volterra_terms: u64,
    /// Number of nonlinear force evaluations F(p) = i * chi * |p|^2 * p.
    pub nonlinear_evaluations: u64,
    /// Number of Horner polynomial evaluation steps at output times.
    pub output_horner_terms: u64,
    /// Number of fallible setup-buffer reservation attempts for represented
    /// packet curves. Each attempt is charged before reservation.
    pub curve_setup_allocation_requests: u64,
    /// Number of coefficient/workspace items requested in represented-curve
    /// setup reservations, charged before the fallible reserve.
    pub curve_setup_reserved_items: u64,
    /// Fallible outward setup operations for reciprocal horizon and per-mode
    /// frequency products, charged before each attempted interval operation.
    pub curve_setup_interval_ops: u64,
    /// Center or interval divisions that normalize a requested physical time
    /// for represented-curve evaluation, charged before the division.
    pub curve_time_normalization_ops: u64,
    /// Model-payload scalar comparisons when revalidating the copied source
    /// model identity for represented-curve use.
    pub curve_model_revalidation_items: u64,
    /// Constructor payload scalar items passed to `CollectiveModel::new` while
    /// making the fallible represented-curve source identity copy.
    pub curve_model_constructor_payload_items: u64,
    /// Scalar arithmetic operations performed by `CollectiveModel::new` for
    /// the represented-curve source identity copy, charged before the call.
    pub curve_model_constructor_arithmetic_ops: u64,
    /// Attempted T1 output arithmetic: each directed scalar/Interval operation,
    /// each magnitude query, one validated exponential kernel, and each
    /// ordinary mean-center product/sum. Precharged before a fallible call;
    /// represented-curve evaluation uses its existing separate counters.
    /// Constructor shape checks and finite-input validation are excluded.
    pub curve_output_certificate_ops: u64,
    /// Fallible preallocation attempts for the represented-curve output workspace.
    pub curve_output_allocation_requests: u64,
    /// Frozen local-output constructor reservation attempts.
    pub curve_frozen_allocation_requests: u64,
    /// Initial uncertainty scalars copied into a frozen local-output owner.
    pub curve_frozen_radius_items: u64,
    /// Calls attempted on the frozen local-output owner, including refusals.
    pub curve_frozen_query_attempts: u64,
    /// Typed refusals returned by the frozen local-output owner.
    pub curve_frozen_query_refusals: u64,
}

impl PacketWorkCounters {
    /// Constructs a zeroed [`PacketWorkCounters`].
    pub const fn new() -> Self {
        Self {
            identity_snapshot_items: 0,
            identity_comparison_items: 0,
            model_copy_items: 0,
            model_bounds_snapshot_items: 0,
            model_bounds_comparison_items: 0,
            output_copy_items: 0,
            output_value_items: 0,
            output_restriction_multiplies: 0,
            certificate_items: 0,
            outward_spectral_power_terms: 0,
            bernstein_transform_terms: 0,
            bernstein_refinement_terms: 0,
            reconstruction_terms: 0,
            moment_terms: 0,
            poly_multiply_terms: 0,
            volterra_terms: 0,
            nonlinear_evaluations: 0,
            output_horner_terms: 0,
            curve_setup_allocation_requests: 0,
            curve_setup_reserved_items: 0,
            curve_setup_interval_ops: 0,
            curve_time_normalization_ops: 0,
            curve_model_revalidation_items: 0,
            curve_model_constructor_payload_items: 0,
            curve_model_constructor_arithmetic_ops: 0,
            curve_output_certificate_ops: 0,
            curve_output_allocation_requests: 0,
            curve_frozen_allocation_requests: 0,
            curve_frozen_radius_items: 0,
            curve_frozen_query_attempts: 0,
            curve_frozen_query_refusals: 0,
        }
    }

    /// Resets all counters to zero.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Aggregate of heterogeneous counters; not a FLOP count or full runtime cost.
    pub fn total_packet_terms(&self) -> u64 {
        self.moment_terms
            + self.identity_snapshot_items
            + self.identity_comparison_items
            + self.model_copy_items
            + self.model_bounds_snapshot_items
            + self.model_bounds_comparison_items
            + self.output_copy_items
            + self.output_value_items
            + self.output_restriction_multiplies
            + self.certificate_items
            + self.outward_spectral_power_terms
            + self.bernstein_transform_terms
            + self.bernstein_refinement_terms
            + self.reconstruction_terms
            + self.poly_multiply_terms
            + self.volterra_terms
            + self.nonlinear_evaluations
            + self.output_horner_terms
            + self.curve_setup_allocation_requests
            + self.curve_setup_reserved_items
            + self.curve_setup_interval_ops
            + self.curve_time_normalization_ops
            + self.curve_model_revalidation_items
            + self.curve_model_constructor_payload_items
            + self.curve_model_constructor_arithmetic_ops
            + self.curve_output_certificate_ops
            + self.curve_output_allocation_requests
            + self.curve_frozen_allocation_requests
            + self.curve_frozen_radius_items
            + self.curve_frozen_query_attempts
            + self.curve_frozen_query_refusals
    }

    /// Explicitly zero by design (no online Jacobian construction).
    pub const fn jacobian_calls(&self) -> u64 {
        0
    }

    /// Explicitly zero by design (no online Jacobian-vector products).
    pub const fn jvp_calls(&self) -> u64 {
        0
    }

    /// Explicitly zero by design (no online Krylov iterations).
    pub const fn krylov_iterations(&self) -> u64 {
        0
    }

    /// Explicitly zero by design (no matrix factorizations or linear solves).
    pub const fn linear_solves(&self) -> u64 {
        0
    }

    /// Accumulates counters from another instance.
    pub fn accumulate(&mut self, other: &Self) {
        self.identity_snapshot_items += other.identity_snapshot_items;
        self.identity_comparison_items += other.identity_comparison_items;
        self.model_copy_items += other.model_copy_items;
        self.model_bounds_snapshot_items += other.model_bounds_snapshot_items;
        self.model_bounds_comparison_items += other.model_bounds_comparison_items;
        self.output_copy_items += other.output_copy_items;
        self.output_value_items += other.output_value_items;
        self.output_restriction_multiplies += other.output_restriction_multiplies;
        self.certificate_items += other.certificate_items;
        self.outward_spectral_power_terms += other.outward_spectral_power_terms;
        self.bernstein_transform_terms += other.bernstein_transform_terms;
        self.bernstein_refinement_terms += other.bernstein_refinement_terms;
        self.reconstruction_terms += other.reconstruction_terms;
        self.moment_terms += other.moment_terms;
        self.poly_multiply_terms += other.poly_multiply_terms;
        self.volterra_terms += other.volterra_terms;
        self.nonlinear_evaluations += other.nonlinear_evaluations;
        self.output_horner_terms += other.output_horner_terms;
        self.curve_setup_allocation_requests += other.curve_setup_allocation_requests;
        self.curve_setup_reserved_items += other.curve_setup_reserved_items;
        self.curve_setup_interval_ops += other.curve_setup_interval_ops;
        self.curve_time_normalization_ops += other.curve_time_normalization_ops;
        self.curve_model_revalidation_items += other.curve_model_revalidation_items;
        self.curve_model_constructor_payload_items += other.curve_model_constructor_payload_items;
        self.curve_model_constructor_arithmetic_ops += other.curve_model_constructor_arithmetic_ops;
        self.curve_output_certificate_ops += other.curve_output_certificate_ops;
        self.curve_output_allocation_requests += other.curve_output_allocation_requests;
        self.curve_frozen_allocation_requests += other.curve_frozen_allocation_requests;
        self.curve_frozen_radius_items += other.curve_frozen_radius_items;
        self.curve_frozen_query_attempts += other.curve_frozen_query_attempts;
        self.curve_frozen_query_refusals += other.curve_frozen_query_refusals;
    }
}
