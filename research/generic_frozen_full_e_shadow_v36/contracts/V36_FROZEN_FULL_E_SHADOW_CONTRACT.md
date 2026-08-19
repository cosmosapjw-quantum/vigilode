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
3. if the prefix completes, recommend a full-E shadow iff finite `zeta34 <= tau_zeta`;
4. for a recommendation, resume the retained level-2 object exactly once; prefix recomputation is forbidden;
5. the resumed E endpoint is read-only and cannot modify R-JF state, controller, accepted/rejected sequence, requested outputs, or existing counters.

## Complete speculative ledger

For every recommendation:

- `prefix_work` is the retained level-1+2 cumulative work;
- `full_e_work` is the cumulative pexprb54s4 work after endpoint completion;
- `continuation_work = full_e_work.delta(prefix_work)`;
- `speculative_jvp_after = speculative_jvp_before + prefix_jvp + continuation_jvp`;
- all failed/discarded work remains charged.

Continuation work is not subject to a new post-hoc cap in v3.6; this node measures whether a separate continuation transaction is needed.

## Safety and economics gates

Hard gates:

- zero unsafe full-E shadow recommendations;
- zero hidden or negative work deltas;
- zero prefix recomputations;
- exact R-JF attempt/accepted/trajectory parity against v3.5;
- zero explicit Jacobian builds, direct factorizations, and Newton iterations in the shadow path;
- full continuation work charged to the speculative ledger;
- active switching remains false.

Economics are descriptive in this node. Authority outputs include:

- per-event continuation and total-shadow JVP ratios against the target R-JF attempt;
- cumulative speculative fraction;
- optimized paired whole-profile wall ratios with alternating order and warm-up;
- same proposed physical interval cost ratio `Gamma_E/Gamma_R` for locally admissible recommendations.

No active-polyalgorithm speedup claim is authorized.
