# 미이식 수학 이식 계획 (loop L1 node V12)

이 문서는 `render_plan.py`가 `port_contracts.json`(계약)과 `SOURCE_STATUS.json`(git으로 확인한 출처·도착 상태, `verify_sources.py` 출력)에서 생성한다.

기준 head: `beed3789`. 상태: Updated 2026-11-17 after the adoption of the switcher lineage (V02) and wave 2 groups 1-2; fields status_20261117, result_this_loop and next_discriminator are current; rows of nodes still running are updated at the close of the loop.

## 1. 한눈에 보기

| 상태 | 항목 수 | 뜻 |
|---|---|---|
| 이식됨·판별 실험 없음 | 5 | code and its own tests at the head, no executed discriminating experiment |
| 이식됨·미연결 | 4 | code at the head, not reachable from a stage client or from the facade route it was meant for |
| 미이식 | 4 | source exists on another line only |
| 설계만 있음 | 2 | no code |
| 이식됨·실측됨 | 1 | code at the head and an executed discriminating experiment exists |
| test 안에만 있음 | 1 | exists only inside a test |
| 이식됨·선행 조건 대기 | 1 | code at the head, blocked by a stated precondition |
| archive | 1 | recorded archive member |
| head에 후속 구현 있음 | 1 | the head holds a successor |

우선순위: P1 = 이번 loop의 node가 사용, P2 = 다음 loop, P3 = 선행 조건이 풀릴 때까지 보류.

## 2. 항목별 계약

| ID | P | 주제 | 기제 | 상태 | 도착 (git 확인) | 이번 loop 결과 | 다음 판별 실험 |
|---|---|---|---|---|---|---|---|
| P01 | P1 | parallel/Krylov | Shared Krylov basis across the 8 stage solves | 이식됨·실측됨 | head에 있음 | V06 stage a FAIL on executed runs: per-stage GMRES JVP count divided by shared-basis JVP count was 1.12-1.78. Only Gray-Scott passed the stage-a threshold of 1.5 at both tolerances; two of three nonlinear families were required. The final separate gate asks for shared/per-stage <=0.5 (research/production_loop_20261116/shared_krylov/RESULTS.json, ledger L-0315). The v2 kernel was not built. Leads moved to node V17 (residual test after every Arnoldi column; linear tolerance tied to the step tolerance). | none for the shared basis; see V17 for the two leads |
| P02 | P1 | Jacobian | Coloured (Curtis-Powell-Reid) and threaded Jacobian assembly | 이식됨·미연결 | head에 있음 | the colouring module is at the head and serves Jacobian::sparse_jvp of the facade; the stage client assembly.rs is a stub. Node V08 (group 3 of wave 2) builds and measures the threaded and coloured assembly. | V08 frozen gate (wall >= 1.5x on finite-difference problems, n >= 64, right-hand side >= 50 microseconds; bitwise equal Jacobian) |
| P03 | P1 | geometry | Spatial PDE battery v3 with sphere surface diffusion | 이식됨·미연결 | head에 있음 | the PDE models are at the head inside crates/vigilode-bench, which is outside the workspace and declares rust-version 1.99.0 (hosted bench job fails for that reason, ci/TRIAGE_KO.md). Node V11 (group 4) moves operators to crates/rodas5p-integrators/src/mol/ and fixes the manifests. | V11 gate (second-order convergence, conservation <= 1e-12, matched-error table) |
| P04 | P1 | algebraic topology | Oriented incidence operators D0, D1 and Hodge Laplacians | test 안에만 있음 | 일부만 head에 있음 | unchanged: one test at the head; no backend. Planned inside V11 (mol/complex.rs). | as written (first 16 Laplace-Beltrami eigenvalues at two refinements) |
| P05 | P1 | dynamics | DOPRI5 secant stiffness estimate | 이식됨·판별 실험 없음 | head에 있음 | module and guard tests are at the head; no executed agreement study exists (ledger L-0213 was a literal count, superseded as INVALID by L-0311). | as written; run on the battery of V11 with labels from a stiff/non-stiff reference solver pair |
| P06 | P1 | dynamics | Nonnormal observers (weighted logarithmic norm, transient growth) | 이식됨·판별 실험 없음 | head에 있음 (후속 branch commit 미병합) | module and contract tests are at the head; the follow-up branch is superseded (field follow_up_612063db). Not an input of any route. | on L=[[-1,100],[0,-1]] and on a 2D advection-diffusion operator the observer must flag transient growth that the spectral abscissa misses, at stated cost |
| P07 | P1 | symbolic dynamics | Hybrid route automaton (dwell time, hysteresis, epoch invalidation) | 이식됨·미연결 | head에 있음 | module and contract tests are at the head (crates/vigilode/src/hybrid_automaton.rs; its documentation link was repaired in this loop). Whether the unified frontend uses it is decided by the V03 nodes. | as written (replay of recorded switching traces) |
| P08 | P2 | Krylov | Secant preconditioner | 이식됨·판별 실험 없음 | head에 있음 | module and contract tests are at the head; never combined with a stage client. | as written; the Krylov stage client of V06/V17 is the place to measure it |
| P09 | P2 | Krylov | N02 triggered flexible GMRES | 미이식 | head에 없음 | source present on the rei line only (278 lines). A port is a new file plus one module line in crates/rodas5p-krylov/src/lib.rs, which is outside the allowlist of this loop. | matvec count versus gmres_into at equal residual on the stage systems of the battery |
| P10 | P2 | certificates | N03 stage interval certificate | 미이식 | head에 없음 | source present on the rei line only (292 lines). | 13 native tests rerun on the loop branch |
| P11 | P2 | events | Affine event certificate | 미이식 | head에 없음 | source present on the sym02 branch only (705 lines). | event time error against a root-polished reference on 3 problems; no missed sign change |
| P12 | P2 | parallel | Work-efficient (Blelloch) and chain-parallel affine scan | 설계만 있음 | 코드 경로 없음 | unchanged. | against the existing Hillis-Steele loop on ring48/96: work <= 2m and equal residual |
| P13 | P2 | algebraic | Chebyshev-product resolvent and complex Leja points | 이식됨·판별 실험 없음 | head에 있음 (후속 branch commit 미병합) | modules and tests are at the head; the follow-up branch is superseded (field follow_up_612063db). | phi-action error against the certified bound on a nonnormal operator |
| P14 | P3 | homotopy | Preconditioned fixed-point homotopy | 이식됨·선행 조건 대기 | head에 있음 (후속 branch commit 미병합) | module at the head; the follow-up branch is superseded (field follow_up_612063db); still blocked on the restricted polynomial lane. | restricted polynomial lane with Bernstein enclosure and a replay verifier before any port |
| P15 | P3 | causal | RVJ N3R/N3H/N4 represented-clock envelopes | 미이식 | 코드 경로 없음 | both source directories exist on origin/research/rvj-n4b-two-window-20261031 (and on the earlier rvj-n3r, rvj-n4 branches). | branch receipts rerun |
| P16 | P3 | symbolic dynamics | Guarded transition words with product-norm bounds | 설계만 있음 | 코드 경로 없음 | unchanged. | U10 found majorants valid but 1e15-1e176 loose: a tighter bound on one switching pair first |
| P17 | P3 | algebraic geometry | Whole-slab trace certificate in time | 이식됨·판별 실험 없음 | head에 있음 | module at the head; the slab experiment was not run. | slab time-error propagation on the triple-root case a=b=3/4 |
| P18 | P3 | algebraic geometry | Cyclic closure and cofactor chart certificate boundary | 이식됨·미연결 | head에 있음 | modules at the head; no certificate boundary. | counterexample suite of U08 extended to the closure |
| P19 | P3 | archive | RVJ archive members (shift_jet, fusion, fft_bridge, witness_lift, fourier_volterra) | archive | 코드 경로 없음 | unchanged; routes recorded in research/production_unification_20261115/HISTORY_RECOVERY.json. | byte and behaviour comparison with head before any port |
| P20 | P3 | matrix-free | PR33 matrix-free W correction; residual_gain/polynomial_action extensions | head에 후속 구현 있음 | head에 있음 (후속 branch commit 미병합) | see field superseded_at_head. | none |

## 3. 출처와 대상

| ID | 출처 | 대상 | 지켜야 할 불변량 |
|---|---|---|---|
| P01 | switcher 933922495045 (arrives by V02 merge if adopted, else blob-identical port): crates/rodas5p-krylov/src/shared_basis.rs + hook in rodas5p_matrix_free_fast.rs | stage client (stage_clients/krylov.rs) + rodas5p-krylov/src/shared_basis_v2.rs | true stage residual checked; every JVP charged; basis reset on new Jacobian |
| P02 | switcher 933922495045 (arrives by V02 merge if adopted, else blob-identical port): crates/vigilode/src/coloring.rs; banded grouping already in crates/vigilode/src/fd.rs:366 | stage_clients/{assembly,coloring}.rs | bitwise equal J and trajectory to sequential assembly; wrong pattern fails closed |
| P03 | switcher 933922495045 (arrives by V02 merge if adopted, else blob-identical port): crates/vigilode-bench/src/pde_models/{spherical,gray_scott,allen_cahn,burgers,sine_gordon}.rs, spatial_grid.rs | crates/rodas5p-integrators/src/mol/ + battery v4 runner under the evidence root | quadrature-weighted symmetry, conservation<=1e-12, second-order convergence |
| P04 | 074a5699: crates/vigilode-coupled/tests/structure_certificate_transfer.rs (test t1 only) | mol/complex.rs (discrete exterior calculus Laplacian for sphere meshes) | D1*D0=0 in integer arithmetic; constants in the kernel; mass conservation of the induced FV operator |
| P05 | switcher 933922495045 (arrives by V02 merge if adopted, else blob-identical port): crates/rodas5p-integrators/src/dopri5_stiffness.rs | input of the opt-in Auto policy (V03) | no extra callbacks; estimate reproduces the scalar linear mode |
| P06 | switcher 933922495045 (arrives by V02 merge if adopted, else blob-identical port): crates/rodas5p-core/src/nonnormal_observers.rs (+ authority fix on 612063db) | input of the opt-in Auto policy; diagnostic in Solution stats | sign of the dissipative diagonal kept; eigenvalue-only tests are not used |
| P07 | switcher 933922495045 (arrives by V02 merge if adopted, else blob-identical port): crates/vigilode/src/hybrid_automaton.rs, margin.rs | typed fallback of the opt-in Auto policy (V03) | bounded number of switches per window; discarded work charged; state/history/controller atomic on rejection |
| P08 | switcher 933922495045 (arrives by V02 merge if adopted, else blob-identical port): crates/rodas5p-krylov/src/secant_preconditioner.rs | right preconditioner of the Krylov stage clients | preconditioner fixed within a step; true residual unchanged |
| P09 | rei 6844c8fe: crates/rodas5p-krylov/src/gmres_into_triggered.rs | new file in rodas5p-krylov (the rei and switcher lines both edit gmres_into.rs: do not merge that file) | opt-in; HOLD |
| P10 | rei 6844c8fe: crates/rodas5p-krylov/src/stage_interval_certificate.rs | verification layer for stage clients | source interval assumptions explicit |
| P11 | origin/research/sym02-native-affine 8357faa5 (PR102/103): crates/rodas5p-integrators/src/affine_event_certificate.rs | in-loop event location (release-gate axes R1/R3: events are post-processed today) | scalar-affine events only |
| P12 | research/vigilode-dyn-symbolic-c8-20261102 3336d682: R4_CANDIDATES_AND_ROUTER.md sections A.4-A.5 | source_owned_graph_schur.rs is protected: a new client only | original W residual and reassociation rounding checked |
| P13 | switcher 933922495045 (arrives by V02 merge if adopted, else blob-identical port) and 612063db: conditioned_polynomial.rs, leja_action.rs | exponential lane (pexprb adapter of V03) | full-space witness and original residual kept |
| P14 | switcher 933922495045 (arrives by V02 merge if adopted, else blob-identical port) and 612063db: crates/vigilode-coupled/src/rhs_homotopy.rs | vigilode-coupled only | a fixed point gives F=0 only for nonsingular P; finite queries cannot bound the derivative supremum (x12 blocker) |
| P15 | PR139-147 branches: research/rvj_n3r_represented_upper_20261030/rust, research/rvj_n4_cert_kernel_rawu8_20261030/rust | specialised collective client | rank-2 source, h<=1/4, degree<=9 |
| P16 | lineage PORT_SYMBOLIC (design only) | Auto policy proof obligations | bound must be evaluated on the real error-transfer map |
| P17 | 074a5699: crates/vigilode-coupled/src/trace_certificate.rs::certify_trace_path | vigilode-coupled certificate boundary | fixed model; trace is the sum over all complex roots, not a selected branch |
| P18 | 074a5699: crates/vigilode-coupled/src/cyclic_closure.rs, crates/vigilode/src/cofactor_chart.rs | certificate boundary port as U08 did for the trace | fixed model, selected branch, incoming radius, clock continuity |
| P19 | vigilode_RVJ_full_history_20261004.tar.zst (sha256 de25f591...) | none yet | per HISTORY_RECOVERY.json routes |
| P20 | 981bf1bc (origin/research/audit2-matrix-free-w); 76e59c69 (origin/audit/rvj-reaudit-remaining-20261011) | check first whether head supersedes them | - |

## 4. 후속 branch 확인 (612063db)

Trial merge of 612063db (7 commits of the switcher follow-up branch, not an ancestor of the loop head) on a scratch branch: conflicts in 8 files and 38 hunks (1777 conflict lines); aborted and removed. The four authority fixes of that branch are present at the head as independent implementations with their own tests (see P06, P13, P14); the branch is not merged.

- P06: superseded at head by an independent implementation: sampled observer modes report SampledDiagnostic or EstimateOnly and only the verified full-slab mode reports Certified (crates/rodas5p-core/src/nonnormal_observers.rs:43-44, 307-310, 502-505, 683-686); a merge of 612063db conflicts in 3 hunks of this file and is not made
- P13: superseded at head by an independent implementation: the Leja witness carries the canonical digest of the operator and a mismatch is refused (crates/rodas5p-core/src/leja_action.rs:2920-2946, 3367-3433; test test_auth_002_normality_witness_identity_binding); only VerifiedSpectralEnclosure can mint a certified Gershgorin source (crates/rodas5p-integrators/src/conditioned_polynomial.rs:817-952; test auth_003_negative_control_caller_arbitrary_intervals_cannot_mint_certified); a merge conflicts in 16 hunks of the two files and is not made
- P14: superseded at head by an independent implementation: tests caller_scalar_evidence_cannot_authorize_contraction_certificate and independent_checker_tamper_suite_rejects_all_corrupted_artifacts (crates/vigilode-coupled/tests/rhs_homotopy_contracts.rs:366, 393); 41 of the 44 items that 52dec17e adds to rhs_homotopy.rs exist at head by name; a merge conflicts in 9 hunks and is not made

## 5. 읽는 법

- '이식됨'은 코드가 loop branch head에 있다는 뜻이고, 이득이 실측됐다는 뜻이 아니다. 이득 주장은 '다음 판별 실험'을 실행한 뒤에만 한다.
- 계약의 규칙: A port keeps the original target and error; the discriminating experiment runs before any claim of benefit; evidence must come from executed runs.
- 다시 만들기: `python3 -I research/production_loop_20261116/port_plan/verify_sources.py && python3 -I research/production_loop_20261116/port_plan/render_plan.py`.
