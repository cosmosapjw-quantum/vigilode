# 기존 연구와 실제 적분 경로의 연결

## 조사 범위와 기준

이번 연결 작업의 기준 소스는 `8878e55a54e3a5f99fa81d6d3b08e05c94ed3db2`이다. 첨부한 두 아이디어 문서와 35쪽 통합 연구보고서의 본문을 읽고, 해당 기준의 Rust 구현과 원격 브랜치 계보를 대조했다. 문서에 기록된 과거 실행을 새 실행으로 계산하지 않았다. NCP를 사용하지 않는다.

가장 중요한 조사 결과는 필요한 coupled solver의 상당 부분이 이미 구현되어 있다는 것이다. `crates/vigilode-coupled`의 PR74·PR75 구현, `crates/vigilode-hybrid`의 PR77 구현은 위 기준에 포함되어 있다. 새로 만드는 대상은 동일 알고리즘의 복제본이 아니라, 기존 coupled 후보가 실제 적응형 시간 전진과 동일 raw-U fallback에 참여하는 연결이다.

| 원격 연구 가지 | 고정 tip | 기준 소스에 대한 관계 |
|---|---|---|
| `research/coupled-matrix-free-rust-20261020` | `15c43ce76734d0a560fcec5909ac49f1e6f2d347` | 조상: PR74 코드·증거가 이미 포함됨 |
| `research/coupled-economy-20261020` | `6c945b6c6e0e024d8f733c21da8d35fbcc3c5f2a` | 조상: PR75 residual-first가 이미 포함됨 |
| `research/hybrid-structure-ad-20261020` | `9bb05bed4cd7df720f2b429c2b47e97ed76712d5` | 조상: PR77 모델 전용 trajectory가 이미 포함됨 |
| `research/cumulative-evidence-20261020` | `ebb5b81ebe77314b6fee941096fd6cba6d8597fc` | 조상: 누적 보존 자료가 이미 포함됨 |
| `production/vigilode-plan-20261021` | `cc2e1a009ac6802ef01a289392e9715074a882ba` | 조상: 별도 본체 개발 가지도 포함됨 |
| `research/bassrei-port-20261021` | `555e71e03fe31e6c9b21961c8e2d263b1794eb6f` | 조상; 두 companion crate의 소스 차이 없음 |
| `research/orchestration-sandbox-20261022-vigilode` | `f9da4aae865623d71bff20c6157318cbaf19ba6c` | 계보는 분기되어 있으나 두 companion crate의 소스 차이 없음 |

이 관계는 해당 소스 경로와 계보에 관한 확인이다. 마지막 가지의 모든 파일이 같다는 뜻은 아니다. 기준 이후의 이번 작업 커밋을 과거 벤치마크를 생성한 소스로 바꾸어 표시하지 않는다.

## 이미 존재하는 기능과 재사용 지점

| 구현 위치 | 실제 API 또는 자료형 | 이번 작업에서의 역할 |
|---|---|---|
| `crates/vigilode-coupled/src/lib.rs` | `coupled_step_residual_first`, `Config`, `Attempt`, `Candidate`, `Work` | 공유 부분공간에서 전체 stage 후보를 구하는 기존 엔진 |
| `crates/vigilode-coupled/src/algebra.rs` | `Basis`, `small_solve` | 두 번 재직교화한 basis, 저장된 J₀V, 제한된 작은 dense 계산 |
| `crates/vigilode-coupled/src/solver.rs` | `homotopy`, `full_check`, `run` | λ=1 직접 시도, 실패 시 구간 분할, 원래 차원 잔차 검사, basis 확장 |
| `crates/vigilode-coupled/src/residual_first.rs` | `correct` | 잔차를 먼저 검사하고 실제 Newton 갱신이 필요할 때만 stage JVP 요청 |
| `crates/vigilode-coupled/src/fallback.rs` | `coupled_or_sequential`, `coupled_or_sequential_budgeted` | 과거 명시적 **native-K** fallback; 기존 동작을 보존할 대상 |
| `crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs` | `Rodas5pMfFastWorkspace::new`, `attempt`, `y_new`, `stage` | 새 동일 **raw-U** fallback에 사용할 실제 순차 경로 |
| `crates/rodas5p-integrators/src/adaptive.rs` | `AdaptiveStepConfig`, `AdaptiveControllerState`, `rodas_next_step_after_attempt` | 기존 RODAS5P controller 및 실패·거절 후 step 갱신 |
| `crates/rodas5p-integrators/src/output.rs` | `OutputCollector`, `adaptive_end_step`, `step_to` | 표현 가능한 시각, 출력 시각으로의 착지, 요청 출력 보존 |
| `crates/rodas5p-integrators/src/parallel.rs` | `ParallelExecution` | 애플리케이션이 수명 동안 재사용하는 제한된 local Rayon pool |
| `crates/rodas5p-integrators/src/problem.rs` | `OdeProblem::linearize_matrix_free`, `model_epoch`, `jvp_only_clone` | 명시적 Jacobian을 만들지 않는 callback과 모델 정체성 |
| `crates/rodas5p-integrators/src/causal_majorant.rs` | `upper_causal_solve`, `upper_path_sum_action`, `RadiusBox`, `MajorantEntries` | 유효한 inverse·미분 상계가 주어졌을 때 유한 하삼각 오차 전파 |
| `crates/rodas5p-integrators/src/raw_stage_target.rs` | `RawStageReceipt`, `raw_coefficient_digest`, `raw_stage_allowance` | raw/native target 구분과 계수 변환 오차 계약 |
| `crates/vigilode-hybrid/src/trajectory.rs` | `integrate`, `Integration`, `Trial` | 특정 생성형 모델의 연속 곡선·상속 반경·rollback 선행 구현 |
| `crates/vigilode-hybrid/src/model.rs` | `Model::admits_packet`, `packet`, `shift_factor` | 정확한 closure 및 band-plus-rank-one 구조에 한정된 후보 |

기존 coupled 작업에는 병렬 실패의 합류와 attempted/completed callback 계수, 부분적으로만 써진 JVP 출력의 NaN 감지, 모델 epoch 확인도 있다. 이를 삭제하거나 단순한 `Result` 조기 반환으로 바꾸면 실패 비용의 관측과 모델 정체성이 약해진다. `ParallelExecution::map_ordered` 자체만으로 모든 작업의 완료를 보장하는 것은 아니다. 기존 coupled 구현은 각 작업의 수치 결과를 값으로 반환한 뒤 전체 결과를 모으는 방식으로 이 요구를 충족한다.

기준 소스는 PR75 이후의 수정도 포함한다. stage 0에서는 현재 상태의 미분이 필요하지 않으므로 eager 경로가 생략하는 stage JVP의 반사실적 수는 `(s−1)m = 7m`이다. 첨부 보고서의 과거 `8m` 기술을 현재 counter 정의로 재사용하지 않는다. 영 허용오차 scale 처리와 root workspace 편입도 이미 수정되어 있다.

## raw-U와 native-K의 경계

기존 coupled 후보는 자율·항등질량 문제에서 아래 raw-U 방정식을 푼다.

\[
(I-h\gamma J_0)U_i
=h\gamma f\!\left(y_n+\sum_{j<i}A_{ij}U_j\right)
 +\gamma\sum_{j<i}C_{ij}U_j.
\]

출력은 `b_code`와 U의 선형결합이며, embedded 벡터는 마지막 raw stage이다. 반면 `sequential.rs::finish_step`은 native-K를 받아 `b`와 `btilde`로 출력과 추정오차를 만든다. raw-U를 그대로 `finish_step`에 넣는 것은 유효한 변환이 아니다. 두 표현은 수학적으로 연결되지만 binary64 계수 변환과 합산 순서까지 동일하지 않다.

그러므로 기존 `coupled_or_sequential*`를 조용히 다른 target으로 바꾸지 않는다. 새 opt-in 적분 경로에는 `Rodas5pMfFastWorkspace::attempt`를 사용하는 raw-U fallback을 별도로 연결한다. 해당 workspace는 `Direct`와 명시 행렬이 필요한 preconditioner, 항등질량 외 문제를 거부한다. fallback도 실제 원래 target을 푸는 유효 수치 경로이며 전역 오차 인증으로 격상하지 않는다.

## 이번 연결의 최소 완결 단위

첫째, 기존 `coupled_step_residual_first`를 호출하여 `Candidate`와 모든 시도 비용을 받는다. 입력·callback·모델 변화 오류와, 계산 예산·projection leakage·작은 계의 특이성·root 반복 실패를 구별한다. 후자의 허용된 수치 실패에만 raw-U fallback을 요청한다. 단순히 embedded 추정치가 1을 넘었다는 이유로 같은 큰 h의 후보를 승인하지 않는다.

둘째, 기존 출력 수집기와 controller를 사용하는 실제 적응형 loop에 연결한다. candidate 생성, 원래 공간 잔차 판단, embedded 물리 step 판단, 출력 보존이 모두 성공해야 accepted time/state가 전진한다. 실패한 candidate, 실패한 fallback, 거절된 물리 step의 작업량은 남긴다. 재활용 상태가 있다면 거절 전에 찍은 snapshot으로 되돌리고, 모델 epoch 변경은 해당 frozen data를 무효화한다.

셋째, companion에서 기존 출력 규칙을 사용할 수 있게 좁은 공개 façade를 제공한다. 기준의 `output` 모듈과 `OutputCollector`는 `pub(crate)`이므로 companion이 직접 접근할 수 없다. `rodas5p-integrators`가 coupled crate에 의존하게 만들면 순환 의존성이 생긴다. 공유 규칙에 얇은 interface를 두고 companion이 이를 소비하는 방향으로 연결한다. 독립적으로 출력 착지 규칙을 다시 작성하는 것은 피한다.

넷째, 선택적 causal bound를 별도 자료형으로 반환한다. 유효 domain/tube와 현재 W의 inverse bound 및 필요한 derivative bound가 사용자 모델에서 제공되어야 한다. `E ≤ a + LE`의 유한 전파는 raw residual의 출력 영향에 관한 조건부 결과이다. 그것만으로 초기값 오차, 시간 절단, dense-output, 공간 이산화, remap 오차가 인증되지는 않는다. 상계 없는 기본 후보의 정확도 권한은 기존 operational-residual 수준으로 유지한다.

다섯째, 작은 전체 stage 선형계의 반복 대각 block 구조를 실제 계산 절감에 사용한다. 현재 변경안의 `algebra.rs::small_solve`는 각 대각 block이 같은 Wₘ이고 상삼각 block이 영인지 검사한 뒤, Wₘ 한 번의 LU와 여덟 forward block solve를 수행한다. 작은 계의 분해 비용은 `O((sm)³)`에서 `O(m³ + s²m²)`로 줄어든다. `s=8`은 고정이며 전체 시간의 512배 개선을 뜻하지 않는다. 큰 `sm × sm` 행렬 저장과 원래 선형계 잔차 재검사는 남는다. full-LU와 block-LU는 부동소수점 pivot·합산 경로가 다르므로 이전 결과와의 bit identity를 보장하지 않는다.

새 연결의 실제 파일은 `crates/rodas5p-integrators/src/adaptive_output.rs`, `crates/vigilode-coupled/src/adaptive.rs`, `crates/vigilode-coupled/src/raw_step.rs`이다. 각각 `AdaptiveOutputSession`, `integrate_coupled_observed`, `RawStepSession`을 제공한다. 기존 공개 후보와 native-K fallback은 보존한다. 조건부 추정은 `RootErrorPolicy::ConditionalL2`이며 caller가 제공한 가정 아래의 binary64 계산이다. outward interval 인증이 아니고, 자료형도 이를 `conditional-f64-estimate-user-assumptions-not-certified`로 명시한다.

이 문서의 연결 설계는 source inspection 결과이며 시험 성공이나 성능 측정의 영수증이 아니다. 최종 구현 상태와 실행 결과는 같은 연구 디렉터리의 별도 검증 기록을 따른다. machine-readable 지도에서도 기존 구현과 이번 신규 연결 목표를 구별했다.

## 범위를 넓히기 전 확인할 사항

- 동일 raw target에서 고정 h 후보·순차 fallback의 출력과 embedded 벡터를 대조한다. 실패 시 비용도 함께 대조한다.
- 실제 trajectory에서 embedded 거절 뒤 retry, 출력시각 착지, 도중 callback 실패, 모델 변경, representation 한계가 accepted state를 잘못 전진시키지 않는지 확인한다.
- worker 수 변화의 수치 결과와 실제 callback 합류를 확인하되, concurrency 존재를 wall-time 가속으로 부르지 않는다.
- 상계가 제공된 경우에는 endpoint·embedded 각각의 오차 운반과 상계의 가정을 확인한다. 상계가 없으면 실험적 오차 비교로 보고한다.
- PR77의 구조화된 순차 stage 성능을 coupled homotopy의 성능으로 보고하지 않는다. packet·structured·coupled는 서로 다른 후보이다.
- 일반 callback이나 실제 `(E, θ)` FVM으로 넓힐 때의 limiter·measure·mesh·remap·공간오차 의무는 별도이다. 이번 fixed-dimensional 자율 ODE 연결이 이를 완료하지 않는다.

## 입력 정체성

| 첨부 | SHA-256 |
|---|---|
| `Pasted text(20261022-105556).txt` | `a517b6fc3f2729b359b879426d604fb303403265e2540ecfcdb0a3b9f2a3da7d` |
| `Pasted markdown (2)(10).md` | `c877a1bbd30439c4488373754ba1c189ea4a99c2e2b8544d37cf8703e93d74ad` |
| `VigilODE_Complete_Research_Report_20261022_KO.pdf` | `9543d9103966488319deb92862f49b63c248b3797f3616839806973fd94875e7` |

두 아이디어 문서의 원래 조사 기준은 `c03ca90f19a9e64ba7751c069325a48b54b39c2e`이다. 그때의 미구현 항목과 현재의 미구현 항목을 혼동하지 않는 것이 이번 intake의 핵심이다.
