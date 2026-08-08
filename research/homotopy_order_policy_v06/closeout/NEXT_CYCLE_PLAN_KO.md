# 다음 cycle — certified nonlinear output error branch

Planned branch: `homotopy-certified-output-v0.7`

## 목표

현재 first-order correction

\[
E_1=\left\|(b^T\otimes I)J_R^{-1}R(K)\right\|_{\rm WRMS}
\]

이 실제 nonlinear root/output defect를 과소평가하는 경우를 감지하고, holdout false accept
0을 달성하는 저비용 a posteriori certificate를 설계한다.

## 작업 순서

### C1 — frozen false-accept corpus

이번 canonical raw artifact에서 false-accept rows를 immutable fixture로 추출한다. 특히
manufactured-mass, Prothero–Robinson, q=7 and theta=1 cases를 보존한다.

### C2 — second-correction diagnostics

동일 target Jacobian 또는 refreshed Jacobian으로 second correction을 계산해

\[
\rho_{\rm out}=\frac{\|\Delta y_2\|}{\|\Delta y_1\|},
\qquad
\rho_R=\frac{\|R(K+\delta K_1)\|}{\|R(K)\|}
\]

을 기록한다. `rho >= 1`, nonfinite, residual growth는 즉시 fallback한다.

### C3 — safeguarded tail estimate

Contractive evidence가 있을 때만

\[
E_{\rm tail}\approx
\frac{\|\Delta y_1\|}{1-\rho_{\rm out}}
\]

형태를 시험하고 exact sequential oracle과 비교한다. 이 식은 증명 없이 bound라고 부르지
않으며, calibration/holdout에서 보수성을 확인한다.

### C4 — Kantorovich/radii-polynomial lane

Approximate inverse `B`, residual `F`, derivative drift를 이용해

\[
Y=\|BF(x)\|,
\qquad
Z_0=\|I-BF'(x)\|,
\]

및 local derivative-variation bound를 stage-triangular structure로 계산하는 방법을 설계한다.
Radii-polynomial 또는 Krawczyk-style sufficient condition이 성립하는 경우에만 rigorous lane으로
분류한다.

### C5 — output-aware recursive triangular bound

각 stage의 local Lipschitz/Jacobian-drift bound를 causal order로 전파하고 최종
`b^T` output direction에 투영한다. Full state norm보다 output-specific bound가 얼마나 덜
보수적인지 비교한다.

### C6 — source generation Rayon parallelization

57 case x 144 candidate generation을 case 단위 local Rayon jobs로 옮긴다. Case 내부 timestep과
stage order는 순차로 유지하고, 결과는 canonical key로 정렬한다.

### C7 — compute/serialization split

- compute-only checksum mode;
- compact summary mode;
- full raw zstd evidence mode

를 분리해 thread scaling을 다시 측정한다.

### C8 — promotion gate

다음을 모두 만족할 때만 adaptive path schedule branch로 넘어간다.

- calibration false accept 0;
- holdout false accept 0;
- nonnormal/stiff corpus false accept 0;
- manufactured-vector all-fast fifth order;
- Prothero–Robinson and mass-nonnormal regression pass;
- at least one low-depth configuration;
- certificate overhead 포함 기대비용이 sequential보다 작을 가능성;
- one-thread/four-thread scientific identity.

## 그 뒤의 분기

Certificate gate를 통과한 뒤에만 adaptive theta/lambda/q, nonstationary sweeps, Anderson,
GCRO-DR tuning을 공동 비교한다.
