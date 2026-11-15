# U04 잔차 → step 오차 예산 (PPSW1 matrix-free common-W)

범위: PPSW1-6b의 stage 선형계 `W x_i = b_i`, `W = I - γ h T`, `T = J(t_m, y_m)`(JVP, step 시작에서 고정). 정리는 아래 가정에서만 성립하고, 상수는 경험적으로 검사했다. 증명된 전역 정리가 아니다.

## 1. 한 stage의 섭동
계산해(x̃_i)의 원 잔차를 `r_i = b_i - W x̃_i`라 하면 `x̃_i - x_i = -W^{-1} r_i`다. backend는 모든 i에 대해 `||r_i||_2 ≤ s·max(atol_lin, η ||b_i||_2)`를 `W`를 한 번 더 적용해 직접 확인한다(s = true_residual_slack, 기본 10). Krylov 내부의 projected residual만으로는 통과시키지 않는다.

가정 A1: 쓰는 노름에서 `T`의 log-norm이 `μ(T) ≤ μ₀`이고 `γ h μ₀ < 1`이다. 그러면 `||W^{-1}|| ≤ 1/(1 - γ h μ₀)`이다. 소산계(μ₀ ≤ 0)에서는 이 값이 1 이하다. Fisher MOL은 T가 대칭이고 μ₂(T) ≤ 1이므로 `||W^{-1}||₂ ≤ 1/(1 - 0.62 h)`다.
결과: `||x̃_i - x_i||_2 ≤ s·max(atol_lin, η ||b_i||_2) / (1 - γ h μ₀)`.
비정규 T에서는 μ₀가 고유값보다 훨씬 클 수 있다. 이때는 이 상계를 쓰지 않고 c1처럼 dense와 직접 비교해 확인한다.

## 2. 오차 추정기와 다음 step으로의 전파
- 추정기 `Est = Y_{m,6} - Σ w_j Y_{m-1,j}`에서 현재 step의 섭동은 `Y_{m,6}`을 통해 계수 1로 들어간다. 직전 stage 섭동은 Σ|w_j|(σ=1에서 253)배까지 증폭된다.
- 다음 step에서는 `B(σ)ΔY + h A F'(Y)ΔY + h T Γ ΔY`로 전파된다. PW04의 영안정 곱 상수(σ ≤ 1.1에서 성분 최대 138)가 전역 누적의 최악 상계다.
- 따라서 `η ||b||`가 step 허용오차 척도(rtol·|y|)의 상당 부분이면 추정기 잡음이 허용오차와 같은 크기가 된다. 그러면 거절과 축소가 반복될 수 있다. 실제 관측이 이렇다: Fisher N=40, block GMRES, η = rtol에서 tol 1e-8은 1.13e6 step을 썼고 끝점이 Dense와 58 tol-단위 달랐다(`logs/u04_try1.log`).

## 3. 채택한 예산
`η ≤ ρ · rtol_step`, ρ = `LINEAR_BUDGET_RATIO` = 1e-2. 소산·W≈I 영역에서 stage 섭동이 step 허용오차의 1% 이하가 되도록 하는 선택이다. `Session::try_step`이 callback 전에 이를 검사해 위반하면 InvalidInput을 반환한다. rtol_step = 0이면 atol_lin ≤ ρ·atol_step을 요구한다. driver가 starter 전에 거부하려면 공개 함수 `LinearBackend::check_budget`을 먼저 호출해야 한다(test driver는 starter 이후에 거부를 받는다).

근거 (c4, Fisher N=40, 끝점 WRMS 차이를 tol 단위로 측정):

| tol | η/rtol | independent(restart 60) | independent(restart 10) | block |
|---|---|---|---|---|
| 1e-5 | 1e-3 | 5.4e-8 | 1.4e-3 | 2.8e-4 |
| 1e-5 | 1e-5 | 5.4e-8 | 3.0e-7 | 2.2e-6 |
| 1e-8 | 1e-2 | 3.1e-4 | 7.1e-4 | 1.1e-3 |
| 1e-8 | 1e-4 | 3.1e-4 | 1.5e-3 | 8.1e-5 |
| 1e-8 | 1 (가드 이전) | – | – | 57.8, 1.13e6 step |
| 1e-5 | 10 (가드 이전) | – | – | 0.20, 10.7배 step |

ρ ≤ 1e-2인 모든 행에서 차이가 1.6e-3 tol-단위 이하였다. ρ=0.1(tol 1e-5, block)은 0.025였지만 가드는 보수적으로 1e-2를 쓴다. 이 값은 이 문제 하나의 근거이며, 다른 문제·비정규 T·atol 지배 성분에서는 별도 확인이 필요하다(미검증).

## 4. 관측된 구현 성질 (Krylov 코드는 변경 없음)
production `solve_gmres`는 restart cycle 사이에서만 임계값을 검사한다(cycle 안 조기 종료 없음, 내부 step마다 작은 최소제곱을 다시 푼다). 따라서 restart ≥ n이면 rtol과 무관하게 Krylov 공간 전체(n)를 쓴다. c4의 restart 60, n=40 행이 η와 무관하게 같은 것은 이 때문이다. restart 10은 JVP 2.3배, 내적 10배를 줄였다. `gmres_givens`는 cycle 안 종료가 있으나 research-only로 표시된 kernel이라 쓰지 않았다.
