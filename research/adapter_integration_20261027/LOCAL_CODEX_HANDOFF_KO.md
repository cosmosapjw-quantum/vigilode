# 로컬 Codex 실행·분석 인계

아래 프롬프트를 그대로 전달한다. NCP는 사용하지 않는다. 사용자의 실행 중인 계산과 기존 checkout은 보존한다.

```text
cosmosapjw-quantum/vigilode의 research/adapter-integration-20261027 결과를 이어받아라.
전달 메시지의 게시 commit을 EXPECTED_COMMIT으로 고정한다. 이 값이 없으면 원격 branch를 한 번 fetch하여 그때의 SHA를 기록하고, 이후 단계는 그 고정 SHA에서 실행한다. 원래 작업 트리를 reset하거나 checkout 변경하지 말고 별도 detached worktree를 만든다.

먼저 research/adapter_integration_20261027/REPORT_KO.md, VALIDATION.json, DECISIONS.json, NEXT_STEPS.json, benchmark/INTERPRETATION_R2.md를 읽어라. 이번 패키지는 최신 성능 계열 e8572eac와 joint 계열9b062e8의 통합이다. 이미 이식된 callback 제한·implicit 구간 병합·후속 warm start를 다시 구현하지 마라.

무결성을 확인한 뒤 Rust1.94.1, 고정 Cargo.lock으로 컴파일·검증한다. offline cache가 부족하면 실패 원인을 기록하고 승인된 온라인 fetch로 정확한 lock을 준비할 수 있다. Cargo.lock을 갱신하거나 별도 버전으로 바꾸어 통과시키지 마라.

루틴 빌드, 로그 파싱, 재현은 사용 가능한 6 luna low 등 저예산 모드로 수행한다. 아직 해소되지 않은 수학적 가정이나 실제 원자료의 정밀 원인 분석에만 astra ultra와 관련 physmath-research-loop/SciSpace/Wolfram 스킬·플러그인을 사용한다. 사용 가능한 모델을 실제로 확인하며 라우팅이 실행되지 않았는데 실행되었다고 기록하지 마라.

이번 최종 검증은263개 테스트,26개 짝 비교다. 기본 gyromotion의 정확도 실패, 완화의12.2%시간 회귀, 시간 변동으로 미판정된 전이 결과를 보존한다. phase와 중간 시간의 정확도를 종료점 결과로 대체하지 마라. 반복 탐색 감소 자체를 성능으로 승격하지 말고 전체 비용을 확인한다.

첫 현장 확인은 adapter가 선택한 방법과 유력 대안 하나의 같은 checkpoint·같은 구간 비교다. 사용자의 기존 raw 자료를 입력으로 실제 h 제안/시도/착지절단/오차/거절 이유, RHS/JVP/Jacobian/선형풀이/할당 비용을 대응시켜라. 순차·병렬·모든 method의 무차별 조합 실행은 하지 않는다.

현재 adaptive 옵션은 변화하는 강성과 초기 무전진 비교에 효과가 있지만, 안정 회전·감쇠의 직접 경로보다 항상 빠르지 않다. case5의 dense 직접 비교는 JVP 경로보다 빠르다. 그러나 원래 요청에 dense provider가 없으면 이를 선택기가 쓸 수 있었다고 가정하지 마라. 주어진 derivative/matrix/resource 권한을 먼저 확인한다.

우선 조사할 미해결 사항은 (1) 과거 implicit 작업률이 현재 explicit 승리를 누락시키는 경우, (2) 완전한 RODAS session 보존의 증분 이득, (3) 실제 provider별 비용 보정이다. NEXT_STEPS.json의 파일·수학 조건·완료 기준을 따른다. 이번에 이미 교정된 native 관측절단 복원을 다시 미구현 버그로 보고하지 마라.

코드를 바꾸면 좁은 의미 있는 회귀검증 뒤 해당 영향을 받는 짝 비교만 재실행한다. 실패와 원자료를 덮어쓰지 말고 새 디렉터리를 사용한다. 최종 반환에는 commit/tree, source SHA256, 호스트·프로파일·스레드, 명령·exit, 전체 시간/작업량/오차, 선택 또는 누락의 이유와 다음 수정안을 포함한다. 수정 push는 새 작업 브랜치에서 non-force로 수행하고 기준 SHA를 재확인한다.
```

## 실행 명령

기존 저장소에서 목표 SHA를 고정하고 별도 worktree를 만든다. 아래 `EXPECTED_COMMIT`이 이미 전달되었다면 fetch한 SHA와 비교한다.

```bash
set -euo pipefail
git fetch origin research/adapter-integration-20261027
AUDIT_SOURCE=$(git rev-parse FETCH_HEAD)
if [ -n "${EXPECTED_COMMIT:-}" ]; then
  git cat-file -e "$EXPECTED_COMMIT^{commit}" 2>/dev/null || git fetch origin "$EXPECTED_COMMIT"
  AUDIT_SOURCE=$(git rev-parse "$EXPECTED_COMMIT^{commit}")
fi
AUDIT_WORK=$(mktemp -d "${TMPDIR:-/tmp}/vigilode-integration-XXXXXXXX")
git worktree add --detach "$AUDIT_WORK/checkout" "$AUDIT_SOURCE"
cd "$AUDIT_WORK/checkout"
python3 research/adapter_integration_20261027/tools/verify_package.py
```

적절한 Rust 환경에서 다음을 실행한다. 현재 다른 benchmark가 돌아가면 먼저 이 계산과 CPU·메모리 사용이 겹치지 않는 실행 시간을 확보한다. 이 명령은 기존 프로세스를 종료하지 않는다.

```bash
export CARGO_BUILD_JOBS=2
export CARGO_INCREMENTAL=0
export CARGO_PROFILE_TEST_DEBUG=0
export CARGO_PROFILE_DEV_DEBUG=0
export RAYON_NUM_THREADS=1
export OMP_NUM_THREADS=1
export OPENBLAS_NUM_THREADS=1
rustc --version
cargo metadata --locked --offline > "$AUDIT_WORK/metadata.json"
python3 research/joint_controller_20261025/tools/run_recorded.py \
  --output "$AUDIT_WORK/validation" --timeout 600 -- \
  cargo test --locked --offline -p vigilode --lib --tests
python3 research/joint_controller_20261025/tools/run_recorded.py \
  --output "$AUDIT_WORK/build" --timeout 900 -- \
  cargo build --profile measurement --locked --offline -p vigilode --example adapter_integration_pilot
AUDIT_TARGET=${CARGO_TARGET_DIR:-$PWD/target}
"$AUDIT_TARGET/measurement/examples/adapter_integration_pilot" rhs-only-availability \
  > "$AUDIT_WORK/availability.json"
python3 research/joint_controller_20261025/tools/run_recorded.py \
  --output "$AUDIT_WORK/pair-receipt" --timeout 900 -- \
  python3 research/adapter_integration_20261027/benchmark/run_pairs.py \
  "$AUDIT_TARGET/measurement/examples/adapter_integration_pilot" "$AUDIT_WORK/raw"
python3 research/adapter_integration_20261027/benchmark/analyze_pairs.py \
  "$AUDIT_WORK/raw" "$AUDIT_WORK/analysis"
```

명령 하나라도 실패하면 그 이후 성공을 가정하지 말고 해당 receipt를 먼저 읽는다. 원자료가 남은 위치와 Git identity를 반환한다. Python 그림 생성에는 matplotlib와 numpy가 필요하다. 이번 공유 환경의 CPU는 Xeon이며 사용자의 Ryzen 환경에서의 시간 개선을 대신하지 않는다.
