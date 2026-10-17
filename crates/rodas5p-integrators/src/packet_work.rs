//! Work counters and resource tracking for the causal packet integrator (LP02).
//!
//! Tracks exact scalar operations performed by the causal packet compiler,
//! Picard waveform generator, certificate evaluator, and mode reconstructor.
//!
//! Note: Online Jacobian builds, JVPs, VJPs, Krylov iterations, and linear
//! solves are strictly zero by design.

use serde::{Deserialize, Serialize};

/// Work counters recording operations performed by the causal packet pipeline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PacketWorkCounters {
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

    /// Total scalar terms computed across all packet phases.
    pub fn total_packet_terms(&self) -> u64 {
        self.moment_terms
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
        self.moment_terms += other.moment_terms;
        self.poly_multiply_terms += other.poly_multiply_terms;
        self.volterra_terms += other.volterra_terms;
        self.nonlinear_evaluations += other.nonlinear_evaluations;
        self.output_horner_terms += other.output_horner_terms;
    }
}
