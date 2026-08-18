# VigilODE

**Causal, Budgeted Jacobian-Free Polyalgorithms for Stiff and Oscillatory ODEs**

VigilODE is a research platform for generic stiff and oscillatory ordinary differential equations. The project studies evidence-gated polyalgorithm design that combines a protected matrix-free Rosenbrock path with guarded exponential-Rosenbrock candidates while preserving complete speculative-work accounting.

## Current research scope

The current program focuses on:

- matrix-free, Jacobian-free stiff integration;
- explicit-Jacobian-free and Newton-free fast-path candidates;
- resumable Krylov / exponential-Rosenbrock stage prefixes;
- causal regime telemetry and event-conditioned method admission;
- tolerance-aware nonlinear-flow diagnostics;
- transactional speculative-work budgets;
- same-error, same-output, failure-preserving performance comparisons.

Active method switching is treated as a research target rather than a completed production capability. Safety, fifth-order recovery after real switches, controller/cache transfer, and full same-error economics remain explicit validation gates.

## Development policy

Scientific claims are evidence-gated. Research branches retain failed, speculative, fallback, and diagnostic work in their ledgers. Comparator implementations are not silently tuned to improve headline results, and holdout problems are not used to retune core policy thresholds.

The repository is being initialized from the project's durable research history. Subsequent research nodes are developed through pull requests with validation receipts and claim-scope notes.

## License

MIT
