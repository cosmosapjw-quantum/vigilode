# PHYS-MATH-CODE AUDIT — Stage-Batch Feasibility v0.13

## Equation-to-code map

| Equation/operation | Code path |
|---|---|
| stage RHS \(f(t_i,Y_i)\) | `OdeProblem::eval_rhs_stage_rows` / `eval_rhs_batch` |
| frozen \(J_nv_i\) | `ParallelExecution::apply_operator_rows` |
| \(W=I-h\gamma J_n\) | `stage_batch::benchmark_problem` |
| eight \(W^{-1}b_i\) | `LuFactorization::solve` |
| one multi-RHS \(W^{-1}B\) | `LuFactorization::solve_rows` |
| within-step concurrency | local Rayon pool + atomic concurrency probe |

## P0/P1 findings

### P0 — none in the declared microkernel scope

All numerical paths are exact-identical on the deterministic corpus, and the full existing workspace tests remain green.

### P1 — strict Jacobian-free blocker

`benchmark_problem` materializes a dense Jacobian from basis JVPs and factors dense W. This is deliberately disclosed and excluded from the strict-JF claim. A block/shared-Krylov W solve is required next.

### P1 — integration blocker

The new stage APIs are not yet wired into `StructuredBlockSystem::nonlinear_remainder_snapshot`, homotopy q=0/1/2 propagation, or the adaptive integrator. Therefore the measured speedup is not an integrator speedup.

### P2 — batching specialization

The component-major callback is problem-specific. The next API must make batch layout a first-class physics-kernel contract rather than duplicate formulas ad hoc.

### P2 — Rayon granularity

Per-stage allocation and task scheduling dominate cheap RHS/JVP kernels. Reusable stage workspaces and one batched JVP callback are preferred to eight tiny tasks.

### P2 — failure accounting

A failed stage batch now records the full scheduled batch vector work before returning the error, so speculative failures cannot erase requested work from the ledger.

## Regression evidence

- stage concurrency/order contract;
- shared-operator sequential/Rayon equality;
- failed-batch work accounting;
- deterministic smoke report;
- deterministic CLI schema;
- complete workspace test and strict Clippy gates.

## Verdict

Code path is suitable as a feasibility and handoff layer. It is not yet a production vectorized/Jacobian-free RODAS solver.
