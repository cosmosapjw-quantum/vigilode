# Complementary review of the parent-authored report

Date: 2026-10-27. Review type: one bounded, read-only complementary review of the parent-authored audit, DOPRI dispersion, and implementation-scope claims. This is not another review of the four theory-lane proofs. No new ODE runs, performance runs, or source changes were performed.

Reviewed report: `research/rigorous_performance_audit_20261027/report/vigilode_audit.tex`. A post-correction snapshot read during this review had SHA-256 `12cfa5786e7d2fec918147b2f665ff31eba2698b35b0c19c1a44b68709f68220`. The report remains a live parent-owned draft; benchmark-result sections were outside this check.

## Findings communicated and corrected during this review

1. **State the precise dispersion hypotheses.** The displayed ratio and phase require `y_0 != 0`. The Fraction script constructs ideal rational coefficients `F(p,q)` corresponding to the source's `f64` expressions `p.0/q.0`; it does not derive the exact polynomial of the rounded binary64 constants or bound runtime rounding. The parent added both qualifications to the phase section and proposition. This resolves the issue.

2. **Keep historical selector percentages scoped to the opt-in preset.** The source packet's `ISSUES.json` applicability note states that the measured selector arms use `adaptive_evidence(ProvidedOnly)`, whereas `SwitchingConfig::default()` is DOPRI-only. The parent added this scope immediately beside the 85%/80% claims. This resolves the ambiguity about default overhead.

3. **Conservative equality rejection is not limited to large counters.** `work_enclosure` applies `next_down`/`next_up` even to positive exactly representable costs. Therefore a rounded equality such as candidate cost 80, comparator cost 100, and ratio 0.8 need not survive the sufficient outward predicate even below `2^53`. This is conservative refusal, not unsound admission. The parent replaced the large-count-only emphasis with the correct all-count statement and restricted unchanged behavior to margins exceeding the enclosures. This resolves the issue.

## Claims checked without a remaining substantive objection

- **DOPRI algebra and signs:** `evidence/dopri_phase_exact.py`, `DOPRI_PHASE_EXACT.json`, and the source tableau agree. The ideal endpoint polynomial is `1 + z + z^2/2 + z^3/6 + z^4/24 + z^5/120 + z^6/600`. The logarithm coefficients at orders 6, 7, 8, and 9 are respectively `1/3600`, `-1/2100`, `1/3200`, and `-1/8100`. For `z=i theta`, the leading log-amplitude defect is negative and the leading phase defect is positive, as reported. The remainder orders and fixed-time powers of `h` are consistent. The idealized controller discussion does not assert a general adaptive gyromotion tolerance law.
- **Historical denominators:** `ISSUES.json` reports 1.285M/1.506M instructions for rotation and 3.94M/4.90M for tight gyromotion, supporting the rounded 85% and 80% shares of selector-minus-direct excess. The case5 comparison is 4.60M/0.67M, supporting approximately 6.9 times the dense instruction count. Least-squares correction and QR occupy approximately 39% and 18% of the same transition-39 total. These are nested inclusive shares, so they must not be added. The report does not add them or turn them into current wall-time gains.
- **Old phase diagnostic:** `ISSUES.json` retains the approximately 3.16e-2 default-tolerance gyromotion error and 1e-4 cap, and explicitly marks the 0.97 tolerance exponent as a two-point estimate. The report preserves that limit and does not claim the physical defect fixed.
- **Adapter defaults and behavior:** the new option is false in `Default`, inherited unchanged by `evidence_driven` and `adaptive_evidence`, and consulted in the explicit-first return-comparator path. The report distinguishes this optional execution change from the unconditional evidence/arithmetic correction, and qualifies fatal-error and attempt-budget equivalence.
- **Faer route:** source selection uses Faer only when the ordinary dense route has `n > 64` and initial Jacobian density exceeds 0.1. Thus “normally dimension above 64” is defensible, and the dense all-to-all fixtures select the intended route. The text correctly does not attribute gains to the already optimized small/sparse in-place route.
- **Evidence scope:** the report preserves 27/27 finite operator checks versus 25/27 strict residual-target successes, leaves the shifted candidate disabled, and does not promote the V8 physical ledger.

## Optional editorial and layout refinements

- Replace “including 18% in QR factorization” with “QR alone accounts for 18% of that same total” if maximum denominator clarity is desired.
- The appendix can also mention that adding the public configuration field can require downstream exhaustive struct literals to change; this is separate from the already mentioned exhaustive enum matches.
- No certain parent-authored equation overflow can be established from source inspection alone. The longest display to inspect in the final REVTeX log/PDF is the first `R(z)` polynomial. If needed, break it after the `z^4/24` term using a continuation line within `align`; its source newline currently does not create a display break. The `log R` line and the inline stability-polynomial formula in the proof are secondary layout checks. These are layout risks, not confirmed rendering defects; this review did not compile the document.

## Disposition

The three concrete interpretation/hypothesis issues found in the bounded review were communicated to the parent and observed corrected in the draft. No remaining substantive objection was identified in the checked parent-authored claims. Pending benchmark numbers and final rendered layout still require their own already planned evidence and verification.
