//! Requested-time certified collective packet runner; JSON output is reproducible.
//! `cargo run --release -p rodas5p-integrators --example packet_verified_run -- 16 32`
use rodas5p_integrators::packet_window::{VerifiedPacketConfig, integrate_verified_packet};
use rodas5p_integrators::{CollectiveModel, PacketWorkCounters};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let modes: usize = args.get(1).map(|s| s.parse()).transpose()?.unwrap_or(16);
    let windows: usize = args.get(2).map(|s| s.parse()).transpose()?.unwrap_or(32);
    if modes == 0 || windows == 0 || !windows.is_power_of_two() || windows > (1 << 20) {
        return Err("positive modes and power-of-two windows <=1048576 required".into());
    }
    let model = CollectiveModel::new(
        (0..modes)
            .map(|j| -0.75 + 1.5 * j as f64 / modes as f64)
            .collect(),
        vec![1.0 / modes as f64; modes],
        (0..modes)
            .map(|j| (0.2 + 0.02 * (j as f64).sin(), -0.1))
            .collect(),
        0.4,
        0.2,
        0.0,
        0.0,
    )?;
    let mesh: Vec<_> = (0..=windows)
        .map(|k| k as f64 / (8 * windows) as f64)
        .collect();
    let config = VerifiedPacketConfig {
        order: 6,
        degree: 6,
        sweeps: 3,
        mean_tolerance: 1e-8,
        mode_tolerance: 1e-8,
    };
    let mut work = PacketWorkCounters::default();
    let result = integrate_verified_packet(&model, &vec![0.0; modes], &mesh, config, &mut work);
    match result {
        Ok(result) => {
            println!(
                "{}",
                serde_json::json!({
                    "schema":"verified-packet-run-v1", "success":true, "modes":modes,"windows":windows,
                    "model":"native_undamped_zero_carrier_collective", "mean_tolerance":config.mean_tolerance,
                    "mode_tolerance":config.mode_tolerance, "source_xi":model.xi(),"source_weights":model.weights(),
                    "source_initial":model.initial_amplitudes(),"width":model.width(),"chi":model.chi(),
                    "outputs":result.points().iter().map(|p| serde_json::json!({
                        "time":p.time(),"modes":p.modes(),"mode_radii":p.mode_radii(),
                        "mean":p.mean(),"mean_radius":p.mean_radius(),"weighted_radius":p.weighted_radius()
                    })).collect::<Vec<_>>(), "selected_work_counters":work,
                    "counter_authority":"selected_arithmetic_not_complete_runtime_receipt"
                })
            );
        }
        Err(error) => {
            println!(
                "{}",
                serde_json::json!({"schema":"verified-packet-run-v1","success":false,
                "modes":modes,"windows":windows,"error":error.to_string(),"selected_work_counters":work})
            );
            return Err(error.into());
        }
    }
    Ok(())
}
