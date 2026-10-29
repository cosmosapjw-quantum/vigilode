# R3 실제 런타임 검증 결과

고정 소스 `cc2cd041737e7ff543624d1b59893a3b4397369f` / tree `182fa306067b84ee10ec1f1b1e5fbad9e769aae1`. **전체 workspace PASS로 판정하지 않는다.** 이전 실행 결과를 상속하지 않았으며 이번 turn의 toolchain compile, native 실행, 실패 로그를 각각 보존했다.

- Native harness **136개**, `--list`로 발견한 이름 있는 test **682개**.
- 실제 test 결과: **625 PASS**, **0 assertion FAIL**, **26 timeout 미확정**, **28 전체 시간 상한으로 미실행**, **3 ignored 미실행**. 기타 상태가 있으면 아래 JSON 원장을 따른다: `{"IGNORED_NOT_RUN": 3, "NOT_RUN_TOTAL_BUDGET": 28, "PASS": 625, "UNRESOLVED_BOUNDED_TIMEOUT": 26}`.
- Harness 상태: `{"BOUNDED_TIMEOUT": 6, "NOT_RUN_TOTAL_BUDGET": 4, "PASS": 126}`. 전체 native 시간 상한 300초, 실제 300.024초. 개별 harness 30초 또는 알려진 장시간군 60초까지; 전체 잔여 시간으로 줄어들 수 있다.
- **변경 표면 7개 harness의 37개 계약 모두 실제 PASS**. 이 숫자는 전체 PASS 수의 부분집합이며 중복 합산하지 않는다.

| 변경 target | PASS test 수 | 관측 상태 |
|---|---:|---|
| `paired_timing_contracts` | 13 | PASS |
| `r2_output_budget_contracts` | 3 | PASS |
| `r2_output_time_identity_contracts` | 9 | PASS |
| `r2_phi_range_contracts` | 4 | PASS |
| `r2_timing_authority_contracts` | 5 | PASS |
| `r2_work_coverage_consumer_contracts` | 1 | PASS |
| `work_unit_contracts` | 2 | PASS |

Rust/Cargo 1.94.1을 사용자 첨부 archive에서 설치하고 hello를 실제 compile/execute했다. Registry 의존성 118개 모두 source lock과 exact version/package checksum이 맞고, network fetch 없이 locked/offline로 실행했다. Python 3.12.14, NumPy 2.3.5, SciPy 1.17.0, Matplotlib 3.10.8 import는 실제 PASS; SymPy/mpmath는 없었다. Native 설정은 opt-level 0/debug info, dev/test codegen-units 1, incremental off, test threads 1이다. 성능 또는 wall speedup 측정이 아니다.

## 전체 build와 native 실행의 구분

최초 full workspace/all-targets/all-features no-run 작업은 600초 상한 안에서 197.453초 후 wrapper exit 0을 보고했지만, live redirect된 Cargo JSON이 compiler/build-script records 중간에서 끝나 native executable이나 `build-finished`를 담지 못했다. 따라서 build completion 증거로 인정하지 않았다. 기존 audit runner의 빈 목록 `all([])`이 일시적으로 낸 PASS는 **무효**로 명시하고 원본 receipt를 보존했다. 수정된 runner는 native artifact 비어 있음과 `build-finished` 부재를 거부한다. 과거 R2의 131개 실제 binary 실행 기록은 빈 목록이 아니어서 이 helper 결함의 적용 대상이 아니다.

Metadata만 회수하려던 cached no-run 재호출에서도 실제 recompilation이 관측되었고 30초 상한에서 exit 124로 끝났다. 더 이상의 Cargo 실행은 하지 않았다. 이미 존재하는 current-turn test binaries는 source의 Cargo metadata, 유일한 `test-*` fingerprint, native `--list` 성공을 연결해 별도로 선정했다. 실제 실행 stdout/stderr를 memory에서 완전히 수신한 다음 저장했다. 이 개별 native 관측이 전체 build evidence의 빈 부분을 소급해서 PASS로 만들지는 않는다.

앞선 설치 중 SO 추출 잘림, shared cache zero-byte object, `fatal library error, lookup self`를 환경/도구 사건으로 보존했다. 지정된 generated cache만 정리하고 incremental을 끈 뒤 standalone probes는 실행 성공했다. all-features build에는 codegen-units 1을 사용했다. 원인 미확정인 I/O/cache 이상을 solver 결함으로 분류하지 않았다. OOM kill counter는 0이다.

Timeout은 수치 또는 assertion 실패로 해석하지 않는다. ignored와 doctest는 실행하지 않았고 PASS 합계에 넣지 않는다. 독립 연구 probe의 결과는 각 lane에서 별도로 보고하며 위 workspace native 합계에 중복 가산하지 않는다.

기계 판독 SSOT는 `evidence/runtime/FINAL_RUNTIME_SUMMARY.json`; 전체 test별 상태와 명령/exit는 `evidence/runtime/recovered_native/summary.json`과 대응 `.list.log`/`.run.log`에 있다.
