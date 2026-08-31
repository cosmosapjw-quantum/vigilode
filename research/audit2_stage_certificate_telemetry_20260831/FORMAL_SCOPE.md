# Formal scope F01--F05

The formal lane is candidate-free finite-dimensional algebra. It cannot replace
a real-client run or raise the project claim ceiling.

## Obligations

| ID | Statement | Mandatory backends | Boundary |
|---|---|---|---|
| F01 | A strictly lower `s x s` stage matrix is nilpotent, and `(I-T)^-1` is the finite sum `I+T+...+T^(s-1)`. | Lean/mathlib, Rocq | Exact algebra only. |
| F02 | For the declared two-timescale triangular synthetic operator and its exact diagonal Jacobi map, the left and right preconditioned deviations from identity are square-zero. | Wolfram Language, SageMath; Singular numerator-pattern cross-check | Singular alone is not the full rational identity authority. |
| F03 | If `||W^-1|| <= kappa`, then `||W^-1 r|| <= ||x|| + kappa ||r-Wx||` for the declared approximate solve. | Lean/mathlib, Rocq, Wolfram Language | Real/exact inequality; binary64 directed rounding is tested separately in Rust. |
| F04 | For nonnegative strictly lower `T` and `q`, forward substitution computes the nonnegative majorant `(I-T)^-1 q`, and weighted endpoint/estimator contamination bounds follow. | SageMath, Lean/mathlib, Rocq | Synthetic finite stage system only. |
| F05 | From `|E-Ehat| <= Theta`, `Ehat+Theta <= 1` safely accepts and `Ehat-Theta > 1` safely rejects. | Lean/mathlib, Rocq | No statement about how `Theta` was obtained for a real client. |

## Tool roles

- Wolfram Language: exact symbolic matrix and inequality cross-checks.
- SageMath: exact-rational stage-majorant and Jacobi checks.
- Singular: polynomial numerator/square-zero pattern only.
- Lean with mathlib: compiled F01, F03, F04, and F05 proof terms.
- Rocq: an independent compiled proof of the same logical core.
- xAct: not required. The scope contains no tensor-calculus obligation; using
  xAct merely to list an installed package would be tool theatre and is not
  evidence.

All mandatory backend source files, versions, commands, exit codes, stdout and
stderr SHA-256 values, byte lengths, and expected success tokens must be bound
in the compact formal receipt. Raw logs stay outside the repository.

If a backend is unavailable or a proof fails, report
`FORMAL_BACKEND_UNAVAILABLE` or `FORMAL_CHECK_FAILED`; do not substitute a
different theorem, relax an obligation, or mark the formal lane PASS.
