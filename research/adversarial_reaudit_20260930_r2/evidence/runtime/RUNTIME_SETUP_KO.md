# 현재 재감사 런타임 복원 결과

고정 소스는 `7708ef90554fc3986478d4602de6a01c7266b14f`다. 이전 대화의 런타임 PASS를 상속하지 않고, 이번 실행에서 사용자 첨부 Rust 1.94.1 배포 아카이브를 설치하고 `rustc`로 작은 프로그램을 컴파일하여 `RUST_EXECUTION_PASS 42`를 직접 실행했다. `cargo metadata --locked --offline`는 123개 패키지를 읽었다.

- 환경 파일: `/workspace/scratch/0dc1e721cf82/runtime_r2/env.sh`.
- 사용자 제공 vendor의 262개 패키지 중 원본 `Cargo.lock`이 요구하는 registry 의존성 118개 모두 버전과 package checksum이 일치했다. 개별 파일 checksum은 실제 Cargo 컴파일 검증에 맡긴다.
- `Cargo.lock` SHA-256: `04c7d6c147bf19ebb1a627463705d9dcf27337a3ff228dd1169f19eb73c71400`.
- Rust/Cargo 1.94.1; rustc commit `e408947bfd200af42db322daf0fadfe7e26d3bd1`.
- Python 3.12.14, NumPy 2.3.5, SciPy 1.17.0, Matplotlib 3.10.8은 이번 import 검사에 성공했다. SymPy는 설치되어 있지 않다.
- 메모리 상한 8 GiB, CPU quota 8 CPU 상당량, CPU affinity 9. 여러 native lane은 `--jobs 1`과 별도 target directory를 사용한다.
- 의존성 네트워크 fetch, system package install, 원본 manifest/lock 변경은 없다. 제공된 `.asc`에 대한 서명 검증은 신뢰키가 지정되지 않아 수행하지 않았으며 archive SHA-256 identity를 기록했다.

이번 최초 압축해제에서도 LLVM 파일이 102,630,400 bytes로 잘렸다. 원본 archive member를 `tar -xO`로 직접 스트리밍하여 복원했고 설치된 파일은 186,862,176 bytes, SHA-256 `158c711c64147bb127a2a5174df22718d26b755560a1487945e7c788c947986f`였다. 최초 vendor tar는 컨테이너가 uid/gid 1000의 chown을 허용하지 않아 metadata 오류를 냈고 `--no-same-owner`를 적용한 복원은 exit 0이었다. 두 사례는 복원된 환경/추출 오류이며 solver assertion 실패가 아니다. 최초 LLVM truncation의 근본 원인은 미해결이다.

제공된 `03-rust_1_94_1_env.sh`와 `09-bootstrap.sh`는 읽었다. 전자는 사라진 `/mnt/data` 경로를 현재 독립 runtime 경로로 변경하여 사용했고, 후자는 Bianchi 패키지 대상 pip 설치와 소스 변경을 수행하므로 이 vigilode 감사에서 실행하지 않았다.

기계 판독 근거는 `setup_evidence.json`, `dependency_match.json`; 실제 컴파일/실행 stdout와 exit는 `setup_commands.log`에 있다. 향후 테스트 campaign은 `run_bounded_suite.py`가 build 1회 후 각 test binary를 bounded 실행하며, 시간 초과와 ignored test를 PASS로 합산하지 않는다.
