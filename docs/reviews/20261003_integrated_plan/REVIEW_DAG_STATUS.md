# Review DAG (REV-01 to REV-04): execution status

This file records the nodes scheduled by `CRITICAL_REVIEW.md`. Each was preregistered and pushed before its code and
runs, and applies the review's process rules (P1 to P3). Each has a ledger row and its results appended below its
preregistration. Amendments made before a recorded run are disclosed there.

| Node | Question | Ledger | Verdict | Finding |
|---|---|---|---|---|
| REV-01 | does orthogonalizing GCRO-DR's first Arnoldi vector against `C` remove the recycle failures (review C3)? | L-0057 | **FAIL** | no: 56 recycle-induced failures remain, and `max |C^T V|` stays near 1. The review's diagnosis C3 was wrong |
| REV-01b | does a second projection pass against `C` (H2) remove them? | L-0058 | **FAIL** | H2 refuted: `max |C^T V|` still reaches 0.9999 on a fresh Brusselator-120 set. A post-hoc trace shows the loss present when each vector is created, with no near-breakdown |
| REV-01c | does restoring `M^-1 A U = C` after every recycle update (H3) remove them? | L-0059 | PASS | yes: no recycle-induced failure on a fresh Brusselator-160 set or on any earlier set (C had 85). A large `C^T V` remains and is harmless, so it was never the cause. The refresh costs 1.08-1.32x cold GCRO-DR's products, so recycling saves nothing on these systems |
| REV-02 | can certified stepping, Osborne balancing and a directed exponential remove INT-05's limits (review C2)? | L-0060 | **FAIL** | the bounds enclose in all 18 cases, the directed exponential in all 251 grid points, and Osborne finds the VIG-A02 metric automatically. Stiff convection-diffusion gets absolute bounds of 1e-13. But three of six solutions decay below that (Gershgorin cannot see decay), and strongly nonnormal matrices stay at 1e9-1e19 relative |
| REV-03 | does the one-sided acceptance rule hold at fresh step sizes (review C7)? | L-0062 | PASS | all 68 acceptances at h = 3e-4, 3e-3, 3e-2 are resolved by the budget; the 6 unresolved decisions are safe-side rejections (Robertson h = 3e-2, budget 2e246) |
| REV-04 | LGMRES `solve_into` (R-NEXT-02 extension) | L-0061 | PASS | bit for bit the existing LGMRES on 1,216 solves, including 344 rolled-back failures; output untouched on failure; allocations only 0.94-0.99x, because the shared augmented Arnoldi routine allocates in every iteration |

## What changed in the conclusions

- **GCRO-DR (R-NEXT-03, INT-01).** The cause of the recycle failures is now identified experimentally. Recycle
  updates break the invariant `M^-1 A U = C`, and later cycles use the broken pair. INT-01's diagnosis (an absolute
  defect tolerance) and the critical review's C3 (the start vector) were both wrong. The repair is the refresh, and it
  removes the benefit of recycling on the systems studied. Recycled GCRO-DR, if used, needs `refresh_after_update`;
  cold GCRO-DR or GMRES is the recommendation.
- **Nonnormal certificates (INT-05).**
  - Stepping removes the stiffness limit for absolute errors.
  - Automatic balancing replaces the hand-chosen metric where balancing is the right metric (VIG-A02).
  - Relative accuracy for strongly decaying or strongly nonnormal problems is beyond a Crouzeix-Palencia bound over a
    Gershgorin box. A tighter numerical-range bound would make the bound decay with `e^{-pi^2 t}`, but not with the
    faster decay of convection-dominated solutions.
- **INT-06 margin (C1).** It is corrected in the critical review and not re-run: q=2 activation stays off.

## Still open

- Removing the per-iteration least-squares solve from the shared augmented Arnoldi routine (REV-04).
- A verified numerical-abscissa bound and a range-minimizing metric (REV-02).
- A q=2 activation decision with a measured directed-operation cost and timing authority (HOLD).
- The timing campaign (HOLD).
