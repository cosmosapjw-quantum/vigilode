# Critical review of R-NEXT and INT results, and the next DAG

This review re-reads the results of L-0044 to L-0056 and the INT-06 decision. It asks three questions: what do they
actually support, where are they weaker than their summaries, and what follows for the next DAG? It changes no
ledger row or verdict. Where it corrects a statement, the correction is made here and in the status files, not in
the historical records.

## 1. Corrections to stated conclusions

**C1. INT-06's n = 16 margin is not robust.** The derived margin of +0.63 solve units per attempt divides the
certificate's *directed* operations by a solve unit counted in *plain* floating-point operations. One directed
operation is not one plain operation:

- `add_up` is a rounded sum, a TwoSum error term (5 operations), a comparison and a possible `next_up`;
- `mul_up` is a product, an FMA and a comparison;
- each also carries a finiteness check and a `Result`.

Charging each directed operation as k plain ones (recomputed from the same records) gives mean margins at n = 16 of
+0.63 (k = 1), +0.26 (k = 2), -0.11 (k = 3) and -0.49 (k = 4). The k that applies is not measured, but k >= 3 is
plausible from the instruction counts above. So the positive margin is within the uncertainty of the unit. The
decision itself (no default activation) is unchanged; its supporting number is weaker than stated.

**C2. INT-05's "useful metric" result was set up to succeed.** RA-03 had already shown, before INT-05, that dyadic
balancing makes the VIG-A02 matrix symmetric. Gate item 3 used exactly that metric, chosen by hand. The other
families' metrics were hand-chosen too, and the one chosen from theory (exponential weights for convection-diffusion)
made the bound worse. The PASS shows that the bound is valid and that a good metric can make it useful. It does not
show that a metric can be found without knowing the answer. Two further limits:

- the growth factor uses the platform `exp` with a margin, not a directed exponential;
- the single Taylor step is useless once `tau |W|` is large, which is the stiff case that matters.

**C3. INT-01's fresh family did not test failure removal.** No control failed on it, cold GMRES included, so it
tested only false convergence and cost. Its "mechanism" explanation for the two remaining failures (a small defect
amplified relative to the projected residual) is an inference from traces, not an experiment. A closer reading of
the code gives a sharper diagnosis:

- After the recycle projection, the solver recomputes the true residual and builds `v_1` from it without projecting
  it against `C` again.
- The small least-squares problem assumes `[C, V]` is orthonormal, and that fails as soon as `v_1` has a component
  along `C` (`max |C^T V|` = 0.64 already in the first cycle).
- The recycle update then inherits the inconsistency through `R^-1`.

This points to a structural fix (orthogonalize `v_1` against `C`) rather than a tighter tolerance.

**C4. INT-04 established little beyond the adapter's safety contract.** The target family is block lower
triangular and is solved exactly by one sequential pass. Newton without globalization from `Psi^-1(0)` diverging
under a polynomial chart is expected, not a discovery. No RVJ predictor was used as a candidate. The node does not
support further work on M1 without a target where the native solve is not sequential.

**C5. INT-02 applies to diagonal Jacobians only.** The structural pipeline is exact and cheaper, but the R4 family is
diagonal by construction. For a non-diagonal `J`, the inverse witness `|W^-1|` is dense in general (a tridiagonal `W`
has a full inverse), so "structural" does not extend by itself.

**C6. INT-03's parity is with v2, not with a reference.** Bitwise identity with v2 shows the banded pipeline
reproduces v2's arithmetic. Neither is checked against an independent reference there; v2's accuracy rests on its own
earlier nodes. v2's counted floor (its `n^2` assembly) was argued from the code, not instrumented.

**C7. R-NEXT-01's item 3 counted a safe-side case as a failure.** The only unresolved decision was a *rejection*
(Robertson h = 1e-2, where the exact step's error norm is 1e6). An unresolved rejection cannot admit a bad step. The
gate required both directions to be resolved, so a guard needs only one: every acceptance must satisfy `err + B <= 1`.
Whether that one-sided rule holds away from the 33 tuned cases is untested.

## 2. Process weaknesses

**P1. Thresholds on things the node did not control.** Three FAILs came at least partly from thresholds set without
a calibration run on the quantity they constrained:

- INT-02: the dense comparison arm's slope;
- INT-04: an absolute 1e-8 WRMS bound below binary64 residual level, and a comparison against a binary64 reference
  without its rounding;
- INT-01: an absolute defect tolerance.

From here on:

- a threshold on a comparator, or on an absolute scale, is either reported and not gated, or calibrated on a
  development set that is disjoint from the recorded set and named in the preregistration;
- an enclosure is checked against the exact (or high-precision) quantity, never a binary64 surrogate.

**P2. Exit codes hidden by pipes.** Twice, a commit went in with a failing lint because the command chain ended in
`| tail`. From here on every chain runs with `set -o pipefail`.

**P3. Counters stand in for cost.** Every efficiency statement is counter-based while timing is on HOLD. C1 shows
that counters in different units cannot simply be compared. A cost comparison must state the unit of each term.

## 3. What remains, after this review

| Item | Source | Decision |
|---|---|---|
| GCRO-DR: `v_1` orthogonalized against `C` (C3), with and without the reuse check, on the L-0046 sets and a fresh Brusselator set that can fail | INT-01, R-NEXT-03 | **REV-01** |
| Nonnormal bound: certified time stepping instead of one Taylor step, automatic Osborne balancing instead of hand-chosen metrics, and a directed exponential (C2) | INT-05, V1 | **REV-02** |
| One-sided acceptance rule for the residual-to-output budget, on fresh step sizes (C7) | R-NEXT-01 | **REV-03** |
| `solve_into` for LGMRES | R-NEXT-02 | **REV-04** |
| q=2 activation at n >= 16 | INT-06, C1 | not run: it needs a measured cost per directed operation and timing authority (HOLD) |
| Stage-chart adapter beyond its contract | INT-04, C4 | stopped (no target where it has a role) |
| Structural witness for non-diagonal `J` | INT-02, C5 | not scheduled: a dense `|W^-1|` is the obstacle, and no decay-bound witness exists yet |
| Timing campaign | all | HOLD |

Each REV node is preregistered before its code and runs, with the P1 rules applied. Verdicts are recorded as they
come.
