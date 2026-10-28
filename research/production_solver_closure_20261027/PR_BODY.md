## Why this change is needed

PR91 established useful exact structural reductions, cubic accuracy planning,
and selective-entry work evidence. Three costs still needed separate treatment:
repeated full-state stage verification and lost failure work; allocations and
expensive candidates in scalar homotopy; and adapter decisions whose callback
proxy misses costly implicit arithmetic and changing regimes.

This change derives and implements narrower, testable improvements while
preserving the original residual and accuracy contracts. The production
default remains **persistent RHS-only DOPRI5**. A supplied JVP, including a
caller-provided AD JVP, remains an explicit, problem-qualified option.

The initial recent-PR survey found no PR after **PR91**, whose baseline is
`206298b4068d923bb47c7160b111b7439098660c`. This work is stacked above that
baseline; it does not merge PR90/PR91 or claim a general nonlinear solver
convergence/order theorem.

## What changes

- **Structural mean-field stages:** `structural_mean_field.rs` adds charged
  `StructuralResult` failures and an outward enclosure of the original
  full-space residual. The represented-stage identity includes computed-mean,
  RHS, and lift defects. Verification changes from `O(s²n)` to `O(sn+s²)`;
  the exact `n=1`, `P=I` degeneration precomputes the scalar weights. Every
  scheduled worker is joined and charged, including failures. Compatibility
  wrappers remain; retry/fallback consumers must use the charged API.
- **Scalar homotopy:** `rhs_homotopy.rs` adds selectable scalar, algebraic,
  and rational candidates. Scalar specialization preserves the generic
  operation/evaluation order and result bits. The rational candidate
  `(1+2β)/(1+3β)` and its nonnegative cubic defect are derived algebraically.
  Every candidate still passes the original outward residual/physical-budget
  check; fallback work is charged. The legacy public entry remains generic;
  scalar Picard is a recommended opt-in, and rational prediction is separate.
- **Adapter:** `switching.rs` adds strict checked-integer return comparisons,
  derived comparator/sentinel caps, and optional reuse of a previous accepted
  pilot's mean-step proposal. Same-target comparisons and failure priority
  remain explicit. Observed-step reuse stays off by default; sentinel probing
  is not enabled as a universal fastest policy.
- **Production setup:** additive `ProductionConfig`/`ProductionSolver`,
  serialized deployment examples, provider preflight checks, and callback
  accounting. Unknown fields and missing required controls are rejected;
  the inherited optional `entry_probe_adaptation` may be omitted as `None`.
  Supplied templates serialize the complete effective schedule. Scope is
  smooth forward identity-mass IVPs with endpoint observations. Callers inspect
  success, reached time, message, and accounting completeness, including the
  accepted prefix on numerical failure.
- **Reproducibility and CI:** source/input/binary-bound command receipts,
  process-group cleanup, serialized native execution, paired benchmarks,
  instruction profiles, REVTeX report, a curated literature map, and a local
  Codex handoff. Inherited format/lint cleanup and its measurement-identity
  implications are recorded separately in `CI_HYGIENE.md`.
- **Integration accounting repair, separate task 1/3:** the full native gate
  exposed an inherited lagged-GMRES failure-work defect. The minimal
  `rodas5p_lagged.rs` patch reconciles physical JVP/matrix entries against
  their own charged ledger, independently of solver-level attempted work.
  The original failing test and its expectations remain unchanged.

No new fitted gain margin, safety multiplier, dimension cutoff, or tolerance
relaxation is used to obtain the reported improvements. Caller accuracy and
resource inputs remain distinct from proved numerical constants and from
existing operational controller/scheduling choices.

## Measured improvements and adverse controls

Speed ratios below are **baseline time / candidate time**. Brackets are the
observed minimum/maximum of five ABBA/BAAB blocks, not confidence intervals.
Each row names its baseline. All comparisons retain their original accuracy
and failure-accounting contracts.

| Comparison | Work/instruction evidence | Native median speed ratio [range] |
|---|---|---|
| Structural enclosed / PR91, n=2048, one worker | RHS 576→64; physical linear actions 512→0; guest instructions −62.52%; all grid-state bits unchanged | **2.063 [1.226, 3.614]** |
| Structural enclosed / PR91, n=1 | Exact scalar specialization addresses the retained initial 0.392× adverse result | **1.190 [1.154, 1.323]** |
| Structural fused direct / PR91, n=2048, one worker | Instructions −10.94%, but final median time is worse | **0.840 [0.619, 1.220]** |
| Nonlinear cubic generic → scalar | Operation/work parity; cubic profile instructions −36.26% versus generic | **1.474 [1.439, 1.828]** |
| Nonlinear cubic generic → rational | Tight cubic plan: ordinary+interval polynomial evaluations 1659→1024, plus 512 separately charged predictors; cubic profile instructions −39.01% | **1.670 [1.102, 1.767]** |
| Stiff adapter PR91 → observed-step | Total work 2958→2634 (−10.95%), but instructions **+0.505%** | **1.0106 [0.8981, 1.4469]** |
| Reentrant adapter RHS-only → observed-step + strict sentinel | Total work 17113→24440 (+42.81%); instructions **+145.02%** | **0.3404 [0.1265, 0.5046]** |

The cubic work example keeps `k=1, g=1/2, Y=T=1`, physical cap `1e-5`,
and the same `N=512` plan. Rational maximum grid error is approximately
`2.245e-7`, with final outward bound `4.466e-6`. Work, wall time, and
instruction rows identify their respective fixtures; their reductions are
not interchangeable measures. Scalar/rational wall-time ranges cross one
in both cubic cohorts, and the structural two-worker enclosure range also
crosses one. There is no blanket rational or parallel speedup claim.

These are shared-host **measurement-profile** observations, with Rust 1.94.1,
opt-level 3, no LTO, and 16 codegen units. Startup-subtracted Callgrind
instructions are not cycles or native elapsed time. Actual deployment
release/LTO qualification is a local gate.

### Adapter measurement identity after the final syntax correction

The paired adapter timings/profiles belong to switching source
`ebea1dd7b76da45def13f6d9e1137f693786f23308f2e0cc589debc82b658347`
and binary
`8671afe3561e0b658c857ca7fe4ee5f892781fcadac69b2d491c21c595680702`.
They remain explicitly historical measurements. The final two let-chain
rewrites preserve left-to-right short-circuit evaluation; 35 focused native
contracts were rerun, and the final 36-arm audit exactly matches every prior
non-timing field, including 17,440 state components, counters, and segment
decisions. That equality supports reuse of the prior Decimal90/44-seed
checks; it does **not** prove identical instructions or timings for the later
binary. See `adapter/PASS3_RECEIPT.json`, `adapter/FINAL_AUDIT.json`, and
`adapter/PERFORMANCE.json` under the research node.

## Validation status and retained limitations

The authoritative final disposition is
`research/production_solver_closure_20261027/evidence/VALIDATION_SUMMARY.json`.
The preceding full native gate **failed before the integration-accounting
repair**. The coordinated post-repair scopes pass 464 distinct native tests,
and the final production fmt/Clippy scopes pass. Full CI and deployment-host
release qualification remain separate; the historical failed sweep is retained.

| Evidence | Actual recorded result |
|---|---|
| Final production fmt scope | Passed after the counter repair: `evidence/post_fix_fmt/RECEIPT.json` |
| Final Clippy scope (`vigilode`, `rodas5p-core`, all targets, `-D warnings`) | Passed after the counter repair: `evidence/post_fix_clippy/RECEIPT.json`; earlier failed runs retained |
| Final facade/coupled package run | 297 + 127 = **424 passed, 0 failed, 0 ignored** after the counter repair; `evidence/facade_coupled_final/RECEIPT.json` |
| All-target check, rustdoc/doctests | Two changed packages compiled; warning-free rustdoc and two doctests passed in their recorded snapshots |
| JSON deployment smoke | Four RHS-only/selective-JVP × rate 1/1000 solves passed the original `1e-6` all-returned-state cap and atomic callback accounting |
| Structural independent oracle | 728 original-equation components and 168 stage bounds across 21 fixtures passed; 8,184 unchanged rounding-radius checks reused explicitly |
| Homotopy independent oracle | 5,404 observations across 24 arms checked against Decimal80 and the unchanged outward bound |
| Adapter final source audit | 35 native contracts passed; 36 arms exactly match prior non-timing outputs; independent Decimal90 evidence is reused through that equality |
| First five-package native gate | Timed out after 905.316 s (900 s command cap), exit 124, unchanged source; completed target summaries: **510 passed, 0 failed, 28 ignored**. Incomplete run, not a full-suite pass |
| Pre-repair five-package retry | Completed with exit 101 after 689.679 s, unchanged source; completed target summaries: **1312 passed, 1 failed, 45 ignored**. Failure: `reaudit_lagged_failure_work::recoverable_jvp_error_retains_completed_column_and_all_work_partitions`, assertion at line 77, actual 603 versus expected 604 |
| Integration-accounting repair | Original failing regression and all eight reachable/adjacent lagged targets pass: **40 passed, 0 failed, 4 ignored**; `evidence/lagged_scope_after/RECEIPT.json`. This gives **464 distinct post-repair scoped native passes**, without adding overlapping focused tests |

The retry's failure is retained in `evidence/native_final_r1/`, with the
isolated before-repair reproduction in `evidence/lagged_failure_before/`.
The source argument is recorded in `theory/FINAL_GATE_FAILURE_REVIEW.md`;
the reviewed reachability and evidence-reuse scope is in
`theory/LAGGED_VALIDATION_SCOPE.json`. The complete five-package sweep was
not rerun wholesale after this counter-only repair. Its final local release
run remains required; the new focused passes do not relabel the old failure.
No ignored or unexecuted test is counted as passed. Actual client
inputs, long integrations, restart/resource exhaustion, and deployment-host
release/LTO remain separately scoped local qualification work.

### Exact accounting repair and its limited scope

Before the repair, the lagged driver, regression test, generic operator
accounting, and relevant GMRES implementation were byte-identical to PR91.
The deterministic failure is not attributed to a shared test counter or to
the new adapter policy. The isolated test reproduces the same missing JVP
charge with unchanged source.

For one private lagged-stage invocation, let `A` count operator entries,
`L` the increase in solver-level attempted products, and `P` the increase in
the charged physical JVP/matrix ledger. Shape checks precede entry. Every
entered stage operator enters one JVP callback or matrix-product attempt, and its default
work hook records it only on successful operator completion. Thus, within
the valid native workspace and representable-counter domain,

```text
L = A
0 <= P <= A
missing physical entries = A - P
```

The old code reused `A-L`, which is zero even after an entered failure, as
the physical correction. The patch instead adds `A-P` to the physical
ledger before propagating the error. A fully successful stage adds zero;
the injected failure restores exactly one JVP call/vector. This includes
entered work omitted because a later operator-output check fails. Stored-LU
preconditioner accounting remains separate and unchanged. Arbitrary custom
composite providers and counter-overflow policy are not covered by this
local identity.

Only counter reconciliation changes: callback execution, states, tolerances,
iteration choices, and the existing test are retained. Measured adapter
routes use the dense/banded/matrix-free-fast router, not this lagged driver;
no speedup is attributed to this repair. It is the first of at most three
corrections for a separately diagnosed integration-accounting task, not a
fourth correction of the adapter research task.

## Proof scope and bounded research loops

Research correction counts are **structural 3/3, homotopy 1/3, adapter 3/3**.
The initial attempt is separate. Compilation/fixture repairs and the final
adapter syntax correction are labeled; they are not presented as additional
numerical performance tuning. Unsuccessful candidates and environment
failures remain in the evidence. The independently diagnosed inherited
integration-accounting repair has its own **1/3** correction count and
before/after qualification; it is not included in those three lane counts.

The structural certificate encloses **stage residuals**, including full-state
rounding defects; it does not yet supply a general outward physical
trajectory bound. The scalar cubic proof applies to its sealed monotone
model. The sentinel bound `2I+6` covers the current implicit window and does
not predict future-regime cost. The general causal route still uses frozen
JVP information and small projected linear algebra. Generic nonlinear
closure/order and branch/domain guarantees remain explicit theory tasks.

## Local qualification and six follow-up PRs

`DEVELOPMENT_PLAN.json` supplies a 16-node DAG, eight executable suites,
acceptance contracts, proposed branches, and machine-readable outputs.
`LOCAL_CODEX_HANDOFF_KO.md` pins the final delivered full SHA in a fresh
worktree, starts with a CLI dry-run, and preserves shared-lock execution.
After publication: `L-RELEASE → L-HARDWARE → R-QUALIFY` for the actual client.

| Follow-up | Dependencies | Purpose |
|---|---|---|
| PR-N01 | L-HARDWARE | Small Krylov norm/MGS/QR cost and rank/true-residual conditions; retain existing enabled SPD04 workspace and known Givens counterexample |
| PR-N02 | L-RELEASE | Finite-step cubic decay-envelope proof, outward accumulation, and a smaller justified grid |
| PR-N03 | L-RELEASE | Variable-step structural residual transport plus temporal/endpoint error and a charged adaptive consumer |
| PR-N04 | L-RELEASE | Complete native-session continuation and generic homotopy workspace with FSAL/controller/epoch identity |
| PR-N05 | L-HARDWARE, PR-N04 | Nonlinear causal closure/order and total work/span against the strong forward baseline |
| PR-N06 | L-HARDWARE, PR-N01 | Conditional temporal residence/exploration-loss theorem with variation/dwell/domain assumptions and charged revalidation |

Review material is under `research/production_solver_closure_20261027/`:
`README_KO.md`, `PRODUCTION_SETUP.md`, the three lane reports/theories,
`report/production_solver_closure.tex`, `CI_HYGIENE.md`, source/evidence
manifests, and `evidence/LITERATURE_MAP.json`. The literature map preserves
metadata, query provenance, and hashes; fetched external abstracts and paper
text are excluded from the public bundle while project mathematics and raw
numerical evidence remain available through their manifests.
