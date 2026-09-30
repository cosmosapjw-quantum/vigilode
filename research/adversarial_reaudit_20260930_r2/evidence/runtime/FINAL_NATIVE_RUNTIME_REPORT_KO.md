# 현재 소스 native 회귀시험 결과

고정 commit: `7708ef90554fc3986478d4602de6a01c7266b14f`. Rust/Cargo 1.94.1, 사용자 제공 vendor의 offline/locked 환경에서 실제 실행했다. 전체 판정은 **NOT_COMPLETE_BOUNDED_TIMEOUT**이다. 컴파일 성공과 모든 시험 통과를 구분한다.

| 항목 | 결과 |
|---|---:|
| `cargo test --workspace --all-targets --all-features --locked --offline --no-run --jobs 1` | exit 0; 203.21 s |
| 방출·시도한 test harness | 131 |
| 완결된 test harness | 125 PASS |
| 시간 상한에 도달한 test harness | 6 |
| 중복 제거한 개별 시험 PASS | 632 |
| 관측한 assertion FAIL | 0 |
| 시간 상한 때문에 미확정 | 23 |
| ignored, 실행하지 않음 | 3 |
| 원본 Cargo.toml/Cargo.lock 변경 | 없음 |

테스트 실행은 debug profile, harness당 test thread 1, 일반 harness 30초·기존 고비용 연구 harness 120초로 제한했다. 전체 campaign test budget은 750초다. 느린 harness를 마지막에 배치했고 이미 통과한 시험을 후속 broad campaign으로 다시 실행하지 않았다. 문서시험은 이 `--all-targets` campaign에 포함하지 않았다. 다른 lane의 focused 시험과 이 결과를 합산하면 중복 계산이다.

| 제한에 도달한 harness | 관측 PASS | 미확정 | 상한 s |
|---|---:|---:|---:|
| `cli_contracts` | 17 | 8 | 30 |
| `common_w_gate_contracts` | 1 | 1 | 30 |
| `fixed_step_order_contracts` | 5 | 4 | 30 |
| `two_arm_v3_campaign_contracts` | 0 | 2 | 30 |
| `a1_two_arm_receipt_contracts` | 0 | 5 | 120 |
| `inner_forcing_fixed_step_ladder_contracts` | 3 | 3 | 120 |

아래 ignored 시험 세 개는 실행되지 않았다. 특히 WU21/V37 snapshot 두 개의 native replay를 이번 campaign이 검증했다는 주장을 하지 않는다.

- `rodas5p_integrators::g4_s5b0_regime_atlas::stage_trajectory_geometry_tests::one_pair_wall_protocol_is_self_describing_and_identity_checked`
- `v37_continuation_transaction_contracts::v37_exhaustion_is_a_charged_abstention_without_endpoint_or_failure_label`
- `v37_continuation_transaction_contracts::v37_trajectory_literals_match_the_latest_snapshot`

`g4_s5b0_regime_atlas_contracts` 12개, `givens_production_differential_contracts` 1개, `homotopy_policy_contracts` 8개, `unified_candidate_contracts` 13개는 현재 소스에서 완결 PASS했다. 이전 재감사의 일부 timeout이 이번에는 종료되었으나, 다른 시간 제한과 소스 차이 때문에 과거 결과와 단순 수치 비교하지 않는다.

증거 수집 중 `adaptive_global_error_contracts` 원래 저장 stdout이 마지막 결과 이전에서 끊긴 현상이 있었다. 실행 runner와 driver는 이미 exit 0 / 4 PASS summary를 기록했지만 원문 보존 간극이므로 그 harness만 30초 제한으로 재실행했고 4 PASS / exit 0을 다시 기록했다. 이 4개는 두 번 합산하지 않았다. 이 현상은 test assertion 실패가 아니라 receipt persistence anomaly이며 근본 원인은 미확정이다. 원래 로그와 복구 로그를 모두 보존했다.

실행 runner의 최초 result regex가 인접 행을 소비하는 집계 오류도 수정했다. 시험은 다시 실행하지 않고 원래 stdout의 각 행에서 결과를 재집계했으며 완결 harness 125개의 Rust summary count와 전부 일치함을 확인했다. `suite_summary_execution_original.json`은 최초 파생 집계를 보존하는 **비권위 기록**이며 최종 수치의 SSOT는 `full_suite/suite_summary.json`이다.

기계 판독 전체 결과, exact unresolved 이름, invocation argv/cwd/exit, timeout, log SHA-256은 `full_suite/suite_summary.json`을 본다. 실제 원문은 target별 `.run.log/.run.stderr/.list.log`; compile 증거는 `all_test_artifacts.jsonl`과 `all_test_build.stderr`다. 호스트 `memory.events`에서 OOM/OOM kill은 0이었다.
