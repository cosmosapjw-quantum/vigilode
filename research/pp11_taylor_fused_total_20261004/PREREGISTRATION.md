# PP11 — total-target certificate for the scaled-Taylor fused phi action (prospective registration)

RVJ DAG node `PP11`. Base commit: the commit that adds this file.

## Target and gap

Exact target `F = sum_{k=0}^{4} phi_k(h A) w_k` for the binary64 `A`,
`h`, `w_k` taken as exact reals; it is the top block of `exp(M) v` with
`M = [[hA, W], [0, J]]` (Al-Mohy-Higham 2011). `taylor_phi_action`
reports only an exact-arithmetic tail for the stored binary64 `M~`
(`hA` rounded once), so it is EstimateOnly. This node closes the gap
without reusing the exp-only certificate for the fused target.

## Certificate (new, opt-in)

`certify_fused_phi_total(A, h, w[0..5], candidate)`:
1. `M~` with `fl(h a_ij)` and the exact rest; `Delta = hA - fl(hA)`
   enclosed entrywise (`|Delta_ij| <= |mul_up - mul_down|`), `||Delta||_2`
   bounded by `sqrt(||Delta||_1 ||Delta||_inf)` upward.
2. A directed stepped certificate (REV-02 `certify_exp_action_stepped`
   with the automatic stepping and metric) of `exp(M~) v`, giving its own
   candidate `y` and bound `E1`.
3. Perturbation `||exp(M) v - exp(M~) v||_2 <= ||Delta||_2 e^{omega}
   ||v||_2`, `omega >= max(mu(M), mu(M~), 0)` with `mu` the symmetric-part
   Gershgorin upper bound (logarithmic norm), rounded up.
4. Bound for the caller's candidate `u` (e.g. `taylor_phi_action`'s fused
   output): `||u - F|| <= ||u - top(y)||_up + E1 + perturbation`.
The type is distinct from the exp certificate; a report built for
`exp(A) v` cannot be passed as a fused certificate.

## Fixtures (fixed now)

The R4 POLY-DEV-06 style inputs: symmetric and nonsymmetric `A`, n in
{4, 8, 16}, `||hA||_1` in {0.5, 4, 20}, `h` not a power of two (so `hA`
rounds), five `w_k` seeded; subnormal-scaled `w` (1e-310) and large
(1e100) variants; plus `h` a power of two (no rounding) as a control.

## Gate

G1 Every reported bound is at least the 50-digit error of `u` against `F`
   computed from the exact binary inputs (mpmath `expm` of the exact
   augmented matrix).
G2 The perturbation term is positive whenever `hA` rounds and zero for
   the power-of-two control; type separation is enforced by a compile-time
   distinct type and a contract test.
G3 Reported: bound / error ratios, which term dominates, and the count of
   cases where the certificate is within 1e-10 of `||F||` (usefulness),
   without a gate on usefulness.

## Results (append only after the recorded run)
