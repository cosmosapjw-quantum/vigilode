# VIGILODE-SYM-01: defect 기반 연속 ODE 이벤트 기호 검증 (2026-10-29)

## 연구 경계 및 원천

이 작업의 입력 HEAD는 `cosmosapjw-quantum/vigilode` PR98 `bf308589e21b524d0ddc6e571b260f5e94a853f0`이다. 원래 VigilODE solver의 23-node/34-edge DAG, 8 suites/73 paths/10 interactions, PR95·99와 별도의 V8 physical source, 실패·거부 자료, correction budget은 수정하지 않는다. PR98의 `DYNAMICS.md`에서 보호된 `prepare/observe` 상태기계의 word-preserving buffer-swap proof는 *solver controller* 정리이고, 물리 연속 궤적의 symbolic abstraction은 아직 별개라는 점을 그대로 따른다.

**선택한 신규 보조 노드:** `VIGILODE-SYM-01`, *scalar affine event-sign oracle*. 실험을 위해 좁게 고른 선형 수축 예제이며 일반 비선형/비정규/stiff vector IVP로 승격하지 않는다. 목표는 exact-rational testbed에서 원래 방정식의 잔차로 **시간 전체의 상태 오차**를 감싸고, 관측 임계값의 기호 `POS/NEG/UNRESOLVED`와 이벤트 횡단 횟수에 대해 거짓 양성을 방지하는 것이다.

## 정의와 가정

시각 `s∈[0,h]`, `h>0`, 단일 물리 clock에 대해

\[
  y'(s)=-\lambda y(s)+b,\quad \lambda>0,\quad
  g(y)=y-\vartheta,\quad e_0=|p(0)-y(0)|\le E_0.
\]

여기서 `b, λ, h, θ, E0`는 명시적 유리수이고 `p(s)=Σ_{k=0}^d a_k s^k`는 같은 clock에 정의된 유리계수 represented continuous candidate다. 전 구간에 정확한 solution이 존재하고 초기 discrepancy 조건을 만족한다고 가정한다. 물리 단위는 `[λ]=1/time`, `[b]=[y]/time`, `[r]=[y]/time`, `[E]=[y]`; 코드 예제의 h, λ, b는 무차원화된 toy-model 단위다.

원래 방정식 잔차는

\[
 r(s)=p'(s)+\lambda p(s)-b.
\]

`t=s/h`로 놓고 `r(ht)=Σ_{k=0}^{d}\beta_k B_{k,d}(t)`라 하면 다항식의 power-to-Bernstein 변환은

\[
 \beta_k=\sum_{j=0}^{k} a_j^{(r)}h^j\frac{\binom kj}{\binom dj},\quad
 B_{k,d}(t)=\binom dk t^k(1-t)^{d-k};\quad
 \delta=\max_k|\beta_k|\ge\sup_{s∈[0,h]}|r(s)|.
\]

이 식의 모든 합·곱·나눗셈을 `fractions.Fraction`으로 계산하여 *represented polynomial*의 실수 연산을 정확히 수행한다. 실제 CPU binary64 호출/입력의 rounding, effective source rate, parameter uncertainty는 포함하지 않는다.

## 정리 1: 전 구간 상태 오차 상계

`e(s)=p(s)-y(s)`면 `e'=-λe+r`; Dini 비교로

\[
 |e(s)|\le E(s)=e^{-\lambda s}E_0+\frac\delta\lambda(1-e^{-\lambda s}),\qquad0\le s\le h.
\]

`z=λh`, `F=hδ`에 대하여 `e^{-z}≤(1+z)^{-1}`, `e^{-z}≥(1-z/2)/(1+z/2)`가 성립한다. 전자는 `E_0-δ/λ≥0`, 후자는 그 반대일 때 올바른 endpoint upper를 준다. 양 branch를 무조건 취하면

\[
 U_1=\frac{E_0+F}{1+z},\quad
 U_2=\frac{(1-z/2)E_0+F}{1+z/2},\quad
 E_h=\max(U_1,U_2),\qquad E_{\mathrm{all}}=\max(E_0,E_h).
\]

증명: `E(s)=δ/λ+(E0-δ/λ)e^{-λs}`는 s에 대해 단조이므로 그 최댓값은 양 끝점 중 하나다. 두 exponential bound의 사용 방향은 `E0-δ/λ`의 부호로 정해지며 `max`가 이를 분기 없이 덮는다. `U1` 단독은 `E0=0,λ=h=δ=1`에서 `1/2<1-e^{-1}`이므로 안전하지 않다. PR97/98에서 유도된 두-envelope 정리를 이 고정 스칼라 모델에 재사용했다. 새 정리의 핵심은 아래 **관측 sign/횡단 판정으로의 연결**이다.

## 정리 2: 이벤트 기호와 유일한 횡단의 충분조건

단일 시각 `s`에서 `p(s)-θ>E_s`면 `g(y(s))>0`, `p(s)-θ<-E_s`면 `g(y(s))<0`; 그 외는 `UNRESOLVED`. 끝점에는 각각 `E0`, `Eh`, 내부 시각에는 보수적 `Eall`을 사용한다. 전 구간 symbol은 `p(ht)`의 Bernstein 계수의 최소/최대로 `p-θ`의 엄격한 분리만을 확인한다.

두 끝점에서 증명된 기호가 반대면 연속성으로 적어도 한 번 횡단한다. 유일성은 별도 충분조건을 요구한다. `p(ht)`의 최소 Bernstein 계수 `m_p`에 대해

\[
 m_p-E_{\mathrm{all}}>b/\lambda
\]

이면 실제 해의 전체 구간에서 `y(s)>b/λ`, 따라서 `y'=-λ(y-b/λ)<0`가 되고 이벤트의 횡단은 단 한 번뿐이다. 대응하는 증가 조건은 `M_p+Eall<b/λ`이다. 끝점이 반대라는 사실만으로 **일반 비선형 ODE**의 횡단 유일성을 주장하지 않는다. 이 모델에서 균일 단조성이 확보되었기 때문에 가능한 판정이다.

`NO_CROSSING`은 ① 전 구간 기호가 확정되거나 ② 양 끝점 동일 기호와 단조성이 확정된 경우에만 반환한다. `UNRESOLVED`는 물리적 이벤트가 없다는 의미가 아니라 검증 근거 부족이다. 일정 threshold의 symbolic itinerary 역시 전 구간을 다수 slab으로 덮고 공통 clock/IC/error carry를 유지해야 하므로 이 단일 slab 실험을 전체 itinerary로 부르면 안 된다.

## 정리 2B: 일반 연속 ODE에 대한 후속 확장 계약 (수학적으로 유도, 코드 미구현)

일반 `x'=f(t,x)`, `x∈R^n`, 연속 candidate `p(t)`, event function `g(t,x)`로 넓히려면 다음이 **독립적으로 인증된 입력**이어야 한다.

1. 고정 induced norm 및 동일 clock에서 원래 방정식 defect `r=p'-f(t,p)`의 전 구간 bound `δ(t)`.
2. convex tube `B(p(t),R(t))`에서 `µ(D_x f)≤m(t)`와 존재성, 자기일관적 first-exit containment. 이에 따라 `E'=m E+δ`의 비음수 비교해가 `||x-p||≤E`를 제공.
3. 같은 tube의 `g`에 대한 Lipschitz constant `L_g(t)`; `g(t,p(t))>L_g E`이면 exact `g(t,x(t))>0`, 음수도 대칭.
4. 이벤트 횡단 유일성에는 `D_tg=∂_t g+∇_x g·f`의 *엄격한 전 구간* signed bound, 예를 들어 `D_tg≤-ν<0`를 실제 tube 위에서 증명. 그러면 exact event value가 strictly decreasing이므로 endpoint opposite signs가 주는 crossing은 정확히 하나.

따라서 `g(t,p)`의 단순 표본 부호, 또는 solver controller label만으로 물리 event itinerary를 증명할 수 없다. grazing event에서 `ν=0`이면 uniqueness·count 보장은 보류. `E`가 tube 밖에 도달하거나 state-dependent norm을 일관되게 이전하지 못하면 sign을 도출하지 않는다. 다중 event surfaces의 비가환 순서, Zeno/accumulation, mode reset/epoch consistency는 추가 조건이다.

위 4개 사항은 기본적인 Dini comparison과 chain rule로부터 **직접 유도한 충분조건**이지, 새로운 문헌상의 보편적 강성 정리라고 주장하지 않는다. 이번 `symbolic_oracle.py`는 그 중 `n=1`, affine autonomous `f`, 선형 `g`에서만 구현·검증한다. 실제 VigilODE physical-symbolic 구현은 새 input provider와 outward-floating interval 계약을 요구한다.

## 정리 3: 연구용 workload 감소 조건

임계값 sign을 결정하기 위해 수치적 root-finder를 먼저 돌릴 필요는 없다. 먼저 exact-rational defect와 error bound로 양 끝점을 평가하고, `NO_CROSSING`은 안전하게 건너뛰고, `UNIQUE_CROSSING`만 구간 이분/roots isolation에 넘길 수 있다. **연산량 감소는 조건부 예측**이며 이것이 실제 VigilODE wall time을 줄였다는 benchmark는 아직 없다. 실제 Rust path의 interval enclosure, binary64 source/rounding·adaptive dense coefficients, event callback/replay·epoch consistency가 완성되어야 통합할 수 있다.

## 대표 실행 (유리수 정밀)

`λ=1,b=0,h=1/8,y(0)=p(0)=1,θ=15/16`, `p(s)=1-s+s²/2`에 대해 `r(s)=s²/2`, `δ=1/128`, `E_h=Eall=1/1088`로 계산한다. `y(s)=e^{-s}`가 정확한 IVP 해이며, `s=1/16`에서는 `POS`, `s=1/15`에서는 `NEG`, `s=33/512`에서는 `UNRESOLVED`로 판정한다. 전 구간 y>0으로 `STRICTLY_DECREASING`이므로 `UNIQUE_CROSSING`. 정확한 threshold crossing `s* = -log(15/16)`는 진단용이고 *오라클은 이 초월값을 입력하지 않는다.*

## 적대적 검토 / claim gate

- `PHYS-MATH`: residual 부호, Dini 비교, 단위, 고정 clock, 초기 discrepancy, Bernstein convex hull, 작은·큰 z 두 branch, 궤적 증가/감소 경계, threshold equality의 미해결 처리를 검사한다.
- `PHYS-MATH-CODE`: exact rational inputs만 수용, coefficient transform, 전 구간 `Eall`, payload label, invalid λ/h, retained source identity, 허위 `NO_CROSSING` negative controls 검사한다.
- `Rounding`: `Fraction` oracle는 외부 binary64 `p,f,λ,b`를 자동 검증하지 않는다. 실제 native floating implementation의 outward interval enclosure가 추가되어야 한다.
- `Full ODE`: 사용자·서버의 제공된 실제 chemical/Thomson/recombination coupled RHS나 source/client binding을 호출하지 않았다. `production=HOLD`, `actual_client_qualified=false`, `whole_domain_certificate=false`.
- `Symbolic`: controller의 finite route-word 동등성과 ODE phase-space approximate bisimulation은 이 결과로 동일시되지 않는다. 후자는 incrementally stable/quantization error, reset invariants, event transversality 등 추가 전제를 가진 독립 문제다.

### 외부 원전·재사용 근거

- Higueras & Söderlind (2002), *Logarithmic norms and nonlinear DAE stability*, DOI 10.1023/A:1021956621531. 로그 노름과 Dini 비교의 배경.
- Pola, Girard & Tabuada (2008), *Approximately bisimilar symbolic models for nonlinear control systems*, DOI 10.1016/j.automatica.2008.02.021. 연속계 approximate abstraction의 incremental stability 전제와 state quantization 구조.
- 기존 PR98 `research/dynamics_symbolic_validation_20261028/theory/DYNAMICS.md` 및 PR97 `research/production_rvj_nag_20261028/theory/CLOSURE.md`: 두-envelope 원래 유도, word-preserving swap와 불변 source scope.

## 단위 완료 판정

`VIGILODE-SYM-01`: 좁은 정리와 코드의 **exact-rational toy oracle**는 유효하다. Rust integration·actual nonlinear client·full state vector·rounding/certificate assembly·observational fitting은 `NOT_RUN/UNRESOLVED`; 원본 production claim은 그대로 `HOLD`. 원래 23개 node의 상태 또는 분모를 증가시키지 않는다.