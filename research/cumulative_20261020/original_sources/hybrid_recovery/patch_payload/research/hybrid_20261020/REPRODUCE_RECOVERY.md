# 복구 결과의 적용 및 재현

## 권한·범위

이 묶음의 문서와 검산 코드는 아직 새로 push되지 않았다. 기존 PR77의 구현/실측 자료는 원격 HEAD `f738fb481bad3f086b568c8493105576851093f2`에 있다. 제공한 patch는 그 위에 **새 파일만 추가**하며 생산 numerical source를 바꾸지 않는다. 이번 환경에서는 read-only GitHub 도구와 비인증 로컬 CLI만 있어 추가 쓰기를 수행하지 않았다.

## 1. 이미 보존된 CSV의 bounded 검산

저장소 루트에서 다음을 실행한다. 실제 실행된 검산과 같은 코드이며 Python 표준 라이브러리만 사용한다.

```bash
python3 research/hybrid_20261020/recovery_20261020/verify_recovery.py \
  --csv research/hybrid_20261020/runs/39378576576/RESULTS.csv \
  --output /tmp/vigilode-pr77-csv-recovery.json --self-test
```

기대 범위: 원본 CSV 5,048bytes/40행의 SHA256와 Git blob, 32개 경험적 오차비 중22개 factor2 이내, 그중18개 동일 certificate 계약, 정확한 유리수 대조32개,9개 회복 시험. 새로운 solver 실행이나 timing sample은 없다. 이 검사는 **전체 continuous proof replay를 대체하지 않는다**.

## 2. 원래 continuous proof 전체를 다시 읽기

현재 저장소에 이미 있는 `audit.py`는 Fraction으로 원래 구간별 Bernstein coefficient와 비교 반경을 확인한다. 아래는 재현 명령이며 **이번 복구에서 이 전체 실행을 다시 했다는 뜻은 아니다**. 역사 디렉터리를 덮어쓰지 않도록 복사본을 사용한다.

```bash
ROOT=research/hybrid_20261020
RUN=$ROOT/runs/39378576576
# 이번 고정 실행의 RECONSTRUCTION.json은 []다.
# 미래 실행에 part 파일이 생겼다면 그 실행의 복원 규칙부터 적용해야 한다.
WORK=$(mktemp -d /tmp/vigilode-pr77-proof.XXXXXX)
cp -a "$RUN/." "$WORK/"
(cd "$WORK" && sha256sum --check SHA256SUMS)
python3 "$ROOT/audit.py" "$WORK"
```

기록된 원래 결과는 `PASS_SCOPED_RATIONAL_CERTIFICATE_AND_RAW_READBACK`, residual coefficient87,056개, segment203개, 반환 출력600개, paired median32개다. 이 재현 역시 생성형 semidiscrete 모델의 조건부 proof data를 검증하는 것이며 임의 ODE/PDE나 독립인원 심사로 확장하지 않는다.

## 3. 원래 실행 소스의 Rust 시험

새 실행은 새 증거다. 원래 measured source를 분리된 worktree에 둔다.

```bash
git worktree add --detach /tmp/vigilode-pr77-source \
  49ca3150c68d527a275c855ecf814122ff9c533a
cd /tmp/vigilode-pr77-source
rustup toolchain install 1.94.1 --profile minimal
cargo +1.94.1 test --manifest-path crates/vigilode-hybrid/Cargo.toml --locked
```

기존 성공 로그의19시험과 새 실행의 결과를 구분한다. 이 crate는 standalone `[workspace]`이므로 루트 `cargo test --workspace`가 자동으로 이 시험을 대신하지 않는다.

외부 DVODE/Diffsol benchmark를 다시 만들려면 `HYBRID_VODE_SOURCE`에 원래 Netlib/LINPACK source와 Fortran bridge를 준비해야 한다. 정확한 원전 commit은 `9c6a6d9145795e35b978652a567b806ddc84a6bd`; 필요한 파일목록과 source SHA 확인은 `run_ci.sh`의 `external_sources` 부분을 따른다. 그 스크립트에는 원격 push도 포함되므로 단순 재현 목적으로 전체 스크립트를 무심코 실행하지 않는다. 준비 후 빌드 명령은 다음과 같다.

```bash
cargo +1.94.1 build --manifest-path crates/vigilode-hybrid/Cargo.toml \
  --locked --profile measurement --features external --example hybrid_bench
# N EPS ARM TOL REPS WORKERS
crates/vigilode-hybrid/target/measurement/examples/hybrid_bench 64 1 auto 1e-6 3 1
```

새 host/time/source identity를 새 디렉터리에 보존한다. 벤치마크 원자료를 나중에 재실행하여 byte-identical time으로 복구할 수 있다고 가정하지 않는다.

## 4. 이 문서 patch의 적용·게시

먼저 사용자의 실제 작업이 없는 clean worktree에서 대상 branch와 HEAD를 확인한다. 아래는 수동 실행용이며 이번 세션에서 수행하지 않았다.

```bash
git switch research/hybrid-structure-ad-20261020
git status --short
git rev-parse HEAD
# 제공 파일의 실제 위치를 사용한다.
git apply --check /path/to/PR77_Hybrid_Recovery_Closeout.patch
git apply /path/to/PR77_Hybrid_Recovery_Closeout.patch
python3 research/hybrid_20261020/recovery_20261020/verify_recovery.py \
  --output /tmp/vigilode-pr77-recovery-check.json --self-test
git add research/hybrid_20261020/RECOVERY_REVIEW_KO.md \
        research/hybrid_20261020/REPRODUCE_RECOVERY.md \
        research/hybrid_20261020/recovery_20261020
git commit -m 'docs: close hybrid structure-AD recovery with bounded readback'
git push origin HEAD:research/hybrid-structure-ad-20261020
```

이 명령은 merge/default 변경을 포함하지 않는다. HEAD가 다른 세션에서 갱신됐다면 diff와 새 파일 충돌을 먼저 확인하고 원격 작업을 강제로 덮어쓰지 않는다. 사용자 인증·토큰은 패치나 증거 파일에 포함하지 않는다.
