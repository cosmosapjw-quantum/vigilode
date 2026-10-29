# 조건부 후보의 로컬 검증 캠페인

`conditional_campaign.py`는 조건이 맞을 때 정확도·안정성·비용 측면의 이점이 생길 수 있는 후보를 **별도의 커널·비선형 근 찾기 실험**으로 실행한다. 전체 ODE를 적분한 벤치마크와 합쳐 순위를 만들거나 production 경로를 자동 변경하지 않는다. 기본 실행은 예제마다 서로 다른 프로세스를 세 번 시작하며, 모든 실행은 물리 CPU 한 개에 고정한다. NCP를 사용하지 않고 패키지를 자동 설치하지 않는다.

포함되는 예제는 다음과 같다.

| 실행 파일 | 범위 | 비교 해석 |
|---|---|---|
| `conditional_chart_sweep` | 조건이 제어된 작은 좌표계에서의 선형 해법, 원래 좌표의 오차 검증, refinement 및 비용 상한 | 좋은 조건·누출·비정규성·퇴화 조건과 허용오차별로 준비와 fallback 비용을 포함하여 비교한다. |
| `conditional_continuation_sweep` | 고정된 전처리 연산자를 사용하는 nonlinear continuation/root 후보 | 단순한 최종 방정식 근 찾기와 비교하고, 초기값·branch·함수평가 예산에 따른 거부까지 보존한다. |
| `structured_residual_sweep` | 원래 큰 시스템의 structured directed residual 검증 | O(nr) 또는 구조별 선형 비용의 검증을 dense 검증 및 정확한 구조 해법과 비교하고 모든 준비·검증 비용을 보존한다. |
| `invariant_module_bench` | 불변부분공간 선형 커널 | 전체 LU 및 직접 구조 해법과 비교한다. 내부 10회 반복을 독립 프로세스 10회로 세지 않는다. |

기존 `stage_campaign.py`의 결합 Rosenbrock 단계 병렬 실험은 별도 자료로 유지한다. 단계 병렬, RHS 데이터 병렬, 독립 궤적 앙상블 병렬, 이 문서의 조건부 커널 실험은 같은 속도비로 합산할 수 없다.

## 실행 준비

최종 배포된 정확한 commit과 tree의 clean checkout에서 실행한다. 아래의 `COMMIT`, `TREE`, 경로는 실제 handoff 값으로 바꾼다. 빌드/실행 산출물과 manifest는 checkout 밖에 둔다. 이미 존재하는 실패 기록 위에는 덮어쓰지 않는다.

```bash
COMMIT='<published-commit>'
TREE='<published-tree>'
TASK_ROOT="$HOME/vigilode-conditional-run"
TARGET_DIR="$TASK_ROOT/target"
RECEIPTS="$TASK_ROOT/receipts"
mkdir -p "$RECEIPTS"

python research/local_frontier_20261024/local/build_local.py \
  --expected-commit "$COMMIT" --expected-tree "$TREE" \
  --example conditional_chart_sweep \
  --target-dir "$TARGET_DIR" --output "$RECEIPTS/build-conditional_chart_sweep"

python research/local_frontier_20261024/local/build_local.py \
  --expected-commit "$COMMIT" --expected-tree "$TREE" \
  --example conditional_continuation_sweep \
  --target-dir "$TARGET_DIR" --output "$RECEIPTS/build-conditional_continuation_sweep"

python research/local_frontier_20261024/local/build_local.py \
  --expected-commit "$COMMIT" --expected-tree "$TREE" \
  --example invariant_module_bench \
  --target-dir "$TARGET_DIR" --output "$RECEIPTS/build-invariant_module_bench"

python research/local_frontier_20261024/local/build_local.py \
  --expected-commit "$COMMIT" --expected-tree "$TREE" \
  --example structured_residual_sweep \
  --target-dir "$TARGET_DIR" --output "$RECEIPTS/build-structured_residual_sweep"

python research/local_frontier_20261024/local/conditional_campaign.py \
  --write-template "$TASK_ROOT/conditional-manifest.json" \
  --target-dir "$TARGET_DIR" --receipts-dir "$RECEIPTS"

python research/local_frontier_20261024/local/conditional_campaign.py \
  --manifest "$TASK_ROOT/conditional-manifest.json" \
  --expected-commit "$COMMIT" --expected-tree "$TREE" \
  --output "$RECEIPTS/conditional-campaign" \
  --repeats 3 --timeout 120 --budget 900 --memory-gib 16
```

빌드 기본값은 `--locked --offline`이다. 의존성 캐시 부족은 solver 결함으로 해석하지 않고 실패 로그를 보존한다. 필요한 경우 명시적인 `--online-locked` 옵션으로 잠긴 의존성만 받을 수 있다. 빌드를 동시에 여러 개 띄우지 않는다.

Manifest의 `experiments`에는 예제당 하나의 항목만 둔다. 예제 내부 조건 sweep은 `args`에 지정한다. 더 큰 차원이나 다른 조건을 실행할 때에는 manifest와 출력 디렉터리를 새로 만들어 이전 캠페인을 보존한다. 차트 sweep은 기본 `--dimension 32 --repeats 1`을 사용하며, 캠페인의 `--repeats 3`이 세 번의 독립 실행을 담당한다.

## 무결성 및 재개

실행 전 다음 항목이 모두 일치해야 한다.

1. 실제 checkout의 clean commit과 tree.
2. 성공한 빌드 영수증의 source-before/source-after와 Cargo.lock 해시.
3. 빌드 명령의 정확한 crate와 example 이름.
4. 실제 바이너리 경로와 SHA256, 빌드 영수증 파일의 SHA256.
5. 재개 시 기존 campaign identity와 모든 raw stdout/stderr 해시.
6. host 이름, 운영체제·플랫폼, 전체 물리 CPU 선택과 허용 affinity. 같은 CPU 번호를 가진 다른 host의 실행을 합치지 않는다.

각 실행 폴더에는 `START.json`, `stdout.jsonl`, `stderr.log`, `resource.txt`(GNU time이 설치된 경우), `RECEIPT.json`이 남는다. 영수증 안의 구조화된 행이 원본 JSONL과 달라져도 재개를 거부한다. 실행 중 source나 바이너리가 바뀌면 그 시도를 별도 실패 상태로 기록하고 후속 실행을 중단한다. 중단되어 영수증이 없는 시도는 삭제하거나 성공으로 바꾸지 않는다. 해당 시도의 전체 timeout을 누적 예산에서 차감한다.

기존 실패는 자동 반복하지 않는다. 동일한 출력 경로에서 재개하면 완료·실패 영수증을 검증하여 재사용하고, 아직 시작하지 않은 작업만 남은 예산 내에서 실행한다. 실패를 다시 시험하려면 새 출력 디렉터리를 사용한다.

## 결과 해석

각 예제의 원본 JSONL을 그대로 영수증에 연결한다. `SUMMARY_*.json`은 다음을 유지한다.

- 모든 accepted/refused/error 상태와 음성 대조 조건.
- 원래 좌표의 오차 또는 residual 관련 필드. nested raw/conditional/bisection 결과와 guard 행도 보존한다.
- setup, certification, fallback, solve 등 예제가 실제로 측정한 시간 필드.
- 프로세스 전체 wall time과 timeout·비정상 종료.
- 동일 프로세스 내부 반복을 먼저 요약한 뒤 계산한 프로세스별 중앙값. `config`와 추가 scenario 필드를 identity에 보존하여 서로 다른 조건을 병합하지 않는다.

측정되지 않은 비용은 0으로 채우지 않는다. 예제별 timing scope를 확인하여 준비·검증·실패·fallback을 누락한 속도비를 보고하지 않는다. 조건부 거부가 예상된 음성 대조에서는 거부 자체가 올바른 동작일 수 있지만, 해당 행과 거부 비용을 삭제하면 안 된다. 허용오차를 통과하지 못한 행은 실행 성공과 구분해야 한다.

이 wrapper는 production 승격이나 전체 ODE speedup을 선언하지 않는다. 여기서 좋은 후보는 이후 실제 solver 호출 경로에 연결하여 전체 비용과 정확도를 다시 확인할 수 있다. 연결 전까지 선형 커널 또는 근 찾기 수준의 조건부 결과로 표현한다.

## 이 환경에서의 검증

`test_conditional_campaign.py`는 source/build/binary/Cargo.lock 연결, 잘못된 package, 변경된 바이너리, 비정상 child 종료 보존, 세 개의 별도 프로세스와 재개, 결과 변조 검출, 내부 반복의 독립 표본 오인 방지를 검증한다. 이때 사용하는 작은 가짜 실행 파일은 캠페인 소프트웨어 검증용 fixture이며 solver 성능 자료가 아니다. 실제 예제의 bounded smoke 증거는 별도 파일로 보존한다.

최종 준비 확인에서는 네 예제 각각 세 개의 독립 프로세스, 총 12개 native smoke가 모두 종료 코드 0으로 완료되었다. 원본 1245행에는 chart의 48개 unresolved HOLD, continuation의 예산·오차 floor·branch 거부, structured residual의 음성 대조와 fallback이 그대로 남아 있다. 이 세션 checkout이 개발 중 dirty 상태였으므로 이를 clean-source 배포 캠페인으로 표현하지 않는다. 담당자가 검증한 최종 바이너리와 예제 소스 해시를 고정한 개발용 smoke이며, 실제 local 캠페인은 위의 clean commit/tree 및 빌드 영수증 절차를 사용한다. 세부 기록은 `../evidence/conditional-native-final-summary.json`을 따른다.
