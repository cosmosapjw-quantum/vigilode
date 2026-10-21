use rodas5p_integrators::{
    ActualKHomotopyConfig, ActualKHomotopyWork, CollectiveModel,
    certify_collective_actual_k_homotopy,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let linear = match args.as_slice() {
        [] => false,
        [flag] if flag == "--linear" => true,
        _ => return Err("usage: actual_k_homotopy_run [--linear]".into()),
    };
    let model = if linear {
        CollectiveModel::new(
            vec![0.5],
            vec![1.0],
            vec![(0.25, -0.125)],
            0.5,
            0.0,
            0.0,
            0.125,
        )?
    } else {
        CollectiveModel::new(
            vec![-1.0, 1.0],
            vec![0.4, 0.6],
            vec![(0.2, -0.1), (0.3, 0.05)],
            0.7,
            0.4,
            0.0,
            0.1,
        )?
    };
    let config = ActualKHomotopyConfig {
        h: 1.0 / 1024.0,
        lambdas: vec![0.0, 0.5, 1.0],
        rounds_per_stage: 10,
        tube_radius: 1e-3,
        stage_output_tolerance: 1e-9,
    };
    let mut work = ActualKHomotopyWork::default();
    let start = std::time::Instant::now();
    let result = certify_collective_actual_k_homotopy(&model, &config, &mut work)?;
    let elapsed = start.elapsed().as_secs_f64();
    let c = result.coefficients();
    println!(
        "{}",
        serde_json::json!({"status":"CERTIFIED_ACTUAL_K_COMMON_LAMBDA_TUBE",
        "ode_truncation_certificate":false,"measured_speed_comparison":false,"config":config,
        "elapsed_including_setup_verification_seconds":elapsed,
        "source":{"weights":model.weights(),"xi":model.xi(),"initial":model.initial_amplitudes(),"width":model.width(),
            "chi":model.chi(),"kappa":model.kappa(),"carrier":model.carrier()},
        "h":result.h(),"alpha":(0..c.stages()).map(|i|c.alpha.row(i).to_vec()).collect::<Vec<_>>(),
        "beta":(0..c.stages()).map(|i|c.beta.row(i).to_vec()).collect::<Vec<_>>(),"gamma":c.gamma,"b":c.b,
        "frozen_jacobian":(0..model.dimension()).map(|i|result.frozen_jacobian().row(i).to_vec()).collect::<Vec<_>>(),
        "candidate":result.stages(),"stage_radii":result.stage_radii(),"output":result.output(),"output_radii":result.output_radii(),
        "common_tube_radius":result.common_tube_radius(),"majorant":result.majorant(),"common_defect":result.common_defect(),
        "inverse_residual":result.inverse_residual(),"leakage_q":result.leakage_q(),"noncausal_alpha_max":result.noncausal_alpha_max(),
        "noncausal_linear_max":result.noncausal_linear_max(),"work":work})
    );
    Ok(())
}
