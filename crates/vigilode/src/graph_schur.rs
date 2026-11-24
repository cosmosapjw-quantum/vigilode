//! Source-declared cooperative cubic networks solved with the original eight-stage RODAS5P.
//! Enable the `structured` feature. A graph path scan acts within each linear stage.
pub use rodas5p_integrators::source_owned_graph_schur::{
    CooperativeCubicOwner, GraphPathScan, GraphSchurTelemetry,
};
pub use rodas5p_integrators::{
    AdaptiveStepConfig, DriverPartial, DriverStopReason, OutputSchedule, Rodas5pFastResult,
};

pub use rodas5p_core::{CoreError, CoreResult};
use rodas5p_integrators::{
    integrate_rodas5p_fast_client_observed_with_atol,
    source_owned_graph_schur::source_owned_cooperative_cubic_problem,
};
use std::sync::{Arc, Mutex};

/// Accepted observation grid, controller stop details and graph-stage work for a single IVP.
#[derive(Clone, Debug)]
pub struct GraphSchurOutcome {
    /// Includes success, attempted/accepted/rejected steps, observed states and work counters.
    pub integration: Rodas5pFastResult,
    /// Last accepted state and reason when the controller returns an incomplete trajectory.
    pub partial: Option<DriverPartial>,
    /// Includes original W residual checks, full-band fallback costs and scan compositions.
    pub stages: GraphSchurTelemetry,
}

impl crate::VigilODE {
    /// Solve `y'_i = -k_i y_i - g_i y_i³ + sum_j c_ij (y_j-y_i)`.
    ///
    /// The immutable coefficients and feedback vertices define both the physical callbacks
    /// and the stage operator. Construction rejects unsupported graphs. Numerical candidate
    /// rejection within a supported graph delegates to the original full-band W solver.
    /// Inspect `integration.observed.success` and `partial` for a controller stop.
    pub fn solve_graph_cubic(
        source: CooperativeCubicOwner,
        scan: GraphPathScan,
        t_span: (f64, f64),
        y0: &[f64],
        controller: &AdaptiveStepConfig,
        output: &OutputSchedule,
        component_atol: &[f64],
    ) -> CoreResult<GraphSchurOutcome> {
        let telemetry = Arc::new(Mutex::new(GraphSchurTelemetry::default()));
        let (problem, client) =
            source_owned_cooperative_cubic_problem(source, Arc::clone(&telemetry))?;
        let (integration, partial, _) = integrate_rodas5p_fast_client_observed_with_atol(
            &problem,
            Box::new(client.with_path_scan(scan)),
            t_span,
            y0,
            controller,
            output,
            component_atol,
        )?;
        let stages = telemetry
            .lock()
            .map_err(|_| CoreError::InvalidInput("graph-Schur telemetry lock poisoned".into()))?
            .clone();
        Ok(GraphSchurOutcome {
            integration,
            partial,
            stages,
        })
    }
}
