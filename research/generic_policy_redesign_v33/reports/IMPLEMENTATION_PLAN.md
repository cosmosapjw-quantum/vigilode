# v3.3 Implementation Plan

1. Commit the independent-calibration contract before any N=192 solver output.
2. TDD a new `StageGrowthCalibration192` profile with the already predeclared N/atol/rtol.
3. TDD CLI exposure for `generic-stage-growth-safety-audit --profile calibration192`.
4. TDD a pure Python zeta34 selector using only the committed v3.3 rules and synthetic data.
5. Freeze implementation before N=192 output.
6. Run six family-sharded N=192 calibration trajectories; R-JF remains committed and full E is audit label only.
7. Run the frozen selector; do not modify Rust or selector code from calibration results.
8. If all-abstain, stop and keep N=384 sealed. If nontrivial, commit policy + hashes before any N=384 execution.
