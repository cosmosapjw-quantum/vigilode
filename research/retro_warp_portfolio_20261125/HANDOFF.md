# RETRO-WARP 7/7 handoff

Status: IMPLEMENTATION_VERIFIED_SCOPED; HOLD_NOT_RELEASE_QUALIFIED.
Base: 7f4ff231e0256aceb060a09782559ee2021aef3f.
Scope: opt-in, typed research portfolio; no production default, agent task, qualification ledger, or unrelated branch changes.

## Implementation
- Integrators: Portfolio, typed cubic, fourier, temporal, nonlinear, affine, green_endpoint, physical_window adapters, process-local bounded timing history, periodic exploration, capped retries and per-attempt receipts. Generic choose requires caller-owned sound bound.
- Spatial facade: adaptive_stage (original represented matrix, outward residual and strict row-dominance gap), adaptive_endpoint (replay-safe endpoint IVP and caller-owned independent verifier), paired/complex spectral and direct/native choices. Trusted mode excluded from these automatic adapters.
- Exact compatible Green conversion is restricted to coordinate cubic terms and the Green source admission. Nonlinear discrete errors never authorize a physical IVP result. No automatic arbitrary mixed-jet compression or generic DAE/event/dense callback certification.
- All earlier direct APIs and preparation paths remain available.

## Actual evidence
88 related tests passed, 0 failed, 12 ignored in the final three scoped test commands; the new policy timing test was separately executed successfully. Clippy -D warnings passed for both touched crates/tests; formatting and diff checks passed. The 70 regression receipts exercised 29 distinct qualified route types and recorded 5 explicit rejected attempts. These are receipt counts, not every attempted negative case.
Native coarse Hermite certification failed at the stiff test; Green qualified on the same physical node contract after native rejection. This is not a claim that the native trajectory was inaccurate. Spatial wrong-operator spectral candidates failed the full original residual even though mass was preserved; direct fallback qualified. Actual spatial IVP native, paired, complex endpoints qualified against an outward independent exact exponential reference.

Policy timing: Ryzen 9 5900X, pinned CPUs 0-3, 24 calls per batch, five measured batches after warmup. Total ms medians (cold-fixed/adaptive/round-robin):
- Temporal: 4.526784 / 5.736535 / 8.849654; max certified bound 1.906e-15, contract 1e-10.
- Affine dense: 217.452791 / 220.136248 / 243.329762; max certified bound 2.626932e-8, contract 1e-7.
Adaptive selection reduces exploratory cost versus cycling all routes, but does not beat the known best cold prior in these stationary tests. This is same certified output tolerance, not exact achieved-error matching. Individual test timings are diagnostic, not production benchmark claims. See RESULTS.json, policy.log, timing_env.txt, regression.log, spatial.log.

## Reproduce
From this worktree, set CARGO_TARGET_DIR to a suitable local build directory.
cargo test --locked -j 4 -p rodas5p-integrators --release --no-fail-fast --test retro_portfolio_contracts --test retro_nonlinear_contracts --test retro_temporal_contracts --test retro_causal_contracts --test retro_green_contracts --test warp_pp06_contracts --test affine_event_certificate_contracts --test fast_dense_contracts --test c5_source_owned_graph_contract -- --test-threads=1
cargo test --locked -j 4 -p rodas5p-core --release --test warp_cubic_contracts -- --test-threads=1
cargo test --locked -j 4 -p vigilode --release --no-fail-fast --test spatial_portfolio --test spatial_separable -- --test-threads=1
Inspect the separately executed policy timing test and archived command/environment evidence in policy.log and timing_env.txt. Run python3 summarize.py from this research directory to reconstruct RESULTS.json. Full external solver corpus, full workspace qualification, arbitrary time-dependent aperiodic Rust route, cached modal-RHS routing and full offline-table ingestion remain unverified/open.

## Disposition
Publish the research branch only, without force or merge. Keep unique evidence and active worktrees. Early six theory archives are indexed in EARLY_ARCHIVES.tsv; the combined archive is a conversation-side deliverable, not stored in this Git tree. All archived negative results remain negative. Independent native subagent/Superpowers execution not available in this session.
