# N4B: 두 시간창을 연결한 source-owned floating-carrier 오차 포위

## 정확한 적용 조건과 전제

2026-10-31, **derived / implementation-verified / numerically checked, scoped**. 기존 N4A ZIP SHA-256 `f50c56e1f5330c37e5ad872e6048ad35741fc1b5c735c51dbe4ca93b1619831f`를 불변 부모로 사용했고, native source `n3h_source_bounded_copy.rs`의 원래 **42,406바이트는 byte-for-byte** 보존했다. Rust 1.99.1, RN IEEE binary64, gradual underflow, no fast-math. 정해진 n=12/24/65/257, p=2, h1=h2=1/4, T=1/2, source epoch0→1→2. 원본 initial state, source token, represented float coefficients, all complex initial modes preserved. 

계는 무차원이고
\[
 \dot a_j=(-\kappa_j+i\omega\xi_j)a_j+i\sum_{\alpha=1}^2\chi_\alpha\phi_{\alpha j}|m_\alpha|^2m_\alpha,\quad m_\alpha=\sum_kw_k\phi_{\alpha k}a_k,
\]
\(\kappa_s=1/4,\kappa_f=1000,\chi=(\mathrm{fl}(0.05),\mathrm{fl}(0.02)),\omega=1/2,\phi=(1,\pm1),w_k=\mathrm{fl}(1/n)\). 초기 physical state는 represented source가 정의한 초기점과 일치하며 양의 weight, 실수 chi/phi, source 내 불변 energy carrier를 사용한다. 실수 계수는 **f64에 저장된 정확한 dyadic rational**로 해석한다. `n=65,257`의 represented weight 합이 명목값 1과 조금 다른 효과까지 포함한다. 외부 arbitrary OdeProblem, driver, 시간 의존 반응원에는 이 정리를 이전하지 않는다.

정확한 에너지 항등식은
\[E(t)=\sum_jw_j|a_j(t)|^2,\qquad\dot E=-2\sum_jw_j\kappa_j|a_j|^2\le0.\]

## 첫 번째에서 두 번째 시간창으로의 constructive error transport

표시된 실제 accepted binary64 상태를 \(\tilde y_k\), 정확한 physical state를 \(y(t_k)\), source-frozen exact-real first-Picard candidate를 \(\mathcal P_h(\tilde y_k)\)라고 정의한다. 검증된 scalar complex kernel \(M_q\)의 outward interval, cubic coefficient의 ordered 8×2 convolution 및 homogeneous exponential에 대한 interval arithmetic은 두 시간창 모두 **동일한 원본 N4A 구현**을 사용한다. 이는 damping exponent를 Taylor time stepping하지 않는다.

실제 floating candidate \(\tilde y_{k+1}\)에 대해
\[g_{k+1,j}\ge|\tilde y_{k+1,j}-\mathcal P_h(\tilde y_k)_j|\]
를 각 complex component의 outward interval difference로 계산한다. First-window \(D_{1,j}=B_{1,j}+g_{1,j}\)를 **private N4B receipt**에 source token, accepted epoch, exact-clock bits, every state real/imag bits와 함께 묶는다. N3H `ideal_error` metadata도 N4B-scoped commit 시 이 full envelope로 수정한다. 따라서 다음 window에서는 이전의 optimistic ideal-only error를 사용하지 않는다.

Contraction-box 전제 아래, \(S=\max(1,\sum_jw_j)\),
\[L_{jk}=3S\sum_{\alpha=1}^2|\chi_\alpha\phi_{\alpha j}|w_k|\phi_{\alpha k}|,\quad Q_j=\min(h,\kappa_j^{-1}),\quad q=\max_jQ_j\sum_kL_{jk}<1\]
(κ=0이면 Q=h). N3R의 exact-source first-Picard local defect \(B_{k+1,j}\)를 바꾸지 않으면서, 기존 source-bound Gronwall 논증으로
\[
D_{k+1,j}\le B_{k+1,j}+\frac{D_{k,j}}{1+\kappa_jh}
+\frac{Q_j\sum_kL_{jk}}{1-q}\max_{\ell} D_{k,\ell}+g_{k+1,j}.
\]
를 얻는다. \(e^{-x}\le1/(1+x)\)를 사용했으며 실제 arithmetic에는 outward operations를 적용했다. **두 exact trajectories의 source energy domain, frozen coefficient/time law, 대표될 수 있는 accepted initial state와 source의 동일성**이 필요하다. 식은 온전한 source-conditional error bound이고, 장시간 일반 ODE solver의 무조건적 보증이 아니다.

## 직접 실행·독립 검산

- Rust1.99.1, no external crates. Base 38 + new 5 = **43/43 tests PASS**; strict Clippy `-D warnings` PASS; 새 fragment와 bin rustfmt PASS.
- `n4b_two_window` 실제 출력 716 rows (n=12/24/65/257 × two windows), float endpoint와 exact Picard interval을 직접 저장, source token / epoch / time / trial mismatch fail-closed.
- 독립 `Fraction`은 모든 716행의 exact-represented analytic recurrence 및 출력 metadata를 검사: **716/716 outward upper PASS**. `mpmath` 독립 analytic moment 180/220자리로 second-window **358/358 component inclusion PASS**; 실제 native vs high precision은 최대 6.22e-18 수준. Direct quadrature는 작은 exponential tail에서 상대오차 위험이 있으므로 공식 oracle로 쓰지 않았다.
- 독립 SciPy BDF n=12,24,65는 t=0.25,0.5의 실제 physical ODE를 재적분. 모든 sampled discrepancies가 조건부 bound 안에 들어갔다. BDF는 exact proof가 아닌 별도 numerical check.
- Native original N4A 42,406-byte source prefix와 RODAS5P coefficient fixture 보존. 첫 시간창 n4a carrier 출력은 이전 N4A와 byte-equivalent (동일 float source).
- Adversarial: fake initial dyadic state, different source token, strict tolerance, basis-sign involution, stale second receipt, small-n fallback, accepted/rejected/pending ledger 모두 회귀 테스트 PASS. 최초 작성에서 Rust `.25` literal 오타로 RED가 있었지만 복구했고 계산식은 바뀌지 않았다. 이어서 state 내부 `ideal_error`가 g1을 버리는 약점을 교정해 재검증했다.

| n | first-window max conditional error | second-window max conditional error | second-window max float-gap |
|---:|---:|---:|---:|
| 12 | 2.97316713385434031e-10 | 4.21619408759423723e-10 | 5.61183044478497292e-16 |
| 24 | 2.97316742875733691e-10 | 4.32716657425073029e-10 | 6.10622663543836294e-16 |
| 65 | 2.95678118840842088e-10 | 4.41614813624834392e-10 | 6.38378239159465208e-16 |
| 257 | 2.96906050268850650e-10 | 4.42931625325811389e-10 | 6.47051856539349243e-16 |


### 비용과 엄격한 수용 범위

선별적인 release 비용 3회 반복의 중앙값에서 두-window candidate 생성 대비 실제 source-owned float certificate+commit이 n12/n24/n65/n257에서 약 175/171/172/152배 느렸다. n=6,8은 protected baseline으로 빠르게 skip. **이는 baseline RODAS5P 대비 속도가 아니라 같은 연구 경로 내 offline 인증 추가비용**이다. N4B가 성공해도 원래 PR131 native all-eight adaptive callback, true RODAS5P complete IVP, distinct-author C9, matched-error native wall-time는 수행되지 않았다. 원래 수치 tolerance를 완화하지 않았다.

**상태:** official VigilODE **7/23**, REI **28/88**, successor admitted **0/12**, separate RVJ **3/8 + N4 PARTIAL (N4B scoped-complete)**, C9 other-author **0/1**, C10 NOT_STARTED, PRODUCTION HOLD, MERGE_READY=false, AUTO_NEXT=false.

### 관련 원전

- H.-J. Dobner, *Reliability in concurrent scientific computing for Volterra type equations*, Bull. Informatics and Cybernetics (1999), DOI [10.5109/13476](https://doi.org/10.5109/13476): rigorous posterior enclosures, but our source-specific recurrence independently derived.
- Tobias Jawecki, Winfried Auzinger, Othmar Koch, *Computable upper error bounds for Krylov approximations to matrix exponentials and associated φ-functions* (2018/2019), [arXiv:1809.03369](https://arxiv.org/abs/1809.03369): matrix-exponential defect methods, not our scalar source kernel.
- J.-P. Lessard, *Computing discrete convolutions with verified accuracy via Banach algebras and the FFT*, [source](https://www.math.mcgill.ca/jplessard/papers/rig_FFT.pdf): general future FFT extension, not used here.
