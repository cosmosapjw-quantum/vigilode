# PHYS–MATH–CODE AUDIT — v2.8 staged level-2 prefix

## Equation-to-code map

| Mathematical object | Code path | Audit result |
|---|---|---|
| retained level-1 \(D_2\) | `Pexprb54s4Level1Prefix` | reused; not recomputed |
| independent U3/U4 | `pexprb54s4_level2_prefix_resume_level1` | `map_ordered([3,4], ...)` after D2 |
| \(D_3,D_4\) | `nonlinear_remainder(...)` | one RHS/remainder evaluation per stage |
| \(\rho_3,\rho_4\) | `early_flow_defect_telemetry(...)` | same telemetry convention as earlier nodes |
| retained level-2 state | `Pexprb54s4Level2Prefix` | stores d2,d3,d4 and fused actions |
| exact resume to endpoint | `pexprb54s4_fused_step_resume_level2` | tested against one-shot path |
| campaign endpoint prohibition | level-2 research runner | 12/12 shards report `full_e_continuations=0` |
| committed method | R-JF runner state | exact parity against v2.7 trace |

## TDD / resumability evidence

RED was obtained from genuinely missing level-2 APIs. GREEN then established one-shot versus `level1 -> level2 -> resume` equality for:

- final state;
- embedded state;
- error estimate;
- fused-phi reports;
- early telemetry;
- total `WorkCounters`.

Existing `resume_level1` was routed through the new level-2 object, avoiding a second duplicate implementation of the level-2 equations.

## Campaign hard-gate evidence

- 12 family×dimension shards completed.
- 56 level-2 events: 26 at N=128, 30 at N=512.
- level-2 prefix failures: 0.
- full E endpoint continuations: 0.
- switching active: false.
- N=2048 executions: 0.
- v2.8 R-JF trace vs v2.7: exact for declared numerical/controller fields.
- v2.8 event selection and retained level-1 prefix reports vs v2.7: exact.
- historical E-K error is joined only after the new run; it is audit ground truth, not an online input.

## Work ledger

Per event:

- level-2 incremental RHS evaluations = 2 exactly;
- level-2 incremental JVP-vector median = 14 at both dimensions;
- level-2 incremental JVP-vector max = 24 (N=128), 48 (N=512);
- cumulative level1+2 JVP-vector median = 19;
- cumulative max = 33 (N=128), 61 (N=512).

Relative to the target R-JF trial, cumulative prefix vector work has median ~14.4% and P95 ~24%. Relative to historical full-E E-K work, median is ~61.3% and P95 ~68%.

Therefore level 2 is **not** a routine cheap probe in the same sense as level 1. Semilinear N=512 produces the largest safe-work tail, so any future policy must include a prospective speculative-work budget rather than inferring cost from a pooled median.

## Adversarial implementation checks

1. **Hidden endpoint work:** absent by shard report and runner path.
2. **Controller/cache mutation:** R-JF parity passes; prefix object is read-only relative to committed state.
3. **Historical-label leakage:** joined in Python analysis after solver JSON generation; not imported by Rust runner.
4. **Same-output retuning:** contract forbids final threshold; no threshold field is emitted by v2.8 result.
5. **Frozen comparators/build lock:** BDF, Radau and Cargo.lock hashes remain at their prior locked values.
6. **Dimension leakage:** both N=128 and N=512 were in the research campaign; neither is a sealed future holdout after this node. N=2048 remains sealed.

## Failure ledger

- **P0:** no production/active safety threshold may be selected from the observed gap.
- **P0:** active switch and forced-switch fifth-order work remain blocked.
- **P1:** independent stage-growth calibration corpus is missing.
- **P1:** level-2 speculative cost has a family-local tail (especially semilinear N=512).
- **P1:** only one unique unsafe structural geometry supports the observed strict gap.
- **P2:** scalar-atol/component reparameterization issue remains from earlier telemetry nodes.

## Verdict

The staged level-2 factoring and read-only research runner are **implemented and numerically coherent within the focused contract**. The result is scientifically positive because later-stage growth exposes information that level 1 lacked, but it is **non-promotional**: no active policy is authorized, and release-wide regression status must be reported separately from focused v2.8 verification.
