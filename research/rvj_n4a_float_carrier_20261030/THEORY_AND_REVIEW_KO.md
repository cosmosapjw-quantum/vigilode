# VigilODE RVJ N4A | Frozen source에서 실제 첫-window floating Picard carrier의 구성적 구간 포위

2026-10-30 / `derived + implementation-verified, scoped`; 별도 작성자 심사 아님.

## Source, 규약, 고유 기여

원본 N3H/N4가 가진 무차원 두-rate rank-2 계를 변경하지 않는다:

\[
\dot a_j=(-\kappa_j+i\Omega_j)a_j+i\sum_{\alpha=1}^{2}\chi_\alpha\phi_{\alpha j}|m_\alpha|^2m_\alpha,
\quad m_\alpha=\sum_j w_j\phi_{\alpha j}a_j,
\quad \Omega_j=\omega\xi_j.
\]

`kappa_s=1/4,kappa_f=1000,omega=1/2,chi=(fl(0.05),fl(0.02))`, `phi=(1,±1)`, `w_j=fl(1/n)`와 initial carrier의 모든 re/im는 실제 binary64로 표현된 dyadic rational을 정확한 실수로 해석한다. 조건은 `n in (12,24,65,257)`, `h=1/4`, `p=2`, `epoch=0`, 실제 frozen `Source`/`Session`/`Attempt`의 token/trial/clock/epoch 적합성과 `E(0)<=1`이다. 시각 `t=0`에서 initial exact physical state를 accepted represented carrier와 동일하게 가정한다.

Source의 free moment를 감쇠군 `c=0,1`에 따라 분리한다:

\[
P_{\alpha c\ell}=\sum_{j\in c}w_j\phi_{\alpha j}a_j(0)\frac{(i\Omega_j)^\ell}{\ell!},\qquad
\widetilde m_\alpha(s)=\sum_c e^{-\kappa_c s}\sum_{\ell=0}^pP_{\alpha c\ell}s^\ell.
\]

그 결과 (순서 및 basis involution을 보존하여)

\[
H_{\alpha cdeq}=\sum_{u+v+w=q}P_{\alpha cu}\overline{P_{\alpha dv}}P_{\alpha ew},
\]

\[
\widehat a_j(h)=e^{(-\kappa_j+i\Omega_j)h}a_j(0)
+i\sum_{\alpha,c,d,e,q}\chi_\alpha\phi_{\alpha j}
H_{\alpha cdeq}\,M_q(\kappa_j,\kappa_c+\kappa_d+\kappa_e,\Omega_j,h),
\]

\[
M_q(a,b,\theta,h)=\int_0^h e^{(-a+i\theta)(h-s)}e^{-bs}s^q\,ds.
\]

N4에서 검증된 scalar damped oscillatory interval `M_q`를 재사용한다. 이번 N4A는 `P` 구성, complex conjugation, 8 cluster triple의 순서를 유지한 `H` 다항 convolution, complex prefactor, homogeneous exponential 및 최종 summation **모두**를 directed binary64 interval으로 승격했다. 8개 `(c,d,e)` 항의 damping 값이 같아도 원래 coefficient는 합치지 않는다. Kernel은 각 mode의 8개 cluster key로 cache하며 alpha는 재사용한다. 나중의 최적화 작업으로 coefficient regrouping 가능하나 물리 source identity를 재검증해야 한다.

`exp(iθh)`는 real phase에 대한 18차 Taylor, integral remainder `|θh|^19/19!`로 둘러싼다. `M_q`의 phase remainder도 `|θ|^19 J_{q+19}/19!`; 모든 실제 계수는 outward interval multiplication/summation으로 누적된다. **Gamma-function 직접 차분이나 큰 damping의 Taylor time-stepping은 사용하지 않는다.** 지원 source box 밖은 fail-closed.

## 두 종류의 오차를 분리한 정리

`Z_j`를 정확한 실수 의미론의 첫 Picard 후보 `\widehat a_j(h)`를 포함하는 계산된 복소 직사각형 구간, `\widetilde a_j`를 실제 N3H `one_picard`가 반환한 represented binary64 complex 값으로 둔다.

\[
 g_j=\max\{|\widetilde a_j^R-Z_j^R.lo|,|\widetilde a_j^R-Z_j^R.hi|\}
 +\max\{|\widetilde a_j^I-Z_j^I.lo|,|\widetilde a_j^I-Z_j^I.hi|\}
\]

를 모두 outward addition/subtraction으로 계산하면
\[
|\widetilde a_j-\widehat a_j|\le g_j.
\]

N3R가 별도로 유도해 보유한 represented-source **ideal first-window physical error** `B_j`의 선행 증명이 이 exact source 조건에 유효하고, 초기 physical state가 frozen initial carrier에 대해 정확하다는 전제 아래,
\[
|a_j(h)-\widetilde a_j|\le B_j+g_j
\]
이다. `B_j+g_j`도 outward 상계로 계산한다. **이는 조건부 원래 물리 해의 첫 window 오차 진술이다.** 이전 epoch의 native floating carrier error `g`를 다음 window에 입력하는 연결 구현은 수행되지 않았고, N3H 기존 research controller는 여전히 ideal-only error로 판정하므로 `epoch>0` certification은 적극 거부한다.

## 실제 실행 결과

* 4 크기 `n=12,24,65,257`에서 총 **358/358** native complex carrier 구간 포함. 고정밀 `mpmath` 180자리 및 별개 220자리 source-exact analytic moment recurrence도 동일 입력으로 계산: **358/358 PASS**. 최대 precision disagreement `1.60e-182` 미만.
* 전체 native state와 독립 고정밀 analytic source 값의 최대 L1 차이 범위는 약 `8.6e-18`에서 `1.63e-17`. Conservative interval gap 최대 `5.76e-16,6.05e-16,6.24e-16,6.28e-16` (n 순서).
* N3R ideal error와 조건부 합한 첫-window physical error 상계의 최대값 `2.97317e-10,2.97317e-10,2.95678e-10,2.96906e-10`. 기존 1e-8 source budget보다 작지만 이것이 production acceptance는 아니다.
* Native Rust 1.99.1 full test **38/38 PASS**, no-failure strict Clippy and scoped rustfmt. Source substitution negative, stale epoch negative, basis involution, odd initial modes, original N4 240 Fraction kernel 포함. 두 처음 실패는 tail interval 0의 outward round가 `-minsubnormal`을 생성해 positive padding이 거부한 **implementation defect**였고, nonnegative analytic tail의 `clamp_nonnegative`를 올바르게 적용했다.
* 동일 release runner에서 proposal과 optional certification 비용을 5번 직접 재측정: n12 `0.119ms / 22.735ms`; n24 `0.224ms / 40.137ms`; n65 `0.613ms / 99.908ms`; n257 `2.722ms / 480.314ms`. 이 수치는 source-specific single-process 후보 vs 별도 enclosure 추가비용이고 **163~192배 certification overhead**이며, end-to-end original adaptive RODAS speedup이 아니다.

## 동일 작성자 적대적 심사 결과

| 위험 | 실제 검사 또는 한계 | 판정 |
| --- | --- | --- |
| `w=fl(1/n)`를 nominal rational로 둔갑 | represented f64를 exact rational로 변환한 180/220-digit 독립 oracle, source bound 재사용 | scoped PASS |
| θ=0 / 0-tail의 negative subnormal | 기존 RED 재현, nonnegative mathematical tail에만 clamp, 새 unit GREEN | scoped FIX |
| alpha/c/d/e의 duplicate damping 잘못 합산 | cluster ordered 8-term map, basis global sign involution과 independent 358-channel oracle 비교 | scoped PASS |
| 동일-source trial이 아닌 후보 섞기 | source token/trial/epoch/clock mismatch test로 거부, active trial 상태 보호 | scoped PASS |
| 다음 window의 미전파된 binary64 rounding | `epoch>0` 새 certificate를 적극 거부, original controller 변경 안 함 | BLOCKED |
| small ODE speed 주장 | 실제 certificate 추가비용 163~192x; opt-in offline validation만 | negative result RETAIN |
| PR131 native all-eight production 및 adaptive admission | 이번 N4A는 source-owned 연구용 N3H one-Picard 한 window만 검사; original PR131 native code를 실행했다고 주장하지 않음 | NOT_RUN |
| formal distinct-author R6/C9 | 같은 assistant의 적대적 검사, 다른 작성자/계정 reviewer receipt 없음 | 0/1 NOT_RUN |

## 결론과 DAG

N4A는 원래 N4의 kernel-only 상계와 N3H의 실제 첫-window `f64` carrier 사이를 처음으로 수치·구성적으로 연결한 source-specific 실험이다. 기존 N4의 stage eight raw-U reference / transaction은 변경하지 않았다. **공식 VigilODE `7/23`, REI `28/88`, successor `0/12`, RVJ `3/8 + N4 PARTIAL`, C9 `0/1`, C10 NOT_STARTED, production HOLD**. 실제 N3H의 둘째 window inherited floating error, 원래 PR131 true callbacks, matched achieved-error adaptive speed가 N4의 미완료 조건이다. 새로운 이론의 선행 문헌 우선권은 주장하지 않는다.

## 관련 1차 문헌

* T. Jawecki, W. Auzinger, O. Koch, *Computable upper error bounds for Krylov approximations to matrix exponentials and associated φ-functions*, arXiv:1809.03369 (2019 version): https://arxiv.org/abs/1809.03369 (defect 방식의 근거, 현재 direct scalar kernel과 동일한 알고리즘은 아님).
* J.-P. Lessard, *Computing discrete convolutions with verified accuracy via Banach algebras and the FFT*, https://www.math.mcgill.ca/jplessard/papers/rig_FFT.pdf (추후 verified FFT 확장에 대한 방법론적 참고, 현재 N4A는 FFT를 사용하지 않음).
* J. Huang, Z. Cen, A. Xu, L.-B. Liu, *A posteriori error estimation for a singularly perturbed Volterra integro-differential equation*, Numer. Algorithms 83 (2020), https://doi.org/10.1007/s11075-019-00693-y (다른 source에 대한 정리이므로 무단 이전하지 않음).
