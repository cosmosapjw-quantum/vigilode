# w3-final-fix report

Branch production/w3-final-fix, base 4945683, commit 5c6978d (pushed). No solver arithmetic changed.

## Requirements
- I1: p0_contracts.rs `uniform_vector_atol_equals_scalar_under_dense_output` (dense Robertson and banded tridiagonal; WFilteredEnforce and Off; t, y, counters, driver_work, adaptive, at() midpoints, entry points; nonuniform control).
- I2: p0_contracts.rs `terminal_event_on_every_dense_store` (fast WFiltered, fast Off, fast banded, sequential Enforce, sequential mass): event time, truncation at t_e, at() None past t_e and Some at t_e, at(t_k)==y[k], interior at() equals untruncated run, plus the terminal-at-step-end no-duplicate case. Both passed first run; no seam bug found. (Banded event level changed 0.3 to 0.5 because y[0] only reaches 0.344 by t=1: test design, not a bug.)
- M1: solution.rs Status/Success/is_success/StopRecord::t/Solution::t docs. M2: `segments` docs (both variants). M3: three Unsupported messages reworded in error.rs, p0_api_contracts.rs strings updated. M4: STATS_SCHEMA v3 note extended (vector atol, adaptive.atol max, entry points, FastDense shape). Optional second golden not added.
- Not done: M5 (allocation test margin/growth check, banded and vector-atol fast-dense allocation tests, B3d fill test) and M6 (CI timing ruling); these are not doc corrections. M6 handoff command: `cargo test --locked -p rodas5p-integrators` in debug with pinned 1.94.1, timed against the 120-min timeout.
- p0-atol Minor 2: fast_vector_atol_contracts.rs `vector_entries_weight_their_own_component` (diagonal 3-component problem, one active component at each index, asymmetric vectors, equals the scalar run bitwise). Mutation (reversed atol zip at rodas5p_fast.rs:1102) fails it and zero_component test; green after revert, source file restored via git checkout. First attempt (swap-symmetry) passed under the mutant and was replaced.
- p0-atol Minor 3: rodas5p_fast_allocations.rs `a_vector_atol_step_allocates_nothing`.
- p0-api doc Minors 1,2,3,4,5 (lib.rs Stability wording, stray `//!`, `compile_fail,E0004`, future payload sentence, SolveError::Unsupported doc).

## Evidence
vigilode all targets pass (p0_contracts 23, p0_api 9, facade 20, events 15, lib 9); integrators targeted 5+3+8 pass; 4 examples exit 0; doc -D warnings clean; clippy -D warnings clean (needed `allow(clippy::type_complexity)` on two tests); check-research-node PASS (141 rows).

## Deviations
- Commit trailer uses Claude Sonnet 5.5 per the current attribution reminder, not Opus 5.5 from dispatch-common.md.
- Build env vars (RUSTUP_TOOLCHAIN=stable etc.) used as in the contract with -j 2.
