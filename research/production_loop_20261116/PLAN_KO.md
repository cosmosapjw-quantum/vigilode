# VigilODE 연구 루프 L1 (2026-11-16) — 감사 · 자격 심사 · 통합 · 8단계 병렬화

## Context

요청: 최신 코드 기준으로 이론/코딩 연구 루프를 실행한다. 지금까지의 갱신 내역을 비판적으로 감사하고 production solver 자격을 심사하며, 부족하면 다음 계획을 낸다. 흩어진 적분기를 단일 frontend의 production 패키지로 통합하고, 이식된/미이식 수학을 계보에서 찾아 이식 계획을 세우며, 특히 RODAS5P 8단계 순차 루프의 병렬화를 연구한다.

계획 단계(read-only)에서 감사의 상당 부분을 이미 수행했다. 1절이 그 결과, 2절이 소유자 결정, 3절 이후가 승인 후 실행할 작업이다.

- 최신 코드: `research/production-unification-20261115` @ `074a5699` (worktree `~/worktrees/vigilode-prod-unif-20261115`, toolchain 1.94.1). U01–U11 실행 완료, production **HOLD**.
- 메인 체크아웃(`research/switcher-v2-20261103` @ `93392249`)의 gate 작업(G0–X11)은 전부 done, READY 없음. 메인 체크아웃은 hook이 편집을 막으므로 새 worktree에서 작업한다.

## 1. 계획 단계 감사 결과 (확인된 사실)

### 1.1 계보가 둘로 갈라져 있다

- switcher 계열(`93392249`, `vig-authority-followup` `612063db`, `task/*`)은 `074a5699`의 조상이 아니다. merge-base `569ce026`(11-02). 고유 52커밋, crates +28k줄(src +16k): `unified/` frontend, `registry.rs`, `hybrid_automaton.rs`, `nonnormal_observers.rs`, `dopri5_stiffness.rs`, `shared_basis.rs`, `secant_preconditioner.rs`, `coloring.rs`, Chebyshev/complex-Leja 확장, PDE battery v3(구면 포함), cross-solver 벤치.
- 통합 프로그램(11-09~16)은 C5 `569ce026`를 코드 권위로 고정했고, `lineage.json`·`LINEAGE.md`·`HISTORY_RECOVERY.json`에 이 계열 언급이 0건이다. 누락 사유 기록 없음.
- 양쪽이 함께 고친 파일은 `crates/rodas5p-integrators/src/lib.rs` 하나뿐이다. 단 switcher 쪽은 CHECKPOINT 보호 파일 5개(`sequential.rs`, `vigilode/src/lib.rs`, `switching.rs`, `Cargo.toml`, `rust-toolchain.toml`)와 `rodas5p_fast.rs`·`adaptive.rs`를 수정했고 toolchain을 stable/1.99.0으로 올렸다.
- LEDGER 분기: 통합 브랜치는 L-0204에서 끝난다(11-09 이후 U 노드 행 0개). switcher는 L-0205..L-0214 보유.
- REI 계열(`6844c8fe`, 고유 287커밋)도 미병합: N02 triggered FGMRES, N03 stage interval certificate, 46,080-cell(n=184,320) Peer-W 실험(별도 세션 진행 중, 실행 단계 X3 미실행).

### 1.2 증거 진위와 실험 설계 결함

- **합성 증거**: switcher의 `eval_k1.py`, `eval_c1.py`, `eval_b7.py`(및 `eval_d1.py`의 리터럴 `14/24`)는 solver를 실행하지 않는 `simulate_cell` 모델이다(step 수 리터럴, `err = 0.42*rtol`). DESIGN.md X3가 요구한 `examples/k1_shared_basis_bench.rs`는 존재하지 않는다. 그런데 ledger L-0209(K1 "FAIL, JVP 6.2배 감소"), L-0214(C1 "PASS"), L-0210(B7), L-0213(D1)이 사전등록 실험 결과로 기록돼 있다. shared basis와 coloring의 성능은 **실측된 적이 없다**. (L-0211 E1R만 callgrind 실행.)
- **U11 holdout의 교란**: (a) G0 속도 판정이 6-thread arm 기준인데 n=3/8/64에서 6-thread는 1-thread보다 1.6–2.9배 느리다. (b) PPSW1 1-thread step당 wall이 RODAS5P의 8–24배(n=3,8), 2.1–2.5배(n=64)다. solve/RHS 횟수는 더 적으므로 구현 성숙도 차이다(미최적화 연구 코드 대 zero-alloc fast driver, 타이머에 Radau/Newton starter 포함). (c) 성장 상한 `ACCEPTED_STEP_SIGMA_MAX=1.10`(`ppsw1.rs:57`)은 논문값 1.5가 아닌 에이전트 제안 규칙(OD-15/25)이고, 6-step hysteresis와 합쳐져 tol에 둔감한 step 수(F2)를 만든다. "step 1.9–7.1배"에는 방법 고유 한계와 controller 선택이 섞여 있다. HOLD 판정은 유효하지만 대규모 영역의 방법 수준 결론은 아니다. U11이 "미설치"라 한 SUNDIALS는 시스템에 설치돼 있다(`libsundials_cvode.so.6`).
- **영역 불일치**: 프로젝트 전 기간의 후보 평가가 n≤64(가끔 1D n≤400) 고전 문제에서 이뤄졌다. 이 영역은 step당 1–30µs, 사용자 RHS 비중 1–11%라 병렬 stage·matrix-free·LU 재사용이 구조적으로 이길 수 없다. 대규모·고비용 RHS·2D/3D 영역의 matched-error 평가는 사실상 없다.
- **판정 기준 혼재**: G0(새 fast lane이 RODAS5P를 1.15배 이김 + depth≤6 + J/Newton 0)는 연구 게이트인데 패키지 릴리스 준비도와 하나의 HOLD로 묶여 있었다.
- 계보 문서 오류: L10의 PR135–147 오귀속, H14/H15가 head에 없는 파일을 가리킴, DICTIONARY의 NOT_PORTED 의미 혼동, `dense_output_v2.rs.rej`가 추적됨.

### 1.3 facade 현실 (`074a5699`)

- frontend가 10개 이상 병존한다(`Rodas5p`, `Dopri5`, `GeneralOde`, `AutomaticOde`, `SwitchingSolver`, `ProductionSolver`, `PeerOde`, `LawsonDopri5`, `ExplicitSession`, `Dopri5Ensemble`). 옵션·결과 타입이 제각각이다.
- `MethodRegistry`(U06)는 6개 method를 기술하지만 **라우팅에 쓰이지 않는다**. `GeneralOde::select`(`selection.rs:176`)가 DOPRI5/RODAS5P를 하드코딩하고 기본 정책은 DOPRI5다.
- 구현됐지만 facade에서 도달 불가: BDF1/2, Radau IIA(2-stage 3차), PEXPRB54s4, lagged RODAS5P, small/batch RODAS5P, SABR, homotopy, transactional q1/q2, PTSW2B, PPSW1 matrix-free, `vigilode-coupled` 전체(trace quotient, cyclic closure, rhs homotopy, SwitchState). `Tier::Performance`는 Protected와 같은 경로다.
- facade의 RODAS5P 경로는 전부 순차(병렬·SIMD 없음)이고 phase별 타이머가 없다.
- switcher의 `unified/router.rs`는 Tier2/3/4/Protected가 모두 같은 `Rodas5p::solve`를 호출한다(라벨만 다름). switching 실패는 항상 `MaxStepsReached`로 매핑된다.
- 3D·구면 연산자는 test 파일에만 있다. `vigilode-bench`·`vigilode-hybrid`는 workspace 멤버가 아니다.

### 1.4 8단계 병렬화: 기존 결과와 새 사실

- 기존(전부 음성): R6 동시 sweep(8회 후 정확, q2 부정확), R7 transactional q2 4.75배 느림, C5 graph-Schur 2.24배 느림, 대각 SDC 8–13 sweep, PPSW1 matrix-free W crossover 없음(n≤400), lagged LU kill gate(HIRES 실패, Brusselator 통과), lane batch 0.93배.
- 시도된 적 없음: 다음 step Jacobian/LU의 투기적 pipeline, 병렬 Jacobian 조립 실측, 공간 영역분할로 stage 사슬 전체를 국소 실행, LU 지배 영역의 lagged 재평가, 영역별 대규모 battery, 축소 depth ROW tableau.
- tableau(`fixtures/rodas5p_coefficients_snapshot.json`)에서 정확 유리수로 계산한 사실:
  1. α(RHS 인자 결합)는 완전 하삼각이다. black-box RHS 평가 depth는 정확히 8.
  2. 설계상 β₂₁=0(표현 소수에서 6e-17)이라 N=β−γI의 멱영 지수는 7이다. 안정성 함수 극 차수 7, 선형 문제에서 R(z)=e^z+O(z⁷).
  3. ‖(N/γ)^m‖∞ = 37, 24, 23, 20, 6.4, 2.3, 0 (m=1..7). 절단 sweep 방식은 stiff 성분 오차를 20–40배 증폭한다. R6/R7 실패의 구조적 이유다.
  4. Σbᵢγᵢ=0(−3e-16)이라 Rodas5P는 W-차수 2 조건을 만족한다(3차 W-tree 결함 ≈0.03–0.04이므로 정확히 2). embedded 가중치는 만족하지 않는다(Σb̂ᵢγᵢ=−0.0256). Jacobian 오차 E는 해에 O(h³E), 오차 추정에 O(h²E)로 들어간다.
  5. g₇(stage 6 직후)은 u_{n+1}의 3차 근사, g₈(stage 7 직후)은 4차 embedded 해다. 다음 step의 J를 g₇/g₈에서 미리 계산해도 국소 섭동은 O(h⁷)/O(h⁸)로 LTE O(h⁶)보다 작다.
- 영역별 비용 구조(모델, V04에서 실측): 8단계 사슬이 병목인 곳은 소형 dense(µs 단위, thread 무의미), 1D banded, matrix-free(8회 Krylov solve)다. dense 중대형과 2D/3D 직접법은 **LU가 병목**이고, FD Jacobian이면 n회 RHS가 step 비용을 지배한다.
- 계획 단계 toy(메모리 내 실행, 증거 아님): 1D Brusselator N=400을 4개 부영역으로 나눠 부영역마다 8단계 사슬 전체를 국소 실행하고 halo를 이전 sweep에서 가져오면, 겹침 4ℓ(ℓ=√(γhD)/dx)에서 sweep당 오차가 10³–10⁴배 줄었다. 0 초기 추정에서 2–4 sweep로 embedded 오차 이하.

### 1.5 잠정 판정

- **G0: HOLD 유지**. PPSW1 경로는 실패, 다른 후보는 미측정.
- **릴리스 준비도(R-gate, 신설): 미충족**. 단일 진입점 없음, 계보 분기, 합성 증거 행, 현재 head의 CI·독립 재현 없음, 대규모·기하 영역 증거 없음. 보호 RODAS5P 자체는 소형 문제에서 Julia Rodas5P와 step 수가 같고(Brusselator 228=228, Robertson 164 vs 162) Hairer RODAS와 step당 비용이 비슷하다(switcher 벤치 수치, 재실행 필요).

## 2. 소유자 결정 (2026-11-16 질문 응답)

1. switcher 계보: **시험 병합 후 조건부 채택**. 1.94.1 빌드·전체 테스트와 보호 RODAS5P 경로 bit-동일성을 통과하면 채택, 실패하면 가산 모듈만 이식. 합성 증거 행에는 INVALID 대체 행 추가(기존 행 보존).
2. PPSW1: **결정 실험 1회 후 판정**.
3. 자동 라우팅: **opt-in Auto만**. 기본 라우팅과 보호 RODAS5P 불변.
4. 판정 기준: **G0 유지 + 릴리스 게이트(R-gate) 신설**.

## 3. 실행 계획

### 3.0 작업 기반

- 새 worktree `~/worktrees/vigilode-prod-loop-20261116`, 브랜치 `research/production-loop-20261116`(부모 `074a5699`). 시험 병합은 `…-merge-trial` 브랜치에서 한다.
- toolchain 1.94.1 고정. `CARGO_TARGET_DIR`은 `/mnt/extdrive/vigilode-loop-20261116/target` 공유, 동시 cargo 빌드 ≤2(RAM 가용 ~11GB, swap 35GB 사용 중).
- 증거 루트 `research/production_loop_20261116/`. 프로그램 문서(`docs/agent/production_solver_program_20261112/`)의 `RESEARCH_DAG.json`·`TASK_CONTRACTS.json`에 V 노드를 추가하고 `MUTATION_GATES.json` allowlist를 갱신한다.
- 모든 실험은 실행 전 사전등록을 commit+push한다. raw 로그·명령·exit code·SHA와 음성 결과를 보존한다. 평가 스크립트는 solver 바이너리를 실제 실행하고 raw 출력에서만 수치를 읽는다(리터럴 금지).
- 측정: 결정적 counter와 `perf stat instructions:u`가 1차 지표. wall은 core pinning, arm 교차, 2 warmup + 7 반복 median. 호스트 부하가 높으면 `diagnostic`으로 표기한다.
- wave마다 commit 후 단독 `git push`(연구 브랜치만)와 원격 readback. 이 계획 승인이 해당 push의 승인이다.
- 사용자 미추적 파일(메인 체크아웃의 PDF·zip 등)과 다른 세션의 worktree(REI, cloud-peer)는 건드리지 않는다.

### 3.1 노드

| ID | 내용 | 판별 기준(사전등록) |
|---|---|---|
| **V01 감사** | 1절을 적대적 검증으로 확정·확장. 증거 진위(U01–U11 선별 재실행, switcher X 노드 스크립트 전수), holdout 통계 재계산, facade·registry·PeerOde·ppsw1과 switcher `unified/` 코드 리뷰, silent-failure 탐색, 계보 문서 교정, **R-gate 매트릭스**(정확성·견고성·API·재현성·성능·패키징·증거 거버넌스·기하 범위) 정의와 축별 심사. 산출: `AUDIT_KO.md`, `audit_findings.json`, `R_GATE_MATRIX.json` | finding마다 재현 근거·영향·최소 교정. 반박 검증을 통과한 것만 등재 |
| **V02 계보 병합** | trial 브랜치에서 `93392249` 병합(`612063db` 고유 7커밋 검토) → 1.94.1 빌드, 전체 테스트, clippy, **보호 RODAS5P 경로 bit-동일성**(DenseFast/BandedFast/FastDense/MatrixFree/SequentialDense × 코퍼스 × 3 tol의 최종 상태·counter 비트 비교; `research/adapter_switcher_economy_20261026/tools/run_campaign.sh` 재사용) | 전부 통과하면 채택. bit 차이는 원인 hunk를 제외하거나 가산 모듈 이식으로 후퇴. 합성 4행(L-0209/0210/0213/0214)은 DESIGN.md 13.2 스크립트로 INVALID 대체 행 추가. 확인된 사전 사실: 공개 이름 충돌 0건, manifest는 1.94.1 쪽 유지, 의미 충돌 1건(switcher가 추가한 `Jacobian::SparseJvp` 때문에 `selection.rs:306` match가 비망라) |
| **V03 단일 frontend** | registry를 유일한 라우팅 권위로 만든다. 단일 진입점 `Method::{Auto(policy), Rodas5p, Dopri5, Bdf, Radau, PeerW, Exponential, Lawson}` + 공통 solution view. 기존 frontend와 테스트는 호환 계층으로 유지. `GeneralOde::select`를 registry로 재표현. 계층 분담: 승인(admission)은 `MethodRegistry`(074a5699), 실행은 switcher `registry.rs`의 `WindowDriver` trait(`Dopri5/Rodas5p/Pexprb WindowDriver`)를 재사용하고 switcher `unified/router.rs`의 라벨뿐인 tier는 실제 dispatch로 바꾸거나 제거. adapter(U07 `PeerOde` 패턴): BDF, Radau, pexprb, lagged RODAS5P, small-batch ensemble, PPSW1-MF. opt-in `Auto`와 typed fallback(작업 과금, 접두 보존, capability 제약) | 기존 vigilode 테스트 무변경 통과, 1024 조합 선택표 동일, 기본 경로 golden 동일, callback 전 typed 거절 |
| **V04 stage 비용 해부** | 보호 소스를 고치지 않고 phase별 비용(J, LU, 8 RHS, 8 solve, 벡터 연산, controller/출력)을 분해. 격자: dense n∈{8,32,128,512,2048}, banded 1D n∈{1e3,1e4,1e5}, 2D/3D MOL, RHS 비용 배율 {1,10,100}×, Jacobian{해석,FD,JVP} | 계측 kernel이 보호 driver와 궤적 bit-동일. lever별 Amdahl 상한표가 V07–V09·V14 착수 여부를 정한다 |
| **V05 이론 노트** | 1.4의 사실을 정확 유리수 스크립트와 독립 재계산으로 검증: depth-8 정리(black-box 모델), 멱영 지수 7, sweep 증폭 상계, W-차수 2와 추정기 민감도, g₇/g₈ 차수, 영역·Amdahl 모델, Schwarz 국소성 상계(V14), 2-processor depth 5–6 5차 ROW tableau의 조건 수 대 자유도(go/no-go). 1차 문헌 대조 | 독립 구현 2개 일치, 문헌 식 대조, 미확인은 UNVERIFIED |
| **V06 shared Krylov basis v2** | (방법 보존) switcher `shared_basis.rs`와 MF RODAS5P hook을 이식해 **실측**. 8 stage가 같은 W를 쓰므로 step당 QR 하나를 증분 갱신해 공유하고 동일 preconditioner를 지원 | PASS: matched error에서 JVP ≤0.5×, instructions ≤0.8×(같은 preconditioner의 per-stage GMRES 대비)가 4 family 중 3. KILL: 어느 family도 instructions 미개선 또는 원 잔차 검사 실패 |
| **V07 투기적 Jacobian pipeline** | (W-섭동 한도 내) 새 opt-in driver. 다음 step의 J를 g₇/g₈에서 worker thread로 미리 계산하고, 지연 step-size 규칙으로 LU도 선행. 보호 파일 무수정 | 착수 조건: V04에서 (J+LU)/stage사슬 ∈[0.25,4]이고 task ≥20µs인 영역 존재. PASS: 관측 차수 ≥4.8, matched-error step ≤1.05×, wall ≥1.15×. KILL: step >1.2× 또는 wall <1.05× |
| **V08 병렬·색칠 Jacobian 조립** | (정확) FD/JVP 열의 thread 병렬, Curtis–Powell–Reid coloring 이식, dense LU 멀티스레드 opt-in | J와 궤적 bit-동일. PASS: FD Jacobian 문제(n≥64)에서 wall ≥1.5×. KILL: <1.1× |
| **V09 LU 지배 영역 lagged 재평가** | 기존 `rodas5p_lagged.rs`(정확 J + 지연 LU 전처리)를 dense n∈{256,512,1024}와 2D MOL에서 평가 | PASS: matched-error wall ≥1.15×. KILL: 전 영역 <1.0× |
| **V10 PPSW1 결정 실험** | σ∈{1.2,1.3,1.5} 안정성(독립 oracle), 동등 구현 수준의 step당 비용, 새 문제(소진된 holdout 3종 제외)의 matched-error step 비, REI 46k 결과 참조. 기본 상수 1.10은 그대로 두고 실험 arm으로만 | σ>1.1이 불안정하면 동결. 안정이면 step 비 ≤1.5이고 사전등록 영역 1곳 이상에서 측정 wall ≥1.15×일 때만 계속 |
| **V11 기하(2D/3D/구면)** | test 전용 3D 주기 Laplacian·위경도 FV 구면 연산자와 switcher `pde_models/*`를 공개 MOL 모듈로 승격(JVP·band·sparsity 선언, 보존·가중 norm 계약, 극 고리 1차 문제 명시). 영역별 battery v4(2D N=128–256, 3D N=32–64, 구면)를 사전등록·실행하고 CVODE/DVODE/RADAU5/RODAS/Julia와 비교 | 공간 2차 수렴, 보존 ≤1e-12, 참조 불확실성을 포함한 matched-error 표 |
| **V12 미이식 수학 이식 계획** | 계보 조사의 20항목(hybrid automaton, nonnormal observers, DOPRI5 secant stiffness, secant preconditioner, Chebyshev/Leja, preconditioned fixed-point homotopy, N02/N03, affine event certificate, Blelloch scan, incidence/chain complex 등)을 계약표로: source SHA, 대상 모듈, 불변량, 판별 테스트, 우선순위. V03/V06/V11이 필요로 하는 것만 이번에 구현 | 항목마다 실행 가능한 최소 판별 실험 |
| **V14 Schwarz 블록 Rosenbrock step** | (2D/3D/구면용 신규) stage–공간 루프 교환: 부영역마다 8단계 사슬 전체를 국소 LU로 실행하고 halo stage 값은 이전 sweep·예측자에서 가져온다. 원 stage 잔차가 통과할 때까지 sweep. step당 동기화 1회, 부영역 데이터가 8 stage 동안 캐시에 머문다. Python 판별 실험(2D heat/Brusselator/Gray–Scott) 후 통과 시 native 프로토타입 | PASS: 겹침 ≤4ℓ에서 median sweep ≤2(halo 예측자 포함)이고 endpoint 오차가 정확 solve와 구분 불가. KILL: sweep >4 |
| **V13 closeout** | wave마다 DAG·계약·CHECKPOINT 갱신, 실측 ledger 행, `REPORT_KO.md`, push와 readback, 가능하면 `production.yml` workflow_dispatch로 독립 CI. 최종 R-gate·G0 판정과 다음 루프 계획 | `validate_program.py`, `check_checkpoint.py` 통과 |

### 3.2 wave 일정

Workflow 도구로 실행한다. wave당 동시 에이전트 ≤7, 쓰기 범위 분리, 핵심 검증은 주 세션이 직접 한다. 각 wave가 commit+push된 체크포인트라 한도에 걸려도 다음 세션이 이어받는다.

- **W0 (단독)**: worktree·브랜치 생성, baseline 빌드와 테스트 receipt, V 노드 사전등록, push.
- **W1**: V01 감사(차원별 리뷰어 → finding별 반박 검증 → 종합) ∥ V05 이론 ∥ V04 계측 설계 ∥ V14 Python 판별 실험.
- **W2**: V02 시험 병합(직렬) → 채택 판정 → V03 ∥ V08 ∥ V06 이식.
- **W3**: V04 실측 → 게이트 → V06·V07·V08·V09 실험 → 독립 리뷰 → 판정.
- **W4**: V10 ∥ V11 ∥ V12 ∥ (V14 통과 시 native).
- **W5**: V13 최종 판정, 보고, 다음 계획.

에이전트 프롬프트에 넣을 주의: 주입되는 CLAUDE.md의 switcher gate 작업은 모두 완료 상태이며 이 루프는 worktree의 프로그램 문서가 규율한다. GateGuard hook은 첫 Bash와 새 파일 생성 때 사실 제시를 요구한다.

계획 단계에서 직접 확인한 실행 가능성: callback은 모두 `Send + Sync`(worker thread 호출 가능), `rodas5p_fast.rs`의 `Workspace::{new,choose_lu,factor,solve}`는 `pub(crate)`이고 `attempt`와 필드는 private(새 driver는 자체 stage loop를 갖고 LU kernel만 재사용), dense LU는 faer 0.24.4(현재 `Par::Seq`), SUNDIALS·Julia·valgrind·perf 설치됨. 파일 수준 설계 검증(읽기 전용 에이전트)은 진행 중이며, 보고가 오면 W0에서 노드별 allowlist에 반영한다.

## 4. 재사용할 기존 자산

- 라우팅·어댑터: `crates/vigilode/src/selection.rs`(`MethodRegistry`, `CapabilitySet`, `PeerOde`), `capabilities.rs`(`FacadeRoute`), `solve.rs`, `production.rs`, `switching.rs`.
- kernel: `rodas5p_fast.rs`(attempt :1118, stage loop :1160–1195), `rodas5p_matrix_free_fast.rs`(:2672, :2802), `rodas5p_lagged.rs`, `rodas5p_fast_small_batch.rs`, `bdf.rs`, `radau.rs`, `exponential.rs`, `adaptive.rs:729`, `parallel.rs`.
- 선형대수: `rodas5p-krylov`(GMRES/LGMRES/GCRO-DR/block GMRES), core `operator.rs`(`ShiftedOperator`, preconditioner), `sparse_direct.rs`, `low_rank_bordered.rs`.
- switcher에서 이식: `shared_basis.rs`, `coloring.rs`, `dopri5_stiffness.rs`, `nonnormal_observers.rs`, `hybrid_automaton.rs`, `vigilode-bench`(pde_models, cross-solver harness, `bench/protocols/*`).
- 프로토콜·검증: `MATCHED_ERROR_PROTOCOL.json`, `production_holdout/{reference.py,evaluate.py,runner}`, `peer_qualification/{oracle_coefficients.py,theta_oracle.py}`, `validate_program.py`, `check_checkpoint.py`, ledger 행 스크립트(switcher DESIGN.md 13.2).
- 기하: `crates/vigilode/tests/geometry_mol_contracts.rs`, `gray_scott.rs`, `conservative_remap.rs`.

## 5. 검증

- 코드: `cargo +1.94.1 fmt --check`, `clippy --all-targets -D warnings`(기존 lint 8건은 별도 표기), 대상 테스트 후 `cargo test --locked --no-fail-fast`(vigilode, integrators, krylov, core).
- 수치: 독립 참조(SciPy Radau/DOP853, CVODE, Julia)와 참조 불확실성, 공통 출력 격자, matched achieved error. 차수, Prothero–Robinson 수렴, 보존량.
- 불변: 보호 경로 bit-동일성, `check_checkpoint.py`, `validate_program.py`, 엄격 JSON.
- 독립성: 구현 에이전트와 분리된 리뷰·재실행 에이전트, finding 반박 검증.
- 원격: wave마다 push 후 원격 HEAD·blob readback.

## 6. 승인 경계 (이번 루프에서 하지 않는 것)

main 병합, 릴리스·기본 method 승격, G0·허용오차·보호 RODAS5P controller 상수 변경, force push와 history rewrite, PPSW1 기본 σ cap 변경, 다른 세션 worktree 수정. 병합이 보호 경로 수치를 바꾸면 채택하지 않고 보고한다.
