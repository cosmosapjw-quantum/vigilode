# 2026-11-15 독립 production 심사

기준 커밋은 `bee5563c51264a7da19c519295bdda2fb317caef`다. 이 문서는 기준 커밋의 사실을 고정한다. 뒤의 코드 수정에 대해서는 새 테스트와 수정 receipt를 연결해야 한다. 최신 연구 branch의 코드를 읽었으며, 오래된 `main`을 현행 연구 코드로 취급하지 않았다.

**판정: 사용자가 요청한 통합 production solver 범위는 HOLD.** 기존 `vigilode` facade는 이미 의미 있는 구현과 계약 테스트를 갖췄다. 이번에 23개 production 계약과 37개 PPSW1 계약을 실제 재실행하여 모두 통과했다. 동시에 기존 테스트가 다루지 않던 PPSW1 상태/트랜잭션 오류를 별도의 실행으로 재현했다. 따라서 60개 green test를 전체 production 인증으로 승격할 수 없다.

## 직접 재현한 오류

| ID | 대상과 관측 | 영향 | 최소 교정 |
|---|---|---|---|
| AUD-01 | `ppsw1::Session::accept`는 generation만 비교한다. A와 B가 generation 0이면 A가 B의 attempt를 수락했다. A의 상태 `1`이 `1.9999999999999716`으로 바뀌었고 A의 RHS/linear-solve 카운터는 모두 0이었다. | 서로 다른 세션/문제의 상태와 비용 회계가 섞인다. | 세션 소유권 token과 generation을 함께 확인하고 실패 시 상태·카운터를 변경하지 않는다. foreign accept/reject 회귀 검증. |
| AUD-02 | `Session::new`가 NaN 시간을 허용했고 `try_step`이 이를 accepted로 반환했다. `t=1e16`, `h=0.01`에서 accepted step의 시간이 그대로였다. `start`도 같은 시간에서 RHS 11회를 실행하고 시간 진행 없이 성공했다. | 유효하지 않은 수치 결과 또는 상위 driver의 무진행 루프 가능성. | 전체 State의 차원·유한성, history step, stage time 및 endpoint의 유한한 전진을 callback 전에 확인한다. |

재현 소스는 `evidence/audit_ppsw_boundary_repro.rs`, 명령은 `evidence/audit_ppsw_boundary_command.sh`, 실제 출력은 `evidence/audit_ppsw_boundary_repro.log`다. 이 probe의 exit 0은 **기준 버전의 결함을 재현했다**는 뜻이다. 수정 버전의 수용 테스트로 실행하면 안 된다. 수정용 회귀 테스트는 위 입력을 거부하고 무변경을 확인해야 한다.

별도로 legacy `integrate_fixed`가 NaN 시간을 받는다는 가설을 공격했다. 실제 실행에서 `FixedGrid::new`가 이를 거부했으므로 그 가설을 철회했다. 실패한 probe도 `audit_legacy_boundary_*`에 보존했다. 뒤에 배치했던 empty-state 가설은 이 실행에서 도달하지 않았으므로 판정하지 않았다. 과거 `main`의 피상적 읽기에서 생긴 의심을 최신 코드의 결함으로 이식하지 않는다.

## 현재 병렬화와 capability

PPSW1의 6개 stage backsolve는 common-W factorization을 공유하며 bounded `ParallelExecution::map_ordered`를 사용한다. 그런데 `advance_capped`의 후속 6개 stage RHS 평가는 직렬 `for` loop다. 이전 step의 stage 자료와 현재 독립 stage 해가 이미 정해져 있으므로, 해당 RHS 행을 병렬 평가하는 것은 수치법 변경 없이 가능한 작업이다. callback의 순수성/스레드 안전성, 결정적 결과 순서, 오류가 난 행의 실제 수행 비용을 계약에 넣어야 한다. 작은 RHS에서는 scheduling 비용으로 더 느려질 수 있어 성능 개선은 별도 계측 대상이다.

현재 PPSW1 경로는 dense Jacobian을 만들고 dense W를 LU 분해한다. JVP-only 입력도 기저 벡터별 JVP로 dense Jacobian을 재구성한다. starter는 `5n × 5n` Newton matrix를 만들고 Newton 반복을 수행한다. 따라서 현재 경로의 정확한 특성은 **6개 stage의 독립적인 선형 solve; step마다 Newton 없음; startup에는 Newton 있음; derivative-based dense 경로**다. matrix-free/derivative-free/Newton-free 전체 알고리즘이라고 기록할 수 없다.

`vigilode`의 `Problem`/`Options`/`Rodas5p`, `GeneralOde`, `ProductionSolver`는 이미 존재한다. `ProductionSolver`는 RHS-only 명시적 경로와 supplied-JVP 선택적 전환을 제공하고, 전역 attempt budget·부분해·작업 카운터를 보존한다. 이번 23개 테스트는 이 범위의 계약을 확인한다. 다만 smooth forward identity-mass endpoint IVP가 현재 계약이며, Peer-W와 geometry-specific kernel의 전체 portfolio 통합은 아직 없다. 기존 타입과 refusal semantics를 확장하는 것이 적절하다. 새 독립 frontend로 같은 기능을 복제할 이유는 없다.

기존 facade의 terminal event는 accepted trajectory를 계산한 뒤 후처리하는 계약이다. 사건 이후의 작업도 과금된다. 실제 in-loop 종료나 discontinuity restart를 일반 production capability로 광고하려면 별도 driver 변경과 검증이 필요하다.

## 과학·수치 심사

기존 공식 G0와 `T01_GLOBAL_ERROR_AUTHORITY` 기록을 재사용했다. 이번 계약 테스트만으로 strong-stiff global order, matched achieved-error wall speed, 허용 step-count ratio, fallback fraction 또는 2D/3D/구면 전체 범위의 우월성을 증명하지 않았다. T01이 보존한 global-error-budget 실패와 reference authority 제한도 그대로 유지된다.

다음 수학·수치 의무는 T02의 실제 qualification이다: 가변 step ratio, nonnormal stiff system, Prothero–Robinson/선형 기준, startup 오차와 고정 Newton stopping floor, estimator 반올림, 거부/rollback, 마지막 step clipping과 `t_end < 2h0`, oracle의 신뢰구간을 포함한 공통 출력 격자 비교. 그 결과에 기반하여 common-W operator/Krylov 경로와 starter를 각각 분리 검증해야 한다. `rtol`이 같다는 사실을 같은 달성 오차로 대체하지 않는다.

## 실행과 근거 범위

| 실행 | 결과 | 근거 |
|---|---|---|
| `cargo test -p vigilode --test production_rhs_contracts --test production_setup_contracts --test production_selection_contracts --test production_session_contracts --locked` | 23 passed / 0 failed / 0 ignored | `evidence/audit_production_contracts_remote.log` |
| `cargo test -p rodas5p-integrators --test ppsw1_6b_contracts --locked` | 37 passed / 0 failed / 0 ignored | `evidence/audit_ppsw_contracts_remote.log` |
| 독립 PPSW1 public API probe | AUD-01/AUD-02 결함 재현 | `evidence/audit_ppsw_boundary_repro.log` |
| legacy 시간검증 가설 공격 | NaN 거부 확인; 가설 철회 | `evidence/audit_legacy_boundary_repro.log` |

실행 환경은 clean `bee5563c...` remote worktree와 repository-pinned `rustc 1.94.1`, debug profile이었다. 시간/속도 qualification을 수행한 release benchmark가 아니다. cloud PATH에는 Rust가 없어 원래 baseline worktree의 기존 tests를 실행했고, adversarial probe는 `/tmp`에서 기존 rlib에 링크했다. 원격 tracked source는 수정하지 않았다. 소스와 출력 SHA-256, 실행 상태, 미실행 항목은 `production_audit.json`에 있다.

전체 workspace 테스트, blind holdout, 배포 패키지, 대규모 성능/메모리 scaling, 모든 과거 branch의 코드 심사는 이번 독립 audit 범위가 아니다. 이 기록은 신규성 판정이나 일반 불가능성 정리를 주장하지 않는다. 이번 실질적 진전은 누락된 계약 오류의 실행 재현, 이미 구현된 기능의 재검증, 그리고 수치법을 바꾸지 않는 다음 병렬화 지점의 식별이다.
