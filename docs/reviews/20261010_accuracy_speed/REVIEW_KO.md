# VigilODE 적대적 재감사 및 정확도·속도 개발 보고서

작성일: 2026-10-10. 대상 소스: `cfed140bf127f5aa8fcf4fe4c16689d7de062b87`.
게시 브랜치: `audit/rvj-accuracy-speed-reaudit-20261010`.

## 판단

현재 가장 효과적인 방향은 **이미 입증된 불필요한 작업 제거를 별도로 채택하고, 결합 stage target에는 실제 잔차가 해의 오차로 증폭되는 정도와 모든 승인 경로의 오염예산을 붙이는 것**이다. 새 다항식이나 homotopy를 추가하는 것만으로 이 정확도 문제를 해결할 수 없다.

이번에 새로 실행한 반례에서는 predictive controller의 underflow와 coupled fallback 판정식의 overflow 문제가 확인됐다. 결과 검사기도 잘린 벡터·NaN·변경된 twin·중복 행을 포함한 네 복사본을 PASS로 판정했다. 기존 published RUNS가 잘못됐다는 증거는 없으며, 이 결과는 입력 검증을 강화해야 한다는 뜻이다.

코드 이식을 위한 작은 Rust 결과물도 추가했다. 현재 represented 2×2 행렬과 metric을 소유하는 `GainWitness2`는 outward residual correction으로 weighted error를 감싼다. 독립 Fraction 계산에서 **30/30 enclosure, 12/12 음성 대조군**이 통과했다. 이 중 4개는 사전에 고정한 별도 holdout이다. 이 결과가 허용하는 것은 **국소 선형계 연구 모듈의 다음 이식 단계**이며, 전체 ODE 정확도·고차 수렴·성능·기본값 승격은 HOLD다. bound의 유용성도 고르지 않다.

| 판정 축 | 결론 |
|---|---|
| 최신 구현의 일반 정확도 승격 | HOLD: nonnormal stage amplification, 모든 acceptance의 오염예산, 긴 궤적 민감도 미해결 |
| ALG04 / ALG05 / ALG06 기록 | FAIL / FAIL / 등록 규칙상 PASS를 그대로 보존. ALG06 robust accuracy는 HOLD |
| 2×2 연구 prototype | exact represented input에 한정한 검증 통과. 독립 판정은 `evidence/independent_decision.json` 참조 |
| 새 speedup | 측정하지 않음. instructions, JVP, allocation의 기존 결과를 구별해 재사용 |
| production 변경 | 없음. 새 example·연구 모듈·검사기·보고서 추가 |
| 기존 과학 campaign 재실행 | 0회 |

## 1. 어떤 업데이트와 연구를 통합했는가

최신 implementation은 Oct8 이름의 브랜치에 있지만 마지막 변경일은 Oct10이다. 아래 genealogy를 기준으로 삼았다. main이나 날짜가 오래된 audit 브랜치를 최신으로 취급하지 않았다.

| 역할 | branch / source | 사용 방식 |
|---|---|---|
| 최신 native ALG04–06 | `audit/rvj-algorithmic-directions-20261008`, `cfed140…` | 실제 감사 대상 |
| SPD01–09 | `audit/rvj-speed-research-20261005`, `a49f7e4…` | 최신 source의 ancestor. native 비용·동일성 근거 재사용 |
| Oct4 이후 PP/SAFE 이식 | `audit/rvj-research-integration-20261004`, `4de1f88…` | 이전 감사의 후속 해결 상태 확인 |
| 별도 보존 연구 | `research/rvj-full-history-20261004`, `6852b4a…` | archive provenance, 기존 once-audited claims 재사용 |
| 새 offline pilot의 수록본 | `docs/reviews/20261008_algorithmic_directions/pilot/` | 수학 가설·Python replica·문헌·비판 보고서를 한 차례 대조 |

별도 연구 결과의 핵심 내용은 이미 최신 branch의 pilot에 들어 있다. 따라서 과거 research branch를 blind merge하거나 원자료가 없는 Python 숫자를 native 결과로 승격하지 않았다. pilot의 일부 JSONL/NumPy reference는 수록되지 않았고 원래 scratch 경로도 남아 있다. 이는 설계 근거로는 유용하지만 standalone 재현 bundle이라는 뜻은 아니다.

전체 source SHA, harness SHA-256, runtime, 36개 ledger row의 재사용/정정 관계는 [SOURCE_BINDING.json](../../../research/reaudit_accuracy_speed_20261010/SOURCE_BINDING.json)과 [REUSED_RESULTS.json](../../../research/reaudit_accuracy_speed_20261010/REUSED_RESULTS.json)에 있다. GPT-6 Astra v4.0.0 수학·코딩 harness의 방법론을 적용했다. 실제 실행 모델 식별은 UNKNOWN이며 모델별 성능 개선을 측정했다고 주장하지 않는다.

### 오래된 미해결 항목을 다시 결함으로 쓰지 않은 이유

| Oct4 당시 우려 | 현재 상태 |
|---|---|
| GCRO-DR refresh의 fast caller 연결 | SAFE-RECYCLE/L-0066에서 opt-in 연결·계수화. default 채택은 별도 |
| directed composition | SAFE-ENCLOSURE의 실제 local 반례 및 후속 exp floor 수정 L-0064/65. 후속 수정을 재사용 |
| complex-shift certificate | PP16/L-0070에서 H=I native 구현·검증. 일반 H까지 확장됐다는 뜻은 아님 |
| Fourier/native predictor 미구현 | PP05/06 native 이식 존재. 등록 gate FAIL은 보존하며 false certificate로 오해하지 않음 |
| shared-shift/Fourier 결합 | PP07/L-0074가 실제 shifted solve 부재로 ABSTAIN. 새 client를 꾸며내지 않음 |
| non-Krylov 대안 부재 | PP08–11에 Chebyshev/Laguerre router, Leja EstimateOnly, fused Taylor total이 존재 |

## 2. 새 적대적 발견

심각도는 기본 solver 전체의 실패를 의미하지 않는다. 아래 구현 경로는 opt-in이며, 각 재현의 경계를 명시했다.

### F101 — P1: predictive factor의 극소 오차 underflow

위치: `crates/rodas5p-integrators/src/adaptive.rs:403–413`의 `predictive_factor`.

이전 accepted step이 h=1, error=0.5이고 현재 h=1e-100, error=1e-200, estimator order=5이면, 정확한 predictive raw factor는

$$0.9\,10^{-100}\left(\frac{0.5}{(10^{-200})^2}\right)^{1/5}
=7.834955069665117\ldots\times10^{-21}.$$

설정의 최소 배율 0.2로 clamp해야 한다. 실제 public controller update는 **5.0**을 반환했다. `error*error`가 0으로 underflow한 뒤 quotient가 infinity가 되어 clamp 방향이 바뀐다. 정상 크기 대조군은 약 0.2로 일치했다. Wolfram의 exact 산술도 최소 배율 1/5를 확인했다.

수정 방향은 `err_acc=max(err_prev,PREDICTIVE_ERROR_FLOOR)`를 보존하고 `log(safety)+log(h)-log(h_acc)+(log(err_acc)-2*log(error))/p`를 먼저 clamp하고 exp하는 것이다. `error=0`의 명시 정책, rejection cap, min/max-step 검증을 보존해야 한다. 이 반례는 public controller 입력 경계의 결함이며 일반 benchmark에서 발생 빈도를 측정한 것은 아니다.

### F102 — P1: physical norm overflow에 따른 fallback 승인

위치: `rodas5p_matrix_free_fast.rs:810–811,843–849`.

유한한 RHS 성분도 L2 norm은 binary64 범위를 벗어날 수 있다. 현재 코드의 production threshold와 unscaled residual norm이 모두 infinity가 되면 `residual <= threshold`가 true다. 대각 scaling 뒤 staged solver의 입력은 작고 유한할 수 있어 내부 finite 검사가 이 우회를 막지 못한다.

등록된 cyclic 4×4, restart=budget=1의 공개 staged-solver callback 경계에서 이를 확인했다. 물리 RHS 성분은 최대 1.5e308로 모두 유한했고 후보도 유한했다. 안정적으로 계산한 상대잔차는 **0.8660254037844385**, 기준 rtol은 **1e-10**인데 copied driver predicate가 승인하여 `FallbackAccepted`가 됐다. scale=1e100 대조군은 거절됐다.

**이것은 private driver 함수나 전체 ODE 궤적을 직접 실행한 결과가 아니다.** public solver callback에 source와 같은 판정식을 넣은 실행과 해당 source trace의 결합이다. 가장 작은 다음 수정은 finite RHS norm/threshold/residual 검증 또는 overflow-safe homogeneous 비교다. 직접 driver 경계 regression을 추가한 뒤 opt-in mode를 다시 심사한다. 보호 경로로 돌아간다는 사실만으로 그 경로의 정확도를 인증했다고 부르지 않는다.

### F103 — P1: floor/stall은 target을 초과해 승인되지만 G3에서 빠진다

위치: `gmres_staged.rs:437–453,550–568`, `rodas5p_matrix_free_fast.rs:871–880`.

11개 작은 staged 설정 중 triangular2 stall/floor, triangular4 floor에서 requested residual target을 초과한 승인이 나왔다. 실제 residual은 각각 약 2.78e-17, 7.85e-17, 6.68e-16이다. 이 상태는 enum에 올바르게 표시되지만, driver G3가 debit하는 것은 `FallbackAccepted`뿐이다.

roundoff floor가 필요한 상황은 있다. 문제는 이를 “원래 target을 달성했다”거나 “모든 선형 오염을 이미 예산에 포함했다”고 해석하는 데 있다. requested target, raised target, 실제 잔차, gain authority, acceptance reason을 모두 기록하고 각 stage를 동일한 오염 계약으로 판단해야 한다. 기존 G3가 임계 결정을 바꾸지 않았다는 published 결과를 그대로 유지하며 같은 G3 실험을 반복하지 않는다.

### F104 — P1: 새로운 결과가 malformed input으로도 PASS가 될 수 있다

위치: `tools/alg04_coupled_target_v2_check.py`, `alg05_controller_v2_check.py`, `alg06_guard_v2_check.py`.

새 검사기는 기존 자료를 수정하지 않고 임시 복사본만 사용했다. helper 15개 관측과 ALG06 전체 checker 4개 복사본 관측을 수행했다.

| 복사본 변경 | 실제 전체 checker 결과 | 의미 |
|---|---|---|
| 320차원 candidate를 reference 첫 성분만 남긴 길이 1 vector로 변경 | PASS | zip metric이 누락된 성분을 검사하지 않음 |
| reference와 같은 candidate의 두 번째 성분을 NaN으로 변경 | PASS | Python max의 비교 순서가 NaN을 놓침 |
| Robertson twin의 첫 성분 변경 | PASS, twins_reproduced=false | identity 진단이 gate에 포함되지 않음 |
| 동일 row를 한 번 더 삽입 | PASS, canonical cell 수 유지 | raw uniqueness 미검사; dictionary가 중복을 흡수 |

ALG05 helper는 길이 mismatch를 assert로 막지만 NaN-second는 놓친다. assert는 `python -O`에서도 실행되는 authority 검사로 사용할 수 없다. INVALID와 유효 실험의 FAIL을 구분하는 versioned 공통 validator가 필요하다.

원본 checkers·BASE·RUNS·RESULTS 해시는 시험 전후 일치했다. 정적 inventory의 1,571개 published state vector는 모두 유한했다. 따라서 이 발견으로 기존 ALG04–06의 수치를 무효라고 선언하지 않는다.

### F105 — P2: local gain, scalar tau, nonlinear certificate의 증거 단계가 다르다

현재 `rodas5p_stage_transfer_constants()`는 imaginary axis의 1501점 표본 최대를 계산한다. 이는 전체 영역의 certified supremum이 아니다. projected Hessenberg의 최소 singular value 역시 full-space inverse norm의 상계가 아니다. 예를 들어 `[[1,K],[0,1]]`를 e1에서 보면 projected operator는 [1]이지만 unseen residual e2의 correction은 (-K,1)이다.

pilot exponential의 defect 적분에도 Simpson quadrature remainder enclosure가 없다. dense actual error와 맞는다는 관측은 native total certificate와 다르다. 각각 유용한 heuristic·설계 자료로 남기고 타입과 문서에서 authority를 나눠야 한다.

### 정책·증거상의 추가 항목

- informative clipping에서 rejection 이후 actual trial 0.75가 승인되면 next h=1로 requested step을 복구하고 pending flag를 지운다. ALG05의 restoration 정책과 양립하므로 새 solver 버그로 판정하지 않았다. “factor≤1”과 “실제 trial 이후 h 증가 금지” 중 원하는 계약을 먼저 정해야 한다.
- L-0097의 24회 h0 민감도 extrema·median·15/24 수치는 published reviewer narrative에 있다. 그 원시 행은 repo에 없다. 이를 `PUBLISHED_REVIEW_NARRATIVE_ONLY`로 표시했고 복원 목적의 재실행은 하지 않았다.
- ALG05의 두 edge-fix는 endpoint-only 276 cells에서 작동하지 않았다. 파일에서 test 이름을 찾는 gate는 실행 receipt와 같은 증거가 아니다.

세부 finding ID·confidence·source·증거·수정 DAG 연결은 [FINDINGS.json](../../../research/reaudit_accuracy_speed_20261010/FINDINGS.json)에 있다.

## 3. 수학·코딩 연구루프의 이번 결과

새 루프는 광범위한 기존 campaign 반복 대신, 현재 coupling의 가장 작은 빠진 부분인 “실제 residual에서 local error로 가는 검증 가능한 연결”을 다뤘다.

1. 공개 ALG/PP/SPD 결과와 source를 읽고 새 gap을 특정했다.
2. 전체 행렬의 amplification을 부분공간 진단으로 대체할 수 없음을 구분했다.
3. 사전등록을 `60645e213f8a5cc38c9c96063ed5ebcba89246f3`에 게시했다.
4. source checkpoint `a6c6ebfc8342dc7f8726ff0d6f76e031c1d55bca` 뒤 작은 native 연구와 checker mutation을 실행했다.
5. 첫 실패를 보존하고 fixture의 전제를 수정했다. 최종 native source는 `95d589e862c35c675fac3bee7ebdb962415a97de`다.
6. exact Fraction oracle와 별도 decision reviewer로 결과 범위를 판정했다.

### 구현물과 보장

`research/reaudit_accuracy_speed_20261010/gain_witness.rs`의 `GainWitness2::new(W,s)`는 W와 양의 s를 소유한다. determinant interval이 0을 포함하거나 어떤 계산이 유한 범위를 벗어나면 거절한다. `certify(b,x)`는 residual을 다시 outward 계산하고 inverse interval로 correction을 감싼다.

$$c=W^{-1}(b-Wx),\qquad E_\infty\ge\max_i|c_i|/s_i
\ge\|x^*-x\|_{\mathrm{WRMS}(s)}.$$

여기서 W는 입력 binary64 행렬의 exact-real 해석이다. `I-h gamma J`를 만드는 이전 오차, JVP 오차, off-block coupling, nonlinear stage chain과 global propagation은 포함하지 않는다. witness/certificate field는 private이며 Deserialize authority가 없다. 연구 module의 interval 구현을 production에 그대로 복사하기보다 기존 directed primitives와 동일성을 확인하여 core seam으로 옮기는 것이 다음 작업이다.

독립 oracle는 Rust recurrence를 모사하지 않는다. JSON float를 정확한 이진 유리수로 바꾸고 2×2 inverse를 Fraction으로 계산하여 signed correction interval, inverse gain, WRMS squared bound를 비교한다.

| 새 실행 | 결과 |
|---|---|
| discovery 24 + 일반/near-singular control 2 | 26/26 exact error enclosure |
| 고정 disjoint holdout | 4/4 enclosure |
| invalid/singular/overflow controls | 최종 12/12 reject |
| 첫 실행 | 30/30 enclosure, 11/12 expected rejects → FAIL 보존 |
| point estimate가 exact solution인 cell | 3개. bound/error는 null로 기록 |
| 유한 nonzero-error cell의 bound/error | 약 1.414부터 1.393e15까지 |
| holdout bound/error | 약 1.414, 1.414, 1.569, 7240.8 |

따라서 이 prototype는 soundness와 API 계약을 좁게 확인하는 결과다. ill-conditioned 또는 metric이 매우 다른 경우 interval dependency 때문에 느슨할 수 있고, 빠른 solver 선택기로서의 유용성은 측정되지 않았다.

첫 실패는 W=I와 subnormal scale이 실제로는 허용되는 입력인데 overflow negative로 지정한 fixture 오류였다. `S^-1 I S=I`이므로 rejection이 필수일 이유가 없다. off-diagonal 1을 넣으면 실제 scaled inverse gain이 binary64 범위를 초과한다. 이 의도한 대조군으로만 바꿨으며 positive/holdout·gate·certificate algorithm은 그대로다. [FIRST_FAILURE.md](../../../research/reaudit_accuracy_speed_20261010/FIRST_FAILURE.md)와 최초 raw/RESULTS를 남겼다.

## 4. 정확도와 속도의 개발 순서

상세 수식, API sketch, polynomial과 homotopy 설계는 [MATHEMATICS_PORTING_KO.md](MATHEMATICS_PORTING_KO.md)에 있다. 모든 다음 단계는 [22-node JSON DAG](../../../research/reaudit_accuracy_speed_20261010/NEXT_DEVELOPMENT_DAG.json)에 prerequisite·파일·command template·accept/kill gate·단위를 포함한다. command는 앞으로 구현할 테스트의 계획이며 이미 존재하거나 통과했다고 표시하지 않는다.

### 첫 개발 묶음

| 우선 | node | 구체 변경 | 완료 조건 |
|---|---|---|---|
| 즉시 | AS01 | fallback finite/overflow-safe 판정 | 작은 seam 반례와 직접 driver 경계가 거절, finite 대조군 유지 |
| 다음 | AS02 | predictive log-factor 계산 | 극소 입력 factor=0.2, zero/rejection/grid 계약 유지 |
| 병행 | AS03 | 공통 evidence schema | 네 malformed copy가 INVALID, 정상 입력·reference identity 확인 |
| 별도 작은 채택 | SP01 | DupFix만 promotion 검토 | acceptance residual은 남기고 중복 진단 JVP만 제거; output/decision parity |
| 구조 경로 | SP03 | declared band/small-dense routing API | 실제 band 선언 검증, assembly부터 포함한 strongest-rival 비교 |

이 묶음에는 다른 controller·target·guard 승격을 섞지 않는다. 기존 수학 campaign 대신 새 caller/실패 경계를 집중 검사한다.

### 정확도 연구의 의존 순서

`AS04 all-acceptance telemetry → AS05 current-operator gain → AS06 whole-stage contamination → AS07 fresh driver holdout` 순서다. AS04/05는 일부 병행할 수 있지만 AS06은 metric·rhs·operator authority가 정해진 뒤에만 의미가 있다.

frozen metric의 tube-Lipschitz bound를 L_i라 하면 보수적인 stage recurrence는

$$q_i\le G_i\left[\rho_i+\xi_i+
\sum_{j<i}(|h\gamma|L_i|a_{ij}|+|\gamma C_{ij}|)q_j\right].$$

rho는 실제 residual, xi는 assembly/evaluation uncertainty다. 모든 연산을 outward 수행하고 tube self-consistency를 확인한다. output·embedded vector·다음 step metric으로 옮기는 비용과 오차도 포함해야 한다. 이 충분조건이 너무 느슨하면 속도 연구를 중단하거나 더 타이트한 구조별 분석으로 바꾼다. scalar tau를 썼다는 이유로 certified라는 이름을 붙이지 않는다.

단순히 G3처럼 local estimator에 작은 charge를 더하면 `h/T`에 비례한 전체 contamination budget을 관리한 것이 아니다. committed attempt의 누적 예산, rejected attempt의 비용, global amplification은 서로 분리한다.

### 이미 관측된 비용 개선의 활용

published native 결과에서 DupFix는 small-n JVP 0.63–0.89×, coupled target의 고유 효과는 Brusselator에서 ProjL2 대비 기하평균 약 0.738×다. 둘을 합쳐 하나의 수학적 개선이라고 쓰지 않는다.

SPD03의 declared-band 경로는 n=400에서 instructions/attempt 약 0.255×다. SPD04의 least-squares workspace는 solve당 11 allocation을 없애며, SPD05의 LGMRES-into에는 아직 실제 driver caller가 없다. 이식의 선후 관계를 바로잡는 것이 또 다른 basis 후보보다 빠른 실용적 이익일 수 있다. n≤8에서는 dense direct가 강한 경쟁자이고, general sparse LU는 기존 banded API와 다른 개발이다.

SPD07의 previous-step warm start와 SPD08 batch lanes의 실패는 그대로 유지한다. zero-start의 별도 이점은 prospective policy 실험으로 검토할 수 있지만, 다른 recycle/preconditioner mode까지 자동으로 적용하지 않는다.

## 5. Polynomial 및 homotopy 확장연구

### Polynomial

**Chebyshev**는 verified symmetric nonpositive domain의 현재 certified 기준이다. PP08은 79개 중 Chebyshev 76, Laguerre 0, fallback 3이며 두 후보를 모두 실행했다. 싸게 계획을 세운 뒤 하나의 backend만 실행하는 router가 실용적 다음 후보지만 비교 대상에는 Chebyshev 단독을 포함해야 한다.

**Laguerre**는 구현 자체가 없는 것이 아니다. PP09는 component export가 없어 등록한 분해에 실패했다. finite-degree envelope의 4.196× 개선만으로 현재 total/budget gap을 해결하지 못한다. supplementary derived reconstruction은 recurrence-adjoint·summation remainder를 의심하게 하지만 실제 component telemetry와 같지는 않다. 첫 작업 PY01은 항별 export, 다음 PY02는 해당 adjoint recurrence의 interval/Taylor/Bernstein 상계다. 근거 없이 exp(L/2)를 계속 주범이라고 설명하지 않는다.

**Leja**는 현재 EstimateOnly이며 그 상태를 유지한다. 72개 fixture가 잘 맞았다는 것과 total forward certificate는 다르다. degree/scaling 선택의 backward error를 forward error·roundoff로 연결한 뒤 expensive-JVP의 좁은 영역에서 비교한다. 기본 문헌은 [Caliari 등, 2016](https://arxiv.org/html/1506.08665v2)이다.

**fused Taylor**는 PP11의 극단 RHS scale 실패에 구체적인 확장 경로가 있다. 공통 power-of-two normalization으로 action을 계산한 뒤 candidate·bound를 outward rescale한다. normalization 자체의 subnormal rounding, final rescaling, underflow 아래 relative criterion의 불가능성을 포함한다. 기존 72개 campaign 대신 새로운 scale boundary가 검증 대상이다.

### Homotopy parallelism

현재 native partial path의 최종 lambda=1 target을 보존한다. frozen 이전 round에서 각 stage RHS를 계산하고 common-W 여러 RHS를 푸는 작업은 병렬화할 수 있다. 반면 lambda continuation, recurrence level, 다음 round는 선행 결과에 의존한다. worker 수로 sequential depth를 지웠다고 주장하지 않는다.

HT01의 재개 조건은 **실제로 비싼 RHS/JVP를 가진 client**와 strongest sequential comparator다. 작업량뿐 아니라 setup + round별 최대 worker 작업 + synchronization + 원래 endpoint certificate + 실패/fallback의 critical path를 측정해야 한다. 기존 Fourier client에는 shifted solve가 없으므로 PP07 ABSTAIN을 유지한다. 별도 many-shift toy를 넣어 그 client의 이득이라고 부르지 않는다.

waveform relaxation·multiple shooting 등 time parallel 접근의 문헌 맥락은 [Gander, 2015](https://www.unige.ch/~gander/Preprints/50YearsTimeParallel.pdf)에 정리돼 있다. 이 문헌은 현재 repository의 parallel speedup 증거가 아니다.

### 새로운 integrator와 전처리

PEXPRB own-step-size, ROCK4 switching, 2D linear-part right preconditioner는 pilot에서 검토할 가치가 있는 niche가 있지만 native adoption 결과는 아니다. 특히 PEXPRB defect integral의 native certification에는 quadrature remainder와 floating arithmetic enclosure가 필요하다. [Jawecki–Auzinger–Koch](https://arxiv.org/html/1809.03369v2)의 defect-bound 이론이 floating Simpson 구현을 자동 인증하지 않는다. 새 sparse/right-preconditioner interface는 true residual 단위와 shift-invariance 조건부터 정의한다.

## 6. 검증·재현·게시물 사용

사용한 offline toolchain은 Rust/Cargo 1.94.1이다. 최신 Cargo.lock의 118 registry packages와 필요한 4,382 file checksum을 supplied vendor와 대조했다. dependency/lockfile은 바꾸지 않았다.

새 example 실행, exact Fraction 검사, copied-input checker probes, fmt 및 example clippy `-D warnings`를 수행했다. production source를 바꾸지 않았으므로 기존 전체 과학 suite를 다시 돌리지 않았다. broad test의 과거 PASS는 새 실행처럼 기록하지 않는다. 명령·exit status·원자료 경로는 [VALIDATION.json](../../../research/reaudit_accuracy_speed_20261010/VALIDATION.json)을 참조한다.

재현 명령은 repository root에서 실행한다. offline 환경 경로는 재현자의 toolchain/vendor 위치에 맞게 구성해야 하며 source에 새 절대경로 의존성을 넣지 않았다.

```bash
cargo run --offline --locked --release -p rodas5p-integrators \
  --example reaudit_20261010 -- /tmp/reaudit-native.json
```

정식 oracle는 node의 `NATIVE.json`을 읽는다. published 원자료를 덮어쓰지 않도록 별도 checkout/copy에서 비교한다. checker probe는 출력 overwrite를 거절하며 별도 `--output` 경로를 받을 수 있다. 다음 DAG의 planned tests는 먼저 구현·사전등록해야 한다.

| 파일 | 용도 |
|---|---|
| REVIEW_KO.md | 통합 판단, 새 발견, 개발 순서 |
| MATHEMATICS_PORTING_KO.md | 자세한 수식·authority 경계·Rust API 이식 설계 |
| FINDINGS.json / CLAIMS.json | findings와 증거 수준·claim ceiling |
| NEXT_DEVELOPMENT_DAG.json / ARTIFACT_SCHEMA.json | 22개 개발 node와 기계 판독 구조 |
| NATIVE.json / RESULTS.json / CHECKER_PROBES.json | 새 실행 원자료·독립 numeric 검사·malformed artifact 관측 |
| NATIVE_FIRST.json / RESULTS_FIRST.json / FIRST_FAILURE.md | 최초 실패와 수정 이유 |
| REUSED_RESULTS.json / SOURCE_BINDING.json | 기존 결과 재사용과 source/harness 구속 |
| evidence/independent_decision.json / VALIDATION.json | 독립 판정과 실제 검증 범위 |

연구 실행의 완료와 production 승격은 별개다. 이 보고서가 권하는 다음 행동은 AS01–03의 작은 정확성 수정, SP01의 분리된 채택 심사, 그리고 AS04–06의 엄밀한 오차 연결이다. polynomial·homotopy의 확장은 해당 증거와 실제 client 조건이 갖춰지는 순서로 진행한다.
