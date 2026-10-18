# A11 — 실제 Gray–Scott 연속 궤적에 연결한 대칭 Jacobian 인증

## 구현 범위와 판정

기존 연구에서 증명한 반응 2×2 블록과 주기 확산의 μ₂ 상한을 재사용하고, 그 상한을 **실제 연속 궤적 튜브의 인증과 전역 오차 전파**에 연결했다. `BoundGrayScottProblem`은 생성된 point RHS/JVP와 구간 RHS, grid, 상수를 함께 소유한다. 호출자가 다른 callback이나 임의의 scalar bound를 끼워 넣는 생성자는 없다. `into_verified()`/`gray_scott_2d_verified()`가 A05의 공통 `integrate_verified_residual`에 전달하는 capability를 만든다.

지원 모델은 기존 `gray_scott_2d`와 같은 주기 격자 및 표현된 binary64 상수 Du=0.002, Dv=0.001, F=0.04, F+k=(0.04+0.06)이다. 상수의 실수 0.1과 binary64 덧셈 결과를 혼동하지 않는다. 임의 PDE, 다른 경계 조건, 다른 반응식에 대한 인증이 아니다. 초기 Euclidean ball, forward span, 고정 absolute budget는 A05의 검증 계약을 따른다. 넓은 배포·장시간 performance 승격은 별도 증거가 필요하다.

## 구성적 정리와 실제 연결

격자 셀 수를 C=g², 상태를 (u₁,…,uC,v₁,…,vC)로 둔다. 반응 Jacobian의 셀별 블록은

\[
R(u,v)=\begin{pmatrix}-v^2-F&-2uv\\v^2&2uv-(F+k)\end{pmatrix}.
\]

그 대칭부는 a=−v²−F, d=2uv−(F+k), b=(v²−2uv)/2에 대해 [[a,b],[b,d]]이고, 최대 고유값은

\[
\lambda_+(u,v)=\frac{a+d}{2}+\sqrt{\left(\frac{a-d}{2}\right)^2+b^2}.
\]

주기 5점 Laplacian L은 대칭이고 xᵀLx=−Σ_edges(x_i−x_j)²≤0이다. g=2의 중복 주기 이웃도 두 번 기여하며 음의 준정부호 성질을 유지한다. Du,Dv 및 표현된 inv_dx2는 양수이므로 전체 확산 대칭부는 음의 준정부호다. 따라서 모든 x에 대해

\[
x^T\operatorname{sym}(J)x\leq \max_i\sup_{(u,v)\in B_i}\lambda_+(u,v)\,\|x\|_2^2.
\]

구간 연산으로 trace/차이/offdiagonal을 외향 계산하고 magnitude 제곱·sqrt·합의 상단을 취하면 그 supremum의 상한 μ를 얻는다. 블록당 일정한 산술 수이므로 **O(C) 시간, O(1) 추가 산술 저장공간**이다. dense symmetric eigensolve나 N×N 저장이 필요하지 않다. 이 정리는 이전 발표 범위를 재사용하며, 이번 구현의 새 부분은 모델 소유권과 실제 튜브/defect 경로에 연결된 것이다.

A05의 represented quartic q(t)의 전체 구간 enclosure Q와 q′ enclosure를 계산한다. Gray–Scott interval stencil 및 cubic를 평가해 ρ ≥ sup‖q′−f(q)‖₂를 얻는다. 후보 튜브 Q+[-r,r]^(2C)에서 위 μ를 계산한다. 이것은 L2 ball의 안전한 box 외포함이다. e(t)=y(t)−q(t)에 대해

\[
D^+\|e\|_2\leq \mu\|e\|_2+\rho,
\qquad E(h)=e^{\mu h}E(0)+\rho\int_0^h e^{\mu s}\,ds.
\]

기존 directed exponential/gain 구현을 사용한다. `max(E(0),E(h))≤r`가 확인될 때 first-exit 논증으로 실제 해가 전체 튜브 안에 유지된다. 구간 끝에서 이 E(h)를 다음 구간 초기 반지름으로 전달한다. 최종 requested output center 평가의 반올림 변위도 별도로 더한 뒤 원래 atol를 검사한다. 이후 사용자가 별도로 `segment.evaluate(t)`를 호출하면 반환 radius는 반올림을 포함하지만 run의 atol를 자동 강제하지 않는다. 그 추가 질의의 budget은 반환 radius로 판정해야 한다. 단지 supplied box에서 μ를 계산한 결과를 trajectory 인증으로 오인하지 않는다. 실패 시 상태/출력 prefix를 commit하지 않으며 실패한 provider/solver 작업은 남긴다.

## 계산량 개선

기존 point RHS 및 JVP는 호출마다 길이 C의 Laplacian 임시 Vec 두 개를 만들었다. 새 코드는 각 셀의 두 stencil을 output loop에서 같은 연산 순서로 직접 평가한다. **RHS/JVP 한 쌍당 Vec 4개, zero-initialization 4C개, 별도 임시 저장 2C개**를 제거한다(동시에 호출하지 않을 때 peak). stencil sum의 기존 l+r+up+down−4c 순서와 inv_dx2 적용 위치를 유지한다. 이 메모리·작업 감소는 코드로 보장되지만 integer indexing tradeoff가 있어 elapsed speedup은 주장하지 않는다. 외부 `eval_rhs`가 반환 Vec를 생성하는 비용이나 solver의 다른 allocation은 이 주장에 포함되지 않는다.

모든 interval primitive는 시도 직전에 provider work에 기록된다. μ의 기존 함수도 counted helper를 사용해 overflow 이전의 prefix가 유지된다. grid/state/sparse storage 크기의 usize overflow는 할당 전에 typed error로 거절한다.

## 검증 설계와 증거의 한계

- 새 공통 driver로 homogeneous v=0 exact u(t)=1−(1−u₀)e^(−Ft)와 nonzero initial radius를 검증한다.
- 4×4 비균일·비영 cubic Gray–Scott 궤적을 전체 continuous tube와 11개 출력에서 인증하고, 독립 coordinate-loop RK4 두 격자와 비교한다. RK4는 분별용 수치 대조이며 인증 권위는 구간 정리다.
- 실제 Gray–Scott homogeneous mode에 대해 full JVP Rayleigh quotient와 2×2 eigenvalue를 비교해 maxdiag가 부적절함을 확인한다.
- g=2 중복 이웃부터 g=4까지 독립 stencil과 point RHS의 bit 동일성, 구간 RHS 포함, JVP 중심 차분을 확인한다.
- overflow/malformed interval 및 단일 실패 trial의 미commit prefix와 retained work를 검증한다.
- grid 크기에 따른 lognorm provider operation 증가가 정확히 선형임을 확인한다.

실행 결과: 새 native test 7개, counted helper 회귀 test 3개, capability compile-fail doc 1개가 통과했다. nonlinear native replay는 11개 출력, 6 accepted/2 rejected, 최대 출력 반지름 4.631424315750511e−5 ≤ 원래 atol 1e−4로 완료했다. 독립 RK4 대조 최대 차이는 8.94e−15이며 두 RK4 격자 간 차이는 9.51e−15다. 해당 작은 값은 reference 반올림 한계 수준의 진단값이며 인증 오차 상한은 반환 반지름이다. 정확한 명령은 `RESULT.json`과 `evidence/`에 기록한다. 첫 compile attempt는 다른 병행 노드 `actual_k_homotopy.rs`의 coefficients 참조/소유 타입 오류로 테스트 실행 전에 실패했으며 그 사실을 보존했다.

## 다음 개발/현장 실행

1. `cargo test --locked --offline -p rodas5p-integrators --test all_dag_a11_verified -- --nocapture`와 compile-fail doc를 재현한다.
2. `cargo run --locked --offline -p rodas5p-integrators --example gray_scott_verified_run -- 4`로 전체 center/radius 및 work JSON을 얻는다.
3. 현장에서는 목표 격자/시간/허용오차에 대한 실제 wall time와 peak memory를 측정한다. 현재 raw interval defect의 dependency 폭 때문에 장시간·엄격 budget에서는 작은 step이 필요할 수 있다. 큰 모델에서 tolerance를 완화해 통과시키지 않는다.
4. 후속 정확도/속도 작업은 quartic→degree12 reaction polynomial의 계수 상쇄를 유지한 residual enclosure 또는 adaptive time subdivision이다. 수학적으로 유효한 현재 경로에 필요조건은 아니며, 이번 A11 원래 수락 조건을 뒤로 미루는 항목도 아니다. 목표는 같은 오류 budget에서 불필요한 interval dependency를 줄이는 것이다.

독립 A05/A11 심사 결과는 `../../reviews/DENSE_INDEPENDENT.md`/`.json`의 PASS_SCOPED이다. 모델/튜브 연결과 위 수학 경로에서 blocking defect는 발견되지 않았다. 전체 production 및 elapsed speed 판정은 승격하지 않는다.
