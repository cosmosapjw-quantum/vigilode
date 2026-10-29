# Task 4 review (h-hygiene) a5a48f7..d41bf16

Spec compliance: PASS with caveat (checker exit 1 is a known item, not scored against the task).
Code quality: Approved.

Named risks
1. Thread-local counters: no measured region spawns threads. collective_admission_contracts.rs:93-122 measures rhs/jvp/energy/mean loops directly on the test thread; pr101 measured regions (enclose_rhs, one_sided_log_norm_upper, verify_segment, output_enclosure) use no ParallelExecution/rayon (grep of both test files: no spawn/rayon). OK. Thread-local const-init Cell has no dtor, so the allocator cannot recurse. Minor: counters would now silently miss any future spawned-thread allocation; the comment states the assumption.
2. Leja assertion (leja_setup_work_contract.rs:~145-165): available == (observation != PreexistingUnattributed) for all three tables. Not vacuous: when a table is initialized/warm it still demands history (same as the original), and it additionally rejects history on a preexisting table. Residual: if a sibling pre-initializes all three, no history is asserted in-process (Minor). Intent is covered because the sibling at :179 documents the same process-global race.
3. L-0138: parses; all required fields present; UNRECORDED + INCONCLUSIVE allowed; supersedes L-0124 (exists, earlier); both cited commits are ancestors of d248e68; both outputs_sha256 match current file bytes (recomputed); diff removes 0 existing ledger lines (+1 only). Minor: row uses ", " / ": " spacing unlike the compact existing rows; harmless to the schema.
4. [profile.measurement] removal: root profile is release + lto=false + codegen-units=16 (Cargo.toml:48-51) vs crate's 1; opt-level stays 3. No LEDGER row references vigilode-coupled, but the coupled_native/economy timing results were built at codegen-units=1. Documented only in docs/plan/nodes/h-hygiene.md:40, not in the crate README or research/coupled_native_20261020/REPRODUCE.md.

Findings
- Minor: README.md/REPRODUCE.md do not say that measurement builds now use codegen-units=16, so reproductions of recorded timings differ in profile; one-line note recommended. FP results are unaffected by codegen-units.
- Minor: the five coupled-*.yml workflows and run_ci.sh keep --manifest-path/crates/vigilode-coupled/target paths that now resolve to the root target dir; the workflows build from $FROZEN checkouts, so they are frozen artefacts. Left unedited and explained (brief item 4 satisfied).
- Minor: lock diff is +10 lines, vigilode-coupled entry only (Cargo.lock hunk verified in the diff). I did not re-run the 37 tests or the 5-run parallel loop; the report claims 20 runs per binary.
- Minor: report's 'ancestors of this commit' wording for the cited commit is verified true.
