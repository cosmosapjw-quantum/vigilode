# RHS-only homotopy 소비자의 production 구현과 대안 비교

## 실제 결과

기존 Vec 기반 scalar 적분을 스택 scalar 연산으로 특수화하고, 동일한
원래 polynomial 잔차를 검증하는 두 후보 생성기를 추가했다. 공개된 기존
`integrate_cubic_decay_accuracy`의 계산 경로는 그대로 유지한다. 새
`integrate_cubic_decay_accuracy_with_solver`에서 구현을 명시적으로 선택한다.

* `ScalarHomotopy`: 기존 scalar 연산 순서와 callback 횟수를 보존하면서
  stage마다 반복되던 작은 Vec 할당과 station-output 복제를 제거한다.
* `AlgebraicPredictor`: 차감 소거를 피하는 양의 합으로 유일한 cubic root의
  해석적 표현을 계산하고, 원래 outward residual로 승인한다.
* `RationalPredictor`: 정확히 유도한 유리식 `(1+2 beta)/(1+3 beta)`를 사용한다.
  cube root와 square root를 제거하며 같은 outward residual로 승인한다.

**23개 서로 다른 native 계약(기존17+새6), 최종24/24 정확도 arm이 통과했다.**
별도 Decimal80 계산으로 모든 반환 grid state의 오차가 native outward bound
안에 있음을 확인했다. 151개 exact rational cubic identity와 151개 rational
defect identity도 독립 계산했다. 수학적 일반 명제의 근거는 THEORY.md의 증명이고,
유한 계산은 구현·식 대응의 추가 검산이다.

## 실제 병목과 수정 귀속

PR91의 물리 오차 planner는 이미 강한 수축 영역의 격자를 선택한다. 이 영역에서
같은 seed의 Picard와 ResidualDominance가 같은 RHS 횟수를 쓰므로, Anderson
추가 기능을 이득의 원인으로 부풀릴 근거가 없었다. 대신 generic 값 한 개를
길이1 Vec 세 개와 station record로 표현하는 비용이 반복됐다.

Scalar 특수화는 네 기존 가속 모드 × 두 seed 정책 × 다섯 모델, 총40개 조합에서
generic 결과의 모든 state/error-bound bit와 계산량을 보존했다. 따라서 이
개선은 수학적 오차 예산이나 목표식을 바꾼 결과가 아니다.

해석적 cubic inverse는 RHS 호출을 줄였지만 cube root 비용이 남았다. Pass0의
강한 비선형 fixture에서 scalar/algebraic 시간비 중앙값은0.77055였다. 이 불리한
관측을 보존한 뒤 **수정 루프1회**에서 유리식 후보를 추가했다. Pass0 원자료와
소스·실행 receipt를 별도로 유지하며 사전 고정 결과로 바꾸어 설명하지 않는다.

## 이론에서 유도한 상수와 알고리즘

중점 stage를 `a m+b m³=y`로 쓰면 `a=1+hk/2`, `b=hg/2`이고,
`m=(y/a)s`, `beta=b y²/a³`로 정규화하여 `s+beta s³=1`을 얻는다.

양의 합으로 된 정확한 표현은

    z=(3/2)sqrt(3 beta),
    u=(z+sqrt(1+z²))^(1/3),
    s=3/(u²+1+u⁻²).

기존 수축 조건과 반환 상태의 물리 오차 bound에서 `beta<1/3`, `z<3/2`가
나온다. 계수3,3/2는 cube 항등식에서 유도된다. 유리식의 계수2,3은 inverse
series `1-beta+3 beta²+O(beta³)`에 일치하는 [1/1] 근사 조건이 유일하게 정한다.

    r=(1+2 beta)/(1+3 beta),
    r+beta r³-1 = beta³(3+8 beta)/(1+3 beta)³ >=0.

잔차 함수의 도함수가1 이상이므로 마지막 식은 `0<=r-s*<=defect`라는 전역
상계도 준다. 이 도함수는 증명에 쓰이며 runtime에서 도함수·JVP·AD·행렬을
계산하지 않는다. Wolfram 기호 검산은 두 항등식을 확인했다.

최종 승인은 기존 원래 stage `F(K)=K+h phi(y+K/2)`의 outward interval에서
유도한다. 후보 생성용 cbrt의 정확한 반올림을 가정하지 않는다. 후보가 예산을
통과하지 못하면 Scalar Picard와 기존 단조 bisection으로 보정하며, 모든
ordinary/interval polynomial evaluation을 과금한다. 정밀도 바닥에서 성공을
만들지 않고 이미 승인한 prefix만 반환한다. 후보 계산 횟수는 별도 telemetry다.

기존 `C=L² phi(Y)/8`, 최소 dyadic 격자, truncation을 제외한 잔여 예산,
state/time 반올림 항은 변경하지 않았다. 임의 margin이나 새로운 tuning
hyperparameter는 도입하지 않았다. Caller의 step/evaluation cap은 자원 입력이다.

## 동일한 물리 오차 예산에서 계산량과 정확도

아래 baseline은 warm Picard다. Scalar 특수화의 계산 횟수와 결과 bits는 baseline과
같다. Rational은 각 step의 유리식 후보 계산도 추가로1회 수행하므로, 이를 별도
열로 표시한다. Polynomial 횟수를 CPU 시간과 동일하게 취급하지 않는다.

| 물리 cap | N | baseline RHS+interval | rational RHS+interval | rational 후보 | rational 최대 grid 오차 | 최종 outward bound |
|---:|---:|---:|---:|---:|---:|---:|
| 1e-3 |64|134+64=198|64+64=128|64|1.43850e-5|2.83923e-4|
| 1e-4 |128|334+128=462|128+128=256|128|3.59356e-6|7.12522e-5|
| 1e-5 |512|1147+512=1659|512+512=1024|512|2.24475e-7|4.46606e-6|

위 모델은 k=1,g=1/2,Y=T=1이다. 가장 엄격한 cap에서 polynomial 평가는
38.28% 감소했다. Generic/Scalar는 같은 accuracy cap을 통과하지만 rational과
모든 state bit가 같지는 않다. 모든 반환점의 원래 물리 bound를 각각 통과한다.
별도 선형, 강한 비선형(k=1,g=20,T=.1), 영해 arm도 포함했다.

## 짧은 paired wall-time 비교

동일 measurement binary, 같은 plan, 각128회 전체 trajectory를 한 표본으로
묶은 5개 ABBA/BAAB block이다. Output Vec 할당과 모든 interval 검사는 시간에
포함하고 JSON 직렬화·oracle 계산은 제외했다. 아래 범위는 관측한 block의
최솟값–최댓값이며 신뢰구간이 아니다. 공유 환경의 변동을 그대로 남겼다.

| 모델 | 비교(앞/뒤 시간비) | 중앙값 | 관측 범위 |
|---|---|---:|---:|
| cubic | generic/scalar |1.728|.958–1.903|
| cubic | generic/rational |1.514|.716–1.666|
| cubic | scalar/rational |.970|.781–1.189|
| cubic | algebraic/rational |1.139|1.025–1.556|
| 강한 비선형 | generic/scalar |1.474|1.439–1.828|
| 강한 비선형 | generic/rational |1.670|1.102–1.767|
| 강한 비선형 | scalar/rational |1.097|.817–1.224|
| 강한 비선형 | algebraic/rational |1.168|1.128–1.846|

스택 구현은 기존 generic 경로의 구조적 overhead를 줄였다. 유리식은 비용이 큰
해석적 inverse를 개선했으며 RHS 횟수도 줄인다. 강한 Scalar Picard에 대한
유리식의 보편적인 시간 우위를 이 측정으로 주장하지 않는다. Production은
`ScalarHomotopy(Picard)`를 보수적인 선택으로, `RationalPredictor`를 검증된
모델 전용 대안으로 제공한다. 동일 binary의 Callgrind slope 비교는 parent의
통합 profiling 자료에 연결된다.

같은 최종 binary의20회/40회 Callgrind 차분에서 한 trajectory의 instruction
slope는 generic3,182,149.3, scalar2,028,310, algebraic2,061,615,
rational1,940,807이었다. Startup을 뺀 guest instruction 수로, generic 대비
scalar36.26%, rational39.01% 감소다. 강한 scalar 기준에서도 rational은4.31%
적은 instruction을 썼다. 이는 hardware cycle 측정과 구별한다.
`instruction_profile_r1/SUMMARY.json`과 원 callgrind 파일을 보존했다.
Generic40회 profile에서 malloc/free의 exclusive instruction 비중은 각각
6.63%/4.83%이고, 별도 libc 내부 연산도9.40%였다. 검증용 Interval::mul은33.65%를
차지했다. 작은 컨테이너 비용을 제거한 후에는 unchanged outward verification이
주요 남은 비용이다. 모든 시간·instruction 비교는 이 검증을 포함한다.

## 실행·한계·후속 PR

`PASS0_EXECUTION.json`, `PASS0_BENCH_EXECUTION.json`, `PASS1_EXECUTION.json`은
실제 명령·종료 코드·source/binary hash를 기록한다. `ANALYSIS_RECEIPT.json`과
`analyze.py`는 독립 검산을 재현한다. 모든 native 실행은 공유 flock 아래서 했다.

현재 조건부 production 사용은 선언된 scalar cubic 모델과 directed arithmetic
가정에 따른다. 일반 vector RHS에 전역 Lipschitz 상수를 붙이거나 continuation
branch를 인증하는 변화는 없다. Generic homotopy 및 기존 연구 경로는 유지된다.

후속 local PR의 과학적 목적은 다음과 같다.

1. 실제 hardware에서 Scalar/Rational을 동일 정확도로 비교하여 배포 선택을
   확정한다. 현재 raw timing을 보존하며 CPU topology·affinity·libm·compiler를
   기록한다. 옵션을 threshold로 자동 전환하는 별도 경험식을 추가하지 않는다.
2. Cubic planner의 유한-step 절단오차 상계를 실제 decay envelope와 결합하여
   최소 N을 줄인다. 새 상계의 전체 finite-step 증명과 outward 구현이 먼저다.
3. Generic vector homotopy의 재사용 workspace와 callback 계약을 설계하고,
   선형/비정규/분지 다중성 입력에서 회귀 검증한다. Scalar model 근거를 임의
   vector 수렴 보장으로 옮기지 않고 필요한 lemma를 각 PR 계약에 명시한다.
