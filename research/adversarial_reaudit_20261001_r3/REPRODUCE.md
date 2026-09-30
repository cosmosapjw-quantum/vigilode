# R3 재현

`reproduce.py`는 보고서·원증거를 덮어쓰지 않는다. 과학 실행에는 **기존에 없는 외부 `--out` 경로**가 필요하고, 실행 명령·exit·로그는 그 경로에 저장된다. `--prepare-only`는 경로·소스 결속과 준비만 검사하며 과학 검산이나 빌드 성공을 의미하지 않는다. 준비한 폴더를 실행에 재사용하지 말고 새 경로를 지정한다.

```bash
# 보고서 폴더 안에서. root validator가 제공된 완성 패키지에 적용한다.
python3 reproduce.py --validate

# Python 3.12, NumPy 2.3.5, SciPy 1.17.0에서 기록된 연구를 재현한다.
python3 reproduce.py --research --independent-checks --out /tmp/vigilode-r3-research-new

# Rust 1.94.1 / Cargo와 offline vendor가 이미 구성된 환경을 사용한다.
# report가 repo 내부에 있으면 --repo를 생략할 수 있다.
python3 reproduce.py --native all --repo /path/to/vigilode --out /tmp/vigilode-r3-native-new

# 빌드 없이 native source binding·manifest relocation만 확인한다.
python3 reproduce.py --native all --repo /path/to/vigilode --out /tmp/vigilode-r3-prepare-new --prepare-only
```

`--native time|statistics|arithmetic`으로 한 lane만 실행할 수 있다. Cargo 작업은 순차이며 `--locked --offline --jobs 1`, 별도 output target, `CARGO_INCREMENTAL=0`을 사용한다. 다른 세션과 빌드를 직렬화하려면 동일한 기존 lock 경로를 `--lock-file /path/to/cargo-build.lock`로 준다. 이 entrypoint는 runtime 설치나 전체 workspace suite를 실행하지 않는다.

Native 모드는 time/statistics의 source hash 계약을 합쳐 검사한다. 보고서만 추가한 후속 commit은 생산 소스 bytes가 같으면 허용한다. 달라진 생산 소스를 기존 감사 결과로 승인하지 않고 source mismatch로 중단한다. Time/statistics는 기존 lane wrapper로 준비하고 해당 probe를 실행한다. Arithmetic은 전체 lane을 새 출력 위치로 복사한 뒤 native 결과 파일을 비우고 manifest dependency 경로를 바꿔 실행한다. 원본 arithmetic 디렉터리에 쓰지 않는다.

Research 모드는 Laguerre/Chebyshev 공동 φ 계산, coefficient 구조 검사, **strict-lower projection**에 대한 homotopy majorant, bootstrap MC 후보를 실행한다. `--independent-checks`는 당시 reviewer가 선택한 두 별도 Python 검산을 재실행하는 옵션이며 새 독립 심사를 수행했다는 의미는 아니다. 캡처된 tableau coefficient의 SHA-256을 확인한다. 현재 checkout의 source까지 확인하려면 research에도 `--repo`를 지정한다.

Homotopy를 exact native full-block parity, Python prototype을 production backend, 계산 성공을 실제 wall speedup, MC bound를 모집단 CI coverage로 해석하지 않는다. `REPRODUCTION_RECEIPT.json`의 `COMPLETED`는 실행 exit가 0이었다는 뜻이다. 실패·불일치·미실행은 원 감사의 결과와 분리해서 판단한다.

현재 게시본의 wrapper는 별도 외부 폴더에서 research 및 reviewer-selected 두 검산을 실제 재실행하여 6 commands 모두 exit 0을 확인했다. 원자료는 덮어쓰지 않았다. 증거는 `evidence/portable_research/REPRODUCTION_RECEIPT.json`에 있다. Native wrapper 자체는 prepare-only를 확인했으며 focused native 및 independent native 실행 증거는 각 lane/decision에 별도로 있다.
