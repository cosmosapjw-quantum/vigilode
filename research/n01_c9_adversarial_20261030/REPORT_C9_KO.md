# VigilODE N01 C9: 원래 8-stage raw-U와 source-owner 권한의 공격적 재검토

**2026-10-30 KST.** Audit object: `cosmosapjw-quantum/vigilode` Draft PR125, HEAD `8afb3cef7f237b0c7ac26ec7b647694df23a7789`, C8 native source Git blob `e6a33b66ba80055cae9439cf683a1715ca77c5a2`. This stage is a **bounded serial, same-assistant adversarial audit**. A distinct author/subagent did **not** review it; `C9_INDEPENDENT_AUTHOR=NOT_RUN`. The C8 archived package (482,799 bytes, SHA-256 `d82bc629ea70beb4f5ac6bd86ce6fab0a0910612cd161f1530f5a45b81538261`) was actually extracted and `python VERIFY.py` passed **27/27 payload identities, original Rust source Git blob, prior C7 ZIP, GitHub Actions artifact and original Fraction/CSV/JSON reproduction**. C8 native 40/40 is inherited, not rerun or reclassified by this audit.

## 1. Exact scientific contract under review

In the original 8-stage identity-mass raw-U target with actual represented source `f`, frozen `J_0`, `tau = h gamma`, and strictly lower tableaux `A,C`, stage residual is

\[
 r_i=U_i-\tau J_0 U_i-\tau f\bigl(y+\sum_{j<i}A_{ij}U_j\bigr)-\gamma\sum_{j<i}C_{ij}U_j.
\]

The C8 code reconstructs `U` from returned native K stages, uses outward `Interval` arithmetic to bound each represented raw-U residual, and under a positive source tube and `tau B < 1` constructs `kappa <= (1-tau B)^{-1}` and finite strictly lower triangular causal transfer. For Robertson `f=(-k_1a+k_2bc,k_1a-k_2bc-k_3b²,k_3b²)`, the state 1-norm Jacobian bound on `z>=0`, `sum(z)<=M_tube` is

\[
\|J(z)\|_1=\max\{2k_1,\,2k_2c+4k_3 b,\,2k_2 b\}\le B:=2\max\{k_1,(k_2+2k_3)M_{tube}\}.
\]

Thus `tau B < 1` is sufficient for the Neumann inverse bound, not necessary; its failure is not a proof of unsolvability. For a **matched and authenticated** source and a true positive argument tube at every stage, the standard recurrence `e_i <= kappa(rho_i + sum_{j<i}(|gamma C_ij|+tau B |A_ij|)e_j)` gives the conditional endpoint **algebraic** error, plus original raw-U/native-K assembly rounding. This statement does *not* bound truncation/global-IVP error.

## 2. F01, BLOCK_NOW for general source promotion: coefficient/clock/epoch ownership

`crates/vigilode-coupled/tests/n01_df_c8_work_precision.rs::certified_raw_stage_bound` receives a free `case: Case, y, h, stage_k, y_out`; it does **not** accept `N01MassActionOwner` or its opaque trial. It therefore evaluates the supplied equations, not necessarily the same equations that generated `stage_k` or `y_out`. Actual C8 source-matched 38 rows are not refuted; there `case.k` is consistently used in native callbacks. But this function cannot be lifted into production as a source-owned certificate without authenticated owner/clock/epoch/represented rates and consistency checks on the current stage before final commit.

Exact forged-source witness: `y=(3/4,1/8,1/8)`, `tau=1/64`, all eight stage vectors zero and `y_out=y`. Supplying **fake** rates `(0,0,0)` makes all fake original raw-U residuals zero and gives Neumann margin1, positivity tube PASS, algebraic bound zero. The **actual** physical rates `(1/2,10,200)` at the same source state give first-stage original residual exactly `(7/2048,93/2048,-25/512)`, maximum `25/512` and total zero. Wolfram exact rational and independently authored Python `Fraction` both reproduce the values. **The algebraic bound can be correct for the forged equation and unauthoritative for the actual source**. Smallest constructive repair: make verifier inputs derived from an opaque, immutable source-trial snapshot with rates/time/state/epoch/owner identity, revalidate at the end; remove independently writable coefficient arrays from the admission API. If source changes, reject without output mutation. Then rerun source-switch and stale-clock negative controls.

## 3. F02, BLOCK_NOW only for global-IVP promotion: local algebraic vs temporal error

The moderate source-matched C8 original-RODAS direct comparator at `n=8` has terminal physical ℓ¹ error `9.639530148951923e-10` against a newly computed high-precision *mpmath* reference. Its maximum per-step **algebraic stage/endpoint** bound is `3.305841880378621e-12`, approximately **291.5908 times smaller** than the achieved terminal IVP error. There is **no logical contradiction**: these quantities bound different errors. They must not be reused as if the stage algebraic bound certified truncation, event, continuous-time or cumulative trajectory error. The current C8 report explicitly says this; preserve its scoped PASS. Future production needs a separate truncation + stability + accumulated global-error argument.

## 4. F03, FIX_SOON for work/precision wording: no achieved-error overlap

On all tested `mild` and `moderate` fixed grids, achieved-error ranges of the original RODAS5P direct oracle and auxiliary value-only BE do **not overlap**. Therefore no matched-*achieved*-accuracy RODAS-vs-BE runtime/work ratio can be inferred. C8's work–precision *cap* table remains a descriptive cap-based comparison, and its **within-BE, same physical method and grid** Picard-vs-accelerated map calls are valid. Scalar map calls, full RHS, Jacobian builds, direct factorization and linear solves are distinct costs. Native fixed-grid RODAS used `force_accept=true`; not adaptive physical admission. Keep all 12 stiff refusals. Do not infer a universal runtime speedup.

## 5. F04, independent source-matched reference cross-check

New independent `mpmath.odefun` Taylor integration of the *same* Robertson ODE at 40 and 75 decimal digits, with no use of SciPy's BDF/Radau implementation, produced max 40-vs-75 differences `1.1776e-35` (mild, T=.1) and `3.8840e-34` (moderate, T=.02). Against the 75-digit values, C8 SciPy Radau differs only `7.0008e-17` and `2.2858e-16`, while SciPy BDF differs `2.4585e-13` and `1.7253e-12`. This supports the Radau reference in the tested two regimes, but **mpmath is not an interval-verified global IVP error certificate**. The stiff negative-control reference is not audited by this new high-precision method, and no absent matches are invented.

## 6. F05, source-independent exact bounds and causal transfer remain valid in scope

An independently implemented `Fraction` oracle with seed20261039 checked **512** nonnegative model/Jacobian 1-norm and conservation cases, **504** cases satisfying Neumann margin with exact 3×3 inverse norm, **128** random nonnegative 8×8 strictly-lower transfers matching exactly `D+LD+...+L^7D` and `L^8=0`. All passed. This supports the **conditional** formulas, not source identity, positivity-tube provenance for arbitrary BASS physical33, or original 8-stage derivative-free production integration.

## 7. F06, BACKLOG test-only hardening: unchecked interval `unwrap()`

The C8 research-only verifier uses `.unwrap()` in `interval_jac` and residual interval assembly. A nonfinite/out-of-domain or overflowing intermediate could panic instead of returning a typed refusal. Current regular native benchmark sources are finite and bounded and do not show this failure. Recommendation: replace these with `?` propagation and test a high-range input. Do not stop the scoped C8 result or invent a valid-input crash without running one.

## 8. Review limits, DAG and next bounded action

GitHub primary C8 Actions run40952770745, 40/40 PASS, original artifact and CSV/JSON byte reproducibility retained. All C9 exact and high-precision analyses above were run **in the present session** from the verified C8 archive. The C9 technical adversarial review is one finite scoped pass, **not a distinct-author C9 independent acceptance**. No native subagent dispatch tool exposed: `native_subagent=NOT_RUN`, `C9_INDEPENDENT_AUTHOR=0/1`, production HOLD.

Official original VigilODE **7/23**, central REI **28/88**, successor admitted **0/12** unchanged. C6/C7/C8 each **1/1 scoped**; C9 self-audit **1/1 scoped** with two concrete scope barriers, different-author C9 **0/1**. Next action: one actual independent reviewer/agent reads immutable PR125 & this C9 package, reproduces F01 and F02, and issues a signed source-scoped opinion; only then C10 decision/handoff can close. Do not merge main, force push, relabel non-overlap as speed or manufacture source-owner proof.

Primary method references consulted: Christoph Ortner, *A Posteriori Existence in Numerical Computations* (SIAM J. Numer. Anal. 2009), DOI [10.1137/060668183](https://doi.org/10.1137/060668183) emphasizes that a residual and stability estimate for a nonlinear equation require justified local existence/inverse assumptions. Gustaf Söderlind, *The logarithmic norm. History and modern theory* (BIT, 2006), DOI [10.1007/s10543-006-0069-9](https://doi.org/10.1007/s10543-006-0069-9). These are context, not external verification receipts of this code.
