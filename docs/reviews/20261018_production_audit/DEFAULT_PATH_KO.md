# 기본 sequential matrix-free RODAS5P 감사

기준 commit은 `2fc61b9`이며, packet/Leja/sparse의 별도 판단을 포함하지 않는다. **문제별 검증을 붙인 통상적 적분 용도는 지원하지만, 요청한 모든 출력의 전역 오차까지 보장하는 범용 production 승격은 HOLD**다. README 자체는 현재 구현을 research platform으로 정확히 한정한다. 아래 DP-01은 기존 F-002/RA-07에 적힌 출력 정책의 한계를 현재 공개 API에서 확인한 것이며, 새로운 회귀로 분류하지 않는다.

## 실제 관측

1. 공개 예제 `cargo run --offline --locked -p rodas5p-integrators --no-default-features --example solve_stiff`는 exit 0, 완전 출력, 최대 절대오차 `2.4423468802936554e-9`, accepted step 93을 기록했다. Jacobian build와 direct factorization은 각각 0, JVP vectors는 4025였다. 이 대각 선형 문제의 미리 정해진 `1e-6` 예산을 충족한다.
2. 정확해가 `sin(t)`인 비자율 nonlinear Prothero–Robinson 문제 `y'=-10^6(y-sin t)+cos t+10(y-sin t)^2`, `y(0)=0`, `[0,1]`, `atol=1e-11`, `rtol=1e-8`, 초기 h `1e-3`, max h `1`, 101개 균등 dense 출력에서, JVP-only 기본 경로는 success=true, accepted 6/rejected 0이었다. 마지막 t=1의 절대오차는 `5.23258e-10`인데 최대 출력 오차는 `1.3083175084860343e-5`, `abs(error)/(atol+rtol*abs(sin(t)))` 최댓값은 **1801.6160215324019**였다. 성공은 완료 여부이며 모든 dense 출력에 대한 정확도 인증이 아니다.
3. NaN adaptive rtol과 NaN 초기값은 진입에서 거절됐다. `linear.rtol=+Inf`는 비영 구간에서 success=false와 zero-work partial result를 반환했다. NaN RHS 2회는 nonfinite failure 2회, success=false로 닫혔다. 시도 예산 1회도 endpoint에 도달한 척하지 않았다.

실행은 병렬 수정 중 작업 트리 빌드를 사용했다. 본 경로의 `dense_output_v2.rs`, `sequential.rs`, `adaptive.rs`, `output.rs`, `solver_types.rs`, 공개 예제는 기준 commit과 byte-identical이었다. 변경 중인 Leja 코드에서 최초 빌드의 float type annotation 오류가 발생했으나 담당자가 수정한 뒤 최종 Cargo 예제 및 두 독립 probe는 모두 exit 0이었다. 이 일시적 편집 오류를 baseline 결함으로 계산하지 않았다. 전체 역사적 campaign은 재실행하지 않았다.

## DP-01: 기본 dense 출력은 endpoint local estimator의 보호 범위 밖이다

기준 `dense_output_v2.rs:124–148`은 Off가 기본이며 stiff dense 출력이 대략 order 3이고 허용오차로 제어되지 않는다고 명시한다. `:1131–1153`은 endpoint embedded norm과 finite state를 수용한 뒤 interpolation을 수행한다. `:1214–1262`는 endpoint와 출력 schedule의 완료로 success를 정한다. 소스와 실제 관측이 일치한다.

한 번의 구성적 비교로 같은 PR 문제에 기존 `DenseErrorControl::Enforce`를 적용했다. 정확해 callback은 **주지 않았고**, analytic Jacobian/JVP/partial_t를 제공하는 일반 sequential driver에서 Off/Enforce를 비교했다. Off는 위와 같은 큰 dense 오차를 재현했다. Enforce는 최대 절대오차를 `7.102842647377372e-9`로 낮췄고 normalized maximum을 `1.4985737320954928`까지 줄였다. accepted 17/rejected 24, RHS 366, Jacobian builds 79, direct factorizations 38이었다. accepted midpoint estimate 최댓값은 `0.9216895307918039`였다.

즉 현재 보정은 이 사례의 정확도를 크게 개선하지만, midpoint estimate가 1보다 작다는 사실로 전 출력 normalized error<=1을 인증할 수는 없다. 이 비교는 일반 explicit-Jacobian driver 내부의 control mode 비교다. Matrix-free forcing 경로에 그대로 끼워 넣을 수 있는 동일 비용 보정이나 wall-time 개선으로 해석하지 않는다.

## DP-02: 실패한 RHS work의 누락

기준 `problem.rs:500–538`에서 callback 실행과 유한성 검사 뒤에 `rhs_calls`, `rhs_evaluations`를 증가시킨다. 따라서 callback이 NaN 또는 Err를 반환하면 실제 실행한 작업이 계산에서 빠진다. NaN RHS probe는 2 attempts, 2 nonfinite failures를 기록하면서 RHS evaluations=0이었다. 이는 해의 fail-closed 동작을 깨지는 않지만 실패를 포함한 work 비교의 완전성에 영향을 준다. 이 패턴은 이번 신규 최적화의 회귀가 아니다.

최소 수정은 attempted callback counters를 실행 직전에 올리고 completed counters가 필요하면 분리하는 것이다. 이미 해석된 역사적 work receipt의 의미가 달라지므로 스키마/정책 버전을 명시한다. 새 검증은 성공, callback Err, NaN output 각각의 attempted/completed counts만 확인하면 충분하다.

## DP-03: 영 구간의 입력 검증 누락

기준 `dense_output_v2.rs:1011–1030`은 adaptive만 즉시 검증하며 linear 검증은 `sequential.rs:1080` 첫 step에 있다. `[0,0]`에서 `linear.rtol=+Inf`를 주면 step을 실행하지 않고 success=true가 된다. 비영 구간은 실패 상태로 닫히므로 낮은 심각도의 입력 계약 불일치다. 공통 strict matrix-free config validation을 공개 진입 시점에 호출하면 고칠 수 있다.

## 다음 개발 단계와 유지할 계약

- 완료, endpoint local acceptance, requested-output accuracy를 별개 상태로 노출한다. 현재 결과에 보장되지 않은 상태를 certified로 채우지 않는다.
- 먼저 DP-02/DP-03을 작은 독립 수정으로 닫는다. 실패 work 보존은 향후 speed 평가의 전제다.
- 정확도 승격은 기존 전역 오차 연구 결과를 재사용하여 `r(t)=ŷ'(t)-f(t,ŷ(t))`의 **구간 전체** enclosure와 verified tube의 stability witness를 결합한다. 예를 들어 one-sided Lipschitz/logarithmic norm이 `<=-alpha`인 client에서는 `||e(t)|| <= exp(-alpha*t)||e(0)|| + integral_0^t exp(-alpha*(t-s))||r(s)|| ds`가 성립한다. norm, tube, full residual, inherited initial error를 실제 descriptor/계산에 연결하는 구현 의무가 남는다. midpoint 측정값을 sup bound로 쓰지 않는다.
- generic callback identity/epoch는 model 불변성에 대한 client 계약이다. 현재 문서의 이 경계를 유지한다. 자동 switching과 신규 packet 경로의 주장을 기존 기본 solver에 옮기지 않는다.
- 속도 승격은 이 정확도 gate를 통과한 client에 대해 동일 출력과 달성 오차, 실패/구간 residual/설정/cache 작업을 모두 포함한 비교로 수행한다. SP01의 opt-in residual accounting과 기존 RecomputeFinal default를 유지하며 이미 채택된 기록을 재실행하지 않는다.

완전한 입력, 결과, counters 및 소스 hash는 `default_path_audit.json`, 재현 소스와 원시 결과는 `default_path_evidence/`에 있다. 핵심 신규 근거는 기본 예제 1회, 유한성/부분 결과/PR의 작은 API probe 1개, 원인 교정을 확인한 Off/Enforce 쌍 1개다.
