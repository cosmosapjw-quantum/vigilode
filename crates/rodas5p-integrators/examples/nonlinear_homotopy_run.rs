use rodas5p_integrators::{
    CollectiveModel, NonlinearHomotopyConfig, NonlinearHomotopyWork, integrate_nonlinear_homotopy,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let n = 8;
    let model = CollectiveModel::new(
        (0..n)
            .map(|j| -1.0 + 2.0 * j as f64 / (n - 1) as f64)
            .collect(),
        vec![1.0 / n as f64; n],
        (0..n)
            .map(|j| (0.2 + 0.01 * j as f64, 0.1 - 0.02 * j as f64))
            .collect(),
        1.2,
        0.7,
        0.0,
        0.0,
    )?;
    let mesh: Vec<_> = (0..=8).map(|j| j as f64 / 64.0).collect();
    let config = NonlinearHomotopyConfig {
        split_substeps: 4,
        correction_rounds: 4,
        max_refinements: 2,
        lambdas: vec![1.0],
        weighted_tolerance: 1e-5,
        packet_order: 5,
        packet_degree: 10,
        packet_sweeps: 8,
        damped_subdivision_levels: 6,
    };
    let mut work = NonlinearHomotopyWork::default();
    let start = std::time::Instant::now();
    let answer = integrate_nonlinear_homotopy(&model, &mesh, &config, &mut work);
    let elapsed = start.elapsed().as_secs_f64();
    match answer {
        Ok(result) => println!(
            "{}",
            serde_json::json!({"status":"CERTIFIED_REQUESTED_ENDPOINTS",
            "source":{"weights":model.weights(),"xi":model.xi(),"initial":model.initial_amplitudes(),
                "width":model.width(),"chi":model.chi(),"kappa":model.kappa(),"carrier":model.carrier()},
            "mesh":result.mesh(),"weighted_tolerance":config.weighted_tolerance,
            "candidate_roundoff_certificate":false,"measured_speed_comparison":false,"config":config,
            "elapsed_including_setup_verification_seconds":elapsed,
            "final_substeps":result.final_substeps(),"final_rounds":result.final_rounds(),
            "points":result.points().iter().map(|p|serde_json::json!({"time":p.time(),"modes":p.modes(),
                "weighted_radius":p.weighted_radius(),"mode_radii":p.mode_radii()})).collect::<Vec<_>>(),
            "work":work})
        ),
        Err(error) => {
            eprintln!(
                "{}",
                serde_json::json!({"status":"REFUSED","error":error.to_string(),"work":work,
            "elapsed_including_setup_verification_seconds":elapsed})
            );
            return Err(error.into());
        }
    }
    Ok(())
}
