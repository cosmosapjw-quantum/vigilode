# Independent programme and integration review

Reviewer: `/root/repo_lineage`, separate from the programme author, Peer patch author and SDC derivation author. Base: `bee5563c51264a7da19c519295bdda2fb317caef`. Review date: 2026-11-15. This review checks raw source/contract semantics; it is not a production release decision or an independent native benchmark.

## Programme findings and resolutions

| ID | Finding | Resolution inspected |
|---|---|---|
| IR-01 | Requiring every Peer stage time to advance would reject valid negative nodes. | U02 now requires finite stage times and advancing endpoints separately. Negative Peer nodes may precede the current committed time. |
| IR-02 | A sigma scan restricted to `[0.8,1.1]` omitted allowed controller halving and smaller final clipping. | U03 now scans `[0.5,1.1]` and smaller positive clipping ratios, with explicit unvalidated-domain/refusal handling. |
| IR-03 | W-approximation tolerance and step ratio were conflated. | U03 explicitly separates `T=theta J` / bounded noncommuting `DeltaT` from `sigma=h/h_prev`; an independent oracle study must not silently alter the exact-J production backend. |
| IR-04 | Old docs-only and qualification-before-registry gate conditions contradicted appended current authorization. | Effective G0 programme-maintenance and G2 frontend conditions were replaced; superseded rules remain only as provenance. |
| IR-05 | Coarse T objectives and concrete U tasks lacked machine-readable correspondence. | `realization_map` and an explicit aggregation rule now prevent a scoped U result from automatically completing T or promoting G0. |
| IR-06 | Proposed `api/`, `capability/`, and transaction directories could induce a duplicate frontend. | Package layout now names existing `problem.rs`, `options.rs`, `production.rs`, `capabilities.rs`, `selection.rs`, `session.rs`, and `solution.rs`. |
| IR-07 | The SDC residual theorem needed its finite-dimensional/fixed-metric domain stated explicitly. | U05 now states finite dimension, fixed SPD H and constant L within each step. |

All 11 task IDs and their dependency/status arrays were compared with the DAG and agreed at review. The write allowlists are repository-relative paths without traversal. This is structural evidence; each task still needs its stated scientific evidence.

## Mathematical check of U05

For `B=diag(3/4,1/4)` and `Q=[[5/12,-1/12],[3/4,1/4]]`, direct rational calculation gives

`BQ + Q^T B = [[5/8,1/8],[1/8,1/8]]`, with determinant `1/16 > 0`.

For finite-dimensional fixed SPD H and `HL=L^T H <= 0`, the symmetric part of `I-h Q tensor L` is positive in the `B tensor H` inner product, hence its inverse has norm at most one. A globally H-Lipschitz remainder with `q=h ||Q||_B ell<1` gives a contraction. The algebraic error from an original collocation residual is bounded by `||r||/(1-q)` in stage norm; extraction of the final stage has norm `1/sqrt(B_22)=2`. The task correctly keeps collocation truncation and floating-point residual enclosure separate. Eigenvalues of nonsymmetric L alone do not justify the same inverse bound.

This check confirms the stated conditional mechanism. It supplies neither uniform stiff order nor a shorter-than-eight measured critical path. The independent SDC experiment/review retains the two-sweep forced-problem failures and cases needing 13 sweeps.

## Peer RHS patch

The six new RHS values depend on their already-computed `(t_i,Y_i)` and do not depend on another new `F_j`. Existing stage arithmetic and tableau construction are unchanged. The parallel branch uses the existing bounded local executor and per-job `WorkCounters`. Callback `Result` is placed inside an outer successful job result; this prevents ordinary callback errors from triggering the executor's early short-circuit. All returned counters are accumulated before ordered result collection chooses the lowest-index error. No state is published until all required values pass.

The serial branch retains prefix-only failure behavior. Bitwise trajectory parity is a valid expectation for pure deterministic callbacks because arithmetic within each stage is unchanged; a thread-safe but order-sensitive callback does not satisfy this condition. The new documentation makes the condition explicit. The tests use a nonlinear nonautonomous vector field, compare every represented state/RHS row and work field, instrument actual concurrent callback occupancy, and check two callback errors plus nonfinite output. Overlap is scheduler evidence, not a wall-time speedup claim.

## Peer transaction and boundary patch

The private `Arc<()>` owner token is retained by both Session and Attempt. `Arc::ptr_eq` plus generation checks prevents a foreign attempt from committing even when two sessions have the same generation number. Keeping the allocation alive through Attempt prevents identity reuse after the originating Session is dropped. Attempts have private fields and are consumed by accept/reject. Advancing generation on rejection invalidates outstanding siblings, while fresh retries remain available.

State validation covers every one of six state and RHS rows, finite values, nonzero dimension, finite time and positive finite previous step size. The step ratio is explicitly positive and finite, catching division underflow/overflow. Public stage preparation/solve boundaries and advance refuse malformed inputs before numerical callbacks. All constructed stage times and arithmetic inputs must be finite. `t+h>t` catches represented-time stagnation. The starter additionally checks representable progress across both h0 subintervals and across the overall 2h0 interval; this deliberately preserves a usable startup time grid rather than accepting a collapsed intermediate reference time.

These checks close the reproduced owner/time/input defects at the Peer boundary. They do not authenticate opaque source callbacks or establish that supplied history values solve the declared differential equation.

## Remaining limits carried into qualification

- `Send + Sync` supplies Rust race-safety; purity, callback side effects and unchanged source/parameter epochs need the planned capability/adapter contract. The session-owner token is not a source-authentication certificate.
- Callback panic/unwind accounting is outside the ordinary `CoreResult` contract reviewed here. It must not be included in a universal complete-failure-accounting claim.
- Existing `solve_stages` precharges six direct solves before its fallible parallel map. A failing solve can therefore leave scheduled-versus-executed work ambiguous. The new RHS aggregation fixes the RHS slice only; total linear-failure work needs a separate correction/qualification test.
- The dense Jacobian, common-W LU and Radau/Newton starter remain. This change establishes neither derivative-free nor Newton-free whole-IVP operation.
- Coefficient/stiff-order/estimator qualification, global achieved-error comparison and frontend method integration remain open. No official G0 or default routing change is justified by this review.

Native RED/GREEN results belong to the patch execution receipt. This reviewer inspected their contract design and source semantics; native tests were not independently rerun by this reviewer because the cloud PATH lacks Cargo. The final source and receipt binding is recorded below.


## Final reviewed binding and disposition

Source SHA-256 values independently matched against local bytes and the implementation receipt:

| File | SHA-256 |
|---|---|
| `crates/rodas5p-integrators/src/ppsw1.rs` | `d37624d65ef2799b6008d86dc159ea6e4b1ea0024576110b4f187603a853c760` |
| `crates/rodas5p-integrators/tests/ppsw1_parallel_rhs_contracts.rs` | `b172a2b84aee926cd9bbb41e71f84ae3af6fe882c72f2121854eb3581b4d871d` |
| `crates/rodas5p-integrators/tests/ppsw1_session_boundary_contracts.rs` | `11631f2f1e14e813fb241dcfcbe47382de1cba51c1a911e70f0fd2b733b0576a` |

`evidence/peer_execution_receipt.json` and all eight referenced execution-log hashes were checked. The final receipt records 123 library tests plus 37 existing Peer contracts, 5 parallel-RHS contracts and 6 boundary contracts passing: **171 passed, 1 pre-existing ignored**. The sixth boundary regression covers the NonFinite `try_step` rejection path, which now also advances generation before returning a rejected decision. These are inspected implementation-worker execution records, not an independent native rerun by this reviewer.

Strict Clippy remains **failed**, with eight findings in unchanged `source_owned_graph_schur.rs`; diagnostic Clippy succeeds with those warnings. It must not be summarized as a clean lint gate. Formatting is recorded as passed for the three edited Rust files.

**Disposition:** no blocking defect found in the bounded RHS scheduling and Peer owner/input/time patch at the hashes above. The reviewed implementation is suitable for this isolated research checkpoint. Whole-solver production qualification, performance and the remaining limits listed above stay open. The programme edits resolve the seven identified specification problems without changing frozen G0 targets or relabeling the method.
