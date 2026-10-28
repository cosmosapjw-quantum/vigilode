# 최신 native packet 감사를 위한 구성적 연구

2026-10-18 KST. 대상은 VigilODE `2fc61b9`의 새 collective packet·trajectory·damped moment 경로이다. 기존 Loop12/DW 및 이전 production 연구의 닫힌 증명과 캠페인은 재실행하지 않았다. 이번 결과는 변경된 native 계약의 오차 반경·연속시간 검증·구간 연산을 보강한다. 아래 T1–T3의 수학적 결론과 native 적용 상태는 `theory_research.json`에 구분한다.

## T1. 불확실한 초기값을 포함하는 가중 에너지 tube와 정확한 logarithmic norm

가정: n≥1, 실수 ω_j, κ≥0, χ∈R, w_j>0. 실제 저장된 가중치의 정확한 실수 합을 S=Σw_j라 둔다. 따라서 binary64 가중치 합이 정확히 1이라는 전제를 추가하지 않는다. 복소 상태 a∈Cⁿ, m(a)=Σw_j a_j와

\[
 f_j(a)=(-\kappa+i\omega_j)a_j+i\chi|m(a)|^2m(a),\qquad
 \|a\|_w^2=\sum_jw_j|a_j|^2
\]

를 정의한다. 초기 참값은 |a_j(0)−â_j(0)|≤ρ_j인 closed product ball에 있다. 다음 유한값을 outward 계산한다.

\[
 E_+=\sum_jw_j(|\widehat a_j(0)|+\rho_j)^2,
 \quad e_0=\Big(\sum_jw_j\rho_j^2\Big)^{1/2}.
\]

**결론 T1a.** 모든 허용 초기값에 대해 전시간 해가 존재하며

\[
 \|a(t)\|_w^2\le E_+e^{-2\kappa t},\qquad
 |m(a(t))|\le \sqrt{SE_+}\,e^{-\kappa t}.
\]

증명: 에너지 미분의 선형 skew 항은 0이고 collective 항의 합은
2 Re(iχ|m|²m\bar m)=0이다. 따라서 E′=−2κE. 유한 차원 polynomial vector field의 국소 존재 해가 이 bounded set을 벗어나지 않으므로 유한시간 폭발이 없고 계속 연장된다. 초기 product ball의 에너지 상한은 삼각부등식으로 위 E_+이며, weighted Cauchy–Schwarz로 |m|²≤S E이다. 이는 중심 상태의 에너지 admission만으로 초기 불확실성의 tube까지 가정하던 오류를 닫는다.

**결론 T1b.** Cⁿ을 실수 Hilbert 공간으로 보고 weighted logarithmic norm을 취하면, 각 a에서 정확히

\[
 \mu_w(Df(a))=-\kappa+S|\chi|\,|m(a)|^2.
\]

증명: g(m)=iχ|m|²m, m=x+iy에 대한 실수 Jacobian symmetric part는

\[
 H(m)=\chi\begin{pmatrix}-2xy&x^2-y^2\\x^2-y^2&2xy\end{pmatrix},
 \quad\det(\lambda I-H)=\lambda^2-\chi^2(x^2+y^2)^2.
\]

그 고유값은 ±|χ||m|²이다. δm=Σw_jδa_j라 하면 collective Jacobian의 quadratic form은 δmᵀHδm이다. |δm|²≤S||δa||²_w이므로 최대 Rayleigh quotient는 S|χ||m|² 이하이고, 모든 δa_j가 H의 양의 고유벡터와 같은 방향인 경우 등호가 성립한다. 주파수별 iω_j는 weighted skew이며 damping의 symmetric part는 −κI다. 이로써 공식을 얻는다. 이 정리는 전체 real Jacobian을 조립할 필요가 없다.

**결론 T1c.** Y∈C¹([0,h],Cⁿ)는 Y(0)=â(0)를 만족하는 실제 출력용 재구성 곡선이며 R(t)=Y′(t)−f(Y(t))의 연속시간 weighted bound r(t)가 검증되었다고 하자. 재구성 곡선의 mean M_Y=Σw_jY_j에 대한 상한도 검증한다. 다음 값을 둔다.

\[
 B_m(t)=\max\{\sqrt{SE_+}e^{-\kappa t},\ |M_Y(t)|\},
 \quad\lambda(t)=-\kappa+S|\chi|B_m(t)^2,
 \quad A(t)=\int_0^t\lambda(s)ds.
\]

그러면

\[
 \|a(t)-Y(t)\|_w
 \le e^{A(t)}e_0+\int_0^te^{A(t)-A(s)}r(s)ds.                 \tag{T1}
\]

증명: segment a↔Y에서 mean의 norm은 양 끝의 최대 이하다. Fundamental theorem of calculus로 f(a)−f(Y)의 inner product를 segment Jacobian들의 적분으로 표현하고 T1b를 적용하면 D⁺||a−Y||_w≤λ||a−Y||_w+r. 적분인자를 곱하면 T1이다. e=0인 시점에는 upper Dini derivative를 사용한다. 상수 상한 λ,r이면

\[
 e(h)\le e^{\lambda h}e_0+h\varphi_1(\lambda h)r,
 \quad \varphi_1(x)=\begin{cases}(e^x-1)/x,&x\ne0,\\1,&x=0.\end{cases}
\]

이 식에는 hL<1 제약이 없다. 출력 mean과 개별 mode에는 각각 √S e(t), e(t)/√w_j가 유효한 상한이다. 작은 w_j의 mode는 기존 componentwise Duhamel certificate와 함께 최소값을 취할 수 있다. 비교하는 두 해 모두 초기 에너지 E_+ 이하이면, κ=0에서 흐름 gain은 exp(S²|χ|E_+h)이며 통상 Lipschitz gain exp(3S²|χ|E_+h)의 지수를 정확히 1/3로 줄인다. κ>0에는 gain exp(−κh+S²|χ|E_+θκ(h)), θκ(h)=(1−e^{−2κh})/(2κ), θ₀(h)=h를 얻는다. κ≥S²|χ|E_+이면 이 gain은 모든 h≥0에서 1 이하다.

**직접 개선량과 비용.** S=E_+=|χ|=h=1이면 같은 초기 perturbation에 대한 증명 gain은 e³에서 e로 감소한다. 초기 tube 계산은 O(n), 한 window의 scalar gain 계산은 O(1)이다. 기존 JVP/Jacobian 없이 계산한다. 허용 residual 예산 증가와 step rejection 감소는 같은 residual·출력 계약에서 비교한다. native wall time은 아직 측정하지 않았다. 현재 `packet_trajectory.rs`의 radius transfer를 이 정리로 교체하려면 연속시간 재구성 residual 및 outward arithmetic이 먼저 구현되어야 한다. 현재 계산된 candidate mean P가 재구성 mean M_Y와 다르면 이 증명에서 P를 M_Y 대신 쓸 수 없다.

## T2. Bernstein 변환으로 전 구간 certificate를 줄이는 정리

실수 τ∈[0,1] 위의 complex/vector polynomial p(τ)=Σ_{j=0}^d c_jτ^j와 임의 norm을 생각한다. 그 Bernstein coefficients는

\[
 b_i=\sum_{j=0}^i c_j\frac{{i\choose j}}{{d\choose j}},
 \quad p(\tau)=\sum_{i=0}^d b_i {d\choose i}\tau^i(1-\tau)^{d-i}.
\]

**결론.** exact arithmetic의 전 구간 상한 U_B=max_i||b_i||는 coefficient bound U_C=Σ||c_j|| 이하이며 sup||p||≤U_B≤U_C이다. c_j가 outward interval이면 interval 변환과 coefficient norm의 outward 상한으로 sup||p|| 포함성이 유지된다. 반올림된 새 상한과 기존 검증 상한의 minimum을 반환하면 기존 상한 이하라는 계산상 dominance도 보존된다. dyadic de Casteljau 분할의 cell별 상한을 취하면 exact arithmetic에서 상한은 단조 감소한다. d=0이면 M_1=0, d≥1이면 M_1=d max_i||b_{i+1}−b_i||라 하면 폭 δ인 partition에서

\[
 \sup\|p\|\le U_{B,\delta}\le \sup\|p\|+\delta M_1.
\]

증명: Bernstein basis는 비음수이고 합이 1이므로 convex hull 포함성을 얻는다. 이항계수 비는 [0,1]에 있으므로 각 b_i의 norm은 U_C 이하다. de Casteljau는 기존 control points의 convex combination을 만들므로 refinement 상한은 증가하지 않는다. derivative polynomial의 control points는 d(b_{i+1}−b_i). 폭 δ subinterval로 제한한 derivative control points 역시 기존 derivative control points의 convex combinations다. 그 cell의 연속 control points 차이는 δ/d배 derivative control point이므로 임의 control point와 cell 왼쪽 endpoint p(a)의 거리는 δM_1 이하이다. 따라서 위 오차 한계를 얻는다. finite interval arithmetic에서는 outward 반올림 폭을 추가로 유지하고, 이전 단계의 상한과 새 상한의 minimum을 저장하면 단조성도 보존한다.

**실제 이득.** p(τ)=(1−τ)^d에 대해 U_C=2^d, U_B=1이다. 현재 `packet_window.rs::poly_bound`의 coefficient sum을 적응적으로 이 변환으로 보강하면 candidate-domain 또는 full residual bound의 cancellation 때문에 발생하는 허위 거절을 줄일 수 있다. 예를 들어 d=12이면 정확한 상한을 4096에서 1로 줄인다. basis 변환 O(d²), 각 이분 O(d²), streaming memory O(d). 비용이 싼 기존 bound가 이미 통과하면 변환을 생략하고, 통과 여부를 바꿀 수 있는 경우에만 적용한다. 이 theorem의 곡선과 residual coefficient는 point samples가 아니라 전 구간 polynomial이다. 원래 convolution residual의 tail 모델은 그대로 재사용한다. T1에 연결할 때에는 실제 재구성 Y의 differential residual을 interval polynomial algebra로 계산한다.

## T3. Damped moment의 정확한 입력 포함성과 beta-prefix 비용 감소

\[
 J_{j,k}(z)=\int_0^1 e^{-zv}v^j(1-v)^k\,dv,\quad
 I_{j,k}(h,\kappa)=h^{j+k+1}J_{j,k}(\kappa h),\quad j,k\in\mathbb N_0.
\]

여기서 κ,h는 API가 받은 binary64 숫자의 정확한 실수값이다. z를 nearest-rounded 점값으로 치환하지 않고 interval point(κ)·point(h)로 포함한다. j,k≤64, κ≥0, h>0, finite 입력을 허용하고 intermediate overflow는 오류로 반환한다.

**포함성 구성.**

1. beta B(j+1,k+1)의 시작값 1/(j+1)을 point(1).div(point(j+1))로 만들고 양의 recurrence를 outward 실행한다.
2. z_hi≤1에서는 N=28개 Taylor 항을 구간으로 합산한다. Lagrange remainder가 |R_N(v)|≤(zv)^N/N!이므로 integrated remainder는 z_hi^N/N!·B(j+N+1,k+1) 이하이다. 이 remainder와 합의 덧셈도 outward 연산이다.
3. z_hi>1에서는 E_m(z)=∫₀¹e^{-zv}v^m dv의 정확한 identity E_{m−1}=(zE_m+e^{-z})/m를 사용한다. top=j+k+45에서 [0,1/(top+1)]을 outward 생성하고 하강한다. e^{-z}는 기존 `directed::exp_interval`을 −z_hi,−z_lo에서 평가하여 포함한다. J는 Σ_{l=0}^k(−1)^l C(k,l)E_{j+l}; 이항계수도 interval recurrence다.
4. 양성 J≥0와 J≤beta를 교차시켜 tighten하고 h의 정수 거듭제곱도 interval 반복곱으로 포함한다.

각 단계는 정확한 항등식 또는 포함 상계이므로 성공해서 반환한 모든 interval은 정확한 입력의 실제 적분을 포함한다. 큰 z의 하강 recurrence 및 alternating binomial 합은 폭을 크게 만들 수 있다. width gate와 fallback이 필요한 수치성질이며, 이번 수정은 좁은 폭을 무조건 보장한다고 주장하지 않는다.

**새 계산량 정리.** small-z Taylor에서 B_n=B(j+n+1,k+1)는

\[
 B_{n+1}=B_n\frac{j+n+1}{j+k+n+2}
\]

이므로 initial beta를 O(k)로 한 번 만들고 N번 갱신하면 O(k+N) interval 연산으로 모든 Taylor·remainder beta를 생성한다. 기존 구현은 외부 beta 한 번, N Taylor 항, remainder 한 번마다 O(k+1) recurrence를 반복하여 O((N+2)(k+1))였다. N=28,k=64에서는 기존 beta 관련 division 1950개(ordinary seed division 30개+interval division 1920개)가 interval division 93개로 바뀌고, interval multiplication은 1920→92로 줄어든다. 다른 Taylor 및 exponential 연산은 이 숫자에 포함하지 않았다. O(1) 추가 storage이며 정확한 실수 recurrence와 outward inclusion을 보존한다.

**이번 native 변경.** `crates/rodas5p-core/src/damped_moments.rs`에 위 포함성 수리와 prefix recurrence를 실제 적용했다. public/Deserialize config도 compile 시 재검증한다. 새 `damped_moment_enclosure_regressions.rs`는 nonrepresentable 1/3, exact-input product가 1보다 큰 branch crossing, 양 branch의 240-digit 독립 series oracle, config mutation, overflow refusal, subnormal product를 검사한다. 기존 7 tests와 새 4 tests를 대상으로 실행한다. 테스트 상태는 JSON의 실제 마지막 실행을 따른다. 독립 검토가 초기 곡선 일치 조건, d=0 경계, interval-bound dominance, seed division의 종류를 지적하여 위 본문과 비용표에 반영했다.

## 독립 문헌 확인과 적용 범위

SciSpace에서 2026-10-17 UTC에 다음 완전 문장 질문으로 10개 문헌을 찾았다: “Which primary research papers develop rigorous a posteriori continuous-time error bounds for dissipative nonlinear ordinary differential equations using one-sided Lipschitz or logarithmic norms, and how do these improve exponential global error estimates?” 아래 두 원전만 이 작업의 맥락 설명에 사용했다.

- Arnold Neumaier, *Global, rigorous and realistic bounds for the solution of dissipative differential equations. Part I: Theory*, Computing 52 (1994),315–336, DOI 10.1007/BF02276880. Author URL https://arnold-neumaier.at/ms/ode.pdf ; 저자 원전 검색본문에서 logarithmic norm·differential inequality 접근을 확인했다. 예전 Vienna PDF URL은404, 새 URL의 direct open은timeout. 전문 정리를 읽었다고 표시하지 않았다.
- R. H. Nochetto and G. Savaré, *Nonlinear evolution governed by accretive operators in Banach spaces: error control and applications*, M3AS16 (2006),439–477, DOI10.1142/S0218202506001224. Author manuscript https://www.math.umd.edu/~rhn/papers/pdf/accretive.pdf ; 37-page PDF의 abstract·§1–2를 열어 comparison principle 및 accretive-flow a posteriori 접근을 확인했다. 그 논문의 Euler discretization 정리를 packet에 그대로 적용하지 않았다.

T1의 모델별 exact symmetric spectrum, T2의 certificate 적용, T3의 native identity/arithmetic 연결은 위에서 직접 증명했다. 문헌 대비 학술적 최초성은 주장하지 않는다. 과거 Loop12 정리는 재사용했고, 현재 native 변경으로 새로 발생한 의무만 다뤘다. strong no-go 성과는 제출하지 않는다.

## R1–R10 진행 기록과 후속 개발

R1은 최신 production-readiness의 changed-dependency scope를 고정했고, R2는 source와 기존 Loop12 증명을 재사용했다. R3에서 true initial uncertainty·actual stored weights·실제 출력 곡선과 residual을 분리했다. R4는 weighted geometry, Bernstein convex hull, exact interval lifting을 선택했다. R5의 완결 증명은 T1–T3이다. R6은 root가 독립 검토자에게 이 증명 및 변경 코드를 전달하여 채택 여부를 판정한다. Wolfram의 T1 characteristic polynomial 검산은 root가 실행하여 원출력을 보존한다. R7은 새 native regression 4개와 영향받는 기존7개이다. R8–R10의 최종 판정과 publication은 통합 보고서에서 결합한다.

즉시 순서는 (i) 현재 trajectory에서 부정확한 gain 필드 사용·dense restriction·지원하지 않는 κ admission을 수리, (ii) interval coefficient/residual과 초기 E_+·candidate mean bound를 묶어 검증된 local window로 만들기, (iii) T1 gain을 inherited uncertainty에 적용하고 반환 mean/mode와 같은 중심에 붙이기, (iv) T2 refinement는 certificate가 bottleneck인 경우에만 적용, (v) T3 table width·setup 비용을 work ledger에 넣은 뒤 실제 damped packet을 연결하는 것이다. production 승격에는 정확도 기준을 고정한 native full trajectory 및 공정한 기존 solver 비교가 남아 있다.
