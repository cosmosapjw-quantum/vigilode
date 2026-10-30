# BASS21 same-event coupled Thomson polarization response: 수학적 정식화

2026-10-30 KST. 최초 사전등록 Git commit `138072bcea820c8a8c583b098c8244fc69ac02e6`은 본 신규 수치 계산 **이전**에 실제 GitHub에 생성되었다. 기존 PR130의 후행 기록을 사전등록으로 바꾸지 않는다.

## 1. 가정 및 원본 식별

Signature `(-,+,+,+)`, `c`와 Thomson 단면적 `σ_T` 유지. 원본 `cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c`, `BASS_ATOMIC_NETWORK.py` SHA256 `baecce07aded483fa60006cc576feff67d068c0d70394f7095abacc6d47eaa80`; 50개의 원본 `raw_state62` 193점이 동일 물리 사건의 기하 `b`, 무편광 광자 모멘트 `J_l` (l<=8), `H`, `n_e`, 누적 forward optical depth `τ`를 제공한다. 원본 완전 ZIP SHA256 `6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8`.

Metric `ds²=-c²dt²+a² e^{-2b}(dx₁²+dx₂²)+a²e^{4b}dx₃²`, clock `x=ln(a/a_ref)`; source 정규화는 `F=a⁴ I/(a_ref⁴ I_ref)`이다. 원본 무편광 `I⁽⁰⁾(x,μ)=Σ_{l=0}^8(2l+1)J_l(x)P_l(μ)`을 **고정된 외력**으로 둔다. 총 강도 `I=I⁽⁰⁾+δI`이며 선형편광 `Q=I_θ-I_φ`, `U=V=0`. 산란 전자 normal-comoving, cold elastic Thomson이다. 이 계산은 source-owned 선형 광자 응답이며 원본 전체 non-linear matter/Einstein/chemistry 또는 무편광 배경을 재적분하지 않는다.

## 2. Maxwell/Thomson coherency kernel 직접 유도

입사 방향 `n'`에서 `P=1-n'n'ᵀ`, screen tetrad `e_θ,e_φ`에 대해

`J_in(n') = (I/2)P + (Q/2)(e_θ e_θᵀ-e_φ e_φᵀ)`.

출사 방향 `n`에 대한 cold elastic gain tensor는

`J_gain(n)=(3/(8π))P(n)[∫ J_in(n')dΩ']P(n)`.

LRS 축대칭 조건에서 모든 off-diagonal은 적분 후 0이고, `M=∫J_in dΩ`의 일반 축대칭 대각성분 차이는

`M_zz-M_xx = -2π (J₂⁽⁰⁾+j₂)+(3π/2)B_Q`, 여기서

`j₀=∫₀¹ δI(η)dη`, `j₂=∫₀¹ δI(η)P₂(η)dη`, `B_Q=2∫₀¹Q(η)(1-η²)dη`.

이때 `S_I=Tr J_gain`, `S_Q=e_θ·J_gain·e_θ-e_φ·J_gain·e_φ`에서

`S_I(μ)=J₀⁽⁰⁾ + j₀ + [ ½(J₂⁽⁰⁾+j₂) -(3/8)B_Q]P₂(μ)`,

`S_Q(μ)= [-¾(J₂⁽⁰⁾+j₂)+(9/16)B_Q](1-μ²)`.

기존 무편광 배경의 gain을 뺀 **응답** `δI`의 source만 남기면

`δS_I=j₀+[½ j₂ -(3/8) B_Q]P₂(μ)`.

이 식은 polarization → intensity quadrupole → polarization의 양방향 gain coupling을 포함하며, δI와 Q의 extinction `-g δI, -gQ`도 포함한다. 소스에 없는 thermal, recoil, electron tilt는 넣지 않는다. Independent explicit Cartesian 48×96 angular tensor integration 10-tests 중 1개로 검증.

Q가 `A(1-μ²)` 패턴이면 `B_Q=(16/15) A`; gain Q→Q 고유계수는 `(9/16)(16/15)=3/5`. 동시에 Q→I gain은 `-(2/5) A P₂`이므로 intensity quadrupole source는 `-2A/25`. 균질 `b=0`, 상수 원본 `J₂`, g, x에서는

`d/dτ [j₂,A]^T = [[-9/10,-2/25],[-3/4,-2/5]][j₂,A]^T + [0,-3 J₂⁽⁰⁾/4]^T`, `τ=g x`.

고유값은 정확히 `-1`, `-3/10`. 따라서 테스트할 manufactured exact 해는

`[j₂(τ),A(τ)]ᵀ = M⁻¹(exp(M τ)-1)[0,-3 J₂⁽⁰⁾/4]ᵀ`.

이는 완전 frozen intensity baseline response의 상수 계수 극한이다. 배경 J₂⁽⁰⁾의 시간진화 자체까지 해결했다고 주장하지 않는다.

## 3. Bianchi-I characteristics와 Volterra 2필드

Observer `(x_o,μ_o)`와 earlier source epoch `x` 사이 `δb=b(x)-b(x_o)`:

`D²=(1-μ_o²)exp(2δb)+μ_o²exp(-4δb)`, `μ_x=μ_o exp(-2δb)/D`.

두 응답에 동일 bolometric energy kernel `e^{-Δτ}/D⁴`를 적용한다:

`δI_o(μ_o)=∫₀^{x_o}dx g(x)exp[-(τ_o−τ_x)]/D⁴ ×{j₀(x)+[j₂(x)/2−3 B_Q(x)/8]P₂(μ_x)}`,

`Q_o(μ_o)=∫₀^{x_o}dx g(x)exp[-(τ_o−τ_x)]/D⁴ ×(1−μ_x²)[−3(J₂⁽⁰⁾+j₂)/4+9 B_Q/16]`.

따라서 Volterra 제2종 결합 적분방정식이다. 정확한 양의 `1−μ_x²=(1−μ_o²)exp(2δb)/D²`를 사용해 Q의 축점 `μ_o=1`은 엄밀히 0이 된다. 초기 `δI=Q=0`. 원래 forward τ는 z=20→2 구간이며 z=0 CMB τ가 아니다.

## 4. 계산, 단위, 한계

원본 193 epochs에서 불균등 `x`의 composite Simpson 가중치를 사전에 형성한다. μ 반구에서 GL24로 모멘트를 계산하고 GL48을 독립 refinement로 적용하며, 출력은 원래 17개 반구 방향이다. Picard로 응답 kernel의 전체 자기일관 수렴을 검사하고 operator residual을 재평가한다. 실험 결과의 `193→97→49`는 해당 frozen data의 sampled quadrature 차이만 평가하고 원래 BDF IVP 전역오차는 보장하지 않는다. Quasi-linear perturbation 부호/양의강도 검사, Q axis zero, zero extinction, 원본 first-gain 환원 및 상수계수 정확해를 함께 테스트한다.

F와 Q는 source reference energy-normalized brightness로 무차원, g=cσ_T n_e/H는 무차원이며 dx 역시 무차원이다. `D`, μ, b, τ도 무차원. 따라서 두 Volterra 식의 차원은 일치한다.

**Claim ceiling**: 동일 원본 BASS21 사건에서 unpolarized L8 baseline 고정 하의 **self-consistent Thomson gain coupling in (δI,Q)**. Not: fully solved polarized BASS hierarchy with changing baseline J_l, Einstein/electron feedback, spectral thermal validity, CMB E/B or observational posterior. 독립 저자 R6/C9, original BDF global convergence 별도 미완료.

## 5. 선행 문헌과 신규성 범위

Anile (1974) axisymmetric Bianchi I polarization transfer: DOI 10.1007/BF02639073. Challinor (2000) 1+3 covariant polarized Thomson collision and source multipoles: DOI 10.1103/PhysRevD.62.043004. Kamionkowski (2021) coupled integral form for temperature/polarization: DOI 10.1103/PhysRevD.104.063512. 따라서 axisymmetric polarization transfer, Thomson multipole coupling 또는 Volterra 방법 자체의 최초 발견을 주장하지 않는다. 이번 새 계산은 특정 frozen BASS21 50×193 source owner 하에서 source-preserved 2-field feedback dataset을 생성한 것으로 제한한다.
