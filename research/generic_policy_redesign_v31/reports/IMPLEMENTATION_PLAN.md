# v3.1 Vector Nonlinear-Remainder Geometry Implementation Plan

**Goal:** Retain directional geometry among already-computed `D2,D3,D4` with zero added solver work, then replay only the already-consumed N=96/256 discovery profiles.

1. Add a pure directional-geometry helper and serializable report with `chi23`, `chi34`, `chi24`, `q34_perp`, and `delta_chi` using the existing physical-component prefix.
2. TDD exact collinear, orthogonal, antiparallel, excluded-tail, zero-norm, and nonfinite limits before wiring the helper into the level-2 prefix report.
3. Add optional v3.1 fields to the existing stage-growth safety row and populate them only from the retained level-2 prefix; do not add RHS/JVP/phi work.
4. Freeze implementation before replay output.
5. Replay N=96/256 family shards only, prove v3.0 non-wall numerical/work parity, and run pooled/cross-dimension/leave-one-family-out single-feature audits without fitting thresholds.
6. Run PHYS-MATH, PHYS-MATH-CODE, and plot CRAG. Open N=192 only if exactly one low-complexity vector witness survives the predeclared robustness gate.
