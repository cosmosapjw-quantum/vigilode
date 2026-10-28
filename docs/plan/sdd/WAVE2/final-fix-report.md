# Wave-2 final fix report

Branch production/w2-final-fix (base 4f92ec3), pushed. Commits:
- 8154810 integrators: Enforce interior estimate uses the A1 zero-scale norm (Important 1)
- d5285bd vigilode: banded doc, no segments without dense output, StepSizeTooSmall wording, banded contract tests (Minor 3, 4, 5, 7)
- 4c04c8b research: L-0140 (Minor 1)

## Items
1. Important 1: crates/rodas5p-integrators/src/dense_output_v2.rs, `rodas5p_dense_interior_error_estimate` now ends in `step_error_wrms` (comment added); `rodas5p_dense_interior_defect_estimate` stays strict with a report-only comment. Test `enforcing_interior_error_survives_a_zero_error_scale_component` in crates/vigilode/tests/p0_contracts.rs (modes WFilteredEnforce and Enforce; Off does not sample).
   - Fail before: "Enforce: estimate unavailable, left: 18, right: 0" (WFilteredEnforce passed, as the review said). Pass after fix.
2. Minor 3: crates/vigilode/src/problem.rs `Problem::banded` doc (both cases: FD/Banded assembled to dense, O(n^3); Dense+band refused); route-table row in crates/vigilode/src/lib.rs.
3. Minor 4: new `integrate_adaptive_dense_observed_with_partial` (dense_output_v2.rs, exported in lib.rs) calls the private impl with `None`; crates/vigilode/src/solve.rs uses it when `!opts.dense_output`. Test `mass_route_without_dense_output_is_the_same_run_without_segments`.
4. Minor 5: crates/vigilode/src/solution.rs, doc and Display of `StepSizeTooSmall`. No test asserted the old string.
5. Minor 7: `banded_provider_on_the_dense_output_route_matches_the_dense_provider` (bit-identical) and `finite_difference_band_on_the_dense_output_route_runs_and_agrees` in p0_contracts.rs.
6. Minor 1: research/LEDGER.jsonl row L-0140 (supersedes L-0139; commit 94c5b0a56fb5d69bc4d7a63a7301b8a359fd4500; inputs report.rs + results_v1.json; output REPORT.md; digests equal at 94c5b0a and HEAD). check-research-node.py --base a5a48f77: PASS (140 rows). unittest tools/test_check_research_node.py: 16 OK.

## Tests
`cargo test -p vigilode -p rodas5p-integrators`: all pass (vigilode: 4 unit, 18 facade_contracts, 17 p0_contracts, 1 doctest). Clippy: no warnings in touched code (the new 10-argument fn got `allow(too_many_arguments)` like its sibling).

## Deviations / concerns
- Without dense_output the mass route now reports `DriverWork::SequentialDense.segments = 0` (was accepted-step count) and `route.entry_point = "integrate_adaptive_dense_observed_with_partial"`. States and counters are unchanged.
- Without dense_output only [t0, t_end] is stored (existing ruling), so the bit-identity test compares final state, counters and step counts, against a dense_output + InteriorError::Off run.
- `cargo fmt --all -- --check` fails only in crates/rodas5p-integrators/tests/production_audit_packet_curve.rs (pre-existing, untouched, out of scope).
- tools/check-authority-refs.py FAILs on many pre-existing citations; not in scope. 94c5b0a is an ancestor of this pushed branch.
