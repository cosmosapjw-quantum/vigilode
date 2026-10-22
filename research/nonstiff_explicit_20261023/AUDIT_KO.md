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

기계 판독 가능한 항목과 기준 소스 해시는 `INTAKE.json`에 있다. 이 파일의 finding은 새로운 코드를 실행하기 전의 경계와 의무이며, 최종 구현 결과로부터 각 항목의 처리 상태를 갱신해야 한다.
