## Task 4: T-verify — exact verification of the plan's numerical claims (node id `t-verify`)

New research node `research/production_theory_20261021/` with `PREREGISTRATION.md` (written and committed
before running), `verify_theory.py` (Python 3, sympy/numpy; `pip install --break-system-packages sympy`
if missing), raw results `RESULTS.json`, and one appended row in `research/LEDGER.jsonl` following
`docs/RESEARCH_LEDGER.md` (next free id after the last row; `outputs_sha256` covering `RESULTS.json`;
`commit` = the commit containing the script, which must be pushed). Run `tools/check-research-node.py
--base origin/production/vigilode-plan-20261021` and record its exit code.

Claims to verify from `fixtures/rodas5p_coefficients_snapshot.json` (exact rationals of the decimal
literals, sympy):
1. T1: output sensitivity polynomials m_j(w), w = W⁻¹, of the U-form stage recursion; their values at
   w = 1 (z = 0): |m| = (0.577, 0.237, 6.246, 5.966, 3.729, 4.568, 1.011, 1.000), sum 23.335; the
   coefficient-sum bounds (100.456, 15.401, 24.467, 18.847, 13.655, 14.583, 1.011, 1.000), sum 189.418;
   the scalar exact sensitivities at z ∈ {−1, −10, −100} (sums 17.819, 3.825, 0.125).
2. The preconditioned-residual sensitivity bᵀ(I − T(z))⁻¹: max entry ≤ 6.25 and sum in [1, 23.34] on a grid
   z ∈ −logspace(−4, 8, 400); report the true max and min found.
3. Lemma 3: R(z) = (z/2)/((1 − γz)(e^{z/2} − 1)) is increasing in −z with limits 1 and 1/(2γ); verify
   the derivative sign symbolically for w = −z/2 > 0 where possible, otherwise on a 10⁴-point grid in
   exact-rational or mpmath 50-digit arithmetic, and report which.
4. T2 Corollary 2a: for P = I − h_old γ J₀ and scalar J₀ = λ ≤ 0, ‖I − P⁻¹W‖ ≤ 2|1 − h/h_old|.
5. T9 example: a = (4096, 1, 1, 1), q = 1/2, B = 1/16 gives 68 iterations for the common threshold and 36
   for (18, 6, 6, 6), exactly.
6. The stage-homotopy determinant claim det(∂H/∂U) = det(W)^s for a random 3×3 J₀ and the RODAS5P A, C.
Any claim that fails is recorded as FAIL with the observed value; the plan document is not edited by this
node (a correction note goes in the node report).

