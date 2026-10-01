# R4 Rust 실행 결과

전체 workspace/all-targets/all-features debug 빌드는 실제 exit 0, 230.67초에 완료했다. Cargo `build-finished: success`와 149개 native harness를 확인했다. 검사 대상 source는 `1c54194123ee6abc6daa512e8574922f510b4e2c`, 사전등록 commit은 `b2914f3e3c03eda60a8547619db6284aac8250a2`다.

실행한 범위의 집계: `{"PASS": 671, "IGNORED_NOT_RUN": 4, "UNRESOLVED_BOUNDED_TIMEOUT": 20, "NOT_RUN_TOTAL_BUDGET": 40}`. 전체 native inventory는 735개 named test다. 전체 suite 판정은 `NOT_COMPLETE_BOUNDED_OR_FAILURE`이며 doctest와 ignored test는 실행하지 않았다.

직접 변경된 15개 harness는 전부 정상 종료했다. 68개 test가 통과했고 1개 ignored test는 실행하지 않았다. 이 결과는 기존 test 계약을 만족한다는 구현 검증이며 별도 적대적 반례나 이론적 주장을 대체하지 않는다.

고정 실행 예산은 build 600초, native 전체 300초, 일반 harness 30초, 역사적 긴 harness 60초다. timeout은 수치 결과 실패나 assertion failure로 재분류하지 않는다. budget 종료로 실행하지 못한 test도 named inventory와 함께 보존했다. 실제 속도·scaling·production 성능은 측정하지 않았다.

## 완료되지 않은 harness

| Target | 상태 |
|---|---|
| `cli_contracts` | BOUNDED_TIMEOUT |
| `common_w_gate_contracts` | BOUNDED_TIMEOUT |
| `fixed_step_order_contracts` | BOUNDED_TIMEOUT |
| `two_arm_v3_campaign_contracts` | BOUNDED_TIMEOUT |
| `a1_two_arm_receipt_contracts` | BOUNDED_TIMEOUT |
| `g4_s5b0_regime_atlas_contracts` | NOT_RUN_TOTAL_BUDGET |
| `givens_production_differential_contracts` | NOT_RUN_TOTAL_BUDGET |
| `homotopy_policy_contracts` | NOT_RUN_TOTAL_BUDGET |
| `inner_forcing_fixed_step_ladder_contracts` | NOT_RUN_TOTAL_BUDGET |
| `unified_candidate_contracts` | NOT_RUN_TOTAL_BUDGET |

## 실행환경과 재현 범위

Rust 1.94.1, `--locked --offline`, registry dependency 118개 exact version/package checksum 일치, workspace metadata 총 123개 package. cgroup RAM 8 GiB/CPU quota 8, build jobs 1, native test threads 1, incremental off, dev/test codegen units 1이다. 모든 Cargo build는 공유 flock으로 직렬화했다. 사전등록 후 실행했고 native 실행은 source 또는 Cargo dependency lock을 바꾸지 않았다.

최초 vendor ownership 복원 실패와 target-dir 상대경로 해석 오류는 `runtime_incidents.json` 및 setup receipt에 분리 기록했다. 원 registered runner는 사전등록 commit bytes와 동일하다. 안정적인 shared target symlink를 통해 native artifact를 공유했으며, 정규화된 별도 helper는 실행하지 않았다.

`FINAL_RUNTIME_SUMMARY.json`은 최종 집계, `TEST_COVERAGE.json`은 모든 test의 상태, `native/suite_summary.json`과 대응 stdout/stderr는 원 실행 증거다.
