# BASS/rei NCP C1 work-precision campaign (harness)

The preregistration is `research/bassrei_ncp_wp_c1_20261022/PREREG_DRAFT.md`, and the gates are in
`harness/analyze.py`.

## Start (one command, from a checkout containing EXEC)

    bash external_audits/vigilode_bassrei_ncp_c1_20261022/launch_from_local.sh EXEC_COMMIT

The command:

1. refuses unless EXEC is pushed and contains the node's PREREGISTRATION.md, then runs the preflight gates
   (cpuset 8–63, no live C1 run or heavy foreign process, load < 4, MemAvailable ≥ 48 GiB, `/root` free
   ≥ 20 GiB + depot size). A host-key mismatch, unreachability and each refusal reason are printed with
   distinct exit codes;
2. archives the source subset at EXEC with its `git ls-tree`;
3. uploads it to `/root/vigilode-bassrei-ncp-c1-<UTC>/`;
4. starts `run_campaign.sh` truly detached (setsid nohup, affinity 8–63) and confirms its PID;
5. waits up to 125 min with short ssh polls (failures are reported), then runs `fetch_results.sh`, which
   copies `OUT.tar.gz` back to `~/vigilode-campaign/bassrei-ncp-c1-<UTC>/ncp/` and verifies every
   sha256;
6. after that, run `harness/import_results.py <copy> research/bassrei_ncp_wp_c1_20261022` to import the
   results into the research node for the ledger.

`--detach` returns right after the start. Later, run:

    bash external_audits/vigilode_bassrei_ncp_c1_20261022/fetch_results.sh NCP_DIR LOCAL_DIR

## Files

| file | role |
|---|---|
| `run_campaign.sh` | NCP driver: phases, clipped deadlines, watchdog at 112 min, kill of every registered child, EXIT-trap finalize ≤ 8 min (analysis, hashes, tarball) |
| `setup_ncp.sh` | source verification, runner build; diffsol build (8 min cap) and private Julia depot copy + load (12 min cap) in parallel |
| `harness/scenarios.py` | cases, arms (applicability a priori), requests, atol rule, observables, B_app/B_src metrics |
| `harness/runners.py` | pinned child processes (wait4 receipts), Julia server client |
| `harness/py_worker.py` | scipy VODE (MF10/21/22/24/25, with a runtime check of the user-Jacobian layout), DOP853, the DOP853 source configuration, Radau reference |
| `harness/julia/` | `kernels.jl` (bitwise ports of the P02/P03/P04 RHS and f_t), `colorjac.jl` (coloured dual-number Jacobian), `minijson.jl`, `solver.jl` (`--server` / `--parity`) |
| `harness/diffsol-adapter/` | standalone diffsol 0.17.1 crate on the adapter kernels by `#[path]` |
| `harness/parity.py` | RHS parity of every adapter against the runner probes (<= 1e-13) |
| `harness/ladder.py` | references and work-precision ladders (PLAN.json, LADDER.jsonl) |
| `harness/vg.py` | allocation and cross-core bit-identity (VG.jsonl) |
| `harness/abba.py` | accuracy-matched paired cost blocks (ABBA.jsonl; diagnostic_only) |
| `harness/analyze.py` | evaluation, matched rungs, gates G1–G7, verdict PASS/FAIL/INVALID, VERDICT.json, REPORT.md |
| `harness/import_results.py` | preregistered import of the verified copy into the research node (results.json, campaign/, IMPORT_MANIFEST.json) |
| `harness/verify_source.py`, `merge_locks.py`, `telemetry.py`, `common.py` | provenance, diffsol lock seed, host telemetry, shared helpers |

## Rules kept

- No Cargo.toml or Cargo.lock edits. The diffsol crate has its own lock under `CAMP/`.
- Rust 1.94.1, `--locked --offline` for the runners.
- The Julia depot is copied, never stacked: every Julia process gets the private copy as its only depot.
- Campaign outputs go only to the campaign directory on NCP. Cargo updates its own home cache, and Julia
  temp files go to CAMP/tmp.
- Cores 0–7 are left free: the campaign runs under `taskset -c 8-63`, and BLAS uses 1 thread.
- Nothing runs past the 120-min cap: timeouts are clipped, and children are registered and killed.
- Partial results are preserved on abort.
- Timing is diagnostic_only.
