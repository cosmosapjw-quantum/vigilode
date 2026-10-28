# R4 현재 실행환경 복구

이 기록은 이번 작업에서 새로 실행한 결과다. R3의 통과 판정을 계승하지 않았다.

- 사용자 제공 Rust 1.94.1 archive에서 compiler, standard library, Cargo, rustfmt, clippy 실행에 필요한 `bin/`, `lib/`, `libexec/`를 streaming 추출했다. 각 파일은 archive member 크기와 일치함을 확인했다.
- 사용자 제공 `15-GENERIC_VECTOR_JF_OFFLINE_CARGO_VENDOR_20260816-1-.zip`의 내부 archive SHA-256은 동봉 checksum과 일치한다. 현재 `Cargo.lock`의 registry package 118개가 vendor의 exact version/package checksum과 전부 일치한다. vendor에는 총 262개 package가 있다.
- Rust archive의 SHA-256은 byte identity를 위한 기록이다. 별도의 trusted signing key를 확보하지 않았으므로 PGP 서명 검증 완료로 주장하지 않는다.
- `rustc --version --verbose`, `cargo --version`, `rustfmt --version`, 간단한 Rust 파일 compile 및 실행, `cargo metadata --locked --offline`은 실제 exit 0이다. 간단한 executable은 `RUST_EXECUTION_PASS 42`를 출력했다. metadata에는 workspace 5개를 포함한 package 123개가 있다.
- Python 3.12.14, NumPy 2.3.5, SciPy 1.17.0, Matplotlib 3.10.8 import 성공. mpmath/SymPy는 현재 설치되어 있지 않다.
- 실행환경은 cgroup RAM 8 GiB, CPU quota 8이다. host 전체 processor 수를 계산 자원으로 오인하지 않는다.

첫 vendor 추출은 tar가 archive의 uid/gid 1000 소유권을 복원하려다가 호스트의 `Invalid argument`로 실패했다. `--no-same-owner`를 명시한 재추출은 exit 0이다. 최초 stderr는 gzip으로 보존했고 setup receipt에 원래 크기와 SHA-256을 기록했다. 이 사건은 solver 또는 수학적 실패가 아닌 실행환경 사건이다.

모든 Cargo 호출은 `runtime_r4/cargo-build.lock`의 `flock`으로 직렬화한다. `CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=1`, dev/test `codegen-units=1`을 고정했다. R3에서 발생했던 zero-byte generated object와 incomplete live-redirection 실패는 이번 결과로 계승하지 않고 예방 설계에만 참고했다. 과학 테스트는 root가 사전등록 commit을 확인한 후 실행한다.

실행 명령과 출력은 `setup_evidence.json`, 파일 identity는 `extraction_receipt.json`, exact dependency 대응은 `dependency_match.json`을 참조한다. source 및 dependency lock 파일은 수정하지 않았다.
