# 재현 안내

이 폴더의 `run_reproductions.py`는 저장소 안의 `research/adversarial_reaudit_20260930_r2/`로 옮겨져도 동작한다. 원본 probe와 연구 스크립트를 새 출력 디렉터리로 복사하고, 복사한 Cargo manifest의 저장소 경로를 `--repo`로 바꾼 뒤 실행한다. 원본 생산 코드나 이번 감사의 기존 로그를 덮어쓰지 않는다.

## source identity와 실행환경

감사 대상 source commit은 `7708ef90554fc3986478d4602de6a01c7266b14f`다. 실제 실행 admission은 `evidence/source/SOURCE_FILE_HASHES.json`의 **모든 파일 SHA-256**을 비교한다. 하나라도 없거나 다르면 실행하지 않는다. 따라서 보고서만 추가한 후속 commit에서는 HEAD가 달라도 명시된 source 파일들이 동일하면 실행 가능하다. 이 판단은 manifest에 열거된 파일들의 byte identity이며 저장소 전체의 semantic equality를 뜻하지 않는다.

최상위 runner는 Python 3.11+ 표준 라이브러리만 사용한다. native probe에는 Rust/Cargo 1.94.1과 locked dependencies가 필요하다. 실행은 `--locked --offline`, 빌드 jobs=1로 고정한다. 자신의 설치된 toolchain과 vendor/cache를 환경에 설정한 뒤 실행해야 하며 runner는 Rust 설치나 dependency 다운로드를 하지 않는다. math 경로에는 별도로 NumPy와 SciPy가 필요하다. 실제 감사 환경의 정확한 버전과 설치 경위는 runtime 및 lane별 영수증을 따른다.

원 저장소를 새로 checkout하거나 branch를 만드는 작업은 runner가 수행하지 않는다. 이번 게시와 동일한 source bytes를 가진 기존 checkout 경로를 지정한다. 최신 후속 수정으로 hash가 달라졌다면 원본 감사의 재현과 후속 버전 재검증을 별개 작업으로 취급해야 한다. hash gate를 무력화하여 과거 PASS를 새 source에 재사용하지 않는다.

## 먼저 준비 상태 확인

다음 예시에서 `VIGILODE_REPO`와 `VIGILODE_AUDIT`는 실제 경로로 바꾼다. `--out`은 기존에 없는 디렉터리여야 하며 저장소와 감사 패키지 밖에 있어야 한다.

```bash
VIGILODE_REPO=/path/to/vigilode
VIGILODE_AUDIT="$VIGILODE_REPO/research/adversarial_reaudit_20260930_r2"
python3 "$VIGILODE_AUDIT/run_reproductions.py" \
  --repo "$VIGILODE_REPO" \
  --out /tmp/vigilode-r2-prepared \
  --case all --prepare-only
```

`PREPARED_NOT_EXECUTED`는 source hash 비교 및 복사/rebase가 완료되었다는 뜻이다. native compile, 수학 계산, 과학 검증의 PASS가 아니다. 실제 실행에는 **다른 새 출력 디렉터리**를 쓴다.

## 선택 실행

```bash
python3 "$VIGILODE_AUDIT/run_reproductions.py" \
  --repo "$VIGILODE_REPO" --out /tmp/vigilode-r2-phi --case phi

python3 "$VIGILODE_AUDIT/run_reproductions.py" \
  --repo "$VIGILODE_REPO" --out /tmp/vigilode-r2-output --case output

python3 "$VIGILODE_AUDIT/run_reproductions.py" \
  --repo "$VIGILODE_REPO" --out /tmp/vigilode-r2-statistics --case statistics

python3 "$VIGILODE_AUDIT/run_reproductions.py" \
  --repo "$VIGILODE_REPO" --out /tmp/vigilode-r2-math --case math
```

| `--case` | 실행 내용 |
|---|---|
| `phi` | 원본 crate를 사용하는 phi native adversarial probe |
| `output` | output/certificate native probe와 별도 `safe_budget` 후보 binary |
| `statistics` | 원본 measurement/assessment/Pareto API probe 및 국소 admission/ledger 후보 |
| `math` | Laguerre joint action, homotopy majorant, similarity telemetry, exact output/certificate oracle, exact empirical bootstrap oracle |
| `all` | 위 항목들을 순차 실행 |

homotopy 계산에는 `--repo/fixtures/rodas5p_coefficients_snapshot.json`을 명시적으로 넘긴다. 그 fixture도 source hash admission 대상이다. 각 Python 스크립트는 출력 디렉터리의 복사본에서 실행하므로 `__file__` 옆에 결과를 쓰는 코드도 원본 evidence를 바꾸지 않는다.

## 로그와 실패 의미

새 `--out` 아래에는 다음이 생긴다.

- `work/`: 복사된 probe, rebase된 Cargo manifest, 복사된 math scripts 및 생성된 결과.
- `logs/`: 각 명령의 stdout/stderr.
- `cargo-target/`: native 선택 시 새 build outputs. 이를 감사 패키지나 git에 추가할 필요는 없다.
- `REPRODUCTION_RECEIPT.json`: source identity, 실제 argv, cwd, exit code/timeout, 실행 시각, 생성 파일 SHA-256.

기본 command timeout은 600초이며 `--timeout`으로 바꿀 수 있다. 실패나 timeout이 발생하면 해당 원인을 기록하고 뒤의 명령들은 `NOT_RUN`으로 남긴다. source mismatch 또는 기존 출력 디렉터리에는 실행을 거부한다. offline dependency가 없으면 runtime/dependency blocker이며 solver의 수학 오류로 분류하지 않는다.

재현 probe에는 **알려진 결함이 다시 나타나는지** 확인하는 assertion도 있다. 그러므로 probe exit 0은 원본 solver가 결함 없이 안전하다는 뜻이 아니다. 재현 기록의 finding, 범위와 기대값을 읽어야 한다. 실제 원본 timing/CLI contracts 또는 전체 workspace suite를 전부 다시 실행하는 도구도 아니다. 전체 suite의 실행 및 부분 timeout은 `evidence/runtime/`의 별도 영수증을 따른다.

## 이번 wrapper의 검증 범위

원본 native probe는 최초 감사 경로에서 실제 compile/run했고, lane별 기록과 독립 재실행 영수증이 있다. 이 최상위 relocatable wrapper는 source 검증 후 **all 준비 모드**와 **math 전체 실행**을 별도로 검증했다. 옮겨진 wrapper를 통한 native rebuild는 이 단계에서 반복하지 않았다. 따라서 다음을 구분한다.

- original native execution: 실제 실행 증거 있음.
- relocated native preparation: 복사/rebase/locked command 준비까지 검증.
- relocated native rebuild/run: 이번 wrapper 검증에서는 `NOT_RUN`.
- relocated math execution: 실제 end-to-end 실행과 원본 script/evidence 보존 확인.

상세 상태와 실제 exit는 `evidence/wrapper_validation/WRAPPER_VALIDATION.json`을 따른다. 이전 evidence가 저장된 사실과 새 경로에서 source-identical 재현을 완료한 사실을 혼동하지 않는다.
