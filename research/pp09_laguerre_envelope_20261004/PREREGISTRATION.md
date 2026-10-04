# PP09 — where the stiff Laguerre bound is lost, and a certified finite-degree envelope (prospective registration)

RVJ DAG node `PP09` (depends on PP08, L-0071). Base commit: the commit
that adds this file. The six large-h-rho budget rejections of R-NEXT-04
(L-0047) stay historical results; no admission changes in this node.

## Part A: decomposition (published data only)

From `research/rnext04_laguerre_admission_20261003/cases.json`, for every
Laguerre case with `h rho >= 20` (the six rejections included): the
components truncation, coefficient, recurrence_adjoint, summation,
normalization as fractions of the total, and which term is the
bottleneck.

## Part B: certified envelope

The coefficient and tail terms use `||L_n(X)|| <= e^{L'/2}` on the
verified spectrum `[0, L']`. A certified finite-degree replacement
`E_n = max_{x in [0, L']} |L_n(x)|` is computed for `n <= 128` with
interval arithmetic (mpmath `iv`, 60 digits): `[0, L']` split into
subintervals, the three-term recurrence evaluated on each in interval
arithmetic, subdivided until each enclosure is within 1 % of its upper
end or 2^20 pieces. The bound uses the maximum upper end. Then the
coefficient and tail terms of the six cases are recomputed with `E_n` in
place of `e^{L'/2}` (post hoc, from published coefficient radii and tails
where the fields allow it; otherwise only the ratio `e^{L'/2} / max E_n`
is reported).

## Gate

G1 Part A covers every case with `h rho >= 20` in the published file.
G2 Every `E_n` is an enclosure: checked against 60-digit point
   evaluations at 10,000 points per n (no point above the bound).
G3 Reported, not gated: the factor gained and whether the recomputed
   totals would meet the published budgets (which would justify a native
   implementation next; this node does not admit anything).

## Results (append only after the recorded run)

Recorded run at source commit `a561e6f592c3ff38270dfed336f4c1c3942eb519` (the tool was committed with no outputs,
after a self-test on n <= 5 over [0, 10] that used no case data). Command, from the repository root, run once,
exit 0, 67.6 s wall clock with 4 worker processes (envelope 21.0 s, G2 point check 11.6 s):

`python3 tools/pp09_laguerre_envelope.py --cases research/rnext04_laguerre_admission_20261003/cases.json --output research/pp09_laguerre_envelope_20261004/RESULTS.json --workers 4`

Input `cases.json` sha256 `700726f0...a01b7d`. Output `RESULTS.json` (mpmath 1.3.0, 60 digits) sha256
`c50789bc...318beb`.

**Verdict: FAIL** (G1 fails, G2 holds).

| Gate | Outcome |
|---|---|
| G1 | **fails**. Six cases have `h rho >= 20`: `n{6,12,20}-rho400-h0.1-{distinct,same}`, `h rho_enc = 48.00000000005`, degree 111-114, scale 16, `L' = 16`. All six are the R-NEXT-04 budget rejections. The published `cases.json` has no component field for any case: no `column_errors`, `truncation`, `coefficient`, `recurrence_adjoint`, `summation`, `normalization` or `fused_summation`. It has only the total (`coverage.total` = `laguerre_adjoint_total`), the admission and the budget. So the registered decomposition cannot be formed from published data, and no fraction or bottleneck is reported under Part A |
| G2 | **holds**. For n = 0..128, no point of `|L_n|` at 60 digits lies above `E_n` (10,000 equispaced points per n on `[0, 16]`, 0 violations). The bound is at most 1.0077 times the largest point value. On 100 of the points, the recurrence values agree with `mpmath.laguerre` to a relative 9e-58 |

Part B (certified envelope, `L' = 16`, n = 0..128). The run used 42 final pieces (83 piece evaluations, 9 levels,
smallest width 1/16) and did not reach the 2^20 cap. Every `E_n` is at most 1.0086 times a certified attained
value. The largest is `max_n E_n = 710.4667` at n = 5. Some values: `E_0..E_10 = 1, 15, 97, 345.7, 705, 710.5,
186.7, 607.0, 181.8, 548.5, 239.9`. For n >= 20 the maximum is 395.6 (n = 24). At the degrees the cases use, `E_111..E_114 =
202.6, 188.6, 175.7, 227.8`. Against `e^{L'/2} = 2980.958`:

- the uniform factor `e^{L'/2} / max_n E_n` is **4.196**;
- the per-n factor `e^{L'/2} / E_n` is 4.2 to 198.7 for n >= 1, and 7.5 to 17.0 for n >= 20.

G3 (reported, not gated):

- **Recomputation.** The registered recomputation from published coefficient radii and tails is not possible,
  because those fields are not published. Only the ratio 4.196 is reported, as registered.
- **Budgets.** Whatever the decomposition, using `max_n E_n` in place of `e^{L'/2}` can divide a total by at most
  4.196. So the six recomputed totals would be at least 2.26e6 to 4.98e7, against budgets of 1e-6. Any valid
  envelope has `E_n >= |L_n(0)| = 1`, so even removing `e^{L'/2}` entirely leaves at least `total / 2981`, which is
  3.2e3 to 7.0e4. **No recomputed total can meet its budget.** A native envelope implementation is therefore not
  justified by these cases.
- **Supplementary, derived, not registered, not gated.** This analysis was declared in the committed tool before the
  run. The tool emulated `choose_transform`, `laguerre_coefficients`, `exp_nonneg`, `norm_up` and the `directed.rs`
  primitives bit for bit, from the published `A`, `h`, `w`, degree and scale.
  - The emulation reproduces the published degree and scale in all six cases. The coefficient radii themselves
    cannot be checked against published data.
  - The derived truncation term is 8.0e-11 to 9.4e-11, and the derived coefficient term is 5.1e-11 to 1.6e-10.
    Together they are 1e-18 to 1e-17 of the totals.
  - The remainder (`recurrence_adjoint + summation + fused_summation + normalization`) is the whole total to 12
    digits. Normalization is 0, since the normalization shift is 0 in all six cases. The published fields cannot
    split the remainder further.
  - With per-n `E_n`, the derived coefficient term falls to 1.8e-12 to 6.1e-12, and the totals do not change in
    their first 12 digits.
  - On these derived numbers, the bound is lost in the remainder, presumably the Bernstein-envelope adjoint
    `recurrence_adjoint`, and not in the `e^{L'/2}` terms. That contradicts the R-NEXT-04 attribution ("the
    components that carry `e^{L'/2}` (truncation, coefficient) make the bound useless"). This is not established
    from published data, and the adjoint has not been isolated.

Deviations and disclosures (all fixed in the tool before the run unless stated):

1. `rho` and `L'` are not in `cases.json`. Both were derived from the R-NEXT-04 test source:
   `rho_enc = 1.2 rho (1 + 1e-12)` in binary64, then `beta = rho_enc / 16` and `L' = div_up(rho_enc, beta) = 16`. The
   emulated Gershgorin discs of the published `A` lie in `[-rho_enc, 0]` for all six cases. The selection uses
   `h rho_enc`; nominal `h rho = 40` selects the same six cases.
2. **Evaluation method.** The recurrence was evaluated in `mpmath.iv` on local polynomials in `t = x - c` (Taylor
   form, interval coefficients, `|L_n| <= sum_k mag(a_k) r^k`). The naive interval extension, with `x` itself an
   interval, was not used for the bound. Before the run, the dependency width of the naive form was estimated to grow
   like `(1 + sqrt 2)^n`, so it could not meet the 1 % rule at n near 128 within 2^20 pieces.
   - As a diagnostic, the naive enclosure was computed on the final cover. It exceeds 1.01 `E_n` from n = 6, and
     reaches about 1e47 at n = 128 on pieces of width 1/16 or more.
   - The naive form was not run to the 2^20 cap. The claim that it fails there for large n is an analytic estimate,
     not a measurement.
3. **The 1 % rule** was read as follows. A piece is final once, for every n, its upper end is at most 1.01 times the
   largest certified point lower bound of `|L_n|` found on `[0, L']` so far. That point bound is at least the
   piece's own width-free value. So each `E_n` is at most 1.01 times an attained value. A literal per-piece reading
   cannot terminate on pieces that contain a zero of `L_n`.
4. G1 was read strictly. Covering a case means giving its component fractions, and the published file does not allow
   that. The verdict FAIL reflects this missing published data, not a failed enclosure. No result was tuned.
   The R-NEXT-04 test would need to export `column_errors` and `fused_summation` for the registered Part A. This
   node does not change it.

Integration note (integrator, 2026-10-04): implemented by a delegated agent in a separate worktree on top of the pushed preregistration `a38f245` (source `a561e6f`), cherry-picked as `cf01fac`. The integrator re-ran the recorded run (`tools/pp09_laguerre_envelope.py`, and for PP11 also the exporter) and obtained identical outputs apart from timing and path metadata. Ledger row added by the integrator.
