//! Analytically derived algebraic residual budget; no timing or tuning.
use rodas5p_integrators::ParallelExecution;
use serde_json::json;
use vigilode_coupled::structural_mean_field::MeanFieldAffine;

fn main() {
    let n = 17;
    let steps = 64;
    let y0: Vec<_> = (0..n).map(|i| 0.5 + 0.1 * (0.7 * i as f64).sin()).collect();
    let forcing: Vec<_> = (0..n)
        .map(|i| 0.2 + 0.05 * (1.9 * i as f64).cos())
        .collect();
    let model = MeanFieldAffine::new(20.0, 19.0, forcing.clone()).unwrap();
    let plan = model.plan(1.0 / steps as f64).unwrap();
    let budget = plan.algebraic_residual_budget(n, steps, 1e-8).unwrap();
    let execution = ParallelExecution::sequential();
    let my = y0.iter().sum::<f64>() / n as f64;
    let mb = forcing.iter().sum::<f64>() / n as f64;
    let mut arms = Vec::new();
    for quotient in [false, true] {
        let mut y = y0.clone();
        let mut maximum_grid_error = 0.0_f64;
        let mut maximum_residual = 0.0_f64;
        let mut rhs_evaluations = 0;
        let mut observations = vec![y.clone()];
        for j in 1..=steps {
            let candidate = if quotient {
                plan.step(&model, &y, budget.residual_limit, &execution)
                    .unwrap()
            } else {
                model
                    .sequential_step(&y, plan.step_size(), budget.residual_limit, &execution)
                    .unwrap()
            };
            rhs_evaluations += candidate.work.rhs_evaluations;
            maximum_residual = maximum_residual.max(candidate.full_residual_linf);
            y = candidate.y_new;
            let t = j as f64 / steps as f64;
            let collective = (-t).exp() * my - (-t).exp_m1() * mb;
            for i in 0..n {
                let reference = collective + (-20.0 * t).exp() * (y0[i] - my)
                    - (-20.0 * t).exp_m1() / 20.0 * (forcing[i] - mb);
                maximum_grid_error = maximum_grid_error.max((y[i] - reference).abs());
            }
            observations.push(y.clone());
        }
        assert!(maximum_grid_error < 1e-6);
        arms.push(
            json!({"quotient":quotient,"rhs_evaluations":rhs_evaluations,
            "maximum_grid_error":maximum_grid_error,"maximum_sampled_residual":maximum_residual,
            "observations":observations}),
        );
    }
    println!("{}",serde_json::to_string_pretty(&json!({
        "accuracy_specification":{"global_algebraic_budget":1e-8,"physical_comparison_cap":1e-6},
        "budget":budget,"arms":arms,
        "rounding_scope":"f64 formula evaluations and sampled residuals; no interval guarantee",
        "coefficient_initialization":"one shared fixed 8x8 metadata inverse outside per-step counters"
    })).unwrap());
}
