# A04 arithmetic research 01 independent review

**Verdict: `PASS_SCOPED`.** This review admits only the exact counterexample and correction of the historical componentwise tail claim. A04 total arithmetic authority remains `EstimateOnly`; full Leja total-error production promotion remains `HOLD`.

## Identity and reproduction

- Research source: `6b722a918deb0ab178b8e77090cac1280dbb3051`.
- Leja implementation SHA-256: `25904301d40f11d6a5fee1d27aac7454ca02874a9e995634e7909af2e4ae97e8`.
- Historical theorem SHA-256: `a6bfee9ed0283d5a0fc24629ab6e0495b38ba7236670d5f343a3a1884c438613`.
- Reviewer independently reran `exact_tail_counterexample.py`: exit 0, empty stderr; output hash matched `668981816e74a16ea0de02a13a1b0ebddce17e1e6a18a2533da67515b8830a28`. All five preregistration/input/output and five current-source hashes matched the research receipt.

## Mathematical finding

The historical `docs/reviews/20261011_py05/LEJA_TOTAL_BOUND.md:110-114` infers a componentwise bound proportional to the individual component of `v_N` from an induced-norm geometric estimate. This implication is false. For a 4x4 lower shift and `v_0=e_0`, at `N=2`, exact Fraction arithmetic gives the last component of the remaining tail as `1/6`, while the historical bound gives zero. The exact witness is in `../arithmetic_research_01/first_run.stdout.json`.

The valid consequence is uniform:

\[
\|\mathrm{tail}\|_\infty \le \|v_N\|_\infty\,\frac{q}{1-q},
\qquad
\mathrm{tail}_i \le \|v_N\|_\infty\,\frac{q}{1-q}
\]

for every component, with `q < 1`. At the exact witness, this scalar upper bound is `1/4`, which encloses `1/6`. The current implementation at `leja_action.rs:1092-1097`, `1122-1140`, and `1156-1163` forms an outward row-sum bound, propagates `term_hi`, forms an outward geometric ratio, and adds the same max-component tail to every component. This matches the corrected statement.

## Claim boundary and next transition

- Historical componentwise T2 lemma: `SUPERSEDED`; original file is unchanged.
- Current uniform infinity-norm tail implementation: `PASS_SCOPED` for this lemma and exact witness only.
- Stale wording that the current implementation lacks interval replay: `SUPERSEDED_AS_CURRENT_CODE_DESCRIPTION`.
- A04 total error/certificate: `EstimateOnly/HOLD`; no coefficient-table or complete fused-output oracle was run in this review.
- Next node `A04-PROOF-ORACLE-01`: `READY`, requiring independent complete-homogeneous coefficient and exact matrix-phi fused-output Fraction oracles, subnormal/branch/refusal controls, and same-region allocator calibration before any total-path admission.

No Leja Rust code, historical theorem, ledger, or tolerance was changed by this review.
