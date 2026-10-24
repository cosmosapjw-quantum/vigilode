# Local runner independent review

2026-10-24. Reviewer: geometry-transfer agent, independent of the runner implementer. Scope: read-only inspection of source/build binding, resume integrity, scientific eligibility and stated runtime limits. No runner code was edited and no campaign was executed by this reviewer. The runner agent owns focused regression tests and any fixes. The inspected sources are identified below; later changes require only review of the corresponding corrected paths.

## Findings and closure

| ID | Target | Observable gap and consequence | Minimal correction | State |
|---|---|---|---|---|
| LR1 | `external_compare.py`, result handling after `run_process` | Unlike the primary runner, external comparisons call `eligible` without comparing returned case/method/layer/threads with the requested task. A malformed adapter can report a valid PASS for a different task and become timing-eligible. | Reuse `result_contract_errors`; mark mismatch ineligible before saving. Test a well-formed wrong-case/method result. | Closed after corrected-path inspection; owner test freeze PASS |
| LR2 | `campaign.py`, resume and `analyze` | Existing receipts are read without verifying source, binary, expected task identity, or current stdout/stderr hashes. A copied foreign receipt or changed stream can contribute to an apparently complete timing group. | Bind each reused/analyzed receipt to the campaign and planned task, recheck recorded stream hashes, refuse mismatches; retain the original files. | Closed after corrected-path inspection; owner test freeze PASS |
| LR3 | `external_compare.py` and `stage_campaign.py`, cumulative budget | Only finished receipts contribute to prior elapsed time. START-only interrupted attempts are preserved but consume zero prior budget, so resumed execution can exceed its declared accounted process budget. | Charge the saved timeout for interrupted attempts, following the primary runner's existing conservative policy. | Closed after corrected-path inspection; owner test freeze PASS |
| LR4 | `build_local.py`, source identity around Cargo | The source is checked only before compilation. A concurrent edit during compilation can still yield a receipt that binds the binary to the earlier clean checkout. | Recheck commit/tree/clean status after Cargo; preserve build outputs but mark source drift as invalid for published planning. | Closed after corrected-path inspection; owner test freeze PASS |

Root separately requested host-resume identity and external dependency hashes. At the inspection snapshot the stage runner identity lacked host topology, and external identity had topology without full host identity. Those existing requests remain applicable; they are not counted as new findings here.

## Correctly scoped behavior observed

The main campaign requires exact commit/tree and a successful matching build receipt for published runs. Exploratory dirty smoke cannot enter speed ratios. Native timing, instrumentation, independent ensemble throughput, data parallelism, accuracy probes and causal-stage runs are separated. Complete eligible groups require every requested replicate, finite positive timings and actual endpoint error within the stated limit. Instrumented callback times are labelled as potentially overlapping. Actual core affinity and memory/time limits are applied to child process groups; timeout outputs are retained. The main runner already charges interrupted attempts conservatively.

External comparisons use descriptor-matched models with explicit initial state/time/RHS probes. VODE includes Python callback overhead, Julia compilation warmup is excluded and reported, and missing Diffsol support is NOT_RUN. These facts should remain in interpretation. Endpoint probes and wrapped phase cannot certify a continuous trajectory or exclude missed oscillation cycles. Host-specific tests and full benchmarks remain the local executor's work; reviewing a runner does not establish solver production readiness.

## Inspected bytes

- `campaign.py`: `c7d7c2bbb0ef2b5ee3a95b4f014c382c19e52ff909ec4780aa372133de2effe0`
- `build_local.py`: `f4ada46b9ca20119e4d555198a1515b44add7e94f1d8eb551bee282172cc0278`
- `external_compare.py`: `1e4afa04d723642bc2aa1f220055cfc58cc56688629a6a2f75f1d22cad981944`
- `stage_campaign.py`: `5f36c212b3f3726321082a7a8fa06eaf8fb8c3cf9c5b3de7bd981fb1169d0ef1`

## Final closure

All four findings are closed for the corrected source snapshot. `external_compare.py` now applies the requested-result contract check before eligibility. `verify_receipt` binds reused/analyzed receipts to source, binary and task, verifies stdout/stderr digests and checks stored result/sample equality with raw JSONL; the primary campaign identity stores its expected task map. External and stage campaigns conservatively charge saved START-only timeouts. `build_local.py` records `source_after` and refuses a successful-source binding when the checkout drifted during compilation. Both additional runners now bind hostname, platform and topology; Julia project/manifest hashes and adapter/dependency metadata are retained.

The owner ran 42 Python tests successfully in `evidence/local-runner-tests-freeze/RECEIPT.json`. This reviewer checked that every source hash in that receipt still matches current files and read the corrected paths. No additional tests were rerun by this reviewer. No remaining blocker was identified within this read-only audit scope. Runtime performance and host-specific installed comparator availability remain measured by the local campaign.

Corrected source hashes:

- `campaign.py`: `04bd3382c4aacdddeca6cb1f700e5ab211d0ad998efcb44f431f23dce1d04f77`
- `build_local.py`: `bf8c16b233caf233be02d4ed074be7eb95a7df5c8ff5bce946ef8e1ca18a5bb9`
- `external_compare.py`: `c5eb7452e3f55fcba352c8e75cd17ec7dfb58cd720189866b1b338da1a0eba02`
- `stage_campaign.py`: `995b55d40500c3cecafb8e36f37791bed1d30a9a6dc92a9528c4b056ce48177f`
