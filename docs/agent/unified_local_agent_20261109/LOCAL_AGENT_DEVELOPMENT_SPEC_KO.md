# VigilODE 통합 Local-Agent 개발 규약

**기준 시점:** 2026-11-09 KST 20:38:53 이후 확인된 원격 상태. **상태:** `HOLD_NOT_RELEASE_QUALIFIED`.

## 1. 목적과 authority

이 문서는 `VIGILODE_UNIFIED_DAG_20261109.json`의 사람이 읽는 해설이다. 기계 판독 기준은 JSON DAG이며, 본 문서는 구현 의미와 실패 경계를 설명한다. 저장소는 `cosmosapjw-quantum/vigilode` 하나만 다룬다. 이 프로젝트 바깥의 Squish-Squash, NS_Cowling, htt_base, BASS/REI 저장소의 코드·수치·결론은 포함하지 않는다. VigilODE 안에 보존된 applied REI/BASS21 branch는 application lineage로 분리한다.

**Git authority 순서:**
1. 최신 연구/문서 HEAD: `research/solver-portfolio-shortdag-20261109` @ `bd82eff9d1d967ef489ac4eb0c302ef6a4102f31`.
2. 선행 계획 branch: `docs/solver-consolidation-plan-20261108` @ `becbc75438354d7bc8ae3fea362ce943d4d2272a`. 최신 research branch의 조상이며 `ahead_by=56, behind_by=0`, merge-base가 plan HEAD다.
3. 구현 코드의 pinned source: `research/vigilode-c5-native-graph-schur-20261102` @ `569ce02629498fec133db9ce368eb303ba943b69`, tree `a4b2f09521b197b37612ff7dc66a13fbb74a0bae`.
4. `main` HEAD `8d0c79184e09efb5bdadc24a6315c60a71a44264`는 2026-08-26의 오래된 상태다. 현재 구현 근거로 대체 사용하지 않는다.

최신 branch의 마지막 커밋 시간은 `2026-11-09T11:38:53Z` = `2026-11-09 20:38:53 KST`다. plan branch에서 최신 연구 branch까지 비교한 diff는 27개 경로로 확인되었고, 보고된 변경 code file은 0개다. 이 package는 문서/연구 통합물이며 코드 기준선을 새로 만들지 않는다.

공식 G0 DAG는 **23 nodes / 34 edges / 7 of 23 historical progress / successor 0 of 12 / Production HOLD**다. 새 DAG는 이 authority graph를 수정하거나 복제하는 대체물이 아니라, 여러 branch 연구를 실행 가능한 task graph로 연결하는 overlay다. 원본 G0 DAG를 찾지 못하거나 blob/branch가 달라지면 숫자를 추정해 재작성하지 말고 HOLD한다.

## 2. 프로젝트 목표와 promotion gate

원래 목표(모두 변경 금지):
- global order >= 4.8;
- protected RJF 대비 accepted-step ratio <= 1.2;
- 같은 achieved global output error에서 median wall speedup >= 1.15;
- fast lane explicit Jacobian builds = 0;
- fast lane Newton iterations = 0;
- critical depth p95 <= 6, absolute maximum <= 8;
- fallback fraction <= 0.1.

이 조건의 달성 상태는 `GOAL_NOT_ACHIEVED`. G0 threshold, tolerance, budget, test target, error/output policy, controller coefficient를 local agent가 임의로 바꾸면 안 된다. 어떠한 개별 테스트 성공도 이 전체 gate를 자동 통과시키지 않는다.

## 3. 작업 시작 전에 반드시 하는 source identity audit

```bash
git remote -v
git status --short --branch
git fetch origin --prune
git rev-parse HEAD
git rev-parse '569ce02629498fec133db9ce368eb303ba943b69^{tree}'
git show -s --format='%H%n%P%n%ci%n%s' bd82eff9d1d967ef489ac4eb0c302ef6a4102f31
git merge-base becbc75438354d7bc8ae3fea362ce943d4d2272a bd82eff9d1d967ef489ac4eb0c302ef6a4102f31
```

실제 checkout은 반드시 dedicated worktree에 둔다. 코딩 전 latest docs HEAD를 읽고, pinned code source commit/tree 및 핵심 blob identity를 확인한다. 하나라도 불일치하면 **코드를 맞춰 바꾸지 말고** source-lock 불일치 보고서를 만들고 중단한다. 브랜치 이름이 같다는 이유만으로 같은 소스라고 간주하지 않는다. 커밋 SHA, tree SHA, file blob SHA를 분리해서 기록한다.

핵심 API blob lock:

| 경로 | SHA-1 | 역할 / 제한 |
|---|---|---|
| `crates/rodas5p-integrators/src/parallel.rs` | `ffdfb1cef258ed43586db0323eb0e7f9de27dd53` | bounded Rayon executor; 새 pool 금지 |
| `crates/rodas5p-integrators/src/stage_batch.rs` | `c2f44c327c53d7c6dfe7ba80b26cd43486d56b43` | RODAS batch feasibility; Peer stage-independence 증명 아님 |
| `crates/rodas5p-integrators/src/sequential.rs` | `4153e286a2ac756252978d5d77cf1abe0e6a4e0c` | protected sequential RODAS5P |
| `crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs` | `d68239d53977e36f98b186b627aa0c4c5c7de7cb` | 8-stage MF RODAS route |
| `crates/rodas5p-integrators/src/source_owned_graph_schur.rs` | `5d2c4319143e60117a7804c1f4524b269df358ea` | source-specific C5 candidate |
| `crates/rodas5p-integrators/src/affine_transport.rs` | `1c7024c2e1c567b337f82b0ba4530a1747022f51` | affine error-map transport kernel |
| `crates/vigilode/src/problem.rs` | `f1b5ef43ec66d11585193b6be9d0ee0ce271f2ec` | Problem / derivative provider contract |
| `crates/vigilode/src/session.rs` | `ce30e5703a8d47a6e258a6df829b19a553fe79f9` | existing session; no Peer two-step history |
| `crates/vigilode/src/automatic.rs` | `ff4f5db411b310039b627d66774b24885eb3cf7d` | current DOPRI5/RODAS5P selector |
| `crates/vigilode/src/switching.rs` | `fb12a401002c0039631a43982cc296a494c366a2` | current accepted-boundary switcher |
| `crates/rodas5p-fair-ab/src/global_error.rs` | `c62f996fa18f9815f1f329766abe8fe2c25ca22b` | current achieved-error/output/work authority |
| `crates/rodas5p-fair-ab/src/adaptive_global_error.rs` | `ccae16353ed1dcc90c6511befea8b42f7d3ad9cd` | clipped/dense paired policy |

The above is a manifest subset; validate all relevant blobs from `SOURCE_LOCK_20261109.json` before source changes. `SOURCE_LOCK_20261109.json` also states current fair-ab authority replay `NOT_RUN`. Do not infer current semantics without T02.

## 4. Scientific and method taxonomy

A method identity is the tuple:

`{equations/tableau, exact coefficient source, formal order, stage dependency graph, step-ratio domain, stability domain, derivative permissions, linear/nonlinear solver, estimator, controller, startup, history, rejection/retry, restart, dense output, events, mass/DAE support, work counters}`.

Never collapse these families:
- **DOPRI5:** explicit one-step, public nonstiff method.
- **RODAS5P protected:** one-step Rosenbrock-W 5(4), eight strictly lower-triangular causal stages; protected stiff reference/fallback.
- **RODAS5P matrix-free:** same RODAS5P stage graph, JVP/Krylov route; matrix-free is not derivative-free and does not remove eight-stage causality.
- **BDF/Radau:** internal reference-only in pinned source; not validated competitive production baselines.
- **PTSW2C:** distinct two-step parallel W-Peer candidate, order 3, common shifted W; source-reported A(81.1°), not L-stable.
- **PTSW2B:** separate two-step order-2 L-stable control; distinct tableau and estimator.
- **high-order p>=5 Peer-W:** source-family literature exists, but no concrete primary coefficient table has been recovered and qualified. PW03 mandatory for G0 order >=4.8.
- **multi-implicit Peer-W, IPP, Rosenbrock-Peer, TASE/SEPP, EPPEER/DQC, diagonal-implicit Peer, reused-stage Peer:** independent method families; exact tableau and dependency graph must be audited independently.
- **homotopy, graph-Schur, polynomial/resolvent charts, quotient/Lawson, Fourier/Volterra:** mostly structure-aware transforms/correctors/source-specific kernels, not automatically generic IVP methods.
- **Gray-Scott and sphere FV:** spatial/MOL clients; spatial parallelism is not time-stage parallelism.

## 5. Derivative and concurrency semantics

- `ANALYTIC_J`, `EXACT_JVP`, `AD_JVP`, `FD_JVP`, `OPERATOR_ONLY`, `RHS_ONLY` are separate capability labels.
- JVP is a derivative action; do not label it derivative-free.
- FD-JVP is approximate and pays extra RHS plus truncation/cancellation risk.
- Matrix-free means no explicit matrix materialization. Linear solves, preconditioners, operator applications and Krylov reductions still cost.
- Newton-free is not solve-free, root-certified or globally convergent.
- Concurrency lanes are separate: RHS/JVP rows, independent IVP ensemble, spatial MOL, current-step method stages, SIMD. Only an exact tableau dependency proof permits time-stage parallelism.
- PTSW common W permits factorization reuse for multiple RHS; it does not automatically imply one shared Krylov recurrence. Validate block Krylov separately.
- Physical time advancement remains sequential across Peer steps; within-step Peer stages may run concurrently only after selected tableau proves no current-step coupling.

## 6. Existing method and negative evidence boundaries

- C5 source-owned graph-Schur actual full-IVP median ratio graph-Schur/fast-direct = `2.242032452000776`; all 12 original tested clipped-grid pairs were slower. Physical path length one, zero affine pair composition. Preserve as a negative measurement, not universal no-go.
- R7 matched achieved-error native transactional-vs-protected comparison is a negative result for tested cases only.
- R9 cycle core and R10 ordered affine scan proofs are source/graph scoped; neither yields generic causal-time parallelism. Ring24 core size one but path length 22 disproves “small feedback core => short critical path”.
- R5 homotopy is source-scoped; not generic unknown-RHS integrator.
- WP6/WP7 Bernstein defect and WP8/WP9 weighted/source response results are source-conditioned; not generic solver certificates.
- C8: finite-state labelled routing; R2/R4 scoped complete, C8 numeric gate pending. Not physical ODE Markov partition.
- PR98 guarded symbolic execution is scoped to labelled controller transitions/ownership; not a physical ODE theorem.
- RVJ/Volterra, PP05/06/15 evidence keeps negatives: PP05 G2/G3 fail, PP06 harmful-alias gate fail, PP15 no timing.
- Applied REI/BASS21 branches are application-only. No physics or source coefficients imported into generic solver claims.

## 7. PW07 error-transfer certificate contract

The current mock supports the algebraic identity

`W (K_tilde - K) = (W - W_tilde) K_tilde - r`, where `r = b - W_tilde K_tilde`,

and the induced bound

`||K_tilde-K|| <= ||W^{-1}|| ( ||W-W_tilde|| ||K_tilde|| + ||r|| )`.

The conditional two-step transfer recurrence is

`E_m^+ <= chi_m E_m^- + eta_m`,
`E_{m+1} <= kappa_m (chi_m E_m + eta_m) + delta_m`.

This is **not** a generic solver certificate. It becomes usable only after the method/source/domain/step-ratio-specific propagation gain `kappa`, full history transfer gain/defect `chi, eta`, and all additive defects `delta` are bounded. `delta` must account for original-W residual/operator perturbation, truncation, root/continuation residual, floating-point/rounding and output defects. Full Peer history must be included in the propagated state. Per-step spectral radius is not a product bound; test nonnormal matrix products and varying step ratios. The affine prefix/scan part is established associative algebra in exact arithmetic and known scan prior art; it does not parallelize the IVP trajectory.

Prior art already includes Peer global error control (2011), ITSP3 global error control (2015), and coupled GL solution/defect estimation (2017). Any novelty hypothesis is restricted to source-attested heterogeneous method/history transfer with original residuals and transactional switch accounting; novelty is unresolved.

PW07 evidence is Python/SymPy mock only. Latest compact driver and result were replayed. No Rust/Cargo/Clippy/native Peer/full-IVP matched-error test was run. The synthetic CPU operator-action overlap was 1.489x (a prior run 1.781x); neither is a solver speed claim.


### PW07 replay values (for triage; details and hashes remain in the primary receipts)
- PTSW2C exact order moment residuals `C(2), Gamma(2), B(3)` are zero; PTSW2B `C(2), Gamma(2), B(2)` are zero.
- PTSW2B stiff-limit characteristic polynomial is `lambda^3`; `M_inf^2 != 0`, `M_inf^3 = 0`. This certifies the stated nilpotent stiff-limit construction, not a whole nonlinear IVP property.
- For `y'=-y`, `[0,1]`, oracle-exact startup, variable ratios alternating `0.8, 1.25`, slopes at N=640 are 2.982554334999267 (PTSW2C) and 1.990300047406758 (PTSW2B). Startup error/cost is excluded.
- Negative real scalar sample at `z=-1e6`: sampled amplification rho about 0.67958 (PTSW2C) and 0.0009543 (PTSW2B); this is not a complex-sector interval proof.
- Nonnormal 2x2 matrix `J=[[-1,K],[0,-10]]`, six-step ratios `[1,0.5,2,0.5,2,1]`: every stepwise spectral radius was <1, but finite-horizon product 2-norm grows with K. For K=100/1000/10000, PTSW2C products are about 7.24/77.47/769.59; PTSW2B about 7.34/80.47/795.23. This is a finite transient witness, not asymptotic instability proof.
- 12D original-W perturbation mock: identity residual `5.2660e-16`, solution error `4.6941e-5`, derived bound `1.1932e-4`.
- RHS-only contraction mock with q=0.65: certified root bound 1.5000000000000004 vs actual 1.5, under its stated invariant-domain/original-residual assumptions.
- 200 synthetic affine error updates: serial recurrence vs composed prefix discrepancy about `4.24e-22`. This is an arithmetic mock, not parallel IVP integration.
- CPU overlap microbenchmark latest ratio 1.489x; previous run 1.781x is retained to show run variability. It is independent operator-action work, not the complete integrator.

### PTSW2B source RHS transformation (keep exact method identity)
The certificate records `p_i = sum_j (Gamma_ij(sigma)/gamma) k_(m-1,j)` and `x_i = k_i + p_i`. With `W_m = I - h_m gamma T_m`, the transformed solve is `W_m x_i = f(t_m+c_i h_m,Y_i) + p_i`, then recover `k_i = x_i - p_i`. It is an exact rearrangement of the source stage equation and removes an explicit `T_m` multiply while building each stage RHS. Both independent stages still solve with the same W. Direct factorization reuse may be possible; do not infer one shared Krylov recurrence or one Krylov solve without a separately tested block-Krylov method.

### Primary literature boundary
- Podhaisky, Schmitt & Weiner (2002), *Design, analysis and testing of some parallel two-step W-methods for stiff systems*, Applied Numerical Mathematics 42, 381–395, DOI `10.1016/S0168-9274(01)00162-3`.
- Schmitt & Weiner (2004), *Parallel Two-Step W-Methods with Peer Variables*, SIAM Journal on Numerical Analysis 42(1), 265–282, DOI `10.1137/S0036142902411057`.
- Schmitt, Weiner & Podhaisky (2005), *Multi-Implicit Peer Two-Step W-Methods for Parallel Time Integration*, BIT Numerical Mathematics 45, 197–217, DOI `10.1007/s10543-005-2635-y`.
- Kulikov & Weiner (2011), *Global error estimation and control in linearly-implicit parallel two-step peer W-methods*, JCAM 236(6), 1226–1239, DOI `10.1016/j.cam.2011.08.006`.
- Kulikov & Weiner (2015), ITSP3 global-error control, DOI `10.1137/140979952`.
- Constantinescu (2017), coupled GL solution/defect global-error estimation, DOI `10.1016/j.cam.2017.05.012`.
- Blelloch (1990), prefix sums / all-prefix-sums prior art: `https://www.cs.cmu.edu/~guyb/papers/Ble93.pdf`.

These sources bound novelty claims. A family theorem is not a certificate for a particular table or implementation.

## 8. Required execution order

**Immediate first action: T02.** T00/T01/T01A/PW00/PW07 are complete within their scoped documentation/mock boundaries. Re-read exact fair-ab sources, reproduce the reference WRMS/global error and clipped/dense pair policy, audit work counters and source authority. Produce a machine-readable receipt with commands, exit codes, stdout/stderr hashes. Until T02 passes, do not rank full-IVP outcomes.

Then proceed on separate worktrees:
1. PW03: recover a concrete primary-source p>=5 Peer-W table and verify exact variable-step order/stability/dependency/startup/estimator.
2. PW01 PTSW2C and PW02 PTSW2B: opt-in distinct prototype kernels; start with RED tests; no public default.
3. T06: qualify BDF/Radau and non-primary Peer/TASE comparators as such.
4. PW04: transactional full Peer history/session/startup/restart, fair-ab adapter and rollback/epoch tests.
5. T03/T04: preregister and measure C5 long-path work economics; T05 only if actual full-IVP crossover appears.
6. PW05: theory-first RHS-only implicit Peer; no production path until original residual-to-root and local/global error proofs are available.
7. PW06: only after PW03/PW04/T02/PW07 source-specific bounds, perform full-IVP matched achieved-error tests.
8. T07 portfolio router extension only after at least two qualified method identities and a safe fallback; reuse existing AutomaticOde/SwitchingSolver; sparse admitted tuples, accepted-boundary switching, hysteresis, transactional history transfer and complete work charging.
9. T08 spatial MOL geometry separately.
10. T09 independent matched-error Pareto/release review; keep HOLD unless every original G0 gate and independent review pass.

## 9. Test and benchmark protocol

Before mutation: clean worktree, exact source locks, task allowlist, preregistered hypotheses and test oracle. Use test-first RED then minimal implementation then GREEN then refactor. Verify:
- exact rational coefficient/order identities;
- variable-step ratio edges and rejection/restart transitions;
- stiff scalar and nonnormal matrix controls;
- startup order, embedded estimator vs achieved error;
- original-W true residual and transform/roundoff defects;
- deterministic serial/parallel parity;
- full history rollback after rejected attempts, callback failure and source/cache epoch change;
- dense/clipped output and event behavior;
- total attempted work includes setup, RHS/JVP, linear solve, preconditioner, Krylov, estimator, startup, probe, rejection, retry, fallback, output/interpolation and allocation.

Suggested commands after code change, adapt only the crate/test filter that exists in source:

```bash
cargo fmt --all -- --check
cargo test -p rodas5p-integrators -- <exact-new-test-filter>
cargo test -p vigilode -- <exact-session-test-filter>
cargo check --workspace --all-targets
cargo test --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Do not claim commands passed unless executed on exact checked-out commit. Record rustc/cargo version, commit/tree, test names, stdout/stderr hashes, exit code, wall-time, peak RSS and artifact hashes. If workspace command is not feasible, record the precise environment blocker and targeted tests completed.

Performance minimum: at least 7 randomized paired AB/BA or blocked timing repetitions where feasible; identical source/RHS/IC/time interval/output grid/output policy; independently validated reference with uncertainty; each method's achieved output error bracketed by actual measurements (no extrapolation, no equal-rtol proxy); include complete work and failures. Separate operation counts, synthetic microbench, native step benchmark and end-to-end full-IVP wall time.

## 10. Mutation gates

**G0 docs-only:** document/replay/provenance/negative ledger updates on dedicated docs branch; no source edits.

**G1 isolated opt-in code:** allowed only for PW01/PW02/PW04 after task-specific prerequisites, own worktree, explicit file allowlist, RED test, opt-in module/API, unchanged protected path, rollback tests, exact hashes and receipt. Never wire default selector.

**G2 structural/portfolio integration:** T05/T07 requires prior authority replay and explicit owner acceptance. No new global pool, no duplicate switcher/WorkLedger/benchmark framework, no exhaustive Cartesian enumeration.

**G3 owner approval:** any default change, Tableau/coefficient/controller/estimator change, threshold/tolerance/budget change, breaking API, official G0 mutation, release/merge, or removal/reclassification of negative evidence.

Forbidden in every gate: direct push to main, force-push/history rewrite, weakening tests, blanket clippy suppression, deleting failed/negative evidence, fabricating run receipts, converting NOT_RUN to PASS, copying unverified source coefficients, silent error-policy changes.

## 11. Low-performance agent self-audit

Before coding, answer from files, not memory:
1. What exact repository, branch, HEAD, tree and task ID am I on?
2. Which exact source blobs authorize the API I will call?
3. Is this task generic method, source-specific transform, backend, spatial operator, or application adapter?
4. What are method's tableau, order, step-ratio domain, stage dependency, estimator, startup/history/restart and output semantics?
5. Is derivative permission ANALYTIC_J / EXACT_JVP / AD_JVP / FD_JVP / OPERATOR_ONLY / RHS_ONLY?
6. What is the original-source residual and what does it not bound?
7. Which output and achieved-error authority is current? Has T02 been completed?
8. Which work items must be charged even on rejection/fallback?
9. What exact file paths are allowed to mutate? Which paths must remain byte-identical?
10. What is the stop condition and what evidence would falsify the current hypothesis?

If an answer is missing: mark `UNRESOLVED`, request source evidence in a bounded report, and stop the affected implementation. Do not guess. Do not fix a source-lock mismatch by changing the source lock to fit new files without review. Do not expand scope to unrelated PRs/repositories. After two no-progress cycles on the same obligation, close it as blocked or change the mathematical question.

## 12. Claim language

- `ESTABLISHED`: textbook/theorem or primary literature directly supports the precise statement.
- `LITERATURE_SUPPORTED`: peer-reviewed/source citation supports family/property but not local implementation.
- `DERIVED`: explicit derivation with stated assumptions.
- `NUMERICALLY_CHECKED`: specified numerical test passed; does not imply proof or source-code integration.
- `IMPLEMENTATION_VERIFIED`: exact source commit compiled/tests ran and source blob identity recorded.
- `CONJECTURAL`: candidate idea without full proof/evidence.
- `UNRESOLVED`: open obligation; do not make a claim.
- `BLOCKED`: execution impossible due to a named dependency/environment/permission blocker.
- `NOT_RUN`: no execution occurred.

Final rule: preserve provenance and failure taxonomy. A successful JSON parser, a passing mock, a green targeted CI job, and a full solver acceptance are four different claims.
