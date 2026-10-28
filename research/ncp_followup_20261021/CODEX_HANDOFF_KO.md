# Codex 실행 인계: NCP 후속 검증

아래 범위는 이미 작성된 코드를 검증·측정하는 실행 단계다. 미구현 AMR/physical adapter를 완료됐다고 가정하지 않는다. 개발 의무는 `NEXT_STEPS.json`과 `BASS_REI_DAG.json`에 남아 있다.

```text
Repository cosmosapjw-quantum/vigilode의
research/ncp-followup-fvm-20261021 branch를 fetch하라.
시작 시 remote tip SHA를 EXPECTED_COMMIT에 고정하고 별도 detached worktree를 만든다.
기존 checkout/reset/사용자 파일은 변경하지 않는다. fetch 이후 tip 이동은 자동 추종하지 않는다.
ARTIFACT_MANIFEST.json의 파일 해시와 git tree를 확인하고 실제 사용 source SHA를 기록한다.

읽을 문서:
research/ncp_followup_20261021/REPORT_KO.md
DECISION.json, NEXT_STEPS.json, SCHUR_IMPLEMENTATION.json, REMAP_IMPLEMENTATION.json,
ACCURACY_IMPLEMENTATION.json, BASS_REI_PORT_RESEARCH.md, BASS_REI_RECENCY.json.

Rust1.94.1, 기존 Cargo.lock, exact dependencies를 사용한다.
검증 output은 checkout 외부의 새 디렉터리다.
python3 research/ncp_followup_20261021/run_followup_checks.py --output <새-절대경로>
이 스크립트는 CARGO_TARGET_DIR를 환경에서 받으므로 실행별 안전한 target을 지정한다.
offline dependency 부재는 환경 HOLD로 기록한다. Cargo.lock을 바꾸지 않는다.
기존 실패 receipt를 덮어쓰거나 새 source에 과거 frozen package PASS를 재사용하지 않는다.

NCP 접근:
ssh -i /home/cosmosapjw/.ssh/ncp_key -o IdentitiesOnly=yes -p 22 root@49.50.142.2
개인키를 출력/복사하지 말고 host-key 검증을 유지한다.
실제 cpuset/RAM/디스크를 읽고 다른 실행을 보존한다.

다음 실행만 먼저 수행한다:
1. 다음 명령으로 빌드:
   cargo build --profile measurement -p rodas5p-integrators --example component_benchmark_native --locked --offline -j 4
   cargo build --profile measurement -p rodas5p-core --example bordered_probe --locked --offline -j 4
2. accuracy/HEAT_BASELINE_REQUEST.json과 HEAT_STENCIL_REQUEST.json을 adapter에 전달.
   "$CARGO_TARGET_DIR/measurement/examples/component_benchmark_native" --request research/ncp_followup_20261021/accuracy/HEAT_BASELINE_REQUEST.json --measurement-mode timing
   "$CARGO_TARGET_DIR/measurement/examples/component_benchmark_native" --request research/ncp_followup_20261021/accuracy/HEAT_STENCIL_REQUEST.json --measurement-mode timing
   adapter process exit0만으로 성공으로 보지 말고 JSON status=OK, times/states 및 reference 정확도를 확인한다.
3. 같은 IVP/출력/원래 application budget으로 baseline/stencil을 교대 실행.
   독립 analytic heat Fourier reference, 정확한 output times/states, derivative_rhs_calls,
   warmed timing.elapsed_ns와 cold process wall을 각각 저장한다.
4. bordered_probe --n 1600 및 --n 4096을 실행해 residual/storage를 먼저 확인한다.
   이 manufactured band probe를 full BASS ODE benchmark로 표시하지 않는다.
5. 비교군은 VODE/DVODE, diffsol, Julia로 제한한다.
   VODE는 archived MF20과 Jacobian 기반 BDF를 별도 arm으로 구분한다.
   구현/API가 준비되지 않은 추가 arm은 NOT_RUN으로 보고한다.

과거 데이터의 교정값:
rotation work precision7/140 pass, VigilODE internal rtol1e-10에서7/7.
이는 stored full-grid summary+independent endpoint 근거다.
새 실행은 full trajectory를 저장하여 원 application budget에서 직접 평가한다.

측정은 실제 사용 가능한 CPU 범위 안에서 독립 job을 병렬화한다.
하나의1-thread solver 실행을64-thread solver scaling으로 표시하지 않는다.
한 비교 block의 CPU/부하/시간범위를 같게 한다. 2warmups+최소7paired confirmation을 목표로 한다.
NCP campaign은 setup 포함 최대120분이며 실패/미실행도 보존한다.
디스크가 부족하면 실행 전 원인을 보고하고 자신이 생성한 재생성 가능한 target만 cargo clean한다.

반환:
source/tree/harness/binary identity, exact commands/exit codes, full raw times/states,
application/internal tolerances, setup/JVP/RHS counters, wall/CPU/RSS 범위,
paired block IDs, coverage, promote/prune/hold/optimization_possible/specialized 판정.
본 branch의 broad production HOLD는 관련 필수 gate를 통과하기 전 유지한다.
```

큰 변경이 없는 local validation은 최종9명령 receipt와 동일 계약이다. `run_followup_checks.py`가 만든 binary는 검증 output에 남아도 Git 산출물에는 추가하지 않는다. 실제 NCP timing의 inference는 동일 정확도와 paired 측정에 제한한다.
