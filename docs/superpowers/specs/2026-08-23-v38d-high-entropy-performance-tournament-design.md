# VigilODE v3.8-D High-Entropy Performance Tournament — Design Specification

Date: 2026-08-23  
Approved design basis: user approval in the VigilODE research thread  
Canonical parent: `main@db51a9537a3f4898149cb463711eab0925387388`  
Canonical parent tree: `4a82e7b9196c383fdd9a9cae5ba566035ea420e0`  
Parent scientific verdict: `PASS_CONSUMED_CONTINUATION_TRANSACTION`

## 1. Status and task layer

This document is a **design-only authority** for a high-entropy coding research
loop. It does not implement a performance candidate, generate an authoritative
paired-wall campaign, activate method switching, or change a frozen scientific
constant.

Current task layer: `design`.

The approved intent is to explore many materially different speed-improvement
mechanisms aggressively, while preserving the original VigilODE objective:
reduce complete same-error sequential cost through guarded, resumable,
Jacobian-free exponential-Rosenbrock candidates without compromising the
protected R-JF authority path or hiding speculative/fallback work.

High entropy applies to candidate generation. It does not relax claim gates,
numerical verification, or work accounting.

## 2. Original objective and current gap

### 2.1 Primary objective

VigilODE should eventually support a committed polyalgorithm in which selected
`pexprb54s4` steps reduce complete sequential cost relative to protected
matrix-free RODAS5P at the same external global error and requested-output
schedule.

A successful performance candidate must therefore improve at least one of:

- JVP-vector count;
- projected dense matrix-function work;
- orthogonalization work;
- allocation and memory-traffic cost;
- restart/substep waste;
- critical-path wall time;

without weakening endpoint accuracy, residual certification, rollback,
requested-output semantics, or the complete work ledger.

### 2.2 Current durable evidence

At the canonical parent:

- R-JF is the sole committed authority trajectory;
- the E lane is read-only, explicit-Jacobian-free, direct-factorization-free on
  the physical state, and Newton-free;
- frozen recommendation policy remains `k=3`, prefix `B_abs=80`,
  `delta=0.25`, and `tau_zeta=13.39706618860016`;
- the continuation transaction has an event-local 80-JVP-vector cap;
- consumed replay gives 64 recommendations, 62 completions, two charged cap
  exhaustions, zero numerical continuation failures, zero unsafe
  recommendations, and zero budget breaches;
- the two continuation-tail events are semilinear-ramped N=192 target attempt
  12 and N=384 target attempt 23;
- v3.7 bounded speculative work is 3,679 JVP vectors against 388,999 committed
  R-JF JVP vectors;
- bounded full-E work at the 64 recommended targets is 2,466 JVP vectors
  against 13,043 matched target R-JF JVP vectors.

These figures identify performance headroom but do not establish a committed
speedup.

### 2.3 Current implementation bottlenecks

The present fused-phi path has several concrete cost centers:

1. every Arnoldi checkpoint forms a projected Hessenberg copy and performs one
   dense exponential action plus a separate dense `phi_1` action for the
   residual estimate;
2. basis, Hessenberg, work, reduced input/output, and nested-difference storage
   use repeated `Vec` or `Vec<Vec<f64>>` allocation/copy patterns;
3. full MGS uses two passes over the active basis at every Arnoldi column;
4. a failed one-substep attempt is discarded before retrying with 2, 4, ...
   substeps;
5. main and embedded endpoint actions are solved independently;
6. scalar operator calls dominate the current fused engine even though the
   operator contract supports batched row application;
7. checkpoint frequency is fixed by `dimension_increment=2` in the consumed
   atlas configuration.

The tournament targets these mechanisms. It does not retune the safety witness.

## 3. Scientific and software contract

### 3.1 Preserved scientific behavior

All candidates must preserve:

- regular ODE or constant nonsingular-mass-system scope;
- binary64 arithmetic unless an explicitly separate diagnostic uses higher
  precision as a reference;
- the same `pexprb54s4` tableau and stage equations;
- the same external tolerance and requested-output contract;
- the same matrix-free JVP definition;
- the same frozen recommendation decisions for consumed data;
- the same R-JF attempts, accepted rows, trajectories, controller state, and
  requested outputs;
- transactional prefix and continuation accounting;
- fail-closed behavior on nonfinite values, nonconvergence, budget exhaustion,
  invalid dimensions, and invariant violations.

### 3.2 Frozen policy and claim boundary

The following remain immutable in this research loop:

```text
persistence k                  3
prefix absolute cap           80 JVP vectors
cumulative prefix fraction    0.25
zeta34 threshold              13.39706618860016
continuation absolute cap     80 JVP vectors
active switching              false
N=2048                        sealed
```

No consumed-profile result may modify those values or promote a new authority
selector.

### 3.3 Non-goals

This design does not authorize:

- active switching;
- controller/cache transfer;
- forced-switch fifth-order recovery;
- a speedup claim;
- a new safety classifier;
- BDF/Radau optimization;
- physical-client tuning;
- N=2048 execution;
- GPU/MPI production claims;
- general DAE support;
- a new production dependency without explicit approval;
- public API or durable file-format breakage;
- broad cleanup unrelated to a surviving candidate.

## 4. DAG and phase ordering

### 4.1 Required prerequisite node

The first implementation node remains the already-sealed **v3.7 Timing
Authority Validator**. It must be implemented and focused-tested without
producing a new paired-wall campaign.

This prerequisite is not a new gate. It is the existing control for the
observed N=384 host pathology and replaces ad hoc timing exclusion decisions.
It must remain minimal:

- source/toolchain/binary/contract identity;
- host fingerprint and thread environment;
- idle/steal, swap, and exposed thermal-throttle checks;
- whole-campaign arm-span and order-sensitivity validation;
- all-pair retention;
- three passing complete campaigns within five attempts;
- decision independence from favorable or unfavorable ratios.

### 4.2 Tournament phases

After the validator implementation is merged, the research loop proceeds in
bounded phases:

1. **Baseline substrate** — characterize current kernels and selected event
   replays; checkpoint.
2. **Independent candidate spikes** — one major optimization idea and one
   verification pass per worktree; checkpoint each candidate.
3. **Survivor ablation** — compare only candidates that pass hard correctness
   gates; checkpoint.
4. **Combined prototype** — combine at most the smallest mutually compatible
   survivor set; checkpoint.
5. **Independent review and closeout** — one code/science/numerical review;
   select at most one or two candidates for a genuinely fresh holdout.

No phase combines planning, several implementations, repeated reviewers, and
promotion into one uninterruptible run.

## 5. Baseline measurement substrate

### 5.1 Authority separation

Two evidence classes are distinct:

- **exploratory tournament evidence:** local microbenchmarks and event replay
  used to rank candidates;
- **timing authority evidence:** a complete paired-wall campaign accepted by the
  sealed v3.7 Timing Authority Validator.

Exploratory measurements may not be cited as host-qualified timing or speedup.
They use a separate schema/path and must carry an explicit
`EXPLORATORY_NOT_TIMING_AUTHORITY` status.

### 5.2 Microkernel benchmarks

The baseline harness measures:

1. projected `exp` plus `phi_1` residual oracle for projected dimensions 2–32;
2. one Arnoldi extension under full MGS and incomplete orthogonalization;
3. checkpoint evaluation at the consumed dimension schedule;
4. contiguous versus nested-vector basis/Hessenberg traversal;
5. dense reduced-space matmul/LU/exponential kernels;
6. scalar versus batched operator application where a genuine batch exists;
7. allocation count and allocated bytes in a dedicated single-thread benchmark
   process using a test-only counting allocator, with no production allocator
   change.

### 5.3 Event replay set

The exploratory event set contains:

- representative low-cost completions from N=96 and N=192;
- intermediate-cost completions from N=256 and N=320;
- the N=192 semilinear-ramped cap-exhausted event;
- the N=384 semilinear-ramped cap-exhausted event;
- matched normal events from the same semilinear family where available.

Every selected event records the exact profile, family, target attempt, source
hash, configuration, and baseline work counters. Event selection is fixed
before candidate output and is not changed to rescue a candidate.

### 5.4 Metrics

Primary metrics:

- JVP vectors;
- `phi_projected_exponentials`;
- `phi_dense_oracle_calls`;
- `phi_krylov_vectors`;
- `orthogonalization_inner_products`;
- `orthogonalization_vector_updates`;
- `phi_restarts` and substep count;
- allocation count and allocated bytes;
- peak workspace size;
- endpoint WRMS difference from authority;
- residual-estimate difference and convergence-decision parity.

Exploratory wall metrics:

- median and interquartile range over predeclared repetitions;
- p95 regression on normal events;
- isolated tail-event wall and work change.

All repetitions and failed runs are retained.

## 6. Candidate registry

Every candidate has:

- a stable ID;
- one mechanism hypothesis;
- one isolated worktree/branch;
- files/functions allowed to change;
- expected signal;
- kill criterion;
- software, scientific, numerical, and reproducibility checks;
- a disposition of `SURVIVE`, `HOLD`, or `KILL` after checkpoint.

The execution lifecycle itself remains `READY → ACTIVE → CHECKPOINT → DONE`,
with `BLOCKED` or `FAILED` exceptional exits.

## 7. Candidate lanes

### 7.1 Lane K — low-risk kernel candidates

#### K1. Projected exp/phi1 oracle fusion

Hypothesis: one augmented projected matrix exponential can supply both
`exp(scale*H) beta e1` and `phi_1(scale*H) beta e1`, replacing the current two
Padé exponentials per checkpoint.

Target locations:

- `crates/rodas5p-core/src/matrix_functions.rs`;
- `projected_exponential_action_with_residual_estimate` in
  `crates/rodas5p-integrators/src/exponential.rs`.

Design constraints:

- operate only in reduced projected space;
- preserve the same Higham Padé-13/scaling-and-squaring authority;
- expose an internal fused result containing both vectors;
- count one projected exponential/oracle call;
- do not change physical-state factorization or JVP semantics.

Expected signal: approximately half the projected exponential/oracle calls at
checkpoints, subject to the one-dimension-larger augmented exponential cost.

Kill criterion: no meaningful projected-oracle or event wall reduction, any
convergence-decision drift not explained by accepted roundoff tolerance, or any
normal-event regression above the tournament ceiling.

#### K2. Reusable fused-phi workspace

Hypothesis: contiguous reusable buffers reduce allocation, copying, and cache
misses without changing arithmetic order.

Candidate internal type:

```rust
struct FusedPhiWorkspace {
    basis: Vec<f64>,
    hessenberg: Vec<f64>,
    work: Vec<f64>,
    reduced_input: Vec<f64>,
    reduced_output: Vec<f64>,
    previous_projected: Vec<f64>,
}
```

The workspace is internal and caller-owned or session-owned. It is not a new
public API in this loop.

Expected signal: sharply lower allocation count/bytes and improved wall time,
with bitwise output and work-counter parity where operation order is unchanged.

Kill criterion: output/work drift, unsafe aliasing, workspace growth beyond the
existing asymptotic bound, or less than the minimum exploratory gain.

#### K3. Contiguous basis/Hessenberg layout

Hypothesis: row-major contiguous storage improves orthogonalization and
projected-copy locality.

This candidate is evaluated separately from K2 first, even if their eventual
implementation shares a type. It must preserve basis-vector traversal order and
Hessenberg indexing.

#### K4. Reduced-space faer/specialized kernel

Hypothesis: existing `faer` dependency or a small projected-space specialized
kernel can accelerate matrix multiplication and LU in Padé-13 without changing
the mathematical algorithm.

No new dependency is allowed. This candidate remains projected-space-only.
Roundoff may differ, so convergence-decision and endpoint tests are mandatory.

### 7.2 Lane C — convergence-control candidates

#### C1. Checkpoint schedule tournament

Compare fixed `dimension_increment` variants and one bounded adaptive schedule.
Initial variants are 1, 2, 4, and 6; the consumed authority value 2 remains the
baseline.

The adaptive variant may use only already-computed residual history. It may not
use endpoint admissibility, future work, physical-client labels, or consumed
unsafe/safe labels.

Expected signal: fewer projected exponentials without material extra JVPs or
late-detection failures.

#### C2. Selective reorthogonalization

Start with one MGS pass. Trigger a second pass only when a predeclared
orthogonality-defect criterion fails. The criterion must be based on current
column quantities and validated against full MGS.

This candidate is numerical, not merely a kernel refactor. It must report
orthogonality diagnostics and may not silently downgrade to one pass.

#### C3. IOP window variants

Evaluate existing incomplete orthogonalization with lengths 2, 4, and 8 against
full MGS. Promotion requires stable residual and endpoint behavior on normal and
nonnormal families. A wall gain alone is insufficient.

#### C4. Adaptive Krylov-dimension/substep controller

The current 1→2→4 substep retry discards failed Arnoldi work. A candidate
controller may choose whether to increase Krylov dimension or substeps using
current residual contraction and counted cost.

It may not claim exact work reuse unless basis/state reuse is genuinely
implemented. All discarded work remains charged.

### 7.3 Lane M — multi-action and batching candidates

#### M1. Lockstep main/embedded endpoint actions

Explore a shared multi-action or block-Krylov formulation for the main and
embedded endpoint combinations at the same operator and scale.

Because the augmented operators depend on the combination vectors, a shared
basis is not assumed. The spike must first prove a mathematically valid common
subspace construction. If it cannot, the candidate is killed rather than
forcing an abstraction.

#### M2. Batched operator application

Use `LinearOperator::apply_rows` only where multiple independent vectors are
available simultaneously. The continuation budget guard must reserve a batch
atomically before any underlying row invocation.

A synthetic batch that increases total JVP work or changes the sequential cost
model is rejected.

#### M3. Block phi action

A true block Arnoldi candidate may be explored only after M1/M2 feasibility.
It remains experimental and must handle rank loss, block orthogonality, and
componentwise work accounting explicitly.

### 7.4 Lane A — alternative matrix-function candidates

#### A1. Leja interpolation for restricted spectra

Explore Leja only on operators with a justified spectral enclosure available at
low additional cost. No universal Leja backend is assumed.

#### A2. Chebyshev or polynomial action

Explore only if the relevant operator family provides a defensible interval or
field-of-values bound. Otherwise kill before implementation.

#### A3. Cross-step basis recycling

This is a high-risk hold candidate. It requires evidence that Jacobian/operator
change, controller rollback, and cache invalidation can be made transactional.
It is not implemented in the first tournament wave.

## 8. TDD and implementation discipline

For each implemented candidate:

1. write one minimal failing characterization/acceptance test;
2. run it and confirm the expected failure;
3. implement the smallest candidate change;
4. run the targeted test and affected regressions;
5. run one scientific/numerical verification pass;
6. benchmark the fixed substrate;
7. perform one independent diff review;
8. checkpoint and assign `SURVIVE`, `HOLD`, or `KILL`.

Production code is not written before a failing test. Experimental spike code
that is not intended to survive must be clearly labeled and discarded or kept
outside production modules.

Each materially equivalent failure receives one initial attempt and at most one
diagnostic correction. Two failures produce `FAILED → CHECKPOINT → STOP` for
that candidate.

## 9. Hard correctness gates

Every candidate must satisfy all applicable gates:

```text
R-JF trace/state/requested-output mutation          0
prefix recomputation                                0
negative or hidden work delta                       0
explicit physical-state Jacobian build              0
direct physical-state factorization                 0
Newton/nonlinear iteration in E lane                0
prefix or continuation budget breach                0
v3.6 durable schema mutation                        0
frozen recommendation drift                         0
active switching                                    false
N=2048 execution                                    false
```

### 9.1 Exact-kernel candidates

K1–K3 should preserve:

- 64 frozen recommendations;
- 62 bounded completions and two charged exhaustions;
- identical JVP/restart/substep counts unless the mechanism explicitly targets
  one of those counts;
- exact work recomposition;
- endpoint and residual behavior within a predeclared roundoff envelope;
- no changed convergence decision on the consumed replay unless the change is
  separately audited and improves authority agreement.

K2/K3 aim for bitwise parity because arithmetic order should remain unchanged.
K1/K4 may use strict numerical parity rather than bitwise parity because the
dense operation order changes.

### 9.2 Numerical-algorithm candidates

C1–C4, M1–M3, and A1–A2 may change Krylov dimensions, projected work, and bounded
completion count. They must still preserve:

- all 64 frozen recommendation decisions;
- no completion loss on the predeclared normal-event set;
- zero unsafe endpoints among completed events;
- endpoint WRMS agreement with the unbounded/full-MGS authority within the
  existing local error contract;
- residual-estimate honesty against an independent reference;
- exact cap enforcement and complete work accounting;
- zero hard numerical failures on events completed by the authority path.

Improving the two cap-exhausted events is a performance result, not fresh safety
evidence.

## 10. Exploratory survival criteria

A candidate survives only if it passes every hard correctness gate and produces
at least one predeclared material signal:

- median microkernel wall reduction of at least 10%; or
- representative-event JVP reduction of at least 10%; or
- projected-exponential/orthogonalization/allocation reduction large enough to
  explain at least a 10% event-level wall reduction; or
- a material improvement in bounded progress on one or both N=192/N=384 tail
  events without normal-event regression.

Additional ceiling:

- normal-event p95 exploratory wall regression must not exceed 5%;
- memory use must not grow asymptotically and must not increase materially
  without a measured compensating gain;
- any gain that appears only under one favorable run order is held, not
  promoted.

These are tournament pruning thresholds, not production selector thresholds or
publication timing claims.

## 11. Survivor combination policy

Candidates are combined only after isolated ablation.

First expected combination, if each component survives independently:

```text
projected exp/phi1 fusion
+ reusable contiguous workspace
+ checkpoint schedule survivor
+ selective reorthogonalization survivor
```

Multi-action, block, Leja, and recycling candidates are added only if they
independently survive and remain mathematically compatible.

The combined prototype must demonstrate that its gain is not less than the sum
of regressions hidden by interaction. An interaction that erases isolated gains
causes the larger combination to be killed; individual survivors remain.

## 12. Parallel worktree plan

Independent candidate exploration may run in parallel only in separate Git
worktrees. Initial candidate branches are:

```text
research/v37-timing-authority-validator
perf/v38d-projected-oracle-fusion
perf/v38d-fused-phi-workspace
perf/v38d-checkpoint-schedule
perf/v38d-selective-mgs
perf/v38d-adaptive-krylov-substep
perf/v38d-lockstep-multi-action
perf/v38d-leja-experimental
```

The first tournament wave should not open one remote PR per failed candidate.
Local commits and durable bundles/checkpoints preserve failed and held work.
Only the validator and a coherent survivor or combined prototype should be
proposed as review PRs.

No two worktrees modify the same production file concurrently without an
explicit integration order. Candidates touching `exponential.rs` are developed
independently and combined only after isolated results are final.

## 13. Evidence and plots

Each candidate produces a compact evidence set:

- machine-readable benchmark JSON;
- event-level work and accuracy CSV;
- before/after residual and endpoint-error plot;
- work-counter decomposition plot;
- exploratory wall distribution plot;
- tail-event comparison plot when relevant;
- candidate decision record with hard-gate results and disposition.

Plots are evidence, not decoration. The review reads them for:

- distribution overlap and outliers;
- gain concentration in one event/family;
- hidden normal-event regression;
- residual or endpoint drift;
- order sensitivity;
- evidence that a wall change is explained by counted work or allocation
  change.

Every plot retains all runs. No unfavorable point is deleted.

## 14. Validation matrix

| Requirement | Check | Level | Promotion expectation |
|---|---|---|---|
| source/toolchain identity | exact hashes | reproducibility | exact |
| R-JF noninterference | consumed trace/state/output comparison | scientific/software | exact |
| frozen recommendation parity | 64-row replay | scientific | exact |
| ledger roundtrip | componentwise prefix+continuation=cumulative | software/scientific | exact |
| cap semantics | scalar/batch cap+1 tests | software | exact |
| projected oracle fidelity | dense reference and high-precision spot checks | numerical | within frozen envelope |
| endpoint fidelity | WRMS difference and local admissibility | numerical/scientific | pass |
| orthogonality | Gram defect versus full MGS | numerical | bounded/no trend failure |
| convergence | tolerance and Krylov/substep sweep | numerical | stable expected behavior |
| reproducibility | repeated fixed-input runs | operational | deterministic counters, stable distributions |
| performance | fixed micro/event substrate | operational | material signal and no ceiling breach |
| diff quality | independent review | review | no P0/P1 blocker |

A candidate is not promoted because tests merely run. Scientific and numerical
checks are independent from software checks.

## 15. PHYS-MATH and PHYS-MATH-CODE audit requirements

### 15.1 PHYS-MATH

For each numerical candidate, audit:

- equivalence of the represented phi combination;
- residual-estimator derivation and limitations;
- dimension and scaling factors;
- happy-breakdown and full-space limits;
- nonnormal counterexamples;
- zero vector, affine/linear, diagonal, Jordan-block, and stiff diffusive cases;
- substep composition and accumulated error semantics.

### 15.2 PHYS-MATH-CODE

Audit:

- equation-to-function mapping;
- actual hot-path use rather than dead experimental code;
- work-counter placement;
- budget reservation before operator invocation;
- allocation/workspace lifetime;
- rollback and failure behavior;
- baseline reproduction;
- benchmark fairness and environment capture;
- missing tests and hidden scope expansion.

One independent diff review is sufficient unless it identifies a material defect
that changes the artifact.

## 16. Process-inflation watchdog

The tournament intentionally generates many technical candidates, not many
assurance layers.

Prohibited process accretion:

- review-of-review chains;
- new mandatory gates without an observed uncovered failure;
- a schema version for wording-only changes;
- repeated 30-shard replay when source and authority inputs are unchanged;
- manifests of existing manifests;
- a new transport harness;
- remote PRs for every killed spike;
- more than one diagnostic retry for the same failure;
- complex ML ranking of a small consumed candidate set;
- feature mining that does not change a candidate decision.

After every candidate phase, ask only whether the durable artifact materially
improved. Two consecutive no-delta cycles classify process accretion and stop
that line.

## 17. Approval boundaries

Already approved by this design:

- local candidate worktrees;
- in-scope internal code changes with TDD;
- non-destructive builds/tests/benchmarks;
- exploratory local timing after the timing-validator implementation exists;
- local commits, durable bundles, and normal feature-branch push/PR under the
  standing repository workflow.

Still requires explicit approval:

- merging any implementation PR;
- force push or direct-main mutation;
- new production dependency;
- public API/file-format break;
- frozen tolerance/convention/policy change;
- active switching;
- tag or release;
- N=2048 execution.

## 18. Completion bar

The v3.8-D tournament design is implemented successfully when:

1. the minimal Timing Authority Validator exists and passes focused tests without
   generating a paired-wall campaign;
2. the baseline benchmark substrate is reproducible and explicitly
   non-authority;
3. at least the K1, K2/K3, C1/C2, and C4 mechanisms receive bounded isolated
   evaluations;
4. every candidate has complete hard-gate evidence and a durable disposition;
5. no frozen recommendation or R-JF authority behavior drifts;
6. survivor ablations identify at most one or two candidates worth a fresh
   holdout;
7. one independent review finds no unresolved P0/P1 blocker in a proposed
   survivor;
8. the final closeout states what improved, what failed, and what remains
   unverified without claiming active speedup.

The tournament may legitimately complete with no survivor. A negative result is
preferable to preserving an optimization that changes physics, hides work, or
only wins under noisy timing.

## 19. Next transition

After this design is committed and reviewed, write a detailed implementation
plan using TDD and isolated worktrees. The plan must begin with the minimal v3.7
Timing Authority Validator and then schedule the first bounded tournament wave.
No performance implementation begins before that plan is approved.
