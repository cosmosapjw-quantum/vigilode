//! Bits of the shipped matrix-free driver on small problems, recorded BEFORE
//! the driver was edited for loop node V24 (owner decision OD-L1-11).
//!
//! The shipped stage-solve policy (restart cycles of the configured length,
//! the true residual tested once per cycle) must keep its arithmetic bit for
//! bit. Every expected line below was printed by this file at the base
//! commit of the node (`V24_PRINT_BITS=1 cargo test ... -- --nocapture`),
//! before any edit of `rodas5p_matrix_free_fast.rs` or `gmres_staged.rs`.
//! The full-size recording of the node is
//! `research/production_loop_20261116/mf_adaptive/RECORDING_A.json`.

#[path = "stage_clients_krylov_v17_problems.rs"]
mod problems;

use problems::{Family, adaptive, declared, v06};
use rodas5p_core::CoreResult;
use rodas5p_integrators::{
    OutputSchedule, Rodas5pMfFastResult, RoutingOptions,
    integrate_rodas5p_mf_fast_observed_gmres_into,
    integrate_rodas5p_mf_fast_observed_gmres_into_ls_workspace,
};
use sha2::{Digest, Sha256};

fn hex(digest: Sha256) -> String {
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The small problems: reduced instances of the families of nodes V17 and
/// V06 (`v06:` prefix).
const SMALL: [&str; 5] = [
    "dev_fhn3d",
    "dev_cusp1d",
    "dev_burgers2d",
    "dev_bruss1d",
    "v06:dev_adr2d",
];

const RTOLS: [f64; 2] = [1.0e-4, 1.0e-6];

fn family(id: &str) -> CoreResult<Family> {
    match id.strip_prefix("v06:") {
        Some(base) => v06::declared(base),
        None => declared(id),
    }
}

/// One line of bits of a run: the end time and state, the step counts and
/// every counter.
fn line(run: &Rodas5pMfFastResult) -> String {
    let mut state = Sha256::new();
    for value in run.observed.y.last().expect("an end state") {
        state.update(value.to_le_bytes());
    }
    let mut counters = Sha256::new();
    counters.update(serde_json::to_string(&run.observed.counters).expect("counters"));
    let t_end = run.observed.t.last().copied().expect("an end time");
    format!(
        "ok={} t={:016x} att={} acc={} rej={} reuse={} jvp={} it={} solves={} lsf={} state={} counters={}",
        run.observed.success,
        t_end.to_bits(),
        run.attempts,
        run.accepted_steps,
        run.rejected_steps,
        run.state_reuses,
        run.observed.counters.jvp_calls,
        run.observed.counters.linear_iterations,
        run.observed.counters.linear_solves,
        run.observed.counters.linear_solve_failures,
        &hex(state)[..32],
        &hex(counters)[..32],
    )
}

/// The shipped entry of the route (`ls_workspace`: the entry the facade
/// uses) on `id` at `rtol`.
fn shipped(id: &str, rtol: f64, ls_workspace: bool) -> Rodas5pMfFastResult {
    let family = family(id).unwrap();
    let config = adaptive(&family, rtol);
    let output = OutputSchedule::new(vec![family.span.0, family.span.1]).unwrap();
    let linear = RoutingOptions::default().matrix_free;
    let integrate = if ls_workspace {
        integrate_rodas5p_mf_fast_observed_gmres_into_ls_workspace
    } else {
        integrate_rodas5p_mf_fast_observed_gmres_into
    };
    integrate(
        &family.problem,
        family.span,
        &family.y0,
        &linear,
        &config,
        &output,
    )
    .unwrap()
}

/// Printed at the base commit 892132e5 (before any edit of the node).
const RECORDED: [(&str, &str); 10] = [
    (
        "dev_fhn3d 1e-4",
        "ok=true t=4044000000000000 att=66 acc=48 rej=18 reuse=18 jvp=22176 it=21120 solves=528 lsf=0 state=22ddc9872412b6a903f41abb8402cf58 counters=beeef1c9ce2569933d20c72eb646609a",
    ),
    (
        "dev_fhn3d 1e-6",
        "ok=true t=4044000000000000 att=117 acc=103 rej=14 reuse=14 jvp=39312 it=37440 solves=936 lsf=0 state=b93992a6bacc8302687374ca08bdac77 counters=96668ab189c2a9370100527abb8e55c3",
    ),
    (
        "dev_cusp1d 1e-4",
        "ok=true t=3fe6666666666666 att=45 acc=32 rej=13 reuse=13 jvp=16878 it=16120 solves=355 lsf=0 state=529514de7b5c9c1bc05779223a216fda counters=0e8557ed3ed7e7ac496ea93fb4adfe63",
    ),
    (
        "dev_cusp1d 1e-6",
        "ok=true t=3fe6666666666666 att=81 acc=60 rej=21 reuse=21 jvp=27662 it=26360 solves=643 lsf=0 state=978bedf3e842b14bf051a7cdbd773cf0 counters=f7a4139145ab6c9fd913f1df79924152",
    ),
    (
        "dev_burgers2d 1e-4",
        "ok=true t=3fe0000000000000 att=16 acc=16 rej=0 reuse=0 jvp=5376 it=5120 solves=128 lsf=0 state=a9fd31b0b823ba712b78f6bea5ddcafb counters=c416d80286fe4d9bca4e062191eb4b7d",
    ),
    (
        "dev_burgers2d 1e-6",
        "ok=true t=3fe0000000000000 att=36 acc=36 rej=0 reuse=0 jvp=12096 it=11520 solves=288 lsf=0 state=7954d94d0ba231d34f4eaacef5a0d4ca counters=1d30480115710fea8ae3e8bc1df2b2f9",
    ),
    (
        "dev_bruss1d 1e-4",
        "ok=true t=4024000000000000 att=61 acc=46 rej=15 reuse=15 jvp=20496 it=19520 solves=488 lsf=0 state=f4af4984caeed5443d29a48c2f8b9e71 counters=bc9e6820cec7aaa882309bac11857d8c",
    ),
    (
        "dev_bruss1d 1e-6",
        "ok=true t=4024000000000000 att=108 acc=95 rej=13 reuse=13 jvp=36288 it=34560 solves=864 lsf=0 state=210b1d54952edd2df11db107bfd1fdbc counters=23095892c3e6f1392691e38de868b991",
    ),
    (
        "v06:dev_adr2d 1e-4",
        "ok=true t=3fb999999999999a att=13 acc=13 rej=0 reuse=0 jvp=4356 it=4148 solves=104 lsf=0 state=ffb3c40e211cbf5135fb5a085ab31e2c counters=e9999ff980c579951a69acc98b3681de",
    ),
    (
        "v06:dev_adr2d 1e-6",
        "ok=true t=3fb999999999999a att=30 acc=30 rej=0 reuse=0 jvp=10068 it=9588 solves=240 lsf=0 state=44e26bdb39cab0f316c2806323562dfb counters=009ba976f808bc6e3353b9a475cc1e4e",
    ),
];

fn recorded(id: &str, rtol: f64) -> &'static str {
    let key = format!("{id} {rtol:e}");
    RECORDED
        .iter()
        .find(|(name, _)| *name == key)
        .unwrap_or_else(|| panic!("no recorded bits for {key}"))
        .1
}

#[test]
fn the_shipped_entries_return_the_bits_recorded_before_the_edit() {
    let print = std::env::var_os("V24_PRINT_BITS").is_some();
    for id in SMALL {
        for rtol in RTOLS {
            let route = line(&shipped(id, rtol, false));
            let route_ls = line(&shipped(id, rtol, true));
            assert_eq!(
                route, route_ls,
                "{id} {rtol:e}: the least-squares workspace changed the run"
            );
            if print {
                println!("V24_BITS (\"{id} {rtol:e}\", \"{route}\"),");
            } else {
                assert_eq!(route, recorded(id, rtol), "{id} {rtol:e}: the bits changed");
            }
        }
    }
}
