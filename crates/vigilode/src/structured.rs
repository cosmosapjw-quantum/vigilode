//! Source-declared dissipative cubic IVPs and continuation providers.
//!
//! Enable the `structured` feature. The plan binds the physical accuracy budget
//! and the exact polynomial source. Every candidate is checked on that source.
//! These models can use polynomial or interval evaluations internally while
//! requiring no Jacobian, JVP, or Newton callback from the user.
pub use rodas5p_integrators::fixed_point_continuation as continuation;
pub use vigilode_coupled::coupled_inverse_be::{
    Attempt as CoupledCubicResult, Config as CoupledCubicConfig, Order as CoupledCubicOrder,
    Source as CoupledCubicSource,
};
pub use vigilode_coupled::rhs_homotopy::{
    CubicDecayAccuracyPlan, CubicDecayAccuracyResult, CubicStageSolver, RhsAcceleration,
};

impl crate::VigilODE {
    /// Integrate `y' = -k y - g y³` with the certified midpoint plan. For
    /// `AlgebraicPredictor`, the source solver uses Picard/bisection repair
    /// when the original interval residual rejects the proposed inverse.
    pub fn solve_cubic_decay(
        plan: &CubicDecayAccuracyPlan,
        solver: CubicStageSolver,
        warm_previous_stage: bool,
    ) -> CubicDecayAccuracyResult {
        vigilode_coupled::rhs_homotopy::integrate_cubic_decay_accuracy_with_solver(
            plan,
            solver,
            warm_previous_stage,
        )
    }

    /// Integrate the source-declared cooperative cubic network with
    /// backward Euler. The result includes the accepted prefix and work
    /// consumed by speculative and rejected root candidates.
    pub fn solve_coupled_cubic(
        source: &CoupledCubicSource,
        config: CoupledCubicConfig,
    ) -> CoupledCubicResult {
        source.run(config)
    }
}
