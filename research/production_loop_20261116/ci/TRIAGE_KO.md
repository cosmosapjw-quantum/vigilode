# PR #190 hosted CI 결과 분류 (2026-11-17)

- 대상: loop 브랜치 head `04430258` (draft PR #190, 병합용 아님).
- 비교 기준: `ef04bfab` (프로그램 baseline `research/unified-t02-20261110`, 2026-11-12 실행). 통합 브랜치 `074a5699` 자체는 PR CI 실행 기록이 없다.
- 기계 판독 기록: `PR190_CI.json`. `collect.py`가 `raw/`의 GitHub API 출력과 job 로그에서만 값을 읽어 만든다.
- 로컬 검증 로그: `raw/local_probe_staged_solve_telemetry.log`.

## 1. 요약

| | 성공 | 실패 | skip |
|---|---|---|---|
| head `04430258` | 5 | 8 | 3 |
| baseline `ef04bfab` | 4 | 9 | 3 |

- 실패한 5개 workflow는 2026-11-01 이후 조회한 모든 연구 브랜치 PR 실행(workflow당 최근 12회)에서 한 번도 성공하지 못했다. hosted CI는 이 loop 이전부터 green이 아니었다.
- baseline 대비: 3개 job이 새로 통과(`clippy`, `audit2-feature-and-usage`, `fresh-clone-build`), 2개 job이 새로 실패(`docs`, `semver`). 새 실패 2건은 모두 switcher 계보 채택에서 들어왔다.
- `docs`는 이번 commit에서 고쳤다. 나머지 7건은 원인별로 아래에 적는다.

## 2. job별 분류

| job | head | base | head 실패 원인 | 분류 |
|---|---|---|---|---|
| `docs` | 실패 | 성공 | `crates/vigilode/src/hybrid_automaton.rs:15`의 문서가 private module `crate::switching`에 link (`-D warnings`) | 채택 회귀. **수정함** |
| `semver` | 실패 | 성공 | `cargo semver-checks` major 5건 (아래 3.2) | 채택 회귀. 버전 정책 결정 필요 |
| `bench` | 실패 | 실패 | `vigilode-bench` manifest가 `rust-version = "1.99.0"`, CI toolchain은 1.94.1 | 채택으로 원인이 바뀜. base 원인(`mpi.h` 없음)도 남아 있음 |
| `test` | 실패 | 실패 | `s1_nine_native_cases_match_anchor` (`staged_solve_telemetry.rs:233`, columns 3 대 5) | host 의존 bit anchor. base의 실패 test는 head에서 통과 |
| `a2-a3-gmres-candidate` | 실패 | 실패 | 위와 같은 test | 위와 같음 |
| `a1-inner-tolerance-parity` | 실패 | 실패 | `test_r_parity_on_off_and_alg06_anchor` (`stage_residual_telemetry.rs:498`) | 기존 실패, base와 서명 동일 |
| `ignored-tests` | 실패 | 실패 | `prothero_robinson_wfiltered_enforce_kill_gate` | 기존 실패. 설계상 항상 실패 |
| `research-process` | 실패 | 실패 | `research/qualification/` 2건 + stall rule 5건 | 채택으로 원인이 바뀜 (base의 247건은 해소) |
| `clippy`, `audit2-feature-and-usage`, `fresh-clone-build` | 성공 | 실패 | | 개선 |
| `fmt`, `mode-and-authority` | 성공 | 성공 | | 변화 없음 |

## 3. 원인 상세

### 3.1 docs (수정함)

- 공개 문서가 private module을 intra-doc link로 가리켰다. link 두 개를 고쳤다(`crate::switching` → 일반 텍스트, `crate::switching::SwitchingMethod` → 공개 재수출 `crate::SwitchingMethod`). 동작 변경 없음.
- 로컬에서 docs job의 세 단계를 실행했다: `RUSTDOCFLAGS="-D warnings" cargo doc -p vigilode -p vigilode-coupled --no-deps --locked` exit 0, doctest 3개 통과, `cargo build -p vigilode --examples --locked` exit 0. 예제 실행 단계는 로컬에서 실행하지 않았다.
- 채택 검증(A-gate)에 `cargo doc`이 없어서 놓쳤다.

### 3.2 semver (소유자 결정 필요)

PR base(`074a5699`) 대비 `vigilode` 공개 API가 깨졌다. 전부 switcher 계보의 변경이다.

- `SwitchingConfig`에 pub field 10개 추가, `EntryProbeAdaptation`에 1개 추가 (struct literal을 쓰는 사용자 코드가 깨짐).
- `SwitchingEvidence`에 variant `ExplicitStabilityLimited` 추가, 뒤 variant 9개의 판별값 이동.
- `CostUnit`, `CostEvidenceKind`에 `#[non_exhaustive]` 추가, `CostEvidenceKind` 판별값 2개 변경, `CostUnit`에 non-unit variant 추가.

채택을 유지하면 다음 배포 버전은 0.1.0이 될 수 없다(0.x 규칙으로 0.2.0). 버전 변경은 릴리스 결정이라 이 loop에서 하지 않는다.

### 3.3 bench

- head: `crates/vigilode-bench/Cargo.toml`과 `crates/vigilode-hybrid/Cargo.toml`이 `rust-version = "1.99.0"`을 선언한다. 두 crate는 workspace 밖이라 시험 병합의 `cargo check --workspace`가 잡지 못했다.
- base: build script가 CVODE bridge를 컴파일할 때 `mpi.h`가 없어 실패했다. runner의 `libsundials-dev`가 MPI header를 요구한다.
- green이 되려면 둘 다 고쳐야 한다: manifest를 1.94.1로 맞추고(1.94.1에서 실제로 빌드되는지 확인 필요, V11에서 수행), workflow의 `apt-get`에 MPI 개발 패키지를 추가한다(workflow 수정, 소유자 결정).

### 3.4 test, a2-a3-gmres-candidate

- 실패: `report.columns == expected_columns`에서 3 대 5. GMRES 열 수를 bit 단위 anchor와 비교하는 test다.
- 로컬 재현 시도(Ryzen 5900X): debug profile 통과, release profile 통과, 같은 debug binary를 glibc 2.39(ubuntu:24.04)와 glibc 2.43(ubuntu:26.04) container에서 실행해도 통과. 따라서 build profile과 libm 버전은 원인이 아니다.
- 남은 가설: runner CPU에 따른 연산 순서 차이(`gmres_staged`가 쓰는 faer의 runtime SIMD dispatch). 확인하지 못했다. runner의 CPU 모델은 로그에 없다.
- baseline에서는 `test` job이 `low_rank_bordered_allocation`(할당 2 대 0)에서, `a2-a3` job이 `givens_production_differential_contracts`에서 먼저 실패해 이 test까지 가지 못했다. 두 test는 head에서 통과한다.
- `test` job은 `--no-fail-fast` 없이 실행하므로 첫 실패 binary 뒤의 결과는 알 수 없다. `rodas5p-integrators` 단계는 실행되지 않았다.

### 3.5 a1-inner-tolerance-parity

- bit anchor 3개가 hosted 값과 다르다. hosted 값은 base와 head에서 bit 단위로 같다. 채택이 이 경로의 hosted 수치를 바꾸지 않았다는 근거가 된다.
- anchor는 개발 머신 값이다. 원인은 3.4와 같은 종류로 보이나 확인하지 않았다.

### 3.6 ignored-tests

- 해당 test는 2026-10-21에 기록된 kill gate 실패를 재현하도록 일부러 `#[ignore]`로 남긴 것이다(주석: 임계값을 완화하지 말 것). workflow가 `-- --ignored`로 전부 실행하므로 항상 실패한다.
- 고치려면 workflow가 문서화된 kill gate test를 제외해야 한다(workflow 수정, 소유자 결정).

### 3.7 research-process

- `research/qualification/`: 사전등록 파일과 ledger 행이 없다(채택 때 보고한 미해결 항목). 사후에 사전등록을 만들 수는 없다.
- stall rule 5건: switcher node `L-0210`..`L-0214`. 이 loop가 INVALID 대체 행(`L-0308`..`L-0312`)을 추가했지만 규칙은 원래 행을 본다.
- base의 "missing commit" 247건은 head에서 사라졌다.

## 4. 릴리스 게이트 영향

- R4 재현성: NOT_MET 유지. 근거가 구체화됐다: hosted CI 8개 job 실패, bit anchor가 host에 의존, `test` job이 첫 실패에서 멈춤.
- R3 API: NOT_MET 유지. 채택된 계보가 semver major 변경을 포함한다.
- R6 패키징: NOT_MET 유지. workspace 밖 crate 2개가 고정 toolchain으로 빌드되지 않는다.
- R7 증거 거버넌스: NOT_MET 유지. `research/qualification/`과 stall rule.

## 5. 조치

| 조치 | 상태 |
|---|---|
| docs link 수정 | 완료 (이 commit) |
| `vigilode-bench`, `vigilode-hybrid`를 1.94.1에서 빌드·시험하고 `rust-version` 정렬 | V11(기하 battery)에서 수행 |
| host 의존 anchor의 원인 확정(CPU feature별 실행) | 미수행. 다음 loop 후보 |
| workflow 수정: `test`에 `--no-fail-fast`, `bench`에 MPI header, `ignored-tests`에서 kill gate 제외 | 소유자 결정 필요 |
| semver: 버전 0.2.0 또는 API 호환 유지 | 소유자 결정 필요 |
| `research/qualification/` 처리(삭제, INVALID 행, 또는 사후 기록 명시) | 소유자 결정 필요 |

## 6. 2026-11-23 조치 (소유자 결정 OD-L1-12)

소유자가 workflow 수정과 버전 0.2.0을 승인했다. 적용 내용과 5절 표의 갱신 상태:

| 조치 | 상태 |
|---|---|
| `test`에 `--no-fail-fast` | 완료. 두 step 모두. `Integrators` step에 `if: ${{ !cancelled() }}`를 더했다(없으면 앞 step이 실패할 때 실행되지 않는다) |
| `ignored-tests`에서 kill gate 제외 | 완료. `--skip prothero_robinson_wfiltered_enforce_kill_gate`. `tools/check_ignored_tests_in_ci.py`가 제외 목록을 검사한다 |
| `bench`의 CVODE bridge 컴파일 | 완료, 단 처방을 바꿨다(6.1). MPI 패키지 추가가 아니라 `runs-on: ubuntu-24.04` |
| semver: 버전 0.2.0 | 완료. `[workspace.package] version` (6개 crate). hosted `semver` job 결과는 push 뒤 확인 |
| `vigilode-bench`, `vigilode-hybrid`의 `rust-version` 정렬 | 미수행. V11. 그때까지 `bench` job은 이 원인으로 계속 실패한다 |
| host 의존 anchor의 원인 확정 | 미수행. 다음 loop 후보 |
| `research/qualification/` 처리 | 소유자 결정 필요 (이번 승인에 없음) |

### 6.1 bench: 3.3절 처방의 정정

3.3절은 "apt-get에 MPI 개발 패키지 추가"를 제안했다. 컨테이너에서 bridge 컴파일 명령(build script와 같은 gcc 인자)을 실행해 보니 그 처방은 통하지 않는다.

| 환경 | `libsundials-dev` | 결과 |
|---|---|---|
| ubuntu:26.04 (runner image), workflow의 패키지 그대로 | 7.1.1 | 실패. hosted와 같은 오류(`sundials_types.h:57: mpi.h`) |
| ubuntu:26.04 + `libopenmpi-dev` | 7.1.1 | 실패. 같은 오류. Ubuntu는 `mpi.h`를 gcc 기본 검색 경로 밖에 둔다 |
| ubuntu:26.04 + `libopenmpi-dev` + `CPATH` | 7.1.1 | 실패. 오류 8건. SUNDIALS 7에는 bridge가 쓰는 `realtype`이 없다 |
| ubuntu:24.04, workflow의 패키지 그대로 | 6.4.1 | 성공 |

- 원인은 MPI header가 아니라 SUNDIALS major 버전이다. bridge와 지금까지의 비교 증거는 CVODE 6.4.1 기준이다.
- 조치: `bench` job만 `ubuntu-24.04`에 고정했다. 비교 solver 버전이 증거와 같아진다.
- 다른 길: bridge를 SUNDIALS 7로 옮긴다. 비교 대상 버전이 바뀌므로 연구 결정이다. 하지 않았다.
- 로그: `raw/local_probe_bench_bridge_ubuntu2404.log`, `raw/local_probe_bench_bridge_ubuntu2604.log`.

### 6.2 버전 범위

- 바꾼 것: `[workspace.package] version` 0.1.0 → 0.2.0. 이 값을 쓰는 crate는 `vigilode`, `rodas5p-core`, `rodas5p-krylov`, `rodas5p-integrators`, `rodas5p-cli`, `rodas5p-fair-ab`.
- 그대로 둔 것: `vigilode-coupled`, `vigilode-bench`, `vigilode-hybrid` (자체 버전 0.1.0). semver 지적은 `vigilode`의 공개 API였고, `vigilode-coupled`는 `vigilode`에 의존하지 않는다.
- lock 파일 5개에서 위 path crate의 버전 줄만 고쳤다(root, `vigilode-bench`, `vigilode-hybrid`, loop runner 2개). 5개 모두 `cargo +1.94.1 metadata --locked` exit 0.
- 릴리스나 배포는 아니다. 승인 범위 밖이다.

### 6.3 검증 상태

- 로컬에서 확인: 검사 스크립트(96개 중 95개 도달, 문서화된 제외 1개; 문서화되지 않은 skip, 실행되는 제외 항목, 없는 이름은 exit 1), workflow YAML 구문, lock 5개.
- 로컬에서 확인하지 않음: hosted job 결과. push 뒤 PR #190 실행으로 확인한다.

## 7. 2026-11-23 추가 조치 (소유자 결정 OD-L1-13, OD-L1-14)

- ledger: 형식 위반 4행(L-0317~L-0320)을 제자리에서 고쳤다. `tools/check-research-node.py`가 ledger 줄을 더 이상 지적하지 않는다.
- `research/qualification/`: 파일은 `negative_controls/negative_fixture_manifest.json` 하나다. `Q_NEGATIVE_EVIDENCE.json`이 인용하고 "negative control의 임계값은 완화하지 않는다"는 규칙을 담고 있어 그대로 둔다. fixture 14개의 hash와 크기가 tree와 일치한다(`verify_negative_controls.py`). 사전등록 파일 요건은 `tools/research_node_exemptions.txt`에 사유와 함께 면제 항목으로 적었다.
- kill gate: `ignored-tests` job에서 단순 제외 대신 별도 단계로 실행한다. 그 test가 기록된 서명으로 실패할 때만 단계가 통과한다. negative control이 CI에서 계속 실행된다.
- 발견: `Q_NEGATIVE_EVIDENCE.json`이 적은 manifest hash(82f7d983...)는 commit된 파일의 hash(e3b06b7e...)와 다르다. 채택한 switcher 계보의 증거 기록 문제이며 이번에 고치지 않았다.
- 남는 `research-process` 실패: stall rule 5건(switcher node L-0210~L-0214). 이 node들은 유효한 수치 결과가 없어서 행을 고쳐 해소할 수 없다.


## 8. 2026-11-23 밤: 호스티드 결과와 두 번째 음성대조

head 892132e5의 호스티드 실행(run 45187691774, 45187691424):

- Production CI: 다음 push가 취소했다. 취소 전에 fmt, docs, clippy, semver는 통과했고 bench는 알려진 대로 `vigilode-bench`의 rust-version에서 멈췄다(V11).
- Ignored long tests: 도달성 검사 통과. kill gate 음성대조 단계는 runner에서도 통과했다(`KILL GATE: 2.143x the Off accepted-step count > 1.3x`). 전체 workspace 단계는 test 하나에서 실패했다: `stage_clients_krylov_contracts.rs`의 `merged_hook_solution_meets_the_protected_threshold`. 이 loop의 V06 node(2026-11-17)가 넣은 test이고 ignore 사유가 "fails by design"이다. 병합된 stage hook이 잔차 1.032519640099976e-4인 해를 받아들이는데 보호 driver의 기준은 1.9887688296747326e-9라는 사실을 실패하는 assertion으로 적어 둔 것이다. 오늘 저녁까지는 kill gate가 먼저 cargo를 멈춰서 이 test binary까지 간 적이 없었다. 그 단계에 `--no-fail-fast`가 없어서 그 뒤 test binary들은 실행되지 않았다.

조치(OD-L1-12 범위): `ignored-tests.yml`에서 이 test를 전체 workspace 단계에서 빼고 kill gate와 같은 형식의 별도 단계로 실행한다. 그 단계는 test가 기록된 문구로 실패할 때만 통과한다. 전체 workspace 단계에 `--no-fail-fast`를 넣었다. `tools/check_ignored_tests_in_ci.py`: ignored test 96개 전부 도달, 제외 항목 없음. local에서 같은 논리를 release profile로 돌린 결과 runner와 같은 두 숫자로 실패한다(`raw/local_probe_merged_hook_negative_control.log`).

설계상 실패하는 ignored test는 이 둘뿐이다(`#[ignore]` 사유 전수 확인).

head 1fcf356c의 호스티드 실행(run 45190467664, 45190467441; 2026-11-24 00:15 KST에 확인):

- Production CI: fmt, docs, clippy, semver 통과. bench는 알려진 대로 `vigilode-bench`의 rust-version에서 멈춤. test job은 두 단계가 처음으로 모두 끝까지 실행됐다(`--no-fail-fast`와 `if: !cancelled()`의 효과).
  - Facade and light crates: 결과 줄 183개, 1211 passed, 1 failed, 22 ignored. 실패는 `rodas5p-krylov`의 `staged_solve_telemetry::s1_nine_native_cases_match_anchor` 하나(알려진 host 의존 bit anchor).
  - Integrators: 결과 줄 270개, 1448 passed, 1 failed, 65 ignored. 실패는 `stage_residual_telemetry::test_r_parity_on_off_and_alg06_anchor` 하나다. 기록된 f64 bit 세 개가 runner에서 하위 bit가 다르다(`3e6bf6471c33a9ed` 대 `3e6bf64d54ceae64` 등). 이것도 host 의존 anchor다.
  - 즉 runner에서 ignore되지 않은 test 2661개 가운데 실패는 host 의존 bit anchor 두 개뿐이다.
- Ignored long tests: 취소됨. job 제한 60분에 걸렸다. `ppsw1_linear_backend_contracts.rs`의 측정 test `c5_measured_crossover_paired_interleaved` 하나가 46분 넘게 돌고 있었다(892132e5 실행에서는 같은 test가 1678초). job 전체가 취소되면서 음성대조 단계도 건너뛰었다.

조치(OD-L1-12 범위): `ignored-tests.yml`에서 음성대조 두 단계를 긴 전체 workspace 단계 앞으로 옮겼고(각 단계 `if: !cancelled()`), job 제한을 180분으로 늘렸다. 전체 workspace 단계는 호스티드 runner에서 한 번도 끝까지 간 적이 없다(어제까지는 실패하는 test에서, 그 뒤로는 60분 제한에서 멈춤). 처음 완주하는 실행이 필요한 시간을 알려 준다. 남는 결정: 계약형 ignored test는 PR마다, 측정 프로그램은 예약 실행이나 수동 실행으로 나눌지 여부. 도달성 검사는 ignored test 98개 전부 도달로 통과한다(V08, V18 병합으로 2개 늘었다).

head 8f840859(group 3 병합 포함)의 호스티드 실행(run 45203782039, 45203782197; 2026-11-24 01:55 KST에 확인):

- Production CI: fmt, docs, clippy, semver 통과, bench는 같은 이유로 실패. test job 두 단계 모두 끝까지 실행: Facade and light crates 1247 passed, 1 failed, 22 ignored; Integrators 1472 passed, 1 failed, 67 ignored. 실패는 앞 실행과 같은 host 의존 bit anchor 두 개뿐이다(V03b, V08, V18 병합 뒤에도 그대로).
- Ignored long tests: 도달성 검사와 음성대조 두 단계(kill gate, 병합 hook의 정지 검사 차이)가 runner에서 통과했다. 전체 workspace 단계는 확인 시점에 실행 중이었다(제한 180분). 이 단계가 처음 완주하면 걸린 시간과 그 뒤 test binary들의 결과를 다음 세션에서 읽는다.

## 9. 2026-11-24: 변경 기반 test 선택 (소유자 결정 OD-L1-19)

소유자 지시(원문): "test job은 누적해서 실행하지 말고 새로 실행해야 하는 것이나 이전 변경에 영향받는 것만 재실행하는 방향으로 수정. full suite는 최종 release 전에 실행 예정." 8절 끝의 분리안(A, B)은 이 지시로 대체됐다.

`tools/ci/select_tests.py`가 Production CI의 test job과 Ignored long tests job이 돌릴 test를 고른다. 조정할 숫자는 없다.

- 기준점(baseline): 같은 branch에서 같은 job이 selection을 끝까지 실행한(outcome이 complete) 가장 최근 run의 head. 취소된 run이나 test 전에 깨진 run은 complete outcome을 남기지 않으므로 그 변경은 다음 diff에 그대로 남는다. outcome과 selection은 artifact(`ci-test-outcome-test`, `ci-test-outcome-ignored-tests`)로 남는다.
- full: 수동 실행(workflow_dispatch, 입력 full 기본값 true), main branch, 기준점이 없거나 head의 조상이 아닐 때, 전역 파일 변경(workspace manifest, lock 파일, toolchain, `.cargo/`, `tools/ci/`, 그 job의 workflow 파일).
- package source 변경(`src/**/*.rs`, `build.rs`, `Cargo.toml`): 그 package와, 그 package에 의존하는 모든 workspace package(일반, dev, build 의존, 전이)의 test 전부. Rust integration test는 crate 전체를 link하므로 이것이 정적으로 안전한 가장 작은 단위다.
- `examples/`, `benches/` 변경: 그 package만.
- test source 변경(`tests/**/*.rs`): cargo가 test binary를 만들 때 쓰는 dep-info에 그 파일이 들어 있는 target만. 어느 target에도 없으면 그 package의 test target 전부.
- 데이터 파일(위가 아닌 모든 파일): test는 문자열 literal로 적힌 경로로만 파일을 읽는다. 모든 workspace Rust source의 경로형 문자열 literal(주석 제외)을 변경 파일과 그 상위 디렉터리에 대조한다(같거나 '/' 뒤 접미사). src의 literal이 맞으면 그 package 변경, tests의 literal이 맞으면 그 test source 변경. 맞는 literal이 없으면 읽는 test가 없다.
- 기준점 run에서 실패한 target은 다시 넣는다(호스트 의존 anchor 두 개는 고쳐질 때까지 매번 돈다).
- 범위: test job은 core, krylov, integrators, vigilode, vigilode-coupled; ignored job은 workspace 7개 전부. 음성대조 두 단계는 해당 target이 선택됐을 때만 돈다. `tools/check_ignored_tests_in_ci.py`는 selector의 ignored 실행을 full mode의 workspace 실행으로 보고 도달성을 계속 검사한다(99개 전부 도달).

로컬 모의 실행(과거 commit을 head로, 그 부모를 기준점으로):

| commit | 내용 | 선택 |
|---|---|---|
| 2f663808 | research 문서만 | test 없음 |
| 8b76ebd4 | ignored-tests.yml과 triage(Production CI 기준) | test 없음 |
| 1cd674a6 | vigilode `solve.rs`, golden 파일, research | vigilode 전체 |
| 1d811039 | integrators `stage_clients/banded_tight.rs`와 test | integrators, vigilode, vigilode-coupled 전체 |
| 66c558a0 | vigilode test 파일 하나와 그 golden 파일 | `banded_kernel_route_contracts` 하나(기준점 실패 하나를 넣으면 그 target도) |

데이터 파일 대조 예: `fixtures/rodas5p_coefficients_snapshot.json` → core의 `coefficients.rs`(include_str)와 `core_contracts.rs` → core 변경이므로 사실상 전체. `research/generic_timing_replication_continuation_transaction_v37/results/` 안의 다른 파일 → 읽는 test 없음(v37 test는 V4 snapshot 하나만 읽는다).

한계: 실행 시간에 환경변수 등으로 조립하는 경로는 literal 대조에 잡히지 않는다. crate 안에서 더 잘게(파일 단위) 고르려면 coverage 기반 test 영향 지도가 필요하다. 최종 release 전 full suite가 이 둘을 덮는다.

## 10. 2026-11-24: coverage 기반 세분 선택 (소유자 결정 OD-L1-20, "잘게 고르는 단계 승인")

9절의 규칙에 두 가지를 더했다.

1. 변경 분류(Rust tokenizer): 수정된 package source 파일의 token이 같으면(주석이나 공백만 바뀜, doc 주석 제외) 효과가 없다. 바뀐 줄이 old, new 양쪽 모두 const fn이 아닌 함수의 본문 안에만 있으면 본문 변경이다. 나머지는 구조 변경이다: item, signature, const, static, type, trait, macro, attribute, doc 주석, const fn 본문, 새 파일, 삭제, 이름 변경, `Cargo.toml`, `build.rs`.
2. coverage 지도: 두 test job은 선택한 target을 coverage 계측(`-C instrument-coverage`)으로 하나씩 실행한다. target마다 실제로 실행한 workspace 소스 파일을 기록하고, 지도를 outcome과 함께 artifact로 다음 run에 넘긴다. 실행된 target의 항목은 매번 새 값으로 바뀐다. profile을 남기지 못한 target은 항목을 잃고, 다음에 해당 package가 변경되면 선택된다.

선택 규칙은 다음과 같다.

- 구조 변경: 9절과 같다(package와 그 의존 package의 test 전부).
- 본문 변경: 지도에서 바뀐 파일을 실행한 target만 선택한다. 그 package와 의존 package의 target 가운데 지도 항목이 없는 target도 넣는다. doc test는 stable에서 coverage가 없으므로 그 package와 의존 package의 doc test를 돌린다.
- 지도가 없으면: 본문 변경도 구조 변경으로 다룬다(9절 규칙).

근거: 함수 본문의 변경은 그 함수가 실행될 때만 행동을 바꾼다. 어떤 target이 마지막 실행 때 그 파일을 실행하지 않았고, 그 뒤로 그 target이 실행한 파일이 하나도 바뀌지 않았다면, 그 target은 같은 코드를 다시 실행한다. 지도가 매 run 갱신되므로 이 논리가 run마다 이어진다. 가정은 test가 결정적이고 CI 기본값 밖의 환경에 의존하지 않는다는 것이다. 남는 틈은 impl Trait 뒤 hidden type의 auto trait가 본문 변경으로 바뀌어 선택되지 않은 target의 컴파일이 깨지는 경우다. release 전 full suite가 이를 덮는다.

로컬 검증:
- self-test: 분류 사례 9개 포함 통과.
- 합성 지도와 scratch commit으로 모의 실행:
  - `unified/analyzer.rs`의 함수 본문 변경 → 그 파일을 실행한 target 2개와 doc test만 선택(vigilode target 56개 중).
  - 같은 파일의 const 변경 → vigilode 전체.
- 실제 계측 실행(LLVM 21.1.8, rustc 1.94.1과 같은 버전):
  - target 3개 모두 지도 갱신, 누락 0.
  - 실행 파일 수: `banded_kernel_route_contracts` 18개(solve.rs 포함, unified 없음), `unified_router_contracts` 56개(unified 전부), krylov `staged_solve_telemetry` 10개(`gmres_staged.rs`).
  - 같은 target을 다시 실행하면 같은 파일 목록이 나왔다.
  - 계측 상태에서 doc test(RUSTFLAGS, RUSTDOCFLAGS 모두 설정)도 통과했다.

비용: full 실행은 target마다 cargo test를 한 번씩 돌리고 지도 export를 병렬로 하므로 길어진다. timeout은 test job 300분, ignored job 330분으로 올렸다.

## 11. 2026-11-24: 선택 v3, 측정한 입력 (소유자 결정 OD-L1-21, "전부 승인")

v1/v2 검토(`SELECTION_REVIEW.json`, 지적 26건)는 정적 규칙(package 단위 규칙, 문자열 literal 대조)이 test의 입력을 다 알지 못한다는 것을 보였다. v3는 입력을 추측하지 않고 매 run 측정한다. 구현: `tools/ci/select_tests.py`(v3), `production.yml`, `ignored-tests.yml`, `a1-inner-tolerance-parity.yml`, `tools/check_ignored_tests_in_ci.py`. 설계가 열어 둔 선택은 `V3_NOTES.md`의 R1-R23에, 검토 두 건을 반영한 수정 라운드의 결정은 R24-R36에 적었다(11.6). 로그는 `v3_logs/`.

### 11.1 무엇을 측정하는가

매 run은 실행한 target마다 아래 집합을 지도(`vigilode.ci.coverage-map.v2`)에 새로 기록한다. 실행하지 않은 target의 항목은 기준점 지도에서 그대로 넘긴다. 측정에 실패한 target은 항목을 잃고 다음 run에서 다시 실행된다.

| 집합 | 출처 | 잡는 연결 (검토 지적) |
|---|---|---|
| compile | target 자신의 artifact의 rustc dep-info: integration test, lib·bin unit test(profile.test), example | `mod`, `#[path]`, `include!`, `include_str!`, `include_bytes!` (F1, F2, F3, F5, F11) |
| executed | coverage에서 실행 영역이 있는 파일(package의 binary 포함) | 함수 본문 변경 (OD-L1-20) |
| read | test process와 자식 process가 성공적으로 open, stat, access, readlink, execve한 경로 | 실행 시간 읽기: hash 고정, 표에 적힌 파일 (F4, F6, F7) |
| dirs | 디렉터리로 연 경로(`O_DIRECTORY`): 목록을 읽었다 | 파일 추가·삭제 (F7) |
| absent | `ENOENT`, `ENOTDIR`로 실패한 조회 | 없던 파일이 생기는 경우 |

- package마다 다른 crate가 link하는 non-test lib·bin의 dep-info도 기록한다. test artifact의 dep-info에는 link하는 library의 파일이 없기 때문이다. 이 집합의 Rust가 아닌 파일은 구조 변경이다. 로컬 실행에서는 두 개였다: core의 `fixtures/rodas5p_coefficients_snapshot.json`, fair-ab의 `tools/reference_v2/generate_references_v2.py`.
- strace는 cargo의 target runner(`CARGO_TARGET_<host>_RUNNER`)로 test 실행 파일과 그 자식에만 붙는다. seccomp filter 때문에 위 집합에 필요한 system call에서만 멈춘다. `-y`로 상대 경로와 descriptor 경로를 푼다.
- 저장소 밖, cargo target 디렉터리, `.git`, 측정 디렉터리의 경로는 버린다. doc test는 측정하지 않는다.
- 음성대조 두 개는 `select_tests.py control`로 같은 측정을 하며 실행하고 `<pkg>::<target>#<test>` 항목을 남긴다(R11). selector 자신의 run은 음성대조를 `--skip`하므로 그 target에서 test를 하나도 실행하지 않는다. 로컬 확인: 그 target의 항목은 executed 0개, kill gate 항목은 executed 17개.

### 11.2 규칙

1. full: 요청(workflow_dispatch의 full, main), 기준점이나 지도가 없음, 기준점 head가 head의 조상이 아님, runner image(`ImageOS`, `ImageVersion`)나 PR base sha가 기준점과 다름, 전역 파일 변경(`Cargo.toml`, `Cargo.lock`, `rust-toolchain*`, `.cargo/`, `tools/ci/`, 어느 깊이든 `.gitattributes`, 그 job의 workflow). 기준점은 같은 event, 같은 저장소, 같은 runner image에서 돈 run을 먼저 고른다(11.6).
2. 변경 경로: `git diff -z --name-status -M base head`. rename은 옛 경로 삭제와 새 경로 추가 둘 다.
3. 변경 경로가 compile, executed, read 중 하나에 있으면 그 target을 고른다. 추가·삭제된 경로는 목록이 바뀌는 디렉터리(양쪽에 있는 가장 가까운 조상)를 연 target, 그 변경이 만들거나 없애는 디렉터리를 조회하거나 읽거나 연 target, 추가된 경로를 조회했던 target도 고른다.
4. package source(`Cargo.toml`, `build.rs`, `src/**/*.rs`, 기록된 lib·bin 입력)는 v2 분류를 유지한다. 구조 변경: package와 그에 의존하는 package 전체. 본문 변경: 규칙 3과 package 및 의존 package의 doc test. noop(주석, 공백; 붙어 있던 구두점을 떼거나 떨어진 구두점을 붙이면 noop이 아니다): package 규칙 없음. 규칙 3은 모든 target의 read 집합과, 그 package 자신의 unit test가 아닌 target의 compile 집합만 센다. 파일을 text로 쓰는 쪽(`include_str!`, `include_bytes!`, 실행 시간 읽기)은 dep-info나 strace에 반드시 나타나므로 빠지지 않고, Rust 코드로 compile하는 쪽은 같은 token을 본다(R8).
5. 지도 항목이 없는 target(새 target, 측정 실패)과 항목이 없는 음성대조의 target은 diff가 비어 있지 않으면 실행한다. lib·bin 입력이 기록되지 않은 package는 구조 변경으로 본다.
6. 기준점에서 실패한 target(실패한 음성대조 포함)은 지금도 있고 required-features가 기본으로 켜지면 다시 실행한다. 아니면 note와 함께 뺀다.
7. compile 집합에 변경 경로가 있는 example은 컴파일한다.
8. doc test는 package가 전체 실행되거나 선택된 target이 있을 때, 그리고 본문 변경 때 돈다.
9. literal 대조는 없앴다(R12). 아무 target도 고르지 않은 경로에는 지도 기준 note가 붙는다.

동작: 두 test job은 취소하지 않고 대기한다(job 단위 concurrency, `cancel-in-progress: false`). workflow 단위 group은 test job까지 취소하므로 없앴고, 짧은 job은 각자의 group으로 non-main ref에서 계속 취소한다. 음성대조가 실패하면 그 target이 outcome의 `failed`에 들어가므로 기준점이 넘어가도 다음 run이 다시 고른다. selector 단계는 모두 `python3 -u`, branch·head·base는 env로 넘기고, upload는 `overwrite: true`. 두 job은 먼저 self-test를 돈다. A1 workflow의 "Run the workspace test closure" 단계는 지웠다. 주기적 full 실행은 넣지 않았다(OD-L1-21 4항).

### 11.3 검토 지적 처리

| 지적 | 상태 | 방법 |
|---|---|---|
| F1 | 해결 | `other_rust` 분류를 없앴다. `#[path]`로 들어간 파일은 test의 compile 집합에 있다. 사례 F1(95d589e86 재현)이 `residual_gain_contract`를 고른다 |
| F2 | 해결 | `include_str!`한 cli `main.rs`가 `constants_provenance_contracts`의 compile 집합에 있다. 구조, 본문, 주석만 바꾼 경우 모두 고른다 |
| F3 | 해결 | 규칙 3은 모든 package의 target을 본다. `alg02_predictive_controller`의 compile 집합에 `rnext_common/mod.rs` |
| F4 | 해결 | sha256 고정은 strace의 read 집합으로 잡힌다(`ptsw2b_contracts`, `ppsw1_6b_contracts`) |
| F5 | 해결 | lib·bin unit test artifact도 지도에 들어간다. integrators `lib`의 compile 집합에 `tests/support/a02_packet_counter_contract.rs` |
| F6 | 해결 | TOML 표가 가리키는 파일도 read 집합에 있다(`FROZEN_ZETA34_POLICY.json` 등 3개) |
| F7 | 해결 | `generic_scope_contracts`의 read 집합 308개, dirs 20개. 비회원 crate(`vigilode-hybrid`, `vigilode-bench`)도 포함. 새 파일은 목록 규칙으로 고른다 |
| F8, F-04 | 해결 | PR base sha를 기록하고 바뀌면 full(소유자 선택). checkout은 merge commit 유지 |
| F9, F-03 | 해결 | 실패한 음성대조를 outcome의 `failed`에 합친다. 음성대조 자체도 측정한다(R11) |
| F10, F-02 | 해결 | required-features가 꺼진 target은 이름으로 부르지 않는다. 기준점 실패는 현재 target으로 거르고 없어진 것은 note와 함께 뺀다 |
| F11 | 해결 | example의 compile 집합을 기록하고 해당 example을 `--example`로 컴파일한다 |
| F12 | 해당 없음 | literal 대조를 없앴다 |
| F13 | 해결 | `git diff -z`. dep-info 경로는 저장소 root 기준으로 정규화 |
| F14 | 해결(나머지는 소유자 결정) | runner image가 바뀌면 full. 주기적 full 실행은 OD-L1-21 4항에 따라 넣지 않았다. 놓친 변경은 release 전 full suite가 다시 본다 |
| F-01 | 해결 | test job은 대기, 짧은 job만 취소 |
| F-05 | 해결 | A1의 workspace test 단계 삭제 |
| F-06 | 해결 | `python3 -u`와 줄마다 flush |
| F-07 | 해결 | workflow는 `has`를 쓰지 않는다. 음성대조 단계는 `control`이 오류면 실패한다. `has`는 선택 파일이 없으면 exit 1 |
| F-08 | 해결 | `--workspace`는 회원 crate의 test에만 센다 |
| F-09 | 해결 | 목록 조회 실패와 다운로드 실패에 각각 note |
| F-10 | 해결(기존 조치 유지) | 제한 300분, 330분 유지. strace를 더한 full 실행의 hosted 시간은 아직 모른다(11.5) |
| F-11 | 해결 | `overwrite: true` |
| F-12 | 해결 | branch는 env로 넘기고 REST query에서 URL 인코딩 |

### 11.4 로컬 검증 (`v3_logs/`)

- self-test(`self_test.log`): 분류 9개, dep-info 정규화, `-z` 경로(따옴표, tab, 비ASCII, rename), strace 줄 해석, required-features, 기준점 실패 거르기, image·base 촉발, 지도 규칙 3-8(F1-F7, F11, 음성대조, 측정 없는 target과 package), 가짜 `gh`로 기준점 조회(F-09), workflow 점검(F-01, F-03, F-05, F-06, F-07, F-10, F-11, F-12, F14) 통과. 같은 workflow 점검을 v2 workflow에 돌리면 25건을 지적한다(`self_test_against_v2_workflows.log`).
- 측정 실행(`e2e_run.log`, `e2e_outcome.json`, `e2e_map.json`): coverage와 strace로 target 7개와 example 1개를 실행. 전부 통과, 측정 실패 0. debug profile, cores 0-5,12-17, host 부하 약 190(24 CPU). 시간은 libtest 보고값이다.

| target | 결과 | 시간 | compile | executed | read | dirs | 추적 호출 |
|---|---|---|---|---|---|---|---|
| core `residual_gain_contract` | 6 통과, 1 ignored | 0.03 s | 2 | 7 | 0 | 0 | 95 |
| integrators `lib` (unit test) | 141 통과, 1 ignored | 14.9 s | 119 | 76 | 0 | 0 | 106 |
| integrators `constants_provenance_contracts` | 2 통과 | 0.05 s | 4 | 3 | 3 | 0 | 97 |
| integrators `generic_scope_contracts` | 4 통과 | 1.7 s | 2 | 10 | 308 | 20 | 1007 |
| integrators `ppsw1_6b_contracts` | 37 통과 | 70.6 s | 1 | 9 | 15 | 0 | 126 |
| integrators `ptsw2b_contracts` | 24 통과 | 0.19 s | 1 | 8 | 12 | 0 | 120 |
| cli `alg02_predictive_controller` | 13 통과, 6 ignored | 192 s | 3 | 32 | 1 | 0 | 105 |
| integrators example `sp01_dupfix_profile` | 컴파일 | - | 3 | - | - | - | - |

- 측정이 찾은, 검토에 없던 연결: `alg02_predictive_controller`가 `research/stiff_native_benchmark_20261001/NATIVE.json`을 읽고, example `sp01_dupfix_profile`이 `alg01_common`을 거쳐 `rnext_common/mod.rs`도 compile한다.
- 첫 측정 시도는 selector 오류로 멈췄다(`e2e_attempt1_strace_works_bug.log`): strace 확인 함수가 경로 대신 버전 문자열을 돌려줬다. 고쳤고 self-test가 이 계약을 검사한다.
- 선택 사례(`selection_cases.log`, `selection_cases.json`): 위 지도에 음성대조 측정(`control_smoke.log`)을 더한 지도(`map_e2e_plus_control.json`)로, scratch clone(공유 object, sparse checkout)에 사례마다 commit을 만들고 selector 규칙을 실제 diff·분류·지도에 적용했다. target과 package 범위는 측정한 것으로 제한했다(CI에서는 첫 full run 뒤 모든 target에 항목이 있다). 15개 전부 통과.

| 사례 | 변경 | 선택 |
|---|---|---|
| F1 | `gain_witness.rs` identity fixture (95d589e86 재현) | core `residual_gain_contract` (+ core doc) |
| F2 | cli `main.rs` 주석만 | `constants_provenance_contracts`, `generic_scope_contracts` (+ doc). cli 자신의 crate는 고르지 않음 |
| F3 | `rnext_common/mod.rs`에 helper 추가 | cli `alg02_predictive_controller`, example `sp01_dupfix_profile` |
| F4 | `unified_candidate_contracts.rs`에 test 추가 | `ptsw2b_contracts`, `ppsw1_6b_contracts` (+ doc) |
| F5 | `tests/support/a02_packet_counter_contract.rs` | integrators `lib` (+ doc) |
| F6 | `FROZEN_ZETA34_POLICY.json` 값 변경 | `constants_provenance_contracts` (+ doc) |
| F7 | `crates/rodas5p-krylov/src`에 새 파일 | krylov 구조 변경: krylov, integrators, fair-ab, cli 전체 (`generic_scope_contracts` 포함) |
| F7-hybrid | 비회원 `vigilode-hybrid/src`에 새 파일 | `generic_scope_contracts` (목록 규칙) |
| F7-vigilode | `vigilode/src/lib.rs` 주석만 | `generic_scope_contracts` (read) |
| F11 | `tests/alg01_common/mod.rs`에 helper 추가 | example `sp01_dupfix_profile` 컴파일 |
| N1 | `rodas5p-krylov/src/lib.rs` 주석만 | `generic_scope_contracts`(이 파일을 text로 읽는 유일한 target) + doc. 실행하거나 compile한 target은 고르지 않음 |
| N1-no-reader | N1, 그 reader를 뺀 지도 | 없음 |
| N2 | `ppsw1_linear_backend.rs` 함수 본문 | `ppsw1_6b_contracts`(실행), `generic_scope_contracts`(읽기), integrators `lib`(자기 crate compile, R9), doc(integrators, fair-ab). 나머지 target은 고르지 않음 |
| C1 | `dense_output_v2.rs` 함수 본문(kill gate가 실행) | `dense_wfiltered_contracts`(음성대조 항목으로), `lib`, `generic_scope_contracts`, doc |
| S1 | core `lib.rs`에 const 추가 | core와 의존 package 전체 |

- 음성 사례 두 개에 대한 정직한 판정: member의 `src` 파일은 모두 `generic_scope_contracts`가 text로 읽으므로 "아무 target도 읽지 않는 파일"은 member source에 없다. 그래서 주석 변경은 그 reader 하나만 고르고(N1), reader를 빼면 아무것도 고르지 않는다(N1-no-reader). 본문 변경은 실행한 target과 doc test 외에 그 reader와 자기 package의 lib unit test(R9: compile 집합에 crate의 모든 source가 있다)를 고른다(N2).
- 같은 사례를 실제 `select` 명령(제한 없는 workspace)으로도 돌렸다. 기대한 target을 모두 골랐고, 측정하지 않은 target 448-449개와 lib 입력이 기록되지 않은 package 2개(`vigilode`, `vigilode-coupled`)를 규칙 5가 더했다. 로컬 지도는 target 10개만 측정했으므로 예상한 결과다.
- `control` 명령(`control_smoke.log`): kill gate가 기록된 서명으로 실패하므로 통과(debug에서도 `KILL GATE: 2.143x the Off accepted-step count > 1.3x`), 틀린 서명이면 단계가 실패하고 `failed-controls.txt`에 target을 쓴다, 선택되지 않으면 실행하지 않는다. run 단계는 음성대조 조각을 지도에 합치고 실패를 outcome `failed`에 `negative_control`로 넣는다.
- 기준점 조회(`baseline_probe_f09.log`, 가짜 `gh`): 더 새 run, 조상이 아닌 head, 미완료 outcome, 다운로드 실패(HTTP 502)를 건너뛰고 완료된 run을 고른다. 목록 조회 실패는 자체 note를 남긴다. shell 문법이 든 branch 이름은 데이터로만 다뤄진다.
- 도달성 검사(`check_ignored_tests_in_ci.log`): ignored test 99개 전부 도달. 탐침(`checker_probe_f08.log`): `crates/vigilode-bench`의 ignored test는 이제 미도달로 잡히고, control 단계를 지우면 음성대조 두 개가 미도달로 잡힌다.
- workflow YAML 3개 구문 통과(`workflows_yaml.log`, python yaml; actionlint는 설치되어 있지 않다). loop 검사기 통과.

### 11.5 남는 틈과 비용

| 틈 | 영향 | 막는 방법과 그 비용 |
|---|---|---|
| doc test는 측정하지 않는다 | 실행 시간에 저장소 파일을 읽는 doc test는, 그 package에서 아무것도 선택되지 않으면 돌지 않는다. member의 doc 주석에서 파일 접근은 찾지 못했다(grep) | cargo 1.94.1은 runner를 doc test에도 넘기므로 `doc` 단위로 측정할 수 있다. 비용: doc test마다 strace, 설계 변경 |
| noop 변경의 span | 주석만 바꿔도 아래 줄 번호가 바뀐다. `line!()`, panic 위치, `Location::caller()`를 비교하는 test는 선택되지 않는다 | 줄 수가 바뀌는 noop을 본문 변경으로 보면 막힌다. 비용: 그 파일을 실행한 target 실행 |
| 본문 변경과 auto trait | `impl Trait`, `async fn`의 auto trait가 본문 변경으로 바뀌어 선택되지 않은 target의 컴파일이 깨질 수 있다(10절) | 의존 package의 `cargo check --tests`. 비용: 컴파일 시간 |
| 비결정적 입력 | 어떤 run에서만 읽는 파일(시각, thread 순서, 난수에 따른 경로)은 빠질 수 있다 | 없음. release full suite |
| 저장소 밖 입력 | env 변수, 네트워크, 시각. image, toolchain, lock 파일은 full 촉발이다 | 없음 |
| build script 입력 | `rerun-if-changed` 입력은 기록하지 않는다. fair-ab는 HEAD와 dirty 표시만 넣고 test 결과는 그 값에 의존하지 않는다(v1 검토 확인) | build script가 코드를 생성하게 되면 입력 기록이 필요하다 |
| `--skip`한 test | 걸러진 test의 입력은 측정되지 않는다. 선언한 음성대조 두 개는 따로 측정한다 | 새 skip은 `--control`로 선언 |
| strace가 못 보는 읽기 | io_uring, 다른 process(daemon)를 통한 읽기, 추적 전에 연 descriptor. workspace에는 없다 | 없음 |
| hosted full 실행 시간 | coverage와 strace를 함께 쓴 full 실행은 hosted에서 재지 않았다. 300·330분을 넘기면 outcome이 없어 다음 run도 full이다. strace 없는 v2 full 실행 두 개는 13:26Z에 157분째 진행 중이었다(11.6) | 첫 hosted full 실행으로 확인. 넘으면 job 분할 |
| 첫 run | schema와 `tools/ci/`가 바뀌었으므로 두 job의 첫 v3 run은 full이다 | 한 번뿐 |
| merge commit 검사 | PR은 merge commit을 검사하고 head끼리 diff한다. base sha가 같을 때만 맞으므로 base가 바뀌면 full | 소유자 선택 |

strace 비용(`strace_cost_probe.log`, 이 host, 부하 약 190):
- seccomp filter 덕분에 추적하지 않는 system call은 멈추지 않는다: `getppid` 2만 번은 strace 유무와 상관없이 0.03-0.11 s.
- 추적하는 호출은 한 번에 약 0.2-0.4 ms였다(`stat` 2만 번: 없이 0.22-0.38 s, strace로 4.6-7.6 s). 대부분 context switch 대기라서 부하가 없는 runner에서는 더 작다.
- 로컬 target은 추적 호출이 95-126개(loader, libtest 시작, 파일 읽기), `generic_scope_contracts`만 1007개였다. 0.4 ms로 잡아도 target당 0.05 s 이하, `generic_scope_contracts` 0.4 s.
- python3를 띄우는 target 8개(integrators `production_audit_*` 5개, `all_dag_*` 2개, krylov `givens_production_differential_contracts`)는 import 때문에 호출이 수천 개일 수 있다: target당 수 초.
- 따라서 full 실행(약 500 target)의 strace 비용은 수십 초 수준으로, 92-109분인 full test job에 비해 작다. 그 밖에 thread마다 trace 파일이 생기고(integrators `lib` 154개) Python이 그것을 해석하는 시간이 든다.

### 11.6 수정 라운드: b0d4bc97에 대한 검토 두 건

두 검토자가 b0d4bc97을 검토했다. 건전성 검토는 새 지적 S-N1~S-N6(모두 minor)과 이전 지적 S-F1~S-F14의 상태를, CI 동작 검토는 새 지적 M-N1~M-N8(M-N1, M-N2가 major)과 이전 지적 M-F-01~M-F-12의 상태를 냈다. 작업 지시문에는 CI 동작 검토가 14000자에서 잘려 있었으므로 이 run의 workflow journal에서 전체 JSON을 읽었다. 고치기 전에 지적마다 재현했다. 규칙은 `V3_NOTES.md`의 R24~R36, 로그는 `v3_logs/fix_*`.

무엇을 바꿨나:
1. 기준점(M-N1): hosted runner는 같은 시각에 image version 두 개를 섞어 준다. run 45358324432에서 08:41:48Z에 함께 시작한 `fmt`는 20261116.88.1, `clippy`는 20261116.202.1이었다(job 로그로 확인). 그대로면 run 10개 중 약 3개가 코드 변경 없이 full이다. 이제 기준점은 같은 runner image(`ImageOS`, `ImageVersion`)에서 돈 가장 새로운 완료 run이다. 이 image의 run이 하나도 없을 때만 다른 image의 run을 기준점으로 쓰고, 규칙 1에 따라 full이다. "image가 바뀌면 full"이라는 소유자 규칙은 그대로이고 image마다 기준점 사슬이 따로 이어진다. 조상인 완료 run은 어느 것이든 올바른 기준점이다: 그 뒤의 변경이 모두 diff에 들어간다(R24).
2. 기준점 후보는 같은 event, 같은 저장소(`head_repository`)의 run뿐이다(M-N3, M-N5, R25). 기록하는 PR base는 checkout한 merge commit의 첫 부모다(S-F8, M-F-04의 남은 가정, R26).
3. rodas5p-cli, rodas5p-fair-ab의 test job(M-N2): A1에서 지운 workspace test 단계가 이 두 package의 일반 test를 돌리던 유일한 곳이었다(hosted run 45358324393, coverage 없는 debug: cli 08:55-09:06, 그중 `triage_cli` 438 s; fair-ab 09:07-09:10). production.yml에 job `test-cli-fair-ab`를 더했다. 같은 selector와 측정, 자기 artifact(`ci-test-outcome-test-cli-fair-ab`), 대기 concurrency, 120분 제한. 처음에는 `test` job의 scope에 넣었으나(1288945b) 되돌렸다: 그 job의 v2 full 실행이 strace 없이도 157분을 넘겼고 300분 제한은 고정이며, production gate는 production crate만 맡는 것이 낫다(R27, R36).
4. trigger의 path filter 제거(S-N2, M-N4, S-F14 일부): 측정한 입력은 `docs/`, `research/`, `fixtures/`, `tools/`에도 있다(`docs/CONSTANTS_PROVENANCE.toml`, `FROZEN_ZETA34_POLICY.json`, `fixtures/rodas5p_coefficients_snapshot.json` 등). 두 test workflow는 모든 PR과 main push에서 돈다. 영향받는 test가 없으면 선택 단계에서 끝난다(R28).
5. tokenizer(S-N1): 사이에 아무것도 없는 두 구두점 사이에 표시 token을 넣는다. `&&`→`& &`, `=>`→`= >`, `..`→`. .`, `->`→`- >`, `+=`→`+ =`는 이제 noop이 아니다. 검토자 사례(`integrate.rs`의 `success & &collector`)는 body로 분류되고, 그 파일을 실행한 cli `alg02_predictive_controller`를 고른다(R29).
6. 추가된 경로가 만드는 디렉터리를 읽거나 연 target도 고른다(S-N3, R30). `.gitattributes`는 어느 깊이든 전역 파일이다(S-N4, R31).
7. strace는 경로를 만들고 지우고 이름을 바꾸고 속성을 바꾸는 호출(`mkdir`, `rename`, `link`, `symlink`, `unlink`, `rmdir`, `utimensat`, `chmod`, `truncate`, `statfs`, `getxattr` 등)도 추적한다. 성공은 read, ENOENT·ENOTDIR은 absent. 실패한 `chdir`은 absent이고 cwd를 바꾸지 않는다(S-N5, R32).
8. self-test의 `check_sources`가 R8, R10의 전제를 member의 `src/`에서 검사한다: `line!()`, `column!()`, `Location::caller`, `track_caller`, 자기 `.rs`의 text include, doc test 안의 파일 접근. 지금은 264개 파일에서 0건(S-N6, R33).
9. strace 설치 실패가 job을 멈추지 않는다(M-N6, R34). A1 workflow가 없어도 self-test가 실패하지 않는다(M-N8, R35).

| 지적 | 상태 | 방법 |
|---|---|---|
| S-F1~S-F7, S-F9~S-F11, S-F13 | 해결 | 검토자가 v3에서 원래 사례를 재현해 닫혔음을 확인했다. 이번 변경 뒤에도 선택 사례 F1-F7, F11이 같은 target을 고른다 |
| S-F8 | 해결 | 검토자 확인. 남은 가정(payload의 base = merge의 base)은 merge commit의 첫 부모를 기록해 없앴다(R26) |
| S-F12 | 해당 없음 | literal 대조가 없다 |
| S-F14 | 해결 | image 촉발(검토자 확인)에 더해 trigger의 path filter를 없앴다(R28). 주기적 full 실행은 OD-L1-21 4항에 따라 넣지 않았다 |
| S-N1 | 해결 | 붙은 구두점 표시(R29). self-test 9개 쌍, 검토자 사례 재현(`fix_tokenizer_probe.log`) |
| S-N2 | 해결 | path filter 제거(R28). 다시 넣으면 self-test가 실패한다 |
| S-N3 | 해결 | 추가 경로가 만드는 디렉터리를 read, dirs와도 대조(R30). self-test, 사례 S-N3 |
| S-N4 | 해결 | `.gitattributes`는 전역(R31). self-test, 사례 S-N4(full) |
| S-N5 | 해결 | 추적 호출 확대, 실패한 chdir은 absent(R32). strace 6.8 탐침(`fix_strace_calls_probe.log`) |
| S-N6 | 해결 | `check_sources`(R33) |
| M-F-01~M-F-12 | 해결 | 검토자가 재현해 닫혔음을 확인했다 |
| M-N1 | 해결 | 같은 image의 run을 기준점으로(R24). self-test: 오래된 같은 image run 선택, 같은 image run이 없을 때의 대체와 full |
| M-N2 | 해결 | job `test-cli-fair-ab`(R27). self-test: 각 workflow의 선택 job이 모든 member를 덮고 artifact 이름이 겹치지 않는지 검사 |
| M-N3 | 해결 | 같은 event의 run만(R25) |
| M-N4 | 해결 | S-N2와 같다(R28) |
| M-N5 | 해결 | 같은 저장소의 run만(R25) |
| M-N6 | 해결 | strace 설치 단계에 `continue-on-error: true`(R34) |
| M-N7 | 미해결(기록) | v2 hosted full 실행 두 개가 13:26Z에 157분째 진행 중이었다(완료된 v1 full 실행은 92-109분). 제한 300·330분은 설계대로 유지. 첫 v3 full 실행이 제한의 2/3를 넘으면 full mode 분할을 소유자에게 묻는다(R36) |
| M-N8 | 해결 | A1 workflow가 있을 때만 검사(R35) |

검증(`v3_logs/fix_*`):
- self-test(`fix_self_test.log`) 통과. 새 사례: 구두점 9쌍, `.gitattributes`, 새 추적 호출 12줄과 인자 분리, 만들어진 디렉터리 규칙, 가짜 `gh`로 같은 image 기준점·대체·fork·다른 event, merge commit의 base, `check_sources` 탐침 workspace, workflow 변형(path filter, strace 설치, scope, artifact 이름, A1 없음).
- 같은 workflow 점검을 b0d4bc97의 workflow에 돌리면 6건을 지적한다(`fix_checks_against_b0d4bc97_workflows.log`).
- strace 6.8 탐침(`fix_strace_calls_probe.log`): 늘린 호출 목록을 `--seccomp-bpf`와 함께 받아들이고, 해석 결과가 기대와 같다. 탐침은 지우거나 바꾸는 호출을 모두 없는 경로에 대고 하므로 아무것도 지우지 않는다.
- tokenizer 탐침(`fix_tokenizer_probe.log`).
<!-- E2E -->
- 도달성 검사 99/99, workflow YAML 3개 구문 통과, loop 검사기 통과(`fix_checks.log`).

남는 틈과 비용(11.5에 더해):

| 틈 또는 비용 | 내용 |
|---|---|
| image마다 기준점 사슬 | 새 image version의 첫 run은 full이다(전과 같다). 그 run은 같은 image의 run을 찾으려고 목록의 run artifact를 최대 100개 받는다 |
| path filter 없음 | 측정한 입력에 닿지 않는 PR도 두 workflow를 선택 단계까지 돌린다(수 분). production.yml의 다른 job(fmt, clippy, docs, semver, bench)도 돈다. PR의 path filter는 PR 전체 diff를 보므로 `crates/**`를 한 번 건드린 PR은 전에도 매 push마다 돌았다. 새로 도는 것은 data만 바꾸는 PR과 main push다 |
| `test-cli-fair-ab` | 무언가를 고르면 다른 runner에서 workspace를 coverage로 한 번 더 build한다. 아무것도 고르지 않으면 선택 단계에서 끝난다 |
| 늘린 추적 호출 | 파일을 많이 만들고 지우는 test는 호출마다 약 0.2-0.4 ms(이 host)를 더 쓴다 |
| `check_sources`의 한계 | 한 줄 pattern이다. 여러 줄에 걸친 구문, 다른 crate의 macro 뒤의 사용, block doc 주석(`/** */`)은 보지 않는다. 걸리면 두 test job이 self-test에서 실패하고, 고칠 것은 코드나 규칙이다 |
| 소유자 확인 | `test-cli-fair-ab`는 연구 crate 두 개에 새 CI job을 준다. 대안은 그 test를 release 전 full suite에서만 돌리는 것이다(R27) |
