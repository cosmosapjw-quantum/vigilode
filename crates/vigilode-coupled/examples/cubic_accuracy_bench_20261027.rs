//! Physical-accuracy comparison with proof-derived scientific constants.
use serde_json::json;
use vigilode_coupled::rhs_homotopy::{
    CubicDecayAccuracyPlan, RhsAcceleration, integrate_cubic_decay_accuracy,
};

fn main() {
    let mut rows = Vec::new();
    for tolerance in [1e-3, 1e-4, 1e-5] {
        let plan = CubicDecayAccuracyPlan::new(1.0, 0.5, 1.0, 1.0, tolerance, 16384, 1_000_000)
            .expect("bounded model plan");
        for mode in [RhsAcceleration::Picard, RhsAcceleration::ResidualDominance] {
            for warm in [false, true] {
                let trajectory = integrate_cubic_decay_accuracy(&plan, mode, warm);
                let mut maximum_error = 0.0_f64;
                let mut worst_bound_ratio = 0.0_f64;
                let mut bounds_hold = true;
                for p in &trajectory.points {
                    let exact =
                        (-p.time).exp() / (1.0 + 0.5 * (1.0 - (-2.0 * p.time).exp())).sqrt();
                    let error = (p.state - exact).abs();
                    maximum_error = maximum_error.max(error);
                    if p.physical_error_bound > 0.0 {
                        worst_bound_ratio = worst_bound_ratio.max(error / p.physical_error_bound);
                    }
                    bounds_hold &= error <= p.physical_error_bound;
                }
                rows.push(json!({"physical_tolerance":tolerance,"mode":mode,"warm_previous_stage":warm,"plan":plan,"completed":trajectory.completed,"maximum_grid_error":maximum_error,"all_observed_errors_below_outward_bound":bounds_hold,"maximum_observed_error_over_bound":worst_bound_ratio,"total_polynomial_evaluations":trajectory.map_evaluations+trajectory.interval_polynomial_evaluations,"trajectory":trajectory}));
            }
        }
    }
    println!("{}",serde_json::to_string_pretty(&json!({
        "target":"cubic-decay-proof-constant-physical-accuracy-20261027",
        "scientific_parameters":{"k":1.0,"g":0.5,"y0":1.0,"horizon":1.0,"accuracy_inputs":[1e-3,1e-4,1e-5]},
        "selection":"smallest dyadic N satisfying outward truncation<physical tolerance and exact domain Lipschitz q<1; algebraic budget is the remaining tolerance",
        "policy":"Picard versus ResidualDominance eta=1 with no magnitude/gain/slow-ratio/rejection-count heuristic; direct lambda=1",
        "work":"all ordinary map callbacks and interval polynomial verifications, including any polishing, counted separately and together",
        "limits":"physical bound belongs to the sealed scalar polynomial model and directed binary64 arithmetic assumptions; no generic callback q is inferred",
        "rows":rows
    })).unwrap());
}
