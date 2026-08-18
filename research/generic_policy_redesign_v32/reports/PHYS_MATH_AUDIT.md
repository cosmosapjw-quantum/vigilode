# v3.2 PHYS–MATH AUDIT — Quadratic-Remainder Drift

## Verdict

PASS for the diagnostic derivation and dimensional/coordinate scope; PASS-WITH-CONSTRAINT for discovery significance. `zeta34` is a discovery survivor, not a safety theorem and not a threshold policy.

## Definitions

With pexprb54s4 stage fractions

- c2 = 1/4,
- c3 = 1/2,
- c4 = 9/10,

and nonlinear remainders `D_i = F(y+u_i)-F(y)-J u_i`, define

`Z23 = D3/c3^2 - D2/c2^2`,
`Z34 = D4/c4^2 - D3/c3^2`.

For each pair, use the physical-component tolerance scale

`s_k^{ij}=atol+rtol*max(|y_n,k|,|U_i,k|,|U_j,k|)`

and

`zeta_ij=|h| sqrt(mean_k[(Z_ij,k/s_k^{ij})^2])`.

The second predeclared authority feature is

`r_Z=(zeta34-zeta23)/(zeta34+zeta23)`

with exact zero if both zetas vanish.

## Leading-order cancellation lemma

Assume F is C^3 near the stage tube and

`u_i = c_i h F_n + h^2 a_i + O(h^3)`.

Taylor expansion gives

`D_i = 1/2 c_i^2 h^2 H[F_n,F_n]
      + h^3 { c_i H[F_n,a_i] + (c_i^3/6) T[F_n,F_n,F_n] }
      + O(h^4)`.

Therefore

`D_i/c_i^2 = 1/2 h^2 H[F_n,F_n]
            + h^3 { H[F_n,a_i]/c_i + (c_i/6) T[F_n,F_n,F_n] }
            + O(h^4)`.

Hence the common O(h^2) Hessian contribution cancels in Z_ij and

`Z_ij = O(h^3)`.

Important limitation: Z_ij is not a pure third-derivative observable. It also contains differences in the O(h^2) stage corrections `a_i` through the Hessian term.

If the pairwise tolerance scale stays nondegenerate, `zeta_ij` is O(h^4) in the small-step limit.

## Dimension and coordinate audit

- D_i has the same state/time dimension as F.
- Z_ij has the same state/time dimension.
- |h| Z_ij has the state dimension.
- dividing componentwise by the solver tolerance scale makes the WRMS reduction dimensionless when component units are compatible with the scalar-atol contract.
- common state scaling together with the same common scaling of atol leaves zeta invariant.
- component permutation leaves zeta invariant.
- arbitrary independent component rescaling is NOT claimed under scalar atol.
- the time-augmentation clock tail is excluded exactly as in the existing early-flow telemetry.

## Known limits / counterexamples

1. Exact quadratic-leading model `D_i = c_i^2 q` gives Z23=Z34=0 exactly.
2. If Z23=Z34=0, relative drift is defined as 0 rather than 0/0.
3. Nonfinite required physical components make the affected diagnostic null; no silent finite imputation is permitted.
4. A large zeta34 does not mathematically imply E-K inadmissibility; discovery safe events reach values larger than some unsafe events.

## Data-alignment verdict

On the fixed N=96/N=256 discovery replay (48 events, 5 unsafe):

- zeta34 pooled orientation-free AUC = 0.7674418605,
- minimum dimension AUC = 0.7105263158,
- minimum leave-one-family-out AUC = 0.6875,
- unsafe orientation is higher-zeta34 in both dimensions.

This satisfies every predeclared v3.2 discovery gate.

`relative_drift` fails: pooled AUC 0.5767 and minimum dimension AUC 0.5395. The phase plot shows zeta23 and zeta34 are strongly co-moving, so normalization by their sum discards much of the useful absolute drift level.

## Claim ceiling

Allowed: “after cancelling the shared leading quadratic remainder, the later tolerance-weighted drift zeta34 carries more robust safety-ranking information on the discovery corpus than the prior scalar-shape/vector-direction controls.”

Not allowed: “zeta34 is a generic safety certificate”, “zeta34 has a validated numerical threshold”, or any active switching claim.
