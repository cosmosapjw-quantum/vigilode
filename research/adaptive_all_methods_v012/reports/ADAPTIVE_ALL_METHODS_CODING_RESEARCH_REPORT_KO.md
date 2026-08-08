# 모든 현재 method의 adaptive step — 코딩 연구 보고서

Date: 2026-08-08
Branch: `adaptive-all-methods-v0.12`
Baseline: `unified-fair-measurement-v0.11.0-alpha1`

## 1. 결과 요약

수학/물리 연구 루프에서 estimator, controller, transactional state, cost ledger와 external error를 분리한 뒤, 코딩 연구 루프에서 현재 실행 가능한 모든 complete-integrator family에 adaptive requested-output API를 추가했다.

대상은 다음 열 개 candidate다.

1. Sequential RODAS5P/direct
2. Sequential RODAS5P/GMRES
3. Sequential RODAS5P/LGMRES
4. Sequential RODAS5P/GCRO-DR
5. SABR5P
6. Homotopy RODAS5P q=7 reference
7. BDF1 reference
8. BDF2 reference
9. Radau IIA1 reference
10. Radau IIA3 reference

Canonical screen은 4개의 analytic problem과 3개의 tolerance에서 120개 run을 만들었고, 120개 전부 성공했다. 40개의 candidate–problem tolerance sequence 모두에서 tighter tolerance가 external maximum-grid L2 error를 증가시키지 않았다.

## 2. 공통 controller

공통 state machine은 acceptance와 step proposal만 담당하고, estimator는 method family가 제공한다. 무차원 WRMS error를 \(E\), estimator order를 \(p_e\)라 하면 integral proposal은

\[
\eta_I
=\mathrm{safety}\,E^{-1/p_e}
\]

이며, PI lane은 직전 accepted error만 사용한다. Rejection은 PI history를 갱신하지 않고 별도의 shrink cap을 적용한다.

이 분리는 모든 method에 동일한 exponent를 억지로 적용하는 오류를 피한다.

## 3. RODAS5P, SABR, Homotopy

이 family들은 이미 존재하는 RODAS5P 5(4) embedded endpoint error를 primary estimator로 유지한다. Homotopy는 algebraic output certificate를 함께 사용하고, certificate 또는 speculative path가 실패하면 protected sequential RODAS5P로 transactionally fallback한다.

이번 Homotopy configuration은 q=7 correctness-heavy lane이며 fast-path 성능 후보가 아니다.

## 4. BDF

BDF2는 step ratio

\[
r=h_n/h_{n-1}
\]

를 갖는 정확한 nonuniform formula를 사용한다.

\[
\frac{1+2r}{1+r}y_{n+1}
-(1+r)y_n
+\frac{r^2}{1+r}y_{n-1}
=h_nf_{n+1}.
\]

Startup은 BDF1이고, adaptive reference estimator는 one coarse step과 two half steps의 차이를 사용한다. Accepted history는 midpoint와 endpoint를 포함하는 two-half trajectory에 맞춰 commit된다.

## 5. Radau

Radau IIA1과 three-stage fifth-order Radau IIA의 fixed kernels를 보존하면서 같은 step-doubling reference wrapper를 추가했다. Method order \(p\)에 대해 estimator order는 \(p+1\)이다. Full coarse/fine nonlinear work와 rejected work를 모두 장부에 포함한다.

## 6. Canonical 결과

```text
runs                         120
successes                    120
failures                       0
candidate families            10
analytic problems              4
tolerances        1e-4, 1e-6, 1e-8
nonincreasing sequences       40 / 40
scientific checksum
3147e9e891c7da462eb07deacf7caee800875dbe067649820c77757cd108bc0e
```

BDF1/Radau IIA1과 BDF2는 tolerance가 작아질수록 external error가 감소하고 work가 증가하는 전형적인 adaptive response를 보였다. High-order candidate들은 requested output spacing에 의해 step이 0.04 또는 0.02로 clip되어 이미 \(10^{-12}\) 수준에 도달했다. 따라서 이 corpus에서는 high-order family의 tolerance sensitivity와 성능 순위를 판단할 수 없다.

## 7. Rayon

One-thread와 four-thread 결과는 scientific field가 exact-identical하다.

```text
T1  0.072130858 s
T4  0.024106208 s
ratio 2.992210886x
```

이 수치는 독립 case throughput이며 solver 내부 stage/linear algebra speedup이 아니다. Cross-family wall ranking에는 randomized repeated timing campaign이 추가로 필요하다.

## 8. 판정

### PROMOTE

공통 adaptive contract와 reference implementation layer를 연구 정본으로 승격한다.

### HOLD

어떤 family도 이 결과만으로 production/default 또는 fastest solver로 승격하지 않는다. Step-doubling BDF/Radau와 q=7 Homotopy는 correctness reference이고, 공통 output-grid clipping과 single-pass timing이 성능 주장을 차단한다.

## 9. 다음 질문

다음 연구는 “adaptive API가 존재하는가”가 아니라 다음을 물어야 한다.

> 각 method의 native estimator와 controller를 사용하고 output-clipping bias를 제거했을 때, 동일 external global error에서 어떤 family가 total work와 wall time의 Pareto frontier를 차지하는가?
