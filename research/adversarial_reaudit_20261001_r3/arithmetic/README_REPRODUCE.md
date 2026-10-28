# 재현

production source 고정은 `EXECUTION_RECEIPT.json/source_files`의 byte identity로 검사한다. report-only 후속 commit에서도 해당 파일들이 동일하면 준비할 수 있다. production 파일이 수정된 버전을 평가하려면 별도 source-bound 재감사 실험을 만들어야 한다.

```bash
python3 prepare_probe.py --repo /absolute/path/to/vigilode --out /new/path/arithmetic-probe
cargo build --manifest-path /new/path/arithmetic-probe/Cargo.toml --bins --locked --offline --jobs 1
```

Rust/Cargo 1.94.1과 lock에 맞는 vendor/cache가 필요하다. 이번 실행은 프로젝트 소스에서 복구한 toolchain과 shared `CARGO_TARGET_DIR`를 사용했으며, 병렬 lane은 동일 cargo build lock을 사용했다. 기록된 첫 실패와 cache recovery는 보존되어 있다. 일반 재현자는 자기 Cargo target을 사용할 수 있다.

기존 raw evidence를 보존하려면 본 `arithmetic/` 폴더를 새 작업 위치에 복사한 뒤 실행한다. `run_native.py --target /absolute/cargo-target`는 그 복사본 옆에 세 binary 실행 결과를 새로 기록한다. 이어서 `python3 exact_oracle.py`, `python3 analyze_closures.py`를 실행한다. `prepare_probe.py`는 원래 `probe/src`와 `Cargo.lock`을 유지하고 dependency path만 재배치한다. 이번 세션에서는 preparation의 source-hash/path 검사도 실제 실행했지만, 재배치된 probe를 다시 compile하여 과학 실행을 중복하지 않았다.

`NATIVE_EXECUTION.json`과 `EXECUTION_RECEIPT.json`은 **이번 세션 기록**이다. 새 실행의 결과를 오래된 execution receipt로 승인하지 말고 새 source/probe/binary identity와 exit를 기록한다. 수치 결과를 원 R2 source gate를 우회한 결과로 해석하지 않는다. 새 R3 probe가 R3 source를 평가한 것이다.
