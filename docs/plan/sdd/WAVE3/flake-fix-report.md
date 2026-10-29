# Wave 3 flake fixes (node w3-flakes)

Branch production/w3-flakes, base 5c6978d. Test-isolation only; no change under crates/*/src.

## Fixes
1. crates/rodas5p-integrators/tests/production_audit_bound_collective.rs
   Added a thread-local `TRACK` flag and `count_event()` (same pattern as krylov_workspace_driver_contract.rs).
   The allocator calls `count_event()` in alloc/alloc_zeroed/realloc. `TRACK` is set only around the two measured
   loops in `warmed_interval_rhs_and_log_norm_make_no_heap_allocations`. The measured work and the
   `(0, 0)` assertion are unchanged. TEST_LOCK is kept.
2. crates/rodas5p-core/tests/leja_setup_work_contract.rs
   Source reading: `scalar_proxy_units` is charged only inside the static-table initializers
   (leja_action.rs:160, :182 points; :620, :645 suffix bounds; :996 node bounds). Coefficient-table
   preparation charges interval kernels and allocation requests but no scalar proxies, and a table
   already initialized by another test charges 0 (leja_setup_work.rs unit tests ~:724).
   So `> 0` was true only if this test happened to initialize a table first.
   Replaced `assert!(scalar_proxy_units() > 0)` with
   `assert_eq!(scalar_proxy_units() > 0, any of the three observations == InitializedByThisCall)`.
   Choice: approach (a). The new assertion is an exact iff, so it is stronger than the old one in the
   cold case and correct in the warm case. The coefficient-table work cannot be used as the positive
   witness because it contributes zero scalar proxies. Approach (b) was not needed; no assertion was weakened.

## Measured failure rates
Before (given, base 5c6978d, 2-core host):
- bound_collective: 3/40 parallel, 0/10 serial
- leja_setup_work_contract: 30/40 parallel, 0/10 serial
After (looped built binaries):
- bound_collective: 0/60 parallel, 0/10 --test-threads=1
- leja_setup_work_contract: 0/60 parallel, 0/10 --test-threads=1

## Other acceptance
- `cargo test -p rodas5p-core --locked -j 2`: all pass.
- bound_collective binary: 5 passed.
- rustfmt --edition 2024 run on both touched files.
- check-research-node.py: PASS (141 ledger rows).

## Deviations / open questions
None. The brief's TDD requirement was satisfied by the user-supplied failure evidence; I did not re-measure the
before rates.
