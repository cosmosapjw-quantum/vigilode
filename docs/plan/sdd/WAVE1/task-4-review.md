# Task 4 review (t-verify), 9b5d083..f2250b7

## Spec Compliance
Spec compliant. All six claims have gated checks matching the brief/prereg; node layout, ledger row L-0137, pushed commit, hashes verified.

Checks run:
- Named risk (recursion fidelity): verify_theory.py:559-570 builds T_ij = g*C_ij*w + A_ij*(w-1), b_code^T (I-T)^-1 w, from snapshot A, C (raw, exact Rational of the decimal strings, :546-552). Rust U-form (rodas5p_matrix_free_fast.rs:9-16, 721) uses (1-g z)U_i = g sum C U + g z sum A U + rhs, y_new = y + sum b_code U, with L_ij = -(z A + C) g. Since g z = 1 - 1/w, w*g*z = w-1, so T matches. Claimed numbers are compared to the computed ones with tolerances, not derived from them. Claim 6 builds H from the same raw A, C. Not circular.
- Ran script once with -I, cwd=checkout, --out to scratchpad: exit 0, all claims true, output byte-identical to committed RESULTS.json (~10 s).
- L-0137: outputs/inputs sha256 match file bytes; commit 81069f9 holds the script unchanged to HEAD; origin/production/t-verify = f2250b7 so it is pushed and an ancestor. Prereg (5811ca9, 69 lines) precedes the run; only a Results section appended afterward (allowed).
- tools/check-research-node.py --base ...: exit 1, only L-0124 CONTINUATION_DAG/FINDINGS sha mismatches (outside node), as reported. No complaint on the new node.

## Strengths
Exact rationals, 50-digit mpmath, honest labelling of Lemma 3 as grid evidence, sign remark on Cor 2a, dry run disclosed, byte-reproducible.

## Issues
Critical: none. Important: none.
Minor:
- verify_theory.py:606-607 dead variables (min_sum_entry, tuple inits); claim2 reports min abs-sum only at the grid end (t=8, 1.0000019), so gate ">= 1" is grid-only.
- No independent cross-check of the polynomial route against a direct numeric solve of (I-T(z))^-1 or against rodas5p_stage_transfer_constants (:707-745); would tie the result to the Rust code more tightly.
- claim4 pass (:713) only symbolically checks the |1-W/P| identity; the factor-2 bound itself rests on the rational grid (h/h_old<=4) plus the identity (t/(1+t)<1), adequate.
- Local remote-tracking ref for production/t-verify absent (git branch -r --contains empty); ls-remote confirms push. Checker exit 1 needs the controller to rule on L-0124 (pre-existing).

## Assessment
Task quality: Approved. Verification is computed from the code's actual stage recursion and snapshot, gates were preregistered, ledger rules satisfied.
