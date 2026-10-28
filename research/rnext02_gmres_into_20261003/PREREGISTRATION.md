# Preregistration: caller-owned GMRES output and bounded workspace (remaining-only DAG node R-NEXT-02)

## Question

L-0038 failed its allocation target (U-form MF driver <= 0.5x the sequential MF step's allocations per attempt;
measured 0.32-1.03x) because every Krylov solve returns a fresh solution vector and report, and allocates inside
the solver. In GMRES (`gmres.rs`) the Arnoldi loop also solves the small least-squares problem after every column
(`least_squares` builds a faer matrix, a column-pivoted QR and an output vector) although only the last solution of
a cycle is used. Does a GMRES entry point that writes into caller storage, solves the small problem once per cycle
and refuses or reports workspace growth remove most of those allocations, with bitwise the same results?

## Design (written after this commit)

`crates/rodas5p-krylov/src/gmres_into.rs`, GMRES only (LGMRES and GCRO-DR unchanged):

`solve_gmres_into(op, pc, rhs, x0, config, residual_scale, output, workspace, capacity, counters)
-> CoreResult<GmresIntoReport>`

- `output: &mut [f64]` is written only on success; on any error it is unchanged. Rust borrows forbid `output` from
  aliasing `rhs`, `x0` or the workspace.
- `GmresIntoReport` is `Copy` (no solution vector, a static method name): residual norm, relative residual,
  iterations, matvecs, preconditioner applications, cycles, small least-squares solves, and whether the workspace
  grew.
- `capacity: GmresCapacity { max_dimension, max_columns, growth }` with `growth` either `Refuse` or `Allow`. With
  `Refuse`, a solve that needs a larger workspace than the declared capacity fails with a typed
  `KRYLOV_CAPACITY_EXCEEDED` error before any operator application. With `Allow`, growth happens and is reported.
- Same stopping rule, same true-residual checks and same counters as `solve_gmres_with_workspace_and_residual_scale`.
  The small least-squares problem is solved once at the end of each cycle with the same `least_squares` routine on
  the same Hessenberg prefix, so the correction is bitwise the same. One difference in failure behaviour is stated
  in advance: the old loop fails if an intermediate (unused) least-squares solution is non-finite; the new one only
  sees the final one. The final true-residual check is unchanged.
- The old API is unchanged. The U-form MF driver gets a research switch (`set_gmres_into`, default off) that uses
  the new entry point for GMRES stage solves, writing directly into its stage storage.

## Tests and command

`crates/rodas5p-krylov/tests/rnext02_gmres_into_contracts.rs` (contracts) and
`crates/rodas5p-integrators/tests/rnext02_gmres_into_study.rs` (counting allocator, writes `RESULTS.json` here):

`RNEXT02_OUTPUT=research/rnext02_gmres_into_20261003/RESULTS.json cargo test --release -p rodas5p-integrators --locked --test rnext02_gmres_into_study -- --ignored --nocapture --test-threads=1`

Frozen systems (fixed before any run): for the JVP-only Robertson, van der Pol (mu = 1000), HIRES, 1-D Brusselator
(50 cells), Prothero-Robinson and quadratic-4 problems of L-0038, `W = I - h gamma J(y0)` at the initial state with
`h` in {1e-4, 1e-2} and the eight stage right-hand sides `b_i = W U_i`, where `U_i` are the stages of one U-form
GMRES attempt from that state; plus a 1-D convection-diffusion matrix (n = 200,
Peclet 10) with three fixed right-hand sides. GMRES configurations: restart 40 / budget 200 at rtol 1e-11 (the
L-0038 adaptive setting), restart 10 / budget 400 at rtol 1e-12, each without and with the previous solution as
`x0`, and with a WRMS residual scale on the convection-diffusion systems. A state whose generating attempt fails
is reported and contributes no frozen systems.

## Gate

**PASS** if all hold:

1. **Bitwise identity.** On every frozen system and configuration, `output` equals the old API's `report.x` bit for
   bit, every `WorkCounters` field is equal, and residual norm and iteration count are equal. Where the old API
   fails, the new one fails with the same error class and leaves `output` unchanged.
2. **Capacity contract.** `Refuse` with a capacity smaller than the system (dimension or columns) gives
   `KRYLOV_CAPACITY_EXCEEDED`, no operator application (counters unchanged) and `output` unchanged; `Allow` succeeds
   and reports growth on the first call and none on a repeated call of the same size.
3. **Solver allocations.** After one warm-up solve per system, the new entry point allocates at most 0.25x as often
   per solve as the old API, on every frozen system whose solves average at least 4 Arnoldi columns per cycle.
4. **MF driver.** With the switch on, the U-form adaptive GMRES runs of L-0038 (rtol 1e-6, the six problems) give
   bitwise the same final state, step counts and counters as with it off, and allocate at most 0.5x as often per
   attempt as the sequential MF step (the L-0038 item-6 criterion, now for GMRES only).

Otherwise **FAIL**. Reported: allocations per solve and per attempt (absolute), small least-squares solves per
solve (old: one per column; new: one per cycle), vector-work counters. No wall time is measured and no speedup is
claimed from fewer allocations; timing authority stays on HOLD. L-0038 stays FAIL.

## Stop condition (DAG)

Stop if a saving needs stale operator products or drops failure accounting.

## Prior information

L-0038 allocation ratios for GMRES: Robertson 0.41, van der Pol 0.40, HIRES 0.62, Brusselator 0.81,
Prothero-Robinson 0.32, quadratic-4 0.55. No code of this node exists before this commit.

---

## Results (appended after the run at `93d75b0`)

Output: `RESULTS.json`. Ledger row L-0045. Contracts: `rnext02_gmres_into_contracts` 5/5.

**Gate: PASS** (items 1-4 hold).

| Gate item | Outcome |
|---|---|
| 1. Bitwise identity | **holds**: 56 families (12 problem/step states x 2 configurations x 2 initial-guess modes, plus 8 convection-diffusion families), 408 measured solves, no failures; output, residual norm, iterations and every counter equal to the old API bit for bit. No state was excluded |
| 2. Capacity contract | **holds** (contract tests): too small a dimension or column capacity with `Refuse` gives `KRYLOV_CAPACITY_EXCEEDED` with zero counters and the output unchanged; a failed (budget-exhausted) solve gives the old error text, the same counters and an unchanged output; `Allow` reports growth on the first call only; a reserved workspace does not grow |
| 3. Solver allocations | **holds**: on the 32 families averaging at least 4 columns per cycle the ratio is 0.025 (Brusselator and convection-diffusion, restart 40), 0.10 (restart 10), 0.23-0.24 (HIRES, quadratic-4). Below 4 columns per cycle (Robertson, van der Pol, Prothero-Robinson; ungated) it is 0.36-0.85 |
| 4. MF driver | **holds**: switch on vs off bitwise identical (final state, times, steps, counters) on all six problems; allocations per attempt relative to the sequential MF step 0.18 (Robertson), 0.19 (van der Pol), 0.15 (HIRES), 0.021 (Brusselator), 0.28 (Prothero-Robinson), 0.14 (quadratic-4). With the switch off they are 0.33-0.81, as in L-0038 |

Where the allocations went: the old loop solved the small least-squares problem (a faer QR with several allocations)
after every Arnoldi column and used only the last solution; the new entry point solves it once per cycle and returns
no solution vector or `String`. The remaining allocations per solve are those of the one QR per cycle. At one or two
columns per cycle there is little to remove, which is why the small problems sit near 0.4-0.85 at the solver level.

No development run of the study preceded the recorded run; the contract tests ran once during development (all
passing). L-0038 stays FAIL: its gate covered all three Krylov methods and the other items. Claim ceiling: allocation
counts and bitwise identity; no wall-time or speed claim (timing authority stays on HOLD); LGMRES and GCRO-DR are
unchanged.
