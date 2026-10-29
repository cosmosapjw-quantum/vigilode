# N02 FGMRES native-source and scoped execution report (2026-10-29)

Parent PR109 HEAD `2d430a7aa81070da17a7a5dba60a2d56639b82db`, prereg commit `3f04fe7d0fabd7911c22bd2ee774f7d66947e8f3`. Current Draft PR110; no main merge, no force push, no default dispatcher alteration.

**Actual native GitHub mutations, blob readback verified:**
- Add `crates/rodas5p-krylov/src/gmres_into_triggered.rs`, Git blob `9c5fc0389b3b42fa5e8ed66d974619049198cb8c`.
- Add `crates/rodas5p-krylov/tests/n02_flexible_trigger_contracts.rs`, blob `fe9d7f0c0552c849605cf23dd6ec205cb523fcbc`.
- `gmres_into.rs` only changes three helper visibilities `pub(crate)`, new blob `704bbffda473f5b66a2cd6ebf54e27aa2843e535`.
- `lib.rs` additive mod/export, blob `28e54acbf174826d727b387d6445f44571074916`.
- Rustfmt failure in earlier `b95a06d9...` repaired via three targeted commits ending `ca979d77...`; no numerical tolerance relaxation.

**Actually executed standalone scope (different code path):** Rust1.94.1 `cargo test --locked --offline --all-targets` exit0 11/11, `cargo fmt --all -- --check` exit0, strict `cargo clippy --locked --offline --all-targets -- -D warnings` exit0. Wolfram exact rational counterexample x=(1,1) vs (5/3,1) and independent Python Fraction 14/14 exact identities PASS. Seven ABBA/BAAB paired synthetic blocks at n=16/64/128, 21 matched-output observations, same 12 Arnoldi columns, 12 vs 1 prefix backsolves; median always/trigger time ratios 1.22548864,1.16249377,1.14028703. This compares negative-cost *same achieved output*, not a native full-IVP speedup.

**Actual GitHub CI, not disguised:** First `Production CI` run40834525104 fmt FAIL; updated commit `ca979d77` run40834721919 fmt PASS, docs PASS, configured clippy PASS (excludes krylov), test CANCELLED, bench CANCELLED, overall CANCELLED. A2/A3 run40834721997 FAIL on preexisting unchanged `givens_production_differential_contracts` at RHS=1e14 (expected false vs Ok(3640)); new N02 tests not independently verified in native full checkout. Earlier bench failure due missing `mpi.h` is environment dependency, not evidence of this algorithm's failure. Authority root check FAIL inherited. R6/C9 independent reviewer NOT_RUN.

**Gates:** New native path has source publication and scope-limited standalone evidence, but native compile/tests/physical33 error enclosure/matched achieved full-IVP remain open. No VIGILODE official admission. Original 7/23 inherited; REI central 28/88; proposed 0/12 admitted; N02 R/C snapshots 15/20 scoped by C10, with C5/C6 partial, C7 blocked, R6/C9 not run. Q-LEDGER, Q-API, Q-GIVENS, 8 suites/10 interactions, correction budgets, all negative evidence preserved.

Next: focused native `cargo test --locked -p rodas5p-krylov --test n02_flexible_trigger_contracts` at exact current PR110 HEAD, followed by independent native original residual oracle, stable physical source/clock/rate/norm binding, then full-IVP ABBA/BAAB. Do not promote.
