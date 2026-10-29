# Bateman local validation: mathematical scope freeze

This note was fixed before the first Bateman candidate invocation. It integrates
the user-supplied non-coding blocker derivation without changing any PR #39
source byte, authority hash, reference, budget, scenario, or admission rule.

## Inputs

- `Pasted markdown(20260831-031734).md`: SHA-256
  `f01889a4dd46d8d0e87c12bf983605ab1cf78089802a38fc919d6b743cdea016`.
- `VIGILODE_NONCODING_MATH_BLOCKERS_20260831(1).md`: SHA-256
  `6da3065246669fc43ee57eec57aff6063081dded1d24c225087d2bddee6d02bb`.
- `VIGILODE_MATH_BLOCKER_LEDGER_20260831(1).json`: SHA-256
  `f84753318c31f8c8e5d8a578eae1c7bf9c1f90c0ad6713e2581ad071566e8956`.

`MATH_BLOCKER_LEDGER.json` is a semantic JSON mirror of the last input. A
parse-and-compare check is required to return equality; its normalized-byte
SHA-256 is
`7c02043767b0e8d4be9e6b484df132962c359456d2c2e4aa44e2985503731f10`.

### Authority and rounding precedence

The ledger's decimal Bateman arrays are explanatory mathematical material, not
an admission authority. The checked-in PR #39 authority manifest and its
IEEE-754 reference bits are the sole candidate authority. In particular, the
ledger prints the nominal and changed-step stable-daughter entries as
`0.0004997500833125` and `0.0002499375104153596`, whereas the manifest's
nearest-double exact-formula values are `0.0004997500833125041` and
`0.0002499375104153647`. The differences are respectively 38 and 94 ULP and
are each below `5.1e-18`, hence inside the separately frozen `1e-15` reference
uncertainty. This records a pre-observation precedence decision; it neither
changes a reference byte nor relaxes a budget.

The derivation verdict is `COMPLETE_WITH_CLIENT_DEPENDENT_CONSTANTS`. Its
direct PR #39 blocker is still non-mathematical: the exact local scientific
execution and preserved receipt have not occurred.

## Frozen formulas and limits

For stage errors and exact target residuals, the strict-lower block structure
gives

```text
e = -(I-L)^(-1) (I x W^-1) r,  L^s = 0.
```

With `q_i = ||W^-1 r_i||` and the nonnegative stage-propagation majorant `T`,

```text
||delta y|| <= |m|^T (I-T)^(-1) q,
||delta epsilon_emb|| <= |d|^T (I-T)^(-1) q.
```

Thus a computed embedded norm is a certified accept only when its contamination
upper bound is also available. Fifth-order preservation additionally needs a
client-bound statement of the form `sum_i a_i q_i = O(h^6)`. Same-operator or
same-preconditioner identity alone proves neither outer accuracy nor order.

For the exact Bateman Jacobi-preconditioned linear oracle,
`(P^-1 W - I)^2 = 0` (and likewise on the right), so exact unrestarted GMRES has
degree at most two. This is a supporting path-consistency kill test only where
the compact receipt exposes sufficient per-solve telemetry; it is not a new
post-observation admission threshold.

The supplied derivation names the stronger certificate as a minimum
mathematical acceptance contract: a recorded `Ehat + Theta <= 1`, a state
budget such as `sum_i a_i q_i <= B_y`, and, for the Bateman oracle, the
per-solve GMRES-degree kill test. The frozen six-case receipt does not retain
`q_i`, `a_i`, `c_i`, `Theta`, or per-solve GMRES telemetry. Consequently an
adjudicator `ACCEPT` in this node means only that its six frozen PR #39
contracts passed; it is not a certified-accept result in that stronger sense.
No threshold is added here. Obtaining that certificate requires a separately
preregistered instrumentation node with its own source, budget, and review.

## What the frozen six-case receipt can decide

| Ledger IDs | Planned conditional disposition | Reason |
|---|---|---|
| `X01` | `EVALUATED_BY_ELIGIBLE_SEALED_RUN` only after an adjudicated ACCEPT or REJECT; otherwise `NOT_ESTABLISHED` | Only an eligible exact-source six-case receipt can evaluate this empirical blocker. |
| `M02`, `M03` | At most `PARTIALLY_EVALUABLE` for an eligible receipt; otherwise `NOT_EVALUATED` | Finite-precision state and work telemetry may support narrow checks, but the compact receipt is not a full proof trace. |
| `M04`–`M12`, `X02` | `NOT_EVALUATED` | The receipt omits at least one of `q_i`, `a_i`, `c_i`, `Theta`, operator drift/FOV, Lipschitz/radii data, fixed-h refinement, or paired timing. |
| `M01` | `PREEXISTING_MATHEMATICAL_AUTHORITY` | The exact Bateman reference construction was already verified candidate-free. |

A successful PR #39 receipt therefore cannot establish general stage-to-output
accuracy, fifth-order preservation, arbitrary stale-Jacobian order, nonnormal
fast convergence, cross-step reuse safety, root-distance certification,
observable pullback, performance, or production readiness. The project claim
ceiling remains
`EXPLORATORY_NONAUTHORITATIVE_REUSABLE_PRECONDITIONER_TRANSACTIONAL_STEP_SUBSTRATE`.

## No scope mutation

The report and ledger arrived before candidate observation. They are used to
narrow interpretation and enumerate missing evidence. They do not modify the
frozen implementation commit `cac7d1b7337a6dff25a60072009658f6ddf155d9`,
tree `c23abbee0d47e2dbe002e01516bf34e2481bc333`, or its checked-in validator.
The executable source checkout is its direct documentation-only child
`6b00a886c4eb38d3fe199e3d77852cc1eb35eb39`, tree
`4a9ede5c442514f1ae86d018419a2afeee5b6d01`; this packaging amendment does not
alter any scientific or mathematical input.
