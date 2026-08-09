# PHYS-MATH AUDIT — Stage-Batch Feasibility v0.13

## Definitions and signs

- Complex Dahlquist real block implements eigenvalues \(-\sigma\pm i\omega\), so damping and phase rotation signs are consistent.
- Oscillatory Prothero–Robinson-like model uses negative stiffness and an exact carrier \(\phi(t)=\sin(\omega t+\varphi_i)\).
- Shifted Rosenbrock operator is \(W=I-h\gamma J_n\) in the identity-mass benchmark.
- All dimensions and stage counts are dimensionless numerical test parameters.

## Known limits

- Sequential and batch RHS/JVP outputs agree exactly.
- Single-RHS and multi-RHS LU outputs agree exactly.
- Four active Rayon tasks are independently observed.
- No inference is made from determinant/eigenvalue data to nonlinear solver conditioning.

## Hidden assumptions

- Identity mass matrix only.
- Frozen JVP at one step state; no stage-varying derivative in this gate.
- Dense factorization setup is excluded from timing.
- Problems are regular ODEs in binary64.
- Microkernel work is deterministic and cache-resident relative to large PDE targets.

## Counterexamples / failure modes

- At N=32 and N=128, thread-per-stage Rayon paths are usually slower.
- JVP stage tasks never reach the 1.15x gate in campaign-median data.
- Multi-RHS LU speedup does not survive automatically if W is matrix-free, poorly preconditioned, or changes inside a round.
- A generic RHS without a supplied batch callback may not realize component-major speedup.

## Verdict

PASS WITH STRICT-JF HOLD.
