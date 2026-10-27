# Preregistration: same-checkpoint field check, return sentinel and matrix-free provider cost

Node `adapter_checkpoint_followup_20261027`, branch `research/adapter-checkpoint-followup-20261027`, from the
fixed packet commit `EXPECTED_COMMIT = dda8d2f16758ce7d3369ad249344b2ac8bbd08ef` (tree `782114b4`,
`research/adapter-integration-20261027`, fetched once on 2026-10-27 16:02 KST; no SHA was supplied). It
follows `research/adapter_integration_20261027/NEXT_STEPS.json` items `return-evidence-renewal` (1),
`persistent-implicit-session` (2) and `provider-aware-economics` (3). Callback ceilings, implicit window
coalescing and the growth-only warm start of the packet are reused, not reimplemented.

**Claim boundary.** Deterministic work counters, states and counted allocator events are the authority.
Every wall time is `"timing_authority": "diagnostic_only"`. No tolerance, hysteresis ratio, entry/return
threshold or accuracy cap is changed. The default derivative permission, `AutomaticOde::solve` and every
switching factory stay unchanged; the new sentinel is opt-in. Local, single solver thread, no NCP, no
holdout profile, no `Cargo.toml`/`Cargo.lock` change. The packet's preserved results (default gyromotion
accuracy failure, relaxation 12.2 % regression, unqualified transition timing) are not re-labelled.

## Reproduction of the packet on this host (before this registration)

AMD Ryzen 9 5900X (24 logical CPUs), Rust 1.94.1, `--locked --offline`, `Cargo.lock` sha256
`efb37907…` identical to `e8572eac`. `tools/verify_package.py`: PASS (222 files). `cargo test -p vigilode
--lib --tests`: 32 suites, 263 passed (47 s). Measurement build and `rhs-only-availability`: PASS (exact
native work/state match, error 3.37e-9). All 26 packet pairs re-run with `taskset -c 11`: every one of the
34 case/arm records (work, state, segments) is bit-identical to `benchmark/raw_r2`. Timing classes differ
from the Xeon packet in 12 cells (diagnostic only; for example relaxation regression 1.098 [1.071, 1.126]
reproduced; stiff-forced evidence→adaptive 0.905 [0.862, 0.997] not qualified as an improvement here).

## Exploration already performed (disclosed; not gated)

E1. **Same-checkpoint trace** (`examples/adapter_checkpoint_trace.rs`, `tools/run_checkpoint_traces.py`).
The packet raw records select 12 decision points (entry pilots and return rechecks of case5, stiff-forced,
relaxation, transition-39). The switching rerun is bit-identical to each raw record. DOPRI5 is stepped one
attempt at a time in the native session; RODAS5P by deterministic attempt-prefix runs. Per attempt: proposed
and trial `h`, landing clip, accept/reject reason (counter category), RHS/JVP/Jacobian/linear-solve/
factorisation counters, allocations, and the true local error of accepted steps against two tight references.
Findings: every discarded explicit return in the registered fixtures lost correctly (e.g. transition-39
[0.875, 1]: DOPRI 37 045 work units vs RODAS 4 606). The largest missed decision is an **entry** miss:
transition-39 spends 24 858 of 48 386 work units (51 %) in an explicit stretch 0.34–0.4676 while six entry
pilots lose on ~7e-4 prefixes; at checkpoint 0.404521 RODAS loses the prefix (245 vs 193) but wins the
window to 0.5 (7 056 vs 28 027; diagnostic time 1.76 ms vs ~4.0 ms). The matrix-free facade route costs
about 12 µs and ~90 allocator events per attempt at n = 2 (dense route: ~1 µs, 0).

E2. **Allocation sites** (heaptrack). ~8 events per GMRES least-squares solve in
`rodas5p_krylov::small::least_squares` (faer `ColPivQr::new`, `Mat::from_fn`, `MemBuffer::new`), plus 3 per
relinearisation (`linearize_matrix_free`, `eval_partial_t`). SPD04 (`L-0087`) and SP02 (`L-0104`) already
established a bitwise-identical reused `LeastSquaresWorkspace`; the routed facade never enabled it.

E3. **Session-preservation estimate.** One uninterrupted RODAS solve over each implicit tenure versus the
adapter's split batches (raw starting step): transition-39 0.5 % of tenure work, case5 20.0 % (196 units),
stiff-forced 35.7 % (90 units), relaxation −4.8 % (uninterrupted is worse). This is a heuristic point estimate
under the same tenure decisions, **not** a bound: step control is not monotone under landing clips, the batches
start from `implicit_seed`, and a real session would change measured rates and hence the recheck schedule
(the relaxation sign shows the non-monotonicity). It is small in absolute terms against E1's entry miss.

E4. **Return false negatives.** On a new fixture `stale-return` (stiff relaxation to a smooth path, then a
nonstiff 150 rad/s oscillation; exact solution y = g(t)), the adaptive policy keeps RODAS on [0.5, 1]. Three
screens can each block a current explicit win: the previous-window rate ceiling, the fixed
`probe_attempt_budget = 16` over a whole window, and an incomplete same-window implicit comparator, which is
treated as "retain implicit" even when its charged work already exceeds the explicit work.

## Changes (frozen at the execution commit)

C1. `SwitchingConfig::return_sentinel_interval` (default 0 everywhere, including `adaptive_evidence`). Every
`k`-th due recheck of an implicit tenure (the first included) runs the implicit window first, as the
productive advance, then an explicit return over the same checkpoint and window bounded by
`floor(return_cost_ratio · W_implicit) + 1` RHS units (checked after each attempt; at most one further
attempt) instead of the previous rate and `probe_attempt_budget`. The return is committed only when
`compare_qualified_costs` prefers it; otherwise the implicit window is committed and the explicit trial is
charged and discarded. A fatal or incomplete implicit window stops exactly as an ordinary implicit batch; after
a fatal (or record-less) explicit trial the already completed implicit window is committed, the trial is never
published, and the solve stops. All work is charged. Worst case per losing sentinel: `(1 + r) W + 1` units plus
one DOPRI attempt (`W` in mixed work units, Jacobian builds weighted `n`); its attempts draw on the global
`max_steps` without a separate cap. A sentinel window whose budget leaves no explicit trial is recorded as a
completed implicit window, not as a comparison.

C2. Incomplete-comparator lower bound (default path). A nonfatal incomplete same-window implicit comparator's
charged work is a lower bound on completing that window, so it is passed as admissible incumbent cost; an
explicit return below `return_cost_ratio` times that bound wins (decision "explicit return passed same-window
lower-bound comparison"). The bound holds for the deterministic continuation of the same replay (replay-safe
callbacks, monotone counters, a budget stop is a prefix); it does not bound the coalesced batch that would
otherwise follow. No registered packet cell takes this path.

C3. `RoutingOptions::krylov_ls_workspace` (default false: routed default and record unchanged); the vigilode
facade sets it for its routed call, so the matrix-free route uses the SPD04 workspace. Numerics are bitwise
the legacy driver's; `driver_work.krylov_workspace` records `krylov-ws-ls-workspace-v1`.

C4. Diagnostics only: `examples/adapter_checkpoint_trace.rs`, fixture `stale-return` and arms
`adaptive-sentinel` and `adaptive-probe-unbudgeted` (control isolating the rate screen) in
`examples/adapter_integration_pilot.rs`. Contracts: `tests/return_sentinel_contracts.rs` (7 tests, including
a negative control where the existing screen misses the win and a `k = 2` alternation/restart contract), and a
bitwise legacy-parity assertion added to `facade_contracts::matrix_free_route_records_state_reuses_and_policy_ids`.

Pre-execution review (single fix loop). The registered reviewer route `codex gpt-6-astra` (ultra) did not run:
the first call blocked on an open stdin and timed out (exit 124, no output), the second stopped at the codex
usage limit (exit 1). The AGENTS.md cross-model reviewer `agy --model claude-opus-5-5-high` (Claude Opus 5.5)
reviewed the diff read-only: 0 BLOCKER, 0 MAJOR, 8 MINOR, 5 NOTE. Applied before this commit: re-arm anchor
equal to the ordinary path, fatal/record-less explicit trial keeps the completed implicit window, honest
labels for budget-only and lower-bound decisions, the `k = 2` test, and the E3/C1/C2 wording above. Kept and
documented: no separate attempt cap on the sentinel trial, `is_probe = true` on committed sentinel windows,
the public `RoutingOptions` field addition, the unchanged `route.entry_point` string.

## Registered measurement (M1–M5)

Base binaries: `adapter_integration_pilot` built at `dda8d2f1` (sha256 `6553a8b9…`), and
`adapter_checkpoint_trace` built from a detached `dda8d2f1` worktree with only this node's trace example
copied in (sha256 `90930cb1…`). Command: `bash research/adapter_checkpoint_followup_20261027/tools/run_followup.sh
OUT BASE_TRACE BASE_PILOT` from the execution commit; `OUT` is a new directory. Timed cells run serially
with `taskset -c 11`, never overlapping compilation.

- M1 identity: all 26 packet pairs, one block, candidate pilot binary; records only.
- M2 allocations: one facade matrix-free solve (`transition-39`, `case5`), 10/20/40 attempts, warm call.
- M3 timed pairs, 5 ABBA/BAAB blocks, packet qualification rules unchanged (`analyze_pairs.analyze`):
  stale-return (adaptive vs adaptive-sentinel; adaptive-sentinel vs dopri; vs rodas), sentinel overhead
  (adaptive vs adaptive-sentinel on case5, stiff-forced, relaxation, transition-39), and the matrix-free
  affected selector/direct pairs (case5 adaptive vs rodas, vs rodas-dense, rodas vs rodas-dense;
  transition-39/79 adaptive vs rodas; rotation and gyro-tight adaptive vs dopri).
- M4 cross-binary (diagnostic): base vs candidate pilot, 6 alternating processes per cell, on case5 rodas,
  transition-39 rodas/adaptive, rotation adaptive, gyro-tight adaptive.
- M5 traces: the E1 decision points re-traced with the candidate trace binary.

## Gates

- G1 identity: every one of the 52 packet pair records (26 pairs × 2 arms) equals `raw_r2` after the single
  normalisation `krylov-ws-legacy-v1 → krylov-ws-ls-workspace-v1`.
- G2 allocations: facade matrix-free marginal allocator events per attempt ≤ 0.10 × base on both fixtures.
- G3 stale-return: adaptive-sentinel total work units < adaptive total, both pass the 1e-5 endpoint cap.
- G4 sentinel accuracy: adaptive-sentinel passes its registered endpoint cap on case5, stiff-forced,
  relaxation and transition-39 (its work ratio is reported, not gated; the sentinel stays opt-in).
- G5 tests: `cargo test -p vigilode --lib --tests`, the two routing contract files of `rodas5p-integrators`
  and `rodas5p-krylov --test spd04_ls_workspace` all pass with sources unchanged during the command.

PASS iff G1–G5 hold; otherwise FAIL with the failing item recorded. Timing classes (M3, M4) never change the
verdict. Not claimed: phase or intermediate-time accuracy, a universal sentinel benefit, a complete RODAS
session, a dense provider for callers that supplied only JVP, or any parallel speed-up.
