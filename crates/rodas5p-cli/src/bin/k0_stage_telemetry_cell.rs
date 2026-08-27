use anyhow::Result;
use clap::{Parser, ValueEnum};
use rodas5p_integrators::{
    G4S5B0Family, GmresKernelArm, K0StageTelemetryExecutionIdentity, run_k0_stage_telemetry_cell,
};

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliFamily {
    #[value(name = "robertson-ramped")]
    RobertsonRamped,
    #[value(name = "hires-ramped")]
    HiresRamped,
    #[value(name = "van-der-pol-ramped")]
    VanDerPolRamped,
    #[value(name = "rotating-nonnormal")]
    RotatingNonnormal,
    #[value(name = "nonautonomous-stiff-forcing")]
    NonautonomousStiffForcing,
    #[value(name = "semilinear-advection-diffusion-ramped")]
    SemilinearAdvectionDiffusionRamped,
}

impl From<CliFamily> for G4S5B0Family {
    fn from(value: CliFamily) -> Self {
        match value {
            CliFamily::RobertsonRamped => Self::RobertsonRamped,
            CliFamily::HiresRamped => Self::HiresRamped,
            CliFamily::VanDerPolRamped => Self::VanDerPolRamped,
            CliFamily::RotatingNonnormal => Self::RotatingNonnormal,
            CliFamily::NonautonomousStiffForcing => Self::NonautonomousStiffForcing,
            CliFamily::SemilinearAdvectionDiffusionRamped => {
                Self::SemilinearAdvectionDiffusionRamped
            }
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliKernelArm {
    #[value(name = "legacy-restarted-gmres")]
    LegacyRestartedGmres,
    #[value(name = "incremental-givens-candidate")]
    IncrementalGivensCandidate,
}

impl From<CliKernelArm> for GmresKernelArm {
    fn from(value: CliKernelArm) -> Self {
        match value {
            CliKernelArm::LegacyRestartedGmres => Self::LegacyRestartedGmres,
            CliKernelArm::IncrementalGivensCandidate => Self::IncrementalGivensCandidate,
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "k0-stage-telemetry-cell")]
#[command(about = "Generate one failure-preserving exploratory K0 stage-telemetry cell")]
struct Args {
    #[arg(long, value_enum)]
    family: CliFamily,
    #[arg(long, value_enum)]
    kernel_arm: CliKernelArm,
    #[arg(long)]
    repository: String,
    #[arg(long)]
    source_pull_request: u64,
    #[arg(long)]
    source_base_sha: String,
    #[arg(long)]
    source_base_tree: String,
    #[arg(long)]
    execution_head_sha: String,
    #[arg(long)]
    execution_head_tree: String,
    #[arg(long)]
    rust_version: String,
    #[arg(long)]
    cargo_version: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let identity = K0StageTelemetryExecutionIdentity {
        repository: args.repository,
        source_pull_request: args.source_pull_request,
        source_base_sha: args.source_base_sha,
        source_base_tree: args.source_base_tree,
        execution_head_sha: args.execution_head_sha,
        execution_head_tree: args.execution_head_tree,
        rust_version: args.rust_version,
        cargo_version: args.cargo_version,
    };
    let cell = run_k0_stage_telemetry_cell(identity, args.family.into(), args.kernel_arm.into());
    println!("{}", serde_json::to_string_pretty(&cell)?);
    Ok(())
}
