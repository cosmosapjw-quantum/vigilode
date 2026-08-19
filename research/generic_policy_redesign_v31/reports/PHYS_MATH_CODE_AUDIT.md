# PHYS-MATH-CODE AUDIT — v3.1

## Verdict

**Implementation PASS / research promotion FAIL.**

## Equation-to-code map

- `D2,D3,D4`: already computed by `pexprb54s4_level2_prefix_resume_level1`.
- physical component count: inherited from `EarlyFlowDefectTelemetryMode::ReadOnly { norm_component_count }`.
- vector geometry: `pexprb54s4_remainder_vector_geometry` in `exponential.rs`.
- serialized level-2 telemetry: optional `remainder_vector_geometry` in `Pexprb54s4Level2PrefixReport`.
- research rows: optional `remainder_chi23`, `remainder_chi34`, `remainder_chi24`, `remainder_q34_perp`, `remainder_delta_chi` in `G4S5B0StageGrowthSafetyRow`.

## TDD evidence

RED:
- missing public geometry API produced compiler `E0432`;
- missing research-row fields produced compiler `E0609`.

GREEN:
- 4/4 pure vector-geometry contracts;
- 1/1 level-2 resumability parity;
- 4/4 early-defect contracts;
- 3/3 tolerance-scaled contracts;
- 1/1 R-JF read-only stage-growth audit contract;
- `cargo fmt --check` PASS;
- integrators+CLI Clippy with `-D warnings` PASS.

## Numerical/work neutrality

The 12 v3.1 replay shards are recursively exact against v3.0 after excluding only wall-clock fields and the five newly added telemetry fields. No new RHS/JVP/phi/Jacobian/Newton action was added by vector telemetry. Runtime E-K continuation remains zero and active switching remains false.

## Discovery results

48 events / 5 audit-E-inadmissible.

Best vector feature by the predeclared robustness ordering: `remainder_chi34`.

- pooled orientation-free AUC: 0.6093;
- N=96: 0.9167, but with only one unsafe event and the *opposite ranking orientation*;
- N=256: 0.7500;
- leave-one-family-out minimum: 0.5405.

Baseline scalar `kappa234` remains stronger pooled (0.6651) and has essentially the same poor LOFO floor (0.5333). Therefore the vector feature does not materially repair the v3.0 failure.

## Ranked issues

- **P0 scientific:** no predeclared vector scalar survives the discovery robustness gate; do not open N=192.
- **P1 mathematical:** Euclidean angle is component-scale dependent for a generic vector ODE.
- **P1 statistical:** only five unsafe events exist, with only one in N=96; high AUC in that dimension is fragile.
- **P2 numerical:** normalized dot products are stable and clamped for roundoff, but their informative deviations are extremely small because remainders are nearly collinear.
- **P3 release:** workspace-wide all-target release closeout remains outside this discovery node's authority unless separately completed.

## What is genuinely fixed

The solver can now inspect retained nonlinear-remainder direction geometry without repeating or extending any pexprb54s4 dependency level.

## What remains uncertain

Whether a *scale-aware higher-order residual of the stage remainders*, rather than their raw angles, carries robust safety information.
