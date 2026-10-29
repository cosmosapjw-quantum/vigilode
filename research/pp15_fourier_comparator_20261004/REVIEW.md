# Independent review of the PP15 draft results (G3)

Reviewer: a separate agent that did not write the PP15 code or text; read-only.
Reviewed: the results section as first drafted (preserved verbatim in
PREREGISTRATION.md under "Executed result"), cases.json, RESULTS.json, the
exporter and checker. Statement, verbatim:

G3: FAIL

The draft keeps correctness and speed apart and runs no timing (PREREGISTRATION.md:58-67). But its matched-error statement is misleading, and the work comparison carries an unstated cost argument.

Numbers checked: G1 (bounds 2.2e-12 to 2.8e-11, actual 1.9e-13 to 2.9e-12), the 6 RODAS errors, "closest = 1e-10", and the counts 11/72/6315, 88/568/49177 and 8/24/8 all match RESULTS.json/cases.json. FFT and direct finals agree to 2.8e-16.

Same problem: yes. rhs_vec (pp15_fourier_comparator.rs:27-39) uses c = g + eps·Re(e^{iωt}a), the same as reference_mp (pp05_fourier_check.py:61). Both arms use initial state (1/4,0,1/2,0) (rs:108 vs FourierState::initial), T = 1/2, σ = 1, g = eps = 1/8. Constant q = 5/4 equals the true q on the invariant leaf.

Findings:
1. Lines 56-57 say only ω = 1e4 misses the client's certified level. ω = 40 misses it too: best RODAS 4.56e-12 against bound 2.89e-12 (1.6x). Only ω = 1 matches (0.14x).
2. The title promises "matched physical error", but none was reached. At ω = 1e4, "closest" is 193x the bound and 2280x the client's actual error. The work at lines 60-62 is quoted at unmatched accuracy, and the draft doesn't say so.
3. Lines 63-65 add an untested causal claim ("client work does not grow with omega… RODAS resolves it with steps"). Next to the counts, this reads as a cost-scaling advantage despite the disclaimer.
4. The draft calls 8 "certificates", but certificate_calls is just a copy of candidate_builds (rs:145), not a separate count.
5. Missing caveats: the ω = 1e4 reference is a non-rigorous scipy pair (difference 3.4e-15). Candidate rejections (4/8) and RODAS Jacobian builds are measured but not reported. Lines 52-55 compare the client's bound with RODAS's actual error without saying the comparison is asymmetric.
