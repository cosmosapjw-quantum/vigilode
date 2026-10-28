//! Small immutable-budget nonlinear adaptive packet execution with full attempts.
use rodas5p_integrators::packet_window::{
    VerifiedPacketAdaptiveConfig, VerifiedPacketConfig, VerifiedPacketDiagnostics,
    integrate_verified_packet_adaptive,
};
use rodas5p_integrators::{CollectiveModel, PacketWorkCounters};

fn main() {
    let bernstein_max_depth = std::env::args().nth(1).map(|arg| {
        arg.parse::<usize>()
            .expect("optional argument: nonnegative Bernstein depth")
    });
    let model = CollectiveModel::new(
        vec![-0.75, 0.125, 1.0],
        vec![0.2, 0.3, 0.5000000000002],
        vec![(0.2, -0.1), (-0.1, 0.2), (0.15, 0.1)],
        0.4,
        0.2,
        0.0,
        0.0,
    )
    .unwrap();
    let config = VerifiedPacketConfig {
        order: 1,
        degree: 1,
        sweeps: 3,
        mean_tolerance: 1e-4,
        mode_tolerance: 1e-4,
    };
    let mesh = [0.0, 0.0625, 0.125];
    let mut diagnostics = VerifiedPacketDiagnostics::default();
    let mut work = PacketWorkCounters::default();
    let result = integrate_verified_packet_adaptive(
        &model,
        &[1e-9; 3],
        &mesh,
        config,
        VerifiedPacketAdaptiveConfig {
            max_bisections: 16,
            max_attempts: 2000,
            bernstein_max_depth,
        },
        &mut diagnostics,
        &mut work,
    );
    let attempts: Vec<_> = diagnostics
        .attempts()
        .iter()
        .map(|a| {
            serde_json::json!({
                    "start": a.start_time, "end": a.end_time,
                    "incoming_weighted_radius": a.incoming_weighted_radius,
                    "accepted": a.outcome.is_ok(),
            "pre_refinement_error": a.pre_refinement_error.as_ref().map(ToString::to_string),
                    "outgoing_weighted_radius": a.outcome.as_ref().ok(),
                    "refusal": a.outcome.as_ref().err().map(ToString::to_string),
                    "work_before": a.work_before, "work_after": a.work_after,
                })
        })
        .collect();
    let outputs: Vec<_> = result
        .as_ref()
        .map(|flow| {
            flow.points().iter().map(|p| {
        serde_json::json!({"time":p.time(), "modes":p.modes(), "mode_radii":p.mode_radii(),
            "mean":p.mean(), "mean_radius":p.mean_radius(), "weighted_radius":p.weighted_radius()})
    }).collect()
        })
        .unwrap_or_default();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version":1, "bernstein_max_depth":bernstein_max_depth, "model":"native undamped collective cubic", "chi":model.chi(),
            "width":model.width(), "xi":model.xi(), "weights":model.weights(),
            "initial":model.initial_amplitudes(), "initial_radii":vec![1e-9;3],
            "requested_mesh":mesh, "mean_tolerance":config.mean_tolerance,
            "mode_tolerance":config.mode_tolerance, "success":result.is_ok(),
            "error":result.as_ref().err().map(ToString::to_string),
            "accepted":diagnostics.accepted(), "rejected":diagnostics.rejected(),
            "attempts":attempts, "outputs":outputs, "work":work,
            "cost_authority":"selected structural counters only; no wall-time or allocator claim",
        }))
        .unwrap()
    );
    if result.is_err() {
        std::process::exit(1);
    }
}
