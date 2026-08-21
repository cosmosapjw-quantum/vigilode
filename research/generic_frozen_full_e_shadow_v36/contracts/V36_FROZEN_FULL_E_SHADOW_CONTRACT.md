# v3.6 Frozen Full-E Shadow Economics Contract

## Parent and immutable policy

Parent scientific state: VigilODE v3.5 enforced speculative-prefix budget.

The following values are immutable in this node:

- committed trajectory: protected sequential matrix-free R-JF;
- persistence trigger: `k = 3`;
- absolute prefix cap: `B_abs = 80` JVP vectors;
- cumulative speculative fraction: `delta = 0.25`;
- quadratic-drift recommendation threshold: `tau_zeta = 13.39706618860016`;
- active switching: disabled;
- N=2048: sealed.

## Runtime shadow decision

For each causal event target:

1. compute the existing transactional level-1+2 pexprb54s4 prefix under `B_k`;
2. if the prefix exhausts its budget, charge all used work and abstain to R-JF;
3. if the prefix completes without any cap breach, recommend a full-E shadow iff finite `zeta34 <= tau_zeta`;
4. for a recommendation, resume the retained level-2 object exactly once; prefix recomputation is forbidden;
5. the resumed E endpoint is read-only and cannot modify R-JF state, controller, accepted/rejected sequence, requested outputs, or existing counters.

## Complete speculative ledger

For every recommendation:

- `prefix_work` is the retained level-1+2 cumulative work;
- `full_e_work` is the cumulative pexprb54s4 work after endpoint completion;
- `continuation_work = full_e_work.delta(prefix_work)`;
- `total_speculative_jvp_after = total_speculative_jvp_before + prefix_jvp + continuation_jvp`;
- all completed work on failed/discarded paths remains charged; a failing
  operator invocation itself follows the repository's completed-operation
  `WorkCounters` convention.

Continuation work is not subject to a new post-hoc cap in v3.6; this node measures whether a separate continuation transaction is needed.

Two ledgers are intentionally distinct.  The frozen v3.5 prefix budget keeps
its original prefix-only state

`S_prefix_after = S_prefix_before + prefix_jvp`,

and only `S_prefix` enters
`B_k=min(80,(R_k/4).saturating_sub(S_prefix_k))`.  The new
`S_total` ledger additionally charges continuation work for economics and
auditability, but does not feed back into later prefix admission in this node.
This preserves the already-frozen recommendation chain instead of silently
changing the v3.5 policy after observing continuation costs.

An endpoint continuation failure is a hard shadow failure.  Every completed
counter before the error remains in `full_e_work`/`continuation_work`; the row
is neither discarded nor treated as an admissible recommendation.

Any lower-layer prefix-cap overshoot is fail-closed: it is recorded as a hard
gate failure and cannot launch the endpoint continuation.

## Safety and economics gates

Hard gates:

- zero unsafe full-E shadow recommendations;
- zero hidden or negative work deltas;
- positive committed-R denominator and finite realized work ratios;
- zero prefix recomputations;
- exact R-JF attempt/accepted/trajectory parity against v3.5;
- zero explicit Jacobian builds, direct factorizations, and Newton iterations in the shadow path;
- full continuation work charged to the speculative ledger;
- active switching remains false.

Economics are descriptive in this node. Authority outputs include:

- per-event continuation and total-shadow JVP ratios against the target R-JF attempt;
- observed prefix, continuation, and total-speculative JVP fractions against
  cumulative committed R-JF JVP work;
- optimized paired whole-profile wall ratios with alternating order and warm-up;
- same proposed physical interval cost ratio `Gamma_E/Gamma_R` for locally admissible recommendations.

No active-polyalgorithm speedup claim is authorized.

The frozen `delta=0.25` policy constant is serialized separately as
`frozen_cumulative_prefix_budget_fraction`; it is not a realized cost ratio.

R-JF parity excludes nondeterministic wall-clock fields only.  Every available
deterministic attempt, accepted-step, trajectory, and work field must match
bitwise.  State/controller/output parity may be claimed only if the
implementation emits and matches explicit deterministic digests; trace parity
alone is not promoted to a stronger state claim.

## Frozen paired-wall protocol

Runtime economics use the existing repository canonical timing convention:

- optimized `measurement` build only;
- the authority profile is attested at compile time from Cargo's target
  profile directory (`target/measurement`); Cargo's raw build-script
  `PROFILE=release` alias is recorded separately and is not accepted as the
  authority identifier;
- one whole-profile suite contains all six frozen families at the selected
  dimension/tolerance profile;
- calibrate repetitions on the R-JF-only arm by doubling from one until at
  least 0.25 wall seconds or 1024 repetitions;
- one alternating-order warm-up pair;
- seven measured pairs, alternating R-first and shadow-first;
- preserve every pair rather than selecting a favorable repetition;
- `Gamma = wall_seconds / sum(abs(proposed_attempt_h))`; exact R-JF trace
  parity makes the denominators identical between arms.

Both arms time the same raw execution seam; report packaging, hard-gate
aggregation, and provenance strings remain outside the timers.

The consumed N=96/192/256/320/384 profiles may establish implementation and
descriptive economics only.  They are not fresh safety evidence.  Any new
shadow holdout must receive a separate committed profile/coverage contract
before its first solver output.  N=2048 remains sealed.
