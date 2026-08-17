# v2.9 Independent Stage-Growth Safety Calibration — Implementation Plan

Goal: independently calibrate the predeclared normalized stage-growth witness A34 on unseen dimensions/tolerances, freeze the rule, then test it on a sealed safety holdout without activating method switching.

1. Add exact fresh profile variants (N=96, 256, 384) with predeclared tolerances while preserving old profile values bit-for-bit.
2. Add TDD contracts for profile isolation and a read-only safety-audit runner.
3. Implement pathwise JVP-reserve gating before staged level-2 purchase; hard-fail any event exceeding the 80-vector reserve.
4. Resume retained level-2 to full E only as an explicit audit label; audit work is separately ledgered and cannot mutate the R-JF trajectory.
5. Freeze implementation before solver output.
6. Run 12 calibration family shards (N=96,256), select tau only with the predeclared multiplicity-aware rule, and commit the frozen policy before holdout.
7. Run 6 sealed N=384 family shards, no retuning.
8. Perform PHYS-MATH, PHYS-MATH-CODE and plot CRAG audits; package complete Git history and exact source.
