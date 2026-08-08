# Phase 10 PROMOTE/HOLD — global-error Pareto authority v0.10

## PROMOTE

- exact analytic reference provenance;
- common output grid with missing-time rejection;
- external endpoint/max/RMS L2 and WRMS errors;
- vector-valued work ledger;
- Pareto front and target-attainment construction;
- calibrated randomized one-thread timing;
- local-Rayon throughput and scientific determinism;
- `global-error-pareto` CLI.

## HOLD

- general solver ranking;
- adaptive BDF/Radau comparison;
- sparse/PDE/DAE/MPI/GPU performance;
- peak-memory claims;
- any claim that current T4 throughput is stage-parallel speedup.

## Blockers transferred

1. common output observer / no internal-step storage;
2. adaptive grid landing or validated dense output;
3. larger stiff/nonnormal/sparse corpus;
4. non-analytic reference hierarchy;
5. adaptive BDF and Radau implementations.

## Final closeout boundary

This cycle validates the comparison authority, not a production solver ranking. The canonical fixed-step corpus is retained as a deterministic mechanics test. Production work-precision conclusions require the successor branch to eliminate every-step storage bias, admit adaptive trajectories on a common output contract, enlarge the stiff/nonnormal/sparse corpus, and establish a reference uncertainty hierarchy.

Frozen tag: `unified-global-error-v0.10.0-alpha1`
Successor branch: `unified-adaptive-output-v0.11`
