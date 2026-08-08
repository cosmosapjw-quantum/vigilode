# PROMOTE / HOLD — homotopy order-policy + Rayon cycle

## PROMOTE — research infrastructure

- `OutputBudgetPolicy` with absolute, embedded-relative, step-power, and mixed policies;
- dimensionally consistent step-power normalization;
- deterministic calibration/holdout replay;
- local Rayon execution with stable output ordering;
- method-specific trajectory false-accept accounting;
- manufactured-vector order, Prothero–Robinson stiff, and noncommuting-mass nonnormal gates;
- compact summaries and compressed raw evidence;
- same-Rust-backend one/two/four-thread timing harness.

## HOLD — scientific method

- embedded-relative, step-power, or mixed policy as a production acceptance rule;
- any low-depth homotopy path as a default RODAS5P replacement;
- current first-order output correction as a rigorous bound;
- current policy family winners as promoted algorithms;
- wall-time speedup claim;
- manual SIMD implementation;
- adaptive theta/lambda/q scheduling;
- Anderson acceleration;
- BDF/Radau final ranking.

## Quantitative blockers

- holdout false accepts for calibration winner:
  - embedded-relative: 85 family-wide;
  - step-power: 59 family-wide;
  - mixed: 53 family-wide;
- maximum observed oracle/certificate ratio among those false accepts: about 502.5;
- homotopy trajectory gates promoted: 0;
- four-thread end-to-end speedup: about 1.076x;
- low-depth absolute paths fail fifth-order or stiff regression gates;
- fifth-order/stiff-safe q=7 path is not low depth.

## Final decision

The cycle is closed as a verified negative/diagnostic result. The next branch must improve the
nonlinear output certificate before tuning continuation schedules.
