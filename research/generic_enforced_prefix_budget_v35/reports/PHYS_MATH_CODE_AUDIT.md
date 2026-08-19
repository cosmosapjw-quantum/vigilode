# v3.5 PHYS–MATH–CODE Audit

## Equation-to-code map
- frozen cap calculation: `enforced_prefix_jvp_cap` in `g4_s5b0_regime_atlas.rs`;
- low-level guard: budgeted operator wrapper in `exponential.rs`;
- budgeted prefix entry point: `pexprb54s4_level2_prefix_with_tolerance_scaled_telemetry_jvp_budget`;
- read-only family runner: `run_g4_s5b0_enforced_prefix_budget_family`;
- CLI: `generic-enforced-prefix-budget`.

## Load-bearing code checks
1. The cap is checked before the inner JVP operator call. Unit tests show cap=1 results in exactly one underlying call and cap=0 results in zero calls.
2. A generous cap reproduces the original unbudgeted level-2 prefix report exactly.
3. Exhaustion returns partial `WorkCounters`; the denied cap+1 call is absent from both physical execution and counters.
4. The runner charges the completed prefix work, emits `budget-exhausted`, omits zeta/full-E audit, and leaves R-JF as the committed trajectory.
5. Consumed-profile replay preserved trajectory summaries, event keys, and stable pre-existing row fields exactly on N=96/192/256/384.
6. BDF, Radau, and `Cargo.lock` hashes remain frozen.

## Replay evidence
- N=96: 25 events, 0 exhaustion, max 31 JVP.
- N=192: 23 events, 0 exhaustion, max 67 JVP.
- N=256: 23 events, 0 exhaustion, max 43 JVP.
- N=384 replay: 27 events, 1 exhaustion, max 80 JVP; historical 109-JVP semilinear event now stops exactly at 80.
- N=320 fresh: 29 events, 1 exhaustion, max 80 JVP.
- cap violations: 0 across all these rows.

## Test status
Focused v3.5 contracts, CLI, level-2 resumability, early-defect and tolerance-scaled regressions pass; `cargo fmt` and Clippy with `-D warnings` pass. A workspace-wide all-target run was attempted but the outer sandbox timed out during the CLI-contract target after three completed result blocks and no observed failure, so workspace-wide PASS is not claimed.

## Remaining code risk
The existing `audit_full_e_work` is offline evidence only. The next node must not silently treat that audit continuation as free runtime work. Any runtime full-E shadow must resume the retained prefix without recomputation and add its incremental work to the speculative ledger while leaving R-JF state/controller/output untouched.

## Verdict
**PASS for v3.5 research-node implementation; release-wide regression remains incomplete.**
