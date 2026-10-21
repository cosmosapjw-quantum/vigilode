VERDICT: ALL ADDRESSED

Scoped re-review of 4f92ec3..4c04c8b (worktree /home/claude/wt/w2-final-fix). Controller rulings (Minor 2, Minor 6, fmt failure) not judged.

## Findings

- Important 1: ADDRESSED. crates/rodas5p-integrators/src/dense_output_v2.rs:1162-1164 `rodas5p_dense_interior_error_estimate` now ends in `step_error_wrms` (A1 zero-scale rule; bit-identical to `wrms` on a positive scale). The report-only defect estimate keeps the strict norm, with a comment (:1243-1245). Test `enforcing_interior_error_survives_a_zero_error_scale_component` (crates/vigilode/tests/p0_contracts.rs:1026-1048) covers WFilteredEnforce and Enforce with atol=0 and asserts unavailable==0, interior_rejections==0 and samples==accepted_steps. It passes.
- Minor 1: ADDRESSED. research/LEDGER.jsonl L-0140 (last row): supersedes "L-0139", commit 94c5b0a56fb5d69bc4d7a63a7301b8a359fd4500 (equals `git rev-parse 94c5b0a`).
  - Inputs report.rs 16e17c41... and results_v1.json 1c8c9a4c... and output REPORT.md 6b5380df... match `sha256sum` at HEAD.
  - `check-research-node.py --base a5a48f77...` prints PASS (140 ledger rows).
- Minor 3: ADDRESSED. crates/vigilode/src/problem.rs:162-169 documents the O(n^3) dense assembly for FD band and Jacobian::Banded, and the refusal of a Dense provider with a band, on the mass / dense_output route. Route-table row in crates/vigilode/src/lib.rs:47.
- Minor 4: ADDRESSED. New `integrate_adaptive_dense_observed_with_partial` calls the impl with `None` (dense_output_v2.rs:1677-1711). The facade uses it when `!opts.dense_output` (crates/vigilode/src/solve.rs, around :250-266). Test `mass_route_without_dense_output_is_the_same_run_without_segments` (p0_contracts.rs:1053-1077): final state, counters and step counts equal the Off dense run. It passes.
- Minor 5: ADDRESSED. crates/vigilode/src/solution.rs:43-48 (doc) and :95-97 (Display) now cover the case of no rejection after the last accepted step.
- Minor 7: ADDRESSED. `banded_provider_on_the_dense_output_route_matches_the_dense_provider` (bit-identical) and `finite_difference_band_on_the_dense_output_route_runs_and_agrees` in p0_contracts.rs. Both pass.

## Implementer concern 1 (segments = 0 and the entry_point string on the mass route without dense output)

1. `segments`: this is not a counter in the strong sense. Its doc is "Accepted steps whose continuous extension was kept" (solution.rs:278-279), so 0 when nothing is kept is the documented meaning. Before the fix it reported the accepted-step count of segments that were then discarded, which contradicted the doc. The only in-tree consumer is p0_contracts.rs: `sequential_work` at :109-111 and the assertions at :185-190 (all with dense_output = true, still the segment count) and the new test. Neither the bench crate nor any schema reads it. No contract break. The accepted-step count remains in `stats.accepted_steps`.
2. `entry_point`: grep of crates/vigilode-bench, docs and schemas finds no reader. The only hits are the facade's own sources and p0_contracts.rs:182-183, which asserts the old string for the dense-output run, and that string is unchanged. The new string appears only when `dense_output` is false. The route table in lib.rs documents both strings. No test, schema or bench contract break. `cargo check --locked` in crates/vigilode-bench (target /home/claude/targets/e1-bench) succeeds.
3. `integrate_adaptive_dense_observed_with_partial`: acceptable as `pub`. It is a sibling of the already-public `..._with_segments`, exported the same way in lib.rs, and the facade (a separate crate) must call it, so `pub(crate)` is impossible. `doc(hidden)` is not needed, because the entry_point string, lib.rs and the docs name it as a public driver. It is documented and takes the same arguments as its sibling.

## Verification run

- `cargo test -j 2 --locked -p vigilode --test p0_contracts`: 17 passed, 0 failed.
- `cargo check` in crates/vigilode-bench: OK. The only warnings are the pre-existing dead-code ones in rodas5p-integrators.

## New defects

None blocking. One cosmetic nit: the `SequentialDense` variant doc at solution.rs:256-258 names only `..._with_segments` as the entry point, while the route table in lib.rs mentions both. This is harmless.
