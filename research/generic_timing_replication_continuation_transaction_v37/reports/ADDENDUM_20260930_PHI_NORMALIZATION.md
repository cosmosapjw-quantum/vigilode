# Addendum: the consumed N=192 replay under the time-normalized phi augmentation (2026-09-30)

Appended note. No sealed v3.7 result, receipt, report or ledger is edited.
V2 and V3 snapshots are unchanged; `results/V37_TRAJECTORY_SNAPSHOT_V4_20260930.json`
records the new values with their attribution.

## What moved

The fused-phi prefix session now runs on the time-normalized augmented
operator (audit 2026-09-30, PHI-P1 / PHI-P2; see
`research/generic_frozen_full_e_shadow_v36/ADDENDUM_20260930_PHI_NORMALIZATION_PINS.md`).
Its Arnoldi residual history, from which the frozen `zeta34` statistic is
formed, differs from the one the frozen v3.x policy was calibrated on.

`StageGrowthCalibration192`, semilinear advection-diffusion:

| Quantity | V3 (integration, 2026-09-29) | V4 (now) |
|---|---:|---:|
| recommendations | 2 (attempts 12, 17) | 1 (attempt 17) |
| continuation budget exhaustions | 1 (attempt 12, 80 JVP) | 0 |
| completions | 1 (attempt 17, 48 JVP) | 1 (attempt 17, 48 JVP) |
| prefix JVP vectors | 116 | 129 |
| continuation JVP vectors | 128 | 48 |
| unsafe recommendations | 0 | 0 |

Attribution: the same replay with only `exponential.rs` reverted (every other
change of the branch applied) gives the V3 values exactly.

The completing row's shadow full-E error is 0.048, with reference local error
1.7e-3 (safe). Across all six families of the StageGrowthCalibration96, 192
and 256 profiles (18 replays, no holdout), no continuation exhausts the
80-JVP cap and no recommendation is unsafe.

## Consequences

- The sealed v3.7 statement that this consumed replay exhausts its
  continuation does not reproduce at head. It stays valid at its sealed
  commit; nothing here is a new result about the policy.
- The frozen tau = 13.397 was fitted on telemetry of the non-normalized
  augmentation. Whether it still separates the classes it was fitted on is
  unknown; it was not refitted, and no holdout was read.
- The abstention semantics (charged, no endpoint, no failure label) are
  now tested with a reduced continuation cap of 24 on the same replay
  (`run_g4_s5b0_v37_continuation_transaction_family_with_cap`), because no
  calibration replay reaches the cap of 80.
