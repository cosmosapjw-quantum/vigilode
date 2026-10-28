# Node t-verify: exact verification of the production plan's numerical claims

Branch `production/t-verify` from 9b5d083. Python only (sympy 1.14.0, mpmath, numpy, seed 20261021); no cargo.
Research node: `research/production_theory_20261021/` (`PREREGISTRATION.md` committed and pushed as 5811ca9 before the run;
`verify_theory.py` and `RESULTS.json` in 81069f9; ledger row L-0137 in a following commit citing 81069f9).

## Commands and exit codes

- `python3 research/production_theory_20261021/verify_theory.py --out research/production_theory_20261021/RESULTS.json`: exit 0, about 11 s.
  A dry run to a scratch path preceded it (same script apart from one dead line removed); a second run reproduced `RESULTS.json` byte for byte.
- `python3 tools/check-research-node.py --base origin/production/vigilode-plan-20261021`: exit 1. The only failures are two
  `L-0124` output hash mismatches (`research/production_audit_20261018/CONTINUATION_DAG.json`, `CONTINUATION_FINDINGS.json`), rows and
  files that this node does not touch (they are identical to the base ref). No failure concerns the new node or L-0137. The checker was not modified.

## Results (all gates met; ledger verdict PASS)

| Claim | Status | Observed |
|---|---|---|
| 1a T1 `abs(m_j(w=1))` | PASS | 0.57734, 0.23705, 6.24636, 5.96585, 3.72942, 4.56782, 1.01099, 1.00000; sum 23.3348 (claimed 23.335) |
| 1b coefficient-sum bounds | PASS | 100.4561, 15.4006, 24.4668, 18.8468, 13.6546, 14.5825, 1.0110, 1.0000; sum 189.4184 (claimed 189.418) |
| 1c exact sums at z = -1, -10, -100 | PASS | 17.8191, 3.82467, 0.125461 (claimed 17.819, 3.825, 0.125). Extra: z = -0.1: 22.695, -3: 11.658, -1000: 0.005599 |
| 2 `b^T (I-T)^-1` on z = -10^t, 400 points | PASS | max entry 6.24632 (at t = -4), abs-sum max 23.33468 (t = -4), abs-sum min 1.0000019 (t = 8); signed sum between 1.0000002 and 4.7182 |
| 3 Lemma 3 | PASS (grid) | limits by sympy: R -> 1 and R -> 25000000000000000/10596878159714507 = 1/(2 gamma) = 2.35919. On 10^4 log-spaced x in [1e-8, 1e4], mpmath 50 digits: min dR/dx = 5.56e-8 (at x = 1e4), R in [1.00000000076, 2.358629]. This is a grid check, not a symbolic proof |
| 4 Corollary 2a | PASS | `1 - W/P = (t/(1+t)) (1 - h/h_old)` with t = h_old gamma abs(lambda); exact-rational grid of 5520 points never exceeds the bound; worst ratio to `abs(1 - h/h_old)` is 0.999999999999. Extra: 2000 random symmetric NSD 3x3 J0 in the 2-norm, worst ratio to the plan's bound 0.49999 |
| 5 T9 example | PASS | common k = 17 (4099/65536 > 1/16 >= 4099/131072), total 68; (18,6,6,6): sum exactly 1/16, total 36. Exhaustive integer optimum: 36, unique at (18,6,6,6) |
| 6 stage-homotopy determinant | PASS | for (lambda, theta) in (1,0), (1,1/2), (1,1), (1/2,0), (1/2,1/2), (0,1): diagonal blocks exactly W, upper blocks zero, exact rational `det = det(W)^8` for N = 0; with a nonlinear N (sin, exp, products) the 24x24 symbolic Jacobian at random points gives relative error 2e-50 against det(W)^8 |

## Corrections for the plan (not edited here)

1. T2 Corollary 2a: the identity `T = (1 - h/h_old)(P^-1 - I)` equals `P^-1 W - I`, the negative of `T = I - P^-1 W` used in the proof. Norms are unaffected. The bound `2|1 - h/h_old|` is valid but loose by a factor 2: the exact scalar constant is `t/(1+t) < 1`, so `q <= |1 - h/h_old|` holds in the scalar case. A 25% step change gives q below 1/4 in the scalar case, not only 1/2. For non-scalar J0 the plan's bound `1 + ||P^-1||` stands; the random NSD check did not exceed half of it.
2. T3 Lemma 3 says monotonicity "was checked on a 2000-point grid"; this node checked 10^4 points at 50 digits. It remains numerical evidence, not a proof.
3. T1 text "100, 15, 24, 19, 14, 15, 1, 1": observed values round correctly (100.46, 15.40, 24.47, 18.85, 13.65, 14.58, 1.01, 1.00); the rounded "24" and "19" are 24.47 and 18.85, within rounding of the claim, no discrepancy.
4. Definitions that the brief fixed and the plan leaves implicit: `m = b^T (I-T)^-1 w` with `T_ij = gamma C_ij w + A_ij (w-1)`; m_j are polynomials in w of degree 8, 7, 6, 5, 4, 3, 2, 1 for j = 1..8, not degree 8 for all stages.
5. Claim 2 was read with absolute values for "entry" and "sum" (stated in the preregistration); the signed sum lies in [1.0000002, 4.718].

## Deviations

None from the brief. The tool-checker failure on L-0124 is outside the node and recorded, not repaired.
