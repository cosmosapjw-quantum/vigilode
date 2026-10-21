# VigilODE production 재감사와 통합 연구 보고서

작성일: 2026-10-21. 대상 저장소: cosmosapjw-quantum/vigilode. 기준은 production/wave3-integration의 f2f20c1과 최신 계획 cc2e1a0이다. 별도 연구 줄기의 solver 구현 33e45625와 그 줄기의 최신 tip ee65d4db에 있는 후속 자료를 함께 검토했다. 정확한 SHA는 INPUTS.json에 있다.

## 1. 결론과 production 사용 범위

**범용 production solver, 임의 callback의 전역 정확도 보증, 일반적인 속도 우위의 동시 승격은 아직 HOLD다.** 기존 protected RODAS5P의 검증된 문제별 사용 가능성은 유지한다. 이번 감사는 표준 ODE solver의 국소 tolerance를 전역 error certificate와 혼동하지 않는다. 관측 global error가 한 tolerance unit을 넘었다는 사실만으로 모든 통상적 solver 사용을 부정하지도 않는다. 다만 사용자가 요구한 전역 정확도·속도 승격과 이 저장소의 G-A 이후 단계에는 아직 충족하지 못한 구체적 의무가 있다.

| 구분 | 이번 판정 | 실질적 의미 |
|---|---|---|
| Protected RODAS5P, 기존 검증 문제 | 제한된 사용 유지 | dense/banded 등 지원 route, local-error 의미, 실패 상태를 확인하면서 사용 가능. 범용 인증이나 release 1.0 승인 아님 |
| 공개 facade | 수정 후 표적 검증 통과, release HOLD | unavailable dense estimate 수용 및 event/FD 결함을 고침. facade-owned Stats와 실행 가능한 지원 조합 질의는 구현됨. 최종 전체 suite·native bench·안정 API 검토는 남음 |
| model-bound 인증 API | 이식된 범위의 기존 증명 재사용 + 새 통합 검사 | PR101, collective packet, common damping, Gray–Scott 등 선언된 모델·norm·horizon·budget 범위. facade의 임의 RHS가 자동 인증되는 것은 아님 |
| Lagged GMRES / B1d | opt-in 연구 후보, 승격 보류 | 분해 횟수 감소는 관측했으나 HIRES의 고정 gate를 실패했고 추가 solve 비용이 큼 |
| Laguerre·Leja·Chebyshev / homotopy | scoped API 통합, 일반 speed HOLD | 동일 입력·original budget의 admission과 실제 ODE endpoint 인증을 보존. 모든 문제에서 Krylov보다 빠르다는 주장 없음 |
| 최신 외부 comparator campaign | 기존 INCONCLUSIVE 유지 | 132 rows 재감사 완료. 부하가 높은 host의 timing을 새 speed PASS로 재채점하지 않음 |

변경별 증거와 실패 로그는 EXECUTION.json, 노드별 JSON 및 evidence/에 있다. 최종 publication SHA는 Git commit 자체가 권위이며, 파일 안에 아직 존재하지 않는 자기 commit SHA를 만들어 넣지 않는다.

## 2. 최신 진행상황과 코드 구조

최근 wave 1–3은 하나의 vigilode facade, fast dense/banded 경로, vector atol 일부 조합, W-filtered interior estimate, post-processing event location, typed unsupported errors, lagged stage solver, transaction과 외부 comparator harness를 구축했다. 최신 측정은 33e60cf에서 실행되었고 f2f20c1까지의 관련 수치 kernel은 동일하다. 그래서 기존 raw 결과를 다시 실행하지 않고 재감사했다.

중요한 통합 누락이 있었다. 10월 18일 A01–A12의 완성된 인증 API는 production 줄기의 조상이 아닌 sibling branch에 남아 있었다. 기준점의 414-file manifest 대조에서 321개 동일, 46개 변경, 47개 부재였다. 이번에는 현 facade와 fast 경로를 보존하면서 해당 scoped 구현을 실제 이식했다.

호환성 처리는 세 곳이 핵심이다. 기존 BoundPr101Problem을 그대로 두고 parameterized 모델을 Pr101VerifiedError와 pr101_verified 모듈로 분리했다. DenseErrorControl에는 WFiltered 모드와 VerifiedResidual을 함께 유지했다. packet query에는 기존 weighted_center_displacement와 새로운 centered_weighted_radius를 모두 보존했다. 반환 중심의 반올림을 누락하거나 중복 가산하지 않도록 triangle inequality와 native 호환성 검사를 적용했다.

기존 A01–A12 전부, production 계획의 15개 workstream A1–E2, wave-4의 8개 task를 research_dag.json에서 다룬다. 원래 계획의 “14개” 표기는 실제 표의 15개와 다르므로 새 보고서에서 교정했다. 노드 번호의 이름 공간도 분리했다. 기존 보고서·실험·실패 판정은 덮어쓰지 않았다.

### 두 갈래 브랜치를 통합하는 구체적 방식

통합의 기준은 production의 최신 실행 경로를 유지하면서 sibling의 전체 변경 집합을 수용하는 것이다. 공통 조상 `59d95eb` 이후 research tip `ee65d4db`의 변경 경로는 1,133개다. 먼저 scoped 구현과 기존 자료를 이식한 상태에서 331개는 이미 동일했고, 788개는 충돌 없이 추가할 수 있었으며, 14개는 명시적 충돌 해결이 필요했다. 단일 노드만 복사한 통합이 아니다. 788개에는 component benchmark 코드·도구와 cloud/component 연구 기록도 포함한다. 최종 내용과 원본의 차이는 BRANCH_INTEGRATION.json 및 독립 검토에서 설명한다.

| 충돌 경계 | 통합 규칙 | 실패를 막는 검사 |
|---|---|---|
| production facade와 인증 연구 API | production의 기본 route 유지, 연구 API는 명시적 진입점으로 추가 | 기존 facade contracts와 이식 API contracts를 같은 source에서 검사 |
| PR101의 서로 다른 모델/API | 기존 BoundPr101Problem 보존, parameterized 모델을 별도 pr101_verified namespace로 제공 | 원래 모델과 parameterized provider의 독립 binding tests |
| dense control과 packet 반경 | WFiltered와 VerifiedResidual 공존, 두 query getter 유지 | enforcing failure rollback, inherited center/radius 포함 검사 |
| 연구 ledger의 L-0136–L-0142 충돌 | production 원문은 append-only, 양쪽 원본 archive와 commit-qualified index로 조회 | 두 archive의 hash, 모든 284개 qualified entry와 7개 의미 충돌 확인 |
| component 도구의 오래된 registry | 과거 246개 모듈의 포함 관계를 보존하고 통합 후 8개 추가 모듈을 명시 | 1차 통합 254개, 후속 global_budget/stage_target_policy 포함 최종 256개. 기존 94개 variant 의미 보존 |
| 과거 실행 결과 | 원래 source·환경·verdict 그대로 보존 | 현재 source에서 재실행한 증거와 historical receipt를 분리 |

Git 이력도 두 줄기를 보존해야 한다. 새 통합 commit은 production `f2f20c1`, plan `cc2e1a0`, research `ee65d4db`를 부모로 갖는 resolved merge tree로 발행한다. 이는 `ours`로 research 이력만 붙여 놓는 병합과 다르다. 실제로 전체 incoming delta를 대조하고 누락 파일을 수용한 뒤 충돌부를 검사한다. 원래 브랜치에는 force-push하지 않는다. 배포 대상 production으로의 후속 병합은 stable API·전체 native suite·해당 정확도/성능 gate가 닫힌 뒤 결정한다.

이후에는 기능 코드를 다시 두 줄기에 중복 구현하지 않는 편이 낫다. 공통 core와 facade는 이 통합 줄기에서 유지하고, 새 연구는 별도 모듈·명시적 capability·자체 receipt로 추가한다. 각 후보의 `proof`, `implementation`, `execution`, `routing` 상태를 따로 기록한다. 자동 route 승격은 동일 원래 budget에서 독립 검증과 전체 비용 비교를 통과한 후보에만 허용한다. 이 규칙으로 코드 통합과 과학적 승격을 각각 완료할 수 있다.

## 3. 적대적 감사에서 찾은 결함과 코드 수정

| 영역 | 발견한 실패 기작 | 적용한 수정과 남은 범위 |
|---|---|---|
| Sequential dense control | Enforce/WFilteredEnforce에서 내부 estimate 계산 오류를 unavailable로 세고 step을 수용 | recoverable 오류는 reject/retry, 실패 작업 계상, estimate_failures 및 Stats v4. Report mode는 진단 의미 유지. fatal Err 전체 cost 반환은 아직 별도 의무 |
| Event output | extension 부재를 무시하거나 event 시각의 상태 대신 step endpoint를 반환 | typed EventInterpolationFailed, 해당 step event rollback. event는 여전히 post-processing이며 다중/tangent root 포착 보증 없음 |
| Finite differences | 유한 y=f64::MAX에서 forward perturbation이 infinity로 변함 | 이 경우만 signed backward increment, 비유한 Jacobian entry 거부. 일반 finite 범위의 연산 순서 유지 |
| Staleness bound | 유한 weight의 비율 overflow → 0·infinity=NaN → max reduction이 행을 버림 → 잘못 작은 q | 모든 관련 ratio·합·분모 검증, 불가능한 bound는 infinity로 처리. 전체 outward certificate로 확대하지 않음 |
| GMRES failure work | monitor/operator 오류에서 이미 끝낸 column 작업이 report와 함께 사라짐 | workspace progress를 성공·실패 양쪽에 보존하고 lagged caller의 실패 호출 비용 보정. happy breakdown의 미수용 결과는 output commit 없이 반환 |
| Admission economics | 개별 finite 비용의 누적합 overflow로 infinity≤infinity가 admission을 허용 | prospective cumulative state의 유한성 확인 후 원자적 갱신, 실패 시 ledger 보존 |
| Benchmark metric | scale overflow가 유한 오차를 0으로 계산 | 부적합 metric은 unavailable, hypot 및 안전한 normalized subtraction 사용 |
| Benchmark protocol | reference 입력 검증 누락, 후속 반복의 실패 status/work 손실 | independent implementation family·유한 tolerance/threshold·중복 검증, 모든 반복의 status/grid/work 보존 |
| Component 도구 | 분기 통합 후 registry 불일치, 짧은 process의 미관측 RSS를 0으로 표기 | 역사적 모듈 포함 관계와 최종 10개 추가를 검사, RSS는 null/UNAVAILABLE, memory gate는 HOLD_UNKNOWN |
| CI | integrator fmt와 core clippy 일부 제외 | 관련 format 문제를 해결하고 gate에 포함, 수동 실행 entry 추가. CI 전체 완료로 표기하지 않음 |

수정의 독립 검토는 INDEPENDENT_CODE_REVIEW와 노드별 peer review에 있다. 최초 semantic RED, 테스트 작성 중 compile 실패, 공유 build 산출물에서 발생한 E0463, 이후 성공 실행을 모두 보존했다. 실패를 지우고 성공 로그만 남기지 않았다.

## 4. 수학 연구: 범위 교정과 구성적 개선

### 4.1 고쳐야 할 기존 주장

T1의 vector mean-value 표현은 일반적으로 하나의 J(ξ)로 쓸 수 없다. segment 위 Jacobian의 적분 평균으로 바꾸면 uniform norm 가정 아래 원래 bound를 보존할 수 있다. T3의 negative-real scalar W-filter 비율은 전 구간 증명을 완성했지만, 모든 normal left-half-plane spectrum으로의 확장은 성립하지 않는다. z=4πi에서 constant-defect midpoint convolution은 0이고 filtered estimate는 0이 아니다.

T4의 continuous residual comparison은 재사용한다. 그러나 degree-p curve에 cubic RHS를 합성하면 residual 차수는 최대 3p이다. quartic Gray–Scott/collective에는 degree 12가 필요하다. rational RHS에는 denominator가 0에서 떨어져 있다는 enclosure가 추가로 필요하다. T6의 Woodbury 비용에는 setup의 nr² 및 RHS의 r² 항을 포함한다. T8의 현재 admission 정책에 UCB1 regret를 자동 부여할 수 없으므로, 구현된 deterministic work-credit 조건과 미입증 확률적 주장을 분리했다.

이들은 새로운 불가능성 정리를 성과로 내세운 것이 아니라 잘못 확장한 전제를 바로잡는 작업이다. 유효한 부분을 다음 구성에 사용했다.

### 4.2 실제로 증명한 정확도·계산량 개선

**Cayley residual majorant.** Hilbert norm에서 J가 dissipative이고 R=(I−aJ)⁻¹이면 C=2R−I는 contraction이다. x=v−aJv, Cx=v+aJv의 제곱 norm 차이가 4a Re〈v,Jv〉≤0이기 때문이다. f(R)를 C의 다항식으로 다시 표현하면 계수 절댓값 합으로 operator norm을 제어할 수 있다. 실제 RODAS5P 계수의 normalized residual→output bound는 189.418367에서 31.334689로 감소한다. 같은 contraction 가정 아래 충분 inner iteration budget도 줄어든다. 이 결과는 일반 nonnormal dissipative Hilbert 문제를 포함하지만 임의 weighted infinity norm으로 전용하지 않는다.

**Exact Bernstein sensitivity.** 실수 비양의 spectrum을 가진 normal frozen operator에서는 기존 400-point grid를 exact rational Bernstein subdivision으로 대체했다. decimal 계수와 실제 binary64 계수 모두에서 stage별 전 구간 supremum 합은 23.35 이하로 닫힌다. scalar pointwise sum 23.34와 operator bound를 혼동하지 않았다. 초기 residual 1, contraction 1/2, output budget 1/16, 동일 iteration cost라는 조건에서는 충분 반복 예산이 8×12=96에서 8×9=72로 줄어든다. 보장된 25% iteration-budget 감소이며 실측 solver speedup은 아니다.

**전역 defect budget G1.** A(s)=exp(∫sᵀ max(μ,0))라 놓는다. 실제 반환 곡선의 검증된 residual δ와 초기 radius r0에 대해 A(0)r0+∫0ᵀ A(s)δ(s)ds≤ε이면 모든 t∈[0,T]의 error≤ε이다. 곡선 jump와 query center rounding도 같은 budget에 추가한다. 이 조건은 고정 norm, full tube, verified residual을 요구하며 midpoint estimate만으로 충족되지 않는다.

**작업 배분 G2.** block j의 오류 상한을 A_j/n_j^p, 단계당 비용 상한을 c_j라 하면 Hölder로 연속 최적 비용을 구하고 n_j를 올림하여 원래 오류 budget을 보존할 수 있다. 올림의 추가 비용 상한은 Σc_j 미만이다. 문서의 두-block 정확 예는 비용 상한 1640을 1000으로 줄인다. 실제 단계 비용이 c_j와 같을 때 39.024% counted-work 감소가 되며, 일반 비용 상한만 주어지면 개선된 비용 보장으로 해석한다.

각 증명, 정확 유리수 oracle, 전제와 실제 code 연결은 integrator.md, research_dag.md, DENSE_GLOBAL_BUDGET_THEORY.md에 있다. SciSpace·Exa는 원전 대조에, Wolfram은 핵심 식의 독립 symbolic check에 사용했다. 도구 출력을 증명 대신 사용하지 않았다.

## 5. 새 실행 결과를 어떻게 해석해야 하는가

기존 E1b raw 자료를 독립 consumer로 다시 읽어 132개 row와 750개 matched-error record를 확인했다. 131개 row 성공, DVODE 1개 실패를 유지했다. dense 30개 중 20개의 global grid error가 한 tolerance unit을 넘었고 최대 9.0004518449였다. local/global 원인을 분리할 per-step trace가 없으므로 단순 retuning으로 승격하지 않았다.

새 B1d는 log-norm 가정 대신 실제 resolvent norm을 계산하는 opt-in 경로다. HIRES GMRES의 factorization은 120에서 90으로 감소했으나 90/204=0.4412로 기존 0.4 기준을 넘었다. Brusselator는 19/82=0.2317이었다. 하지만 추가 resolvent solve는 각각 720/1900회, 전체 LU solve는 10546/8527회였다. fast 기준 1680/736회와 함께 비교해야 한다. 분해 횟수만 보고 speed PASS로 승격할 수 없다.

기존 all-DAG 로컬 450-row 결과의 dense/structured 비율 방향은 후속 erratum을 따라 읽는다. 비율이 1 미만이면 structured가 느리다. 원래 뒤집힌 문장은 역사적 파일에 남겨 두고 correction과 함께 연결했다.

현재 실행 환경에서 전체 외부 bench crate의 Rust typecheck/link는 완료하지 못했다. 제공 archive에서 29개 dependency를 추가 복원했지만 lock에 필요한 정확한 41개 version이 없고, GNU Fortran/SUNDIALS 개발 패키지도 없다. apt의 UID/setgroups 권한 실패도 기록했다. 대체 dependency나 lock 변경으로 PASS를 만들지 않았다. 실제 순수 Rust module 시험 9개와 raw replay의 PASS는 이 미완료 상태와 구분한다.

## 6. 추가 요청에 따라 여기에서 완료한 코드

아래 코드는 후속 Codex에 구현 과제로 넘기지 않고 이 통합 줄기에 작성했다. N01–N08은 각각 전달한 범위를 명시한다. 광범위한 production 계획의 모든 미지원 조합을 완성했다는 뜻은 아니다.

| 노드 | 실제 구현 | 검증 범위와 남은 경계 |
|---|---|---|
| N01 | facade 소유 record 22종, work counter 61개, exhaustive 변환, 지원 조합 질의 | 기존 v4 JSON byte 일치와 384개 조합 검사. Rust 타입 identity는 바뀌므로 downstream migration 필요. 미지원 수치 조합은 typed refusal |
| N02 | build-time 전체 source closure, binary/corpus hash, 실행 전후 일치 검사, RADAU5/CVODE 3수준 독립 reference | standalone·실제 harness+mock adapter 검사 통과. 외부 native bench 전체 typecheck/link는 미완료 |
| N03 | sealed provider/run에 결합한 outward GlobalBudget replay | 원래 atol보다 큰 budget으로 바꾸는 요청 거부. initial radius·jump·query rounding·실패 작업 보존. PR101 full-curve 범위 |
| N04 | outward 정수 배분 검증과 유한 resource cap | 같은 오류 예산 0.5에서 uniform [20,20] 비용40 → [8,24] 비용32. A_j/c_j의 ODE 모델 결합은 후속 연구 |
| N05 | immutable matrix/norm 기반 Cayley·Bernstein target planner, output와 embedded-estimator contamination 검사, 대각 staleness 경로 | 32차원 동일 IVP에서 dense Resolvent의 추가 RHS32회 → Diagonal0회. 기존 Bounded도0회이므로 Diagonal의 일반 속도 우위는 주장하지 않음. planner와 일반 nonlinear driver 연결은 미완료 |
| N06 | 같은 IVP·원래 budget의 polynomial 비교 실행기 | Krylov, Laguerre/Leja/Chebyshev priority, router 및 analytic diagonal-exp baseline. fallback backend와 비용을 그대로 기록. baseline은 protected RODAS5P가 아님 |
| N07 | direct λ=1 대 continuation 비교 실행기 | nonlinear endpoint와 actual-k stage-functional 계약 분리. stage 인증을 trajectory 인증으로 승격하지 않음 |
| N08 | NCP preflight·검증·측정·resume 도구와 독립 결과 consumer | source/binary/shared-library/raw hash와 전체 필수 row를 결합. 실행 성공만으로 production 또는 speed PASS를 만들지 않음 |

N03 full-curve smoke에서 기존 provider radius는 약 8.392×10⁻¹², 전역 replay는 약 9.495×10⁻¹²이며 두 값 모두 원래 예산 10⁻⁴ 안에 있다. replay가 radius 자체를 줄인다는 주장이 아니다. 그 목적은 full horizon·initial error·jump·query rounding의 소유권을 하나의 재사용 가능한 검증 계약으로 만드는 데 있다. N04의 20% 개선은 stated algebraic work model의 보장이고, wall-time이나 임의 ODE의 복잡도 개선으로 확대하지 않는다.

N05 대각 검사는 inverse RHS를 없애는 대신 matrix/weight 검사 비용을 계상한다. 반환 endpoint bound는 세 arm에서 약 3.1333×10⁻¹⁵로 동일했고 fallback은 0이었다. NCP에서는 Bounded/Resolvent/Diagonal 전부를 같은 조건에서 비교한다. 단계 target planner는 별도로 측정하고 `used_by_driver:false`로 기록하여 실제 driver 가속으로 오인하지 않게 했다.

실제 검증 중 발견한 새 회귀도 수정했다. facade mirror의 zero-counter JSON 생략 속성이 빠져 golden 비교가 실패한 문제, source closure가 합법적인 내부 report symlink까지 거부한 문제, promotion reader가 누락된 reference ladder·잘못 결합한 실행 파일을 수용할 수 있던 문제를 고쳤다. 최초 실패와 수정 후 성공 로그를 모두 남겼다. symlink는 link text·내부 target 경로·target bytes를 함께 결합하고 외부/broken/cyclic link는 거부한다.

최종 변경 소스의 통합 검사는 `EXECUTION.json`과 `evidence/final_04/`의 최종 단일-source receipt가 권위다. 앞선 `final_01`은 각 명령이 통과했어도 실행 중 registry test source가 바뀌었으므로 전체 source identity 판정 FAIL로 보존했다. 두 번째 실행은 디스크가 가득 찬 시점에 facade linker SIGBUS와 progress 기록 ENOSPC로 중단되어 FAIL로 보존했다. 이전의 성공 11개 group은 원래 log·출력된 exit code와 연결하고, 실패한 facade와 미실행 7개 group만 별도 복구 실행으로 검증한다. 복구 실행 final_03은 native298·doctest4·Python142 검사에 성공했으나 마지막 scoped Clippy의 single_match 경고로 전체 FAIL이었다. Jacobian::Banded 검사를 같은 조건·순서·반환값의 if-let으로 바꾸고 독립 검토했다. 소스가 한 파일 바뀌었으므로 final_04에서 등록된19개 group 모두 같은 source manifest로 통과했다. 이전의 실패와 합성 receipt는 삭제하지 않는다. 노드별 앞선 성공 횟수를 서로 겹쳐 더해 총시험 수를 부풀리지 않는다. N02의 mock adapter 성공은 native comparator 실행으로 집계하지 않는다.

## 7. Codex 6 Luna low에 남기는 작업

저예산 executor의 작업은 고정된 코드를 검증하고 NCP에서 사전 고정 campaign을 실행하는 것이다. `CODEX_HANDOFF_KO.md`와 `NCP_RUNBOOK.md`를 전달하면 된다. 이미 완료된 증명, branch 통합, 핵심 API 설계, 새 solver 구현을 다시 맡기지 않는다.

NCP 설정은 2회 warmup·21회 측정·1/2/4/8 threads와 3,221개 필수 결과 ID를 고정한다. 3,220개의 comparison/bound-client 결과와 한 개 multi-arm work-precision 결과다. source·설정·host boot·환경·binary·loader/shared-library identity가 같을 때만 resume한다. missing dependency, 미관측 RSS, timeout, 실패한 원래 budget, reference disagreement는 HOLD로 반환한다. 관측 RSS는 엄밀한 메모리 상계가 아니며, CPU affinity는 독점 host의 증명이 아니다.

이 환경에서는 실제 NCP campaign을 실행하지 않았다. dry-run은 dirty source, GNU Fortran/GNU time 부재, 설정된 8GiB admission에 부족한 메모리를 실제로 발견하여 HOLD했다. 저예산 세션은 준비된 NCP host에서 정확한 lock/toolchain/native dependencies로 실행하고 raw receipt를 반환하면 된다. cloud 계정 생성이나 유료 자원 생성은 실행기에 포함하지 않았다.

향후 연구는 `NEXT_STEPS.json`의 `future_research`에 분리했다. 우선순위는 (1) exact native comparator 및 전체 suite 의무 닫기, (2) G2 상수를 실제 accepted-step ODE 오류·비용 법칙에 결합, (3) nonlinear drift를 포함한 adaptive stage target 연결, (4) band/low-rank resolvent witness, (5) 독립 window homotopy의 full-cost 병렬 span 검증이다. 이들은 이번 실행 인계에 숨겨 둔 코딩 과제가 아니다. 해당 범위의 새로운 근거가 마련될 때 별도 변경으로 진행한다.

최종 source-bound 통합 검증은 native 298개, doctest 4개, Python 142개 통과이며 ignored 5개와 Python skipped 3개는 PASS에 포함하지 않았다. Python 및 command별 정확한 수와 검사 범위는 EXECUTION.json을 따른다. 실행 전후 source manifest가 동일하고 모든 등록 명령의 exit code가 0임을 확인했다.
