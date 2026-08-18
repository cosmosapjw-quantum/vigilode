# v3.2 Quadratic-Remainder Drift Implementation Plan

1. Freeze the mathematical/data-role contract before solver output.
2. TDD a pure tolerance-weighted quadratic-remainder drift helper:
   - exact leading `c_i^2` cancellation;
   - permutation invariance;
   - common state/atol scaling invariance;
   - physical-prefix clock-tail exclusion;
   - nonfinite/invalid scale fail-closed.
3. Attach the helper to the already-computed pexprb54s4 level-2 prefix report without adding RHS/JVP/phi work.
4. Serialize `zeta23`, `zeta34`, `relative_drift` in the existing stage-growth safety audit row.
5. Close focused regression, Clippy, frozen hashes, then commit before discovery replay.
6. Replay only N=96 and N=256 discovery profiles family-by-family.
7. Verify exact numerical/work parity against v3.1 after dropping only new fields and wall-time fields.
8. Run pooled/dimension/leave-one-family-out single-feature analysis under the predeclared numeric survivor gate.
9. If no witness survives, do not execute N=192. If one survives, stop and freeze it before any N=192 calibration output.
