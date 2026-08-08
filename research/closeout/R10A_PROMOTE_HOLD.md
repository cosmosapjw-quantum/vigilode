# R10-A allocation/kernel audit — PROMOTE/HOLD closeout

Date: 2026-08-07 Asia/Seoul  
Baseline HEAD: `cbb29dfb32ef4a358f5e77f4320190534543fa36`  
Benchmark code HEAD: `2d3fcaffa688e2cb99d7e79d9f59536fae4dae06`

## PROMOTE — infrastructure

- allocation-free `ShiftedOperator::apply` after construction;
- reusable GMRES, LGMRES and GCRO-DR workspaces;
- Fair A/B session-level scratch reuse;
- recycle-state reset without discarding scratch capacity;
- dedicated diagnostic allocation binary;
- unchanged external true-residual and operator-budget contracts.

Evidence:

- 252 paired rows;
- exact operator-work delta `0` in every row;
- maximum absolute solution-error delta `5.294e-23`;
- baseline failures `0`, current failures `0`;
- GMRES/LGMRES paired wall-ratio medians `0.9918` and `0.9907`; LGMRES/PERSISTENT `0.9836`.

## HOLD — algorithm/default selection

- no default solver change;
- no default STAGE/PERSISTENT recycling;
- no GCRO-DR performance promotion;
- no production/sparse/MPI/GPU/DAE claim;
- no BDF/Radau ranking;
- no homotopy implementation or performance claim.

Blocking evidence:

- GCRO-DR/STAGE paired wall ratio median `1.0151`;
- GCRO-DR/PERSISTENT paired wall ratio median `1.0056`;
- GCRO-DR/OFF allocation events are about `4.59x` GMRES/OFF;
- allocation source has not yet been localized below method-level categories;
- small wall differences remain far below the `1.15x` promotion gate.

## Branch close

This allocation/recycling branch is closed at the pre-homotopy baseline. GCRO-DR hotspot profiling,
`(m,k)` calibration and adaptive admission/reset are deferred until the homotopy stage solver has its own
certified Rust prototype. They will then be tuned and compared together under the same frozen controls.

Next handoff: `research/HOMOTOPY_BRANCH_HANDOFF_KO.md`.
