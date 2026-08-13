# G4-S0 Plot-based CRAG Audit

## Generated plots

1. `G4_S0_STRICT_ORACLE_MAP.png`
2. `G4_S0_AUTO_ORACLE_MAP.png`
3. `G4_S0_ORACLE_GAIN_BY_CELL.png`
4. `G4_S0_FEATURE_SEPARABILITY.png`
5. `G4_S0_WINNER_STABILITY.png`
6. `G4_S0_JVP_PROBE_OVERHEAD.png`

## Plot reading

### Strict oracle map

E-K와 protected R-JF가 problem family별로 6:6으로 정확히 갈린다. Complex linear/semilinear spatial problem에서는 E-K, stronger nonlinear scalar/oscillatory problem에서는 R-JF가 이긴다. 단일 static method보다 portfolio가 유리한 visual evidence다.

### Auto oracle map

Frozen Radau가 quadratic, advection–diffusion 및 loose/moderate PR에서 강하다. 그러나 strict-JF requirement를 만족하지 않고 small-dense direct-J setting이므로, auto-profile의 upper-bound opportunity로만 읽어야 한다.

### Feature separability

Current zero-extra-work feature가 winner를 안정적으로 분리하지 못한다. Dimension이 in-sample에서 완전 분리하지만 problem identity를 사실상 encode한다. 이 plot은 active selector를 막는 negative evidence다.

### Winner stability

대부분 strict-JF labels는 7개 campaign과 ±10% timing mutation에서 안정적이다. Quadratic tight target의 empirical winner fraction은 약 0.714로 낮아 near-tie임을 보인다. 작은 predicted gain에 의한 switching은 위험하다.

### JVP probe overhead

3-RHS quality probe는 여러 cell에서 5% 선을 넘고 최대 12.5%다. 매-step feature로는 부적합하다.

## CRAG

### Correctness

Plots는 JSON oracle rows와 직접 생성됐고 target matching은 candidate tolerance ladder 전체에서 수행됐다. Colorbar는 protected RODAS 대비 speedup으로 일관된다.

### Retrieval

Offline method selection에서 end-of-step information과 low overhead를 쓰는 Petzold/LSODA 계열의 설계 철학과 정성적으로 일치한다. 다만 이 checkpoint는 actual switching을 구현하지 않았다.

### Augmented

±5%, ±10% timing mutation과 target multiplier 1.0/1.05/1.2를 적용했다. 주요 strict winner geometry는 유지됐지만 일부 near-tie cell은 불안정했다.

### Generation

다음 holdout에서는 accepted-step-local nonlinear defect와 measured switch cost가 추가되면 problem-name/dimension confounding을 줄일 것으로 예측한다. 반대로 이 telemetry 없이 active switching을 시작하면 unsafe selections 또는 thrashing이 생길 가능성이 높다.

## Claim status

- `Regime complementarity exists`: SURVIVING CLAIM.
- `Current free features are sufficient for safe switching`: REJECTED CLAIM.
- `Auto profile would be substantially cheaper on this corpus`: NARROWED CLAIM — offline small-dense upper bound only.
