# 범용 ODE 알고리즘 선택과 개발 우선순위 재감사

## 판단과 적용 범위

현재 자료가 지지하는 최소 구성은 **비강성 문제의 DOPRI5와 강성 문제의 보호된 RODAS5P**이다. 후자는 사용자가 허용한 미분 정보 및 구조에 따라 조밀 직접해법, 선언된 대역 직접해법, 제공된 Jacobian-vector product(JVP) 경로를 사용한다. 이것은 새 선택기의 제한된 기본 구성을 정한 것이며, 모든 입력에 대한 production 적합 판정은 아니다. 알 수 없는 문제에 DOPRI5를 먼저 허용하더라도 명시적인 계산 예산을 가진 시도이며, 실패가 강성의 증명이나 미분 사용 허가가 되지는 않는다.

현재 일반 공개 사용을 위한 판정은 조건부 보류이다. 필수 검토 대상은 약속한 endpoint·identity-mass 범위의 실패 전달, 지원 조합의 일관성 및 변경된 코드의 강성·비선형·진동 문제에 대한 정확도/강건성 증거다. 이벤트, 연속 출력, mass/DAE, 지속 실행은 명시적으로 제외하거나 거절할 수 있는 선택 기능이다. 그 부재 자체가 endpoint 전용 솔버의 production 부적격을 뜻하지 않는다. 현재 broad 보류는 이러한 모든 기능을 의무화한 판단이 아니라 폭넓은 현 코드 증거가 충분하지 않다는 판단이다. 통상적인 ODE 솔버는 국소 허용오차 계약을 가질 수 있다. 임의의 사용자 함수에 대한 전역오차 정리를 제공하지 못한다는 이유만으로 영구히 production 부적격이라고 판단하면, VODE와 같은 일반 수치 솔버의 목표와 다른 요구를 부과하게 된다. 대신 국소 추정치를 전역 상계로 광고하지 않고, 전역 보장이 필요한 사용자를 위한 추가 가정과 별도 검증 인터페이스를 구분해야 한다.

이번 범위는 일반 ODE이다. BASS/rei 물리 모형, 특정 복사전달 관측량 및 공간 재격자화의 완성 여부는 이번 일반 ODE 출시 판정의 필수조건에 포함하지 않는다. 해당 작업의 기존 자료를 지우거나 결론을 변경하지 않는다. 동일하게 특정 PDE 응용의 성공을 일반 ODE 적합성의 증거로 전용하지 않는다.

기준 소스는 `057fecf0698fa262d881f101625e6b0bb93791cf`이며 기존 독립 개발 tip `c03ca90f19a9e64ba7751c069325a48b54b39c2e`도 조사 범위에 포함했다. 최신 후속 개발 `1cdf2c3071498888f2d8ff3ddc0629cb3cf443ef`의 native 소스와 그 후 portable-norm 연구 변경을 구분한다. 이 분석에서는 솔버를 새로 실행하지 않았다. `PORTFOLIO.json`은 재사용 자료 13개의 SHA256, 알고리즘군 25개의 결정 및 다음 행동을 기록한다.

## 세 갈래 정책 탐색

본 탐색은 자료에 근거한 세 정책의 분기와 가지치기이다. 임의 rollout을 수행한 통계적 Monte Carlo tree search나 전역 최적해의 증명을 뜻하지 않는다. 기능 지원, 실제오차, 실패 처리, 단일 스레드 총시간, 유지보수 비용 순으로 판단한다.

| 정책 | 선택 결과 | 근거 |
|---|---|---|
| 강성 압력만으로 explicit/implicit 및 후보들을 자동 전환 | 현 기본 경로에서 제외 | pilot의 rapid-JVP 이득과 동시에 rapid-FD 약 2.03배, transition-FD 약 3.05배 손실이 관측되었다. 초기에 한 번 본 구간은 이후 강성 전이를 증명하지 못한다. |
| 문제 선언과 허용 미분 경로를 먼저 검사하고 DOPRI5 또는 보호된 RODAS5P를 선택 | 제한된 핵심 구성으로 채택 | 현재 실제오차 자료가 보여 주는 상보적 역할을 가장 적은 알고리즘 수로 제공한다. 기능 부재는 명시적으로 거절할 수 있다. |
| 샘플에서 구조가 보이면 causal/Lawson/다항식/homotopy를 자동 채택 | 명시적 구조 사용으로 제한 | 몇 번의 RHS 관측으로 구조, 오차 상계 또는 비용 우위를 보장할 수 없다. 성공 자료는 구조와 비교 기준이 제한되어 있다. |

기존 `Tier::Performance`는 현재 `Protected`와 같은 경로를 실행한다. 이 이름을 기존 실험 후보들이 이미 생산 경로에 들어갔다는 증거로 해석해서는 안 된다. 가지치기는 자동 선택과 핵심 최적화 우선순위에서 제외한다는 뜻이다. 코드와 회귀시험, 재현 자료를 삭제하라는 뜻은 아니다.

## 동일 실제오차 비교가 지지하는 범위

| 알고리즘/관찰 | 재사용된 관측 | 적용 가능한 결론 | 적용 불가능한 결론 |
|---|---|---|---|
| 비강성 DOPRI5 | 90 설정, 360 실행. sampled error cap `1e-6`에서 native RODAS 대비 약 2.73–8.93배의 국소 관측 | 비강성 기본 경로에 불필요한 Jacobian/분해 비용을 부과할 이유가 없다. | 모든 함수·차원에서 같은 배수, 외부 최강 솔버 대비 우위 |
| DOPRI5의 강성 negative control | 31,014 시도, 186,085 RHS 호출, native RODAS가 약 277배 빠름 | 강성 문제에 explicit를 무조건 고집하는 정책을 제외한다. | 실패/작은 스텝만으로 강성을 판별하거나 미분을 강제할 권한 |
| implicit pilot | rapid-JVP 약 9.46배 개선; FD와 전이 설정에서 손실; 120행 중 105행이 `1e-5` cap 통과 | session 재사용과 전환 비용 측정에 개발 가치가 있다. | 비용 개선이 모든 regime에 유지됨, global no-regret |
| affine quotient Lawson | 선택된 stiff/rapid/shared128에서 full-RHS DOPRI 대비 약 4.36/22.71/1.58배; zero/slow/generic-forcing에서는 약 4.25/5.09/44.58배 손실 | 고정 affine/harmonic 선언이 있는 호출만 명시적 특수 API로 유지 | 일반 RHS 자동 치환, 약한 비교군에 대한 큰 배수를 전체 우위로 전용 |
| 최신 긴 주기 LP01 | 2차원 선형 동차 3문제, 100/1000주기, 4,097 관측점; 보충 실행 36/36 sampled cap 통과 | 해당 세 문제의 표본 상태·위상·진폭 검증 범위 확대 | 시간 권위는 `diagnostic_only`, `INCONCLUSIVE_TIMING`; Lawson 속도 승격 불가 |
| causal-forward | residual-first coupled 대비 7개 그룹 중 6개에서 약 1.17–2.79배; broad-spectrum은 0.99856배, 35시도 중 33fallback | 해당 coupled 내부에서는 작은 기계용 serial-forward가 우선 | 보호된 전체 솔버를 이겼다는 결론. 이전 few-mode coupled/economy 비교는 speed ratio 0.422였다. |
| 두 스레드 causal | 84 bitwise 비교쌍, 42 동일 설정; 시험한 작은 RHS에서 1.2배 유의한 이득 없음 | 단일 trajectory에서 무조건 thread pool을 켜지 않는다. | 비싼 RHS의 병렬화가 본질적으로 불가능하다는 결론 |
| 저차 다항식 resolvent | 4 kernel 예에서 1/1/2/2 operator actions; Neumann은 20회 성공 또는 65회 후 실패 | 증명된 저차 관계가 있으면 kernel 작업을 줄일 수 있다. | 전체 ODE 실행시간·오차 개선, 임의 operator의 자동 저차 환원 |

이 수치는 다른 소스와 실행 프로토콜에서 나온 결과다. 합쳐서 하나의 평균 speedup이나 전체 순위를 만들지 않는다. 같은 입력 `rtol`도 같은 실제오차를 뜻하지 않는다. 현재 자료의 표본 cap 검증도 연속시간 최대오차 증명이 아니다. 최신 LP01의 grouped pilot과 cyclic supplemental timing은 함께 pooling하지 않는다.

## 선택 계약과 실패 의미

새 선택기는 먼저 사용자가 요청한 출력, 질량행렬, 허용오차 형태와 미분 정책을 확인해야 한다. 공개 `Rodas5p::capabilities`는 사용자 callback 호출이나 workspace 할당 없이 지원 조합을 조회한다. 이 판단을 재사용하고, 별도 선택기가 더 넓은 기능을 묵시적으로 약속하지 않도록 한다.

* 비강성 경로는 `Dopri5`다. RHS 평가만 사용하고 Jacobian, 행렬 분해, Newton 반복을 수행하지 않는다. FSAL 재사용은 같은 문제와 마지막 수락 상태가 유지될 때만 허용한다.
* 강성 경로는 명시적 사용자 허가가 있는 RODAS5P다. dense/banded Jacobian은 행렬을 만든다. 유한차분도 미분 근사다. supplied-JVP 경로는 전체 Jacobian을 만들지 않지만 미분 작용과 Krylov basis는 사용한다. 현재 구성을 완전 derivative-free 강성 솔버라고 표현하면 안 된다.
* banded 경로의 복잡도는 bandwidth에 의존한다. 고정 bandwidth 가정 없이 `O(N)`이라고 표현하지 않는다. 질량행렬이나 특정 dense-output 설정이 sequential dense route로 이어지면 원래 대역 선언이 있어도 실제 dense factorization 비용을 기록한다.
* JVP와 mass/dense output, JVP와 band, 지원되지 않는 vector atol 조합은 기존 capability refusal을 유지한다. 요구 기능을 버리고 더 빠른 알고리즘을 반환해서는 안 된다.
* 현재 façade 이벤트는 적분 후 연속확장에 대한 후처리다. terminal event가 있다고 callback 실행이 즉시 멈추는 것으로 설명하지 않는다. 새로운 online terminal 의미를 제공하려면 실제 실행 제어와 시험이 필요하다.
* 작업 예산 실패, 비유한 결과, callback fatal과 특수 구조 위반은 성공으로 둔갑시키지 않는다. 수락된 마지막 상태와 이미 소비한 작업을 유지한다. fallback이 허가되었을 때만 그 비용까지 포함해서 수행한다.

## 전체 알고리즘군의 처분

`PORTFOLIO.json`의 각 항목은 실제 API/모듈, 근거 키, 범위, 다음 검증을 포함한다. 여기의 '핵심'은 제한된 선택기 후보라는 뜻이며 unrestricted production 인증이 아니다.

| 구분 | 알고리즘군 | 처분과 구체적 이유 |
|---|---|---|
| 핵심 | DOPRI5, protected dense RODAS5P, declared-band RODAS5P, supplied-JVP RODAS5P | 선언과 capability에 따라 선택. 비강성/강성의 비용과 안정성 역할을 분리한다. |
| 구조 선언 특수용 | compile-time small-N, affine/harmonic quotient, causal-forward, 독립 trajectory ensemble | 명시적 호출과 시험된 범위 유지. 전체 평균에 근거한 자동 선택 금지. |
| 구성요소만 특수용 | 저차 resolvent, collective low-rank/packet | 정확한 구조 및 원 residual을 확인. packet의 강한 비교 대상에는 Woodbury/collective-direct를 포함한다. |
| 최적화 후보 | switching pilot | persistent session을 연결한 다음 중복 적분·초기화 비용을 측정. 실패 비용을 제거한 숫자로 비교하지 않는다. |
| 연구용 | generic Lawson, SABR, homotopy/nonlinear/actual-K, transactional Q1/Q2·lagged, exponential Krylov/pexprb | 현재 end-to-end 실제오차·총시간 자료가 범용 채택을 지지하지 않는다. 명확한 workload와 강한 비교 대상이 생길 때만 다시 연다. |
| 다항식 구성요소 연구 | Laguerre, Chebyshev/Taylor/router | component의 수학과 시험을 재사용하되, 차수 감소를 전체 속도 개선으로 간주하지 않는다. 기존 polynomial router의 유효 이득 영역이 없다는 결과를 유지한다. |
| 현 영역의 자동 채택 제외 | Leja, 비수축 Neumann, cheap-RHS parallel Picard | Leja는 시험 영역에서 Chebyshev보다 비쌌고 total floating certificate는 EstimateOnly다. Neumann은 contraction 조건이 없으면 선택하지 않는다. Picard의 병렬 오버헤드는 현재 비용을 상회했다. |
| 참조 전용 | 내부 BDF1/BDF2, Radau1/Radau3 | 검증용으로 유지. 성숙한 VODE/DVODE, diffsol, Julia를 대체하는 비교 대상으로 간주하지 않는다. |
| 지원 도구 | continuous residual, global/physical budget, affine norm | 필요 가정이 있는 선택적 정확도 서비스. 독립 솔버 또는 자동 가속 후보가 아니다. |
| 이번 선택 밖 | graph/Markov/RKL, remap 및 특정 PDE 응용 | 일반 ODE 출시의 필수조건이나 speedup 근거로 사용하지 않는다. 이번에 BASS/rei 연구를 추가하지 않는다. |

export가 많다는 사실은 지원 가능한 생산 알고리즘 수가 많다는 뜻이 아니다. controller, norm, output, verification, model fixture, candidate ledger는 독립 algorithm family가 아니므로 별도 솔버 수로 세지 않는다. 최종 대외 설계문서에는 선정된 DOPRI5/RODAS5P 사양과 해당 지원 서비스만 기술하고, 위 연구 후보 나열은 내부 개발 리뷰에 남기는 편이 정확하다.

## 새 선택 코드의 독립 검토

이번 작업의 `crates/vigilode/src/selection.rs`는 위 최소 구성에 맞춰 `GeneralOde`를 구현한다. 기본값은 `Unknown`, 미분 금지, dense dimension resource ceiling 64, matrix-free 비활성이다. 64는 실측 crossover가 아닌 사용자 변경 가능한 자원 제한이다. mass, events, dense output은 먼저 거절한다. 기존 method-specific API는 계속 사용할 수 있다.

소스와 `production_selection_contracts.rs`를 독립적으로 읽은 결과, 검사한 dispatch 논리에서 차단해야 할 결함을 찾지 못했다. dense callback에 band가 선언되어도 dense 행렬 생성에 대한 ceiling을 적용한다. unknown/nonstiff는 implicit로 자동 전환하지 않는다. `ProvidedOnly`는 유한차분 Jacobian과 비자율 문제의 누락된 시간 편미분을 거절하며, JVP는 별도 허가와 기존 vector-atol 거절을 유지한다. 실패한 부분 결과도 typed error에 보존한다. 이 서브분석은 Rust 시험을 새로 실행하지 않았다. 실행 결과와 전체 회귀범위는 통합 보고서에서 분리해 기록해야 한다.

이 선택기의 소스 검토 통과는 일반 stiffness detection, 전역 정확도 또는 성능 향상의 증명이 아니다. 선택 전용 wrapper는 보호된 기존 엔진을 호출하므로 이번 변경의 직접 효용은 허가되지 않은 경로와 근거 없는 후보의 자동 진입을 막는 데 있다.

## 조건부 계산량 개선과 switching의 손익분기점

정상적인 동일 길이 구간에서 explicit 비용을 `E`, implicit 비용을 `I`, pilot/재시작 추가비용을 `P≥0`라 하자. 모든 비교가 같은 출력과 실제오차를 만족하고 다음 `K`구간에서 이 비용이 유지된다고 가정한다. 전환 정책과 explicit 유지 정책의 총 비용 차는

\[
C_{\rm switch}-C_{\rm explicit}=P+K I-K E.
\]

따라서 `E>I`이면

\[
K > \frac{P}{E-I}
\]

에서만 엄밀한 비용 이득이 생긴다. `E≤I`이면 양의 `P`를 더한 switching이 이 모형에서 이길 수 없다. 증명은 위 차의 부호를 계산하는 것으로 끝난다. 이는 실제 문제의 미래 비용이 일정하다는 증명이 아니며, callback wall time을 수학적으로 상계하지도 않는다. 구현에는 미래의 이득 구간 길이에 대한 근거가 없는 짧은 pilot만으로 자동 전환하지 않는 이유를 제공한다. session 재사용은 `P`를 줄이는 직접적인 개선이다.

각각 `C_F`의 RHS batch 비용을 가진 serial stage `m`개를, `p`개 worker로 `r`번 sweep하는 후보를 생각하자. batch가 이상적으로 나뉘어도 synchronization과 부가 작업을 `C_O≥0`라 하면 실제 구현의 한 단순 비용 모형은

\[
C_{\rm par}=r\lceil m/p\rceil C_F+C_O,
\qquad C_{\rm seq}=m C_F.
\]

측정된 `C_O`에서 `m-r ceil(m/p)>0`일 때만 `C_F>C_O/[m-r ceil(m/p)]`가 이 모형의 개선 조건이다. 그렇지 않으면 이 모형으로 이득을 보장할 수 없다. 이 불등식은 speculative sweep을 무료로 세지 않는 출발점이다. 독립 ensemble의 throughput 가속을 이 단일 trajectory latency 식과 혼동하지 않는다.

저차 관계 `q(B)=0`과 `q(1)≠0`가 실제 operator 전체에 성립하면 `q(1)-q(z)=(1-z)r(z)`로부터

\[
(I-B)r(B)=q(1)I-q(B)=q(1)I
\]

이므로 `(I-B)^{-1}=r(B)/q(1)`다. `degree(q)=d`이면 `r`의 차수는 `d-1`이며 Horner 평가는 `d-1`회의 operator action을 사용한다. 실제 구현에서 근사 관계만 있으면 `q(B)`의 norm, `q(1)`의 분리 및 계산된 원 residual을 별도 확인해야 한다. 이 정리는 미지의 임의 행렬에 낮은 차수 `q`가 존재한다는 보장을 제공하지 않으며, 2-state provider의 성공을 범용 solver로 승격시키지 않는다.

## 다음 개발 순서와 종료 기준

1. **이번에 구현한 작은 선택 API의 계약을 유지한다.** DOPRI5와 protected RODAS5P 외 후보는 자동 경로에 넣지 않는다. 선택 자체가 callback을 호출하지 않는 시험, 허가하지 않은 미분을 쓰지 않는 시험, 지원 불가능한 output/mass/atol 조합을 거절하는 시험이 우선이다. 기존 직접 API는 보존한다.
2. **지속 실행 또는 switching을 다음 출시 기능으로 채택할 경우 두 엔진의 실행 수명을 연결한다.** endpoint 전용 출시의 필수조건은 아니다. 최신 native session 결과를 façade에 연결하고, continuation과 uninterrupted 실행의 결과 및 비용을 비교한다. terminal callback, accepted state, FSAL/Jacobian cache, output ordering을 함께 검증한다. 한 driver의 성공을 switching 전체 완료로 기록하지 않는다.
3. **현재 소스로 제한된 광범위 비교를 한 번 등록한다.** 비강성 smooth, stiff decay, stiffness transition, oscillation/decaying oscillation, 비정규 선형, 비선형 smooth, 저차원과 큰 벡터를 포함한다. 비교 대상은 VODE/DVODE, diffsol, Julia로 제한한다. NCP는 사용하지 않는다. 단일 스레드부터 측정하고, 실제 retained output 오차를 만족하는 configuration끼리 setup과 출력 비용을 포함해 interleave한다. 실패·timeout을 삭제하지 않는다.
4. **한 가지 구체적 특수 구조에서만 재진입시킨다.** 예를 들어 affine/harmonic이나 expensive-RHS causal이 이길 수 있는 실제 사용 사례가 있을 때 가장 강한 적용 가능 baseline과 비교한다. 진입에 reference solution을 몰래 사용할 수 없다. LP01의 inconclusive timing은 승격 증거로 사용하지 않는다.
5. **프로파일에 근거해 메모리 트래픽을 줄인다.** DOPRI 큰 벡터의 반복 순회, finite 검사, 배치 구성과 band/JVP 작업공간을 측정한다. correctness 검사를 무단 삭제해서 나온 속도는 채택하지 않는다. 비용 감소는 해당 전체 trajectory에서 실제오차와 함께 다시 확인한다.

이후 특수 후보를 기본 경로에 넣는 공학적 기준으로, 미리 선언한 영역의 반복 median speed ratio가 1.2 이상이고 지원되는 control에서 2배 넘는 손실을 피하도록 제안한다. 이 수치는 현재 증명된 통계적 우위나 보편적인 최적 임계값이 아니라 앞으로 사용할 정책 제안이다. 성능 이득을 지지하는 자료가 없으면 단순히 'optimization possible'이라고 붙이지 않는다. 실제 병목과 구체적인 개선 경로가 있는 경우에만 그 분류를 사용한다.

## 재현과 자료 식별

```sh
python research/general_ode_selection_20261024/src/analyze_portfolio.py
```

검사는 13개 재사용 파일의 내용 hash와 25개 알고리즘군의 정책 일관성, 3개 후보 정책, LP01의 `diagnostic_only`를 확인한다. 솔버를 실행하지 않고 native selection 동작을 검증하는 시험도 아니다. source identity와 정책 분석의 재현성만 확인한다. 코드 변경에 대한 별도 회귀시험 결과는 통합 보고서에서 확인해야 한다.

근거 키의 경로와 SHA256은 [PORTFOLIO.json](PORTFOLIO.json)의 `sources`에 있다. 본 문서의 새 수치 추론은 손익분기식과 조건부 polynomial action 수뿐이며, 이전 측정값으로 새 runtime 승격을 선언하지 않는다.
