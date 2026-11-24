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
