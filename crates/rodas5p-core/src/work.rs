use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkCounters {
    pub rhs_calls: u64,
    pub rhs_batch_calls: u64,
    pub rhs_evaluations: u64,
    pub ft_calls: u64,
    pub jacobian_builds: u64,
    pub jvp_calls: u64,
    pub jvp_vectors: u64,
    pub mass_matvecs: u64,
    pub nonlinear_solves: u64,
    pub nonlinear_iterations: u64,
    pub nonlinear_residual_evaluations: u64,
    pub nonlinear_jacobian_evaluations: u64,
    pub nonlinear_failures: u64,
    pub linear_solves: u64,
    pub linear_iterations: u64,
    pub linear_matvecs: u64,
    pub preconditioner_apps: u64,
    pub direct_factorizations: u64,
    pub direct_solve_calls: u64,
    pub recycle_projection_calls: u64,
    pub recycle_same_operator_uses: u64,
    pub recycle_cross_operator_refreshes: u64,
    pub recycle_refresh_matvecs: u64,
    pub recycle_updates: u64,
    pub recycle_vectors_selected: u64,
    pub recycle_dropped_vectors: u64,
    pub harmonic_ritz_solves: u64,
    pub orthogonalization_inner_products: u64,
    pub orthogonalization_vector_updates: u64,
    pub diagnostic_matvecs: u64,
    pub phi_actions: u64,
    pub phi_krylov_vectors: u64,
    pub phi_projected_exponentials: u64,
    pub phi_restarts: u64,
    pub phi_dense_oracle_calls: u64,
    pub block_linear_solves: u64,
    pub block_linear_iterations: u64,
    pub block_matvecs: u64,
    pub block_preconditioner_apps: u64,
    pub fast_attempts: u64,
    pub fast_accepts: u64,
    pub fallback_steps: u64,
    pub accepted_steps: u64,
    pub rejected_steps: u64,
    #[serde(default)]
    pub local_error_failures: u64,
    #[serde(default)]
    pub linear_solve_failures: u64,
    #[serde(default)]
    pub nonlinear_solve_failures: u64,
    #[serde(default)]
    pub nonfinite_step_failures: u64,
    /// Stage solves whose residual scale and tolerance came from the v2 WRMS
    /// stage-residual heuristic rather than fixed linear tolerances. This does
    /// not certify endpoint contamination without a resolvent bound.
    #[serde(default)]
    pub forced_stage_solves: u64,
    /// Products with an explicit Jacobian matrix. These are not user JVP
    /// callbacks, so they are never counted in `jvp_calls` (audit F-048).
    /// Omitted from serialized ledgers while zero, so JVP-only receipts keep
    /// their bytes.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub jacobian_matvecs: u64,
    /// Krylov and diagnostic operator applications in state-vector units: a
    /// sequential shifted-operator apply is 1, a block apply over s stages
    /// is s. `linear_matvecs` and `diagnostic_matvecs` stay solver-level
    /// application counts, so only this field compares lanes (audit F-051).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub linear_matvec_vectors: u64,
    /// Preconditioner applications in state-vector units (audit F-051).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub preconditioner_vectors: u64,
    /// Krylov and diagnostic applications merged into this ledger from
    /// ledgers without vector units (re-audit R2, R2-STAT-02). Omitted while
    /// zero; see [`Self::unknown_vector_calls`].
    #[serde(default, skip_serializing_if = "is_zero")]
    pub merged_unknown_vector_calls: u64,
    /// Nonzero phi-combination inputs whose weight `h^k b_k` fell below the
    /// smallest subnormal and became 0 (re-audit R2, PHI-R1). Omitted while
    /// zero.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub phi_weight_underflows: u64,
    /// Research polynomial phi actions (re-audit R3, POLY-01): operator
    /// applications to a whole block (one per recurrence step), the same in
    /// vector units (block width times steps), coefficient tables built and
    /// reused, block buffers allocated, and dense fallbacks. Omitted while
    /// zero.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub poly_block_products: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub poly_vector_products: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub poly_coefficient_setups: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub poly_coefficient_reuses: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub poly_block_allocations: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub poly_fallbacks: u64,
    /// GCRO-DR recycle updates followed by a refresh of `C = M^-1 A U`
    /// (RVJ DAG node SAFE-RECYCLE); the refresh products themselves are in
    /// `recycle_refresh_matvecs`. Omitted while zero.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub recycle_update_refreshes: u64,
}

fn is_zero(value: &u64) -> bool {
    *value == 0
}

impl WorkCounters {
    /// Count vector shifted-operator applications added after `before`.
    ///
    /// Matrix-free shifted actions are recorded by their Krylov categories,
    /// including recycled-subspace refreshes.
    pub fn shifted_operator_applications_since(self, before: Self) -> u64 {
        let delta = self.delta(before);
        delta
            .linear_matvecs
            .saturating_add(delta.diagnostic_matvecs)
            .saturating_add(delta.recycle_refresh_matvecs)
    }

    /// Shifted-operator applications counted as solver-level calls: a block
    /// apply over s stages is one call (audit F-051). Comparable only
    /// between lanes with the same batching; across lanes use
    /// [`Self::operator_state_vectors`].
    pub fn operator_applications(self) -> u64 {
        self.linear_matvecs
            .saturating_add(self.diagnostic_matvecs)
            .saturating_add(self.recycle_refresh_matvecs)
    }

    /// Shifted-operator work in state-vector units: Krylov and diagnostic
    /// applies as declared by the operator (s for an s-stage block apply)
    /// plus recycled-subspace refreshes, which are single-vector applies.
    ///
    /// `None` when calls were recorded but no vector units: a ledger written
    /// before the vector counters existed deserializes them as 0, and that
    /// must read as unknown, not as free work (audit 2026-09-30, B-02).
    pub fn operator_state_vectors(self) -> Option<u64> {
        if self.unknown_vector_calls() > 0 {
            return None;
        }
        Some(
            self.linear_matvec_vectors
                .saturating_add(self.recycle_refresh_matvecs),
        )
    }

    /// Krylov and diagnostic applications whose vector units are unknown.
    ///
    /// A ledger with applications but no vector units predates the vector
    /// counters (every current application records at least one vector), so
    /// all its applications are unknown; otherwise the count merged in from
    /// such ledgers. Accumulation adds these counts, so unknown coverage is
    /// absorbing under merging, in any order and grouping: 16 legacy calls
    /// merged with 1 current call of 1 vector give 17 calls and an unknown
    /// vector cost, not a cost of 1 (re-audit R2, R2-STAT-02).
    pub fn unknown_vector_calls(self) -> u64 {
        let calls = self.linear_matvecs.saturating_add(self.diagnostic_matvecs);
        if calls > 0 && self.linear_matvec_vectors == 0 {
            calls
        } else {
            self.merged_unknown_vector_calls
        }
    }

    /// Saturating component-wise accumulation for independently measured work ledgers.
    ///
    /// This is used when bounded parallel stage/RHS jobs keep local counters and merge them
    /// only after every job has completed.  Saturation preserves the existing failure-safe
    /// accounting semantics instead of wrapping on pathological runs.
    pub fn accumulate(&mut self, other: Self) {
        let unknown = self
            .unknown_vector_calls()
            .saturating_add(other.unknown_vector_calls());
        macro_rules! add_fields { ($($f:ident),* $(,)?) => { $(self.$f = self.$f.saturating_add(other.$f);)* } }
        add_fields!(
            rhs_calls,
            rhs_batch_calls,
            rhs_evaluations,
            ft_calls,
            jacobian_builds,
            jvp_calls,
            jvp_vectors,
            mass_matvecs,
            nonlinear_solves,
            nonlinear_iterations,
            nonlinear_residual_evaluations,
            nonlinear_jacobian_evaluations,
            nonlinear_failures,
            linear_solves,
            linear_iterations,
            linear_matvecs,
            preconditioner_apps,
            direct_factorizations,
            direct_solve_calls,
            recycle_projection_calls,
            recycle_same_operator_uses,
            recycle_cross_operator_refreshes,
            recycle_refresh_matvecs,
            recycle_updates,
            recycle_vectors_selected,
            recycle_dropped_vectors,
            harmonic_ritz_solves,
            orthogonalization_inner_products,
            orthogonalization_vector_updates,
            diagnostic_matvecs,
            phi_actions,
            phi_krylov_vectors,
            phi_projected_exponentials,
            phi_restarts,
            phi_dense_oracle_calls,
            block_linear_solves,
            block_linear_iterations,
            block_matvecs,
            block_preconditioner_apps,
            fast_attempts,
            fast_accepts,
            fallback_steps,
            accepted_steps,
            rejected_steps,
            local_error_failures,
            linear_solve_failures,
            nonlinear_solve_failures,
            nonfinite_step_failures,
            forced_stage_solves,
            jacobian_matvecs,
            linear_matvec_vectors,
            preconditioner_vectors,
            phi_weight_underflows,
            poly_block_products,
            poly_vector_products,
            poly_coefficient_setups,
            poly_coefficient_reuses,
            poly_block_allocations,
            poly_fallbacks,
            recycle_update_refreshes,
        );
        self.merged_unknown_vector_calls = unknown;
    }

    /// Component-wise accumulation that rejects counter overflow.
    ///
    /// Scientific campaign segments are independent solver invocations.  Their
    /// ledgers must not silently saturate while being assembled into one row.
    pub fn checked_accumulate(&mut self, other: Self) -> Option<()> {
        let unknown = self
            .unknown_vector_calls()
            .checked_add(other.unknown_vector_calls())?;
        let mut next = *self;
        macro_rules! checked_add_fields {
            ($($f:ident),* $(,)?) => {{
                $(next.$f = next.$f.checked_add(other.$f)?;)*
            }};
        }
        checked_add_fields!(
            rhs_calls,
            rhs_batch_calls,
            rhs_evaluations,
            ft_calls,
            jacobian_builds,
            jvp_calls,
            jvp_vectors,
            mass_matvecs,
            nonlinear_solves,
            nonlinear_iterations,
            nonlinear_residual_evaluations,
            nonlinear_jacobian_evaluations,
            nonlinear_failures,
            linear_solves,
            linear_iterations,
            linear_matvecs,
            preconditioner_apps,
            direct_factorizations,
            direct_solve_calls,
            recycle_projection_calls,
            recycle_same_operator_uses,
            recycle_cross_operator_refreshes,
            recycle_refresh_matvecs,
            recycle_updates,
            recycle_vectors_selected,
            recycle_dropped_vectors,
            harmonic_ritz_solves,
            orthogonalization_inner_products,
            orthogonalization_vector_updates,
            diagnostic_matvecs,
            phi_actions,
            phi_krylov_vectors,
            phi_projected_exponentials,
            phi_restarts,
            phi_dense_oracle_calls,
            block_linear_solves,
            block_linear_iterations,
            block_matvecs,
            block_preconditioner_apps,
            fast_attempts,
            fast_accepts,
            fallback_steps,
            accepted_steps,
            rejected_steps,
            local_error_failures,
            linear_solve_failures,
            nonlinear_solve_failures,
            nonfinite_step_failures,
            forced_stage_solves,
            jacobian_matvecs,
            linear_matvec_vectors,
            preconditioner_vectors,
            phi_weight_underflows,
            poly_block_products,
            poly_vector_products,
            poly_coefficient_setups,
            poly_coefficient_reuses,
            poly_block_allocations,
            poly_fallbacks,
            recycle_update_refreshes,
        );
        next.merged_unknown_vector_calls = unknown;
        *self = next;
        Some(())
    }

    pub fn delta(self, before: Self) -> Self {
        macro_rules! sub_fields { ($($f:ident),* $(,)?) => { Self { $($f: self.$f.saturating_sub(before.$f),)* } }; }
        sub_fields!(
            rhs_calls,
            rhs_batch_calls,
            rhs_evaluations,
            ft_calls,
            jacobian_builds,
            jvp_calls,
            jvp_vectors,
            mass_matvecs,
            nonlinear_solves,
            nonlinear_iterations,
            nonlinear_residual_evaluations,
            nonlinear_jacobian_evaluations,
            nonlinear_failures,
            linear_solves,
            linear_iterations,
            linear_matvecs,
            preconditioner_apps,
            direct_factorizations,
            direct_solve_calls,
            recycle_projection_calls,
            recycle_same_operator_uses,
            recycle_cross_operator_refreshes,
            recycle_refresh_matvecs,
            recycle_updates,
            recycle_vectors_selected,
            recycle_dropped_vectors,
            harmonic_ritz_solves,
            orthogonalization_inner_products,
            orthogonalization_vector_updates,
            diagnostic_matvecs,
            phi_actions,
            phi_krylov_vectors,
            phi_projected_exponentials,
            phi_restarts,
            phi_dense_oracle_calls,
            block_linear_solves,
            block_linear_iterations,
            block_matvecs,
            block_preconditioner_apps,
            fast_attempts,
            fast_accepts,
            fallback_steps,
            accepted_steps,
            rejected_steps,
            local_error_failures,
            linear_solve_failures,
            nonlinear_solve_failures,
            nonfinite_step_failures,
            forced_stage_solves,
            jacobian_matvecs,
            linear_matvec_vectors,
            preconditioner_vectors,
            phi_weight_underflows,
            poly_block_products,
            poly_vector_products,
            poly_coefficient_setups,
            poly_coefficient_reuses,
            poly_block_allocations,
            poly_fallbacks,
            recycle_update_refreshes,
            merged_unknown_vector_calls,
        )
    }

    /// Exact component-wise subtraction for ledgers that must prove monotonicity.
    ///
    /// Unlike [`Self::delta`], this rejects any component whose cumulative value
    /// is smaller than the claimed prefix instead of hiding the violation through
    /// saturation.
    pub fn checked_delta(self, before: Self) -> Option<Self> {
        macro_rules! checked_sub_fields {
            ($($f:ident),* $(,)?) => {{
                if $(self.$f < before.$f)||* {
                    None
                } else {
                    Some(Self { $($f: self.$f - before.$f,)* })
                }
            }};
        }
        checked_sub_fields!(
            rhs_calls,
            rhs_batch_calls,
            rhs_evaluations,
            ft_calls,
            jacobian_builds,
            jvp_calls,
            jvp_vectors,
            mass_matvecs,
            nonlinear_solves,
            nonlinear_iterations,
            nonlinear_residual_evaluations,
            nonlinear_jacobian_evaluations,
            nonlinear_failures,
            linear_solves,
            linear_iterations,
            linear_matvecs,
            preconditioner_apps,
            direct_factorizations,
            direct_solve_calls,
            recycle_projection_calls,
            recycle_same_operator_uses,
            recycle_cross_operator_refreshes,
            recycle_refresh_matvecs,
            recycle_updates,
            recycle_vectors_selected,
            recycle_dropped_vectors,
            harmonic_ritz_solves,
            orthogonalization_inner_products,
            orthogonalization_vector_updates,
            diagnostic_matvecs,
            phi_actions,
            phi_krylov_vectors,
            phi_projected_exponentials,
            phi_restarts,
            phi_dense_oracle_calls,
            block_linear_solves,
            block_linear_iterations,
            block_matvecs,
            block_preconditioner_apps,
            fast_attempts,
            fast_accepts,
            fallback_steps,
            accepted_steps,
            rejected_steps,
            local_error_failures,
            linear_solve_failures,
            nonlinear_solve_failures,
            nonfinite_step_failures,
            forced_stage_solves,
            jacobian_matvecs,
            linear_matvec_vectors,
            preconditioner_vectors,
            phi_weight_underflows,
            poly_block_products,
            poly_vector_products,
            poly_coefficient_setups,
            poly_coefficient_reuses,
            poly_block_allocations,
            poly_fallbacks,
            recycle_update_refreshes,
            merged_unknown_vector_calls,
        )
    }
}
