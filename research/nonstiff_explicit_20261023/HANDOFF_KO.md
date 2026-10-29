# 로컬 Codex 실행 인계서

아래 프롬프트를 새 Codex 세션에 복사해 사용한다. 보고서가 추가된 최신
commit과 실제 검증·측정 대상인 소스 commit을 구별한다. 실행 대상 소스는
`45037fc24e8f8101605f309e1fdb3a9a4e3df56d`로 고정한다. 현재 호스트에서
아직 실행하지 않은 비교의 결과나 외부 adapter의 가용성을 미리 가정하지
않는다. 실행 방법의 상세 설명과 완전한 Rust 사용 예시는
[`REPRODUCE.md`](REPRODUCE.md)에 있다.

```text
cosmosapjw-quantum/vigilode의 nonstiff 명시적 적분 경로를 로컬에서
검증하고 광범위하게 비교하라. 새 알고리즘 연구를 처음부터 반복하는
작업이 아니라, 고정 소스를 컴파일하고 준비된 시험·측정 하네스를
실행하여 재현 가능한 결과를 남기는 작업이다.

대상:
- 저장소: https://github.com/cosmosapjw-quantum/vigilode
- 게시 브랜치: research/nonstiff-explicit-20261023
- 고정 실행 소스: 45037fc24e8f8101605f309e1fdb3a9a4e3df56d
- 실행 문서: research/nonstiff_explicit_20261023/REPRODUCE.md
- 계획/프로토콜: 같은 폴더의 BENCHMARK_PLAN.md
- 새 보고서 commit이 고정 실행 소스의 자식이어도 실행 소스 라벨을
  최신 HEAD로 바꾸지 않는다. 위 고정 소스의 바이너리만 측정한다.
- REPRODUCE.md와 HANDOFF_KO.md는 고정 소스 이후의 보고서에 추가된
  문서다. 게시 보고서에서 먼저 읽거나 별도로 보관하라. 고정 소스의
  worktree에 이 두 문서가 없다는 이유로 중단하지 않는다. 실제 실행
  스크립트, 예제와 시험은 고정 소스에 포함되어 있다.

1. 원래 checkout을 보존하라.
   원래 HEAD, branch, status를 기록한다. reset, git clean, 강제 checkout,
   기존 파일 삭제를 하지 않는다. dirty 여부와 관계없이 mktemp로 만든
   새 디렉터리에 git worktree add --detach를 사용하여 고정 commit의
   clean checkout을 만든다. 해당 object가 없을 때만 게시 브랜치를 fetch한다.
   새 worktree의 HEAD와 tree를 저장하고, 모든 빌드·실행 결과는 그
   worktree 밖의 새로운 디렉터리에 쓴다. 기존 결과 디렉터리는 덮어쓰지 않는다.
   최초 checkout이 다른 브랜치라는 이유로 전체 작업을 중단하지 말고,
   위 격리 checkout을 만들어 계속한다.

2. NCP와 SSH를 사용하지 마라. 로컬 실행만 허용한다.
   Rust 1.94.1과 Cargo.lock을 사용한다. 이 연구 환경에서는 다음이 있다:
     /workspace/scratch/0dc1e721cf82/task_runtime/installed/bin
     /workspace/scratch/0dc1e721cf82/task_runtime/cargo
   실제로 존재할 때만 전자를 PATH, 후자를 CARGO_HOME에 사용한다.
   다른 호스트에 같은 cache가 있다고 가정하지 않는다.
   CARGO_TARGET_DIR는 새 실행 디렉터리 아래로, CARGO_BUILD_JOBS=2,
   CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0으로 설정한다.
   rustc/cargo 버전과 cargo metadata --locked --offline 결과를 보존한다.

   의존성이 부족하면 실패 로그를 보존하고, 승인된 네트워크에서
   cargo fetch --locked를 실행하거나 같은 lockfile에 맞는 offline
   cache/vendor를 준비한다. Cargo.lock 변경, 임의 dependency upgrade,
   --locked 제거로 우회하지 않는다. 의존성 준비 시간은 solver 측정과
   구분한다. 네트워크 또는 offline 공급원이 없으면 정확한 누락 의존성을
   보고하고 그 이후 시험은 NOT_RUN으로 남긴다.

3. 다음 25개 표적 테스트를 실행하고 로그와 exit code를 남겨라.
   cargo test --locked --offline -p rodas5p-integrators \
     --test dopri5_contracts --test dopri5_allocation
   cargo test --locked --offline -p vigilode \
     --test explicit_contracts --test explicit_ensemble_contracts
   대상은 kernel contract 12개, allocation 1개, public API 8개,
   ensemble 4개이다. 성공 횟수만으로 전체 저장소나 모든 production
   문제군이 검증되었다고 주장하지 않는다. 실패하면 먼저 원인을 조사한다.

4. 다음 두 예제를 함께 빌드하라.
   cargo build --locked --offline --profile measurement -p vigilode \
     --example nonstiff_bench --example nonstiff_ensemble_bench
   measurement는 opt-level=3, LTO=false, codegen-units=16이다.
   바이너리 SHA256, 컴파일러, source/tree, CPU 모델, 허용 affinity,
   CPU quota, 메모리 한도, Python/NumPy/Matplotlib 버전을 남긴다.
   이미 설정된 CARGO_TARGET_DIR와 --binary 경로를 일치시킨다.

5. 외부 비교는 VODE/DVODE, diffsol, Julia ODE solver로 한정하라.
   내부 비교는 새 DOPRI5와 기존 RODAS5P dense/matrix-free 경로이다.
   SciPy가 설치되어 있으면 bundled vode_adapter.py를 사용한다.
   이것은 Fortran VODE/DVODE의 Adams/BDF 경로이지 SciPy RK45나
   Radau/LSODA 대체 비교가 아니다. Python callback overhead와 VODE
   출력 보간 차이를 보고서에 명시한다.
   diffsol/Julia adapter는 이 소스에 포함되어 있지 않다. 이미 준비된
   local executable이 있을 때만 BENCHMARK_PLAN.md의 JSON stdin/stdout
   규약과 saved state 검증을 확인한 후 연결한다. 없으면 NOT_RUN이다.
   정확한 solver method, 버전/commit, adapter source·binary SHA256,
   Python 환경 lock 또는 Cargo.lock 또는 Julia Project/Manifest의
   사본과 hash를 EXTERNAL_ADAPTERS.json에 기록하라. Julia의 JIT/warmup
   범위도 고정하고 밝힌다. 설치 여부나 최신 버전을 추측하지 않는다.

6. 다음 확장 비교를 새 output 디렉터리에 실행한 뒤 분석하라.
   변수는 REPRODUCE.md처럼 실제 값으로 설정한다. 필요하면 검증된
   --diffsol-adapter / --julia-adapter 인자를 추가한다.

   python3 research/nonstiff_explicit_20261023/run_bench.py \
     --binary "$CARGO_TARGET_DIR/measurement/examples/nonstiff_bench" \
     --out "$VIGILODE_RUN_ROOT/full-comparison" \
     --source-commit "$VIGILODE_SOURCE" \
     --external --full --budget-seconds 7200
   python3 research/nonstiff_explicit_20261023/analyze_bench.py \
     "$VIGILODE_RUN_ROOT/full-comparison"

   비교 대상은 decay, 비자율 forcing, logistic 비선형성, harmonic,
   rapid oscillation, damping, mixed scales, nonnormal transient,
   4096/16384차원 vector 및 moderate/stiff negative control이다.
   4개 허용오차, 동일 관측시각, 정확해, warmup 1회, timed 3회를 사용한다.
   7200초는 비교 캠페인 예산이다. 사전 빌드와 뒤의 ensemble 측정까지
   포함한 전체 작업 시간이 무조건 2시간 이내라고 주장하지 않는다.
   timeout, budget exhaustion, solver failure, adapter 부재를 모두 남긴다.
   하네스의 45초 per-process 한도를 임의로 해제하지 않는다.

7. 독립 궤적 병렬처리를 별도의 180초 예산으로 측정하라.
   python3 research/nonstiff_explicit_20261023/run_ensemble_bench.py \
     --binary "$CARGO_TARGET_DIR/measurement/examples/nonstiff_ensemble_bench" \
     --out "$VIGILODE_RUN_ROOT/ensemble-comparison" \
     --source-commit "$VIGILODE_SOURCE" --budget-seconds 180
   16개 독립 4096차원 문제를 1/2 worker에서 비교하며 각 문제의
   적응형 시간간격과 오차 제어는 독립적이다. worker 수 사이의 saved
   state 일치를 확인한다. 이것을 단일 궤적 RK stage 병렬화나 homotopy
   가속의 증거로 해석하지 않는다.

8. 수치적 성공과 속도를 분리하여 보고하라.
   process exit 0만으로 정확하다고 판정하지 않는다. SUMMARY.json의
   saved state 재검증과 동일 achieved-error 조건을 만족한 데이터만
   비용비에 포함한다. rtol 자체를 전역오차라고 부르지 않는다.
   min/max 반복 범위는 신뢰구간이 아니다. endpoint/5개 관측점 오차는
   모든 시간에 대한 엄밀한 오차보증이 아니다. 누락 비교를 임의 수치로
   채우거나 실패한 configuration을 삭제하지 않는다.
   원시 stdout/stderr, PLAN, RECEIPTS, SUMMARY, REPLAY, 도표,
   RAW_OUTPUTS.tar.gz, RAW_MANIFEST.json과 환경 identity를 보존한다.
   각 문제군의 사용 가능 범위, 성능 이득/손실, 정확도 실패 및 다음
   개선 항목을 한국어 보고서와 machine-readable JSON으로 정리하라.

9. 공개 API 범위를 정확히 유지하라.
   Dopri5는 사용자가 직접 선택하는 forward identity-mass ODE 경로다.
   비자율 RHS, scalar/vector atol을 지원하고 Jacobian/JVP/dfdt,
   matrix factorization/Newton solve를 호출하지 않는다. mass matrix
   선언, event, dense output은 RHS 실행 전에 거절한다. 관측점은 실제
   step을 잘라 도달한다. 자동 stiffness 탐지·method switching은 없다.
   Jacobian이 없다는 이유로 nonstiff라고 판정하지 않는다. FSAL을 위해
   한 solve 중 RHS의 수학적 모델은 동일해야 한다.
   partial solution의 success=false와 마지막 accepted state를 보존하고,
   호출 예제에서는 ensure_success() 또는 동등한 명시적 검사를 사용한다.

10. 종료 보고에서 원래 checkout이 보존되었는지와 새 worktree의
    source/tree를 확인하라. 이미 게시된 연구를 일괄 재검증하지 말고
    새 실행의 결과와 실제 변경 영향만 다룬다. 수정이 꼭 필요하면
    최소 수정과 그 원인을 분리하여 기록하고 새 commit을 고정한 뒤
    영향을 받는 시험·측정만 새 output 디렉터리에서 다시 실행한다.
    수정된 binary에 기존 고정 commit 라벨을 붙이지 않는다.
    근거가 부족한 범용 production 판정을 내리지 말고 정확한 적용
    범위와 남은 검증을 제시하라.
```

전체 사용 예시는 `REPRODUCE.md`의 `Problem::new` closure가 모든
미분 성분을 채우고 `Ok(())`를 반환하며, `Dopri5::solve_observed(...)?`
뒤에 `ensure_success()?`를 호출하는 형태다. 간단한 성공 반환과 부분
실패 반환을 혼동하지 않는 것이 로컬 검증의 핵심이다.

전체 벤치마크 예산을 2시간으로 나눌 때에는 사전 준비 후 main 6900초, ensemble 180초, 분석 여유 120초로 설정한다. 두 실행기의 개별 제한을 전체 실행 시간 보증으로 바꾸어 말하지 않는다.
