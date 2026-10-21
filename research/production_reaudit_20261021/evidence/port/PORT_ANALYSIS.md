# Published certificate-package port inventory

Current source: `4a5fc656730fe8243f06b6e603cb8aa758b00b2f`
Incoming branch head: `ee65d4dbecf3b3cd5f4587c27f87ef297d2bb406`
Published audited package: `33e45625be0803771156704263c3b1c1f92d2b40`
Merge base: `59d95eb7404882f024eb8bd3856f2ce4f1f830c9`

A no-commit merge was attempted in an isolated detached worktree. It found four Rust source conflicts and one ledger conflict. Source delta: 9 additive modules, 9 clean modified modules, four conflicted modules. Compile and regression checks were outside this inventory-only task.

## Concrete resolutions

### crates/rodas5p-integrators/src/bound_pr101.rs
Current: BoundPr101Problem frozen lambda=-1e6, eta=10; segment-local verifier and immutable certificate identity
Incoming: Parameterized public Pr101Model plus sealed Pr101VerifiedSegment and transactional verified driver
Resolution: Keep current bound_pr101.rs; add incoming file as pr101_verified.rs; preserve public Pr101Error for current users and expose incoming error as Pr101VerifiedError. Update incoming generic provider, tests/examples to this alias. Distinct model types stay distinct. Do not claim current frozen receipt certifies arbitrary parameterized model.

### crates/rodas5p-integrators/src/dense_output_v2.rs
Current: WFilteredReport and WFilteredEnforce scalar-mode defect estimates and current direct/JVP guard
Incoming: VerifiedResidual entry-point refusal and stable power-Horner interval evaluation with fine-grained counters
Resolution: Union enum variants and keep is_w_filtered; retain all current W-filter validation/control branches and add VerifiedResidual refusal. Port power-Horner arithmetic/counters with existing dense, frozen PR101, W-filter contracts plus imported generic verified tests.

### crates/rodas5p-integrators/src/packet_curve_output.rs
Current: Public weighted_center_displacement for output-radius provenance
Incoming: Weighted carry owner, from_weighted_radius and centered_weighted_radius Option
Resolution: Preserve both queries and fields. Compute weighted displacement once unconditionally; centered radius is Some(outward add(flow radius, displacement)) only in weighted-carry branch. Keep per-mode legacy semantics and work accounting; verify both callers.

### crates/rodas5p-integrators/src/lib.rs
Current: Current BoundPr101Problem exports and production wave1-3 symbols
Incoming: Additional certificate-module exports
Resolution: Add exports without deleting production symbols; alias incoming PR101 error and add its separate module.

### research/LEDGER.jsonl
Current: Production and triage ledger entries
Incoming: All-DAG and component benchmark entries
Resolution: Preserve both provenance sequences and disambiguate duplicate entry IDs if needed by a new reconciliation entry; do not silently replace historical rows.

## Targeted validation before accepting a port

- Existing production W-filter estimate/enforcement and explicit-Jacobian/JVP refusal tests.
- Existing sealed BoundPr101Problem local certificate and identity tests, plus incoming PR101/generic continuous-driver tests.
- Existing packet displacement/output contracts, plus A01 weighted-owner, A02 trajectory, A03 damped, A09 homotopy and A10 Bernstein contracts.
- Existing Leja trust-boundary and polynomial, collective direct, Gray–Scott tests and their new imported contracts.
- Source-bound receipts must distinguish imported historical evidence from new integrated-source execution; never rewrite old hashes to imply replay.

The full incoming head adds component benchmark tooling beyond the published certificate package. It is not needed for the solver/API port. The isolated merge is aborted after saving inventory; main audit worktree is untouched.
