# Local Codex 인계: 적응 제어·구조 후보·실행 선택의 실제 손실 줄이기

아래 지시를 local Codex에 그대로 전달할 수 있다. 게시 커밋 번호를 이미 받았다면 원격 branch 해석 결과와 그 번호가 같은지 먼저 확인한다. 아래에서는 아직 게시되지 않은 로컬 commit 번호를 최종 소스인 것처럼 넣지 않고, 원격 branch의 한 번 해석한 commit/tree를 고정한다.

```text
대상 저장소: https://github.com/cosmosapjw-quantum/vigilode
대상 브랜치: research/joint-controller-20261025
목표: 현재 연구 패치를 사용자 컴퓨터의 실제 원자료와 연결하여, adapter/switcher가 잘못 선택하거나 불필요한 탐색·전환 비용을 지불하는 원인을 정확히 구별하고 최소 수정한다. 장기 benchmark 재개보다 동일 checkpoint의 선택 경로와 유력 대안 하나를 우선한다.

1. 작업 원본과 source identity
- 현재 checkout, 브랜치, HEAD, dirty 파일, 기존 측정·실패·중지 결과를 보존한다. reset, clean, stash, 기존 checkout 전환을 하지 않는다.
- 대상 원격 브랜치를 한 번 fetch하여 commit SHA와 tree SHA를 기록하고, 그 commit의 별도 detached worktree에서 시작한다. 로컬 복원 commit 040b676…나 과거 측정 source를 이번 게시 SHA로 쓰지 않는다.
- fetch 이후 원격 브랜치가 바뀌더라도 한 campaign 도중 새 코드를 자동으로 섞지 않는다. 별도 코드 수정은 새 branch/commit과 source manifest로 구분한다.
- NCP, SSH 원격 실행, 전체 method×thread sweep, 이전에 중지한 장기 campaign 재개는 하지 않는다.

2. 먼저 읽을 자료
- research/joint_controller_20261025/REPORT_KO.md
- 같은 폴더의 VALIDATION.json, NEXT_STEPS.json, DECISIONS.json, theory/DERIVATIONS_KO.md, history/REUSE_MATRIX.json
- switching/IMPLEMENTATION.json, execution/IMPLEMENTATION.json, causal/ 자료와 review/의 최종 지적·종결 근거
- evidence/NATIVE_PILOT_SUMMARY_FINAL.json 및 관련 실제 실행 receipt/source/binary identity
- external_reports/adapter_local_followup_20261025/REPORT_KO.md, DECISIONS.json, RAW_MANIFEST_FINAL.json
- research/adapter_research_20261025/selection_benchmark/PROTOCOL_KO.md

초기 case5 transient 이후 endpoint의 새 국면과 방금 지난 prefix의 비교 비용을 같은 위치의 근거로 취급했던 수정 이력도 확인한다. 최종 정책은 완화 신호 이후의 다음 prefix에서 비교한다. 최초 실패 실험과 수정 뒤 실험을 분리하고, 최종 결과 폴더를 VALIDATION/통합 요약에서 해석한다. 이전 onsets 0.47/0.91은 재사용된 회귀 대조군이며, 이후 추가된 0.43/0.87을 이미 튜닝에 사용했으면 다시 미사용 holdout이라고 부르지 않는다.

3. 실제 로컬 원자료
- 우선 /home/cosmosapjw/vigilode-local-archive/n01-vigilode-local-run-20261024.geLVCp9Q, /tmp/vigilode-local-run-20261024.geLVCp9Q, /tmp/vigilode-adapter-local-followup.1PRDjIEs의 존재와 이전 manifest를 읽기 전용으로 확인한다. 실제 존재하는 위치를 사용하고 경로가 없으면 missing으로 남긴다.
- 공개된 소형 packet은 원자료의 위치와 해시를 제공한다. 그 packet만 읽고 대용량 원자료 바이트까지 검증했다고 말하지 않는다.
- 새 출력은 기존 파일과 겹치지 않는 디렉터리에 쓴다. 실패, timeout, inaccurate 후보, 불완전 trajectory는 제거하거나 성공으로 바꾸지 않는다. 필요한 백업은 공간을 확인한 뒤 create-only로 수행한다.

4. 라우팅과 예산
- 고정 형식 파싱, 이미 정해진 시험 실행, 로그/해시 정리, 보고서 갱신은 Luna low 또는 실제 사용 가능한 저비용 모드로 한다. 일반 코드 수정은 적절한 workhorse low/medium을 사용한다.
- 새로운 증명, 기존 수식과 원래 ODE의 불일치, 해석되지 않은 전이 기작처럼 명확한 연구 문제가 있을 때만 Astra ultra와 physmath/SciSpace/Exa/Wolfram을 사용한다. 해당 문제와 종료 조건을 먼저 짧게 적는다.
- 실제 runtime에서 없는 모델을 사용했다고 쓰거나 외부 API 계정/키를 만들지 않는다. 반복적인 이미 완료된 감사와 문헌 재검색을 하지 않는다.
- 첫 진단은 30분 이내, 새 수치 pilot 전체는 10분 이내를 목표로 제한한다. 개별 binary 30초, build/test는 아래 명령의 timeout을 적용한다. 새로운 원인을 얻지 못한 동일 실패 반복은 중단하고 원인을 기록한다. 새 원인이 확인된 수정·재시험은 수행한다.

5. 검증과 좁은 개발 순서
- 먼저 변경된 소스의 계약 시험과 짧은 측정을 재현한다. 정확도가 나쁜 후보는 속도 비교에서 분리한다.
- NEXT_STEPS의 J01(지속 implicit 세션), J02(causal prefix 재사용), J03(구조 provider와 물리 오차), J04(탐색 예산/실측 비용) 중 실제 원자료가 지목하는 병목부터 구현한다. 모든 과제를 한 번에 대규모 refactor로 바꾸지 않는다.
- 각 선택 관측에는 같은 source/build/host, 원래 물리 checkpoint, 구간, tolerance와 norm, chart/provider/mesh epoch, 출력 조건을 붙인다. 선택 경로와 대안 하나의 setup/probe/transition/검증/실행 비용을 한 번씩 합산한다.
- 작고 정확한 explicit 해가 가능한 문제가 단지 작은 h를 쓴다는 이유로 stiff라고 분류하지 않는다. requested/trial/proposed h, 수락/거절, error norm, clipping/domain, RHS/JVP/factor/solve 횟수, 실제 방법, discarded probe를 구별한다.
- chirp/gyro는 endpoint뿐 아니라 원래 전체 상태의 시간 격자 오차, 진폭과 위상 오차, 참조 불확실도를 함께 본다. 변환 좌표에서 작아진 오차만으로 원래 물리 정확도를 승인하지 않는다.
- 같은-prefix 비용이 적다는 것과 미래 방법 전환이 유리하다는 것을 구별한다. backward/forward 전환과 반복 probe의 비용을 분리한다. 구조상 부적격, 경제성이 나쁜 실제 전환, 불필요한 탐색, 전체 실행 초과 비용을 서로 다른 결과로 남긴다.
- 새 execution adapter의 비용 변화 검출은 실행 계약을 보강했지만 이번 세 단계에서는 calibration 포함 총비용이 가장 빠른 직접 backend보다 모두 컸다. 단순히 관측 폐기가 작동했다는 이유로 속도 향상으로 기록하지 않는다. 재보정 전에 잔여 호출 수에서 비용 회수가 가능한지, 보수적 관측 범위가 유리한 병렬 경로를 놓치는지 조사한다.
- causal_budget의 stage 오차 상한은 원래 λ=1 목표 잔차와 유효 inverse/Lipschitz 상한에 의존한다. embedded 추정치를 자동으로 전역 엄밀 오차 상한으로 바꾸지 않는다. stage 병렬 pilot의 round 에뮬레이션 결과를 실제 병렬 wall-time 이득으로 사용하지 않는다.
- 완전 무미분 경로를 먼저 적용하되, 구조 전제가 실제로 충족되고 원래 출력 정확도를 만족해야 한다. JVP/AD 또는 dense fallback은 명시적인 권한과 관측된 필요성을 유지한다.

6. 추가 벤치마킹
- 우선 1 thread로 선택기 오버헤드를 분리한다. 실제 큰 callback의 병렬화는 직렬과 유력한 분할 하나를 비교한다. 튜닝용 구간과 보지 않은 전이 위치·문제의 평가 구간을 구분한다.
- 허용되는 외부 비교 대상은 VODE/DVODE, Diffsol, Julia이다. 외부 toolchain 준비가 안 되면 기존 확인된 결과를 재사용하고 설치/참조 누락을 새로운 solver 실패로 취급하지 않는다.
- 더 좋은 대안이 나오면 ranking으로 끝내지 않고 구조 적격성, 변화 감지, 비용 추정, probe 예산, 전환/초기화 비용 중 무엇이 원인인지 진단한다.

7. 반환 및 게시
- source commit/tree, 변경 diff, compiler/lockfile/binary hash, 실제 명령, stdout/stderr/exit/timeout, 원자료 경로/해시, 정확도와 총비용, 선택 이유와 대안 비교를 반환한다.
- human REPORT와 machine-readable findings/next steps를 갱신한다. 비교 불가/실패/미실행은 서로 구분한다.
- 의미 있는 수정과 해당 검증이 완료되면 새 local 후속 브랜치에 push하고 branch/commit/tree를 검증한다. 기존 공동 연구 브랜치의 원격 tip을 force-push하지 않는다. scope가 검증된 기능만 권고하며 범용 production 승격은 남은 정확도·강건성·통합 조건에 따라 판정한다.
```

## 안전한 별도 checkout과 실행 명령

아래 명령은 현재 위치가 기존 저장소 안에 있는 경우의 예다. source/receipt/target은 새 디렉터리에 둔다. 기존 캐시와 Rust 1.94.1을 먼저 사용한다.

```bash
set -euo pipefail
joint_repo_root="$(git rev-parse --show-toplevel)"
git -C "$joint_repo_root" status --short --branch
git -C "$joint_repo_root" fetch origin refs/heads/research/joint-controller-20261025
joint_source_commit="$(git -C "$joint_repo_root" rev-parse FETCH_HEAD)"
joint_source_tree="$(git -C "$joint_repo_root" rev-parse "$joint_source_commit^{tree}")"
joint_run_root="$(mktemp -d "${TMPDIR:-/tmp}/vigilode-joint-local.XXXXXXXX")"
git -C "$joint_repo_root" worktree add --detach "$joint_run_root/checkout" "$joint_source_commit"
printf '%s\n' "$joint_source_commit" > "$joint_run_root/EXPECTED_COMMIT.txt"
printf '%s\n' "$joint_source_tree" > "$joint_run_root/EXPECTED_TREE.txt"
cd "$joint_run_root/checkout"
git status --short --branch
rustc --version
cargo --version
export CARGO_TARGET_DIR="$joint_run_root/target"
export CARGO_BUILD_JOBS=2
export CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
export OMP_NUM_THREADS=1
export OPENBLAS_NUM_THREADS=1
export MKL_NUM_THREADS=1
joint_recorder="research/joint_controller_20261025/tools/run_recorded.py"
python3 "$joint_recorder" --output "$joint_run_root/receipts/metadata_offline" --timeout 120 -- \
  cargo metadata --locked --offline --format-version 1
```

offline metadata가 cache 누락으로 실패하면 실패 receipt를 유지한다. 기존 승인된 인터넷 연결과 Cargo 설정으로 접근 가능한 경우에만 `cargo fetch --locked`를 별도 receipt에 기록해 한 번 실행하고, 새 `metadata_offline_retry` 디렉터리에서 검사를 반복한다. `Cargo.lock`을 바꾸거나 offline 실패를 solver 오류로 분류하지 않는다. 네트워크·인증·패키지 원본이 없으면 정확한 누락 항목을 기록하고 원자료 분석을 계속한다.

```bash
python3 "$joint_recorder" --output "$joint_run_root/receipts/facade_tests" --timeout 300 -- \
  cargo test --locked --offline -p vigilode --tests
python3 "$joint_recorder" --output "$joint_run_root/receipts/measurement_build" --timeout 600 -- \
  cargo build --locked --offline --profile measurement -p vigilode \
  --example adaptive_entry_pilot --example joint_execution_pilot --example causal_budget_pilot
```

시험과 build가 끝나고 다른 timed process가 없는 상태에서만 다음 pilot을 실행한다. CPU 번호는 현재 host의 허용 affinity에서 실제로 확인한 번호를 쓴다. `CPU_A`에는 그 번호 하나, `CPU_PAIR`에는 실제 사용 가능한 두 CPU 번호를 쉼표로 연결해 넣는다. 한 CPU만 가능하면 execution pilot은 worker 1로 실행하고 해당 범위로 기록한다.

```bash
python3 research/joint_controller_20261025/tools/run_switch_pilot.py \
  --binary "$CARGO_TARGET_DIR/measurement/examples/adaptive_entry_pilot" \
  --output "$joint_run_root/switch-pilot" --repeats 3 --cpu "$CPU_A"
python3 "$joint_recorder" --output "$joint_run_root/receipts/execution-pilot" --timeout 60 -- \
  taskset -c "$CPU_PAIR" "$CARGO_TARGET_DIR/measurement/examples/joint_execution_pilot" 2
python3 "$joint_recorder" --output "$joint_run_root/receipts/causal-pilot" --timeout 60 -- \
  taskset -c "$CPU_A" "$CARGO_TARGET_DIR/measurement/examples/causal_budget_pilot"
```

이 초기 스크립트는 새 정책, 직전 정책, 유력한 직접 방법을 비교하는 제한된 회귀 연구다. 이후 정밀 조사에서는 실제 선택과 같은 checkpoint 대안 하나를 중심으로 줄인다. 기존 증거가 유지되는 같은 실험을 새 성과를 위해 반복하지 않는다. 소스 수정 후에는 변경된 의존성에 해당하는 계약·holdout만 추가 검증하고, 새 source/binary manifest로 결과를 분리한다.
