# R3 현재 실행환경 검증

- 고정 소스: `cc2cd041737e7ff543624d1b59893a3b4397369f`.
- 사용자 첨부 Rust 1.94.1 archive를 독립 `runtime_r3/`에 설치했다. `rustc`/`cargo`/`rustfmt` 실제 version 실행, Rust hello 컴파일 및 `RUST_EXECUTION_PASS 42` 실행 exit 0을 확인했다. 이전 turn PASS를 상속하지 않았다.
- `source /workspace/scratch/0dc1e721cf82/runtime_r3/env.sh`; `CARGO_HOME=runtime_r3/cargo-home`, shared `CARGO_TARGET_DIR=runtime_r3/target-shared`, `CARGO_BUILD_JOBS=1`.
- 모든 Cargo build는 `flock runtime_r3/cargo-build.lock`으로 직렬화한다. build 이후 독립 test binary 실행은 Cargo lock을 필요로 하지 않는다.
- Cargo.lock SHA-256 `04c7d6c147bf19ebb1a627463705d9dcf27337a3ff228dd1169f19eb73c71400`. 첨부 vendor의 262 packages 중 lock 요구 registry 의존성 118개 모두 exact version + package checksum 일치. 각 파일 checksum은 Cargo build 검증에 맡겼다. `cargo metadata --locked --offline` 123 packages, exit 0.
- Python 3.12.14; NumPy 2.3.5/SciPy 1.17.0/Matplotlib 3.10.8 import PASS. SymPy와 mpmath는 설치되어 있지 않다. 연구 lane의 Decimal/Fraction 독립 계산은 이와 별개다.
- RAM cap 8 GiB, CPU quota 8 CPU, affinity 9. 환경 preflight 때 cgroup OOM kill 0. 최초 archive 탐색은 기본 tar에서 1,771개 후 멈췄으나 `--ignore-zeros`로 51,085개를 보았다. 최소 components를 추출했지만 rustc_driver가 27,959,296 bytes로 잘려 loader exit 127이 발생했다. 해당 member를 `tar --ignore-zeros -xO`로 스트리밍 재추출한 뒤 compile/execute PASS. 원인은 archive/extraction 경로 이상으로 미해결이며, solver 실패로 분류하지 않는다. 전체 LLVM은 186,862,176 bytes이며 이전 제공 원본 member hash와 일치했다.
- 최초 임시 receipt writer는 rustc 실패 후 변수 초기화 누락으로 NameError가 발생했다. 수정된 `verify_setup.py`가 재실행 성공했고 최초 실패를 JSON에 보존했다.
- 배포 `.asc` 서명검증은 지정된 신뢰키가 없어 하지 않았다. archive SHA-256 identity만 기록했다. 패키지 network fetch, system install, production manifest/lock 수정은 없다.

기계 판독 근거: `setup_evidence.json`, `python_host_preflight.json`, `dependency_match.json`; 실제 stdout/exit: `setup_commands.log`.

전체 native campaign은 source 변경 대상 R2 regression binary를 먼저 실행하고, build 600초·binary 30/120초·전체 실행 750초로 제한한다. timeout은 `UNRESOLVED_BOUNDED_TIMEOUT`이며 assertion 실패도 PASS도 아니다. `--all-targets --all-features --no-run`의 native artifacts만 대상으로 하며 doctest는 NOT_RUN이다.

## 공통 build cache 복구

Time/arithmetic lane build가 `failed to map object file: memory map must have a non-zero length`로 exit 101을 냈다. 공유 target에서 zero-byte generated object 7개를 관측했다. 세 lane Cargo 작업을 중지하고 exclusive flock하에서 `cargo clean -p rodas5p-krylov` 및 관측된 잔여 zero-byte `.o` 제거만 수행했으며, `CARGO_INCREMENTAL=0`을 공통 환경에 추가했다. 이후 arithmetic standalone crate의 모든 bin build는 25.05초, exit 0이었다. solver source 오류가 아니라 생성 cache/실행환경 오류로 분류하며, zero-byte object가 생긴 근본 원인은 미해결이다. OOM kill counter는 전후 0이었다. 상세 증거는 `runtime_incidents.json`, `targeted_cache_recovery.json`, `cache_recovery_build.log`에 있다.

`CARGO_INCREMENTAL=0` 이후에도 time probe의 미사용 `audit2-research` feature variant에서 신규 integrators object 하나가 zero bytes로 생성되어 다시 map 실패했다. time probe에서 사용하지 않는 feature만 제거한 default public API 실행은 exit 0이었다. 이 feature를 전체 native campaign에서 빼지 않았으며, 전체 build는 dev/test `codegen-units=1`, `incremental=0`으로 code generation 동시성을 줄여 한 번 실행한다. 기본 opt-level 0과 debug info를 유지한다. 이는 성능 측정이 아닌 제한된 correctness campaign 설정이다. 모든 실패 로그를 보존하며 해당 근본 원인을 미해결로 표시한다.
