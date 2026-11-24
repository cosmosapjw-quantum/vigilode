# Stage–row lattice of a RODAS5P attempt and two-way stage solves

Referee run of 2026-11-24 (second loop of the day), branch `research/stage-lattice-referee-20261124`,
base `abf29825` (`research/unified-solver-referee-20261124`). Tags follow the V05 theory note:
**[exact]** holds in exact arithmetic or as a combinatorial identity and is checked by a script,
**[numerical]** is a floating-point measurement, **[measured, diagnostic]** is a wall-time measurement
on a non-isolated host, **[conjecture]** is not proved.

## 0. Why the earlier attempts did not shorten the eight stages

V05 (theory S1) proved that, for a right-hand side available only through point evaluations, the eight
RODAS5P right-hand sides form a chain of depth 8: argument `g_i` needs `f(g_{i-1})` because
`A_{i,i-1} != 0` for every `i`. Every recorded attempt at parallelising "the eight sequential loops"
either changed the stage equations (simultaneous sweeps R6, diagonal SDC, transactional q2 R7,
Schwarz V14, level tableaux V16, peer methods PPSW1) or targeted another part of the step
(speculative Jacobian V07, lagged LU, shared Krylov basis V06, parallel FD assembly V08/V26). The
measured verdicts were negative or HOLD. One attempt acted inside the exact stage solves: the
partition arms C2/C4/C8 of V19 (SPIKE-like block LU with a reduced interface system). They failed the
backward-error validity rule on the pivoting-hostile suite S4 with `err:block_pivot`, because
factoring each block independently restricts pivoting to the block.

The vector-level depth-8 statement remains true and is not challenged here. The question of this run
is different: **at the scalar level, what is the critical path of one attempt on a banded system, and
can it be shortened without changing the stage equations, the tableau, the controller or the
stability of the linear algebra?**

## 1. Which part of the step dominates (regime map) [numerical, from V04]

`scripts/regime_map.py` reads the V04 cost anatomy (`cost_anatomy/AMDAHL.json`, 59 cells, fractions
of the instrumented step) and assigns each cell its dominant part (`results/regime_map.json`):

| structure | cells | dominant part |
|---|---|---|
| banded 1D (Brusselator, `l = u = 2`) | 10 | stage solves in 8 (f_S 0.61–0.70 with a cheap RHS), RHS in 2 (RHS cost multiplied by 10 or 100) |
| banded 2D / 3D (natural ordering, band `m`) | 9 / 3 | factorisation in all 12 (f_LU 0.72–0.94) |
| matrix-free 2D / 3D | 6 / 4 | stage solves (Krylov) in all 10 (f_S ≥ 0.86) |
| dense | 27 | Jacobian 10 (finite differences), factorisation 7, RHS 6, stage solves 4 |

Reading. The eight right-hand-side evaluations dominate only when the RHS is expensive (RHS
multiplier 10–100, or the dense radiation network). On banded problems the cost of the
eight-stage chain is the sixteen triangular sweeps (1D) or the factorisation that precedes them
(2D/3D). The project's motivating premise ("eight sequential loops") is correct about the
dependency structure, but the expensive object inside the loop is regime dependent. Each regime
needs its own lever; section 6 maps them. Amdahl bounds for a two-front split of factorisation and
solves (`bound_two_way_P2_factor_and_solves`): 1.27–1.38 on the authoritative 1D cells with an
expensive RHS and 1.69–1.995 on every authoritative 2D/3D cell.

## 2. Theorem SL-1: depth of the stage–row lattice [exact]

**Setting.** `W = I/(h gamma) - J` is banded with half widths `(l, u)` and is factored once per
attempt by banded partial pivoting. The right-hand side is row-local with stencil half width `w`:
row `k` of `f(t, g)` depends on `g_{k-w..k+w}`. Per stage `i` and row `k` the attempt has the nodes
`G(i,k)` (stage argument), `F(i,k)` (right-hand side), `R(i,k)` (assembled linear right-hand side),
`Z(i,k)` (after the forward substitution) and `U(i,k)` (stage solution), with the edges of the
transformed RODAS5P step (`A`, `C` patterns of the protected snapshot, all strictly lower entries
nonzero) and the substitution windows of banded partial pivoting. Cost model "sweep": one unit per
`Z` or `U` row, zero for pointwise nodes; model "unit": one unit for every node.

**Statement.** With `s = 8` stages, the longest path of the attempt is affine in `n` with exact
slopes

| schedule | depth (sweep model) | processors used by the critical path |
|---|---|---|
| one orientation (protected kernel): forward 0→n−1, backward n−1→0 in every stage | `2 s n` = `16 n` | 1 |
| alternating orientation: odd stages LU, even stages UL (LU of the flipped matrix) | `(s + 1) n + c(l,u,w)` = `9 n + c` | ≈ 2 (wavefront) |
| two-way elimination (fronts from both ends, dense middle block) | `s n + c(l,u)` = `8 n + c` | 2 |
| SPIKE-P (P blocks, reduced interface system) | `2 s n / P + c` | P |

**Evidence.** `scripts/stage_lattice_dag.py` builds the DAG for five band shapes
`(l,u,w) ∈ {(1,1,1), (2,2,2), (2,2,1), (4,3,2), (16,16,16)}`, both cost models and
`n ∈ {480, 960, 1920}`; every depth is exactly affine on the three sizes and every slope is the one
in the table (`results/stage_lattice_dag.json`, `all_fits_exact_and_slopes_as_claimed: true`).

**Mechanism.** With one orientation the forward sweep of stage `i+1` starts at row 0, which the
backward sweep of stage `i` finishes last: no overlap, sixteen sweeps in sequence. With alternating
orientation the first sweep of stage `i+1` runs in the direction of the second sweep of stage `i` and
trails it by a lag of `O(l + u + w)` rows, so two sweeps overlap. Two-way elimination halves every
sweep. The pointwise right-hand-side and stage-combination nodes contribute only the intercepts:
**at the scalar level the eight RHS evaluations are not the critical path of a banded attempt; the
sixteen triangular sweeps are.**

**Scope.** The alternating schedule needs a row-streaming right-hand side (`f` evaluated on a row
range as soon as the stage arguments of `range ± w` exist) and a driver that interleaves rows of
consecutive stages. Neither exists in the protected driver; the schedule is a design (plan node
SL-N3). The two-way schedule needs neither: it fits the existing `StageLinearSystem` client
interface (one factorisation, eight solves per attempt) and leaves the driver untouched.

## 3. Theorem SL-2: two-way elimination is partial pivoting of a reordered matrix [exact]

**Setting.** Split at `m = n / 2` with `l <= m <= n - u`, `K = m - l`. The top front eliminates
columns `0 .. K-1` in ascending order and pivots among rows `k ..= k + l`. The bottom front
eliminates columns `n-1 .. m+u` in descending order and pivots among rows `j - u ..= j`. The
`l + u` rows left over are solved by dense partial pivoting on the middle columns `K .. m+u-1`.

**Statement.** This is Gaussian elimination with partial pivoting of `Q W Q^T`, where `Q` orders the
unknowns as `[0 .. K-1, n-1 .. m+u, K .. m+u-1]`, with the same pivot sequence (ties broken by the
first maximum in the order of `Q`).

**Proof sketch.** Column `k < K` has nonzeros only in rows `k-u .. k+l`, all `< m` because
`k + l <= m - 1`; the partial-pivoting search of `Q W Q^T` at that step therefore sees exactly the
top front's window, and its row operations touch rows `k+1 .. k+l < m` and columns
`<= k + u + l <= m + u - 1`. Column `j >= m + u` has nonzeros only in rows `j - u .. j + l`, all
`>= m`; its row operations touch rows `>= m` and columns `>= j - u - l >= m - l`. The two fronts
therefore act on disjoint rows, their updates meet only in the middle columns, and the rows that
remain have nonzeros only in the middle columns: an `(l+u) × (l+u)` dense block. Interchanges inside
a front never leave that front's rows, so the positions of the working matrix map one to one.

**Consequences.**
1. The backward-error analysis of banded partial pivoting applies unchanged (to `Q W Q^T`, hence to
   `W`): no pivot is restricted. This is the property the V19 partition arms lacked.
2. The flop count is that of one-way banded elimination plus `O((l+u)^3)` for the middle block.
3. The two fronts read and write disjoint memory with fixed operation orders, so running them
   concurrently returns the bits of running them in sequence.

**Evidence.** `scripts/twoway_gepp_check.py`: 30 seeded cases (diagonally dominant, weak diagonal,
pivoting-hostile, nearly singular shifted Laplacian, stage-like `W` of a two-species 1D
reaction-diffusion Jacobian; `n` 24–200, half widths 1–6). Pivot sequence equal to an independent
dense partial-pivoting elimination of `Q W Q^T` in 30 of 30 cases; fronts disjoint in 30 of 30;
worst normwise backward error 2.2e-16 against 1.7e-16 for LAPACK `gbsv` on the same systems
(`results/twoway_gepp.json`). Rust unit tests (`stage_clients::banded_two_way::tests`): 30 seeded
cases with `eta <= 2e-15`, serial and joined execution bit-identical, typed `LinearSolve` errors for
zero columns in either front and in the middle block.

**Prior art.** Elimination from both ends of a band is classical ("two-way" or "twisted"
factorisation; "burn at both ends", BABE). The equivalence above is the standard reason it keeps
partial pivoting. SPIKE (Polizzi and Sameh, Computers & Fluids 2006, doi 10.1016/j.compfluid.2005.07.005)
is the P-way partition. What this run adds is the placement: the two-way factorisation used as the
stage solver of an unchanged eight-stage Rosenbrock attempt, with the stage-lattice depth of section 2
as the quantitative reason. Novelty as a numerical method is not claimed.

## 4. Implementation and validation [numerical]

`crates/rodas5p-integrators/src/stage_clients/banded_two_way.rs` (`BandedTwoWayStageLu`,
`TwoWayExecution::{Serial, Join}`) implements section 3 as a `StageLinearSystem` client: no driver
change, the stages, tableau, controller, error estimate and output rules are the protected driver's.
Systems with fewer than `2 (l + u + 1)` unknowns are delegated to the bit-identical tight kernel.
Nothing is allocated after construction.

Whole-IVP checks (`tests/stage_clients_banded_two_way.rs`, `crates/vigilode/tests/banded_stage_policy.rs`):

- 1D Brusselator `n = 200, 500, 600` (rtol `1e-6`, `1e-8`) and 2D Allen–Cahn `12 × 12`: attempt
  counts within 2% of the protected driver (equal in every measured case), final state within `1e-2`
  tolerance units (measured `1.1e-9` to `7.2e-8` tolerance units in the timing runs);
- `TwoWay { workers: 1 }` and `{ workers: 2 }` through the facade: identical states, step counts,
  counters and callback records, with scalar and vector `atol`;
- below the two-front threshold the client equals the protected driver bit for bit.

## 5. Measured cost [measured, diagnostic]

Host: 2 vCPU Xeon 2.1 GHz cloud sandbox, rustc 1.97.0 stable (not the pinned 1.94.1), profile
`measurement`, other build paused, three process runs, no preregistration
(`results/two_way_timing_summary.json`). Ratios are `min(arm) / min(protected)` within a try.

Stage pattern (one factorisation and eight solves), relative to the protected kernel as a client:

| case | tight (same bits) | two-way, 1 thread | two-way, 2 threads |
|---|---|---|---|
| 1D Brusselator `n = 2·10^5`, `l = u = 2` | 0.705–0.770 | 0.768–0.964 | **0.538–0.611** |
| 1D Brusselator `n = 2·10^4` | 0.659–0.691 | 0.926–1.014 | 0.612–0.816 |
| 2D Allen–Cahn `64 × 64`, band 64 | 0.352–0.358 | 0.358–0.449 | **0.243–0.326** |
| 2D Allen–Cahn `32 × 32`, band 32 | 0.340–0.357 | 0.349–0.467 | 0.286–0.319 |

Whole solves (relative to the protected driver with the facade's kernel choice): tight 0.72–0.81 on
1D `n = 2·10^3, 2·10^4`; two-way with two threads 0.64–1.04 on the same cases; on the 2D `32 × 32`
case all arms are within noise of the Slices-kernel driver (0.91–1.29).

Reading. On large bands the two-thread two-way client reaches 1.2–1.45 times the speed of the
same-bits tight kernel per stage pattern, the size of effect section 2 predicts on two processors
once the slower serial two-way kernel (no fixed-offset loops yet) is accounted for. On this host the
gain does not survive into whole solves at `n <= 2·10^4`: the serial two-way kernel is slower than
the tight kernel, a pool is built per solve, and the 2-vCPU host is shared. **No performance claim is
admitted**; the effect is consistent with the theory and needs the owner host (12 cores) with a
preregistered protocol (plan node SL-N2).

## 6. What each idea becomes per solver family

`IDEA_SOLVER_MATRIX.json` records, for every idea of the project (causality, Jacobian-free and
matrix-free in one row, derivative-free, Newton-free, homotopy roots, parallel eight stages, split
forcing, symbolic routing, algebraic geometry, topology/geometry: ten rows) and every solver family
(eleven columns), where the idea
lives today, its status and the next discriminating experiment. The stage-lattice result adds:

- RODAS5P banded: two-way (implemented, opt-in), alternating wavefront (design, needs a
  row-streaming RHS), SPIKE-P with a residual check and two-way fallback (design);
- RODAS5P dense: the factorisation is the lever (parallel blocked LU, V18 not run); the eight solves
  are `O(n^2)` each and parallel within a solve;
- RODAS5P matrix-free: the eight GMRES solves stay sequential; parallelism is inside the Krylov
  matvec and in geometric preconditioners (longitude lines on S²);
- sphere: the FV operator with ring-constant coefficients is block-diagonalised by the DFT in
  longitude into `n_phi` tridiagonal systems in colatitude (`C4` of `results/prior_claims.json`
  verifies the spectrum of the colatitude block); each mode is independent, and each tridiagonal can
  be solved two-way: an exact, mode-parallel stage solver of cost `O(N log n_phi)` (design; needs an
  FFT, and no FFT crate is locked in the workspace);
- BDF and Radau: the same banded two-way client applies to their Newton matrices `I - h beta J`
  (BDF) and to the real block of the Radau transformation; their nonlinear iteration stays.

## 7. Claims and their status

| id | claim | status |
|---|---|---|
| SL-1 | depth of the stage–row lattice: `16n`, `9n + c`, `8n + c`, `16n/P + c` | [exact] (script, 30 fits) |
| SL-2 | two-way elimination = partial pivoting of `Q W Q^T`; no restricted pivoting | [exact] proof sketch + 30/30 pivot sequences |
| SL-3 | serial and joined two-way runs are bit-identical | [numerical] unit, whole-IVP and facade tests |
| SL-4 | two-way stage pattern 0.54–0.61 of the protected kernel at `n = 2·10^5` with 2 threads | [measured, diagnostic] |
| SL-5 | whole-solve gain over the same-bits tight kernel | not shown on this host |
| SL-6 | alternating-orientation wavefront reaches `9n` with a row-streaming RHS | [exact] for the DAG; implementation open |
| SL-7 | sphere: DFT in longitude + tridiagonal in colatitude is an exact stage solver | [exact] for the operator algebra; implementation open |
