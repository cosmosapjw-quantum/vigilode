# VigilODE: 남은 연구·개발 과제만을 다루는 정밀 리뷰

검토 기준: base `5a8d7fe9ffc681bca98a98a2f9889a2a05505783`, PR #70의 게시 코드 `77275e2b431e4280366060d44a883f1832909fe6`. 이 문서는 **아직 구현·실행·입증하지 않은 작업만** 제안한다. 수정 내역과 테스트 결과는 `EXECUTION_RECORD_KO.md`, 새로 점검한 수학은 `DERIVATIONS_KO.md`, 실제 실행 판정은 `FINAL_VERIFICATION.json`으로 분리한다. 완료된 L-0034..L-0043의 작업을 새 TODO로 재등록하지 않는다. 아래 식은 향후 구현/검증해야 할 계약의 명세이지 해당 기능이 존재한다는 뜻이 아니다.

## 1. 우선 결정: RVJ라는 새 방법 전체가 아니라 검증 가능한 구성요소를 이전한다

범용 raw-coordinate RVJ5를 기존 RODAS5P 대신 채택하는 작업은 승인하지 않는다. 다음 연구는 (i) 현재 연산자에 대한 inexact solve의 물리 출력 오차, (ii) signed Laguerre bound의 실제 admission, (iii) 모델별 chart의 coordinate-error provider 및 controller로 좁히는 것이 합리적이다. 새로운 scalar filter를 최적화하기 전에 numerical map의 섭동 증폭을 함께 평가해야 한다.

이 방향은 새로운 Rosenbrock tableau나 별도 collocation solver를 당장 추가하는 것과 다르다. 보호된 기본 R-JF 경로는 유지하고, 새로운 기능을 별도 식별자/opt-in interface로 넣어 원래 방법의 native regression과 대조해야 한다. 연구적 후보가 실패하면 알고리즘을 강제로 합치는 대신 그 조합만 중단한다.

## 2. R-NEXT-01: raw/K 비교를 실제 residual-to-output 계약으로 닫기

대상은 `raw_stage_target.rs`, `rodas5p_matrix_free_fast.rs`, `sequential.rs`와 L-0038의 새로운 후속 실험이다. 남은 문제는 raw-stage 변환식의 재유도가 아니라, 두 표기가 서로 다른 RHS를 사용하는 상황에서 stopping criterion과 물리 오차를 함께 다루는 것이다.

고정된 물리 scale S_y에서 다음 분해를 기준으로 acceptance를 설계해야 한다.

    E_compare <= E_native_coefficient + E_K_linear + E_U_linear
                 + E_projection_rounding + E_nonlinear_remainder.

각 항의 단위는 물리 상태변수 또는 명시적으로 동일한 WRMS여야 한다. absolute residual의 gamma scaling을 상대 residual tolerance에 그대로 적용하지 않는다. 작은 embedded 값 둘의 상대 차이를 무조건 오류로 판정하는 대신, bounded additive error와 그 estimator의 resolved/unresolved 상태를 함께 보고해야 한다. 반대로 기존 FAIL을 없애려고 사후적으로 tolerance만 넓혀서는 안 된다.

실험은 동일한 accepted mesh 또는 동일한 frozen state/h에서 direct reference, current-J Krylov, tightened current-J Krylov를 비교해야 한다. exact reference가 없으면 reference uncertainty를 명시적으로 포함한다. 지표는 최종 해, dense output, residual, coefficient allowance와 failed/rejected work를 분리한다. 기존 L-0038은 보존하고 새 사전등록 row가 자신의 명시적 판단 기준을 가져야 한다.

수락 기준은 원래 상대 판정의 모든 실패가 새로운 error budget 안에서 재현 가능하게 분류되는 것, unresolved solve가 silent acceptance를 만들지 않는 것, benign cases의 기존 상태·시간 정책을 보존하는 것이다. 새 bound가 사용 불가능하면 개선을 주장하지 않고 보호 경로를 유지한다.

## 3. R-NEXT-02: Krylov의 caller-owned 출력 및 bounded-capacity 저장

대상은 `rodas5p-krylov/src/{gmres,lgmres,gcrodr,workspace}.rs`와 MF driver이다. 단계 조립용 JVP를 줄이는 것과 solver 전체의 allocation/JVP가 줄어드는 것은 다르다. 반환 solution/report, recycle basis 갱신, least-squares workspace를 단계적으로 caller-owned storage로 바꾸는 경로를 시험해야 한다.

제안 API는 `solve_into(operator, preconditioner, rhs, output, workspace, telemetry)` 형태이며 기존 소유형 API를 wrapper로 남겨 호환성을 유지한다. `output`과 RHS의 alias 정책, workspace growth, recycled images의 operator epoch, 오류가 발생한 뒤 output validity를 명시해야 한다. 초기 warm-up allocation과 정상/실패/resize attempt의 allocation을 구분한다.

우선 GMRES 한 경로에서만 proof-of-concept를 실행한다. 동일 operator/RHS/초기값/tolerance에서 residual과 결과가 합의한 allowance 안에 있고, 저장 한도에 도달한 경우 typed failure 또는 명시적 growth가 발생해야 한다. 그 후 LGMRES와 GCRO-DR로 확대한다. 벡터당 작업, orthogonalization, reductions, memory traffic을 세고 allocation 감소만으로 속도 향상을 주장하지 않는다.

새로운 allocation 목표는 held-out 문제를 열기 전에 등록한다. 반복마다 solution clone이 남거나 work counter가 실패 경로를 누락하면 중단한다.

## 4. R-NEXT-03: GCRO-DR의 남은 수렴 실패를 recycle-subspace 문제로 분해

대상은 `gcrodr.rs`, `small.rs`, MF workspace 실험이다. 작은 generalized pencil의 scratch 문제를 다시 구현할 계획이 아니라, 그와 구별되는 tight residual/Arnoldi budget 실패를 분석해야 한다.

동일 frozen W와 RHS를 대상으로 cold GMRES, cold GCRO-DR, recycled GCRO-DR를 고정 budget으로 비교한다. 매 checkpoint에서 true residual과 projected residual, basis orthogonality, retained subspace rank, residual의 retained-space 성분을 함께 기록한다. 단순히 maxiter를 크게 하는 것만으로 원인을 판정하지 않는다. 나쁜 recycle subspace를 버리는 bounded fallback은 중단 전 작업까지 모두 계상해야 한다.

가능한 RVJ 전달 요소는 residual의 출력 방향 전달과 중요한 fast subspace의 추적이지, 과거 J의 operator action을 현재 J인 것처럼 사용하는 것이 아니다. 사전에 주어진 dissipativity/energy bound 또는 실제 inverse witness가 없는 비정규 문제에서는 Ritz eigenvalue만으로 residual-to-error gain을 인증하지 않는다.

수락 기준: 같은 입력에서 어느 설정이 실패를 유발하는지 재현하고, 수정된 reset/fallback 정책이 별도 미사용 synthetic cases에서도 false convergence를 만들지 않아야 한다. numerical rank나 weighted norm을 조절한 경우 그 비용과 물리 norm 환산을 보존한다.

## 5. R-NEXT-04: Laguerre의 diagnostic total을 admission 가능한 total로 연결

대상은 `laguerre_adjoint.rs`와 `polynomial_action.rs`이다. `laguerre_adjoint_total`이라는 diagnostic 합계 자체를 새로 만드는 것이 아니라, 그 합계가 truncation, stored coefficient, recurrence, summation, normalization을 동일한 exact target에 대해 모두 포함하는지 증명하고 소비 API를 연결하는 것이 남은 작업이다.

우선 현재 degree 한도와 verified symmetric nonpositive domain 안에서만 수행한다. 계수의 정확한 의미, exact transformed X, local residual의 인덱스, input/output normalization, fused columns의 cancellation을 같은 전제로 묶는다. Sufficient certificate는 다음 항들 중 하나라도 없으면 발급하지 않는다.

    E_total = E_transform + E_tail + E_coeff + E_recurrence
              + E_accumulation + E_normalization.

이미 다른 항에 포함된 변환 오차를 중복 계산하거나 누락하지 않도록 component registry를 먼저 고정한다. coefficient intervals와 stored coefficients의 역할을 구분하고, envelope cache는 parameter equality뿐 아니라 proof version과 resource limits를 포함해야 한다. 극단적인 subdivision depth에 대해서는 계산 예산을 명시하고 무제한 재귀를 public input으로 받지 않는 계약도 필요하다.

수락 기준은 non-scalar symmetric matrix, near-cancellation, subnormal/large-amplitude, scalar branch, invalid enclosure와 degree-limit cases의 독립 고정밀 enclosure 대조 및 total admission의 negative tests다. range가 커지거나 domain을 벗어나면 EstimateOnly/unsupported 상태를 보존한다. 모든 비정규 matrix를 self-adjoint theorem의 대상으로 넓히는 작업은 포함하지 않는다.

## 6. R-NEXT-05: chart의 coordinate-error 공급원과 물리 controller

대상은 선택적인 chart interface와 adaptive controller이며 기본 RODAS5P의 silent replacement가 아니다. 입력이 이미 정확한 coordinate-error bound라고 가정하는 함수와, 실제로 그러한 bound를 만들어내는 적분기를 구별해야 한다.

제안하는 첫 모델은 고정 positive kappa와 regular ratio chart다. slow coordinate에는 enclosure된 derivative/remainder 또는 독립 verified defect를 사용하고, fast coordinate에는 nonzero initial-layer amplitude와 rational-vs-exponential error를 남겨야 한다. well-prepared data의 성공만으로 임의 초기층까지 확대하지 않는다. local coordinate errors를 물리 absolute/WRMS scale로 운반한 뒤 step acceptance에 쓰고, stage-target distance 및 embedded proxy와 별도 필드를 사용한다.

근사 cofactor를 쓸 경우 `(r_C - w r_D)/D`를 실제 transformed ODE의 forcing으로 넣거나 그 누락 오차를 enclose한다. numerator residual이 작아도 D의 하한이 작으면 무효다. kappa가 불확실하거나 변하면 reciprocal의 오차와 시간 의존항을 추가해야 하며, 현재 고정-parameter 계약을 그대로 적용하지 않는다. moving frame에는 connection term을 반드시 포함한다.

수락 기준: whole-step tube가 chart에 남음을 보이고, nonzero fast mode를 보존하며, reconstruction rounding을 더한 물리 오차가 native independent reference와 맞아야 한다. endpoint, dense output, event evaluation, coordinate switch 모두 같은 branch/time/parameter identity를 사용한다. chart가 실패하면 그 chart만 거절하며 다른 chart의 존재 불가능성을 추론하지 않는다.

## 7. R-NEXT-06: homotopy의 순이익을 certificate의 전체 비용으로 평가

대상은 transactional q1/q2, radius/action, inverse witness 및 work telemetry이다. 반경이 닫혔다는 사실과 outer step이 허용된다는 사실, 그리고 빨라진다는 사실은 세 단계다.

certificate 비용에는 radius preflight, 실패한 radius, witness 생성·재사용, bound evaluation, allocations, dispatch, fallback을 모두 넣어야 한다. 보호된 sequential target과의 동등성/오차 예산을 먼저 확인한다. 독립 component 작업과 한 trajectory의 critical path를 구별하며, 8-stage batch의 이상적 wave 수를 CPU speedup이라고 표현하지 않는다.

다음 연구는 실제 q2 candidate를 사용한 counter-only cost 비교부터 시작하는 것이 낫다. serial bound와 action-first를 같은 후보, same target, same physical output budget에서 비교한다. 전체 비용을 포함한 여유가 없으면 병렬 engine을 더 정교하게 만드는 대신 해당 regime의 router를 abstain으로 둔다. timing authority HOLD가 해제되기 전에는 새로운 wall-time promotion을 하지 않는다.

## 8. R-NEXT-07: 내부 가변 callback 데이터의 explicit epoch 계약

`Arc` allocation identity는 외부 배열/parameter를 interior mutability로 바꾼 경우를 검출하지 못한다. 이것을 임의의 callback 내용을 검사해서 해결할 수는 없다. 향후 ODE client API에 monotone model-generation/linearization epoch 또는 immutable snapshot capability를 선택적으로 제공하는 방안을 설계해야 한다.

같은 t,y와 callback pointer라도 epoch가 변하면 f0, f_t, JVP와 recycle images가 함께 무효화되어야 한다. epoch의 출처는 client가 책임지며 hash만으로 semantic identity를 입증했다고 하지 않는다. epoch가 없는 client의 manual fresh 계약은 보존한다. 이 작업은 API 설계이며 현재 문서화된 계약을 지키는 client에 대한 기존 정확성 결함으로 분류하지 않는다.

## 9. R-NEXT-08: generic no-chart RVJ에는 uniform perturbation stability가 필요

새 rational filter의 Taylor 차수와 scalar L-stability만 확인하는 연구는 충분하지 않다. 한 step map Psi_h의 작은 transverse perturbation이 stiffness-independent 상수로 전달되는지, 필요하면 적합한 energy norm과 physical norm 사이의 uniform equivalence까지 함께 분석해야 한다.

다음 비교군은 derivative-light correction/peer 또는 문헌상의 stiff-order conditions를 실제 후보 class에 맞게 유도하는 경로다. Roberts–Shirokoff–Biswas–Seibold의 arXiv:2505.15099v4는 constant stiff linear part를 갖는 semilinear RK 분석에 대한 원전이다. 그 RK 정리가 RVJ 또는 좌표변환된 Rosenbrock에 자동으로 적용되지는 않는다. 새로운 tableau/iteration의 단계 의존성, stage order, internal amplification과 nested solve 비용을 함께 비교해야 한다.

원래의 method-labelled 반례를 유지한 상태로 최소 두 개의 fast mode와 비정규 coupling에 대한 새로운 대조실험을 설계한다. counterexample를 피하려고 좌표나 초기조건을 바꿨으면 그것은 다른 map/domain임을 명시한다. 일반 정리를 닫지 못하면 범위를 제한하고 범용 승격을 중단한다.

## 10. R-NEXT-09: 정확한 최종 HEAD의 CI 및 게시 완료

남은 외부 작업은 PR #70의 추가 보완분 게시, 로컬600초 budget에서 완료하지 못한 전체 테스트의 종결, 그 exact source identity에 대한 hosted CI, 그리고 기존 timing-domain 독립 검토다. 이전 commit의 녹색 CI, artifact export 성공, 로컬 native 테스트는 서로 다른 증거다. `action_required`는 성공으로 바꾸어 기록하지 않는다. 현재 세션의 쓰기 도구 부재는 사용자 권한 미승인과 구별한다.

동일 PR branch에서 concurrency check 후 non-force commit/push하고, remaining-only 보고서와 execution log를 분리해 PR body를 갱신한다. base가 이동했으면 diff를 재검토하되 과거 ledger·fixture·holdout를 수정하지 않는다. PR을 자동 merge하거나 workflow approval 정책을 우회하지 않는다. timing 독립 검토 및 새로운 campaign은 별도의 책임과 사전등록을 가진 작업으로 남긴다.

## 권고 순서

R-NEXT-01과 R-NEXT-04를 독립적으로 진행하고, R-NEXT-02의 비용 개선은 R-NEXT-01의 정확도 판정으로 검증한다. R-NEXT-03의 실패 분류는 같은 frozen systems를 재사용하되 policy retuning과 분리한다. Chart 경로 R-NEXT-05는 모델별 opt-in으로 유지한다. R-NEXT-06은 실제 counter 여유가 확인되는 경우에만 병렬화/타이밍으로 확대한다. 범용 no-chart 연구 R-NEXT-08은 탐색 경로이며 production 승격의 선행조건이 아니다.

### 근거 위치

저장소의 `docs/reviews/thread_transfer_20261002/EXECUTION_STATUS.md`, `research/thread_transfer_mf_workspace_20261002/PREREGISTRATION.md`, `crates/rodas5p-integrators/src/raw_stage_target.rs`, `rodas5p_matrix_free_fast.rs`, `crates/rodas5p-core/src/polynomial_action.rs`, `laguerre_adjoint.rs`를 기준으로 남은 작업을 골랐다. 외부 문헌 확인 범위와 코드별 hash는 `SOURCE_MANIFEST.json`에 분리한다. 문헌의 정리를 이 코드에 적용했다거나 새로운 성능 연구를 수행했다는 뜻은 아니다.
