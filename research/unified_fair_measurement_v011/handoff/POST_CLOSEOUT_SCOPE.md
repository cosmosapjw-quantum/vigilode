# Post-closeout scope

The fixed-step measurement-authority phase is closed.

Any later project may branch from the final tag to implement:

1. variable-order adaptive BDF 1–5;
2. adaptive Radau IIA3 with a validated estimator or dense output;
3. parallel SDC and Peer/W;
4. Rosenbrock–Krylov, BOROK and exponential/Leja;
5. sparse, DAE, MPI or GPU test corpora.

Such work must retain:

- common external output grids;
- explicit output policy metadata;
- requested-output-only storage accounting;
- preserved failures;
- global-error matched Pareto comparison;
- thread-count scientific determinism.
