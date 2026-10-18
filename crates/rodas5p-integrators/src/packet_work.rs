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
pub struct PacketWorkCounters {
    /// Binary64 payload items copied into packet/candidate identity snapshots.
    pub identity_snapshot_items: u64,
    /// Binary64 payload items compared while validating identities; metadata
    /// length/order checks are not included in this count.
    pub identity_comparison_items: u64,
    /// Binary64 payload items copied while cloning the model into a packet.
    pub model_copy_items: u64,
    /// Binary64 payload items copied while restricting a candidate for output.
    pub output_copy_items: u64,
    /// Binary64 scalar values written into returned reconstructed modes.
    pub output_value_items: u64,
    /// Scalar multiplications used to restrict the candidate polynomial.
    pub output_restriction_multiplies: u64,
    /// Scalar coefficient/mode items visited by certificate setup and bounds.
    pub certificate_items: u64,
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
}

impl PacketWorkCounters {
    /// Constructs a zeroed [`PacketWorkCounters`].
    pub const fn new() -> Self {
        Self {
            identity_snapshot_items: 0,
            identity_comparison_items: 0,
            model_copy_items: 0,
            output_copy_items: 0,
            output_value_items: 0,
            output_restriction_multiplies: 0,
            certificate_items: 0,
            reconstruction_terms: 0,
            moment_terms: 0,
            poly_multiply_terms: 0,
            volterra_terms: 0,
            nonlinear_evaluations: 0,
            output_horner_terms: 0,
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
            + self.output_copy_items
            + self.output_value_items
            + self.output_restriction_multiplies
            + self.certificate_items
            + self.reconstruction_terms
            + self.poly_multiply_terms
            + self.volterra_terms
            + self.nonlinear_evaluations
            + self.output_horner_terms
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
        self.output_copy_items += other.output_copy_items;
        self.output_value_items += other.output_value_items;
        self.output_restriction_multiplies += other.output_restriction_multiplies;
        self.certificate_items += other.certificate_items;
        self.reconstruction_terms += other.reconstruction_terms;
        self.moment_terms += other.moment_terms;
        self.poly_multiply_terms += other.poly_multiply_terms;
        self.volterra_terms += other.volterra_terms;
        self.nonlinear_evaluations += other.nonlinear_evaluations;
        self.output_horner_terms += other.output_horner_terms;
    }
}
