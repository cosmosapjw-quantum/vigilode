# Addendum 2026-09-28: F01, F03 and F04 for arbitrary n (audit F-058)

This addendum is new. It does not change `FORMAL_SCOPE.md`, any receipt, or
anything under `research/audit2_stage_certificate_repair_20260831/`. It makes no
PR #42 terminal disposition and does not mark the formal lane PASS.

## Finding

Audit F-058: the formal sources proved F01, F03 and F04 only for the fixed
3x3 matrix and the numeric fixtures. Only F05 was quantified. `FORMAL_SCOPE.md`
states F01 for an `s x s` stage matrix, F03 for any `W` with `||W^-1|| <= kappa`,
and F04 for any nonnegative strictly lower `T` and `q`.

## Rocq backend: compiled

`formal/rocq/StageCertificate.v` keeps every existing fixture theorem and every
existing `Compute` token. A new `Module General` adds these theorems. They are
quantified over every `n : nat` and every matrix that meets the premises.
Matrices are `nat -> nat -> R`, read on indices `< n`.

| Theorem | Statement |
|---|---|
| `General.f01_strict_lower_nilpotent` | `strict_lower n T -> T^n = 0` on `[0,n)^2` |
| `General.f01_finite_neumann_left_inverse` | `strict_lower n T -> (I - T) (sum_{k<n} T^k) = I` |
| `General.f01_finite_neumann_right_inverse` | `strict_lower n T -> (sum_{k<n} T^k) (I - T) = I` |
| `General.f03_residual_solution_norm_bound` | For any additive group `E` with a norm that satisfies the triangle inequality, additive `V` and `W` with `V W = W V = id`, `kappa >= 0` and `||V v|| <= kappa ||v||`: `V b = x + V(b - W x)`, `||V b - x|| <= kappa ||b - W x||`, and `||V b|| <= ||x|| + kappa ||b - W x||`. The right-hand side `b` and the residual `b - W x` are distinct. |
| `General.f04_stage_majorant_nonnegative` | `A >= 0`, `q >= 0` imply `z = (sum_{k<n} A^k) q >= 0` |
| `General.f04_stage_majorant_recurrence` | `strict_lower n A` implies `z_i = q_i + sum_{j<i} A_ij z_j` |
| `General.f04_stage_majorant_dominates` | `A >= 0`, `strict_lower n A`, `d <= q + A d` imply `d <= z` |
| `General.f04_weighted_contamination_bound` | Under the same premises with weights `alpha, beta >= 0`: `alpha.d <= alpha.z` and `beta.d <= beta.z` |
| `General.n4_instance_nilpotent` | A concrete `n = 4` instance of F01, distinct from the 3x3 fixture |

Commands run in this session, with Coq 8.18.0 (OCaml 4.14.1) from Ubuntu apt.
Rocq 9 was not installable. The source header `From Stdlib Require Import ...`
is Rocq 9 syntax, so every command maps `Stdlib` onto Coq 8.18's standard
library through a shim directory of `Stdlib/<M>.v` files. Each shim file is one
`Require Export Coq.<M>.` line. The shim is not part of the repository.

```
coqc -q -noglob -Q shim Stdlib StageCertificate.v              # exit 0, all 8 tokens printed
coqc -q -Q shim Stdlib -Q . SC Check.v                         # Print Assumptions, exit 0
coqchk -o -silent -Q shim Stdlib -Q . SC SC.StageCertificate   # exit 0
```

The compiled file prints the four existing tokens, then three new ones, then
`ROCQ_FORMAL_PASS`. The new tokens are `F01_ROCQ_GENERAL_N_NILPOTENT_AND_NEUMANN_INVERSE`,
`F03_ROCQ_GENERAL_RESIDUAL_SOLUTION_BOUND` and `F04_ROCQ_GENERAL_N_MAJORANT_DOMINATION`.
Compilation takes about 2 s.

`Print Assumptions` reports the same two axioms for each of the eight general
theorems and for `f05_safe_accept` and `f05_safe_reject`:
`ClassicalDedekindReals.sig_forall_dec` and
`FunctionalExtensionality.functional_extensionality_dep`. The Coq 8.18 real
number library brings in both. The development declares no axiom and uses no
`Admitted`.

`coqchk` also checks the whole library. It lists four axioms in the library's
context: the two above, plus `ClassicalDedekindReals.sig_not_dec` and
`Classical_Prop.classic`. It reports no type-in-type, no unsafe fixpoint and no
assumed positivity.

A separate consumer file keeps `n` symbolic. It applies F01 and F04 to an
arbitrary `n` and to the `n = 4` instance, and it compiles.

Not checked:

- The file has not been compiled with Rocq 9.
- It has not been run through `tools/run_audit2_stage_certificate_formal.py`.
  That script requires a mathlib project and records raw products. It was not
  run so that nothing is written under the repair package's `evidence/`
  directory.

## Lean/mathlib backend: FORMAL_BACKEND_UNAVAILABLE

The mathlib toolchain could not be installed. The environment's network policy
returned 403 for these hosts:

- `releases.lean-lang.org`
- `lakecache.blob.core.windows.net`
- `mathlib4.lean-cache.cloud`
- `reservoir.lean-lang.org`

Only the elan binary from GitHub releases was reachable.

The same statements are drafted in `formal/lean/StageCertificateGeneral.lean`.
That file is marked UNCHECKED and has never been compiled. The runner does not
read it; it compiles only `StageCertificate.lean`, which is unchanged.

`FORMAL_SCOPE.md` makes Lean/mathlib mandatory for F01, F03, F04 and F05. The
formal lane therefore stays not PASS for the general statements. The Rocq
proofs above do not substitute for the missing Lean backend.

## Boundary

These are exact real-arithmetic statements. They say nothing about binary64
rounding; the Rust evaluator bounds rounding separately with directed upward
rounding. F03 needs the premise `||W^-1|| <= kappa`, and the Rust side now
verifies it from an inverse witness. None of this is a real-client run, and the
claim ceiling is unchanged.
