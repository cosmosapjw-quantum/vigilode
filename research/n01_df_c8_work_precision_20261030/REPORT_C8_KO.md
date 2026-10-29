# VigilODE N01 C8: 실제 8-stage 원잔차, 수치 안정성 및 derivative-free work–precision

**판정:** `PASS_SCOPED_C8_NUMERICAL_WORK_PRECISION`, 원래 production `HOLD`. 2026-10-30 KST. 연구는 PR123(원래 raw-U C7) 위에 쌓인 [Draft PR125](https://github.com/cosmosapjw-quantum/vigilode/pull/125)의 C8만 수행한다. 기본 solver, RODAS 계수, JVP fallback, 허용오차, step controller를 바꾸지 않았다.

## 1. Source와 과학적 목표

- 원래 아키텍처: causal·parallelized·**derivative-free 우선**, Rosenbrock/Runge–Kutta + accelerated Newton-free homotopy. PR91의 `rhs_homotopy`를 실제 derivative-free BE 보조 stage에 사용하며, JVP는 optional fallback이다. **Dense 3×3 Jacobian은 원래 RODAS5P의 비교 oracle에서만 사용했다.**
- Source: Robertson형 3성분 `f=(-k1 a+k2 b c,k1 a-k2 b c-k3 b²,k3 b²)`, nonnegative rates, mass=1, identity mass matrix, autonomous proper time in this experiment. BASS physical33 원본과는 다른 제한된 source다.
- RODAS5P 원래 8-stage tableau snapshot SHA256 `211255f6125b648c6955833c3da537d9c031ec78168b2918563c29fa88c8bce6`; 상위 PR123 HEAD `2d3d1ef3dce11fbc3c9168bfc69ab16240e40dc6`.
- 기존 C7 사실: BE의 정확한 root가 RODAS 첫 raw-U stage 해라는 결론은 거짓이며, 제조 반례의 잔차는 `(5/65536,-35/131072,25/131072)`이다. 따라서 C8도 BE 보조 stage의 성공을 RODAS8 admission으로 옮기지 않았다.

## 2. 새 수학적 결과: 질량 보존 ≠ ℓ¹ 수축

정확한 Jacobian은

\[
J=\begin{pmatrix}-k_1&k_2c&k_2b\\k_1&-k_2c-2k_3b&-k_2b\\0&2k_3b&0\end{pmatrix}.
\]

`1ᵀJ=0`이지만 `J[2,3]=-k2*b<0`이므로 Metzler 성질이 없다. 따라서 `μ₁(J)=max_j(J_jj+Σ_{i≠j}|J_ij|)=2k2*b`이다. Wolfram 정확식과 256개의 독립 Fraction 3×3 사례가 일치한다. `k=(1/2,1,1/2), b=c=1/4, τ=2`에서는 `||(I−τJ)^−1||₁=19/15>1`. **무조건적인 비팽창 resolvent는 사용할 수 없다.** 이 문제는 물리적 질량 보존 자체를 부정하지 않는다.

## 3. 실제 original raw-U에 대한 조건부 algebraic stage certificate

Source의 허용된 양의 tube에서 `z≥0, Σz≤M_tube`이고 모델/시각/coeff identity가 고정되었다고 가정하자. 정확한 source derivative의 `ℓ¹` 유도 노름은

\[
\|J(z)\|_1\le B=2\max\bigl(k_1,(k_2+2k_3)M_{\rm tube}\bigr).
\]

`τ=hγ>0` 및 **`τB<1`**일 때만 Neumann 상계 `κ=||(I−τJ(y))^{-1}||₁≤1/(1−τB)`를 사용한다. 이것은 충분조건이지 필요조건이 아니다. RODAS 원래 8-stage 원방정식 잔차를 **반올림된 후보 U와 tableau bits를 exact-real 입력으로 읽어** `rodas5p_core::directed::Interval`로 outward 재평가한다. 

\[
\rho_i\ge\|r_i\|_1,\quad
D_i=\kappa\rho_i,\quad
L_{ij}=\kappa\bigl(|\gamma C_{ij}|+\tau B|A_{ij}|\bigr)\ (j<i),
\]
\[
e_i\le D_i+\sum_{j<i}L_{ij}e_j,
\qquad \|\Delta y_{\mathrm{algebraic}}\|_1\le\sum_i|b_{{\rm code},i}|e_i+\epsilon_{\mathrm{assembly}}.
\]

반올림 불확실성은 outward 연산으로 포함하고, 각 진짜 stage 인자가 계산된 후보·이전 stage 오차로 구성한 양의 tube 내부에 들어가는지 선행 방향으로 검증한다. 8×8 `L`은 strictly lower이므로 `I+L+...+L⁷`가 정확한 유한 전달 구조다. 이번 research tube의 상한 `M_tube=1.1*M`은 **사전 선언된 제한된 분석 영역**이며 원래 BASS/physical33의 source-domain 인증이 아니다.

**매우 중요한 의미 차이:** 위 식은 `exact solution of the same discrete 8-stage algebraic equations`와 계산된 stage 사이의 오차 상계다. 실제 시간적분의 truncation error, full-IVP global error, dense/event query의 오차 상계가 아니다. 원래 K/native representation과 raw-U의 차이는 **원래 raw-U 원잔차를 직접 재평가**하여 8-stage source-bound 계산에 반영하고, 마지막 실제 native K endpoint와 raw-U 출력 표현의 차이를 `ε_assembly`에 포함한다. Source outside tube·margin 불성립·callback 실패 시 미승격한다.

## 4. 실제 Rust1.94.1 source-matched 수치 실험

[GitHub Actions run 40952770745](https://github.com/cosmosapjw-quantum/vigilode/actions/runs/40952770745), 소스 `a20a9f6c6a8a60e19ea1f46f0dcab50c1b139125`, 실제 native C8 2/2, C7 3/3, C6 6/6, PR91 29/29 = **40/40 PASS**, strict Clippy/focused rustfmt PASS, 독립 Fraction 256/256 PASS. Source-matched 원자료 38개 정상 격자·12개 거절 격자 및 모든 시도 회계는 Actions artifact `12931054380`의 원시 stdout, CSV, JSON으로 보존했다. 이 artifact를 다운로드해 `parse_native.py`로 **CSV SHA256 `468b2db7...` 및 summary SHA256 `a71d7873...` byte-identical** 재생성했다.

각 모델은 `y0=(.75,.125,.125)`이고 엄격한 SciPy Radau/BDF 각각 `rtol=5e-13, atol=1e-14`의 같은 물리 시스템을 참조한다. 외부 기준해 상호차의 `ℓ∞`는 mild `2.458e-13`, moderate `1.725e-12`, stiff `4.626e-13`으로, **정확한 IVP 오차 보증이 아닌 진단치**다.

| Source / 종료 T | 경로 | 비교 지점 | 실제 관측 endpoint ℓ¹ 오차 | 작업 회계 |
|---|---|---|---:|---|
| mild (.5,1,.5), T=.1 | RODAS5P direct n3 oracle | n=8 | `2.07056594e-14` | RHS 64, Jacobian 8, linear solves 64 |
| mild | BE Picard | n=8 / 512 | `3.20385863e-4` / `5.02873615e-6` | scalar map 48 / 1536 |
| mild | BE homotopy | n=8 / 512 | `3.20385858e-4` / `5.02873615e-6` | scalar map 48 / 1536 |
| moderate (.5,10,200), T=.02 | RODAS5P direct n3 oracle | n=8 | `9.63953362e-10` | RHS 64, Jacobian 8, linear solves 64 |
| moderate | BE Picard | n=8 / 512 | `2.49879683e-3` / `4.09686768e-5` | scalar map 208 / 2560 |
| moderate | BE homotopy | n=8 / 512 | `2.49879678e-3` / `4.09683196e-5` | scalar map 64 / 2048 |
| very stiff (.04,1e4,3e7), T=.001 | RODAS / BE Picard / BE homotopy | n=8–64 | **모두 거절(12행)** | source-domain / 128-call budget / 물리 gate |

온건 RODAS n8의 관측 오차는 Radau–BDF **기준해 상호차보다도 작다**. 따라서 이 영역에서 높은 차수나 더 작은 정확도 보장을 수치값만으로 주장하지 않는다. Moderate는 RODAS가 n8에서 이미 약 `9.64e-10`까지 도달했고 BE n512는 `4.10e-5` 부근이다. 

동일 물리 BE stage의 n8, moderate에서 `ResidualDominance` homotopy가 Picard의 208 map 평가를 64로 줄였지만, mild n8은 48 vs 48이다. **이는 scalar map 평가 절감이지 wall-time이나 생산 solver의 보편적 가속률이 아니다.** Dense 3×3 풀이·Jacobian 비용과 scalar map의 벡터 단위를 합산하지 않았다. RODAS는 `force_accept=true`로 진단적 fixed-grid 진행했으며 적응적 물리 step admission과 구분한다.

C8 source owner는 각 BE step마다 원본 rate를 소유하는 새 trial을 구성했지만 full trajectory 전체의 atomic clock/epoch/callback provenance를 검증하지 않았다. 강직성 negative control은 원래 RODAS가 음수 stage에서 source-domain을 거절했고, BE Picard는 평가 예산, homotopy는 원잔차/물리 stage gate를 거절했다. 이들 상태를 누락하거나 자동 fallback으로 가리지 않았다.

## 5. 보고/검증 한계와 다음 C9

- **완료:** 제한된 3성분 source에서 원래 8-stage outward algebraic 원잔차/조건부 증폭, 같은 물리 source 기준 fixed-grid work-precision, 실제 hosted native 테스트, Fraction 독립 수학 oracle, RED 실패 보존.
- **미완료:** 실제 derivative-free 8-stage *production* solver, 넓은 물리 모델의 JVP-free source-owned operator/clock·tube, 엄밀 global-IVP/reference enclosure, adaptive matched achieved-error with all controller work, parallel speedup, independent-author C9, BASS physical33/whole-slab CMB source, 기존 전체 authority/CI.
- **다음 단계 C9만:** 수학, 원래 raw-U source/callback/epoch, tube 검산, source specificity, 금지된 비교 단위/원격 CI 회계의 독립 공격 검토를 수행하되, 실재 native 별도 검토자가 없다면 `NOT_RUN`으로 기록한다. C10이나 생산 승격을 자동 실행하지 않는다.

공식 원래 VigilODE **7/23**, REI 중앙 **28/88**, successor admitted **0/12** 유지. C8 보조 연구 **1/1 scoped**, production `HOLD`.

선행 문헌 맥락: [Söderlind (2006), logarithmic norm](https://doi.org/10.1007/s10543-006-0069-9), [Hairer–Wanner, stiff ODEs](https://doi.org/10.1007/978-3-642-05221-7). 이번 결과는 이 정리들의 일반적 신규성 주장이 아니라 **source-specific numerical verification**이다.
