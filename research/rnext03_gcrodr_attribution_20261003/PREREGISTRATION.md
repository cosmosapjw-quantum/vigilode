# Preregistration: GCRO-DR failure attribution on frozen Brusselator systems (remaining-only DAG node R-NEXT-03)

## Question

In L-0038, unpreconditioned GCRO-DR (restart 40, budget 200, recycle dimension 8, rtol 1e-11) on the 1-D
Brusselator (50 cells, n = 100) did not complete in any driver ("Arnoldi budget exhausted"), and at rtol 1e-12 with
budget 4000 it failed with a non-finite least-squares solution. The faer scratch panic fixed in `01f4266` is a
different defect and is not revisited. Which mechanism causes the remaining failures, and does a bounded reset rule
remove them without false convergence or uncharged work?

## Frozen systems (fixed before any run)

- **Trajectory set.** The U-form MF driver with GMRES (it completes on this problem) runs the Brusselator from
  t = 0 to 10 at rtol 1e-6 (atol 1e-6), as in L-0038. For each of its first 40 attempts, the state `(t, y, h)` gives
  `W = I - h gamma J(y)` (JVP operator, no preconditioner) and the eight right-hand sides `b_i = W U_i` from that
  attempt's stages. The stage order and the order of attempts are kept.
- **One-step set.** The L-0038 one-step states: `y0` at `h` in {1e-4, 1e-2}, same construction.
- **Held-out synthetic set** (not used to design the rule): 1-D upwind convection-diffusion operators
  `I - tau (D2 - Pe D1)` on n = 200 with `Pe` in {1, 10, 100} and `tau` in {1e-3, 1e-2}, each as a sequence of 8
  slowly varying operators (`tau` scaled by `1 + 0.01 k`) with 8 fixed pseudo-random right-hand sides (seeds 0..7,
  SplitMix64), i.e. 6 sequences of 64 solves.

## Controls (same operator object, right-hand side, initial guess, tolerance and budget in each comparison)

A. cold GMRES (restart 40, budget 200); B. cold GCRO-DR (fresh state for every solve); C. recycled GCRO-DR (state
carried across stages and attempts, as in the driver); D. recycled GCRO-DR with the reset rule below. Tolerances:
rtol 1e-11 with budget 200 (trajectory and synthetic sets) and rtol 1e-12 with budget 4000 (one-step set). The
initial guess is zero for every control.

## Diagnostics (computed by a trace hook added to GCRO-DR after this commit; no effect when off)

At every cycle: true residual norm after the cycle (the solver's own diagnostic product), the least-squares
residual of the small problem, `||C^T C - I||_max`, `||C^T V||_max` and `||V^T V - I||_max` for the recycle images
`C` and Arnoldi basis `V`, the recycle rank, and the retained-space fraction `||C^T r|| / ||r||` of the residual
before projection. Plus per solve: outcome and error text, cycles, matvecs (Krylov, diagnostic and refresh).

## Classification rules (predeclared, applied to every failed solve of control C)

- **R1 restart-length stagnation:** control B also fails on the same system.
- **R2 recycle-induced:** control B succeeds and C fails.
- **R3 residual gap:** in some cycle the true residual exceeds 10x the small least-squares residual.
- **R4 orthogonality loss:** in some cycle `||C^T C - I||_max` or `||C^T V||_max` exceeds 1e-8.
- **R5 non-finite small problem:** the failure is a non-finite least-squares solution.

A failure may carry several labels; one with none is reported as unclassified.

## Reset rule (control D; predeclared)

After a cycle with recycle rank > 0, if the true residual has fallen by less than a factor 0.5 relative to the
residual at the start of that cycle, drop the recycle space for the rest of this solve (continue as GCRO-DR with
rank 0, which rebuilds a recycle space from its own cycles). Every matvec before and after the drop counts toward
the same budget and the same counters. The recycle state stored after the solve is the one GCRO-DR builds
normally.

## Command

`crates/rodas5p-integrators/tests/rnext03_gcrodr_attribution.rs`, writing `RESULTS.json` here:

`RNEXT03_OUTPUT=research/rnext03_gcrodr_attribution_20261003/RESULTS.json cargo test --release -p rodas5p-integrators --locked --test rnext03_gcrodr_attribution -- --ignored --nocapture --test-threads=1`

## Gate

**PASS** if all hold:

1. **Reproduction.** Control C fails on at least one trajectory or one-step system (otherwise the L-0038 failure is
   not reproduced on frozen systems, and the node is FAIL), and the whole study gives identical outcomes and traces
   in two runs in the same process.
2. **Attribution.** Every failure of control C carries at least one label R1-R5.
3. **No false convergence and full accounting.** Every reported success of every control has a true residual (an
   independent product with the operator, outside the solver) at most the threshold times (1 + 1e-12), and the
   counters of each solve equal the sum of its traced cycle work plus initial and final residual products.
4. **Reset rule.** Control D fails on fewer trajectory and one-step systems than control C, fails on no more
   held-out synthetic systems than C, and uses at most 1.10x C's total matvecs on the held-out set.

Otherwise **FAIL**. A FAIL on item 4 alone means the rule is not supported; items 1-3 are reported either way.
Reported, not gated: per-control failure counts, matvecs, and cycle traces of every failure. No Ritz-value-based
error certificate is made. No timing.

## Stop condition (DAG)

Stop promotion on false convergence or uncharged resets.

## Prior information

L-0038 recorded the failure texts above. In a development check during L-0038, maxiter 1000 did not help. No code of
this node exists before this commit.

---

## Results (appended after the run at `f2743bf`)

Output: `RESULTS.json`. Ledger row L-0046. Contract tests `rnext03_gcrodr_trace_contracts` 2/2 (the traced solve
without a reset is bitwise the plain solve).

**Gate: FAIL** (items 1, 2 and 4 hold; item 3 fails on its accounting part).

| Gate item | Outcome |
|---|---|
| 1. Reproduction | **holds**: control C fails on 13 of 320 trajectory solves and 8 of 16 one-step solves; the two in-process runs are identical |
| 2. Attribution | **holds**: all 21 failures carry R2 (recycle-induced: cold GCRO-DR succeeds on every one of them), R3 (residual gap) and R4 (orthogonality loss); 7 also carry R5 (non-finite least squares). None carries R1 |
| 3. No false convergence and full accounting | **fails**. No false convergence: every reported success of every control has an independent true residual within the threshold. Every solve's traced total equals its counters. But on 7 failed one-step solves of control C, the solve aborted inside a cycle (a non-finite least-squares solution), so that cycle's 33-37 operator products are charged to the counters but appear in no cycle record. The preregistered rule counts them as work outside the cycles and finds 52-53 against its limit of 19. This is a limitation of the rule (and of the trace, which records only completed cycles), not uncharged work. It still fails the gate as written |
| 4. Reset rule | **holds**: control D fails on 0 of the 336 trajectory and one-step solves (C: 21). On the held-out set D and C fail equally often (256 of 384), and D uses 76,141 operator products against C's 76,230 (0.999x) |

Failure texts of control C: "Arnoldi budget exhausted" (13), "least-squares solve produced NaN/Inf for 41x40
system" (7), "shifted operator produced NaN/Inf" (1). In the failing cycles, `max |C^T V|` reaches 0.9999997,
while `C^T C` and `V^T V` stay orthonormal to 1e-15. The true residual grows within a cycle (for example
6.5e-4 -> 6.4e13 -> 4.0e31) while the small least-squares residual stays far smaller (gap up to 1.4e22).

Work (operator products, trajectory set): cold GMRES 13,358, cold GCRO-DR 13,328, recycled 15,741, recycled with
reset 13,604. Recycling saves nothing on these systems even when it does not fail.

The held-out set turned out to be uninformative about failures: all four controls, cold GMRES included, fail 256 of
its 384 solves at rtol 1e-11 within 200 products, the same ones. It was fixed before the run and is reported as is.

### Post-hoc diagnostic (after the recorded run; not part of the gate)

`tests/rnext03_posthoc_invariant.rs` (`POSTHOC_INVARIANT.json`, command
`RNEXT03_POSTHOC_OUTPUT=research/rnext03_gcrodr_attribution_20261003/POSTHOC_INVARIANT.json cargo test --release -p rodas5p-integrators --locked --test rnext03_posthoc_invariant -- --ignored --nocapture --test-threads=1`)
checks the recycle invariant `A U = C` of the carried state before and after each control-C solve. On the first
solve of each attempt the state is stale (new operator, defect 1e-2), and the cross-operator refresh restores it to
1e-15 to 1e-12. Within an attempt, some recycle updates return a pair that breaks the invariant (for example 3.4e-5
-> 2.59 at attempt 9, stage 6; 6e-4 after the first one-step solve at h = 1e-4). Because the operator is unchanged,
the next solves reuse that pair without checking it, and they fail. Every failure follows such a broken pair. So the
mechanism is a recycle update that does not preserve `A U = C`, reused unverified under "same operator". Restart
length and the scratch defect fixed in `01f4266` play no part.

A rule that checks the invariant on reuse (k extra products per solve) or always refreshes would address the
mechanism directly; the stagnation reset addresses its symptom and, here, removes every failure. Neither is
promoted by this node. Claim ceiling: attribution on these frozen systems; no Ritz-value-based certificate; no timing.
