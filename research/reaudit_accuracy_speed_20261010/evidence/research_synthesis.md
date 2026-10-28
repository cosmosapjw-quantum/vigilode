# VigilODE 2026-10-10 연구 통합 및 확장 제안

기준 소스: `cfed140bf127f5aa8fcf4fe4c16689d7de062b87`.
이 문서는 공개된 결과를 한 차례 대조하여 재사용한 수학·연구 감사 메모다. 기존 과학 실험을 다시 실행하지 않았다. 실제 실행 모델의 식별값은 UNKNOWN이며 GPT-6용 방법론과 모델 실행 사실을 혼동하지 않는다.

## 1. 이번 변경에서 의미가 달라진 결과

| 증거 | 정확한 결론 | 다음 작업에 미치는 영향 |
|---|---|---|
| ALG04 / L-0094 | 결합 target의 PDE 사례 작업량 개선은 native에서 확인됐지만 vig1b-k20, rtol 1e-8에서는 twin 대비 오차 4.18배 | projected Hessenberg의 nu를 full-space gain 상계로 승격하지 않는다 |
| ALG05 / L-0095, L-0098 | predictive controller의 van der Pol 작업량/거부 감소가 새 seed 집합에서도 반복됨. 단일 실패 outlier는 baseline에서도 거의 동일 | shared outlier를 지운 새 gate로 같은 자료를 재채점하지 말고, prospective baseline-relative calibration과 interior output grid 사용 |
| ALG06 / L-0096, L-0097 | 등록 규칙상 PASS. h0의 1e-9 perturbation 24회 중 15회가 twin 1.5배 초과, median 2.39배라는 후속 감사 이미 존재 | robust accuracy는 HOLD. 같은 24회를 재실행할 필요 없음 |
| PP08 / 연구 node | 총오차 router는 79개 중 Chebyshev 76, Laguerre 0, fallback 3. 두 후보 비용을 모두 지불 | Laguerre 자동 선택의 속도 동기를 제시할 수 없음 |
| PP09 / 연구 node | component export 부재 때문에 G1 FAIL. finite-degree envelope는 검증됐지만 기존 총상계 개선에 불충분 | 이전의 exp(L/2) bottleneck 설명을 그대로 재사용하면 안 됨 |
| PP10 / 연구 node | Leja는 72개 모두 목표 정확도를 충족하나 EstimateOnly. 제품수 감소는 8개, 증가 58개 | 새 basis 추가만으로 certified replacement가 되지 않음 |
| PP11 / 연구 node | fused target certificate는 seeded/control 36개에서 useful. 1e-310 입력은 scale-independent floor, 1e100 입력은 perturbation overflow | RHS homogeneity를 이용한 power-of-two normalization이 구체적인 확장 |
| PP12/PP12b | weighted lognorm witness가 native에 이미 존재. PP12b 기존 stiff6개와 holdout10개 useful; 나머지2개 exact norm은 binary64 아래 | 새 lognorm helper를 중복 작성하지 말고 기존 interval machinery 재사용. underflow 문제는 mixed absolute-relative contract로 명시 |
| PP07 | 실제 Fourier client는 shifted solve가 없고 homotopy/shared-shift 연결은 abstain | 가짜 many-shift workload를 실제 client 이득으로 보고하지 않는다 |

참조 파일: 해당 `research/*/PREREGISTRATION.md`의 append-only 결과, 최신 `ALGORITHMIC_DIRECTIONS.md`, `SPEED_RESEARCH_STATUS.md`. 연구-only pilot은 `docs/reviews/20261008_algorithmic_directions/pilot/`로 최신 구현 브랜치에 수록돼 있다. raw JSONL/NumPy references는 수록되지 않았고 absolute scratch paths가 남아 있어 standalone 재현 자료는 아니다. 기존 base-arm fidelity는 새 candidate의 native fidelity를 증명하지 않는다.

## 2. 새 수학 gap: residual gain과 전체 stage 오염은 별개의 계약

현 coupled solve는 `D W D^-1 z = D b`, `W=I-h gamma J`, `D_ii=1/(atol+rtol |y_i|)`를 사용한다. 정확한 scaled residual을 `r=D(b-W u)`라 하면 필요한 것은

`||D(u-u*)||_2 <= G ||r||_2, G >= ||(D W D^-1)^-1||_2`

이다. projected Hessenberg의 최소 singular value는 탐색 부분공간 밖의 방향을 보지 못한다. 예를 들어 `B=[[1,K],[0,1]]`와 첫 Arnoldi vector `e1`는 항상 `H=[1]`을 주지만, `B^-1 e2=(-K,1)`이다. 이것은 현재 ALG04의 특정 궤적을 재현했다는 주장이 아니라, projected gain을 전체공간 인증서로 사용할 수 없다는 정확한 반례다.

재사용 가능한 witness는 다음으로 제한하는 편이 낫다.

- interval-scaled `D J D^-1`에서 verified `mu_up`가 있고 `1-h gamma mu_up>0`이면 `G<=1/(1-h gamma mu_up)`. 현재 `nonnormal_certificate.rs::symmetric_part_upper`를 재사용하되 h gamma 양수 계약과 반올림 방향을 고정한다.
- 명시된 block diagonal 구조 또는 작은 시스템에는 각 block의 verified inverse를 사용할 수 있다. 다만 n<=8에서는 dense LU 자체가 가장 강한 저비용 경쟁자다.
- 일반 matrix에는 approximate inverse X와 verified `q=||I-XB||<1`를 만들어 `G<=||X||/(1-q)`를 얻을 수 있으나, 이를 큰 dense matrix-free 문제의 무료 전처리로 간주하면 안 된다.
- witness는 current J/model epoch, h, gamma, D 및 norm에 결합하고, 입력이 달라지면 폐기한다. certificate 실패는 작은 tolerance로 이름을 바꾸는 동작이 아니라 protected path로 이동하거나 정확한 실패 상태를 반환해야 한다.

이 gain을 붙여도 전체 step 오염이 곧바로 인증되지는 않는다. 현재 `rodas5p_stage_transfer_constants()`는 scalar transfer function의 imaginary-axis 1500개 표본 최댓값을 사용한다. 코드 주석은 supremum이라고 쓰지만 표본은 전구간 상계가 아니며 nonnormal matrix function 또는 nonlinear stage chain으로 바로 운반할 수 없다.

비선형 rhs에 대해 tube 안의 weighted Lipschitz 상계 L_i를 알고 exact U-stage residual의 WRMS 크기를 rho_i라고 하자. 같은 frozen J, 같은 시작 상태의 정확한 stage와 candidate 사이의 차이 q_i는 보수적으로

`q_i <= G [rho_i + sum_{j<i} (|h gamma| L_i |a_ij| + |gamma c_ij|) q_j]`

를 만족한다. 마지막 output은 `sum_i |b_i| q_i`, embedded output은 실제 U-form 계수로 운반한다. 이 식은 계산 가능하지만 stiff 문제에 느슨할 수 있다. pointwise estimate를 certificate로 포장하지 않고, 기존 causal/outward certificate의 tube·roundoff·current-target 계약과 합치는 것이 바람직하다. 목표는 first-order scalar tau를 일괄 폐기하는 것이 아니라 **calibrated target**과 **verified error budget**의 타입·보고 필드를 분리하는 것이다.

관련 이식 지점:
- 새 `rodas5p-core/src/residual_gain.rs`: immutable current-operator witness와 gain admission.
- 기존 `nonnormal_certificate.rs`, `directed.rs`, `outward.rs`: verified matrix enclosure/positive denominator 계산 재사용.
- `rodas5p-integrators/src/rodas5p_matrix_free_fast.rs::staged_stage_solve`: current weighted residual에 witness 적용.
- 같은 파일의 `StageTargetOptions`, `StageSolveStatistics`: heuristic/verified status, rejected witness, gain cost, fallback charge를 독립 기록.
- `outward_certificate.rs` / `transactional_q1_q2.rs`: 전체 chain contamination의 다음 단계.

## 3. 정확도와 속도를 함께 개선할 우선순위

### 3.1 현재 확인된 저비용 변경의 채택

DupFix와 SPD04 workspace 연결은 새로운 수학 가설보다 앞선다. DupFix는 small-n에서 0.63–0.89배 JVP이고 같은 trajectory다. SPD04는 allocation-free least-squares에서 bit identity 증거가 있다. 현재 default 연결 여부를 source로 확인한 뒤 소수의 focused caller tests로 adoption을 심사하면 된다. SPD05는 driver 호출자가 없는 상태이므로 library-level PASS를 solver-level gain으로 보고하지 않는다.

`OdeProblem`은 Jacobian/JVP callbacks를 갖지만 general sparsity declaration이 현재 struct에 없다. `rodas5p_fast.rs`에는 banded entry point가 이미 있다. 따라서 선언 구조 router를 하려면 우선 `ProblemStructure` 메타데이터와 검증된 `lower/upper` 경계 또는 별도 banded constructor를 추가해야 한다. dense storage 문제에 대한 banded kernel의 계수 결과를 general sparse LU 지원이라고 쓰면 안 된다. 작은 dense / declared band / unstructured matrix-free의 routing 이유를 사용자에게 expose하고 가장 싼 적용 가능한 arm과 비교한다.

### 3.2 Polynomial 경로

현재 가장 근거가 강한 목표는 symmetric nonpositive operator의 joint phi action이다. 같은 target `F=sum_{k=0}^4 phi_k(hA) w_k`, 같은 total budget, 같은 exact binary inputs에서 비교한다.

- **Chebyshev:** 현 certified baseline. router가 매번 Laguerre까지 전부 계산하는 비용을 절약하려면 coefficient/degree preflight와 single-backend 실행을 분리한다. 강한 상대는 현재 double-run router가 아니라 Chebyshev 단독이다. cold/warm coefficient setup, enclosure 비용과 failed admission을 모두 센다.
- **Laguerre:** 새 recurrence-adjoint bound가 생기기 전에는 speed family expansion을 중단한다. 첫 작업은 component telemetry export로 PP09의 missing-data gap을 닫는 것이다. 기존 6개 실패 campaign의 rerun은 성능 재검증이 아니라, 꼭 필요한 항별 계측을 위한 별도 preregistered exporter일 때만 허용한다. 그 다음 local Taylor/Bernstein subdivision으로 실제 adjoint-polynomial supremum을 타이트하게 감싸고, coefficient-cache key에 operator enclosure/degree/scale identity를 유지한다. 전체 총상계가 target을 충족하지 않으면 basis 선택을 바꾸지 않는다.
- **Leja:** 현재 binary64 Opitz divided differences와 Newton recurrence는 EstimateOnly. 문헌의 backward error analysis를 쓰려면 scaling/degree selection, backward-to-forward conversion 및 roundoff를 모두 포괄해야 한다. last-two-term stop이 fixture에서 conservative했다는 관측은 tail theorem이 아니다. 새 family는 real-spectrum expensive-JVP에 한정하고 largest h rho에서 얻은 8개 product win의 전체 비용 이득부터 판별한다.
- **Taylor fused certificate:** 임의 RHS 스케일 문제는 `F(A,h,sigma w)=sigma F(A,h,w)`의 동차성을 이용할 수 있다. exact power-of-two sigma를 골라 normalized vectors를 인증한 후 `|sigma| E + output_rescale_rounding`을 반환한다. normalization 중 underflow/overflow가 없음을 증명하거나 그 차이를 enclosure에 넣어야 한다. `taylor_phi_total.rs` wrapper의 자연스러운 확장이다.

다항식의 또 다른 실용적 용도는 exact W의 preconditioner다. SPD/positive-real verified domain이면 Chebyshev polynomial로 inverse preconditioner를 만들고 true unpreconditioned residual로 판정한다. 이것은 Rosenbrock tableau나 frozen J 자체를 polynomial approximation으로 바꾸는 것과 다르다. general nonnormal case의 spectrum-only admission은 거절하고, shared-shift reuse와의 상호작용을 별도 검증한다.

### 3.3 Homotopy 병렬화

q=2 candidate의 low dependency depth는 단독 speed argument가 아니다. R-NEXT-06의 후보는 depth7에 certificate가 추가되고, n16 corrected margin은 0.043 solve units뿐이며 dispatch가 0이었다. 같은 cheap diagonal family를 다시 측정해 다른 verdict를 얻으려 하지 않는다.

재개 조건은 실제 독립적인 비싼 RHS/JVP 또는 batch callback을 사용하는 client가 있을 때다. 새 preregistration에는 `BatchRhsFn`, fixed worker pool, 배치 준비·동기화·fallback·certificate·실패 후보를 모두 포함하고 다음을 판단한다.

`T_par_est = T_setup + sum_round max_worker T_tasks + T_cert + T_fallback + T_sync`

와 strongest sequential arm의 측정값을 비교한다. 비용 추정은 gate이며 wall-time speedup은 아니다. 정확성은 원래 lambda=1 target에 대한 현 certificate로 유지하고 certificate가 실패하면 sequential fallback 비용까지 남긴다. real client가 없으면 shared shifted jet와 Fourier path의 연결은 PP07대로 HOLD다. 이름만 homotopy인 cheap benchmark나 별도 solver target으로의 변경은 목적에 맞지 않는다.

### 3.4 새 방법의 순서

Pilot의 EXPRB-OWN-H, ROCK4 hand-off, 2-D linear-part right preconditioner는 흥미롭지만 native result가 아니다. 선언 구조를 쓸 수 있는 경우 먼저 banded/sparse direct competitor를 구현하고, unstructured nonnormal semilinear niche에서만 exponential own-step-size 방법을 preregister한다. pilot B3의 Simpson defect integral은 quadrature remainder와 arithmetic enclosure가 없으므로 machine certificate가 아니다. native 이식에서 이 부분을 반드시 새 증명/구현 대상으로 다룬다.

## 4. 권장되는 최소 신규 연구

첫 bounded node는 **full-space weighted residual gain witness**다. 공개 ALG04/06 campaign을 재실행하지 않고 다음을 고정한다.

1. 분석적 triangular adversary와 작은 exact binary rational matrices.
2. witness를 받아들이는 positive denominator 사례, ill-conditioned/nonfinite/metric-change 반례.
3. 정확한 residual과 independent exact/high-precision solve를 비교하는 checker.
4. gain 인증 PASS와 solver accuracy/speed adoption의 HOLD를 분리.
5. matrix assembly, gain verification, residual and fallback 비용을 export.
6. 다음 native driver port는 별도 prospective seeds와 interior output grid, pairwise reference uncertainty를 사용.

이 node는 최신 source의 가장 중요한 proof gap을 좁히지만 아직 전체 coupled target을 인증하거나 speed gain을 실증하지 않는다.

## 5. 문헌 확인의 범위

SciSpace를 사용해 관련 문헌을 찾고 primary arXiv 원문을 열어 확인했다. 검색 결과의 repository 날짜를 출판일로 쓰지 않았다.

- Caliari, Kandolf, Ostermann, Rainer, *The Leja method revisited: backward error analysis for the matrix exponential*, SIAM J. Sci. Comput. 2016, DOI 10.1137/15M1027620, arXiv:1506.08665v2. 원문의 §3은 scaling/degree/interval 선택의 backward-error 분석을 다루며, 이는 PP10의 heuristic stop을 자동으로 total forward certificate로 만들어주지 않는다. https://arxiv.org/html/1506.08665v2
- Jawecki, Auzinger, Koch, *Computable upper error bounds for Krylov approximations to matrix exponentials and associated phi-functions*, BIT 2019, DOI 10.1007/s10543-019-00771-6, arXiv:1809.03369v2. defect-based bound의 문헌 계보로 사용한다. 현재 pilot의 floating quadrature 구현에 대한 증명으로 인용하지 않는다. https://arxiv.org/html/1809.03369v2

SciSpace 응답은 `intake_1010/scispace_polynomial_1010.json`에 보관했다. 이 검색은 이미 발표된 결과의 재검증이 아니라 향후 이식의 theorem boundary 확인이다.
