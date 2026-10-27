# VigilODE mode-level audit and constructive optimization

Source: `cosmosapjw-quantum/vigilode`, baseline commit
`3064d453091af1dec7f99f0c8afabd98b447ea34`.
Status: implementation and bounded verification COMPLETE (32 tests passed).
Performance timing is owned by the parent and is reported separately.

This note concerns numerical-mode kernels, independently of the adapter's
choice, pilot scheduling and restart state. Sources were read at the current
baseline. No historical numerical campaign was rerun.

## Evidence and scope

The most recent source report is
`external_reports/adapter_profile_audit_20261027/ISSUES_KO.md` and its machine
record `ISSUES.json`. Its measurements are tied to source `46939bbe` on an AMD
Ryzen 9 5900X; its wall times are explicitly diagnostic. This audit uses those
instruction/allocation findings to select concrete surfaces, not as speedups
on the current host. The earlier integration report is
`research/adapter_integration_20261027/REPORT_KO.md` at `dda8d2f...`.

The kernel-level observations must not be collapsed into selector overhead:

* For case5 the latest report records MF RODAS 4.60M versus dense 0.67M
  instructions, despite equal attempts/RHS in its controlled assembled-JVP
  comparison. This is an inner linear-algebra/provider cost, not a failure of
  the explicit/implicit selector alone.
* Its MF least-squares correction accounts for 39% of transition-39 MF RODAS
  instructions, with QR factorization 18%. Replacing QR with Givens would
  change numerical arithmetic and requires rank/breakdown semantics; that
  larger change is not admitted by this exact-storage patch.
* Default small dense RODAS already uses in-place zero-skipping LU. The Faer
  patch below applies to the separately selected dense Faer kernel, normally
  dense dimension above 64. It cannot explain or fix the reported n<=6 MF
  penalty.
* Nonautonomous MF relinearization allocates an unnecessary analytic `f_t`
  vector. Its state-owning JVP closure still allocates a copied state and an
  Arc. Only the first site is closed here.
* The latest report places persistent DOPRI session overhead at roughly
  1--2.5% instructions and landing-helper inlining recovery at at most about
  3%. These are recorded prior findings, not newly measured gains. No DOPRI
  arithmetic/controller change is included in this mode patch.
* The causal/residual-first coupled path has an independent avoidable
  allocation at every row of the projected stage residual. It does not
  change how the projected root is certified in full space.

## Patch 1: dense Faer stage solves

### Actual defect and remedy

`rodas5p-core/src/matrix.rs::LuFactorization::solve` constructs an RHS Mat,
calls Faer's allocating `Solve::solve` (which owns a solution Mat), allocates
permutation scratch and collects another Vec. `Workspace::solve` in
`rodas5p-integrators/src/rodas5p_fast.rs` repeats this at each of eight stages
and copies that Vec back to the already owned stage RHS. Dense-output
corrections and lagged-factor preconditioner applications use the same
allocating path.

New `LuSolveWorkspace` retains one aligned Faer RHS column and the permutation
scratch. `solve_in_place_with_workspace` copies the caller RHS into that
column, calls exactly the same Faer partial-pivot permutation and triangular
solve kernels, then copies back. The aligned Mat preserves the layout of the
old allocated solve, so an unaligned caller Vec does not silently select a
different triangular kernel. The ordinary adaptive workspace retains this
storage across all successful factorizations of a fixed dimension. A second
convenience method, `solve_in_place`, creates temporary aligned storage for
immutable preconditioner users and avoids the extra solution Mat/Vec; it is
not claimed allocation-free.

The original `solve` remains unchanged and is used as a direct same-binary
control. The factorization itself, pivoting, stage equations, acceptance,
controller and native counters are unchanged. Failed numerical solves may
modify the caller RHS, as explicitly documented; they remain errors and are
not accepted states.

### Derived storage equivalence lemma

Fix the LU factors, row permutation, dimension, Faer version and parallelism.
Let the old and retained aligned RHS matrices have the same shape, strides
and entries at kernel entry. Let permutation scratch be sufficiently sized
and scratch-only (never read before initialization by the kernel). Both
executions call the same permutation, unit-lower solve and upper solve in the
same order, on identical factor entries. Induction over those kernels gives
the same mathematical solution. With identical kernel selection and floating
operation order, the resulting binary64 values are also identical. The tests
check this stronger numerical identity on actual pivoted matrices; the
mathematical conclusion does not rely on benchmark timings.

The reusable object owns no factorization or Jacobian identity. Therefore it
may be reused after refactorization at another time or step size, provided
the dimension matches. Reusing scratch does not license reusing stale LU
factors. Invalid RHS/workspace dimensions and nonfinite RHS are rejected
before modification.

Complexity remains O(n^2) arithmetic for a dense single-RHS solve. The saved
cost consists of O(n) allocation/copy traffic and allocator overhead. Dense
factorization remains O(n^3), so large-n full-trajectory speedups must be
smaller than a solve-only microbenchmark. The persistent storage is O(n),
not another O(n^2) matrix.

## Patch 2: matrix-free analytic partial time derivative

The current MF fresh-state branch computes
`let ft = problem.eval_partial_t(...); self.ft.copy_from_slice(&ft)`.
It now calls the existing `eval_partial_t_into(..., &mut self.ft, ...)`.
That method already zeros the destination before the callback, increments
the same `ft_calls`, invokes the same callback and checks finiteness. Thus
callbacks accumulating into a zero buffer retain their semantics, and the
same state/epoch cache invalidation occurs before any fallible callback.

For an analytic supplied nonautonomous partial-time callback this removes
one Vec allocation per fresh linearization. It changes no callback counts
or arithmetic. Autonomous problems already use `fill(0)` and receive no
new saving. The finite-difference `f_t` fallback still allocates its RHS
vectors and difference result; that path is not represented as closed.
The existing `partial_t_into_contracts` verifies the zero-buffer and
counter-equivalence contract independently of the MF call-site change.

## Patch 3: coupled projected residual rows

In `vigilode-coupled/src/residual_first.rs::projected_residual`, every
projected row was copied into a fresh Vec merely to compute a compensated
dot product. The new call `dot(matrix.row(i), z)` borrows the same row-major
entries in the same ascending order. The local `dot` algorithm remains
unchanged, including its compensation recurrence. This removes `d=S*m`
row allocations and `d^2` entry copies for a projected residual evaluation
of dimension d. It does not remove the O(d^2) arithmetic, change the basis,
stage target, residual norm or full-space validation.

This exact row-substitution lemma is independent of whether the coupled
method converges quickly. Inadequate basis information and residual-to-error
certification are mathematical questions that this storage optimization
does not resolve.

## Tests and bounded experiments

New core test: `lu_workspace_contracts`, seven pivoted system dimensions
1, 2, 7, 32, 65, 96, 192; known exact solutions, bit comparisons against
the old API, warmed zero-allocation solves, wrong shape/workspace rejection,
nonfinite input rejection and scratch reuse across different matrices.

New `rodas5p-core` example `lu_workspace_bench` compares old and retained
storage within one binary, one LU, identical RHS and accuracy. Setup and
JSON serialization are outside its timer. This is a kernel experiment.

New baseline-compatible `rodas5p-integrators` example
`mode_dense_trajectory_20261027` solves

`y' = (-lambda I + kappa/n 11^T)y`, lambda=200, kappa=50,

with exact solution

`y_i(t)=exp(-lambda t)(y_i(0)-mean(y0)) + exp((kappa-lambda)t)mean(y0)`.

It measures the full adaptive solve including solver setup, rejection,
output and counters at 33 common times on [0,0.2], with rtol=1e-7,
atol=1e-10, initial h=1e-5 and max-grid-component-error cap 1e-7. Dimensions
65, 96 and 192 force the actual dense Faer route. Entire time/state output
and counters are emitted to permit baseline/candidate identity checks.
This uses a dense two-eigenvalue linear problem to isolate storage cost;
it is not a nonlinear or broad-spectrum solver superiority result.

Validation uses the parent's checksum-verified official Rust 1.94.1,
`--locked --offline --profile measurement`, `CARGO_BUILD_JOBS=2`, and
single test threads. Commands, complete stdout/stderr, exit codes and monotonic
elapsed times are in `evidence/mode_validation/`.

Completed targeted checks:

| Target | Tests | Exit | Command elapsed (including compilation) |
| --- | ---: | ---: | ---: |
| core `lu_workspace_contracts` | 1 | 0 | 159.049 s |
| integrator `rodas5p_fast_contracts`, `partial_t_into_contracts`, `fast_dense_contracts` | 16 | 0 | 105.938 s |
| coupled `economy_contracts`, `economy_robustness` | 14 | 0 | 7.892 s |
| MF analytic nonautonomous `work_counter_lane_contracts` (one named test) | 1 | 0 | 130.827 s |

The core test includes seven pivoted dimensions, bitwise equality to the
original allocating solve, zero warm solve allocations and error preconditions.
The retained integration tests check prior golden trajectory digests, stage
and dense-output contracts, counters, full-space validation, budget and callback
failure behavior. The direct analytic nonautonomous MF trajectory contract
also passed: the existing `jvp_calls_equal_user_jvp_callbacks_on_the_matrix_free_lane` executes
a nonlinear Prothero--Robinson problem with an analytic partial-time callback,
on [0,1], through the production MF adaptive driver, then checks success and
independently counted RHS/JVP callbacks. Exactly this one test was selected.
In total, 32 focused tests passed and no numerical test failed.
Both candidate examples built successfully after the local artifact
invalidation described below: LU example 26.988 s, full dense trajectory
example 101.246 s (both exit 0).

The first abort-profile example build failed because the shared target cache
still contained a baseline core artifact that did not export the new API;
source and the separately compiled test-profile API were present. The failure
receipt is retained (`build_lu.receipt.json`). Only local core/integrators
package artifacts were invalidated before the rebuild; dependency artifacts
were preserved. No numerical test failed. Performance timing belongs to the
parent's serialized comparison and is not asserted in this note yet.

## Remaining concrete work

Large unresolved mode costs are the small-system MF QR/orthogonalization
policy, per-linearization operator allocation and finite-difference Jacobian
buffers. A tiny dense provider may be more economical than MF, but replacing
a caller's declared JVP provider needs an explicit admissible representation
choice; a fixed numerical WorkUnit count is not a time oracle. These are
separate from implicit-session reconstruction and repeated uncompetitive
pilot costs owned by the adapter lane.

Skill context actually used: PhysMath Research Loop 1.3.0 SKILL.md,
POLICY_INTEGRATION.md and CODING_LOOP.md (cloud c38), selected R3-R7/C2-C9.
No recursive delegation, no history rewrite, no broad campaign or remote
compute, and no independent publication/commit by this lane.


## Finite complementary adapter review

The mode lane reviewed the adapter's changed production predicates and tests
once, without recursively reopening the audit. One concrete reporting issue
was found: the newly documented incomplete-comparator ratio was advertised as
an upper bound but initially used ordinary rounded division. The adapter lane
replaced it with outward conversion of exact integer work totals, outward
`next_up` division and `None` for an invalid denominator. Admission was already
conservative; the repair makes the displayed ratio certificate conservative
as well. The adapter also explicitly distinguishes incomplete prefix evidence
from a completed implicit trajectory. No further concrete defect was found in
that bounded review.

Early comparator stopping preserves the completed explicit output and omits
future comparator callback execution. Equivalence to an unbounded comparator's
future failure behavior therefore requires those unobserved callbacks to remain
well-defined; an observed fatal callback is checked before the stop. This is a
scope qualification, not a claim that all future failures can be predicted.


## Candidate binary identities

These are built artifacts, not benchmark results:

* `tmp/bin_candidate/lu_workspace_bench`:
  SHA256 `03188f4c4fdea8348baa290acba23387b3670098e977542f7ff801eb9a204ed9`.
* `tmp/bin_candidate/mode_dense_trajectory_20261027`:
  SHA256 `b6abdeb2dc92c1eb89d5d852428dca54095ff9ba4b92b3cb871573c99113204b`.
* Parent baseline `tmp/bin_baseline/mode_dense_trajectory_20261027`:
  SHA256 `62b9155edf531d6e3122353fc109a4e438ba8e8871e6b3aa72056d7ab158ade9`.

The parent owns serial wall-time collection and baseline/candidate trajectory
comparison. This lane has not run the performance binaries.


Verification is complete for this bounded mode patch. The parent was notified
that the cargo slot is released. No performance workload was timed by this
lane, so timing collection can run without concurrent compilation. The only
compiler diagnostic in successful runs is the pre-existing unused
`VerifiedTrialFault::EndpointAdmission` variant in `packet_trajectory.rs`.
