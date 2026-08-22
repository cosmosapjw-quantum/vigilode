# VigilODE v3.7 Continuation Transaction — Result

## Verdict

`PASS_CONSUMED_CONTINUATION_TRANSACTION`

The sealed v3.7 contract was implemented without changing the frozen prefix
policy or activating method switching. Recommended retained level-2 prefixes
now resume under a separate event-local cap of 80 JVP vectors. Prospective JVP
81 is refused before operator invocation, including atomic denial of an
otherwise partially admissible row batch.

## Runtime result

The consumed five-profile, six-family replay produced exactly 30 reports and
preserved all 127 v3.6 prefix-policy rows and all 64 frozen recommendation
decisions.

- frozen recommendations: 64;
- bounded continuation completions: 62;
- charged continuation-budget exhaustions: 2;
- numerical continuation failures: 0;
- unsafe recommendations: 0;
- prefix or continuation budget breaches: 0;
- explicit Jacobian builds, direct factorizations, and nonlinear/Newton work in
  the shadow path: 0;
- active switching: false;
- N=2048 execution: false.

The two charged abstentions are exactly:

1. semilinear-advection-diffusion-ramped, N=192, target attempt 12;
2. semilinear-advection-diffusion-ramped, N=384, target attempt 23.

Both consume 80 continuation JVP vectors, emit no full-E endpoint, no endpoint
error, no admissibility label, and no numerical-failure label.

## Work ledger

Across all 127 policy rows:

- committed R-JF JVP vectors: 388,999;
- speculative prefix JVP vectors: 2,669;
- charged continuation JVP vectors: 1,010;
- total speculative JVP vectors: 3,679.

At the 64 recommended target attempts:

- target R-JF JVP vectors: 13,043;
- retained-prefix JVP vectors: 1,456;
- charged continuation JVP vectors: 1,010;
- total bounded full-E attempt work: 2,466 JVP vectors.

Only `S_prefix` remains prefix-admission authority. `S_total` records the
bounded prefix-plus-continuation economics and therefore intentionally diverges
from v3.6 after a capped continuation abstention.

## Compatibility evidence

- v3.6 schemas and APIs remain separate and unchanged;
- all 62 completed v3.7 endpoints match their v3.6 deterministic endpoint and
  work fields exactly, excluding declared wall fields;
- R-JF attempt rows, accepted rows, and trajectories are exact to v3.6 after
  excluding declared wall fields;
- the seven durable v3.6 derived products reproduce byte-exactly.

## Claim ceiling

This node is consumed implementation-regression evidence. It does not establish
fresh shadow safety, timing replication, speedup, active switching,
controller/cache transfer, release-wide completeness, or N=2048 behavior.
