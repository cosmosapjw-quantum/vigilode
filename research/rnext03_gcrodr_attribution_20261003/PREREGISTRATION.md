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
