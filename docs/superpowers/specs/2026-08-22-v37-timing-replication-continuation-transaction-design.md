# v3.7 Timing Replication and Continuation Transaction — Design

## Status and parent authority

Parent authority is public VigilODE `main` at
`2e83cd1052d6276671c6199b06a1301dbb2ab9dd`, tree
`31a9baf921fec9d91157cf5044540cd31638bcbc`, with verdict
`PASS_DESCRIPTIVE_ECONOMICS`.

The v3.6 campaign is immutable evidence. All 35 measured pairs, including the
host-noise-dominated N=384 pairs, remain retained. This design is written before
any v3.7 wall output and before any mutation of the runtime shadow path.

## Goal

Close two P0 ambiguities without activating switching:

1. freeze host-quality and exclusion rules before timing replication; and
2. make endpoint continuation prospectively bounded and fail-closed without
   changing the frozen prefix recommendation chain.

R-JF remains the sole committed trajectory. E remains a read-only shadow.

## Design choices

### Timing replication: whole-campaign authority gate

A timing attempt is one complete five-profile campaign using the existing
measurement binary, six-family whole-suite seam, one warm-up pair, and seven
alternating-order measured pairs per profile. Every pair is retained.

The replication target is three passing campaigns, with at most five campaign
attempts. Failed attempts remain durable and visible. If three campaigns do not
pass within five attempts, the host is classified unsuitable and no timing
claim is promoted.

A campaign is authority-eligible only if:

- the exact source, Rust toolchain, measurement binary SHA-256, host fingerprint,
  CPU affinity, thread-count environment, and timing protocol are unchanged;
- the host fingerprint records kernel, CPU model, logical/physical core counts,
  microcode, NUMA count, frequency governor, and boost/turbo state;
- the thread environment records `RAYON_NUM_THREADS`, `OMP_NUM_THREADS`,
  `OPENBLAS_NUM_THREADS`, `MKL_NUM_THREADS`, `BLIS_NUM_THREADS`,
  `VECLIB_MAXIMUM_THREADS`, and `NUMEXPR_NUM_THREADS`;
- a ten-second aggregate `/proc/stat` probe has
  `idle_fraction = delta(idle)/delta(total) >= 0.90` and
  `steal_fraction = delta(steal)/delta(total) <= 0.001`, where `total` is the
  delta of user+nice+system+idle+iowait+irq+softirq+steal;
- no swap-in or swap-out activity occurs during the preflight/campaign window;
- any exposed thermal-throttle counters do not increase;
- every measured wall and Gamma value is finite and positive and the paired
  proposed-interval denominators remain exact;
- for every profile, each arm's maximum/minimum wall span is at most 1.50; and
- for every profile, the absolute gap between R-first and shadow-first median
  wall ratios is at most 0.10.

A violation marks the entire campaign non-authority. Individual pairs and
profiles are never deleted, replaced, or omitted from descriptive summaries.
The rules do not depend on whether the shadow/R ratio is favorable.

This node still does not authorize a speedup claim: it establishes host-quality
replication for descriptive read-only-shadow economics only.

### Continuation transaction: event-local absolute cap

The frozen prefix policy remains unchanged:

`B^P_k = min(80, (R_k / 4).saturating_sub(S^P_k))`.

Here `R_k` is cumulative committed R-JF JVP work before the target event and
`S^P_k` is cumulative prefix JVP work. Only `S^P` enters future prefix
admission. The frozen `delta=0.25`, `k=3`, and `tau_zeta` are not reinterpreted.

A recommended retained level-2 prefix enters a separate continuation
transaction with an event-local absolute cap

`B^C = 80 JVP vectors`.

The lower-level guard refuses any scalar or batched JVP request whose prospective
vector count would exceed 80 before invoking the operator; a denied batch is not
partially executed. There is no new tuned constant and no new cumulative
continuation fraction. `S_total` continues to record all prefix plus
continuation economics, but it is not an admission authority in v3.7.

This choice is deliberately narrower than feeding `S_total` back into policy:
that alternative would alter policy semantics while still failing to establish
a clean global 25% invariant because prefix admission remains governed by
`S_prefix`. A newly tuned cap above 80 is rejected as post-hoc fitting to the
consumed continuation tail.

### Recommendation and outcome separation

Recommendation remains exactly the frozen prefix witness:

- completed, non-exhausted level-2 prefix;
- finite `zeta34 <= 13.39706618860016`.

Continuation admission and outcome are separate fields. A continuation budget
exhaustion:

- is a charged policy abstention, not a numerical failure;
- emits no full-E endpoint or local-admissibility label;
- preserves all completed continuation work;
- consumes the retained prefix at most once and never recomputes it; and
- cannot mutate R-JF state, controller, attempts, outputs, or counters.

An operator or invariant error remains a hard shadow failure and preserves all
completed work. Recommendation counts must remain bit-exact to v3.6; only the
number of completed endpoints may change under the explicit new cap.

## Consumed-replay regression oracle

Applying the event-local cap to the durable v3.6 continuation ledger predicts:

- 64 frozen recommendations unchanged;
- 62 continuation completions;
- 2 charged continuation-budget exhaustions;
- the two predicted exhaustions are the semilinear ramped events at N=192
  (target attempt 12) and N=384 (target attempt 23), each with durable v3.6
  continuation work of 140 JVP vectors;
- no inference of safety or endpoint admissibility is made for exhausted rows.

This is a consumed semantic regression oracle, not fresh safety evidence.

## Required implementation tests

1. the continuation guard refuses JVP 81 before operator invocation;
2. zero budget performs zero continuation JVPs;
3. every completion/failure/exhaustion round-trips prefix, continuation, and
   cumulative work componentwise;
4. the 62 non-exhausted consumed rows reproduce v3.6 endpoints and deterministic
   fields exactly, excluding wall fields;
5. the two predicted outliers exhaust at 80 with no prefix recomputation;
6. all 64 recommendation decisions remain exact;
7. R-JF attempts, accepted rows, trajectories, and requested outputs remain
   exact;
8. explicit Jacobian builds, direct factorizations, and Newton iterations remain
   zero;
9. timing campaign validation rejects only whole campaigns and retains every raw
   pair and failed attempt.

## Forbidden in this node

- active switching or controller/cache transfer;
- retuning `k`, `B_abs`, `delta`, or `tau_zeta`;
- a continuation cap fitted above 80 from consumed rows;
- feeding continuation cost into the frozen prefix chain;
- individual pair or profile deletion;
- a speedup, fresh-safety, release-wide, or N=2048 claim;
- BDF/Radau optimization or physical-client tuning.

## Next implementation boundary

After this design and its machine-readable contract are committed, implement
only the continuation budget outcome/schema, whole-campaign timing validator,
and their focused tests. Do not generate a new wall campaign until those tests
and the pre-output contract hash gate pass.
