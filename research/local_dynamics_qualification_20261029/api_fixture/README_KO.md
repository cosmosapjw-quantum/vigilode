# Downstream Q-API 재현 fixture

이 디렉터리는 downstream source compatibility를 두 개의 독립 소비자 프로그램으로 확인한다.

- `legacy_config_literal.rs`: pre-conversation `SwitchingConfig`의 모든 필드를 나열하는 완전한 struct literal
- `legacy_exhaustive_evidence_match.rs`: pre-conversation `SwitchingEvidence`의 모든 variant를 처리하는 exhaustive match

고정 baseline은 `3064d453091af1dec7f99f0c8afabd98b447ea34`이다. fixture의 필드와 variant 목록은 이 commit의 `crates/vigilode/src/switching.rs`에서 확인했다. `SwitchingConfig` baseline은 `Serialize`만 derive하므로 legacy JSON `Deserialize` 비교는 `NOT_APPLICABLE`이다.

실행은 현재 코드가 commit되어 validation remote branch에서 읽힌 뒤에 한다. cargo 결과는 각 baseline/current × fixture 조합별 stdout, stderr, exit 파일에 보존한다. output 경로는 새 경로여야 하며 checkout 밖에 둔다.

```bash
VIGILODE_EXEC_SHA=$(git rev-parse HEAD)
VIGILODE_BRANCH=validation/local-dynamics-YYYYMMDDTHHMMSSZ
VIGILODE_EVIDENCE=/tmp/vigilode-api-fixture-YYYYMMDDTHHMMSSZ
python3 -B research/local_dynamics_qualification_20261029/api_fixture/run_fixtures.py \
  --repo "$PWD" \
  --source-sha "$VIGILODE_EXEC_SHA" \
  --remote-ref "origin/$VIGILODE_BRANCH" \
  --output "$VIGILODE_EVIDENCE"
```

Runner는 HEAD와 remote-ref가 `--source-sha`와 같은지 확인하고, baseline 및 current 전체 Git tree를 각각 별도 tar 파일로 스트리밍 archive한 뒤 output 안에 푼다. Cargo는 이 두 immutable archive의 source만 사용하며 현재 checkout을 직접 빌드하지 않는다. Runner, manifest template, fixture source 세 파일은 `git show <source-sha>:<path>` bytes의 SHA256과 현재 실행 파일 bytes를 대조한다. 불일치면 `SOURCE_IDENTITY_MISMATCH`로 멈추며 Cargo를 호출하지 않는다. archive에 필요한 `fixtures/`, `research/`, `docs/`, `tools/` 중 하나가 빠지면 `INPUT_DEPENDENCY`로 멈춘다. Cargo 네 명령은 `/tmp/vigilode-cargo-20261027.lock`을 각각 획득하고, lock 대기 후 각 명령에 900초 제한을 적용한다. `--locked --offline`, Rust 1.94.1, Cargo jobs 2, 조합별 분리 target을 사용한다. 시스템 registry cache가 잠긴 dependency를 제공하지 않으면 환경 실패로 판정해야 한다.

예상 관찰은 두 baseline fixture가 컴파일되고, 현재 버전은 새 `SwitchingConfig` 필드(E0063) 및 `SwitchingEvidence` variant(E0004) 때문에 각각 컴파일 오류가 나는 것이다. 현재 fixture의 비정상 종료만으로는 break로 분류하지 않는다. 두 baseline 명령이 exit 0이고, 두 current 명령 모두 각자의 fixture 파일에 해당하는 정확한 Rust 진단을 포함해야 `BREAKING_CHANGE_REPRODUCED`가 된다. timeout, Cargo/lock 실행 문제, dependency 누락, 무관한 컴파일 오류는 모두 `INCONCLUSIVE_*` 상태다. 이는 source compatibility break의 재현이지 호환성 승인 기준이 아니다. 최종 API/version/migration 판정은 별도로 필요하다.

이 fixture는 Cargo를 실행하지 않은 상태로 준비되었다. 사전 검증은 Python runner 도움말과 파일/소스 identity만 대상으로 한다.
