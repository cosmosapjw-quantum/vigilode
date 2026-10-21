use rodas5p_integrators::CollectiveModel;
use rodas5p_integrators::damped_packet::{
    DampedPacketConfig, DampedPacketWork, integrate_verified_damped_packet,
};
use rodas5p_integrators::packet_window::VerifiedPacketConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = CollectiveModel::new(
        vec![-1.0, 0.25, 1.0],
        vec![0.25, 0.5, 0.25],
        vec![(0.4, 0.1), (-0.2, 0.3), (0.1, -0.1)],
        0.4,
        0.3,
        0.0,
        0.2,
    )?;
    let mesh = [0.0, 0.03125, 0.0625, 0.125];
    let config = DampedPacketConfig {
        packet: VerifiedPacketConfig {
            order: 8,
            degree: 12,
            sweeps: 6,
            mean_tolerance: 1.0e-6,
            mode_tolerance: 1.0e-6,
        },
        max_subdivision_levels: 8,
        moment_width_limit: 1.0e-12,
    };
    let mut work = DampedPacketWork::default();
    let result = integrate_verified_damped_packet(&model, &[0.0; 3], &mesh, config, &mut work)?;
    let points: Vec<_> = result
        .points()
        .iter()
        .map(|p| {
            serde_json::json!({
                "time":p.time(),"centers":p.modes(),"mode_radii":p.mode_radii(),
                "mean":p.mean(),"mean_radius":p.mean_radius(),"weighted_radius":p.weighted_radius(),
                "subwindows":p.subwindows(),
            })
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version":1,"source":"represented rotating-frame collective model",
            "kappa":model.kappa(),"chi":model.chi(),"width":model.width(),
            "xi":model.xi(),"weights":model.weights(),"initial":model.initial_amplitudes(),
            "mean_tolerance":config.packet.mean_tolerance,"mode_tolerance":config.packet.mode_tolerance,
            "points":points,"work":{"window_attempts":work.window_attempts,
            "subwindow_attempts":work.subwindow_attempts,"rejected_windows":work.rejected_windows,
            "scalar_operations":work.scalar_operations,"moment_fallback_attempts":work.moment_fallback_attempts},
            "claim":"verified requested outputs; no measured speed or zero-allocation claim"
        }))?
    );
    Ok(())
}
