# Structural production closure: protocol before native execution

Baseline: PR91, `206298b4068d923bb47c7160b111b7439098660c`. The unchanged baseline implementation is retained in `control_source/structural_mean_field_pr91.rs` and compiled in the same benchmark binary.

## Fixed scientific contract

The immutable model is `f(y)=b-d y+kappa mean(y) 1`, finite `d>=kappa>=0`, arbitrary represented state and forcing components. The eight-stage raw-U tableau, observation grid, derivative-free model definition, and prior physical accuracy cap remain unchanged. This work makes no general nonlinear or global production-certification claim.

1. Preserve partial work on every numerical or input failure; complete scheduled independent stage jobs and aggregate their work before deterministic failure selection.
2. Preserve the direct original-space screen as an independently evaluated control. Fuse its temporary arrays while preserving componentwise arithmetic order.
3. Explore an outward-rounded factorized screen. Its bound must cover the full original equations evaluated on the actually represented eight-stage vectors, including initial RHS, mean, and lift rounding. A small scalar residual alone is insufficient.

## Alternative designs

- **Direct fused screen:** original `O(s^2 n)` arithmetic with fewer arrays and passes; preserves the operational f64 residual bit pattern on successful finite rows.
- **Factorized enclosure:** `O(s n+s^2)` work using the two-channel operator, actual lift-error bounds, a compensated-mean error bound, and outward coefficient norms. It may decline when its conservative bound cannot meet the original residual limit. No cap is widened to obtain success.
- **Unscreened two-channel recurrence:** rejected as an implementation alternative because it omits rounding defects of the full-state lift. It is not called an impossibility theorem.

## Validation and timing

Reuse the five existing structural contracts, then add independent exact-rational or interval checks of the new bounds, overflow/refusal/partial-work tests, arbitrary transverse state/forcing, and serial/parallel parity. Existing common-grid physical cap is `1e-6`; existing same-target cap is `1e-11`; operational residual cap for timing is `1e-9`. A separate stricter bound-derived residual-limit diagnostic will preserve failures rather than loosen the limit.

Brief same-binary full-trajectory timings: `n=64,2048`, 64 steps on `[0,1]`, one worker; larger case additionally two workers. Five ABBA/BAAB blocks compare each candidate with the immutable baseline, with at least approximately 0.1 seconds per timing batch where practical. These are descriptive paired results, not confidence intervals. Pools and output serialization remain outside timing; model allocation, plan setup, stage construction, verification, endpoint construction and return allocations remain inside. Fixed-iteration one-arm CLI enables Callgrind instruction-slope profiling; no output inside the loop.

All native builds, timings and profiling use the shared lock `/tmp/vigilode-cargo-20261027.lock`. Rust 1.94.1, locked/offline dependencies, measurement profile, jobs=2. Every execution retains command, exit status, log and source hashes. Initial implementation is followed by at most three diagnosis-driven correction passes. Changes after seeing measurements are labeled as such.
