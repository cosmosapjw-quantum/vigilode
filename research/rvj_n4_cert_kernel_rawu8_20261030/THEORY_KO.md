# RVJ–VigilODE N4 | 실제 source에 대한 exponential convolution interval과 원래 raw-U 8단계

Date: 2026-10-30 (KST). 연구단위 N4 (PARTIAL SCIENCE SLICE), 기존 공식 DAG와 독립 작성자 C9를 승격하지 않음.

## 1. 기준 source와 물리 전제 (derived, code-implemented)

무차원 모드 `a_j∈ℂ`, 양수 `w_j`, 실제 이진64의 표현값을 실수로 해석한다. `κ_j≥0`, `χ_α,φ_{αj}∈ℝ`, `ω,ξ_j∈ℝ`. 목표 source는

```
(a_j)' = (-κ_j+i ω ξ_j) a_j + i Σ_{α=1,2} χ_α φ_{αj} |m_α|²m_α,
m_α = Σ_j w_j φ_{αj} a_j.
```

비율이 다른 두 감쇠율 `κ_s=1/4`, `κ_f=1000`, `ω=1/2`, `χ=(1/20,1/50)`, `φ_1=1`, `φ_2=(-1)^j`, `h≤1/4`를 사용한 source-specific 표본이다. `w=fl(1/n)`은 명목상 실수 `1/n`과 같은 값이라고 **가정하지 않는다**.

가중 에너지 `E=Σw_j|a_j|²`에 대해 실수 계수와 같은 mode coupling 사용으로

```
E' = 2Re Σ_j w_j conjugate(a_j) (a_j)'
   = -2 Σ_j w_j κ_j |a_j|²
   +2Re i Σ_α χ_α |m_α|²m_α conjugate(m_α)
   = -2 Σ_j w_j κ_j |a_j|² ≤ 0.
```

이는 위의 정확한 미분방정식에 대한 identity다. floating 계산 결과의 에너지가 모든 순간 감소함을 인증하지는 않는다.

## 2. 실제 causal convolution kernel (derived)

시간창에서 nonzero source rate를 보존하는 복소 moment는

```
M_l(a,b,θ,h) = ∫₀ʰ exp((-a+iθ)(h-s)) exp(-b s) s^l ds
             = exp(iθh) Σ_{q≥0} (-iθ)^q/q! J_{l+q}(a,b,h),
J_r(a,b,h) = ∫₀ʰ exp[-a(h-s)-b s] s^r ds ≥ 0.
```

실수 `J_r`는 `b≥a`이면 `exp(-a h)∫₀ʰ exp[-(b-a)s]s^r ds`, `a>b`이면 `exp(-b h)∫₀ʰ exp[-(a-b)(h-s)]s^r ds`로 분기한다. 양의 적분, 고정밀 이진 정확 값, 적절한 큰 damping 차이 recurrence를 사용한다. `θ=0`에선 복소 Taylor를 사용하지 않고 실수 `J_l` 반환으로 위상 0의 정확 대칭을 보존한다.

P차 truncation의 remainder는 모든 실제 `s∈[0,h]`에서

```
| exp(-i θ s) - Σ_{q=0}^P (-iθs)^q/q! |
  ≤ (|θ|s)^(P+1)/(P+1)!.
```

따라서 실제 양수 커널에 의한 적분 remainder가

```
|δM_l| ≤ |θ|^(P+1)/(P+1)! · J_{l+P+1}(a,b,h).
```

`exp(iθh)`도 독립 P차 Taylor와 `(|θ|h)^(P+1)/(P+1)!` enclosure를 적용한다. 곱의 구간 연산이 이 둘의 cross term까지 포함한다. 이 연구는 P=18, `|θh|≤1/4`, `0<h≤1/4`, supported integer l=0..9, selected represented a,b>=0의 source box만 계산한다. 이보다 큰 창/위상은 무조건 새 decomposition/증명을 요구한다.

정확한 `θ=0`, `l=0`, `a≠b` 기준식은

```
M_0(a,b,0,h) = (exp(-a h)-exp(-b h))/(b-a)
```

이고 `a=b`에서는 `h exp(-a h)`의 연속 극한이다. Wolfram kernel에서도 이 symbolic formula를 확인했다. 수치적으로 거의 같은 `a,b`에서 이 차감식을 무조건 쓰지는 않는다.

## 3. Interval 수치 구현과 인증 범위 (implementation-verified)

독립 Python `Fraction` exact dyadic rational interval은 1280-bit outward quantization과 분석적 지수급수 꼬리로 양의 kernel을 감쌌다. Rust 1.99.1은 직접 이진64 directed interval을 사용하며 실제 곱·차가 정확히 표현될 때만 해당 source 경로를 허용하고 나머지는 fail closed한다. Range-reduced exp(-x), 양의 적분 recurrence, phase Taylor 및 remainder를 합성한다. Runtime RN/gradual underflow가 전제다.

테스트 grid는 `a={0.25,1000}`, `b={0.75,1000.5,2000.25,3000}`, `θ={-0.5,0,0.125}`, `l=0..9`, `h=0.25`의 2×4×3×10=240 case. 독립 high precision `mpmath` closed-form integration-by-parts recurrence와 Python Fraction `M_l` 구간은 240/240 부합했고 Rust 구간이 exact Fraction reference를 240/240 포함했다. 최대 Rust interval component width 약 `2.1649348980190553e-14`. 사용자가 제공한 N3H Rust `monomial_kernel` 원본을 변조하지 않고 별도 복사 후 wrappers만 붙여 비교하니 원 native 반환값 **115/240개는 매우 좁은 Fraction true interval 바깥**이었고, 240/240개 전부 새 Rust 넓은 구간 안에 있었다. 이것은 실제 반환값의 오차 포위에 필요한 검증이지 N3H trajectory 전체의 f64 global certificate가 아니다.

## 4. 원래 RODAS5P의 여덟 raw-U 방정식 (derived, native Rust small-reference checked)

고정 원본 tableau `fixtures/rodas5p_coefficients_snapshot.json` SHA256 `211255f6125b648c6955833c3da537d9c031ec78168b2918563c29fa88c8bce6`. `γ=0.21193756319429014`, `J_0=Df(y_n)`을 원래 두-rate rank-2 source의 정확한 표현값에서 계산. 각 i=0..7에서

```
Y_i = y_n + Σ_{j<i} A_ij U_j
R_i = U_i - hγ J_0 U_i - γΣ_{j<i} C_ij U_j - hγ f(Y_i) = 0
```

를 원래 coefficient frame에서 직접 풀고 독립 Python/SciPy dense reference와 비교했다. Rust `I` outward 원래 `R_i` 성분별 구간 upper도 8개 stage에 대해 생성했다. `n=12,24; h=.001,.25`의 4개 시도에서 max ordinary floating raw-U residual 2.43e-17..1.78e-15, max directed residual interval 2.52e-16..3.91e-14. 두 독립 구현의 출력 앞 6성분 차이는 ≤1.61e-17이다. **반드시 source 자체와 stage snapshot을 고정**해야 하며 일반 `OdeProblem`이나 PR131의 실제 `N01FrozenSourceProblem`을 대체한 검사는 아니다.

### 핵심적인 negative result: 정확한 stage root != 물리적 step acceptance

같은 소스와 시작점에서 SciPy BDF를 reference로 사용해 `h=.25`를 강제로 시도하면 Rust RODAS8 embedded WRMS는 약 253,000, 실제 endpoint 최대 오차 약 `9.466e-4`, 따라서 embedded<=1 기준은 FAIL. 한편 N3H source-specific slow/fast exponential 후보가 동일 h=.25에서 나타낸 BDF 대비 최대 오차는 n12 약 `1.603e-13`, n24 약 `7.742e-14`. 둘 다 동일 initial source이지만 시간이력 생산량/비용 매칭은 수행하지 않았다. PR131의 진단 `force_accept=true`를 실제 adaptive acceptance로 취급하면 안 된다.

## 5. 연구용 fail-closed transaction (implementation-verified, not production)

본 연구 standalone Rust source-specific reference에 합법적인 carrier를 캡처하는 작은 `ResearchTransaction`을 추가했다. `run_on_source(&source,&accepted,h)`의 trial은 private source에 의해서만 생성되며, `ResearchTransaction::decide`는 이전 carrier의 정확한 비트 일치, finite h, `embedded_wrms<=1`, 모든 8 stage interval residual 제한과 finite output을 검사한다. 실패한 시도는 accepted carrier/time을 변경하지 않고 8 dense solves를 rejected_work에 청구한다.

실제 네이티브 재현: 같은 n12 initial state에서 `h=.25` 거부 + `h=1e-8` 수용, `(attempts,accepts,rejects)=(2,1,1)`, `(charged,accepted,rejected)=(16,8,8)`. 이 demo는 **별도 Rust 연구 implementation**이다. 실제 PR131 native solver의 adaptive driver와 전체 callback source/epoch 연결, 범용 rollback을 실행했다고 주장하지 않는다.

## 6. 연구 결론과 새로운 N4 blocker

N4의 kernel subnode는 `IMPLEMENTATION_VERIFIED_SCOPED`로 수용 가능하다. source-specific original raw-U eight-stage direct reference는 `NUMERICALLY_CHECKED`이지만 실제 PR131 integration은 `NOT_RUN`. N3H whole trajectory의 complex exp/cubic moment/accepted-carrier rounding, 단계간 진짜 source-locked native state, adaptive integrator global error & matched achieved-error speed 모두 `OPEN`. RVJ 공식 후속 `3/8 + N4 partial`, VigilODE 공식 `7/23`, REI `28/88`, successor `0/12`, 실제 별도 작성자 C9 `0/1`, C10 NOT_STARTED, PRODUCTION HOLD. 새 source의 과학적 신규성은 주장하지 않는다.

## 문헌 참고 (verified discovery; not a proof of this implementation)

- T. Jawecki, W. Auzinger, O. Koch, “Computable upper error bounds for Krylov approximations to matrix exponentials and associated φ-functions,” arXiv:1809.03369 v2 (2019), https://arxiv.org/abs/1809.03369 . **현재 직접 kernel은 Krylov 소프트웨어가 아니며**, 참고할 것은 defect-based 검증철학과 φ-function 에러 계산.
- J.-P. Lessard, “Computing discrete convolutions with verified accuracy via Banach algebras and the FFT,” https://www.math.mcgill.ca/jplessard/papers/rig_FFT.pdf . 별도 고차 다중 모드 convolutions에 interval FFT를 고려할 때의 참고; 현 N4 n12/24 low-rank 집합에서 FFT 구현의 정확도·성능을 입증하지 않음.
- J. Huang, Z. Cen, A. Xu, L.-B. Liu, “A posteriori error estimation for a singularly perturbed Volterra integro-differential equation,” *Numerical Algorithms* 83 (2020), DOI 10.1007/s11075-019-00693-y . 문제 구조가 본 source와 달라 그 오차 정리를 그대로 이전하지 않음.
