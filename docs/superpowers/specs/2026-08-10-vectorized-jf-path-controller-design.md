# Vectorized/Jacobian-Free Homotopy Path Controller — Design

## Status and recovery boundary

The durable source tree available for this cycle ends at `vectorized-jf-rhs-telemetry-v0.15.0-alpha1` (`0c1b15d`). The previously reported v0.16 integrated-dispatch experiment is preserved only as numerical artifacts under `/mnt/data/rodas5p-v016/repo/research/vectorized_jf_dispatch_v016`; no durable v0.16 source commit or bundle exists. Those artifacts are admissible as comparison evidence, not inherited implementation state.

This cycle therefore advances from the durable v0.15 source and does not silently reconstruct or claim the missing v0.16 implementation.

## Goal

Determine why fixed low-depth partial-coupling homotopy paths reject and whether a small family of nonstationary, Newton-free schedules can reduce the original RODAS5P target defect in at most four propagation rounds without modifying frozen BDF2/Radau comparators.

## Scientific contract

For the 8-stage RODAS5P system

\[
(D-C)K=g+hN(K),
\]

a schedule consists of monotone homotopy endpoints

\[
0=\lambda_0<\lambda_1<\cdots<\lambda_m=1
\]

and per-round parameters \((\theta_r,q_r,\omega_r,c_r)\), where

- \(\theta_r\in[0,1]\) controls frozen linear coupling in the path split;
- \(q_r\in\{0,\ldots,7\}\) is the truncated nilpotent propagation depth;
- \(\omega_r\in(0,1]\) damps the predictor increment;
- \(c_r\in\{0,1,2\}\) is the number of residual corrections at the endpoint.

Changing \(\theta\) between rounds is treated as a nonstationary preconditioned sweep, not as a single classical smooth homotopy path. Reports must state this explicitly.

## Scope

### In scope

- read-only per-round path rejection telemetry;
- fixed schedules reproduced through the new schedule engine;
- bounded nonstationary schedules with at most four rounds;
- direct/common-W factorization as a correctness reference for isolating nonlinear path behavior;
- exact original-target certificate after the schedule;
- comparison against protected sequential RODAS5P and existing fixed homotopy paths;
- deterministic CLI artifact, plots, dual audit, and frozen BDF/Radau diff receipt.

### Out of scope

- runtime common-W dispatcher activation;
- BGMRES-DR tuning;
- BDF/Radau development;
- production certificate replacement;
- global solver speedup claims;
- sparse/GPU/DAE support;
- Anderson acceleration in this cycle.

## Architecture

### 1. Schedule model

`HomotopyRoundSpec` stores one round's endpoint, theta, truncation depth, damping, and correction count. `HomotopyScheduleConfig` validates monotonic endpoints, final endpoint 1, bounded parameters, and at most four rounds for the research gate.

### 2. Schedule engine

A new `run_scheduled_homotopy_path` function reuses the existing partial-path residual, tangent, truncated inverse, and observer hooks. Existing `run_fixed_homotopy_path` remains unchanged. A fixed schedule equivalent to an existing fixed config must produce the same stage vector and path-point values to binary64 roundoff.

Numerical failures produce a partial report with preserved points and work, rather than deleting attempted work.

### 3. Telemetry

Each round records:

- lambda interval, theta, q, damping, corrections;
- path residual before and after;
- original RODAS target residual before and after;
- target-residual ratio;
- predictor increment norm;
- whether AB2 history was valid or reset by a parameter change;
- failure point and reason.

### 4. Research screen

The screen compares:

- fixed q=0,1,2,7;
- q escalation `[0,1,2]`;
- front-loaded propagation `[2,1,1]`;
- persistent q2 `[2,2,2]`;
- mixed `[1,2,2]`;
- frozen-linear-to-decoupled theta ramp `[1,0.5,0]` with q `[2,1,1]`;
- damped q2 with a final correction.

Cases cover affine, complex stiff/oscillatory, Prothero–Robinson, moderate nonnormal, hostile nonnormal, and noncommuting mass systems.

### 5. Decision gates

A schedule is a scientific survivor only if:

- numerical failure rate is zero on non-hostile holdout cases;
- false acceptance is zero under the original RODAS output certificate;
- median rounds are at most 4;
- median final target residual ratio is lower than fixed q2 at equal or lower common-W vector work;
- it does not rely on q=7 in every round.

This cycle does not promote a production solver even if a schedule survives.

## Error handling

- invalid schedules return typed input errors;
- nonfinite path states return a partial numerical outcome with failure reason;
- all attempted RHS/JVP/common-W work remains in counters;
- existing fixed-path and adaptive APIs are regression-tested unchanged.

## Validation

- fixed schedule equivalence test;
- schedule validation tests;
- partial failure preservation test;
- deterministic smoke/canonical report test;
- independent Python analysis of JSON;
- plots of residual ratios, work versus defect, and failure location;
- `cargo fmt`, strict Clippy, full measurement-profile tests, clean-source rebuild.
