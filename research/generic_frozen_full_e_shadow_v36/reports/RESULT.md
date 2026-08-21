# VigilODE v3.6 — Frozen Full-E Shadow Economics

## Verdict

`PASS_DESCRIPTIVE_ECONOMICS`

The frozen policy was consumed without retuning:

- committed trajectory: protected sequential matrix-free R-JF;
- `k = 3`;
- `B_abs = 80` JVP vectors;
- `delta = 0.25` on the prefix-only cumulative budget ledger;
- `tau_zeta = 13.39706618860016`;
- active switching: false;
- N=2048: not executed.

This node establishes v3.6 shadow-path contract conformance on the 30 consumed shards and
descriptive shadow economics only. It does not authorize a speedup claim, active switching,
forced-switch fifth-order recovery, a fresh safety claim, release-wide regression completeness,
or state/controller/output parity beyond the emitted deterministic traces.

## Runtime shadow result

All 30 consumed profile/family shards completed. The durable verifier established:

- all 127 prefix-policy event rows are bit-exact against the v3.5 durable shards, including the
  complete R-JF attempt/accepted/trajectory traces after excluding only wall-clock fields;
- all 64 frozen recommendations are bit-exact against the v3.6 ledger preflight;
- 64 retained level-2 resumptions and 64 full-E completions;
- zero prefix-cap breaches, continuation failures, unsafe recommendations, explicit Jacobian
  builds, direct factorizations, and Newton iterations in the shadow path;
- exact componentwise `prefix + continuation = full-E` work round-trips;
- exact causal continuity of both `S_prefix` and `S_total` across every event.

| Ledger scope | R-JF JVP denominator | Prefix JVP | Continuation JVP | Total shadow JVP | Ratio |
|---|---:|---:|---:|---:|---:|
| All 127 events | 388,999 | 2,669 | 1,130 | 3,799 | 0.976609% |
| 64 recommended target attempts | 13,043 | 1,456 | 1,130 | 2,586 | 19.826727% |

The two rows answer different questions. The first is the realized cumulative speculative cost
against all committed R-JF work. The second compares recommended full-E work only with the 64
corresponding target R-JF attempts.

## Optimized paired-wall economics

The `measurement` build ran one R-only calibration, one warm-up pair, and seven measured
alternating-order pairs for each consumed profile. All 35 measured pairs are retained.

| Profile | Recommendations | Realized total JVP / R-JF | Median wall ratio E-shadow/R-JF | Measured range | R-only wall max/min |
|---|---:|---:|---:|---:|---:|
| N=96 | 12 | 0.799244% | 0.970168 | 0.899259–1.069665 | 1.151× |
| N=192 | 13 | 1.193719% | 1.021013 | 0.949447–1.035171 | 1.078× |
| N=256 | 11 | 0.926436% | 1.024548 | 0.870039–1.090244 | 1.273× |
| N=320 | 13 | 0.974746% | 1.028679 | 0.977682–1.071116 | 1.073× |
| N=384 | 15 | 1.011751% | 1.034660 | 0.547436–4.431604 | 13.825× |

The median of the five profile medians is `1.0245478174919398`. This is not promoted to an
overhead or speedup constant. The N=384 campaign is visibly host-noise dominated: the same R-only
suite varied by 13.825× and pair ratios ranged from 0.547× to 4.432×. No pair was excluded and no
favorable repetition was selected.

## Interpretation

The retained-prefix continuation is technically viable and fully accounted. Its cumulative cost
is small relative to all committed R-JF JVP work (0.290489% for continuation alone; 0.976609% for
prefix plus continuation). The paired wall evidence, however, clusters around parity for N=96–320
and is contaminated at N=384. Therefore this node supports neither an active-polyalgorithm
speedup claim nor an active-switching decision.

Any timing replication must preserve this campaign and precommit its host-quality and exclusion
rules before observing new results. Any safety promotion requires a separately committed fresh
shadow holdout. N=2048 remains sealed.
