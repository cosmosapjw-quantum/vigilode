# VigilODE 정확도·속도 개선의 수학 계약과 코드 이식 경로

작성일: 2026-10-10
검토 기준: cfed140bf127f5aa8fcf4fe4c16689d7de062b87
범위: 최신 구현·테스트와 별도 연구 자료의 연결, 새로운 국소 인증 경계, 다음 개발 단계의 수학적 조건.

이 문서는 기존 연구를 재실행한 보고서가 아니다. 최신 ALG04–06, SPD 계열, PP 계열의 공개 기록을 한 차례 대조한 뒤 재사용한다. 새로운 2×2 prototype의 **계약**을 설명하지만, 이 문서 자체는 그 실행 결과나 promotion 판정을 주장하지 않는다. 실행 결과와 독립 검토의 판정은 이번 감사의 별도 결과 파일이 담당한다. 아래 Rust 코드는 명시한 경우를 제외하고 **제안하는 인터페이스 스케치**이며 현재 존재하는 공개 API가 아니다.

핵심 개발 순서는 명확하다. 이미 확인된 중복 작업 제거와 workspace 연결을 먼저 채택 심사하고, 정확도 측면에서는 현재 연산자에 대한 residual-to-error 관계를 명시한다. 그 다음 전체 stage·시간 구간으로 오차를 운반한다. 다항식 기저 교체와 homotopy 병렬화는 이 정확도 계약과 가장 강한 비교 대상 아래에서 평가한다.

## 1. 연구 자료를 통합한 현재 위치

| 경로 | 이미 확보된 증거 | 남은 조건 |
|---|---|---|
| DupFix | native에서 같은 trajectory, small-n JVP 0.63–0.89배 | 실제 caller/default 채택의 focused review |
| SPD04 workspace | native least-squares bit identity와 allocation 제거 | 지원하는 solver caller에 연결, failure/reset 상태 보존 |
| SPD05 LGMRES | library solve에서 중복 least-squares 제거 | 실제 driver 호출이 있어야 solver 효과를 주장할 수 있음 |
| ALG04 결합 target | PDE 사례의 JVP 감소, ladder 결과 | tight strongly nonnormal case의 정확도와 full-space gain |
| ALG05 predictive controller | 새 seed 집합에서도 van der Pol 작업량·거부 감소 | baseline-relative calibration, interior output grid, 수치적으로 안전한 controller 계산 |
| ALG06 guard | 등록 규칙상 PASS | 이미 공개된 h0 perturbation 실패 때문에 robust accuracy는 HOLD |
| PP08 polynomial router | total-budget admission 및 accounting | 79개 중 Laguerre 선택 0; Chebyshev 단독보다 싸다는 근거 없음 |
| PP09 Laguerre envelope | finite-degree scalar envelope 연구 | 구성별 telemetry와 recurrence-adjoint remainder의 실제 병목 |
| PP10 Leja | 72개 fixture의 후보 정확도 | EstimateOnly를 넘어설 완전한 total certificate |
| PP11 fused Taylor | 일반 크기 입력의 유용한 total certificate | 극단적인 RHS 크기에 대한 scale-homogeneous 계산 |
| PP12/PP12b | verified logarithmic norm과 real diagonal metric | 강한 transient growth, metric transport, representability |
| q=2 homotopy | original-target certificate가 있는 opt-in 경로 | 실제 client와 certificate·dispatch·fallback을 포함한 양의 비용 여유 |

별도 연구의 최근 내용은 docs/reviews/20261008_algorithmic_directions/pilot/에도 들어 있다. 이 pilot은 native 신규 알고리즘의 결과가 아니다. 일부 base-arm replica fidelity가 좋아도 새로운 EXPRB, ROCK4, preconditioner의 Rust 결과를 대신하지 않는다. 보관되지 않은 raw JSONL·NumPy reference와 이전 scratch 절대경로도 있으므로 standalone 재현 가능성은 제한적이다.

이전 결과의 FAIL은 지우지 않는다. 새 계약이 필요하면 새 node를 등록하고, 어떤 실패 원인을 바꾸는지 명시한다. 반대로 변하지 않은 공개 실험을 다시 돌려 같은 주장을 확인하는 일은 하지 않는다.

## 2. 오차 예산은 무엇을 대상으로 하는가

서로 다른 다음 네 대상을 한 숫자로 합치지 않는다.

1. **선형 stage solve의 forward error:** 같은 W와 rhs를 정확히 풀었을 때와의 차이.
2. **수치 step의 오염:** inexact stage들 때문에 같은 시작 상태의 정확한 RODAS5P step과 달라진 양.
3. **ODE의 local truncation error:** 정확한 수치 step과 정확한 ODE 흐름의 차이.
4. **전역 오차:** 여러 step을 거쳐 최종 ODE 상태와 달라진 양.

현재 residual certificate prototype가 다루는 것은 첫 번째다. 선형 solve를 인증했다고 5차 전역 정확도, stiffness-uniform order 또는 controller의 global accuracy를 증명한 것은 아니다. embedded estimate 역시 일반적인 전역 오차 상계가 아니다.

목표 데이터도 고정해야 한다. “binary64로 주어진 W의 exact-real 해”와 “exact h·gamma·J로 정의한 W의 해”는 W 구성 반올림이 있는 경우 서로 다르다. JVP가 finite difference라면 callback 계산의 오차가 추가된다. proof·checker·보고서가 서로 다른 목표를 쓰지 않도록 target identity를 먼저 정의한다.

## 3. 현재 가중 residual에서 선형 forward error로

### 3.1 metric과 exact 관계

identity mass를 쓰는 fast U-form에서

$$
W=I-h\gamma J,\qquad
s_k=\mathrm{atol}+\mathrm{rtol}|y_k|,\qquad
D=\operatorname{diag}(s_k^{-1})
$$

라 하자. 이 section에서는 s를 해당 solve 동안 고정하고 모든 성분이 유한한 양수라고 가정한다. 정확한 stage 해 u*와 candidate u에 대해

$$
r=D(b-Wu),\qquad B=DWD^{-1}
$$

이면

$$
D(u^*-u)=B^{-1}r.
$$

따라서 $\|B^{-1}\|_2$의 검증된 상계 G가 있을 때만

$$
\|u^*-u\|_{\mathrm{WRMS}(s)}
\le G\,\frac{\|r\|_2}{\sqrt n}
$$

를 얻는다. 작은 residual이라는 관측만으로 작은 forward error가 따라오지는 않는다.

실제 residual 계산이 exact b와 W를 쓰지 않는다면, 오른쪽에는 residual 계산·rhs assembly·W 형성·JVP 평가의 오차도 포함해야 한다. 특히 approximate operator $\widetilde W$로 계산한 residual에는 $(W-\widetilde W)u$ 항이 남는다. 이 항을 모르면서 certificate라고 부르지 않는다.

### 3.2 projected Hessenberg가 놓치는 방향

$$
B=\begin{pmatrix}1&K\\0&1\end{pmatrix}
$$

에서 Arnoldi 시작 방향이 $e_1$이면 관측된 한 차원 projected operator는 K와 관계없이 [1]이다. 반면

$$
B^{-1}e_2=(-K,1)^T
$$

이므로 관측하지 못한 residual 방향의 amplification은 K에 따라 커진다. 이것은 현재 ALG04 trajectory의 직접 재현을 뜻하지 않는다. **projected matrix의 inverse norm을 full-space inverse norm의 상계로 승격할 수 없다는 정확한 반례**다.

현재 nu-guard는 유용한 진단이나 heuristic tightening일 수 있다. 그러나 이 구분 없이 nu를 error authority로 사용하면 탐색 공간 바깥의 방향을 누락한다.

### 3.3 적용 가능한 verified witness

가능한 witness를 구조별로 제한한다.

| 구조 | 사용할 수 있는 관계 | 비용·제약 |
|---|---|---|
| verified weighted logarithmic norm | $a=h\gamma\ge0$, $\mu_2(DJD^{-1})\le\mu_{\rm up}$, $1-a\mu_{\rm up}>0$이면 $G\le(1-a\mu_{\rm up})^{-1}$ | interval formation과 양의 denominator 확인; symmetric_part_upper 재사용 가능 |
| 작은 dense 또는 선언된 block | verified inverse와 outward residual correction | block 밖 coupling이 없다는 계약 필요; n≤8에는 direct solve 자체가 강한 경쟁자 |
| approximate inverse X | $q=\|I-XB\|<1$이면 $\|B^{-1}\|\le\|X\|/(1-q)$ | X setup·검증 비용이 크면 matrix-free 목적에 맞지 않음 |
| JVP-only, 구조·오차 witness 없음 | certified gain 없음 | heuristic 상태 유지 또는 protected path |

logarithmic norm 관계는 spectrum-only 관계가 아니다. weighted operator와 metric이 실제로 검증돼야 한다. h·gamma<0에 같은 부등식을 그대로 적용하지 않는다. real diagonal metric을 통해 더 좋은 lognorm을 얻더라도 최종 물리적 metric으로 돌아오는 norm transport를 누락하면 안 된다.

nonnormal_certificate.rs의 interval symmetric-part bound와 directed primitives는 이미 있으므로 중복 구현보다 재사용이 우선이다. 다만 exponential semigroup certificate 타입과 resolvent certificate 타입은 분리한다.

### 3.4 이번 2×2 prototype의 범위

등록된 prototype는 represented binary64 W, b, candidate x 및 양의 scale s를 exact-real 입력으로 해석한다. outward interval inverse로

$$
c=W^{-1}(b-Wx),\qquad
E_\infty\ge\max_k |c_k|/s_k
$$

를 감싸며 $E_\infty$는 weighted RMS error도 상계한다. 이는 scalar gain을 따로 추정한 뒤 residual norm에 곱하는 방식보다 직접적인 componentwise correction enclosure다.

현재 W와 scale을 소유하는 immutable object라는 점은 stale witness의 단순 재사용을 줄인다. 하지만 미래 caller가 W를 잘못 구성하거나 다른 model epoch에 적용하는 문제까지 자동으로 해결하지는 않는다.

명시적으로 범위 밖인 것은 다음이다.

- W 구성 이전의 exact h·gamma·J 및 그 반올림;
- finite-difference/JVP uncertainty;
- off-block coupling;
- nonlinear stage transfer와 global time propagation;
- 큰 matrix의 실용적인 runtime gain;
- production/default promotion.

determinant interval이 0을 포함하거나 입력·중간 연산이 유한하지 않으면 거절한다. independent Fraction checker는 같은 interval 알고리즘을 복제하지 않고 exact binary rational 선형계를 풀어 비교해야 한다. 이 section에는 prototype의 통과 여부나 속도 결과를 기입하지 않는다.

## 4. U-form 전체 nonlinear stage로의 오차 운반

### 4.1 source와 일치하는 원래 식

rodas5p_matrix_free_fast.rs의 rhs assembly는 다음과 같다.

$$
W U_i =
h\gamma f\!\left(t+c_i h,\,
y+\sum_{j<i} a_{ij}U_j\right)
+\gamma\sum_{j<i} C_{ij}U_j
+h^2\gamma\gamma_i f_t.
$$

여기서 a, C, gamma_i는 해당 코드의 변환된 U-form 계수다. 다른 K-form tableau의 기호를 섞어 쓰면 안 된다. 같은 frozen J, 시작 상태 y, h, 시간 t와 $f_t$를 쓰는 exact stage를 $U_i^*$, candidate를 $\widehat U_i$라 하자.

고정 metric s에서 stage state를 포함하는 tube $\mathcal T_i$ 안의 모든 v,w에 대해

$$
\|f(t_i,v)-f(t_i,w)\|_{\mathrm{WRMS}(s)}
\le L_i\|v-w\|_{\mathrm{WRMS}(s)}
$$

를 검증했다고 가정한다. exact residual과 모든 assembly·evaluation uncertainty를 포함한 상계를 $\rho_i$라 두자. 이전 stage의 상계를 이미 구했다면, 다음 outward recurrence로 $q_i$를 정의하여 실제 stage 차이의 상계를 얻는다.

$$
q_i =
\operatorname{up}\!\left\{G\left[
 \rho_i+
 \sum_{j<i}
 \left(
 |h\gamma|L_i|a_{ij}|+|\gamma C_{ij}|
 \right)q_j
\right]\right\},\qquad
\|\widehat U_i-U_i^*\|_{\mathrm{WRMS}(s)}\le q_i.
$$

초기 상태와 $f_t$가 같으면 차분에서 그 항은 상쇄된다. callback의 $f_t$ 값이나 시작 상태가 다르면 해당 차이도 rho에 포함해야 한다. 이 부등식은 엄밀한 충분조건의 설계이며 현 coupled target이 이미 이 조건을 계산한다는 뜻이 아니다.

q를 계산한 뒤에도 exact stage state가 사용한 tube 안에 존재한다는 self-consistency 검사가 필요하다. 단순히 candidate 주변 Jacobian 한 점의 값을 L로 사용하는 것으로 대체할 수 없다. 현재 quadratic/diagonal model certificate의 선언 모델과 실제 callback을 연결하는 ModelBinding도 동일한 문제다. sampled agreement는 model identity의 증명이 아니다.

### 4.2 output·embedded error·metric 변화

U-form output이 $\widehat y=y+\sum_i b_i\widehat U_i$이면

$$
E_y \le \sum_i |b_i|q_i + E_{\rm output\ assembly}.
$$

embedded output 역시 실제 coefficient vector로 운반한다. 이 driver에서 마지막 U-stage가 error vector를 담당하는 관계를 사용할 때도 그 vector의 assembly·norm 계산 오차와 metric을 포함한다.

현 stage target의 scale은 atol + rtol*abs(y)다. 일반 error_scale은 atol + rtol*max(abs(y_old),abs(y_new))를 쓴다. 같은 atol/rtol이라면 후자의 scale이 성분별로 더 크므로 stage metric에서 얻은 상계가 그 보고 metric에 보수적으로 운반될 수 있다. 그러나 다른 scale 정의나 다음 step으로 넘어갈 때 이를 자동 가정하지 않는다.

일반적인 metric 변경 $D_{\rm new},D_{\rm old}$에는

$$
\|v\|_{\mathrm{WRMS,new}}
\le \kappa_D\|v\|_{\mathrm{WRMS,old}},
\quad
\kappa_D=\max_k s_{{\rm old},k}/s_{{\rm new},k}
$$

를 사용하고 $\kappa_D$도 outward로 계산한다. 같은 차원의 norm이라는 전제도 필요하다.

### 4.3 sampled tau와 global claim의 경계

현재 rodas5p_stage_transfer_constants()는 scalar transfer function을 imaginary axis의 1500개 양의 점과 0에서 평가한 최댓값이다. 전체 half-plane의 supremum을 엄밀하게 감싸려면 표본 사이, 0 부근과 무한 tail에 대한 추가 bound가 필요하다. scalar transfer의 상계가 있더라도 nonnormal matrix function과 nonlinear tube에 대한 운반은 별도 문제다.

따라서 다음 상태를 분리한다.

- CalibratedStageTarget: 현 tau와 order factor로 정한 실용적 target.
- VerifiedLinearError: 같은 W·rhs·metric에 대한 선형 correction 상계.
- VerifiedStepContamination: 모든 stage dependency 및 output assembly를 포함한 step 오염 상계.

전역 ODE error에는 한 단계 더 필요하다. step-map stability $S_n$, metric transport, local truncation error $\ell_n$, inexact contamination $E_{y,n}$를 포함해 예를 들어

$$
E_{n+1}\le S_n E_n+\ell_n+E_{y,n}
$$

형태의 검증된 recurrence가 있어야 한다. $\sum_n h_n/T=1$이라는 budget accounting만으로 $S_n$이나 $\ell_n$를 없앨 수 없다. 출력 구간별 호출이라면 진짜 integration span인지도 계약으로 명시한다.

## 5. native 타입과 caller 경계의 제안

다음은 인터페이스 설계 스케치다. certificate의 필드·생성자는 비공개로 두고, 외부 JSON은 telemetry로 읽으며 authority object로 역직렬화하지 않는다.

~~~rust
// Proposed interfaces: not current public APIs.
struct FrozenMetric { /* positive finite scales, identity */ }
struct CurrentOperatorId { /* model epoch, J/W bits or checked binding */ }
struct LinearTargetId { /* operator, h, gamma, metric, RHS identity */ }

struct VerifiedCorrection {
    target: LinearTargetId,
    weighted_error_upper: f64,
    /* private provenance and outward arithmetic status */
}

enum StageEvidence {
    Heuristic { target: LinearTargetId, estimate: f64 },
    Verified(VerifiedCorrection),
    Rejected { reason: WitnessRejection },
}

trait CurrentLinearCertificate {
    fn certify_current(
        &self,
        target: &LinearTargetId,
        rhs: &[f64],
        candidate: &[f64],
    ) -> Result<VerifiedCorrection, WitnessRejection>;
}

struct StepContaminationCertificate {
    /* exact tableau target, tube, each stage proof, output bounds */
}
~~~

권장 이식 위치는 다음과 같다.

| 파일·호출 지점 | 작업 | 통과 조건 |
|---|---|---|
| 연구 node의 gain_witness.rs | 현재 2×2 계약의 immutable correction enclosure | exact Fraction oracle, invalid-input rejection |
| 향후 rodas5p-core/src/residual_gain.rs | 일반화할 때 core로 이동 | 목표 identity, directed arithmetic, norm 타입의 API review |
| nonnormal_certificate.rs::symmetric_part_upper | weighted lognorm witness 재사용 | resolvent의 양의 denominator·metric transport 확인 |
| rodas5p_matrix_free_fast.rs::staged_stage_solve | heuristic target과 verified evidence 구분 | true current residual, 불일치·overflow 거절 |
| StageSolveStatistics와 attempt charge | floor/stall/fallback 등 실제 허용된 모든 범주 계측 | requested budget 초과분을 빠짐없이 기록 |
| outward_certificate.rs | nonlinear causal chain/tube 증명 재사용 | 같은 원래 target, output까지 운반 |
| problem.rs | 구조·callback/epoch 계약 | declaration과 sampled check를 구분 |

인증 실패의 protected fallback도 반드시 종료 가능해야 한다. 더 비싼 solve를 반복해도 representability나 JVP noise 때문에 목표에 닿지 않는다면 명시적인 unattainable-budget 상태가 필요하다. 이것을 tolerance를 조용히 높이는 방식으로 처리하지 않는다.

## 6. Laguerre·Chebyshev·Leja와 Taylor의 실제 발전 방향

### 6.1 같은 target에서의 비교

공통 target을

$$
F=\sum_{k=0}^4\varphi_k(hA)w_k,\qquad \varphi_0=\exp
$$

로 고정한다. w_k=h^k b_k 같은 입력 변환을 사용하면 그 변환의 rounding/loss도 별도로 다룬다. 기저별 비교는 같은 exact binary inputs, 같은 total error budget, 같은 certified/estimate-only 등급에서 해야 한다.

Krylov를 다른 polynomial로 바꾸는 것은 두 가지 서로 다른 작업이다.

- **matrix function action backend 교체:** phi/exponential target을 계산하는 기저를 바꾼다.
- **linear solve preconditioner:** exact W를 유지하고 polynomial을 inverse preconditioner로 사용한다.

후자는 Rosenbrock tableau나 J 자체를 근사식으로 바꾸지 않으므로 true unpreconditioned residual로 최종 선형 target을 확인할 수 있다. 다만 residual-to-forward-error 문제는 여전히 남는다.

### 6.2 Chebyshev: 기본 비교 대상과 single-backend 실행

verified symmetric nonpositive domain에서는 현재 certified Chebyshev action이 강한 기준이다. PP08은 두 basis를 모두 실행해 더 싼 admissible result를 선택하지만 Laguerre를 한 번도 선택하지 않았다. router를 개선한다면 다음 순서가 낫다.

1. enclosure·h·budget에서 coefficient/degree 후보를 preflight한다.
2. 하나의 backend만 실행한다.
3. 실제 total admission이 실패할 때만 대체 backend 또는 protected path를 호출한다.
4. preflight, cold/warm setup, 실패한 action과 fallback을 모두 센다.

제안 API는 plan_joint_phi(...) -> ActionPlan과 execute_joint_phi(plan, ...)의 분리다. plan은 certificate가 아니며 실제 result의 admit_total_error를 대체하지 않는다. 비교 대상은 double-run router뿐 아니라 **Chebyshev 단독**이어야 한다.

### 6.3 Laguerre: coefficient envelope보다 먼저 recurrence propagation

Laguerre는 현재 $X=-A/\beta$의 verified nonnegative spectral interval과 degree 제한 아래에서 recurrence와 adjoint total을 계산한다. PP09 이후에는 exp(L/2)를 기존 stiff 실패의 확정적인 주범으로 쓰면 안 된다.

PP09의 공개 데이터는 component 필드를 포함하지 않아 등록된 분해 자체가 불가능했다. 별도의 재구성은 truncation·coefficient 항이 total에 비해 매우 작고, recurrence-adjoint 및 summation remainder가 대부분임을 시사했다. 이것은 telemetry로 확인된 분해와 같은 증거가 아니다.

다음 단계는 작은 새 exporter로 다음 필드를 출력하는 것이다.

~~~rust
// Proposed export shape; total is not recomputed by naive summation.
struct PolynomialErrorComponents {
    truncation: f64,
    coefficient: f64,
    recurrence_adjoint: f64,
    summation: f64,
    fused_summation: f64,
    normalization: f64,
    total_upper: f64,
    upward_aggregation_slack: f64,
}
~~~

그 다음에만 local Taylor/Bernstein subdivision 등으로 **adjoint polynomial의 실제 interval supremum**을 개선한다. degree/scale/operator enclosure/cache identity는 유지한다. scalar Laguerre envelope를 타이트하게 하는 것만으로 recurrence-adjoint total이 개선된다고 가정하지 않는다.

재개 gate는 “새 bound가 작은가”가 아니라 “total admission을 만족하면서 Chebyshev 단독보다 전체 비용이 작은가”다. 이 gate가 실패하면 Laguerre의 연구 상태를 유지하고 자동 dispatch에 연결하지 않는다.

### 6.4 Leja: last-two-term estimate와 인증을 분리

현재 leja_action.rs는 binary64 Opitz divided differences와 Newton recurrence를 사용하며 EstimateOnly다. PP10 fixture에서 estimate가 actual error를 낮게 보고하지 않았다는 사실은 일반 tail theorem이 아니다.

total certificate의 다음 node에는 최소한 다음이 필요하다.

- degree·scaling·interpolation domain의 theorem-backed 선택;
- interpolation/backward error의 forward-error 운반;
- divided-difference 및 recurrence rounding;
- nonnormal이면 spectrum 밖의 amplification bound;
- fused phi target 전체와 입력 변환 오차;
- coefficient setup과 certificate의 비용.

Leja의 관심 영역은 현재 증거상 largest h·rho의 일부 product-count 절감이다. 따라서 처음부터 universal router를 만들기보다 real-spectrum expensive-JVP client에서 좁게 평가한다. certified Chebyshev에 비해 product가 적더라도 coefficient·verification 비용을 합치면 이득이 없어질 수 있다.

### 6.5 fused Taylor: RHS scale을 보존하는 certificate

PP11의 extreme RHS 문제에는 선형성이라는 직접적인 확장 경로가 있다.

$$
F(A,h,\sigma w_0,\ldots,\sigma w_4)
=\sigma F(A,h,w_0,\ldots,w_4).
$$

power-of-two sigma를 골라 $v_k=w_k/\sigma$로 normalize하고 v에 대한 candidate z와 bound E를 얻으면, 최종 candidate $\widehat F=\operatorname{fl}(\sigma z)$에 대해

$$
\|\widehat F-F(w)\|
\le |\sigma|E+
\|\operatorname{fl}(\sigma z)-\sigma z\|
$$

를 사용할 수 있다. normalization이 exact하지 않으면 $w_k-\sigma v_k$의 phi action도 추가로 감싸야 한다.

이 방식은 augmented matrix의 W block이 너무 커지는 문제와 O(1) nilpotent coordinate 때문에 bound가 입력 스케일을 따라가지 못하는 문제를 겨냥한다. 하지만 subnormal 출력의 상대정확도를 무제한 보장하지 않는다. underflow·overflow를 포괄하는 absolute-plus-relative budget과 outward rescaling이 필요하다.

이식 지점은 taylor_phi_total.rs wrapper이며, 기존 exp-only certificate를 fused certificate로 취급하지 않는다. 새로운 disjoint scale fixtures를 등록하고 기존 PP11 72개 campaign은 재실행하지 않는다.

## 7. Homotopy가 sequential dependency를 줄일 수 있는 조건

### 7.1 현재 native 경로와 일치하는 수식

이 section은 위의 fast U-form과 구분하여, block.rs와 homotopy.rs가 사용하는 K-form을 따른다. stage block diagonal을 $\mathcal D=I_s\otimes W$, strictly lower stage coupling을 $\mathcal C$, base rhs를 $b_0$, nonlinear remainder를 N(K)라 쓰면 source의 partial path는

$$
\eta(\theta,\lambda)=\theta+\lambda(1-\theta),
$$

$$
H_{\theta,\lambda}(K)
=(\mathcal D-\eta\mathcal C)K-b_0-\lambda hN(K)=0.
$$

이는 evaluate_partial_path()의 partial - base - lambda*h*remainder와 일치한다. 여기서 의도한 continuation 범위는 theta, lambda가 각각 [0,1]인 경우다. 일반 범위로 확장할 때 아래 contraction 식의 eta에는 절댓값을 적용해야 한다.

- $\lambda=1$이면 $\eta=1$이므로 항상 원래 target $(\mathcal D-\mathcal C)K-b_0-hN(K)=0$이다.
- $\theta=0,\lambda=0$이면 $\mathcal D K=b_0$라서 common-W stage solves가 독립이다.
- $\theta>0,\lambda=0$이면 base에 이미 선형 coupling이 있다.
- round마다 theta를 바꾸는 것은 현재 source 주석처럼 하나의 고정 smooth homotopy curve가 아니라 nonstationary preconditioned sweep이다.

λ의 여러 값을 순서대로 추적하는 continuation 자체는 sequential이다. 단순히 homotopy라는 이름을 붙인다고 병렬성이 생기지는 않는다.

### 7.2 실제로 독립인 작업

frozen K의 각 stage state에서 RHS를 평가하는 일과, 같은 $\mathcal D$에 대해 여러 rhs를 푸는 일은 같은 round 안에서 독립이다. 예를 들어 Picard map

$$
\Phi_{\theta,\lambda}(K)
=\mathcal D^{-1}\bigl(b_0+\eta\mathcal C K+\lambda hN(K)\bigr)
$$

의 Jacobi식 iteration에서는 **이전 round K만 읽도록 고정한** stage별 RHS/solve를 병렬화할 수 있다. 이 round들이 서로 독립인 것은 아니다. 새로운 K를 얻어야 다음 round를 시작한다.

native의 truncated partial inverse는

$$
(\mathcal D-\eta\mathcal C)^{-1}
=\sum_{\ell=0}^{s-1}
(\eta\mathcal D^{-1}\mathcal C)^\ell\mathcal D^{-1}
$$

의 앞 q+1항을 쓰는 구조다. strictly lower block coupling 때문에 power s에서 0이 되는 exact-arithmetic 선형 관계다. 각 level의 rhs batch는 병렬화할 수 있지만 level $\ell+1$은 level $\ell$을 기다린다. q가 커질수록 total solves와 sequential depth가 함께 늘어난다. q< s−1인 truncated inverse가 nonlinear target의 정확한 역연산이라는 뜻도 아니다.

전역 contraction은 수렴의 한 충분조건이지 causal stage 구조의 유일한 분석법은 아니다. 고정 W의 엄밀한 lower-triangular dependence는 stage별로 더 타이트하게 분석할 수 있다. 이를 활용하는 것이 현재 causal/outward certificate의 방향과 맞는다.

### 7.3 contraction과 endpoint certification

적절한 weighted block norm과 invariant tube $\mathcal T$에서

$$
q_\lambda
\ge
\|\mathcal D^{-1}\|
\left(\eta\|\mathcal C\|+|\lambda h|L_N\right),
\qquad q_\lambda<1
$$

가 검증되면 $\Phi_{\theta,\lambda}$는 그 tube에서 contraction이다. candidate $\widehat K$의 preconditioned defect를

$$
\delta_\lambda=
\|\mathcal D^{-1}H_{\theta,\lambda}(\widehat K)\|
$$

라 할 때, candidate-centered ball의 radius r에 대해 $\delta_\lambda+q_\lambda r\le r$이고 이 ball이 tube 안에 있으면 self-mapping을 보장할 수 있다. 따라서

$$
\|K_\lambda^*-\widehat K\|
\le \delta_\lambda/(1-q_\lambda)
$$

를 얻는다. q, delta, tube 경계, inverse 적용의 rounding까지 감싸야 certificate가 된다. pointwise Jacobian correction이나 small residual을 이 조건의 대체물로 쓰지 않는다.

accepted output은 반드시 **λ=1의 original target**에서 인증한다. 중간 λ의 수렴, predictor의 작은 차이 또는 q=2 truncation의 자체 residual은 원래 step의 오차 상계가 아니다. endpoint의 causal certificate가 더 타이트하다면 intermediate path는 candidate 생성기로만 사용할 수 있다.

현재 Q2Admission::OperationalDiagnostic과 NativeTargetCertificate / PreparedStructuredCertificate의 타입·상태 구분은 유지한다. 선언 quadratic/diagonal family 밖에서 callback consistency sample만 맞는다고 rigorous model binding으로 올리지 않는다. K/U 변환을 넘나드는 경로에는 coefficient conversion와 rounding 계약도 필요하다.

### 7.4 parallel work와 span은 따로 계산한다

parallel budget은 두 숫자를 모두 요구한다.

$$
\mathcal W_{\rm total}
=
\mathcal W_{\rm setup}
+\mathcal W_{\rm candidate}
+\mathcal W_{\rm certificate}
+\mathcal W_{\rm rejected}
+\mathcal W_{\rm fallback}
+\mathcal W_{\rm dispatch}.
$$

$$
T_{\rm par,model}=
T_{\rm setup}
+\sum_{\rm round}\max_{\rm worker}T_{\rm assigned\ tasks}
+T_{\rm sync}
+T_{\rm certificate}
+T_{\rm fallback}.
$$

첫 번째는 총 작업량이고 두 번째는 고정 P worker에서의 span/시간 모델이다. scheduling이 불균등하면 $\mathcal W/P$만으로 span을 대체할 수 없다. shared resource와 메모리 대역폭 때문에 모델은 실제 elapsed time과도 다르다.

같은 output accuracy의 strongest sequential arm과 비교하고 다음을 전부 charge한다.

- common-W factorization, witness preparation, batch packing;
- nonlinear RHS/JVP, orthogonalization, coefficient setup;
- preflight와 실패한 certificate radius;
- speculative work가 버려진 경우의 비용;
- worker pool, barriers, synchronization;
- sequential fallback의 8 stage와 새 step-size retry.

R-NEXT-06에서 q=2 depth는 7이고, corrected n=16 margin은 0.043 solve units 수준이며 dispatch는 0이었다. 이미 공개된 이 결과를 다시 실행할 이유는 없다. 실제 비싼 RHS/JVP나 효율적인 BatchRhsFn을 제공하는 client가 생긴 경우에만 새 regime을 등록한다.

### 7.5 실제 client를 유지하는 이식안

~~~rust
// Proposed interfaces: candidate work is never an acceptance certificate.
struct FrozenStageTarget { /* tableau, operator, state, metric, epoch */ }
struct HomotopyCandidate { /* endpoint K, work ledger, path status */ }
struct EndpointCertificate { /* original target only, output budget */ }

enum CandidateDecision {
    Accepted { candidate: HomotopyCandidate, proof: EndpointCertificate },
    Fallback { reason: AdmissionFailure, spent_work: WorkLedger },
}

fn build_parallel_candidate(
    target: &FrozenStageTarget,
    model: &dyn BoundStageModel,
    execution: &ParallelExecution,
    policy: &HomotopyPolicy,
) -> Result<HomotopyCandidate, CandidateFailure>;

fn certify_original_endpoint(
    target: &FrozenStageTarget,
    candidate: &HomotopyCandidate,
    budget: &OutputBudget,
) -> Result<EndpointCertificate, AdmissionFailure>;
~~~

현재 파일 매핑은 homotopy.rs의 candidate/path, transactional_q1_q2.rs의 original-target admission, parallel.rs와 problem.rs::BatchRhsFn의 실행이다. 새 generic certificate hierarchy를 만들기 전에 이 기존 경계에 work telemetry와 model binding을 연결한다.

PP07의 actual Fourier client는 shifted solve가 없었다. 따라서 shared-shift jet를 억지로 연결하지 않는다. step halving으로 한 번에 하나씩 등장하는 shift를 “처음부터 동시에 아는 많은 shift”와 같은 문제로 취급하지도 않는다. 실제 client의 독립 작업이 없으면 병렬화 연구는 HOLD가 맞다.

## 8. 구조 router·preconditioner·새 integrator의 순서

OdeProblem은 Jacobian/JVP callback을 가지지만 현재 struct에는 일반 sparse-pattern declaration이 없다. 반면 rodas5p_fast.rs에는 banded entry point가 있다. 첫 이식은 구조 계약을 분명하게 만드는 것이다.

| 선언 가능한 정보 | 우선 비교 대상 | 후속 후보 |
|---|---|---|
| 작은 dense J | dense direct LU | verified residual correction, isolated programming improvements |
| 실제 band와 현재 J | 기존 banded driver | bandwidth-aware routing |
| 큰 sparse pattern | sparse direct kernel이 먼저 필요 | colored JVP assembly, linear-part right preconditioner |
| verified symmetric nonpositive linear part | certified Chebyshev | Laguerre/Leja를 같은 total budget에서 제한적으로 비교 |
| unstructured nonnormal semilinear | 개선된 matrix-free RODAS | own-step-size exponential candidate |
| 비싼 independent stage callbacks | strongest sequential driver | endpoint-certified homotopy batching |

다항식 inverse preconditioner의 실용적인 첫 영역은 SPD shifted W다. Chebyshev semi-iteration의 spectral interval을 검증하고 exact W의 true residual로 종료한다. preconditioner가 iteration 중 바뀌면 solver가 이를 지원하는지도 계약으로 고정한다. 임의의 preconditioning은 기존 shift-invariant Krylov reuse 가정을 보존하지 않으므로 별도 증명이 필요하다.

Pilot B4의 2-D linear-part preconditioner는 sparse LU와 right-preconditioned path가 필요한 연구다. pilot의 counted flops나 최소 두 번 kernel time을 native end-to-end speed로 승격하지 않는다. 선언 구조를 사용할 수 있는데 비효율적인 matrix-free baseline만 비교하는 실험도 피한다.

Pilot B3의 exponential method는 own step size라는 연구 가치는 있지만, defect integral을 floating Simpson quadrature로 계산하고 actual error와 비교한 것은 **numerical validation**이다. quadrature remainder, Arnoldi defect, coefficient/recurrence arithmetic을 감싸지 않은 계산을 machine certificate로 이식하지 않는다.

## 9. 다음 개발 단계의 순서와 중단 조건

| 순서 | 구체적인 산출물 | 새로 확인할 것 | 중단 조건 |
|---|---|---|---|
| 1 | current 2×2 correction witness와 independent oracle | 국소 represented-W 계약 | 부적절한 admission 또는 invalid-input 통과 |
| 2 | DupFix·SPD04 caller adoption patch | focused bit identity, reset/failure, 작업 계수 | 알고리즘 trajectory 변경 |
| 3 | Calibrated / Verified evidence 분리 | 실제 허용된 floor/stall/fallback charge | 상계 없는 허용을 certified로 표현 |
| 4 | declared-structure metadata/router | 가장 싼 applicable direct arm과 비교 | 구조를 검증/계약할 수 없음 |
| 5 | causal whole-step contamination | tube, frozen metric, output assembly | bound가 항상 budget 초과; 비용 이득 없음 |
| 6 | prospective controller calibration | 새 seed·interior grid, baseline-relative gate | 기존 outlier를 보고 gate를 다시 맞춤 |
| 7 | normalized fused phi certificate | 새로운 extreme-scale fixtures | representability를 무시한 relative-only 약속 |
| 8 | Laguerre component exporter와 adjoint redesign | total bottleneck과 Chebyshev-only 비교 | coefficient 개선만 있고 total 개선 없음 |
| 9 | Leja total certificate 또는 explicit EstimateOnly | 실제 expensive-JVP niche | 전체 검증 비용 포함 우위 없음 |
| 10 | real-client homotopy prototype | 전체 work·span·endpoint certificate | 실제 client 부재 또는 net margin≤0 |

이 순서는 default promotion을 자동 승인하는 DAG가 아니다. 각 node는 등록된 입력·예산·checker를 가지며 gate 통과 범위만 다음 단계로 넘긴다. correctness, usefulness, counted cost, wall time은 독립 판정이다.

## 10. 문헌과 증거의 역할

SciSpace 검색으로 문헌을 찾고 다음 primary arXiv 원문을 열어 확인했다. 아래 요약은 각 source당 200단어 미만이며, 이 문서의 prototype나 최신 repository 실험을 문헌으로 증명했다고 주장하지 않는다.

- Caliari, Kandolf, Ostermann, Rainer, *The Leja method revisited: backward error analysis for the matrix exponential*, SIAM J. Sci. Comput. 2016, DOI [10.1137/15M1027620](https://doi.org/10.1137/15M1027620), [arXiv:1506.08665v2](https://arxiv.org/html/1506.08665v2). Scaling·degree·interpolation interval을 backward error 관점에서 선택하는 근거로 사용한다. PP10의 last-two-term stop 또는 현재 floating implementation의 total forward certificate를 대신하지 않는다.
- Jawecki, Auzinger, Koch, *Computable upper error bounds for Krylov approximations to matrix exponentials and associated phi-functions*, BIT 2019, DOI [10.1007/s10543-019-00771-6](https://doi.org/10.1007/s10543-019-00771-6), [arXiv:1809.03369v2](https://arxiv.org/html/1809.03369v2). Defect 기반 error bound의 문헌 계보로 사용한다. pilot의 floating quadrature나 norm 가정이 검증됐다는 근거로 확장하지 않는다.

Repository의 실험 수치는 해당 research node의 PREREGISTRATION.md, raw/result 파일과 append-only ledger를 근거로 한다. 위의 exact residual 항등식, triangular 반례, causal Lipschitz recurrence와 homogeneous rescaling은 이 문서에서 조건을 밝힌 수학적 유도다. 이론적 관계, native 구현, 표본 실험, 성능 측정의 증거 수준을 합쳐 표현하지 않는다.
