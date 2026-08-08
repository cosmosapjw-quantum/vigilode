# SCIENTIFIC_CONTRACT.md

## Scientific objective

비교 대상 ODE integrator가 solver 내부 tolerance나 timestep 수가 아니라, 동일한 외부 reference와 common output grid에서 얻은 global error 대비 실제 비용으로 비교되도록 한다.

## Governing definitions

Reference trajectory:

\[
y_{\rm ref}(t_j),\qquad t_j\in\mathcal T.
\]

Candidate error:

\[
e_j=y_{\rm cand}(t_j)-y_{\rm ref}(t_j).
\]

Primary P0 error:

\[
E_{\max,L^2}=\max_j\|e_j\|_2.
\]

External WRMS:

\[
E_{\rm WRMS}(t_j)=
\sqrt{\frac1N\sum_i
\left(\frac{e_{j,i}}{a_i+r|y_{{\rm ref},j,i}|}\right)^2}.
\]

Pareto dominance는 동일 problem/error metric에서 error와 한 cost component를 동시에 비교한다. 임의의 weighted scalar work score는 사용하지 않는다.

## Conventions

- unit system: dimensionless manufactured problems;
- norm: Euclidean L2 and external reference-weighted WRMS;
- output-grid convention: exact common time match, no interpolation in P0;
- timing: T1 authoritative latency, T4 case-level throughput only;
- random seed: `20260808`;
- thread policy: local Rayon pool, external BLAS/LAPACK 없음.

## Valid regime

- regular ODEs with nonsingular mass matrices;
- fixed-step anchors;
- binary64;
- analytic exact reference;
- small dense P0 authority-validation corpus.

Excluded: singular-mass/index>0 DAE ranking, adaptive trajectory interpolation, sparse/MPI/GPU ranking, peak-RSS claims.

## Required invariants

- exact reference hash independent of solver;
- every fixed-step ladder hits the common output grid;
- missing output time is failure, not interpolation;
- T1/T4 scientific checksum equality;
- failed/nonfinite records remain in output and never enter successful Pareto fronts;
- timing fields do not enter scientific checksum;
- vector cost counters are preserved exactly.

## Known limits

| Limit | Expected result | Tolerance | Reference/test |
|---|---|---:|---|
| exact candidate = reference | all error metrics 0 | exact binary64 | `global_error_contracts` |
| known 2-vector perturbation | L2=1, WRMS=1/sqrt(2) | 1e-14 | `global_error_contracts` |
| missing common time | explicit error | exact | `global_error_contracts` |
| T1 vs T4 | scientific checksum identical | exact | contract + canonical artifact |
| BDF1/Radau IIA1 | first-order trend | prior native gates | canonical analysis |
| BDF2 | second-order trend | prior native gates | canonical analysis |
| Radau IIA3/RODAS5P | fifth-order before roundoff | prior native gates | canonical analysis |

## Reference cases

- scalar linear;
- Prothero–Robinson;
- manufactured nonnormal vector;
- manufactured nonlinear noncommuting-mass system.

## Numerical requirements

- release binary;
- one-thread repeated randomized timing;
- minimum 2 ms calibrated timing sample;
- median and IQR retained;
- one-/four-thread scientific identity;
- all failures explicit;
- no solver algorithm modification in this cycle.

## Failure semantics

NaN/Inf, nonlinear/linear non-convergence, missing grid output, timing failure, empty result, silently interpolated state, or unavailable reference는 성공으로 처리하지 않는다.

## Change control

Primary error metric, reference source, output-grid rule, timing authority, cost axes, or failure semantics 변경은 새 design/research gate를 요구한다.
