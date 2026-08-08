# Phase 8 — Numerical and Reproducibility Validation

## Determinism

- `native-integrator-gates` was executed twice; serialized bytes were identical.
- The unified smoke screen produced the same scientific checksum at 1 and 4 Rayon threads:

```text
3819bbddaec6fdc27ea01159535e75018ce9e0b1d26f4276f7e94c1c8381db32
```

## Convergence and stiffness coverage

- Fixed-step scalar sweeps used `h = 0.04, 0.02, 0.01, 0.005` over `T=0.2`.
- The stiff gate used `lambda=-1000`, `h=0.01`, four steps.
- The nonlinear mass gate used a noncommuting manufactured mass/Jacobian problem over `T=0.05`.

## Stable and unstable scope

Stable/validated:

- binary64, regular ODEs and nonsingular dense mass matrices;
- fixed-step BDF1/BDF2 and Radau IIA1/3;
- explicit Jacobian or dense materialization from JVP;
- small dense reference solves.

Not validated:

- variable steps or BDF order changes;
- BDF2 endpoint grids requiring a shortened final step without order reduction;
- adaptive Radau error control;
- sparse, MPI/GPU, very large systems;
- singular mass matrices, constraints or high-index DAEs;
- production wall-time ranking.

## Work ledger observation

After the exact-linear Newton termination fix, scalar linear trajectories require one Newton iteration/factorization per step rather than a redundant confirming factorization. This changes work accounting but not numerical results.
