# G4-S5B1 Asymmetric Transactional Shadow Research Report

## 1. Current DAG state

```text
G4-S5B0 expanded paired regime atlas        COMPLETE / PASS
G4-S5B1 asymmetric transactional shadow     COMPLETE / HOLD activation
G4-S5B2 startup economic admission          OPEN
active method switching                     NOT AUTHORIZED
N=2048 sealed holdout                       UNOPENED
proper homotopy third mode                  DEFERRED
BDF/Radau optimization                      FROZEN
physical-client execution                   NOT PERFORMED
```

이번 node는 numerical source를 바꾸지 않고, G4-S1과 G4-S5B0의 immutable campaign을 결합한 read-only replay다.

## 2. CLEAN_CONTEXT

### 중심 질문

E-K와 R-JF가 상보적이고 긴 residence가 존재한다는 사실이 확인된 뒤, 다음 두 asymmetric policy 중 어느 것이 fallback-inclusive total cost까지 포함해 active-switch 후보가 될 수 있는가?

- R-JF-default, event-triggered E-K entry
- E-K-first, local inadmissibility 시 protected R-JF fallback

### 고정 contract

\[
L_{\min}=6,
\qquad
m_{\rm econ}=1.05,
\]

\[
\eta_{\rm flow}\le 2,
\qquad
\delta_{Jv}\le 10^{-6}.
\]

- failed speculative work retained
- routine dual prefix forbidden
- active solution switching forbidden
- BDF/Radau byte-frozen
- physical clients excluded

## 3. P0 metric correction

초기 provisional analysis는 각 method의 현재-step raw wall을 직접 비교했다. 그러나 hard corpus에서 E-K와 R-JF는 서로 다른 다음 step size를 제안한다. Method economics의 올바른 비교량은

\[
\Gamma_m=\frac{T_m}{\widehat h_m}
\]

이며, 한 RODAS accepted interval \(h_R\)를 덮는 예상비용은

\[
C_{m\to h_R}=\Gamma_m h_R
\]

이다.

이 수정은 headline detail이 아니라 load-bearing correction이다.

```text
hard accepted-step rows      258
winner-label flips            24
flip fraction               9.30%
raw E → unit-time R            5
raw R → unit-time E           19
```

따라서 raw current-step wall로 얻은 policy ranking은 authority에서 제외했다.

## 4. Data and policy construction

### Calibration

- G4-S1: complex Dahlquist, quadratic, linear→nonlinear blend
- G4-S5B0: six N=128 transition trajectories

### Holdout

- G4-S1: oscillatory PR, advection–diffusion, nonlinear→linear blend
- G4-S5B0: six N=512 transition trajectories

각 split에는 12 trajectories가 있다. 7개 paired timing campaign의 scientific/work fields는 exact-identical하다.

### Prefix accounting

- Atlas E-K prefix: G4-S5B0 actual two-vector prefix timing
- Hard-corpus E/R prefix transfer: G4-S4 selected-prefix P95 fraction을 conservative하게 사용
- selected prefix is reused
- discarded prefix and failed complete trial are charged

### Calibration-only grid

```text
cooldown       3, 6, 12
review interval 6, 8, 12
```

Gate violation score를 먼저 최소화하고, 그 뒤 regret와 overhead를 비교했다.

Selected policies:

```text
R-default: cooldown 6, interval 6
E-first:   cooldown 12, interval 12
```

## 5. Execution result

### R-JF-default

| Split | Median regret | P95 regret | Median overhead | P95 overhead | Aggregate speedup |
|---|---:|---:|---:|---:|---:|
| Calibration | 1.121 | 1.664 | 0.00% | 1.87% | 2.416x |
| Holdout | **1.316** | **5.458** | 0.00% | 1.71% | 2.369x |

Holdout failure geometry는 명확하다. `advection-diffusion` trajectories는 accepted step이 3개뿐이라 \(L_{\min}=6\)에 도달하지 못한다. E-K가 3.84x와 7.44x 싸지만 probe가 한 번도 열리지 않아 policy regret가 그대로 3.84와 7.44가 된다.

따라서 residence threshold는 thrashing을 막지만, 짧고 강하게 favorable한 startup regime을 놓친다.

### E-K-first

| Split | Median regret | P95 regret | Median overhead | P95 overhead | Aggregate speedup |
|---|---:|---:|---:|---:|---:|
| Calibration | 1.141 | 2.506 | 2.84% | 7.34% | 2.540x |
| Holdout | **1.072** | **1.570** | 2.61% | **7.67%** | 2.538x |

Holdout median은 좋지만 van-der-Pol/forcing transition에서 fallback/review tail이 남는다. P95 regret와 P95 overhead가 모두 gate 밖이다.

### Safety-only E-first diagnostic

이 lane은 activation candidate가 아니다. Flow/JVP safety가 허용하면 E-K를 시도하고, E-K가 inadmissible하면 해당 full work를 보존한 뒤 R-JF로 fallback한다.

| Split | Median regret | P95 regret | P95 overhead | Aggregate speedup |
|---|---:|---:|---:|---:|
| Calibration | 1.058 | **2.506** | 16.14% | 2.990x |
| Holdout | **1.000** | **1.137** | 13.69% | 2.971x |

Holdout에서는 safety feature가 oscillatory PR를 R-JF로 보내고 E-friendly cases를 E-K로 보내므로 regret가 거의 닫힌다. 그러나 calibration quadratic에서는

```text
flow defect       about 0.036–0.040
JVP quality       about 8e-9
E admissibility   true
E/R economics     E-K about 2.4–2.6x more expensive
```

이므로 safety는 통과하지만 economics는 실패한다.

## 6. Main scientific conclusion

현재 selector problem은 더 이상 broad regime detection이 아니다.

\[
\boxed{
\text{flow/JVP/admissibility는 safety를 판별하지만}
\quad
\text{E-admissible 상태의 economics는 판별하지 못한다.}
}
\]

특히 다음 두 상태를 구별해야 한다.

```text
quadratic-like:
  E-K safe and admissible, but R-JF cheaper

complex/advection/blend-like:
  E-K safe, admissible, and substantially cheaper
```

따라서 다음 정보는 trajectory 전체에서 계속 수집할 필요가 없다. **startup에서 한 번의 bounded economic admission**이면 충분할 가능성이 있다.

## 7. Campaign robustness

7개 campaign에서:

```text
R-default holdout P95 regret      3.902–5.558
E-first holdout P95 regret        1.552–1.623
safe-E holdout P95 regret         1.134–1.288
```

Timing noise가 policy verdict를 바꾸지는 않는다. R-default와 E-first는 각각 다른 gate에서 안정적으로 실패하고, safe-E diagnostic의 holdout safety/economic opportunity는 반복된다.

## 8. Final disposition

```text
G4_S5B1_METRIC_CORRECTION     PROMOTE
G4_S5B1_R_DEFAULT             HOLD
G4_S5B1_E_FIRST               HOLD
G4_S5B1_SAFE_E_DIAGNOSTIC     PROMOTE / DIAGNOSTIC ONLY
G4_S5B1_ACTIVE_SWITCHING      HOLD
```

Aggregate speedup이 2x 이상이라는 사실만으로 activation하지 않는다. Tail regret와 speculative overhead가 sealed gate를 통과해야 한다.

## 9. Refined next node

### G4-S5B2 Startup Economic Admission Gate

첫 accepted state에서:

1. flow/JVP safety가 E-K를 거부하면 R-JF 선택
2. safety가 통과하면 actual reusable Arnoldi/GMRES prefix를 각각 1–2 vector만 실행
3. conservative cost envelope가 E-K 15% 이상 우위면 E-K profile 선택
4. 그 외에는 R-JF로 abstain
5. 이후 periodic economic review 없음
6. E-K local inadmissibility는 즉시 protected R-JF fallback

이 node는 startup economic admission만 검증한다. Active switching, controller transfer, N=2048 holdout은 여전히 금지한다.
