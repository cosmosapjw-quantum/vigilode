# VigilODE 심사 루프 2 · 8단계 stage의 scalar 임계경로 · 2026-11-24

**판정: 패키지 전체 HOLD 유지.** 기본 경로는 바꾸지 않았다. 이번 루프의 결과는 세 가지다.
(1) RODAS5P 8단계의 병렬화가 지금까지 실패한 이유를 scalar 수준의 의존 그래프로 다시 정리했고,
stage 식·tableau·제어기를 그대로 둔 채 임계경로를 `16n`에서 `8n`으로 줄이는 two-way stage solver를
구현했다. (2) 흩어진 stage solver 경로를 `VigilODE::solve_stages` 하나로 묶기 시작했다.
(3) 통합 패키지 작업을 위한 기계 판독용 인벤토리·아이디어 행렬·후속 DAG를 만들었다.
성능 주장은 채택하지 않았다. 측정 호스트가 2 vCPU 공유 환경이었다.

## 1. 기준점과 감사

- 작업 기준은 최신 연구 브랜치 `research/unified-solver-referee-20261124`의 `abf29825`다.
  사용자가 준 `a8d196b4`는 이 브랜치의 35번째 아래 커밋(V27 설계 기록)이다. main(`8d0c7918`)은 8월 상태다.
- 툴체인: 고정된 1.94.1을 받을 수 없어(프록시가 rustup 채널을 거부) CLAUDE.md 지침대로
  `RUSTUP_TOOLCHAIN=stable`(1.97.0)을 썼다.
- 기준 재현: 이전 계획의 진입 테스트 11개 binary 전부 통과, facade 전체(structured) 571 통과/0 실패/1 무시.
- 이전 실행의 폐형식 주장 5개를 독립 스크립트로 재검산해 모두 통과했다
  (`results/prior_claims.json`): 6개의 서로 다른 stage clock, 비정규 W 예제의 증폭 250,
  `u'=u²` backward Euler의 fold `(2, 1/2)`와 τ=1 실근 부재, S² colatitude 블록의 최대 고유값
  `2κ cot(Δθ/2)/(R²Δθ)`(교대 벡터가 정확한 고유벡터이고 Gershgorin 한계와 일치), 제조해.
- 결함 탐색:
  - `SplitForcing`이 정의역 밖 trial 상태(NaN)를 일반 경로보다 엄격하게 처리하는지 확인했다.
    결함 없음: 두 경로가 같은 시점에 같은 이유로 멈추고, `Recoverable`로 보고하면 둘 다 시도를
    거절하고 같은 비트로 완주한다. 이 동작을 계약 테스트 3개로 고정했다
    (`crates/vigilode/tests/split_forcing_nonfinite_recovery.rs`).
  - 기존 코드에서 `clippy -D warnings`가 `ppsw1_independent_qualification.rs:805`의 불필요한
    `u64` cast로 실패했다(기준 worktree에서도 재현). 한 줄 수정했다.
  - 같은 `target` 디렉터리를 다른 소스 리비전과 공유하면 오래된 artifact가 섞여 import 오류가
    난다는 것을 이번에 재현했다. 후속 계획의 규칙에 넣었다.
- 나머지 workspace crate 전체 테스트는 환경 재시작으로 중단되어 **미실행**이다(SL-N1로 이관).

## 2. 무엇이 8단계 비용을 지배하는가

V04 비용 해부 59개 셀을 다시 분류했다(`results/regime_map.json`).

| 구조 | 지배 항목 |
|---|---|
| 1D banded(값싼 RHS) | stage solve 0.61–0.70. RHS가 10–100배 비쌀 때만 RHS |
| 2D·3D banded | factorization 0.72–0.94 (12/12) |
| matrix-free | Krylov stage solve ≥ 0.86 (10/10) |
| dense | FD Jacobian 10, factorization 7, RHS 6, solve 4 |

"8번의 순차 RHS 호출"이 비용의 중심인 경우는 RHS가 비싼 경우뿐이다. banded 문제에서 8단계의 비용은
16번의 삼각 sweep(1D)이나 그 앞의 factorization(2D·3D)이다. 따라서 병렬화 수단도 regime마다 다르다.

## 3. 8단계 병렬화 연구

**SL-1 (stage–row lattice 깊이) [exact].** V05가 증명한 벡터 수준 깊이 8은 그대로 성립한다.
이번에는 한 시도를 (stage, row) 격자의 scalar DAG로 풀었다. RHS가 행 국소적(stencil 반폭 w)이고
W가 band (l, u)일 때 최장 경로는 다음과 같다(5개 band 모양 × 2개 비용 모형 × 3개 크기에서 정확한 일차식,
`results/stage_lattice_dag.json`).

| 일정 | 깊이 |
|---|---|
| 보호 kernel(모든 stage 같은 방향) | `16n` |
| stage마다 LU/UL 방향 교대 | `9n + c` (stage i+1의 첫 sweep이 stage i의 둘째 sweep을 뒤따름) |
| two-way 소거(양 끝에서 가운데로) | `8n + c` |
| SPIKE-P | `16n/P + c` |

결론: banded 시도의 scalar 임계경로는 RHS 8번이 아니라 삼각 sweep 16번이다. 이 부분은 stage 식을
바꾸지 않고 줄일 수 있다. 이전 시도(R6, R7, SDC, V14, V16, PPSW1)는 stage 식을 바꿨고, V07/V06/V08은
다른 항목을 겨냥했다. stage solve 안을 겨냥한 유일한 시도인 V19 분할(C2/C4/C8)은 블록 안으로 pivoting을
제한해 S4에서 `err:block_pivot`으로 무효가 되었다.

**SL-2 (two-way = 재배열 행렬의 partial pivoting) [exact].** 양 끝 front의 pivot 창이 분할점을 넘지
않으므로 two-way 소거는 `Q W Q^T`의 partial pivoting과 같은 pivot 열을 낸다. 제한된 pivoting이 없다.
30개 seed 사례에서 pivot 열 30/30 일치, 최악 후방오차 2.2e-16(LAPACK gbsv 1.7e-16)
(`results/twoway_gepp.json`). 양 끝 소거 자체는 고전 기법(two-way/twisted/BABE)이다. 이번 기여는
이를 변경 없는 8단계 Rosenbrock 시도의 stage solver로 배치하고 그 근거를 격자 깊이로 정량화한 것이다.

**구현.** `BandedTwoWayStageLu`(`rodas5p-integrators/src/stage_clients/banded_two_way.rs`)를
`StageLinearSystem` client로 넣었다. driver·tableau·제어기·출력 규칙은 보호 driver 그대로다.
두 front는 `rayon::join`으로 동시 실행되며 직렬 실행과 같은 비트를 낸다. 작은 계는 같은 비트의 tight
kernel로 넘긴다. 생성 후 할당이 없다.

**검증.** 단위 테스트 30 사례 후방오차 ≤ 2e-15, 직렬/병렬 비트 동일, 세 위치(위 front, 아래 front,
가운데 블록)의 특이 열에서 typed `LinearSolve`. 전체 IVP: 시도 수 동일, 종점 차이 1.1e-9–7.2e-8 허용오차
단위.

**측정 [진단용].** 2 vCPU 공유 호스트, 3회, 사전등록 없음(`results/two_way_timing_summary.json`).
stage pattern(분해 1 + solve 8)에서 2 thread two-way는 보호 kernel 대비 1D n=2·10⁵에서 0.54–0.61,
2D 64×64에서 0.24–0.33이었다. 같은 비트의 tight kernel(0.71–0.77, 0.35–0.36)보다 1.2–1.45배 빠르다.
전체 solve(n ≤ 2·10⁴)에서는 tight보다 낫다는 결과가 나오지 않았다. 직렬 two-way kernel에 아직 고정
offset 루프가 없고, solve마다 pool을 만들고, 호스트가 공유되었기 때문으로 본다. 성능 판정은 owner
호스트(12 core)에서 사전등록 프로토콜로 할 일이다(SL-N2).

## 4. 아이디어별·solver별 정리

`IDEA_SOLVER_MATRIX.json`에 10개 아이디어(matrix-free는 Jacobian-free 행에 포함) × 11개 solver 가족의 상태(default / opt-in / crate-only /
separate-crate / research-record / design / negative)와 위치, 다음 판별 실험을 기록했다. 요점:

- Newton-free와 derivative-free는 RODAS5P의 기본 경로에서 이미 살아 있다(선형 암시, FD Jacobian).
- Jacobian-free(JVP+GMRES)는 기본 matrix-free 경로, P1/P2는 opt-in이다. BDF/Radau의 Newton–Krylov는 설계 단계다.
- causality는 이번 결과로 새 해석을 얻었다. (stage, row) 격자에서의 인과 원뿔과 방향 교대 wavefront(SL-N3)다.
  transactional 경로는 음성 판정을 유지한다.
- homotopy root finder는 RODAS5P의 선형 stage에서는 인증용으로만 의미가 있다(R5 성능 음성).
  실제로 연속 변형이 필요한 곳은 BDF/Radau의 비선형 잔차다(이전 계획 implicit-roots).
- 8단계 병렬화: banded는 two-way(구현), wavefront(설계), SPIKE-P+fallback(설계). dense는 factorization
  병렬화(V18 미실행). matrix-free는 Krylov 내부 병렬만 가능하다. 구면은 경도 DFT + 모드별 삼중대각으로
  정확·모드 병렬 stage solver가 된다(설계; FFT 의존성이 없어 owner 결정 필요).

## 5. 단일 frontend와 패키지 통합

- `VigilODE::solve_stages(problem, span, y0, options, StagePolicy)`: `Shipped`(= `Rodas5p::solve`),
  `MatrixFree(MfStageSolvePolicy)`(= `solve_matrix_free`), `Banded(BandedStagePolicy)`(= 새
  `solve_banded`)를 한 진입점으로 묶었다. 정책 밖 요청은 callback 전에 typed refusal을 낸다
  (`Unsupported::BandedPolicyRequiresBandedRoute`). 후보 정책은 `tier_executed = Performance`,
  `DriverWork::BandedClient`에 client id와 worker 수를 남긴다.
- `BandedStagePolicy::Tight`: 지금까지 integrator crate 전용이던 V21 kernel을 facade에 노출했다.
  보호 driver와 상태·시도 수·callback 계수가 비트 동일함을 facade 테스트로 확인했다(해석/FD band 모두).
  기본 경로 승격은 owner 결정으로 남겼다(SL-N12).
- 인벤토리(`results/INTEGRATOR_INVENTORY.json`): integrator 쪽 crate 155,673줄 중 기본 경로 10,140줄,
  facade opt-in 30,919줄, crate 전용 82,811줄, 연구 gate 28,636줄, 음성 판정 3,167줄. 연구 gate를
  기본 빌드에서 빼는 작업(SL-N6)과 모든 진입점을 하나의 `SolveSpec`으로 덮는 작업(SL-N7)을 계획에 넣었다.

## 6. production 자격 심사

| 축 | 이번 근거 | 판정 |
|---|---|---|
| 정확성 | 새 경로의 후방오차·IVP 대조, 기본 경로 golden 11/11 불변, facade 577/0 | 새 opt-in 경로 범위에서 통과 |
| 강건성 | 특이 행렬 typed error, 정책 밖 요청의 callback 전 거부 | 범위 내 통과 |
| API | 단일 stage 진입점, non_exhaustive enum, 기존 진입점 불변 | 부분 |
| 성능 | 진단용 측정만 | 미판정 |
| 재현성 | 고정 툴체인 미사용, 다른 crate 전체 테스트 미실행, 독립 리뷰 미실행 | 미충족 |

## 7. 다음 단계와 재현

`NEXT_PLAN.json`(SL-N1–SL-N14, 이전 계획의 11개 과제 이관)과 `CODING_AGENT_KO.txt`를 보라.
우선순위는 SL-N1(owner 호스트·1.94.1 재현) → SL-N12(tight 기본 경로 승격 결정) → SL-N2(two-way
고정 offset kernel + 상주 pool + 사전등록 측정) → SL-N7(SolveSpec) → SL-N6(연구 gate 분리) → SL-N5(구면).

```
python3 -I research/stage_lattice_referee_20261124/verify.py
cargo test --locked -p vigilode --features structured --release --no-fail-fast
cargo test --locked -p rodas5p-integrators --release --lib --test stage_clients_banded_two_way
```
