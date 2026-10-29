$physmath-research-loop $scispace $exa $wolfram

# VigilODE PREFIX01 -> native original-residual GMRES prefix handoff

PROJECT=cosmosapjw-quantum/vigilode
BASE_DRAFT_PR=104
BASE_DELIVERY_SHA=10854bc0bff81b6d3cf5cb0a44520ed9165c205f
BASE_EXECUTED_NATIVE_SHA=0a2a8eb9b9b0b775561f42f42cf36b1ef6e96f4b
SIBLING_PR103=1845e1d8bdfda222bf77637e8d38d5076743f238
SIBLING_PR102=8357faa5b42724a8be24609d4d2f16aec8a6eb3a
SOURCE_OWNER_BASS=cc602e209e7deeb511ab2a1f3b2268d55206c43c
UNIT=VIGILODE-PREFIX01
STATUS=PASS_SCOPED_STANDALONE_PENDING_NATIVE_AND_INDEPENDENT_REVIEW
WORKFLOW=18/20 (R6,C9 independent review NOT_RUN)
ORIGINAL_DAG=7/23 inherited_scoped; 34 edges; delta0
CENTRAL_REI=28/88 canonical; delta0
AUXILIARY_PREFIX01=0/1 pending independent/native admission
PRODUCTION=HOLD

Read this package's THEORY_KO.md, REPORT_KO.md, STAGE_LEDGER.json, EVIDENCE_MANIFEST.json, code src/lib.rs/tests/contracts.rs, all saved CSV, Fraction oracle and original first-failure logs. Verify package SHA/member hashes. Re-read current PR104/PR103 and synchronized-production sibling head before any merge.

New result: weighted-infinity μ(J)≤m and σ≥0, σm<1 imply ||(I−σJ)^−1||≤1/(1−σm). Outward original residual rho implies stage error≤rho/(1−σm). Givens projected residual is only a trigger. The standalone Rust1.94.1 point-matrix implementation uses 2-pass MGS, incremental Givens, transactional output, outward interval arithmetic and FP probes; 9 Rust tests PASS, strict Clippy PASS, 60 bit records×4 exact Fraction inclusions PASS. Synthetic 16/64/128D selective prefix vs end-only at same budget1e−7 median 1.2559/1.1730/1.1902; outputs have different achieved errors. Matched-output always vs selective trigger is bit-identical, median 3.4654/3.4729/3.5835, but this compares with an intentionally expensive always-check control, not production. Negative control always-check can be slower despite fewer Arnoldi columns.

NEXT THEORY: read actual PR104 `crates/rodas5p-krylov/src/gmres_into.rs`, `workspace.rs`, `small.rs`, PR19 incremental-Givens candidate and existing monitored-right GMRES. Derive separate left/right preconditioned prefix original-residual formulas, source/clock/epoch and stage inverse bound, accounting and refusal semantics. Do not assume the identity-preconditioner standalone proof applies to arbitrary preconditioners. Reuse PR103 chemical6 signed lognorm only if the actual owner-accepted J family and norm binding are verified. Preserve Q-LEDGER/authority failures and 10 open interactions.

NEXT CODING: isolated worktree from exact PR104 source. Implement opt-in prefix trigger on existing native solver without changing default route, public API or correction budgets. Native tests must include both preconditioning sides, warm starts, breakdown, wrong projected residual, nonfinite/partial callbacks, retry/transaction, workspace growth, residual-scale policy, source epoch changes. Original residual must be rechecked before output commit. If outward evaluation error and K not available, label floating original-residual PASS, NOT rigorous physical certificate. Benchmark full-IVP same source, controller, tolerances, actual achieved accuracy, seven paired ABBA/BAAB blocks; record JVP/MGS/QR/allocations and rejected attempts, compare positive and adverse arms. Do not combine with historical 2,460 PR104 solves or silently reset budget.

R6/C9 independent-author review of this new node is NOT_RUN. Do not claim 20/20 or 1/1 until independently completed. Preserve prior first failures. No main merge, force push, production promotion, global-campaign rerun or unknown physical input fabrication. Return exact commit/tree SHA, test commands/exits, original residual/forward bounds, timing raw, source/callback identity, DAG counts, GitHub PR and Drive+Dropbox actual ACK/restore status, plus a copyable next handoff.
