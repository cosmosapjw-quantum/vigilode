# 범용 ODE 도구로의 통합: 오차 예산과 검증 계획

## 판정의 대상

이번 branch의 국소 개선은 범용 production 판정과 분리해야 한다. 원래 목표는 작은 계산환경에서 stiff·빠른 진동을 잘 처리하면서 비강성 문제, 관측 출력, 실패, 재시작도 예측 가능하게 다루는 ODE 도구다. 이를 위해서는 빠른 한 경로의 성공보다 **요청 기능이 어떤 경로에서 보장되는지**, **실제 목표 정확도를 달성하는지**, **실패 후 마지막 수락 상태가 안전하게 반환되는지**가 먼저 고정되어야 한다.

이번 문서의 `BENCHMARK_PLAN.json`은 후속 실행을 위한 구체적 계약이다. 외부 비교와 광범위 qualification을 이번에 새로 실행했다고 주장하지 않는다. NCP는 사용하지 않으며, 기존 결과의 유효한 범위는 그대로 재사용한다. 과거 개발 목록 N01–N30과 적응/causal 목록 SW01–PB01의 40개 항목을 모두 출처·기존 상태와 함께 연결했다. 이 매핑은 과거 상태를 새 검증 없이 완료로 바꾸지 않는다.

## 코드가 현재 제공하는 기능과 실제 간극

| 기능 | 현재 코드에서 확인한 동작 | 남은 구체적 개발 |
|---|---|---|
| 명시적 기본 경로 | `Dopri5`는 RHS-only, identity mass, 비자율 RHS를 지원한다. 실행 중 동일 RHS 함수라는 계약이 있으며 숨은 모델 변경은 FSAL을 무효화한다. | 계수·모델 변경은 accepted endpoint에서 새 session을 시작하고 FSAL·controller·cache를 명시적으로 초기화한다. 객체 내부 변경을 자동 탐지한다고 주장하지 않는다. |
| switching | `switching.rs`는 mass·events·dense-output 요청을 거절한다. 승인된 implicit 경로와 RHS-only 경로의 기능 범위가 다르다. | checkpoint와 typed capability를 통일하고, 전환 뒤 원래 출력·오차 norm을 유지한다. 제공자 허용과 제공자 성능 근거를 분리한다. |
| RODAS dense output | dense/banded fast 경로의 연속 확장과 내부 오차 추정이 존재한다. 기본 W-filtered midpoint 검사는 추가 RHS+backsolve를 사용한다. 이는 추정이며 연속 구간 전체의 증명된 상계가 아니다. | endpoint·quarter·midpoint·three-quarter·고정 물리 관측점의 오차를 독립 채점한다. 출력 밀도에 따른 비용을 기록하고 dense-only 경로와 landing 경로를 섞어 속도를 주장하지 않는다. |
| JVP/mass 조합 | 공개 capability는 JVP-only의 mass 및 dense-output 조합을 거절한다. constant mass와 일부 index-1 DAE는 별도 dense 경로다. | 지원되지 않는 조합의 typed refusal을 유지한다. 큰 PDE에서 mass+dense가 dense LU로 돌아가는 비용을 숨기지 않는다. coupled identity-mass 결과를 DAE로 전이하지 않는다. |
| 사건 | `events.rs`는 accepted segment를 **실행 후** 조사한다. terminal event는 이미 계산한 출력을 자르며 사건 뒤 callback도 실행된다. 반 구간에서 짝수 crossing이나 tangent는 놓칠 수 있다. | accepted-step loop 내부에서 terminal event를 결정하고 이후 물리 step callback을 금지하는 경로를 추가한다. 현재 기능은 terminal domain safety를 제공하지 않는다. 다중/tangent 사건은 따로 지원 범위를 지정한다. |
| 실패 | RODAS는 `Status::Failed`와 partial solution/work를 반환하고 `ensure_success`로 오류화할 수 있다. 명시적 경로도 fatal·불완전 write·nonfinite 입력에서 수락 prefix를 보존한다. | 전환·새 flow·AMR 경로가 같은 계약을 유지하는지 callback boundary에서 확인한다. failed trial, rejected pilot, fallback, 이후 event 후처리 비용을 모두 포함한다. |
| AMR | 보존 remap 연구/구현과 client 예제가 있지만 모든 solver façade에 동일한 가변차원 accepted-step restart 계약이 연결된 것은 아니다. | remap의 상태·보존·오차 예산을 승인한 뒤만 차원 변경을 commit하고 workspace 크기·provider·basis·dense segment를 함께 무효화한다. |

근거는 `crates/vigilode/src/{lib,problem,options,events,capabilities,solution,explicit,switching}.rs`와 `crates/vigilode-coupled/src/{adaptive,raw_step}.rs`다. `Tier::Performance` 명칭만으로 candidate가 기본 route에 연결됐다고 판단하지 않는다. 실제 driver/route record가 기준이다.

## 정리: 연속 residual, 공간 오차, remap, 근사 flow를 한 예산으로 연결

고정 격자 구간 j에서 정확한 연속 물리 해의 투영을 xⱼ(t), 수치적으로 재구성된 연속 궤적을 vⱼ(t)라 하자. 선택한 격자 방정식은 fⱼ이고,

\[
\dot x_j=f_j(t,x_j)+s_j(t),\qquad
\dot v_j=f_j(t,v_j)+r_j(t)
\]

로 쓴다. sⱼ는 **공간 consistency defect**, rⱼ는 연속 재구성의 **시간 residual**이다. 투영된 참해를 기준으로 한 sⱼ와 두 수치 reconstruction의 차이는 같은 양이 아니다. 이동 격자에서는 d(Pⱼ(t)u)/dt의 격자 이동 항도 sⱼ의 정의에 포함하며, discrete geometric conservation law를 별도로 만족시켜야 한다.

동일 norm에서 해당 오차 선분 전체에 one-sided growth bound μⱼ(t)가 성립한다고 가정한다. Euclidean norm과 미분 가능한 f라면 선분상의 대칭 Jacobian 부분의 최대 고유값 상계로 충분하다. 이 계산을 실행 시 Jacobian으로 구할 필요는 없으며, dissipative 물리 구조나 클라이언트의 해석적 bound로 공급할 수 있다. 이 가정하에 Dini 미분 부등식은

\[
D^+ E(t)\le\mu_j(t)E(t)+\|r_j(t)\|+\|s_j(t)\|,
\qquad E(t)=\|v_j(t)-x_j(t)\|
\]

이다. 증명은 두 미분식을 빼고, f의 차이에 one-sided growth bound를 적용하고, residual 차이에 삼각부등식을 적용하면 된다.

격자 변경 시 v⁺=Rⱼv⁻+ξⱼ이고 참해의 새 투영은 x⁺라 하자. Rⱼ의 구·신 norm 사이 Lipschitz 상수가 Lⱼ이며

\[
d_j\ge\|R_jx^- -x^+\|,\qquad \|\xi_j\|\le a_j
\]

이면 E⁺≤LⱼE⁻+dⱼ+aⱼ이다. dⱼ는 참해 투영에 대한 remap consistency error이며, aⱼ에는 구현 roundoff·보존 보정 등의 독립적으로 제한한 추가 교란을 포함할 수 있다. 별도 근사 matrix-function/flow 호출이 endpoint를 ηₖ만큼 교란하면 그 시점의 jump로 같은 방식으로 더한다. 같은 시각에 remap과 flow 교란이 있으면 여기서는 remap을 먼저 commit한 뒤 새 공간에서 ηₖ를 더하는 순서를 정의한다. 반대 순서로 계산한 교란은 remap의 Lⱼ를 곱해서 전달해야 한다. **이미 rⱼ에 포함한 flow 교란을 ηₖ로 다시 더해 이중 계상하거나, 양쪽에서 모두 생략하면 안 된다.**

미래 증폭 인자를

\[
\Gamma(T,s)=\exp\!\left(\int_s^T\mu(\tau)d\tau\right)
             \prod_{s<t_j\le T}L_j
\]

로 정의한다. 단일 구간의 적분 인자 공식과 jump 재귀를 차례로 적용하면

\[
E(T)\le\Gamma(T,0)E(0)
 +\sum_j\int_{I_j}\Gamma(T,s)(\|r_j(s)\|+\|s_j(s)\|)ds
 +\sum_j\Gamma(T,t_j)(d_j+a_j)
 +\sum_k\Gamma(T,t_k)\eta_k.
\]

이는 contraction을 가정하지 않고도 성립한다. μ가 크거나 remap L이 1보다 크면 bound 자체가 커진다. 음의 고유값만 보고 비정규 행렬의 μ≤0이라고 놓으면 안 된다. 반면 감쇠 회전 A=−κI+ωR, Rᵀ=−R에서는 μ₂(A)=−κ이므로 이 continuous-residual bound의 증폭은 ω에 의존하지 않는다. 이것이 빠른 회전에서 정확한 carrier flow와 잔차 예산을 결합할 수 있는 구체적 이유다. 특정 수치 step의 stability나 오차 estimator가 이 bound를 자동으로 만족한다는 별도 주장은 아니다.

### 보존 remap의 유용한 충분조건

밀도 벡터, 양의 cell volume w⁻, w⁺를 사용하고 R의 모든 원소가 음이 아니며 (w⁺)ᵀR=(w⁻)ᵀ이면

\[
\|Rz\|_{1,w^+}\le(w^+)^TR|z|=(w^-)^T|z|=\|z\|_{1,w^-}.
\]

따라서 weighted L1에서 Lⱼ≤1이다. mass conservation과 오차 비증폭을 동시에 얻지만 dⱼ=0이나 높은 차수는 결론에 포함되지 않는다. 실제 exact-rational 검사에서는 mass가 정확히 0인 변화라도 상태 consistency defect가 `1/100`일 수 있음을 보존했다. AMR 뒤에도 전체 5차를 주장하려면 시간 defect 외에 누적 가중 remap defect가 O(h⁵)인 조건을 별도로 충족해야 한다.

### 정보기하 norm을 사용할 때의 추가항

고정 구간에서 **외부에서 지정한** 미분 가능한 양의 정부호 H(t)에 대해 ‖e‖²_H=eᵀH(t)e라 하자. 관련 선분의 Jacobian J에 대한 성장률은

\[
\mu_H(t)\le\tfrac12\lambda_{max}\left[H^{-1/2}
 (J^TH+HJ+\dot H)H^{-1/2}\right]
\]

의 상계로 충분하다. Fisher 형태의 가중치를 사용하면서 \dot H를 생략하면 이동 metric의 에너지 변화가 빠진다. H가 미지 해에 의존하는 일반 Riemannian metric이면 이 간단한 선형 metric 공식만으로 결론을 확장하지 않는다. 영점 부근의 Fisher 가중치 정칙화, norm equivalence 상수, remap Lipschitz 상수와 실제 물리 observable 연결이 필요하다. 이 조건이 없는 경우 physical weighted L1/L2 norm을 정확도 기준으로 유지하고 Fisher metric은 비용 개선용 좌표/전처리에 한정한다.

## 구현 가능한 오차 예산 알고리즘

목표 물리 오차 ε를 initial 5%, time defect 35%, approximate flow/root 15%, spatial 25%, remap 15%, roundoff 5%로 분할한다. 이 비율은 제안하는 개발 정책이며 수학적으로 유일한 최적값이 아니다.

1. 클라이언트는 각 구간의 μ 상계, remap norm·L, reference projection, 물리 norm, 미래 최대 증폭 상계 G(T,t)를 제공한다. 아래 나눗셈 기반 예산을 적용하기 전에 모든 사용 시각에서 0<G(T,t)<∞를 확인한다. 0이나 무한대이면 그 공식을 적용하지 않고 별도 퇴화 경우를 처리하거나 operational 정책으로 돌아간다. 검증되지 않은 수치 추정은 `estimated`로 기록하고 certified라는 표현을 사용하지 않는다.
2. 매 accepted 후보 step에서 재구성 residual의 적분 상계와 공간 consistency 상계를 따로 구한다. 몇 점의 residual 표본만으로 적분 상계라 부르지 않는다. 다항식·analytic bound 또는 interval enclosure가 없으면 관측 오차 기반의 operational 정책으로 남긴다.
3. approximate-flow/root budget은 ηₙ≤0.15 ε hₙ/[T G(T,tₙ₊₁)]로 배분하면 총 가중 기여가 0.15ε 이하가 된다. root 단계의 원공간 residual에서 ηₙ를 얻는 조건은 `CAUSAL_TRAJECTORY.md`에 제시했다. 일반 h-dependent bound를 검증하지 않고 고정 residual tolerance만으로 대신하지 않는다.
4. remap 횟수를 사전에 모르면 j번째 remap에 최대 0.15 ε·2⁻ʲ/G(T,tⱼ)를 주어 무한 기하급수의 합으로 누적 예산을 제한할 수 있다. 지나치게 보수적이면 남은 횟수/시간에 근거한 재배분을 하되 이미 쓴 예산을 되돌리지 않는다.
5. positivity correction, limiter change, 정밀도 손실은 jump 또는 residual 항에 명시적으로 반영한다. mass만 정확히 맞췄다는 이유로 상태오차 비용을 0으로 기록하지 않는다.
6. bound·예산·norm 검사 실패 시 후보를 commit하지 않는다. 마지막 수락 상태를 반환하거나 더 작은 step/격자/검증된 다른 경로를 시도하며, 실패 비용을 모두 남긴다.
7. 출력 시각 τ마다 같은 공식을 τ까지 적용한다. event 시간오차도 수치 root finder bracket 폭만으로 평가하지 않는다.

단순한 transversal 사건의 경우 실제 근과 수치 근이 모두 같은 구간 안에 있고 그 구간 전체에서 실제 g(t,x(t))가 고유 근을 가지며 |d g/dt|≥ν>0이고 g의 상태 Lipschitz 상수가 L_g라면, 수치 root의 g residual이 ε_g 이하일 때

\[
|\hat t-t_*|\le (L_g E(\hat t)+\epsilon_g)/\nu
\]

가 성립한다. 평균값정리와 상태오차 bound를 결합한 결과다. tangent 또는 여러 근에는 이 충분조건을 적용하지 않는다.

## 다음 개발의 명확한 완료 조건

| 작업 | 구체적 완료 조건 | 현재 판단 |
|---|---|---|
| 실행 중 terminal event | accepted-step hook에서 근을 찾고 이후 **추가 물리 trial**을 실행하지 않는다. 마지막 event state/time, 비용, partial prefix를 일치시킨다. 이미 trial 내부에서 평가한 사건 이후 stage 시간과, 사건을 알아낸 뒤의 추가 callback을 구분한다. | 현재 후처리 방식은 미달. domain wall을 넘는 stage 평가 자체를 막으려면 별도 safe-domain bracket/step cap 또는 client domain 계약이 필요하다. |
| 전환·재시작 | provider·model epoch·dimension·method 변경 후 stale FSAL/factor/basis를 사용하지 않는다. fatal callback 뒤 다른 solver를 호출하지 않는다. 동일 method 유지 시 output/work의 기존 일치 계약을 보존한다. | scoped 개선과 미완성 session 재사용을 분리해서 판정. |
| 전역 정확도 | 명시한 smooth fixture에서 독립 기준해 uncertainty≤목표 cap의 10%, 모든 필수 관측점의 actual normalized error≤1. 3개 이상 refinement 수준에서 시간·공간·remap을 따로 변화시킨다. | 표본 오차와 인증된 상계는 별도로 보고. |
| 장시간 진동 | 100/1000 cycles에서 amplitude error≤1e−5, phase error≤1e−3 rad를 제안한다. 진폭 0 근처 phase는 제외 조건을 기록한다. 관측점이 phase unwrap에 부족하면 analytic reference와 각 구간 branch 정보를 사용한다. | 20 rad의 짧은 causal fixture만으로 충족하지 않음. |
| 비정규·강성 | transient peak, stiff forcing, Prothero order reduction, 실패/최소step를 유지한다. eigenvalue 범위만으로 operator norm을 대신하지 않는다. | 일반성 주장을 위해 필수. |
| PDE/AMR | 두 periodic 격자와 구면 격자의 공간 수렴, weighted mass drift≤1e−11 상대+1e−13 절대, 음수 크기와 physical source budget을 보고한다. positivity 보장은 별도 방법 조건에서만 부여한다. | 새 grid prototype 성공은 nonlinear multiphysics production과 다름. |
| 성능 | 실제 오차·출력 조건이 같은 항목만 비교한다. setup·pilot·failed work 포함, 단일 스레드 필수, 2-thread 별도. 선택 기본값의 2배 초과 손실은 원인 분석 없이 통과시키지 않는다. | 1.2배 이득·2배 손실 기준은 사전 engineering 정책. 보편적 no-regret 정리가 아님. |

## 실행 예산과 비교 범위

계획은 로컬 1-thread 필수, 2-thread 선택으로 제한한다. 빌드 jobs≤2, solver RSS≤1GiB, timed solve 동시 실행 1개, 각 process≤90초, 전체≤5400초, process 생성≤500개다. 모든 parameter의 Cartesian product를 실행하지 않고 먼저 correctness와 민감한 영역을 확인한 후 사전 지정한 subset을 확장한다. 예산이 끝난 항목은 `NOT_RUN`, 시간 초과는 `TIMEOUT`, 지원 밖 조합은 `UNSUPPORTED`로 남긴다.

외부 비교는 **VODE/DVODE, diffsol, Julia**만 허용한다. Julia compilation/JIT와 steady solve를 구분하고 양쪽 setup 포함 시간도 기록한다. 동일 RHS, 물리 norm, 관측시각, 실패 정책, 정밀도, thread 조건을 적용한다. 외부 dependency가 없으면 오류를 보존하고 `NOT_RUN`으로 둔다. 설치되지 않은 비교군의 숫자를 문헌이나 다른 기계 결과로 채우지 않는다.

공개 출처 고정, CPU affinity, binary/compiler/build identity, 모든 상태와 기준해 ID·불확실성, raw 실패를 남긴다. 기본 판정은 문제군별 `production-qualified scope / specialized-only / optimization-possible / hold / pruned in this cohort`다. 몇 개의 빠른 kernel 결과나 많은 테스트 개수만으로 범용 promotion을 선언하지 않는다.

## 이번에 실제 확인한 것

`src/validate_campaign_contract.py`를 실행했다. 계획의 자원/비교군/40개 과거 항목 매핑/오차 예산/증거 필드를 검사했고, exact-rational weighted positive remap 사례 52개와 오차·remap jump 부호 조합 32개에서 재귀 bound를 확인했다. 최대 실제 오차와 bound가 모두 `1079/31250`인 sharp 사례도 포함한다. NCP 허용, 필수 문제군 삭제, 예산 초과, 비교군 추가, 과거 매핑 삭제, 실패 비용 누락의 여섯 변조를 모두 거절했다.

이 검사는 계획과 수학적 작은 예의 검증이며 광범위 solver benchmark 실행이나 물리 상수의 인증이 아니다. 결과는 `results/production_validation_checks.json`에 있다. 변경하지 않은 과거 수치 연구는 다시 실행하지 않았다.
