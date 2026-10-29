# R4 다항식·변환 오차 수학 검토

대상은 `F = Σ_{k=0}^4 φ_k(hA) w_k`, φ₀(z)=exp(z), φₖ(z)=∫₀¹ exp(uz)(1-u)^{k-1}du/(k-1)!이다. h≥0, A=Aᵀ, spec(A)⊂[-ρ,-λ], 0≤λ≤ρ가 다항식 경로의 전제다. hA는 무차원이고 모든 wₖ는 F와 같은 단위다. `wₖ=h^k bₖ`의 형성 오차는 이 action의 오차와 별개다. 기본 시공간 metric/자연단위는 이 순수 행렬 함수 검토에 해당하지 않는다.

## 실제 개선된 점과 증명의 범위

`polynomial_action.rs`는 일반 가중치 5개와 동일 벡터의 5개 계수를 구분한다. 전자는 m회의 block product이지만 5m회의 vector product이고, 후자는 m회다. 따라서 차수만으로 모든 사용례의 속도를 비교할 수 없다. `Certified`는 절대 2-norm 상한이며, `truncation_budget`을 전체 오차 허용치로 읽으면 안 된다. Laguerre recurrence의 반올림 전파와 단순 선언한 spectrum은 `EstimateOnly`로 남긴 구현이 옳다. 더 약한 timing용 경로의 실제 속도는 certified 경로의 속도로 전용할 수 없다.

Chebyshev에서 실제 binary64 shift s와 half-width d를 정확한 실수로 해석하여 X=(A+sI)/d, a=-hs, b=hd를 두면 hA=aI+bX가 정확히 성립한다. 반올림으로 X의 spectrum이 [-1,1]을 벗어나는 경우도 구현은 δ와 η=√(2δ)를 사용한다. arcosh(1+δ)≤√(2δ)이므로 |Tₙ(x)|≤exp(nη), |Uₙ(x)|≤(n+1)exp(nη)가 해당 확대 구간에서 성립한다.

DLMF 10.35의 modified-Bessel 생성함수를 spectral calculus와 φ 적분 표현에 넣으면

    cₙₖ = (2−[n=0]) ∫₀¹ exp(ua) Iₙ(ub) (1−u)^(k−1) du/(k−1)!   (k≥1)

을 얻는다. Iₙ의 양의 급수와 Kummer 변환을 적용하면 현재 소스의 `exp(a) 1F1(k;p+k+1;−a) p!/(p+k)!` 계수가 나온다. 이때 p=2j+n이다. 모든 항이 비음수가 되는 표현은 interval 계산에 적합하다. 이는 원문 공식을 source implementation으로 옮기는 직접 유도이며 부동소수점 실행 검산과 별도로 분류한다.

Laguerre에서 X=−A/β, a=hβ, q=a/(1+a)라 두면 exp(−aX)=(1−q)ΣₙqⁿLₙ(X)이다. φ 적분 뒤 Euler 적분과 Pfaff 변환으로

    cₙₖ = [n!/(n+k)!] qⁿ(1−q) ₂F₁(n+1,k;n+k+1;q)

가 나오므로 현재 양의 계수 급수는 수학적 형태가 일치한다. DLMF 18.14.8의 α=0 부등식은 x≥0에서 |Lₙ(x)|≤exp(x/2)를 준다. spec(X)⊂[0,L′]에 한정하여

    ||ΣₖΣₙ>m cₙₖLₙ(X)wₖ||₂ ≤ exp(L′/2) q^(m+1) Σₖ ||wₖ||₂/k!

가 성립한다. 따라서 Laguerre tail은 성립하지만 이 식만으로 계산 recurrence의 전체 오차 인증을 얻지는 못한다.

Chebyshev의 각 computed recurrence local residual εⱼ를 실제 소스처럼 outward interval로 감싸면, 초기 벡터가 정확할 때

    ||t̃ₙ−Tₙ(X)w||₂ ≤ Σ_{j=0}^{n−1}(n−j)exp[(n−j−1)η] ||εⱼ||₂.

선형 비동차 recurrence의 Green 함수가 U 다항식이라는 사실에서 직접 나온다. 최종 합은 이 bound에 선택 계수의 절댓값을 곱하고, coefficient enclosure·합산오차·절단오차를 더한다. 이 구조는 올바르다. 단, directed arithmetic의 플랫폼 전제와 입력 spectral witness가 필요하며, 본 감사의 유한 oracle sweep은 그 일반 정리 전체를 대체하지 않는다.

## 새 계산 후보: scale-safe norm과 역팩토리얼

norm을 √Σxᵢ²로 직접 계산하면 유한 norm을 가진 1e300 벡터도 중간 제곱이 overflow한다. 반대로 아주 작은 값은 제곱 상한이 최소 subnormal로 넓어져 지나치게 느슨해질 수 있다. 후보는 M=max|xᵢ|, M=m2ᵉ를 구하고 yᵢ=|xᵢ|2^(−e)를 outward로 감싸서

    ||x||₂ ≤ 2ᵉ sqrt_up(Σᵢ mul_up(yᵢ,yᵢ))

를 `ExpBound`로 반환한다. 최대 yᵢ가 [1/2,1)이므로 overflow 위험을 크게 제거한다. yᵢ가 subnormal로 반올림되면 한 ULP outward widening을 사용한다. 전역 finite-range 지원 주장이나 polynomial action 전체 이식은 이 후보의 범위가 아니다.

역팩토리얼은 부동소수점 k!를 만든 뒤 나누지 않고 r₀=1, rₖ=div_up(rₖ₋₁,k)를 mantissa/exponent 표현에서 반복한다. k가 binary64로 정확히 표현되는 양의 정수이고 exponent 합이 i64 범위를 넘지 않으면 귀납적으로 rₖ≥1/k!이다. k=171 부근의 팩토리얼 overflow를 피한다. nilpotent 계수도 termⱼ₊₁=termⱼ·(|h|N)/(j+k+1)로 갱신해 같은 전략을 적용할 수 있다.

## 후속 대안의 선택

우선순위는 symmetric nonpositive frozen operator에서 현재 Chebyshev의 전체오차 인증과 안전한 scale handling을 유지하며 coefficient reuse를 최적화하는 것이다. Laguerre에는 local residual의 전파 상한을 추가해야 한다. 단순한 안전 baseline은 E₀=0, E₁≥||ε₀||, Eₙ₊₁≤dₙEₙ+[n/(n+1)]Eₙ₋₁+||εₙ||이며 dₙ=max(|(2n+1)/(n+1)|,|(2n+1−L′)/(n+1)|)이다. 이는 유도된 상한이나 실용성이 낮을 수 있으므로 새로운 시험에서 tightness를 먼저 평가한다.

Leja/Newton, Faber 또는 rational 방법으로 범위를 확대하려면 nonnormality를 다루는 별도 operator error bound가 필요하다. 현재 symmetric 정리를 eigenvalue만 확인한 비정규 행렬에 적용하지 않는다. 이 대안들은 이번에 실행하지 않았으며 현재 성능 결과로 우열을 정하지 않는다.

## 실제 확인한 문헌

- NIST DLMF §10.35, 생성함수/modified Bessel expansion: https://dlmf.nist.gov/10.35 (2026-10-01 직접 조회).
- NIST DLMF §18.14.8, Laguerre bound, α≥0, x≥0: https://dlmf.nist.gov/18.14.E8 (2026-10-01 직접 조회). 원 증명은 해당 페이지가 Koornwinder 1977 Remark 4.1을 가리킨다. 원 논문 전체를 읽었다고 주장하지 않는다.
