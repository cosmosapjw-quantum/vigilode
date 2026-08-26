use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use rodas5p_integrators::{
    A1ScientificExecutionIdentity, G4S5B0Family, GmresKernelArm,
    run_a1_post_a2a3_kernel_receipt_cell,
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
#[command(name = "a1-post-a2a3-kernel-cell")]
#[command(about = "Generate one exploratory, nonauthoritative A1 kernel-isolation cell")]
struct Args {
    #[arg(long, value_enum)]
    family: CliFamily,
    #[arg(long, value_enum)]
    kernel_arm: CliKernelArm,
    #[arg(long)]
    repository: String,
    #[arg(long)]
    pull_request: u64,
    #[arg(long)]
    scientific_execution_head_sha: String,
    #[arg(long)]
    scientific_execution_head_tree: String,
    #[arg(long)]
    base_sha: String,
    #[arg(long)]
    base_tree: String,
    #[arg(long)]
    tested_execution_merge_sha: String,
    #[arg(long)]
    tested_execution_merge_tree: String,
    #[arg(long)]
    execution_workflow_run_id: u64,
    #[arg(long)]
    execution_workflow_run_attempt: u64,
    #[arg(long)]
    rust_version: String,
    #[arg(long)]
    cargo_version: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let identity = A1ScientificExecutionIdentity {
        repository: args.repository,
        pull_request: args.pull_request,
        scientific_execution_head_sha: args.scientific_execution_head_sha,
        scientific_execution_head_tree: args.scientific_execution_head_tree,
        base_sha: args.base_sha,
        base_tree: args.base_tree,
        tested_execution_merge_sha: args.tested_execution_merge_sha,
        tested_execution_merge_tree: args.tested_execution_merge_tree,
        execution_workflow_run_id: args.execution_workflow_run_id,
        execution_workflow_run_attempt: args.execution_workflow_run_attempt,
        rust_version: args.rust_version,
        cargo_version: args.cargo_version,
    };
    let cell =
        run_a1_post_a2a3_kernel_receipt_cell(identity, args.family.into(), args.kernel_arm.into())
            .context("post-A2/A3 kernel receipt cell failed")?;
    println!("{}", serde_json::to_string_pretty(&cell)?);
    Ok(())
}
