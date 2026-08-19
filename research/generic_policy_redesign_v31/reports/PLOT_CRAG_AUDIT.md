# PLOT-DRIVEN CRAG AUDIT — v3.1

## C — Correctness

The plots use the 48 frozen discovery events and the authority audit full-E error. Unsafe points remain embedded in the safe cloud in both `chi34` and the `(chi23,chi34)` phase plane. No visual threshold is selected.

## R — Retrieval

Primary exponential-Rosenbrock formulations use nonlinear remainders `D_i` as vector inputs to matrix-function weighted stage/endpoint combinations. The literature supports retaining the vector remainders as meaningful method objects, but does not provide a theorem that their Euclidean mutual angles certify local admissibility.

Reference: Hochbruck, Ostermann & Schweitzer (SIAM JNA 2009, DOI 10.1137/080717717); parallel stage formulation in the pexprb54s4 construction literature.

## A — Augmented adversarial checks

1. **Dimension mutation:** `chi34` changes orientation: N=96 ranks lower values as unsafe, N=256 ranks higher values as unsafe.
2. **Family mutation:** LOFO `chi34` AUC drops to 0.5405 when Van der Pol is removed; excluding HIRES also changes the preferred orientation.
3. **Feature mutation:** `q34_perp` contains essentially the same information as `|chi34|` near +1 and gives identical rank metrics here.
4. **Scale mutation:** component-wise rescaling can change Euclidean cosines, so generic interpretation is not invariant.
5. **Baseline contrast:** vector direction does not improve pooled ranking over scalar `kappa234`.

## G — Generation

The observed near-collinearity and the Taylor expansion both predict that the next useful object should cancel the common leading quadratic Hessian contribution rather than inspect its direction. A natural candidate is stage-scaled remainder drift,

\[
Z_{ij}=\frac{D_j}{c_j^2}-\frac{D_i}{c_i^2},
\]

measured in a common tolerance-weighted metric. The leading `O(h^2)` Hessian term cancels, so this targets higher-order variation directly.

## Claim status

- Read-only vector telemetry: **SURVIVES**.
- Raw vector-direction safety witness: **REJECTED**.
- N=192 independent calibration: **REMAINS BLOCKED**.
