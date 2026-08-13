# G4 실행계획 — EPIRK-K/W + Expanded Sealed Generic Holdout

## 목적

G3 pexprb54s4 정책을 더 이상 oscillatory-PR 결과에 맞춰 조정하지 않고 동결한다. G4는 두 작업을 분리한다.

1. frozen G3 candidate의 expanded sealed holdout 평가;
2. EPIRK-K/W를 별도 수학적 family로 구현·검증.

## G4-A — Freeze and seal

동결 대상:

- pexprb54s4 coefficients;
- full-MGS phi configuration;
- time controller parameters;
- phi tolerance split;
- current T1 default;
- same-error matching logic.

Holdout output 전에는 계약과 seed를 commit한다. Holdout 결과를 본 뒤 current pexprb threshold를 수정하면 그 결과는 exploratory로 강등한다.

## G4-B — Expanded generic holdout

최소 문제군:

- Robertson;
- HIRES;
- stiff van der Pol;
- rotating/temporally varying nonnormal operator;
- nonautonomous stiff forcing;
- semilinear diffusion/advection at larger dimensions;
- independent problem parameters/seeds not used in G3.

Dimensions for PDE-like problems:

\[
N\in\{128,512,2048\}.
\]

Record:

\[
E_{global}, E_{phase}, E_{amplitude}, N_f, N_{Jv}, N_{phi},
N_{Krylov}, N_{orth}, N_{accepted}, N_{rejected}, T_{wall}, M_{peak}.
\]

Within-step T4 is measured but not enabled by default. A calibration-only admission rule may be proposed only if a clear dimension/work crossover exists.

## G4-C — Cost-aware controller candidate

Traditional PI remains the frozen G3 control. A separate candidate may choose `h` under the accuracy constraint by minimizing measured cost per simulated time, because Krylov/phi cost depends on step size. It must be calibrated on a disjoint calibration set and evaluated unchanged on the sealed holdout.

## G4-D — EPIRK-K/W siblings

Implement separately:

- EPIRK-K4 correctness lane using its specific Krylov projection and order theory;
- EPIRK-W3 correctness lane allowing declared approximate operators;
- later high-order performance candidates only after the initial order gates pass.

Do not feed an arbitrary approximate Jacobian into pexprb54s4 and call it a W-method.

## G4-E — Promotion gates

For a generic primary candidate:

- target coverage `100%`;
- observed order `>= declared - 0.2`;
- explicit Jacobian builds `0`;
- primary Newton iterations `0`;
- false convergence `0`;
- median step ratio versus protected JF RODAS5P `<=1.2`;
- no individual holdout family with catastrophic step ratio or systematic miss;
- median same-error speedup `>=1.15`;
- repeated timing and T1/T4 scientific identity;
- bounded memory.

## Kill rule

If expanded holdout shows that polynomial Krylov dimension grows catastrophically with spectral radius, open a rational-Krylov research branch. Do not add rational Krylov before that evidence.
