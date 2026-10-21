# NCP verification and measurement handoff

Run these commands in a **clean checkout of the published integration commit** on the intended Linux benchmark host. No development, lockfile update, package installation, cloud provisioning, or solver tuning is part of this handoff. `SUMMARY.json` is an execution receipt; only the separate promotion reader evaluates numerical evidence. Broad production and speed promotion remain HOLD unless their stated evidence obligations are satisfied.

The checked-in `NCP_CONFIG.json` binds Rust/Cargo **1.94.1**, both active Cargo locks, `rust-toolchain.toml`, and the strict N02 independent-reference corpus by SHA-256. The N02 protocol uses the frozen grid, tolerances and reference refinements. The comparison driver runs all polynomial cases p4/p16/p64 with protected/Krylov/Leja/Laguerre/Chebyshev/router arms; homotopy h3/h8/h16 direct/continuation; and actual-k k2/k4 direct/continuation. Every comparison has 2 warmups and 21 recorded repetitions at 1/2/4/8 threads. Arms rotate deterministically within each repetition; failures and refusals are retained. The added bound-client cases cover diagonal bounded/resolvent/diagonal arms, sealed PR101 provider/replay global-curve budgets, and uniform/optimized allocation witnesses. There are 3,220 comparison and bound-client rows plus one multi-arm work-precision result (280 warmups, 2,940 measurements).

## Commands for Codex 6 Luna low

Prepare the host environment outside this task: the exact Rust toolchain, offline Cargo caches/vendor matching both locks, GNU time at `/usr/bin/time`, `taskset`, GCC/GFortran/binutils, BLAS, SUNDIALS headers and libraries, and Python pytest/NumPy/SciPy/mpmath must already be present. The runner reports missing prerequisites and performs no installation. Its native preflight compiles and runs a small SUNDIALS link probe outside the checkout. No credentials or NCP account connection are used.

Preview only (versions, source identity and planned commands; no Cargo build, solver measurement or native compile):

```sh
python3 tools/run_ncp_reaudit.py --mode dry-run --output ../vigilode-ncp-plan
```

One command performs sequential verification, release builds and the full frozen benchmark protocol:

```sh
python3 tools/run_ncp_reaudit.py --mode benchmark --output ../vigilode-ncp-run
```

Alternatively, first stop after verification, then resume the exact same environment and host to measure:

```sh
python3 tools/run_ncp_reaudit.py --mode verify-only --output ../vigilode-ncp-run
python3 tools/run_ncp_reaudit.py --mode benchmark --output ../vigilode-ncp-run --resume
```

`--cpus auto` is the default and chooses the first eight allowed **logical CPUs**. To use a host-approved set, append e.g. `--cpus 8-15` to both verification and benchmark commands; that example is valid only if those CPUs are actually allowed. The selected CPU list, SMT sibling lists, host boot identity, thread environment and build flags are recorded. Pinning alone is not evidence of physical-core reservation, exclusive allocation, fixed clock frequency, or an uncontended host.

Read the recorded evidence with the gate, using the same checked-out commit:

```sh
python3 tools/production_promotion_gate.py --summary ../vigilode-ncp-run/SUMMARY.json --corpus crates/vigilode-bench/protocols/n02_independent_reference_v1.json --expected-commit "$(git rev-parse HEAD)" --out ../vigilode-ncp-run/GATE.json
```

Do not change source, config, tolerances, caps, CPU allocation or environment to turn an existing failed campaign green. `--resume` accepts only the exact source/config/tool/package/environment/host-boot identity, verifies stored artifact hashes, and reuses completed receipts. Release binaries are hashed in their build receipts. The resolved loader/shared-library files reported by `ldd` are hashed at build and checked before/after each invocation and on resume; a changed or missing runtime identity is rejected. This covers declared dynamic dependencies, not arbitrary future `dlopen` or external data assets. A failed receipt is preserved and remains failed; it is not silently retried. Correcting a prerequisite or deliberately changing the scientific protocol requires a fresh output directory, with the previous campaign retained. A dry-run output is therefore separate from the execution output.

## Receipts and boundaries

- `PLAN.json`: exact source commit/tree and all-tracked-file closure digest, policy digest, config, environment, tools, host, command matrix. Relevant Cargo user configuration is hashed. Tracked report symlinks bind their link text, resolved in-repository regular-file path, and target bytes; external, broken and cyclic targets are rejected. No full environment/credential dump is made.
- `PREFLIGHT.json`: tool/package observations and native probe results. Dirty source or missing dependencies returns HOLD/exit 2.
- `SUMMARY.json`: required validation step IDs and hashed receipts; all 3,221 required benchmark IDs and their raw paths, hashes, per-process receipt hashes, observed status and HOLD reasons.
- `runs/<id>/RECEIPT.json`, `stdout.log`, `stderr.log`, `gnu_time.txt`: complete attempted command and evidence. Commands use `--locked --offline -j1`; thread controls are explicit. Tests/builds, benchmark workspace, and Clippy use separate output-owned Cargo target directories. Native test runners explicitly use one test thread; the workspace matrix runs unit/integration tests and separate doctests, without running historical benchmark executables. Strict Clippy covers the existing production `vigilode` and `rodas5p-core` packages with `--no-deps`; this is not a claim that every legacy crate is warning-free. Clippy's separation avoids reusing the artifact state implicated in an earlier E0463 observation; that historical cause was not established conclusively.
- `BENCHMARK_CASES.json`: incremental case index, preserved when a later step fails. `WORK_PRECISION.json`: unmodified native work-precision output, including reference and failure data.

The driver `elapsed_ns` measures the solver transaction and certificate with the documented exclusions. The outer `process_elapsed_ns` also includes process/threadpool startup, JSON serialization, process launcher and monitor cleanup. These are different timing scopes and must not be substituted for each other. Neither an observed wall time nor a native selected-event count is a proved scheduler span or a complete FLOP count.

Memory is the positive observed maximum from GNU time and sampled `/proc` process trees. It is **not a complete process-tree upper bound**. Unknown RSS stays `None/UNAVAILABLE` and produces HOLD. The configured 8-GiB threshold is an admission check on observed RSS, not an OS-enforced memory limit. Work-precision RSS belongs to the entire multi-arm process and cannot establish per-arm memory use. CPU busy/steal samples and cgroup OOM deltas are retained with their actual scopes; missing or shared-cgroup information is not invented.

Numerical refusal, changed input identity/budget, nonfinite or missing bound, original-budget violation, wrong stage/endpoint scope, missing raw output, timeout, affinity escape, source change, or unknown/excess observed RSS keeps that case on HOLD. Actual-k certifies a stage functional only; allocation is an algebraic schedule witness with zero executed ODE steps. Only the sealed PR101 global-budget case may report `BOUND_MODEL_FULL_CURVE`, within that model and its private provider authority. Endpoint certificates do not certify the full trajectory. A successful executable exit does not establish same-accuracy speed improvement, global accuracy, or production readiness.

The runner's default limits are explicit: 4 hours per verification/build step, 120 seconds per comparison process, 6 hours for work precision, and 24 hours of benchmark wall time per invocation. A timeout preserves attempted work. Resuming uses the retained receipts and a new invocation wall-time allowance; it does not erase previously failed measurements.

## Validation performed here

The harness contracts exercise source mutation, untracked input, unsafe output paths, CPU admission, strict numerical scope/budget handling, nonfinite data, unknown/excess memory, affinity/OOM refusal, distinct Clippy targets, and exact resume/artifact identity. They run fake child-process results and temporary Git repositories. No NCP connection, cloud expenditure, native comparison campaign, or solver timing was executed for these harness tests. The earlier agent-local Python snapshot passed **139 tests**, with **3 skips** for unavailable frozen Hairer libraries; this included **27 harness contracts**. This historical count is preserved, not the final combined checkout count. The authoritative final Rust/doctest/Python counts and source identity are in `EXECUTION.json`. The separate machine-readable `NCP_IMPLEMENTATION.json` binds exact file hashes and logs. The actual dry run correctly returned HOLD for dirty source, missing GFortran, missing GNU time, and insufficient observed memory for the configured 8-GiB admission requirement. An earlier overly strict symlink policy refusal and the earlier 138-test receipt are retained; 15 legitimate tracked report links now bind their in-repository targets. No skipped native result is counted as a pass.

## Disk capacity and retained failed attempts

Prepare sufficient free disk space for separate native, benchmark and Clippy target directories before beginning a campaign. No universal GiB requirement is asserted: profiles, target counts, debug information, toolchain and caches affect storage. During this audit, a 32-GiB filesystem containing prior target directories filled while linking; the runner then lost its progress write to ENOSPC. The failed linker output and zero-byte progress artifact were retained. Only inactive historical Cargo targets were cleaned under root control; the interrupted facade and unrun steps were recovered with unchanged source. A subsequent scoped Clippy lint prompted an equivalent `match` to `if let` rewrite and a new complete source-bound matrix. These attempts remain separate under `evidence/final_02`, `final_03`, and `final_04`.

On NCP, preserve ENOSPC/failure evidence and treat the attempt as HOLD. Do not clean or change active build/runtime artifacts during a benchmark campaign. Fix host capacity outside the campaign, retain the failed output, and use the documented source/identity policy for a fresh attempt.

The completed final uniform-source matrix is `evidence/final_04/EXECUTION.json`: **298 native tests, 4 doctests and 142 Python tests passed**, with 5 native measurement/kill-gate tests ignored and 3 Python tests skipped. All 19 commands, including examples, formatting and scoped Clippy, exited successfully; the before/after 890-file source manifest is identical. Ignored/skipped work is explicitly excluded from the pass counts. This supersedes the earlier agent-local snapshot for final checkout validation; it does not supply missing NCP measurements.
