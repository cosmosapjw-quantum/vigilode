# Scientific Validity v2 — Claim Scope and Invalidation

## Exact boundary

```text
base: 8d0c79184e09efb5bdadc24a6315c60a71a44264
implementation revision: ab8fbcdb709aa1e87603b1ef6f83c5e610c8cb04
implementation status: IMPLEMENTED and locally committed with local contract evidence
synthetic CI smoke protocol: VALIDATED as nonauthoritative wiring only
canonical reference generation: COMPLETE (22 artifacts / 66 bindings)
canonical scientific campaign: EXECUTED_NONPASSING (54/54 output-policy-dominated)
canonical freeze: NOT_CREATED_BY_PROTOCOL
canonical Oregonator holdout replay: NOT_RUN_BY_PROTOCOL
external calibration: PARTIAL_TYPED (SciPy 54 success; CVODE 54 unavailable)
v2 scientific claim admission: NOT_ADMITTED
```

This document is the v2 claim quarantine. Local tests establish implementation
contracts, not scientific authority. The checked-in I smoke freeze/replay is built from
explicitly synthetic rows and cannot admit a numerical, performance, scaling, ranking,
equal-error, comparator, or production claim. Failure and dominated rows remain evidence
and must be preserved.

## Historical evidence is not v2 evidence

The v3.5, v3.6, v3.7, and A1 artifacts, including
`research/a1_inner_tolerance_audit_20260825/**`, remain historical,
byte-unchanged evidence. They are valid only under their original corpus,
comparator, output, and inner-solve policies. They are explicitly
non-transplantable to v2: no historical result, threshold, trace, timing,
ranking, or receipt admits a v2 claim without the applicable v2 replay.

## Implementation evidence is not campaign evidence

The A-I implementation implements problem-family separation, diversified calibration operators,
source-anchored holdouts, numerical-reference validation, weighted inner residual
certification and residual forcing heuristics, comparator evidence contracts, controller-neutral clipping and
dense output, corrected work accounting, and the source-bound v2 freeze/replay protocol.
Focused local contracts exercise those code paths. The source-bound runner then consumed a
complete 22-artifact/66-binding reference manifest and completed all 54 calibration cases.
Every row was `output-policy-dominated`, with no execution failures, so the pass-only freeze
was withheld and the three-row Oregonator input remained unopened. This is a preserved
scientific nonpass, not evidence that the holdout failed.

For discontinuous holdouts, the whole-domain callback is inspection-only: Medical Akzo
and Brusselator 2-D must be integrated through their branch-fixed
`integration_segments`, including at the shared endpoint. A validated numerical reference
is returned together with its exact WRMS error scale; candidate-error admission rejects a
different absolute scale, relative scale, or uncertainty. These are runner obligations,
not evidence that the canonical runner has executed.

## A–I dependency overview

| Cell | Gate | Implementation status | Scientific execution status |
| --- | --- | --- | --- |
| A | Claim quarantine, failure counters, and historical invalidation boundary | IMPLEMENTED; local document/counter contracts | NOT_APPLICABLE as a numerical campaign |
| B | Scale-invariant Krylov breakdown/certification and refresh accounting | IMPLEMENTED; local Krylov contracts | exercised by the nonpassing 54-row solver campaign; no performance claim admitted |
| C | Failure-typed adaptive diagnostics, operator accounting, and clip-neutral controller state | IMPLEMENTED; local adaptive/work contracts | 54 clipped/dense pairs completed with zero execution failures |
| D | Diversified calibration corpus and family-separated source-anchored holdouts | IMPLEMENTED; corpus, spectral, and branch-fixed segment contracts | 54-row calibration EXECUTED_NONPASSING; holdout remained sealed |
| E | Pinned high-accuracy numerical-reference generation/loading and scale-bound uncertainty dominance | IMPLEMENTED; reference artifact/loader/scale contracts | complete 22-artifact/66-binding manifest generated, self-checked, and consumed |
| F | Optional WRMS true-residual APIs and per-stage residual forcing heuristic | IMPLEMENTED; weighted-residual contracts plus nonnormal resolvent counterexample | endpoint contamination is not certified; canonical inner/outer study NOT_RUN |
| G | Frozen-Jacobian internal anchors and external comparator evidence contracts | IMPLEMENTED; comparator contracts | external surface covered: SciPy 54 success, CVODE 54 typed unavailable, zero solver failures |
| H | Dense output and controller-neutral hard-stop/output handling | IMPLEMENTED; dense-output contracts | all 54 clipped/dense comparisons were output-policy-dominated |
| I | Source-bound pass-only calibration freeze and checksum-first Oregonator replay | IMPLEMENTED; canonical freeze is emitted only with its validated 54-artifact aggregate; raw-row paths are smoke-only | freeze correctly withheld; Oregonator NOT_RUN_BY_PROTOCOL |

## Claim audit

| Claim | Status | Evidence | Risk | Required fix before promotion |
| --- | --- | --- | --- | --- |
| v2 gate accepts only exact predeclared profile sets | VALIDATED locally | `scientific_validity_v2_gates` | implementation evidence only | retain in canonical runner |
| v2 canonical rows are bound to one declared runner/configuration/reference/output lineage | VALIDATED for identity | 54 complete artifacts at revision `ab8fbcdb709aa1e87603b1ef6f83c5e610c8cb04`; record-set SHA-256 `73aeb3e9cdab1f4c59acb584cc0df2991106e144d7c201cd228ff7c52c27d8af` | bindings prove identity, not scientific quality | independently review any future passing campaign |
| v2 freeze threshold is eligible-row conservative maximum | WITHHELD AS DESIGNED | 54 output-policy-dominated rows; no freeze file | no real threshold exists | improve output-policy agreement under a new declared campaign; do not tune the frozen gate |
| Oregonator replay cannot alter the freeze | VALIDATED locally; NOT_RUN canonically | checksum-first replay and mutation contracts; no canonical freeze | holdout has no numerical evidence | run only after a future immutable passing freeze |
| synthetic smoke threshold `0.06` is scientifically meaningful | FORBIDDEN | fixture text says no campaign was run | claim inflation | never transplant it |
| v2 improves performance, scaling, or rank versus production solvers | FORBIDDEN at current evidence state | Rust campaign nonpassing; external aggregate incomplete because CVODE is unavailable | headline false positive | obtain valid same-error rows and a complete competitive external baseline |
| A-I local tests establish publication readiness | FORBIDDEN | tests cover implementation contracts only | authority overreach | separate artifact and scientific review required |

No claim was upgraded to scientific or publication authority. The newly validated
operational facts are limited to complete reference generation, source-bound execution of
all 54 calibration cases, correct withholding of a freeze, and typed retention of the
external comparator surface. None establishes output-policy equivalence, a ranking, or
publication readiness.

## Comparator and scaling boundary

Internal BDF1/BDF2/RadauIIA1/RadauIIA3 are reference implementations, not
competitive production baselines. The external aggregate contains 54 successful SciPy
Radau records, but the Rust same-error rows are invalidated and CVODE is unavailable;
therefore it supports no relative production claim versus RADAU5, CVODE,
OrdinaryDiffEq, SciPy, or SUNDIALS. The synthetic
stage-batch/parallel probe measures only thread-pool and compute-bound
overhead; it does not predict memory-bandwidth-bound JVP scaling.

## Claim ceiling

Until frozen calibration **and one untouched holdout** both pass their v2
gates, no scaling, ranking, equal-error, or production-baseline claim is
admitted. The 2026-08-29 calibration reached the declared terminal nonpass:
54/54 cases completed, 54/54 rows were output-policy-dominated, no freeze was
created, and Oregonator was not run. This includes claims inferred from legacy
receipts, internal reference implementations, synthetic parallel probes, or
partial v2 runs.

The checked-in smoke replay does not satisfy this condition because its measurement rows
are synthetic fixtures. Canonical claim promotion is `NOT_ADMITTED`; campaign execution
alone does not override the pass-only freeze gate.
