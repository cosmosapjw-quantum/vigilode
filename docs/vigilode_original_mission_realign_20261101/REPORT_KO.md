# VigilODE 원래 고유 목표와 적분기 재고조사, DAG 재설계, 적대적 감사
2026-11-01 KST. 이 문서에서는 VigilODE 독립 범용 stiff/oscillatory ODE 연구만 분석한다. 별도 적용 프로젝트 브랜치와 그 결과는 제외한다.

## 1. 정본과 원래 연구의 정체성
2026-08-11 원본 G0 범위 고정 문서: research/generic_vectorized_jf_v1/00_SCOPE_LOCK_KO.md, pre-import Git history의 원래 commit. 당시 질문은 원래 8-stage RODAS5P의 accepted-step economy와 strict same-error 조건을 보존하면서 causally permitted stage batching/vectorization, low-depth Newton-free homotopy correction, Krylov/exponential Rosenbrock과 complete speculative-work accounting으로 보호된 R-JF보다 빠른 stiff·oscillatory·nonnormal general ODE solver를 만드는 것이다.

초기 정량 성공조건: primary fast lane explicit Jacobian builds=0, Newton iterations=0, median critical batch depth<8 / P95≤6, observed global order≥4.8, same global-error median wall-time speedup≥1.15, accepted steps≤1.2× protected R-JF, fallback fraction≤10%. 전부 최초 목표이며 현재 달성 판정이 아니다.

최근의 더 강한 선택 연구에서는 RHS-only/derivative-free가 가능할 때 이를 첫 후보로 두고, JVP/AD는 권한이 부여된 대안으로 둔다. Jacobian-free가 derivative-free라는 뜻은 아니다. Protected R-JF는 baseline 및 fail-closed fallback, E-K fast lane은 독립 증명·동일 정확도 실험 뒤에만 선택된다.

## 2. 복수 DAG가 만든 오류
- 2026-10-18 A01–A12 original DAG: node별 implementation이 12/12 scoped completed로 기록되었으나 이후 audit가 full arithmetic, global trajectory, valid comparator, production을 HOLD했다. 12/12=production PASS가 아니다.
- 2026-10-28 LOCAL_DEVELOPMENT_PLAN.json: 정확히 23개 노드. complete_scoped 7, pending_local 15, historical authority pending 1. 익숙한 7/23 수치의 직접적 원천이다.
- 2026-08-26의 main HEAD 이후 여러 적분기 연구가 독립 브랜치에 남았다. 본 조사 시점 원격 전체 open PR 144개 중 적용 프로젝트 키워드로 명시적으로 제외한 28개를 뺀 VigilODE generic 중심 open PR은 116개이며, merged PR 24개에는 비과학적 연결 시험도 포함된다(파일명·PR 제목 필터 기반이므로 경계는 수동 검사 필요). 원형 G0, A01–12, 23-node 계획의 분모를 서로 합칠 수 없다.
- 현재 정확한 연구 산출물(수치 실험·증명·코드)과 배포 승격(전체 CI·외부 comparator·API·장기 테스트)은 별개 차원이다.

## 3. 구현 적분기 목록과 용도
INTEGRATOR_INVENTORY.json의 28개 항목은 실제 source 파일과 공개 function을 식별한 방법/driver/backend/selector 목록이다. 서로 다른 완성된 production integrator가 28개라는 주장은 하지 않는다.

| 기술 계열 | 적합 영역 | 한계 및 승격 분류 |
|---|---|---|
| Protected sequential matrix-free RODAS5P | 강성/진동/비정규 일반 ODE의 JVP-허가된 기준 | 보호된 실제 trajectory, 8-stage serial work 기준 |
| Dense, banded, dense-output RODAS5P | 작은 stiff, 검증된 band, requested dense events | 이미 보호된 route이나 explicit Jacobian을 사용하여 J-free fast-lane innovation이 아님 |
| Matrix-free fast GMRES / GCRO-DR, lagged LU | 행렬 비조립 JVP, 느린 Jacobian 변화 | epoch와 원래 true residual, accepted step 및 total work 필요 |
| Small dense / small batch drivers | 작은 독립 stiff IVP의 전용 최적화 | 배치 throughput은 인과적 stage span 단축과 다름 |
| Full adaptive fused pexprb54s4 | smooth stiff/oscillatory에서 phi-Krylov가 경제적인 경우 | 원래 핵심 E-K 연구 후보. 과거 frozen 수치 order 4.92877, median gain 1.519×, coverage 11/12는 광범위 production 증거가 아님 |
| exprb2, exprb43 | 단계 검증 및 phi action oracle | step-only. 독립 full-IVP integrator로 집계 금지 |
| Transactional q1/q2, JVP/homotopy stages | 낮은 critical depth의 원래 raw-U 근사와 검증된 fallback | full trajectory, original-stage residual, exact accepted-step cost와 함께 비교 필요 |
| RHS-only nonlinear root/homotopy | 도함수 금지인 특정 smooth source-bound stage | source owner/epoch 공격과 일반 8-stage fifth-order가 미해결 |
| DOPRI5, structured Lawson-DOPRI5 | nonstiff, 선언된 damped-rotation exact flow | stiffness stiffness-test 없는 DOPRI5 일반 보장은 없음, Lawson 구조 선언 필요 |
| BDF1/2 and Radau IIA1/3 | exact/physical correctness의 내부 reference와 negative control | reference-only: CVODE/Radau5 경쟁 baseline으로 사용 금지 |
| SABR5P | original-stage predictor/reference | one-step reference로서 full adaptive solver와 구별 |
| Verified continuous residual, nonlinear window homotopy, packet trajectory, inverse-Picard BE | model-owner source-specific outward enclosure | 일반 Rosenbrock 5차 대체는 아니며 proof/physics source domain이 제한됨 |
| Opt-in switching and automatic selector | 실제 계의 regime 변화가 확인된 경우 | reentrant negative control과 full overhead; RHS-only 기본 경로 유지 |
| Leja/Laguerre/Chebyshev/Taylor phi actions, stage batching | 적분기 내부의 matrix-action/critical-depth backend | 단독 complete integrator가 아님 |

## 4. 적대적 drift watchdog
| Finding | 관측 증거 | 피해 | 이번 조치 |
|---|---|---|---|
| D1_STATUS_COLLAPSE | A01–12 scoped completion, 7/23 delivery plan, production HOLD가 공존 | 구현/수학/승격의 결과가 한 숫자에 합쳐짐 | maturity를 derived, runnable, physical-IVP, matched-error, C9, production으로 분리 |
| D2_DEPENDENCY_INVERSION | L-RELEASE→L-HARDWARE→PR-N01/05/06 및 L-RELEASE→PR-N02/03/04 | 기본 실험이 lint/배포/비교기 준비에 막힘 | 연구 착수 edge 7개만 별도로 변경하고 release edge는 원형 유지 |
| D3_BRANCH_ACCUMULATION | 144 open, main 8월 이후 정체 | 새 후보는 있지만 integration authority가 없음 | 하나의 고정 baseline, 실험 결과 중심의 소수 후보 비교, 명시적 다음 전환 |
| D4_CERTIFICATION_OVERHANG | WP proofs, symbolic panels, file authority 업무 반복 | 중요하지만 original same-error / stage depth KPI 대체 | 이미 증명된 자료 재사용, 추가 toy bound 미세조정 중단 |
| D5_UNFAIR_ECONOMICS | 구조적 fast path가 작은 8-mode에서 dense보다 느림, reentrant switch 손실 | callback work 감소로 wall speed를 오독 | true error, complete work, runtime, retry/refusal/fallback/report 비용을 분리 |
| D6_GLOBAL_HISTORY_BARRIER | exact pre-import bundle 복원 뒤에도 post-import commit CI 실패 | 현재 연구까지 정지 | 엄격한 authority release gate 보존, 격리 실험은 scoped 허용 |
| D7_WRONG_STAGE_TARGET | 보조 BE root는 원래 8-stage RODAS U root가 아님, source-owner epoch 공격 | 잘못된 오차 검증으로 false performance | original raw-U source/clock/epoch/basis binding 필수 |
| D8_REVIEW_STARVATION | 기존 완료 물리/수학의 중복 감사와 PR/CI 반복 | 새로운 science witness와 full-IVP work를 늦춤 | 하나의 실험, 하나의 adversarial falsification, 다음 과학 단계로 전환 |

상반된 실제 결과: 2026-10-27 structural n2048 charged enclosure는 특정 문제에서 중앙값 2.063×이었지만 strong fused direct 비교는 0.840×, reentrant selective 경로의 총 work는 RHS-only 대비 42.81% 증가한 사례가 있다. 2026-10-18 A07의 작은 8-mode 테스트에서 dense/structured 시간비 0.392–0.437도 있었다. 하나의 성공 사례만 골라 일반 speedup으로 승격하면 안 된다. WP3 native RODAS5P 16/16 source-matched 결과도 same-runner original BE/RODAS5P speed comparison 완료를 뜻하지 않는다.

## 5. 23-node DAG 재편과 과학 우선 전환
REFOCUSED_DAG.json은 원래 23개의 ID·title·historical_status·historical_release_dependencies를 그대로 보존한다. 비정본 실행 overlay의 research_start_dependencies만 7개 노드에 대해 바꾸었다. 연구 시작 허가와 original completion은 서로 다른 논리이며, strict production prerequisite은 한 개도 삭제하지 않는다.

| ID | 원래 prerequisites | 새 scoped research-start 조건 |
|---|---|---|
| L-HARDWARE | L-RELEASE | P-PUSH, 이미 검증된 source와 동일 입력에서 연구용 benchmark만 |
| PR-N01 | L-HARDWARE | R-STRUCT, R-HOMO, polynomial/phi 후보 구성 |
| PR-N02 | L-RELEASE | R-HOMO 수학/오차 계획 |
| PR-N03 | L-RELEASE | R-STRUCT full-state residual |
| PR-N04 | L-RELEASE | R-HOMO, R-ADAPT owned session |
| PR-N05 | L-HARDWARE, PR-N04 | R-STRUCT, R-ADAPT, 독립 causal-span 시험 |
| PR-N06 | L-HARDWARE, PR-N01 | R-ADAPT regime/reentry 비용 분석 |

나머지 16개의 research 및 release dependencies는 원본 그대로다. 각 노드의 정확한 우선순위, 산출물, 허용 범위는 기계 검증된 DAG JSON을 사용한다.

## 6. 네 가지 실제 과학 성과 마일스톤
M1 (바로 실행 가능): 기존 original protected R-JF, adaptive fused pexprb54s4, RHS-only DOPRI5의 same-runner achieved-global-error work precision. 미리 frozen generic IVP 4종(비선형 stiff, oscillatory, nonnormal, reentrant)과 사용 원본 source, IC/BC, 시계, output grid, 오류 norm을 기록. Accepted, rejected, calibration, JVP/RHS, allocation과 실제 elapsed time을 전부 보고한다. Scaled error 도달 전의 런타임 비교는 ranking 금지.

M2 (바로 실행 가능): q1/q2 causal-prefix, E-K, sequential R-JF의 원래 8-stage raw-U 잔차 및 실제 critical path와 연산량. 원본 계수, 원래 실패/작업 ledger, epsilon 및 source epoch identity를 유지한다. 개선이 없으면 후보 불인정하고 fallback 보존.

M3 (M1/M2 뒤): 부동 조건이 바뀌는 구간에서 robust owned session 및 opt-in switch를 비교한다. 부정 사례의 residence loss와 full rollback accounting을 이길 증거가 없으면 RHS-only 기본/보호된 R-JF 유지. 모델별로 수렴한 positive/negative empirical results와 참조 수치를 그대로 보존.

M4 (strict production gate): 원래 Q-GIVENS, Q-FAIR-AB, Q-LINT, Q-NEGATIVE, Q-LEDGER, L-RELEASE, R-QUALIFY 및 독립 C9, real measured scope가 만족되는 곳에 한정해 승격. 자동 merge/default 전환 없음.

**M1과 M2가 없는 M3/M4 증설은 다시 drift다.** 반대로 M1/M2의 설계·정량 계약을 이 문서만으로 결과 PASS라고 주장하지 않는다.

## 7. 정량 계약과 종료
- source authority: protected original code와 frozen reference 및 release profile, original accepted-step accounting.
- science admission: observed global order≥4.8, same achieved global error, complete original requested output, no false acceptance, 0 explicit J/Newton for the declared derivative-free fast path, critical depth reduction and total costs.
- performance admission: median same-error wall ratio≥1.15 plus per-problem failure ranges, candidate accepted steps≤1.2× R-JF, fallback≤10%; confidence/uncertainty and hardware identity.
- formal claim: if source/model dependent, classify scoped. No blanket theorem and no suppressed failures.
- stop: source owner/epoch mismatch, invalid original raw-U residual, false-PASS, implicit budget increase or renamed repeat audit.
- C9 reviewer absent means no independent-author admission. Original production HOLD. Existing official progress 7/23, successor admitted 0/12 unchanged. No merge, default change, tolerance relaxation or automatic next.

## 8. 검증과 재현
Run: python3 tools/validate_vigilode_core_realign.py --check.
Test asserts original SHA-256, 23 exact IDs/status, seven historical scoped completions, 15 pending-local, one pending authority, old and scoped research graph acyclicity, 28 inventory source files and symbols, reference-only comparator label, and production HOLD.
Original plan/source/ledger not modified. Actual new solver/full-IVP benchmark NOT_RUN in this design-only audit. No applied-project code is imported.
