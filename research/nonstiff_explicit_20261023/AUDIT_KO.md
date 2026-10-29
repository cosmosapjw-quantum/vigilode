# 비강성 explicit 경로 도입을 위한 소스 감사

이 문서는 `4929c0f86cee03a06ac1ab2713e1b438dd10cfc5`의 현재 native API와 별도 응용 연구 `69e83044aacfb9a8fd2c6c3fb1146911beeedd79`를 읽은 결과이다. 새 explicit 구현의 최종 검증과 baseline 조사를 구별한다. 이 조사에서는 기존 과학 계산·테스트를 다시 실행하지 않았다.

## 1. 현재 구조와 가장 직접적인 개선

공개 진입점은 `Rodas5p::solve`이다. `Options`에는 적분 방법을 선택하는 필드가 없고, `Problem::new`의 기본 Jacobian은 유한차분이다. 따라서 RHS만 가진 비강성 문제도 Jacobian 구성과 선형해법이 있는 Rosenbrock 경로로 들어간다. Rust 소스 전체에서 Dormand–Prince, Tsitouras, Bogacki–Shampine 또는 일반 adaptive explicit RK 구현은 발견하지 못했다. `BlockMethod::Explicit`과 exponential 연구 모듈은 이 공백을 대체하는 일반 explicit RK 진입점이 아니다.

가장 직접적인 구현은 Dormand–Prince 5(4)와 FSAL을 사용하는 별도 RHS-only 적분기이다. 한 번 할당한 단계·상태·오차 배열을 재사용하고, 단계 결합은 상태 성분을 연속 순회한다. 성공적인 첫 단계는 RHS 7회, 이후 FSAL을 재사용하는 완전한 단계는 6회가 필요하다. 거절 시 첫 단계의 RHS는 같은 기저 상태와 같은 모델에서만 재사용하고, 거절된 종점의 마지막 RHS를 다음 단계의 첫 RHS로 승격하지 않는다. 실제 count는 callback 실패의 중도 종료를 포함해 기록해야 한다.

`Rodas5p::solve`의 이름과 동작은 Rosenbrock 방법을 명시한다. 공개 `Dopri5::solve` 또는 별도의 일반 solver 선택 API를 추가하고 실행된 방법을 결과에 기록하는 것이 자연스럽다. RHS-only라는 표현은 비강성 판정이 아니므로 기존 기본 진입점을 정보 없이 explicit으로 바꾸는 근거가 되지 않는다.

## 2. 구현 전에 해결해야 할 실제 경계

| 항목 | 현재 소스의 관측 | 구현 조치 |
|---|---|---|
| RHS-only 생성 | `OdeProblem::new`는 Jacobian과 JVP가 모두 없으면 거절한다. | 전용 explicit 입력 또는 추가 생성자를 제공한다. 가짜 zero JVP를 붙이지 않는다. |
| 결과 의미 | `JacobianRecord`와 `DriverWork`는 현재 미분·implicit 경로를 가리킨다. | 사용하지 않은 Jacobian을 실행한 것으로 기록하지 않도록 별도 explicit 기록을 둔다. |
| 오차 계산 비용 | `error_scale`는 호출마다 `Vec`를 할당한다. | 영속 scratch 또는 streaming WRMS로 같은 scalar/vector·zero-scale·overflow 규칙을 구현한다. |
| 시간 제어 | 공유 controller는 estimator order 인수를 받고, `AdaptiveOutputSession`은 실제 표현 가능한 시각과 관측 착지를 처리한다. | DOPRI 오차 추정의 지수 1/5와 별도 method metadata를 사용하여 재사용한다. |
| 실패 계약 | facade는 recoverable/fatal callback을 구별하고, 물리 상태는 수락 뒤 확정한다. | explicit 경로에서도 부분 상태·실패 작업량과 동일한 의미를 제공한다. |
| 기능 범위 | 질량행렬·dense extension·사건 검출은 기존 Rosenbrock 구현에 결합돼 있다. | explicit 지원 범위를 실제 구현한 조합으로 열고 나머지는 callback 전에 거절한다. |

공통 블록 LU나 기존 coupled homotopy를 explicit RK에 그대로 붙이면 절약한 선형대수 비용을 다시 도입할 수 있다. 첫 구현은 RHS 평가와 선형 결합만으로 완성하고, parallel stage iteration은 별도의 비용·수렴 비교로 판단하는 편이 타당하다. 순차 단계 의존성을 무시한 동시 계산은 동일 RK 식이 아니다. 반복적으로 단계 방정식을 푸는 병렬 방법을 택한다면 반복 횟수, 전체 RHS 수, 정밀도, pool 비용을 함께 제시해야 한다. 성분별 SIMD 친화 루프, 사용자 RHS 내부의 독립 성분 병렬화, 여러 IVP의 병렬 실행은 서로 다른 가속 경로로 측정한다.

## 3. 최신 별도 연구의 정확도 계약

별도 브랜치의 최신 commit은 native Rust solver를 변경하지 않았다. SciPy 1.17.0 BDF subclass가 33차원 물리 상태를 30개 affine 좌표에서 적분하면서 전체 상태의 오차 norm을 사용한다. 문서에 보존된 구간은 `[0,0.01]`, 공통 관측 시각은 9개이며, full33 WRMS 불일치의 보고값은 약 `7.22297e-8`이다. 이 값은 이미 게시된 유한 구간의 비교 근거로 재사용한다. 회복된 원자료에서 작업 계수와 공통 초기 단계값은 UNKNOWN이며 5.422초의 전체 프로세스 시간은 별도 solve 성능값이 아니다.

이 연구에서 새 explicit client로 전달할 핵심은 `v=d+Lu`에서 상태는 affine lift, 오차·증분은 tangent lift `Le`를 사용한다는 점이다. 30차원 좌표의 대각 WRMS만으로 원래 33차원 물리오차를 제어했다고 주장할 수 없다. 또한 해당 BDF의 predictor·endpoint scale과 DOPRI의 `max(|v_old|,|v_new|)` scale은 서로 다른 방법 계약이다. 새 적분기에 full-state norm hook을 붙일 때에는 DOPRI 단계·내장 오차에 맞게 연결하고 원래 물리 상태 기준의 수락·거절을 직접 비교해야 한다.

## 4. 이번 단계의 수용 기준과 후속 순서

첫 단계는 자율·비자율 항등 질량 ODE, scalar/vector tolerance, 전진 적분, 관측 시각 착지, 실패 후 부분 상태를 포함하는 explicit 경로를 완성하는 것이다. 독립 정확해에서 수렴률과 표본 오차를 측정하고, Jacobian/JVP/factor/Newton 호출이 실제로 0인지 계수로 확인한다. FSAL 재사용, 거절, callback 실패, 표현 가능한 최소 시간간격, 모델 변경 및 tolerance 성분별 scale을 표적 검증한다. 저할당 주장은 수락된 단계 수에 무관한 workspace 할당 측정과 출력 저장 비용의 구분으로 확인한다.

그다음 자동 선택은 caller opt-in으로 개발한다. 강성 판정의 수학적 인증으로 취급하지 않는 제한된 probe, 반복 거절·진척량에 대한 정해진 전환 조건, 이전 수락 상태에서의 restart, 전체 probe 비용 기록을 요구한다. 선택 자체의 성공은 원래 오차 요구 아래 전체 비용이 줄어드는지로 판정한다. 비강성 문제에서 싸다는 사실은 강성 또는 고주파 문제 전체에 대한 효율을 뜻하지 않으므로, broad benchmark는 비강성·강성·고주파·감쇠·비정규·크기 변화·짧은/긴 구간을 모두 포함한다.

기계 판독 가능한 항목과 기준 소스 해시는 `INTAKE.json`에 있다. 위 1–4절은 baseline에서 발견한 경계와 의무를 보존한다. 다음 절은 새 코드에 대한 별도의 독립 검토이며, 구현 여부와 실행 검증 여부를 구분한다.

## 5. 새 explicit 코드의 독립 검토

검토 대상은 `dopri5.rs`, 공개 `explicit.rs`, `explicit_ensemble.rs`, 그리고 이들에 연결된 계약·allocation 테스트이다. 이번 읽기 전용 검토에서 확인된 중대한 수치·실패 처리·API 결함은 없었다. 이는 모든 가능한 입력에서의 정확성 증명이나 production 적합성 판정이 아니다. 검토한 파일의 SHA256은 `INTAKE.json`의 `independent_new_source_review`에 남겼다.

| 초기 항목 | 새 소스에서 확인한 처리 | 남은 범위 |
|---|---|---|
| NS-A01: 일반 explicit 경로 부재 | `Dopri5::solve`와 `solve_observed`가 추가됐다. | 광범위한 실제 정확도·비용 비교 |
| NS-A02: 미분 공급자가 필요한 생성자 | RHS를 직접 받는 전용 driver를 사용한다. | 가짜 Jacobian/JVP는 추가하지 않았다. |
| NS-A03: implicit 결과 형식 | `ExplicitSolution`·`ExplicitStats`가 실행 방법과 실제 작업량을 기록한다. | 외부 도구가 두 결과 형식을 구분하도록 사용 문서 유지 |
| NS-A04: 단계별 scale 할당 | streaming WRMS와 DOPRI 오차 제어 지수 `1/5`를 사용한다. | 전체 실행의 출력 저장·workspace 비용을 분리 측정 |
| NS-A05: FSAL와 실패 계약 | 수락된 7번째 stage만 승격하고 거절 때는 불변 기저 RHS를 재사용한다. | 모델 변경·차원 변경 hook은 별도 개발 |
| NS-A06: 자동 선택의 근거 부족 | 사용자가 명시적으로 DOPRI를 선택한다. 기존 기본값은 유지한다. | 제한된 비용 예산을 가진 opt-in selector |
| NS-A07: 물리 상태 오차 norm | 기존 Python BDF 연구의 근거를 보존했다. | native DOPRI full33 metric 연결은 미구현 |

Dormand–Prince tableau, 5차 종점과 4차 내장 추정의 직접 차이 계수, 오차 지수 `1/5`가 일치한다. 7번째 stage의 입력 상태를 종점으로 사용하므로 수락 시 같은 종점 RHS를 다음 첫 stage로 재사용할 수 있다. 모든 시도가 마지막 stage까지 진행되는 고정 모델에서는 캐시 사용 시 `1+6A`, 미사용 시 `7A`회 RHS가 필요하다. 여기서 `A`는 수락과 거절을 합한 시도 수이다. callback이 중간에 실패하는 경우에는 이 식을 기계적으로 적용하지 않고 실제 진입 횟수와 완료 횟수를 기록한다.

오차 norm은 `atol_i+rtol·max(|y_old,i|,|y_new,i|)`를 직접 계산하고 scaled sum of squares로 중간 제곱 overflow를 줄인다. RHS 출력은 호출 전에 `NaN`으로 채워져 부분 쓰기나 잘못된 `Ok`를 탐지한다. facade는 이런 오류를 fatal로 취급하고, 명시적인 `Recoverable` 오류만 재시도한다. 출력·수락 상태·FSAL의 갱신 순서도 실패 후 마지막 수락 상태를 보존한다. 실제 Jacobian/JVP/시간미분/선형해법/Newton 경로는 없다.

ensemble은 각 IVP의 오차 norm과 적응 단계를 독립 유지하고 입력 순서대로 결과를 돌려준다. 한 IVP의 거절이나 fatal 오류로 다른 IVP를 취소하지 않는다. 따라서 이 구현은 독립 궤적의 처리량을 늘릴 수 있는 기반이며, 하나의 궤적에서 순차 시간 의존성을 제거한 구현은 아니다.

독립 검토에서 제안한 두 보완은 소스에 반영됐다. 첫째, `fsal=false`도 적분 중 수학적 모델 변경을 허용하지 않는다는 문장을 명시했다. 둘째, scalar `atol=rtol=0`, vector 전체 `atol=0`과 `rtol=0`을 callback 전에 거절하고, 양의 `rtol` 아래 정지한 영벡터를 허용하는 계약을 기존 테스트에 추가했다. 검토자는 수정된 소스를 확인했지만 직접 테스트를 실행하지 않았다.

현재 kernel과 facade는 같은 RHS 출력에 각각 한 번씩 `NaN`을 채운다. 이는 정확성 결함은 아니지만 매 RHS마다 불필요한 `O(n)` 메모리 패스가 될 수 있다. private wrapper가 kernel의 선행 초기화를 전제로 하도록 계약을 고정한 뒤 facade의 중복 초기화만 제거하고, 부분 쓰기·비유한 출력 거절 테스트와 크기별 성능을 비교하는 것이 가장 작은 후속 최적화다.

## 6. 실행 근거의 현재 효력

초기 실행 자료 `evidence/checks_v1/SUMMARY.json`은 kernel 계약 12개, allocation 1개, 공개 API 8개, ensemble 4개, 합계 **25개 PASS / 0개 FAIL**을 기록한다. 다만 당시 실행에는 게시된 소스와 다른 import·format 수정이 포함돼 있어, 이 최초 자료는 잠정적인 실행 이력으로 보존한다.

후속 실행에서는 게시 소스 `45037fc24e8f8101605f309e1fdb3a9a4e3df56d`, tree `b43b69b1ec5b86f3577d7cd6efdefe2d3acb9bad`에 대해 **25개 계약이 모두 통과**했다. kernel·allocation 13개는 `evidence/checks_v2/00.log`, 공개 API·ensemble 12개는 `evidence/checks_v3/01.log`에 있다. 그 사이 `checks_v2/01.log`의 공개 테스트 빌드는 `rodas5p_integrators`의 필요한 rlib 형식을 찾지 못해 exit 101로 중단됐다. 이 빌드 환경 실패는 그대로 보존했다. 두 패키지의 생성된 cargo cache를 제한적으로 정리한 뒤 같은 게시 코드로 통과했으며, 이 중단에서 실행된 수치 테스트나 실패한 수치 assertion은 없었다. 검토자는 로그·receipt·해시를 읽어 확인했고 테스트를 별도로 재실행하지 않았다.

이 25개는 새 explicit 범위의 표적 계약이다. 저장소 전체 회귀 검증, 광범위 성능 비교 또는 전역 정확도 인증으로 확대 해석하지 않는다.

이 절 갱신 시 광범위 벤치마크는 측정용 빌드 진행 중이며 결과는 미완료이다. 따라서 속도 배수, 외부 solver 대비 우위, 일반 production 승격을 기입하지 않았다. 새 공개 경로는 항등 질량의 전진 비강성 ODE와 지정 관측 시각 착지로 제한되며, 질량행렬·dense interpolation·사건 처리는 거절한다. 내장 오차 tolerance는 전역 오차 상계가 아니므로 물리적 positivity·보존량·장기 위상 오차도 별도로 확인해야 한다. broad production 판단은 계속 보류한다.

## 7. 기존 개발 항목과 연결한 다음 단계

`NEXT_STEPS.json`은 이전 coupled 연구의 **18개 항목을 삭제하지 않고**, 상태와 의존 관계를 그대로 유지한다. 그중 `N13`의 기존 `implemented_scoped`만 계승하며, 새 explicit 구현으로 다른 coupled·물리 항목이 완료됐다고 바꾸지 않았다. 이어서 `N19`–`N28`의 10개 항목을 추가했다.

`N19`–`N21`의 kernel·공개 API·ensemble 표적 계약은 위 고정 소스에서 통과했다. `N19`·`N20`은 제한된 기능 구현 완료로 기록하며 `N21`의 처리량 측정은 남겨 둔다. 다음 우선 작업 `N22`는 비강성, 강성, 빠른 진동, 감쇠, 비정규, 차원·출력 밀도 변화를 같은 실제 오차 기준으로 비교하는 것이다. 외부 비교 대상은 VODE/DVODE, diffsol, Julia로 한정한다. 설치되지 않은 비교기는 사용할 수 없다고 기록하며, 다른 solver로 대체하지 않는다. NCP는 사용하지 않는다. 내부 직렬 방법 비교와 독립 궤적 처리량 비교도 분리한다.

다음 정확도 작업은 `N23`의 full33 물리 norm 연결이다. 기존 affine BDF 연구를 재실행하는 대신 상태의 affine lift와 오차의 tangent lift를 새 DOPRI 경로에 맞춰 구현한다. `N24`의 continuous extension은 관측 밀도 때문에 내부 단계가 지나치게 잘리는 비용을 줄일 수 있다. 그러나 종점 5차 정확도가 보간 구간 전체의 차수를 보장하지 않으므로 interior·phase error를 직접 검증해야 한다.

실측 결과가 확보되면 `N25`에서 explicit/implicit 선택 비용을 제한하는 정책을 개발하고, `N26`에서 중복 메모리 패스와 성분 루프를 최적화한다. 자동 선택은 Jacobian의 존재 여부 대신 실제 진척당 비용, 거절과 실패 비용, 사용자의 모델 지식을 이용한다. 벡터화는 연속 배열 순회라는 소스 형태와 컴파일러·실측 근거를 구분하여 보고한다.

`N27`은 stage 고정점 또는 homotopy 반복으로 순차 stage 의존성을 바꾸는 별도 연구이다. 같은 RK target을 유지하는지, 반복 오차가 시간 적분 오차 예산에 들어오는지, 모든 sweep과 동기화를 포함해 기존 6–7회 RHS보다 유리한지를 확인한다. `N28`은 확산형 spectrum 같은 조건이 있는 모델에 한정해 안정화 explicit 다항식을 검토한다. 기존 Laguerre/Leja/Chebyshev 연구는 목적과 가정이 일치하는 부분만 재사용하고, 기저 이름의 변경 자체를 새로운 차수 또는 성능 정리로 해석하지 않는다.

## 최종 실행 결과 반영

위의 독립 소스 검토 이후 고정 소스에서 main 90개 설정·360회와 ensemble 4개 설정·16회 실행이 완료됐고, 모든 solver 반환과 저장 상태의 독립 정확해 재검산이 통과했다. `1e-6` 표본 오차 기준에서 비강성 9개 문제의 native RODAS 대비 중앙값 시간비는 2.73–8.93이었다. 반면 강성 대조군에서는 RODAS가 약 277배 빨랐고, 큰 벡터에서는 VODE가 새 경로보다 빨랐다. 작은 문제의 시간 변동과 더 엄격한 오차 기준의 미충족 사례를 포함해 최종 판정은 REPORT_KO.md 및 DECISIONS.json에 기록했다. 범용 생산 사용 판단은 계속 보류한다.
