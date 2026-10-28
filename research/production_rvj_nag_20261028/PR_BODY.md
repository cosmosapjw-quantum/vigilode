## Problem and scope

The supplied RVJ/NAG work exposes two distinct opportunities: cyclic closure certification repeats expensive generic polynomial work, while a valid all-root trace observable can be evaluated without enumerating roots. These do not share the semantics of an arbitrary selected physical root, and neither closes PR96's remaining production gates.

This draft is stacked on PR96 (`validation/cumulative-polyalgorithm`). PR95's actual-model remainder theory was inspected as a separate input and was not merged. Theory agents completed the original-equation proofs, quotient semantics and PR96 audit before parent admission and subagent-driven implementation.

## Changes

- Add opt-in `cyclic_closure` and `trace_quotient` modules and two native campaign examples. No default dispatcher change.
- Derive sharp Bernstein weights 3/4 and 25/32, branch/error factors 8/7 and 6/7, an O(n) scalar cyclic lift, N+1 endpoint reuse, and a safe two-envelope radius recurrence. Runtime uses no Jacobian/JVP/AD/Newton callbacks or matrix factorization.
- Make the quartic trace explicitly all-complex and algebraic-multiplicity preserving, including the triple root. Reject selected-real and real-only substitutions. Share the exact aggregate-field circuit with a proved 39/64 contraction bound.
- Add an independent Fraction oracle checking every exported original-equation slab, coefficient interval, clock/domain/radius and the multiplicity negative controls.
- Preserve complete raw evidence in deterministic compressed form with original-byte hashes, source receipts, all repeats, refused grids and inherited negative results.
- Deliver an 11-page REVTeX report, Korean companion, numeric results, updated 23-node/34-edge DAG plus bounded leaves, and a standalone local Codex prompt for a new independent cumulative validation PR.

## Actual execution and results

Actual source `64004e23042f2cf076da5378be724f50c3856f4c` was remotely published/read back before execution. The numerical executable source lines, oracle and frozen theory are unchanged. A postpublication CI fix changes four rustdoc-comment literals; initial file-byte binding and the later doc-only receipt are distinguished.

- New native unit tests: 6 cyclic + 9 trace PASS, 0 ignored.
- Independent exact-rational oracle: **732,947 checks PASS**, including 720 distinct cyclic slabs / 2,160 arm-slab records / 383,040 coefficient intervals and 992 trace slabs.
- Registered Wolfram supporting identities: 10/10 PASS. These do not replace the analytical or floating-point checks.
- At the same exact whole-state, whole-path cap 1/100000000, the first passing cyclic grid changes from historical N=64 to both direct arms N=32. Using the same Direct/tight arm in n=4/8/16 gives native whole-call median ratios **8.7206 / 9.3338 / 9.1984**. At fixed N=64, ratios are 4.2534 / 4.6292 / 4.7859. All seven repeats and all 12 nonadmitted cells remain.
- Trace candidate/interval/combined field median ratios are 1.1941 / 1.4974 / 1.2792. **Complete candidate trajectory has mixed results**: marginal medians 607.695 μs direct vs 667.394 μs shared, ratio 0.91055, overlapping ranges, shared faster 4/7 repeats. No stable trajectory improvement is established.
- Trace residual whole-path gate first passes at N=256. The exact-reference path already meets the cap at N=64, so the earlier residual refusals show conservatism of this certificate, not proven physical-error violations. The gate was not changed post hoc.

The environment is a shared AMD EPYC 9V74 container, Rust 1.94.1 measurement profile, one serialized benchmark job. Timings are descriptive. The trace whole-path certificate is currently external exact-rational code; route-specific certification timing and a native root-enumeration baseline were not measured, so no complete-certified-workflow trace speedup is claimed.

## Gates and follow-up

**Overall production remains HOLD.** Q-LEDGER/authority, Q-API, ten interactions, the full N01–N06 tasks, inherited negative-cost behavior, ignored supported features, vendor/release and real-client qualification remain separate obligations. New bounded PASS does not close these tasks. Metadata output is retained under `evidence/metadata/`; the append-only research-node/ledger check passed. The unmodified authority checker timed out during lazy object retrieval; a cached-object run with `GIT_NO_LAZY_FETCH=1` failed (178 distinct unallowlisted missing objects, 919 failure citations). This partial-clone result is not directly comparable with PR96's fuller-checkout missing49. Three new-node citations concern external BASS commit/blob input; typed repository/object authority remains open. Exact raw stdout retains its original terminal blank lines.

N03/N05 each used one coherent prepublication correction episode. A later rustdoc-only CI hygiene episode makes N03 **2/3**; N05 remains **1/3**. R-STRUCT and R-ADAPT remain exhausted at 3/3; Q-LEDGER remains 1/3. No task was renamed to acquire a new budget.

The local handoff distinguishes historical execution source from the actual newly published validation HEAD, gives the correct `--remote-ref`, avoids duplicate ancestor cherry-picks and double locking, and retains adapter/session, mode/kernel and certificate costs separately. It identifies selected-singular-branch, F5 global-count, generic-quotient, whole chemical-model and sharper trace-certificate theory as open. Parallel affine scan and broader polynomial routing remain future qualified implementations.

Start with `research/production_rvj_nag_20261028/REPORT_KO.md`, `RESULTS.json`, `MANIFEST.json`, `LOCAL_DEVELOPMENT_PLAN.json`, `LOCAL_CODEX_HANDOFF_KO.md`, and `report/rvj_production.pdf`.

## Postpublication CI follow-up

Initial delivery25e76dd exposed three introduced rustdoc broken links. Four mathematical interval literals were wrapped as code, preserving all executable non-doc-comment source lines; a focused `RUSTDOCFLAGS="-D warnings" cargo doc -p vigilode -p vigilode-coupled --no-deps --locked --offline` build verifies the correction. No numerical benchmark was rerun or reattributed. Initial CI clippy/fmt passed. CI authority reported missing56 in its fuller checkout, and the unchanged comparator setup failed because installed SUNDIALS headers require `mpi.h` unavailable to its GCC invocation. These remain explicit production/local gates. See `CURRENT_STATE.json` and `CI_ADDENDUM_KO.md`.

Initial CI `test` also fails the unchanged Givens historical outcome pin: scale1e14 now returns candidate `Ok(3640)` where the table expects Err, after its independent binary64 residual check. No exact-oracle pass is inferred. This is retained as a source/profile-bound local Q-GIVENS investigation; the expected flag was not relaxed.
