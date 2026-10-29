# Integrated report review

Reviewer: `/root/production_homotopy`; 2026-10-27 UTC.

This review covers the integrated REVTeX source, its three included research
sections, figure-generation inputs, the development DAG, retained numeric
results, and primary-source bibliography receipts. It does not repeat native
tests, profiles, or PDF rendering. The reviewer authored the homotopy lane and
previously reviewed the structural lane and production API; the integrated
review is a separate reading by a sibling agent, not blinded, heterogeneous-model,
or independent authorship of every included result.

## Findings sent to the integrator

### RPT-01 — Incorrect description of the cubic API default

**Initial status: correction requested.** Main text calls scalar Picard the
“default scalar Picard representation.” The actual legacy
`integrate_cubic_decay_accuracy` in
`crates/vigilode-coupled/src/rhs_homotopy.rs` still explicitly selects
`CubicStageSolver::GenericHomotopy(acceleration)`. There is no
`CubicStageSolver::Default` implementation. The new entry point requires an
explicit solver selection. The included homotopy section correctly states
that the generic public entry point is retained.

Replace “default” with “recommended” or state that scalar Picard is an opt-in
deployment choice. This is a report correction; no API change is requested.

### RPT-02 — Ambiguous error column in the homotopy work table

**Initial status: clarification requested.** The final table column is headed
“Max. error,” but the displayed errors belong to the rational predictor.
For example, `homotopy/ANALYSIS.json` records approximately `8.010599e-5`
for generic/scalar Picard on the loose cubic case, whereas the table's
`1.43850e-5` is the rational result. The methods use the same physical
accuracy plan, but do not return identical trajectories across these distinct
root algorithms.

Label the column “Rational max. error.” The generic/scalar bit-parity statement
is correct and is a different comparison. This clarification changes no work,
accuracy, or timing result.

### RPT-03 — Make the standalone structural bound notation explicit

**Initial status: clarification requested.** Structural S2 and S4 use
`a * epsilon` rather than `abs(a) * epsilon`. The implementation accepts only
positive `h`, and its fixed tableau has positive `gamma`, so `a=h*gamma>0`
is the applicable domain. State these hypotheses explicitly next to the
standalone theorem. There is no demonstrated runtime defect.

The stage-lift paragraph should specify
`||Delta_i||_infinity <= delta_i := [the four-term rounding bound]`
rather than merely `delta_i <= [bound]`. S2 needs an upper bound on the actual
lift defect; explicitly defining the chosen radius avoids confusing a free
upper-bound variable with the defect norm itself.

### RPT-04 — Reconcile final adapter correction count and evidence identity

**Initial status: known finalization item.** At the initial review snapshot,
the root report and JSON plan say adapter `3/3`, but the included adapter
section still says two passes. The integrator has identified the third pass
as a lint-only let-chain correction. Update that included passage and state
the evidence distinction: historical timing/profile results bind their
recorded binary and source, while fresh contracts/accuracy checks qualify
the final source. Do not relabel a historical timing receipt as a new profile
of a different source revision.

### RPT-05 — Final gate aggregation remains an integration obligation

**Initial status: pending final receipt, not a detected overclaim.** The report
does not claim that all CI passed. It explicitly separates tests, compilation,
linting, publication, and local release qualification, and points to
`evidence/VALIDATION_SUMMARY.json`. That final aggregate did not yet exist
when this initial review read the tree. The integrator is finishing it.

The reported 297 facade tests and 127 coupled tests were independently
recounted from their retained stdout: 424 passed, zero failed, and zero
ignored in those invocations. The docs logs contain two passing doctests.
All-target and docs receipts show zero exit. These observations must retain
their source identity and cannot substitute for the still separate full CI,
ignored-test, deployment release/LTO, or actual-client qualification.

## Claims checked against retained evidence

| Claim | Evidence and review result |
|---|---|
| Structural full residual versus physical trajectory | S1 contains computed-mean discrepancy, RHS error, and all lift defects. S2 uses full-space induced norms. Main text and included section expressly leave temporal truncation, endpoint rounding, and outward trajectory transport to PR-N03. Correctly scoped. |
| Structural complexity and exact scalar case | `O(sn+s^2)` applies to the factorized full-stage screen; the n=1 `P=I` specialization is algebraic and has plan-only weighted sums. Fixed tableau setup is disclosed separately. No fitted dimension threshold is claimed. |
| Structural speed and work | `FINAL_BENCHMARK_SUMMARY.json` gives enclosed n=2048/worker1 median 2.062922, range [1.225588,3.613706], same-grid bits, unchanged maximum grid error 2.397597e-10, RHS 576→64 and linear actions 512→0. The report rounds these correctly and preserves the fused negative median and two-worker range crossing one. |
| Structural instruction improvement | `instruction_profile/SUMMARY.json` gives enclosed/PR91 0.374822714 and fused/PR91 0.890638320. The stated reductions 62.52% and 10.94% are correctly attributed to guest instructions, separately from wall time. |
| Cubic mathematical scope | Exact inverse and rational defect are for the sealed monotone cubic and its admitted domain. Original outward residual acceptance, represented-update displacement, and finite-budget fallback remain explicit. Generic vector homotopy convergence/branch control is not inferred. |
| Cubic work and instructions | Tight-plan ordinary+interval evaluations 1659→1024 retain N=512. Generic/scalar/rational instruction slopes are 3182149.3, 2028310, and 1940807. The 36.26%/39.01% reductions and strong-scalar 4.31% comparison match the JSON. Predictor work is counted separately. |
| Cubic wall-time qualification | Nonlinear generic/scalar and generic/rational medians are 1.473590 and 1.670431; all five relevant ratios exceed one. Scalar/rational ranges cross one. Main report and included section do not convert these host observations into universal runtime superiority. |
| Adapter arithmetic versus routing | Stiff PR91→observed work falls 2958→2634, while instructions rise approximately 0.505% and timing range crosses one. Reentrant RHS-only→observed sentinel timing median is 0.340358 with range [0.126476,0.504606], and instructions rise approximately 145.02%. The adverse control remains visible. |
| Adapter theorems | Strict integer preference is limited to a fresh same-target window. The `2I+6` theorem is relative to its implicit incumbent, not a future-regime or horizon bound. PR-N06 states the needed variation/dwell/domain assumptions and exploration accounting. Correctly distinguished from PR-N01 arithmetic cost. |
| Causal parallelism | The report states that inherited general causal paths still use a frozen JVP basis and small projected algebra, and does not claim arbitrary nonlinear sequential dependencies can be removed profitably. Forward substitution, communication, validation, and work/span remain comparison obligations. |
| Configuration and failure scope | Persistent RHS-only default, explicit supplied-JVP permission, replay safety, strict recursive production JSON, endpoint-oriented smooth forward identity-mass scope, and charged failure APIs agree with the reviewed API contracts. |
| Machine-readable development plan | Current JSON has 16 nodes and eight suites. PR-N01 depends on L-HARDWARE; N02/N03/N04 on L-RELEASE; N05 on L-HARDWARE+N04; N06 on L-HARDWARE+N01. These match the report table. |

The figure generator reads the same structural, homotopy, and adapter JSON.
It uses median and min/max from the actual paired blocks, and does not
manufacture confidence intervals. Different baseline conventions in the
timing and instruction figures are labeled and explained in their captions.

## Primary bibliography check

The retained `evidence/PRIMARY_SOURCES.json` contains the actual arXiv and
CWI primary-page/PDF text, not merely a secondary search summary. It confirms:

- Jiachen Jin, Hongxia Wang, Kangkang Deng, *Anderson acceleration of
  derivative-free projection methods for constrained monotone nonlinear
  equations*, arXiv:2403.14924, revised 2025, related DOI
  `10.1007/s10957-025-02841-y`. Initials J. Jin, H. Wang, K. Deng are correct.
- P. J. van der Houwen and B. P. Sommeijer, *Parallel iteration of high-order
  Runge–Kutta methods with stepsize control*, JCAM 29 (1990), 111–127.
  Its discussion explicitly limits the functional-iteration construction
  to nonstiff problems; implicit-corrector stability is not automatically
  retained by a truncated iteration.
- B. P. Sommeijer, *Parallel-iterated Runge–Kutta methods for stiff ordinary
  differential equations*, JCAM 45 (1993), 151–168. The corrected title and
  year match the CWI original. Its abstract uses a diagonally implicit
  construction and one LU factorization per step per processor; the report
  does not cite it as a derivative-free theorem.

The Ogita–Rump–Oishi citation is *Accurate sum and dot product*, SIAM J. Sci.
Comput. 26 (2005), 1955–1988, DOI `10.1137/030601818`. The earlier independent
structural review checked its FastTwoSum result and the magnitude-ordering,
finite-arithmetic, and gradual-underflow assumptions against the actual
implementation. This report review reuses that evidence and does not claim
a second full paper replication.

## Initial source/evidence snapshot

These hashes identify the first recorded snapshot during the review. The
integrator was applying prose corrections concurrently; the root TeX hash
already includes its scalar-default wording fix. The findings above record
the earlier text reading. Closure hashes below identify the subsequently
reviewed source. A hash does not by itself constitute a test result.

| Relative path | SHA-256 |
|---|---|
| `report/production_solver_closure.tex` | `66439bd637929afe8e8b2d1c6611c598f8346e5a7575e4475abc995bdf4247b4` |
| `structural/section.tex` | `667abb38e9558487aead8a4aadf1d5b342682935da9bf5a265d67478d42da695` |
| `homotopy/section.tex` | `a35531de29629f445eaffec1425970c8a47faf0c1553adbd46ce05d61cb5894e` |
| `adapter/adapter_section.tex` | `2fb885e3ea9c141024c7b8b1085ad9bc1da82e1399d0ed4839fa8e4cda9e0e1d` |
| `DEVELOPMENT_PLAN.json` | `b9742a399c9533a9a788a2600a47fbf1698acb43b5093528cc15a4a5b02838c8` |
| `structural/FINAL_BENCHMARK_SUMMARY.json` | `863872f342f2f123d7994c275d7dd873766ea8c4b1f68076da19d97be66f819b` |
| `homotopy/ANALYSIS.json` | `4645c4fb54c9dbf348eb827eae227cd4451c91efa71afa30f8cd7a2dbc469351` |
| `adapter/PERFORMANCE.json` | `c1d57b53a17f25bd71651c61d05277afa1cd0d5ce97f508e397ba691f2ef0d3c` |
| `evidence/PRIMARY_SOURCES.json` | `4390c0503cb46a51ea6f60a86b56accfd58e2d56549271ea2f1ad8cbc30b42b9` |

## Closure of report findings

**Status: scoped report review passes after the recorded prose corrections;
final integration/native/publication gates remain separate.**

RPT-01 is closed: the root text now says “recommended ... opt-in” and expressly
states that the legacy entry still selects generic homotopy. RPT-02 is closed:
the table column now says “Rational max. error.” RPT-03 is closed: the
structural section now declares `h>0`, fixed `gamma>0`, and
`||Delta_i||_infinity <= delta_i := ...`.

RPT-04's report/provenance wording is closed: the included adapter section
now records all three corrections, identifies the last two equivalent
let-chain rewrites, and retains historical timing/profile identities.
`adapter/PERFORMANCE.json` now explicitly labels its measurements
`historical_pre_style_cleanup_measurement`, with measured switching-source
hash `ebea1dd7b76da45def13f6d9e1137f693786f23308f2e0cc589debc82b658347`
and binary hash `8671afe3561e0b658c857ca7fe4ee5f892781fcadac69b2d491c21c595680702`.
It expressly says that later numerical/counter equality does not imply
identical instructions or timings. Final adapter native/audit receipts were
still being produced at this closure; their completion is part of RPT-05,
not certified by this read-only review.

RPT-05 remains a named integration action: publish actual final gate receipts
and retain inherited/initial failures plus the separate final qualification.
No “all CI green,” whole-repository release, or generic nonlinear theorem
claim has been approved here. PDF re-render after the accepted changes is
also the integrator's responsibility.

| Closure path | SHA-256 |
|---|---|
| `report/production_solver_closure.tex` | `66439bd637929afe8e8b2d1c6611c598f8346e5a7575e4475abc995bdf4247b4` |
| `structural/section.tex` | `ba33e4ede5eaa40ba5d23b4a004ab9a15b60bf1f23f2ccc0e53f8429d3f5582c` |
| `homotopy/section.tex` | `fe924bcd96e1b70843c36649bceca27344a178d6917fe2cd63548277e81a44d2` |
| `adapter/adapter_section.tex` | `47d0d0975d718c8e94a97af49f405a727172b454174fda6487203326f8e01304` |
| `DEVELOPMENT_PLAN.json` | `b9742a399c9533a9a788a2600a47fbf1698acb43b5093528cc15a4a5b02838c8` |
| `adapter/PERFORMANCE.json` | `8213878c0d3efa3e04624c525a344e3b87c6bdf05bcc45982b0509227f41b528` |

No native command was executed by this reviewer for this task. Source
manifest generation and publication remain the integrator's separate actions.
