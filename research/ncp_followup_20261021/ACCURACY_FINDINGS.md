# NCP 정확도 해석과 heat Jacobian 개선

기준은 게시 commit `7a63876a97fc17f640771ec06658269947049994`의 원시 결과와 부모 solver `de4c10a9f41321beef2313a3d7fb503bab35d714`다. 이 작업은 원시 benchmark를 변경하지 않고 파생 진단과 opt-in adapter 개선을 추가한다.

## 회전계: 판정 오류와 실제 오차의 분리

회전계는 `y'=omega[-y_1,y_0]`, `omega=10`, `y(0)=(1,0)`, `T=2 pi`이며 저장 output grid는 `t_j=2 pi j/40`이다. 따라서 `omega t_j=pi j/2`로 매 표본에서 한 성분이 영점을 지난다. 실제 판정은 `|y_i-y_i*|/(atol_i+rtol |y_i*|)`이고 `atol=1e-10`이므로 영점의 위상 오차에 대한 허용폭은 `1e-10`이다. `rtol=1e-6`이 상태의 비영 성분에 주는 scale보다 10,000배 엄격하다. 이 기준은 사용자의 원래 기준으로 유지한다.

RODAS5P embedded local controller가 이 시간구간 전체의 성분별 전역 정확도를 보증한다는 API 계약은 없다. 해당 결과를 로컬 오차제어의 구현 결함으로 즉시 분류할 수 없다. 작은 위상 지연 `delta`이면 영점 성분의 오차는 `A sin(delta) ~ A delta`, 비영 성분은 위상 기여만 보면 `A(1-cos(delta)) ~ A delta^2/2`이므로 componentwise global target은 위상 오차를 직접 제어하도록 설계해야 한다.

원시 native endpoint를 정확해에 직접 대입해 확인했다:

| rtol | endpoint scaled error | endpoint pass | 저장 sampled-grid pass |
|---|---:|---:|---:|
| 1e-4 | 903534.6119907275 | 0/7 | 0/7 |
| 1e-6 | 5156.614120926653 | 0/7 | 0/7 |
| 1e-8 | 24.37043373518188 | 0/7 | 0/7 |
| 1e-10 | 0.38768280732598714 | 7/7 | 7/7 |

따라서 게시 요약의 회전 work-precision `0/140`은 원시 결과와 모순된다. 전체 집계 교정은 `NCP_AUDIT` 파생물에서 관리한다. 여기서는 저장 native endpoint만 재계산했으며 저장되지 않은 trajectory를 재구성하지 않았다. `accuracy/ROTATION_ENDPOINT_DIAGNOSTIC.json`은 정확해 대입과 원래 endpoint metric 일치를 확인하고, 전체 sampled-grid pass flag의 재사용을 따로 표기한다.

실용적 후속은 외부 accuracy budget을 고정하고 내부 local tolerance를 별도 입력으로 분리해 work-precision 곡선을 구성하는 것이다. 현재 `rtol=1e-10` native 실행은 원래 `atol=1e-10` 영점 제약도 통과한 범위 근거다. 임의의 전역 tolerance 보증으로 확장하려면 residual/tube 또는 실제 목표 관측량의 a posteriori bound가 필요하다. 동일 fixed grid에 대한 재실행을 우선하고 phase/amplitude 진단은 기존 합격 기준을 대체하지 않는다.

## Heat Jacobian: 정확히 제거한 연산

기존 `Case::jac_into`는 각 표준기저 e_j에 JVP를 적용하여 dense Jacobian 열을 만든다. periodic_heat의 JVP는 RHS 호출이므로 각 Jacobian당 n회의 추가 RHS 호출과 n^2 stencil 방문을 수행한다. NCP n=128 run의 42 Jacobian builds에는 5376회 추가 derivative RHS가 붙었다.

새 선택은 case JSON의 `algorithm_config.jacobian_assembly="analytic_stencil"`이다. 기본값은 원래 basis-JVP 경로이므로 원래 비교 계약을 유지한다. 지원 모델은 periodic_heat 및 기존 advection_diffusion 이산식이다. 계수는

`d=D n^2`, `v=velocity*n`, `J_ii=-2d-v`, `J_i,i-1=d+v`, `J_i,i+1=d`.

동일한 periodic index가 겹치는 n=1/2를 별도 처리한다. n=1은 상수 단일 셀의 flux difference가 항등적으로 0이므로 cancellation 계산 대신 정확히 0을 쓴다. n=2는 중첩된 이웃을 합산한다. dimension/model/nonfinite 계수 오류는 matrix 변경 전에 거절한다.

복잡도는 **dense matrix zero fill O(n^2) + stencil fill O(n)**이며 storage와 dense LU의 복잡도는 유지된다. n회의 auxiliary RHS는 0회가 된다. 이것은 구현된 연산 제거이지 NCP wall-time speedup 추정이 아니다. periodic coupling의 첫/끝 행 corner를 버려 tridiagonal이라고 선언하지 않는다. cyclic band 또는 sparse/low-rank correction 경로는 별도 개발 과제다.

`component_benchmark_native` 출력에 실제 stencil build 횟수와 실행된 경우에만 complexity 설명을 추가한다. 기존 `extra_rhs_calls`는 보존된다. 내부 matrix-free 및 small-array 경로는 stencil dense assembly를 호출하지 않을 수 있으므로 횟수가 실제 실행 근거다. 음의 velocity 테스트는 기존 이산식의 Jacobian 일치 검증이며, 해당 left-biased advection의 물리적 upwind 안정성을 주장하지 않는다.

## 단계 수 및 이식 계약

실제 RODAS5P fast driver와 continuous extension은 **8 stages**다. 첨부 조언의 6-stage 설명을 코드 설계에 그대로 사용하면 workspace와 stage-loop 계약을 위반한다. 행렬재사용·리매핑·limiter 동결을 검토할 때 실제 8-stage coefficient/tableau를 기준으로 한다.

정확한 다음 실행은 기존 heat request를 복제하여 analytic_stencil만 설정하고 동일 소스·tol·CPU·output grid에서 baseline과 paired 비교한다. exact Fourier reference, 전체 RHS 비용, Jacobian/factorization 횟수, peak RSS 및 성능 불확실성을 함께 반환한다. 이번 코드는 generic FVM production 이식이나 matrix-free preconditioner 적용 완료를 의미하지 않는다.
