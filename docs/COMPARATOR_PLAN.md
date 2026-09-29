# External comparators

External re-audit RA-09, 2026-09-29.

The in-tree BDF and Radau arms are labelled `reference-implementation-only`
(audit F-052, F-056), and relative verdicts against them are
`NotEvaluated`. That label blocks performance headlines against those arms.
It does not block comparison: the plan is to compare against mature external
solvers through adapters, not to grow the in-tree BDF/Radau into production
solvers first.

## Candidates

| Solver | Role |
|---|---|
| SUNDIALS CVODE (BDF 1-5, matrix-free Krylov with JVP, preconditioner setup and reuse) | primary large-state JVP-only BDF comparator |
| Hairer-Wanner RADAU5 (and RADAU, orders 5/9/13) | high-accuracy Radau comparator with dense output and mass matrices |
| SciPy `BDF` / `Radau` | immediate external correctness control; Python overhead kept separate from any timing |
| SciML `Rodas5P`, `QNDF`, `FBDF`, `RadauIIA5` | optional; `Rodas5P` is the only independent implementation of the same tableau (Hairer's `RODAS` is order 4(3), not RODAS5P) |

CVODE is an ODE solver; singular-mass DAEs belong to IDA and are out of
scope here.

## What an adapter must record

A comparator row is admissible only with:

- the original callback, linearization and preconditioner supply of the
  problem, with nothing precomputed for one arm only;
- source or dependency revision, thread count and build profile;
- the actual output grid and the stopping norm, mapped to the same case WRMS
  norm as the VigilODE arms;
- all failed and rejected work, and partial failures kept as failures (no
  silent snap to the endpoint);
- the mass-matrix treatment.

An external implementation is not Tier A by reputation; it earns
`comparative_reading = permitted` through the adapter checks above.

## Two separate questions

1. **Algorithmic ablation**: the same linear-algebra backend and the same
   callbacks for every arm, to attribute a difference to the method.
2. **End-to-end comparison**: each production implementation at its best
   legal settings, to measure what a user gets.

Their numbers are not mixed. Wall-time verdicts in either use the paired
protocol in `crates/rodas5p-fair-ab/src/paired_timing.rs` with independent
process blocks and an A/A control.

## Status

No external adapter exists in the tree beyond the SciPy reference path; no
comparison has been run.
