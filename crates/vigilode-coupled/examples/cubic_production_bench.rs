//! Audit, paired timing, and one-arm profiling of proof-budget cubic stages.
use serde_json::json;
use std::{hint::black_box, time::Instant};
use vigilode_coupled::rhs_homotopy::{
    CubicDecayAccuracyPlan, CubicStageSolver, RhsAcceleration,
    integrate_cubic_decay_accuracy_with_solver as integrate,
};

fn solver(name: &str) -> CubicStageSolver {
    match name {
        "generic" => CubicStageSolver::GenericHomotopy(RhsAcceleration::Picard),
        "scalar" => CubicStageSolver::ScalarHomotopy(RhsAcceleration::Picard),
        "algebraic" => CubicStageSolver::AlgebraicPredictor,
        "rational" => CubicStageSolver::RationalPredictor,
        _ => panic!("arm must be generic, scalar, algebraic, or rational"),
    }
}

fn timed(plan: &CubicDecayAccuracyPlan, name: &str, repetitions: usize) -> (f64, u64) {
    let stage = solver(name);
    let mut checksum = 0u64;
    let start = Instant::now();
    for _ in 0..repetitions {
        let result = black_box(integrate(black_box(plan), stage, true));
        assert!(result.completed, "{:?}", result.failure);
        checksum = checksum.wrapping_add(result.points.last().unwrap().state.to_bits());
        black_box(&result);
    }
    (start.elapsed().as_secs_f64(), checksum)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|s| s == "--arm") {
        let name = args.get(1).expect("arm");
        let repetitions = args
            .get(2)
            .map(|s| s.parse::<usize>().unwrap())
            .unwrap_or(20);
        let plan = CubicDecayAccuracyPlan::new(1.0, 0.5, 1.0, 1.0, 1e-5, 65536, 1_000_000).unwrap();
        let (seconds, checksum) = timed(&plan, name, repetitions);
        println!(
            "{}",
            json!({"arm":name,"repetitions":repetitions,"steps":plan.steps(),"seconds":seconds,"checksum":checksum})
        );
        return;
    }
    if args.first().is_some_and(|s| s == "--bench") {
        let repetitions = args
            .get(1)
            .map(|s| s.parse::<usize>().unwrap())
            .unwrap_or(128);
        let mut rows = Vec::new();
        for (case, k, g, t, tol) in [
            ("cubic", 1.0, 0.5, 1.0, 1e-5),
            ("nonlinear", 1.0, 20.0, 0.1, 1e-4),
        ] {
            let plan = CubicDecayAccuracyPlan::new(k, g, 1.0, t, tol, 65536, 1_000_000).unwrap();
            for name in ["generic", "scalar", "algebraic", "rational"] {
                black_box(timed(&plan, name, 1));
            }
            for (a, b) in [
                ("generic", "scalar"),
                ("scalar", "algebraic"),
                ("generic", "algebraic"),
                ("generic", "rational"),
                ("scalar", "rational"),
                ("algebraic", "rational"),
            ] {
                for block in 0..5 {
                    let order = if block % 2 == 0 {
                        [a, b, b, a]
                    } else {
                        [b, a, a, b]
                    };
                    let mut a_seconds = 0.0;
                    let mut b_seconds = 0.0;
                    let mut samples = Vec::new();
                    for name in order {
                        let (seconds, checksum) = timed(&plan, name, repetitions);
                        if name == a {
                            a_seconds += seconds;
                        } else {
                            b_seconds += seconds;
                        }
                        samples.push(json!({"arm":name,"seconds":seconds,"checksum":checksum}));
                    }
                    rows.push(json!({"case":case,"a":a,"b":b,"block":block,"steps":plan.steps(),"repetitions":repetitions,"a_over_b":a_seconds/b_seconds,"samples":samples}));
                }
            }
        }
        println!("{}",serde_json::to_string_pretty(&json!({"protocol":"five ABBA/BAAB paired blocks; complete certified trajectory including output allocation; same plan; no oracle/JSON in timing","rows":rows})).unwrap());
        return;
    }
    let mut rows = Vec::new();
    for (case, k, g, y, t, tol) in [
        ("cubic_loose", 1.0, 0.5, 1.0, 1.0, 1e-3),
        ("cubic_medium", 1.0, 0.5, 1.0, 1.0, 1e-4),
        ("cubic_tight", 1.0, 0.5, 1.0, 1.0, 1e-5),
        ("linear", 1.0, 0.0, 1.0, 1.0, 1e-5),
        ("nonlinear", 1.0, 20.0, 1.0, 0.1, 1e-4),
        ("zero", 1.0, 0.5, 0.0, 1.0, 1e-5),
    ] {
        let plan = CubicDecayAccuracyPlan::new(k, g, y, t, tol, 65536, 1_000_000).unwrap();
        for name in ["generic", "scalar", "algebraic", "rational"] {
            let result = integrate(&plan, solver(name), true);
            let mut maximum_error = 0.0_f64;
            let mut bound_holds = true;
            for point in &result.points {
                let exact = y * (-k * point.time).exp()
                    / (1.0 + (g / k) * y * y * (-(-2.0 * k * point.time).exp_m1())).sqrt();
                let error = (point.state - exact).abs();
                maximum_error = maximum_error.max(error);
                bound_holds &= error <= point.physical_error_bound;
            }
            rows.push(json!({"case":case,"arm":name,"k":k,"g":g,"y0":y,"horizon":t,"tolerance":tol,"plan":plan,"completed":result.completed,"maximum_grid_error":maximum_error,"all_errors_below_bound":bound_holds,"trajectory":result}));
        }
    }
    println!("{}",serde_json::to_string_pretty(&json!({"scope":"identical proved plan, sealed cubic, warm seed, original outward polynomial verification","rows":rows})).unwrap());
}
