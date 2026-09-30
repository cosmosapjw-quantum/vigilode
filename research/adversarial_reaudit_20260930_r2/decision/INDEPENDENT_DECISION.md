# VigilODE 재감사 독립 판정

검토자: `/root/r2_decision`. 검토 source: commit `7708ef90554fc3986478d4602de6a01c7266b14f`, tree `23ffcacb8e4afcd72734e162c0e83536d565ba6c`. 후보 설계·구현에 참여하지 않은 별도 검토자로, 최신 변경과 원본 소비 경로, 재현 client, 원 로그 및 연구 수식을 읽었다. 생산 소스와 원격 저장소를 변경하지 않았다. GPT-6 Astra v4.0.0 연구·코딩 계약을 적용했으며, 그 명칭을 실제 모델 성능의 검증으로 해석하지 않았다.

**판정은 `ACCEPT_BOUNDED_AUDIT_AND_RESEARCH_RESULTS`, 생산 준비도는 `HOLD / MAJOR_REVISION_REQUIRED`다.** 이 두 판정은 별개다. 현재 감사의 제한된 발견·수학 유도·실행 결과는 보고서와 개발 입력으로 사용할 수 있다. 원 저장소의 일반 정확도, 총오차 인증, 새 병렬 경로의 속도 향상을 승인하지 않는다.

## 독립적으로 확인한 범위

`ff84ed6…`에서 현재 source까지의 변경과 다음 실제 파일을 확인했다: `output.rs`, `integrate.rs`, `homotopy_policy.rs`, `audit2_reusable_transaction_research.rs`, `exponential.rs`, `matrix_functions.rs`, `work.rs`, `paired_timing.rs`, `global_error.rs`, CLI `main.rs`. 현재 HEAD/tree를 직접 조회했고 당시 tracked/untracked 변경은 없었다. 검사하는 동안 원본 source를 고쳐 반례를 만들어내지 않았다.

이미 source에 링크되어 빌드된 세 client를 검토자 실행으로 다시 돌렸다. phi 104행, output/policy 9행, statistics 5행의 원 출력을 보존했고 세 프로세스는 각각 실제 exit 0이었다. 레코드 수는 test 수가 아니며, 반례 client의 exit 0은 solver correctness PASS도 아니다. `EXECUTION_RECEIPT.json`은 실행한 binary hash와 소요시간, 원 출력 경로를 뒷받침한다. 새 컴파일을 독립적으로 반복했다고 주장하지 않는다.

추가로 `independent_math_checks.py`를 작성·실행했다. 후보 구현을 바꾼 것이 아니라 별도 oracle 검산이다. 여섯 판별 항목이 모두 참이며 실제 프로세스 exit 0이다.

- Mixed policy의 유효 입력을 정확한 Fraction으로 변환한 budget은 `0.999988867182683…`이고 output 5를 허용하지 않는다.
- M08의 정확한 residual은 `2^-104`이다. 해당 `PW`는 가역이라는 사실을 유지했다.
- 작업자 scalar fixture와 다른 8-stage **2-component** rational `W,J`에서 quadratic componentwise majorant를 독립 구현했다. zero/perturbed 두 후보의 모든 성분별 stage 부등식은 정확 유리수 비교로 참이다.
- 동일 24차 Laguerre `hρ=100` 실험을 재실행했다. cap64/degree111의 exp와 φ1 실패, cap16/degree209의 다섯 출력 통과가 유지됐다.
- owner의 4×4 similarity toy를 별도 행렬 계산으로 확인했다. 물리 오차 `1/2-exp(-1)`은 불변이고 auxiliary residual norm은 `α^-2`로 달라졌다.

## 원 finding 폐쇄와 남은 결함의 판정

이전 PHI-P1/P2/P3의 수치 반례는 현재 native 결과에서 닫혔다. 시간 정규화 similarity와 physical tolerance scale 수정은 타당하다. 기존 output/dense-accounting/invalid-policy/NaN primitive/overflow-certificate 회귀의 성공도 인정한다. 통계 경로에서는 명시적 global session ID 처리, vector-unit Pareto 소비 및 CLI median-only 승격 폐쇄를 인정한다. 원 fixture의 폐쇄를 모든 입력 영역의 완결로 확대할 수는 없다.

다음 **7개 실행 기반 finding**을 보고서에 유지하는 데 동의한다.

| ID | 독립 검토 결과 | 주장 범위 |
|---|---|---|
| R2-OUT-01 | CONFIRMED, P1 | 표현 가능한 8-ULP 구간에서 fixed solver가 0회 전진하고 success를 반환. 별도 dense 경로는 미래 ULP request를 이전 endpoint로 소비. 실제 공개 API 결과. |
| R2-POL-01 | CONFIRMED, P2 | 유효 finite fields의 중간 overflow/NaN으로 Mixed budget이 과대평가됨. 공개 policy acceptance 재현; 전체 ODE false fast-accept는 미실행. |
| PHI-R1 | CONFIRMED, P2 | 최종 `h^k b_k`가 표현 가능해도 먼저 계산한 `h^k`가 underflow/overflow. Arnoldi 이전 입력 변환 문제이며 이전 구현이 같은 극단 입력에서 성공했다는 회귀 비교는 없음. |
| PHI-R2 | CONFIRMED, P2 | dense combination oracle의 큰 공통 진폭에서 finite 결과가 `Ok(0)`. 같은 입력의 direct fused 경로는 통과하므로 전체 fused solver 실패로 일반화하지 않음. |
| R2-STAT-01 | CONFIRMED, P2 | 실제 하나의 process에서 public producer가 만든 빈 session labels가 6개의 독립 session으로 소비됨. disjoint A/A도 연결 검사 없이 허용. 합성 clock이며 성능 측정 결과가 아님. |
| R2-STAT-02 | CONFIRMED, P2 | legacy 16-call Unknown에 modern 1-vector ledger를 더하면 실제 Pareto consumer가 cost=1로 취급. 실제 운영 campaign이 이 혼합을 사용했다는 주장은 없음. |
| R2-STAT-03 | CONFIRMED, P2 | B=1 public protocol이 authoritative interval/Promote를 생성. default B=10000 결과와 독립 empirical-bootstrap oracle는 Inconclusive. |

M08 scalar underbound는 위 7개에 추가하는 새로운 singular false-certificate가 아니다. 소스가 비-directed 계산임을 이미 설명한다. 이 계산을 정확한 inverse norm bound로 소비할 권한이 없다는 **LIMITATION_CONFIRMED**로 보존한다. 구조적 synthetic certificate를 실제 nonlinear production certificate로 해석하지 않는다. subnormal output probe의 명시적 실패 또한 허위 성공으로 세지 않는다.

## 수학·코딩 연구 결과의 제한된 승인

| 결과 | 승인되는 범위 | 승인되지 않는 범위 |
|---|---|---|
| 순서 보존 weighting·공통 진폭 balancing | 동치식, bounded native fixture에서 정확도 회복 | 임의 mixed dynamic range/cancellation의 안정성, 총오차 상계 |
| fail-closed Mixed budget 후보 | 개별 항의 오류 전파와 zero budget 보존, 세 fixture의 잘못된 수락 방지 | 모든 유한 표현 가능 budget을 성공적으로 계산하는 full-range evaluator |
| Laguerre 공동 exp–φ | 대칭 negative-semidefinite A와 검증된 spectral interval에서 직접 유도한 exact-arithmetic truncation bound, 동일 v의 다섯 출력 재사용, bounded 실행 | 임의 nonnormal operator, 서로 다른 v_k의 fused 비용, roundoff/계수 적분 총상계, Chebyshev 대비 일반 우위 |
| quadratic stage majorant | h>0, strict-lower stage 의존성, 검증된 비음수 `U≥|W^-1|`에서의 유한 귀납과 endpoint 부등식 | 현재 binary64 actual-tableau 값을 directed certificate로 승격, 원 ODE 오차 또는 stiff-uniform 5차 증명 |
| componentwise 구조 보존 | scalar norm의 과도한 보수성을 줄이는 지정 fixture의 수치 결과 | 대규모 matrix-free 문제에서 U를 싸게 구성할 수 있다는 보장 |
| telemetry covariance | projection까지 함께 변환한 defect integral의 similarity 불변성, Euclidean residual norm drift toy | 실제 ζ34 policy의 false acceptance, 기존 threshold의 신규 representation에 대한 유효성 |
| 통계 admission·typed Unknown 후보 | balanced same-session 최소 계약과 Unknown의 흡수적 합산 | 임의 불균형 설계의 통계적 정당화, finite-sample 95% coverage 보장 |

Homotopy q1의 8→6 깊이는 ideal common-W stage depth 모형이다. q2+diagnostic은 8이고 certificate 구성·work 증가·fallback·RHS/JVP 병렬화 비용은 별도다. `p1>4pf`는 그 단순 비용 모형의 필요조건이며 실측 speedup이 아니다. 시간창 병렬화는 다른 solver 실험으로 분리하는 것이 타당하다.

통계 coverage 실험은 6^6 empirical bootstrap 분포를 정확 열거하지만 모집단 coverage는 20,000회 Monte Carlo다. 두 종류의 정확성을 구분한 현재 설명은 적절하다. 6개 session의 순서통계 구간으로 target을 바꾸는 방안도 **다른 estimand**임을 명시해야 한다.

## 개발 및 종료 결정

우선순위는 timestamp membership/완료 계약, 안전한 budget 평가, φ weighting·oracle 안정성이다. 다음으로 session/ledger/policy authority를 닫고, 수학 연구 후보를 실제 Rust 소비자와 연결한다. 성능 campaign은 이러한 correctness·measurement 계약과 전체 시도 비용이 고정된 뒤 실시한다. failure를 단순 상태명 변경으로 폐쇄하거나, source identity를 정확도 증거로 대체하지 않아야 한다.

이 판단은 감사 보고서와 구체적 후속 개발 계약을 전달하는 것을 승인하는 과학적 범위 판정이다. push 권한은 사용자 요청에서 따로 주어졌으며, 이 검토자가 remote push나 복원 가능성 검증을 수행한 것은 아니다. 전체 runtime campaign의 최종 수·timeout·ignored 상태는 root의 실행 영수증을 따르고, 이 세 probe 또는 lane의 overlapping tests를 합산하지 않는다. 독립 리뷰의 재귀적 추가는 요청하지 않는다.

## 메인 본문 검토

`REPORT_KO.md` 초안을 별도로 읽어 7개 finding의 수치·분류, 원 fixture 폐쇄, polynomial/homotopy/telemetry의 전제와 생산 승격 제한을 확인했다. 과학적 blocking correction은 없다. Mixed zero-budget fixture는 ε_ref=0이라는 표현으로 통일하도록 요청했다. 전체 runtime summary는 후속 최종 readback에서 아래와 같이 확인했다. 패키지 전체 파일 존재·게시 검증은 owner의 별도 전달 gate이며 새 광범위 감사를 요구하지 않는다.

## 최종 runtime readback 및 종료

갱신된 §5, `NATIVE_RESULT.json`, authoritative `suite_summary.json`을 대조했다. summary SHA-256 일치, build exit 0, 131 harness의 125 완료 PASS/6 timeout, 고유 시험 658개의 632 PASS/23 timeout 미확정/3 ignored가 서로 일치했다. 시험 레코드를 검토자 코드로 재집계했고 125개 완료 harness의 libtest-summary 일치 기록도 확인했다. assertion failure 0을 전체 통과로 바꾸지 않은 `NOT_COMPLETE_BOUNDED_TIMEOUT` 판정이 적절하다. ignored WU21/V37 두 snapshot과 doctest 미실행도 본문에 드러나 있다. 이 검토는 실행 영수증의 제한 readback이며 full suite 재실행이 아니다. `RUNTIME_FINAL_READBACK.json`에 대조 결과를 보존했다. ε_ref=0 문구 수정도 반영됐다. 메인 보고서의 독립 검토를 종료하며, 기존 생산 HOLD는 유지한다.
