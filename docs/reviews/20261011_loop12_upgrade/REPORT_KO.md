# VigilODE: Loop 12 통합과 정확도·작업량 보장의 추가 승격

작성일: 2026-10-11. 기준 commit: 2e944afc36a20fd20ce33438b2a1d4f4e0a6a5c4.
이전 연구: [20261011 promotion theory](../20261011_promotion_theory/REPORT_KO.md).
새 입력: rvj_independent_loop12_20261011.zip, SHA-256
2735bf3ea1c52abae2e97a3310714eb4a9b47e4e08e786c7c0fc32831d97835c.

## 1. 이번에 실제로 달라진 것

Loop 12의 중심 성과는 generic Jacobian을 근사하는 데 그치지 않고, 알려진 선형 흐름과 저차원 collective feedback이 있는 모델에서 큰 차원의 비선형 문제를 만들지 않는 것이다. 모든 초기 모드와 transverse 정보를 저장한 채 nonlinear 계산을 scalar 또는 rank-r Volterra equation으로 옮긴다. 이번 작업은 이 원본을 독립적으로 검토하고, 공통 감쇠·rank-r/여러 주파수 군집·다중 시간창으로 확장했다. 여기에 실제 저장소 API에 연결되는 native 이식 계약과 작은 실행 가능한 계수 컴파일을 추가했다.

최종 판정의 SSOT는 [DECISION.json](../../../research/promotion_loop12_20261011/DECISION.json), 개별 전제와 근거는 [THEOREMS.json](../../../research/promotion_loop12_20261011/THEOREMS.json)이다. 조건부 수학 정리와 실제 native production·전역 궤적·실측 속도의 판정은 분리한다. 기존 발표 결과를 일괄 재실행하지 않았고, 이전 20개 승인 항목 및 K-form leakage 보정은 그대로 보존한다.

원본에서 보완한 두 비용 문제는 실질적이다. 출력이 많을 때 공유 복원 다항식 준비 비용이 빠져 있었고, reference의 반복 factorial 계산은 간단한 coefficient-product count 밖의 작업이었다. 둘 다 아래에서 수정했다. 원본 ZIP과 저자의 HOLD·실패 기록은 byte-identical 상태로 남겼다.

## 2. 개선 정리와 계산에 주는 효과

| 구분 | 직접 증명한 보장 | 적용 범위와 의미 |
|---|---|---|
| 원본 A–G의 scoped adoption | 정확한 collective/transverse 분리, 원모델 kernel-tail 및 모든 mode 오차, same-map kernel contraction 감소 | 알려진 frequency/weight/initial data와 공통 cubic 피드백; 원 코드 κ=0 |
| LC1–LC3 | 출력 준비를 포함한 복잡도, recurrence로 계산한 causal weights, 엄격한 총 작업량 개선 부등식 | 산술 비용 모델·동일 맵 비교; bit 비용과 시간은 별도 |
| DW-A–C | 감쇠를 지수함수로 보존한 nonlinear homotopy, κ에 공통인 차수 선택, 정확한 mixed Duhamel integral | 명시적 작은 비선형 regime; scalar interval evaluator는 새 native 과제 |
| DW-D–E | inherited uncertainty와 kernel/roundoff/phase를 포함하는 모든 mode 및 다중 창 오차 | 현재 packet 모델의 에너지 항등식과 candidate tube 필요 |
| DW-F | 감쇠를 보존한 같은 계산의 집계와 총 작업량 감소 조건 | 준비·복원·scalar precision·실패 비용 포함 |
| R1–R3 | rank-r exact memory, 여러 군집의 factorial tail, 원래 전 상태 복원과 오차 | 직접 제공된 D,U,V와 실제 r·군집 폭을 비용에 반영 |
| R4 | 연속 Volterra 반복의 factorial 수렴, 선형화 inverse와 regular homotopy branch | 전역 Lipschitz 또는 별도 인증된 공통 존재영역; projection 오차는 별도 |
| R5–R6 | same-map rank-r work gate, 소멸다항식에 따른 정확한 rd차원 closure | explicit comparator 및 full-space polynomial identity 필요 |

### 2.1 감쇠를 시간 다항식으로 억지로 해상하지 않는다

공통 감쇠와 carrier를 분리하여 \(a_j=e^{-\kappa t}b_j\)로 놓으면

\[
b'_j=i\Delta\xi_jb_j+e^{-2\kappa t}i\chi|u|^2u,\qquad
u=\sum_j\pi_jb_j .
\]

여기서 빠른 \(e^{-2\kappa t}\)는 정확한 함수 인자로 남기고 비선형 항 앞의 homotopy 변수 λ를 전개한다. λ의 목표값은 1이며 원래 방정식을 다른 작은-coupling 문제로 바꾸지 않는다. 반복 적분이 사용하는 유효 길이는

\[
\theta_\kappa(T)=\frac{1-e^{-2\kappa T}}{2\kappa}\le T,\qquad
\theta_0(T)=T .
\]

명시된 family \(T\le1/2,\ |\Delta|\le1,\ |\chi|\le1/8,\ A_0\le9/32,\
\max_j|a_j(0)|\le11/32\)에서 detuning 차수 p=8과 nonlinear homotopy 차수 Q=10은 **모든 κ≥0**에 대해 exact-arithmetic mode 오차 상계 \(2.33475\times10^{-8}\) 미만을 준다. 추가 representation·phase·roundoff는 별도로 더한다.

이것은 감쇠가 커질 때 p/Q를 κT에 비례하여 늘릴 필요가 없다는 구성적 정리다. 보수적인 시간 polynomial degree는 258이고, 기존 κ=0 후보보다 총 작업량이 작다는 주장이나 scalar exponential의 bit 비용이 κ에 독립이라는 주장은 아니다. [전체 증명과 stable evaluator 계약](DAMPING_WINDOWS_KO.md)에 이러한 구분을 명시했다.

### 2.2 전역 오차 전달이 dense 행렬을 요구하지 않는다

고정 가중치에 대해 \(P=\mathbf1\pi^T,\ P^2=P\)다. 창 n의 실제 초기 오차 상계 벡터를 \(d_n\), 새 국소 오차를 \(\eta_n\)이라 하면

\[
d_{n+1}\le M_nd_n+\eta_n,\qquad
M_n=e^{-\kappa h_n}
\left[I+\big(e^{C_n\theta_\kappa(h_n)}-1\big)P\right],
\quad C_n=3|\chi|R_n^2 .
\]

한 번의 weighted sum과 n개 scalar update로 이 전파를 **O(n)**에 계산한다. 이전 창의 불확실성을 0으로 초기화하지 않으며, local error와 inherited error를 중복 부과하지 않는다. P가 고정이면 여러 transport의 곱도 닫힌 식으로 계산된다. \(\kappa\ge C_n\)이면 이 전파의 norm은 1 이하이므로 local budget의 합으로 전역 endpoint budget을 보장할 수 있다. 전 조건과 phase·시간·descriptor 변경의 처리는 DW-D/E에 있다.

### 2.3 rank-r memory와 homotopy의 이점

\[
y'=Dy+UF(V^*y),\qquad m=V^*y
\]

라는 정확한 모델 구조가 주어지면

\[
m(t)=V^*e^{Dt}y_0+
\int_0^t V^*e^{D(t-s)}U F(m(s))\,ds
\]

로 축약한다. 원래 \(y_0\)는 모두 보존하고 mode별 Duhamel 식으로 재구성한다. 여러 주파수 군집에는 matrix moment와 군집별 tail을 쓴다. 실제 r이나 군집 폭이 크면 이득이 줄어드는 비용도 그대로 드러난다.

연속 Volterra 선형화의 반복에는

\[
\|\mathcal A^s\|\le (BLT)^s/s!,\qquad
\|(I-\mathcal A)^{-1}\|\le e^{BLT}
\]

가 성립한다. 따라서 명시된 존재영역에서는 \(BLT<1\)이라는 단순 contraction 조건 없이도 수렴과 regular homotopy branch를 얻는다. 여러 λ target이 실제로 필요한 경우 같은 초기값에서 독립적으로 계산할 수 있지만, 한 target 안의 Picard sweep이나 실제 시간창의 초기값 의존성은 사라지지 않는다. finite degree projection의 오차도 별도 인증해야 한다.

새 rank-2 예제에서는 선언된 packet 계산의 곱셈·덧셈 상계가 **16,904회**, full-profile 비교자의 적분 곱셈만으로 얻은 하계가 **49,152회**다. 나머지 비공통 비용의 차액이 **32,248회 미만**이면 총 scalar arithmetic work가 엄격히 작다. 이미 moment를 hoist한 비교자에는 같은 이득을 다시 주장하지 않는다. [RANK_GEOMETRY_KO.md](RANK_GEOMETRY_KO.md)에 같은 projection·forcing·출력의 조건과 증명을 실었다.

## 3. 적대적 검토에서 수정한 내용

1. **원본 로그 복잡도 식:** \(\ell=1+\log(1/\epsilon)\)일 때 일반 상계는
   \[
   O\!\left(n\ell+\ell^3+R_{\rm out}(n\ell+\ell^2)\right)
   \]
   이다. 출력 준비 항 \(R_{\rm out}\ell^2\)를 임의로 버린 원 식은 해당 일반성에서 승인하지 않았다.
2. **계수 생성과 메모리:** \(w_{jk}=j!k!/(j+k+1)!\)를 \(w_{j0}=1/(j+1)\), \(w_{j,k+1}=w_{jk}(k+1)/(j+k+2)\)로 생성한다. 새 cache prototype은 추가 \(O(pD)\) 저장 비용을 갖는다.
3. **정확한 비용 계수:** 모든 sweep이 같은 최대차수라고 셀 때는 zero-padding한 dense schedule을 명시한다. trim된 길이를 쓰면 실제 sweep별 길이의 합으로 계산한다.
4. **Projected reconstruction:** 같은 맵 항등식의 전 상태 출력에도 같은 선형 projection을 적용해야 한다. unprojected Duhamel 복원은 물리 오차 정리로 다룬다. 새 rank 후보의 혼동을 독립 검토 후 수정했다.
5. **기존 기능 중복:** Loop12 §11의 \(R_0W-W\)는 현재 shared_shift_jet에 이미 구현되어 있다. 항등식은 채택하지만 이번의 새 native 성능 개선으로 세지 않는다.

상세 비용 증명은 [COST_CORRECTIONS_KO.md](COST_CORRECTIONS_KO.md), 실제 독립 판정과 최초 finding은 evidence의 두 review에 보존한다. 새로운 감쇠 표현의 출력별 공유 준비 비용도 같은 원칙으로 부과한다.

## 4. 현재 코드에 어떻게 들어가는가

현재 OdeProblem은 Jacobian/JVP 중 하나를 요구하는 struct다. 순수 derivative-free packet은 생성형 descriptor를 받는 **별도 opt-in API**부터 구현한다. 기존 constructor에 dummy JVP를 넣거나 callback 구조를 추정해 자동 admission하지 않는다.

현재 joint_phi_action은 symmetric nonpositive operator를 대상으로 하고, scalar_phi_enclosure는 phi0–4 및 [-600,0] 범위를 지원한다. 이를 skew-oscillatory·nonnormal packet과 임의 차수 mixed integral의 인증으로 재사용할 수 없다. 새 scalar moment API가 필요하다. 기존 quadratic transaction과 특정 chart certificate 역시 다른 모델에 대한 authority다.

이번 source 조사에서 기존 nonlinear 실제 고객 모델이 collective structure를 만족한다는 근거는 확립하지 못했다. 그러나 첨부의 명시적 모델군 자체는 생성형 backend의 출발점이 될 수 있다. 구현은 다른 모든 일반 solver 연구가 끝날 때까지 기다릴 필요가 없다.

권장 첫 경로는 **LP01 생성형 모델 → LP02 κ=0 finite-window 구현**이다. 이후 감쇠 LP03, inherited-error chaining LP04, rank-r LP05를 선택한 적용범위에 맞추어 연결한다. 실제 고객 모델 적용 주장은 LP06, 실측 속도는 LP09, 완료한 경로의 독립 native 승격은 LP10에서 닫는다. FFT와 implicit closure는 별도의 선택적 분기다.

[PORTING_KO.md](PORTING_KO.md)는 실제 파일·행·API, 새 타입의 제안 상태, negative test와 통과 기준을 설명한다. [PORTING_DAG.json](../../../research/promotion_loop12_20261011/PORTING_DAG.json)은 이전 TP01–TP10과 원본 N1–N8을 12개 disposition/개발 단위로 연결한다. 이 중 기존 구현 확인과 missing-source 기록은 새 native 구현 완료로 세지 않는다.

## 5. 이번 실행과 재사용을 구분한다

- 첨부 manifest의 84개 파일 identity를 확인했고 원본 ZIP 전체를 저장소에 보존했다.
- 원본의 23개 test·34개 symbolic check 및 기존 numerical campaign은 제공된 증거로 재사용했다. 이번에 재실행한 결과로 표기하지 않는다.
- 새 causal-weight reference에 5개 검사군을 실행했고, 독립 검토자는 추가 3개 interface/경계 검사를 했다.
- 새 rank-2·두 군집의 exact kernel/free-profile/sweep 항등식과 새 감쇠 상수·projector transport를 경량 exact arithmetic으로 확인했다.
- Wolfram은 감쇠 적분의 6개 항등식을 0으로 확인했다. WolframContext는 service internal error였으며 CAS 평가 경로는 성공했다.
- SciSpace 검색 후 관련 원전의 확인 가능한 범위를 대조했다. 알려진 exact reduction·constrained lumping·RHS-only UA 기법을 새로 발명했다고 주장하지 않는다.
- Rust native source·default·dependency 변경 및 native campaign은 이번 범위에 없다. 새 Python 파일은 수학적 비용 계약을 실현하는 연구 reference다.

재현 가능한 검산과 자료 검증 결과는 [VALIDATION.json](../../../research/promotion_loop12_20261011/VALIDATION.json)에, 코드 실행 인계는 [NATIVE_HANDOFF_KO.md](NATIVE_HANDOFF_KO.md)에 있다. 입력 ZIP이 패키지에 포함되어 있으므로 후속 실행에 추가 첨부가 필요하지 않다.

## 6. 남는 승격 조건

수학적으로 승인된 구조·오차·복잡도 정리를 native 코드의 실제 결과와 연결해야 한다. 필요한 것은 모델 identity, outward scalar/coeff arithmetic, 전 구간 residual/tube, 모든 mode 복원, inherited error, 동일 정확도의 강한 비교자 및 실제 작업·시간 기록이다. 이전 current-K leakage 조건이나 일반 RVJ5의 미해결 사항을 이번 model-specific packet의 성과로 해제하지 않는다.

반면 새 이론을 얻기 위해 이미 승인된 과거 결과의 캠페인을 반복할 필요도 없다. 이번 패키지는 원본의 independent-review 부재를 해소하고, 감쇠와 rank-r/전역 연결의 구체적 충분조건을 공급하며, 다음 구현이 바로 소비할 수 있는 API·증명·fixture·DAG를 고정한다.
