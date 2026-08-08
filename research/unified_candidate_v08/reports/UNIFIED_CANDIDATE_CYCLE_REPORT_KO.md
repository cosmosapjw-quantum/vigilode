# RODAS5P 전체 후보 통합 코딩 연구 cycle v0.8

## 1. 목적

이번 cycle의 목적은 특정 알고리즘을 유리하게 만드는 것이 아니라, 지금까지 구현된 모든 RODAS5P 관련 경로와 앞으로 구현할 완전한 적분기 family를 하나의 후보 registry와 판정 규약 아래에 넣는 것이었다.

기존에는 세 종류의 비교가 섞일 위험이 있었다.

1. 동일한 순차 RODAS stage equation을 다른 선형 solver로 푸는 비교;
2. SABR/homotopy처럼 stage propagation 자체를 바꾸는 비교;
3. BDF/Radau/Peer/SDC/ROK처럼 시간적분법 전체가 다른 비교.

이번 하네스는 이를 Tier L, Tier N, Tier I로 분리하면서 최종 판정 형식은 하나로 통일한다.

## 2. 후보 공간

총 163개 registry entry를 고정했다.

| family | 수 | 상태 |
|---|---:|---|
| sequential RODAS5P | 8 | executable |
| SABR5P | 4 | executable |
| partial-coupling homotopy | 144 | executable |
| BDF | 1 | deferred |
| Radau/IRK | 1 | deferred |
| Peer/W | 1 | deferred |
| parallel SDC | 1 | deferred |
| Rosenbrock–Krylov | 1 | deferred |
| BOROK | 1 | deferred |
| exponential/Leja | 1 | deferred |

Deferred는 누락이 아니라 의도적인 상태다. Rust 구현이 common step/output/work contract를 만족하기 전에는 수치 순위에 들어가지 않는다.

## 3. 동일 기준

모든 executable candidate에는 다음을 적용했다.

- official RODAS5P tableau와 동일 mass-matrix/sign convention;
- 동일 WRMS scale와 output budget;
- 동일 protected direct C0 oracle;
- 동일 C1/C2 correction diagnostics;
- 동일 C3 refined-root reference;
- 동일 global fifth-order qualification;
- 동일 stiff-decay와 nonnormal/noncommuting-mass screen;
- fallback-inclusive `WorkCounters`;
- candidate compute와 certificate compute 분리;
- 동일 local Rayon scheduler;
- 1/4-thread scientific identity;
- 최종 promotion에 최소 1.15x median wall speedup 요구.

## 4. blocker 해결

### 해결된 하네스 blocker

- solver별 서로 다른 acceptance 기준;
- fallback 비용 누락;
- C3 iteration exhaustion으로 인한 unclassified row;
- batch depth만으로 발생하던 false promotion;
- GCRO-DR scalar QZ panic;
- GMRES happy-breakdown threshold mismatch;
- roundoff floor 이후 order 오판;
- thread-dependent scientific ordering;
- unimplemented family의 순위 혼입.

### 남은 과학 blocker

- cheap online nonlinear output certificate;
- actual stage-batch concurrency;
- sparse/large-PDE scaling;
- index-1 DAE common gate;
- deferred seven families의 native Rust implementation;
- complete-integrator tolerance/work matching.

즉 “모든 blocker를 숨김없이 분류하고 하네스 blocker를 해소”한 것이지, 모든 알고리즘의 과학적 문제를 해결했다고 주장하지 않는다.

## 5. canonical 결과

One-thread canonical result:

```text
catalog entries          163
Tier-N cases               6
Tier-N rows              936
completed                349
rejected                 587
uncertified                0
numerical failures         0
C0 reference fallbacks     4
```

Scientific-only gates:

```text
reference   1
promote    23
hold      132
deferred    7
```

Measured Tier performance를 결합한 authoritative joint verdict:

```text
reference   2
promote     0
hold      154
deferred    7
```

Scientific-only PROMOTE는 “현재 finite correctness screen 통과”를 뜻한다. 성능 승격이 아니다. 최종 joint 결과에서 승격된 비기준 후보는 없다.

## 6. 선형 solver 결과

Tier-L 기준은 GMRES/OFF다.

- LGMRES/OFF: 약 1.06x wall speedup, 그러나 1.15x gate 미달 및 operator work 증가;
- GCRO-DR STAGE/PERSISTENT: operator work는 줄지만 wall time은 증가;
- GCRO-DR OFF: operator work와 wall 모두 불리.

이는 Krylov recycling이 무효라는 뜻이 아니다. GCRO-DR의 원래 목적은 변화하는 선형계 sequence에서 유용한 subspace를 재사용하는 것이며, 실제 이득은 subspace persistence와 orthogonalization/update overhead의 균형에 달려 있다. 현재 small dense Rust screen에서는 그 균형이 불리했다.

## 7. SABR 결과

SABR 네 variant는 global order/stiff/nonnormal finite screen을 통과했지만:

- batch depth ratio: 0.25;
- batch vector ratio: 2.0;
- RHS evaluation ratio: 3.125;
- JVP-vector ratio: 약 5.7–21.7;
- candidate time ratio: 약 2.3–5.6.

따라서 batch call depth 감소만으로는 promotion하지 않았다. 실제 concurrent multiple-RHS backend가 붙은 뒤 다시 평가해야 한다.

## 8. homotopy 결과

144개 중 12개의 `theta=1,q=7` configuration이 현재 finite scientific screen을 통과했다. 그러나 이들은 correctness oracle에 가까운 고비용 경로다.

- median batch depth: 24–56;
- median batch vectors: 192–448;
- RHS/JVP work: sequential보다 훨씬 큼;
- measured wall speedup: 1 미만.

낮은 q/depth 후보는 일부 one-step case에서 성공하지만 global order, stiff, nonnormal, 또는 C3 availability를 동시에 만족하지 못했다.

C3가 iteration budget을 소진한 4개 severe homotopy row는 C0 oracle로 분류해 모두 거부했다. 연구 결과를 완결했지만, deployable certificate blocker는 남아 있다.

## 9. 병렬 실행

Rayon local pool에서 independent case를 병렬화했다.

```text
1 thread: 4.17 s
4 threads: 3.11 s
speedup : 1.341x
```

Scientific checksum은 동일하다. 이 결과는 case-level throughput 개선이며 stage-parallel speedup의 증거는 아니다.

## 10. 선행 방법과의 관계

- GCRO-DR: changing matrix/RHS sequence의 recycled subspace가 핵심이며, wall-time 이득은 update/orthogonalization overhead를 포함해 판단해야 한다. DOI `10.1137/040607277`.
- Rosenbrock–Krylov: Krylov approximation을 시간적분 order theory에 내장한 별도 family다. DOI `10.1137/130923336`.
- parallel SDC: diagonal/nonstationary sweeper가 stability와 efficiency를 지배하며, 최근 nilpotent error-propagation product가 제안됐다. DOI `10.1137/24M1649800`.
- PETSc TS: BDF/IRK/Radau 계열과 SNES/KSP infrastructure를 제공한다. 향후 외부 control에는 유용하지만, 현재 pure-Rust fair baseline과 섞어 즉시 순위를 내지는 않는다.

## 11. 결론

이번 cycle에서 가장 중요한 결과는 특정 solver의 승리보다 판정 기준의 통일이다.


a) 정확성만 통과하면 성능 후보가 될 수 있지만 promotion되지는 않는다.

b) batch depth, scalar work, wall time을 서로 대체 가능한 지표로 취급하지 않는다.

c) C3 failure는 숨기지 않되 C0 offline oracle로 연구 row를 분류할 수 있다.

d) unimplemented family는 registry에 남기되 rank하지 않는다.

현재 authoritative verdict는 “비기준 승격 0개”다. 다음 단계는 이 판정 규약을 바꾸는 것이 아니라, deferred family를 동일 Rust contract로 구현해 후보 공간을 실제로 확장하는 것이다.
