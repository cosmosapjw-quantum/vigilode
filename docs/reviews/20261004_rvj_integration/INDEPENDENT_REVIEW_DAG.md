# Independent review of the RVJ DAG execution (2026-10-04)

Reviewer: a separate agent that wrote none of the reviewed code; read-only,
own numerical checks. Scope: `git diff 0153540..47a86b6 -- crates/`, focused
on soundness of every reported bound. Integrator's verification and
disposition follow each finding.

**No P0 (no reported bound below the true error) was found.**

| # | Severity | Finding (summarized from the review) | Verified by integrator | Disposition |
|---|---|---|---|---|
| 1 | P1 | `fourier_path_certificate.rs` documents `sup_bound` as `sup |path|_1` (`|re|+|im|`) and the PP05 checker tests G1 in that 1-norm. The 1-norm is not rotation invariant (`|e^{i pi/4}|_1 = sqrt 2`), and the Lipschitz radius argument also needs the modulus. Every component is a valid **modulus** bound, which is what the code uses. | Yes (`|e^{i pi/4}|_1 = 1.41421`) | Documentation corrected to the complex modulus; PP05 results note added. The recorded G1 compared a 1-norm error (>= modulus) with the bound and held, so it also holds in the modulus; no verdict changes. |
| 2 | P2 | `commit` rounds `t + h` without an exactness check, so the next phase witness could use an uncharged time error (unreachable in the recorded runs: dyadic steps). | By test | `commit` now refuses unless the TwoSum residual is zero; test `commit_refuses_inexact_time_sum`. PP05 export re-run: byte-identical. |
| 3 | P2 | `shared_shift_policy.rs`: repeated shifts re-factored the Hessenberg LU per target while charging it once (executed work above charged work). | By reading | Factor once per distinct shift, solve per target; PP04 export re-run: byte-identical (candidates and charges unchanged). |
| 4 | P2 | `certify_exp_action_lognorm_auto3` failed as a whole if one metric failed. | By test | A failing metric is skipped; error only if none certifies; test `auto3_skips_a_metric_that_cannot_be_built`. PP12b export re-run: byte-identical. |
| 5 | P3 | `directed.rs` doc and the SAFE-ENCLOSURE texts give the false window of the old floor as `(-707.0234, -707)`; correct is `(-707.0101, -707)` (`ln 2^-1020 = -707.010124`). The L-0064 counterexample was at `x = -707.00709` (0.304 % under), not at `-707.01`. | Yes (mpmath) | Doc corrected; correction notes appended to both nodes. The SAFE-ENCLOSURE-b "neighbours of ln 2^-1020" used the wrong centre (-707.0234); the 20,001-point grid with spacing 4.5e-4 does cover the true window. The fix itself (`2^-1019`) is correct. |
| 6 | P3 | `chart_transport.rs`: on underflow `mul_down(kappa, tau)` can be `-tiny`, making the decay enclosure refuse. | By reading | Lower end clamped at 0 (`kappa, tau > 0`). |

Checked and found sound by the reviewer: the stepped certificate's error
recursion and the log-norm propagation without the Crouzeix-Palencia factor;
the interval Cholesky sufficiency; `decay_upper`; `midpoint_radius`; the 2π
Cody-Waite constants and tail test; the Fourier certificate's binomials,
rotation widening, start mismatch and binding; the complex-shift gain (2,000
random cases); the fused-φ perturbation bound in the 2-norm; Leja's
EstimateOnly status; the unchanged default GCRO-DR path.
