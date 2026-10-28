//! Untimed same-stage/work comparison for the declared collision provider.
//! The direct formula is retained; no wall-clock or general router claim follows.
use rodas5p_integrators::polynomial_resolvent::{
    integrate_exchange_implicit_euler, ExchangeCollisionProvider, ExchangeImplicitEulerWorkspace,
    PolynomialPhysicalNorm, PolynomialProviderWork, PolynomialResolventWorkspace,
};
use serde_json::json;

fn main() {
    let blocks = 64;
    let h = 0.125;
    let a = 2.0;
    let b = 1.0;
    let provider = ExchangeCollisionProvider::new(a, b, blocks, 1, 1).unwrap();
    let mut work = PolynomialProviderWork::default();
    let plan = provider
        .prepare_implicit_euler(
            h,
            PolynomialPhysicalNorm::Infinity,
            20261023,
            1e-12,
            &mut work,
        )
        .unwrap();
    let initial: Vec<f64> = (0..2 * blocks)
        .map(|i| if i % 2 == 0 { 1.0 } else { 0.0 })
        .collect();
    let mut quotient = vec![0.0; 2 * blocks];
    let stage = plan
        .apply_provider_checked(
            &provider,
            h,
            PolynomialPhysicalNorm::Infinity,
            &initial,
            &mut quotient,
            &mut PolynomialResolventWorkspace::new(2 * blocks),
            1e-12,
            1e-15,
            &mut work,
        )
        .unwrap();
    let mut direct = vec![0.0; 2 * blocks];
    provider
        .direct_implicit_euler(h, &initial, &mut direct, &mut work)
        .unwrap();
    // Neumann is a legitimate contraction control here: ||hA||inf=0.375.
    // Its final residual action is charged, just as the quotient's residual is.
    let mut iterative = initial.clone();
    let mut actions = 0_u64;
    let mut residual = f64::INFINITY;
    let mut accepted = false;
    for _ in 0..128 {
        actions += 1;
        for (x, rhs) in iterative.chunks_exact_mut(2).zip(initial.chunks_exact(2)) {
            let transfer = h * (-a * x[0] + b * x[1]);
            let next = [rhs[0] + transfer, rhs[1] - transfer];
            x.copy_from_slice(&next);
        }
        // Always evaluate the residual of the actual candidate that is returned.
        actions += 1;
        residual = 0.0;
        for (x, rhs) in iterative.chunks_exact(2).zip(initial.chunks_exact(2)) {
            let transfer = h * (-a * x[0] + b * x[1]);
            residual = residual
                .max((rhs[0] - x[0] + transfer).abs())
                .max((rhs[1] - x[1] - transfer).abs());
        }
        if residual + 1e-15 <= 1e-12 {
            accepted = true;
            break;
        }
    }
    let mut final_state = vec![0.0; 2 * blocks];
    let trajectory = integrate_exchange_implicit_euler(
        &provider,
        &plan,
        h,
        8,
        &initial,
        &mut final_state,
        &mut ExchangeImplicitEulerWorkspace::new(2 * blocks),
        1e-12,
        1e-15,
        &mut work,
    )
    .unwrap();
    let exact_u = 1.0 / 3.0 + (2.0 / 3.0) * (-3.0_f64).exp();
    let max_stage_difference = quotient
        .iter()
        .zip(&direct)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max);
    println!(
        "{}",
        json!({"schema":"vigilode.frontier-polynomial-provider.v1","timing_authority":"diagnostic_only","timed":false,
      "scope":"homogeneous two-state exchange provider plus implicit Euler; causal/RODAS adoption HOLD",
      "blocks":blocks,"step":h,"physical_norm":"infinity","relation_degree":plan.degree(),"symbolic_relation":"A^2=-(a+b)A",
      "floating_relation_defect_ratio":plan.relation_defect_ratio(),"bound_status":"conditional floating estimate; callback rounding allowance separate",
      "stage":{"accepted":stage.residual_accepted,"original_residual":stage.original_residual_norm,"candidate_actions":stage.candidate_operator_actions,"residual_actions":stage.residual_operator_actions,"maximum_difference_from_direct":max_stage_difference},
      "direct":{"solves_attempted":work.direct_solves_attempted,"blocks_solved":work.direct_blocks_solved,"formula_evaluations":work.direct_block_formula_evaluations,"operator_actions":0},
      "neumann":{"accepted":accepted,"actions_including_all_residuals":actions,"actual_original_residual":residual},
      "trajectory":{"completed_steps":trajectory.completed_steps,"last_accepted_time":trajectory.last_accepted_time,"success":trajectory.failure.is_none(),"maximum_endpoint_error":(final_state[0]-exact_u).abs()},
      "all_work":{"setups_attempted":work.setups_attempted,"setup_failures":work.setup_failures,"applications_attempted":work.applications_attempted,"applications_refused":work.applications_refused,"operator_actions_attempted":work.operator_actions_attempted,"operator_actions_completed":work.operator_actions_completed,"stages_attempted":work.stages_attempted,"stages_accepted":work.stages_accepted},
      "comparison":"same stage residual target; direct-block arithmetic counted separately; no blanket speed superiority"})
    );
}
