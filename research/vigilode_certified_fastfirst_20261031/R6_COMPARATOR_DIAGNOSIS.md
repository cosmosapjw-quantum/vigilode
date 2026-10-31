# R6 bounded red-to-repair: comparator identity, not physics failure

**Fail retained**: GitHub Actions run 41155034250, job 124399832009, exact checkout fca5f770b145ac7f33c4a48b6af097154ff25bc5. Scoped Rust 1.94.1 9/11 new native tests passed; exactly two failed at baseline `map_attempted` expectations (2293 and 2347 versus expected 4). The original exact coefficients, contraction bound, directed certificates and root accuracy gates did not fail.

**Diagnosed mechanism:** `cfg.fast_first=false` implemented pure certified Picard. The tests and scientific comparison sought conventional certified baseline-first Anderson, which uses a Picard candidate and then the same depth-one Anderson trial. Pure Picard is a DIFFERENT algorithm and q≈.99 legitimately needs thousands of steps. Measuring against it would confound generic Anderson acceleration with fast-first callback savings.

**Constructive repair:** replace only the comparator scheduling, not source or tolerances. For conventional certified baseline-first, evaluate source-certified Picard first, then the original candidate and admit candidate only if U(trial)<=L(baseline). For certified fast-first, trial is evaluated first and admitted only if U(trial)<=down(q_upper L(current)), skipping the baseline. Both branches use identical original exact-residual interval evaluator, source-locked q, inexact-rounding Picard lemma, full cost ledger and final certified root-error target. All returned failures retain attempted work and the accepted station prefix. A baseline already meeting the certified root target does not trigger a speculative trial.

**Evidence gate:** rerun target Rust tests and 5 paired ABBA/BAAB blocks including all interval costs at the SAME source-certified root error. Do not rewrite previous red result, pretend the 2293 map calls were a slowdown bug, relax a tolerance, or claim generic nonlinear proof.

Official VigilODE 7/23, external REI informational 28/88, successor 0/12; different-author C9 0/1, production HOLD, no merge or C10 promotion.
