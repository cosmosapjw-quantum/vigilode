# Local Codex handoff

아래 블록의 `PUBLISHED_COMMIT`과 `PUBLISHED_TREE`는 최종 게시 영수증에 제시된 값으로 교체한다. HEAD나 원격 tip을 임의로 대신 쓰지 않는다. 원격 브랜치가 더 진행됐더라도 지정 commit을 별도 worktree로 준비한다.

```text
VigilODE의 고정된 로컬 벤치마크 패키지를 검증하고 병목을 측정하라. 코드의 수학적 의미를 바꾸거나 production 승격을 임의로 선언하지 않는다. NCP는 사용하지 않는다.

원격: https://github.com/cosmosapjw-quantum/vigilode
게시 브랜치: research/local-frontier-20261024
EXPECTED_COMMIT=PUBLISHED_COMMIT
EXPECTED_TREE=PUBLISHED_TREE

1. 현재 checkout의 branch/HEAD/tree/status와 실제 hostname, CPU/메모리를 먼저 수집한다. 기존 checkout은 보존한다. reset/clean/기존 checkout 변경 없이 git fetch origin research/local-frontier-20261024 후 지정 commit의 tree를 검증하고 새로운 경로에 git worktree add --detach NEW_CHECKOUT EXPECTED_COMMIT을 실행한다. 원격 branch tip이 달라도 지정 commit object와 tree가 맞으면 진행한다. commit/tree가 틀리거나 object를 얻지 못하면 실패 원인을 보존한다.

2. NEW_CHECKOUT의 research/local_frontier_20261024/local/README_KO.md와 CATALOG.json, analysis/LOCAL_CAMPAIGN.json을 읽는다. 출력과 Cargo target은 checkout 밖의 새 RUN_ROOT에 둔다. 모든 Python 명령은 python3 -B로 실행한다. 기존 dirty 작업공간이나 오래된 바이너리를 새 소스로 간주하지 않는다.

3. python3 -B research/local_frontier_20261024/src/verify_package.py를 실행하여 패키지와 실행 소스의 SHA256을 확인하고 출력/exit를 보존한다. 이어 python3 -B -m unittest discover -s research/local_frontier_20261024/local -p 'test_*.py' -v를 실행해 stdout/stderr/exit를 보존한다. build_local.py --expected-commit EXPECTED_COMMIT --expected-tree EXPECTED_TREE --output RUN_ROOT/build --target-dir RUN_ROOT/target을 실행한다. 기본은 --locked --offline이다. cache 누락이면 실패 영수증을 보존하고 정확한 누락 패키지를 보고한다. lockfile 갱신이나 자동 패키지 설치는 하지 않는다. 이미 허용된 네트워크 locked dependency fetch가 있을 때에만 --online-locked로 새 build 출력 디렉터리에서 재시도한다.

4. campaign.py preflight --output RUN_ROOT/PREFLIGHT.json으로 실제 허용 물리 코어를 확인한다. 먼저 smoke.json으로 plan을 만들고 --build-receipt RUN_ROOT/build/BUILD_RECEIPT.json을 반드시 제공한다. 동일 expected commit/tree와 RUN_ROOT/target/measurement/examples/local_bottleneck_suite를 사용한다. run 후 analyze한다. 실패/timeout/UNVALIDATED_REFERENCE를 삭제하거나 PASS로 바꾸지 않는다. smoke는 모델이 실행된다는 확인이지 일반 production 적합성 인증이 아니다.

5. smoke의 계약/identity 문제가 없으면 profiling.json의 병목 전용 계획을 먼저 별도 output에서 실행한다. 최대81개 직렬 instrumented task, process당20초/총1800초의 별도 예산이며 결과를 clean timing에 섞지 않는다. 전체2시간 제한이면 checkout 밖 config 복사본으로 profiling1800초+standard5400초를 배분한다. 그 다음 standard.json으로 새 plan/output을 만들어 실행한다. plan에 출력된 실제 프로세스 수와 worst timeout 합계를 보고한다. 24×5=120은 총 실행 수가 아니다. 기본 standard accounted budget은 7200초이다. 모든 구성이 완료되지 않으면 미실행을 그대로 기록하고 요청된 시간 예산 내에서 끝낸다. 엄밀한 이번 invocation 2시간 종료가 필요하면 외부 timeout 7200s를 사용하되 중단된 attempt를 보존한다. resume은 같은 plan/output으로 하며 실패 재시도는 --retry-failed와 새 attempt를 사용한다. 동시에 다른 컴파일/벤치마크를 돌리지 않는다. governor나 perf 권한을 바꾸지 않는다.

6. 별도 결과로 accuracy.json의 16개 양의 물리 시각 cold solves를 실행할 수 있다. 초기값 포함 17개 시각이며 하나의 연속 trajectory latency가 아니다. 추가 3600초 예산은 standard 2시간과 별개이므로 실행 총량을 분명히 보고한다. endpoint/wrapped phase의 일치로 missed cycle이 없다고 주장하지 않는다.

7. 외부 비교는 external_compare.py를 이용해 현재 계약과 일치하는 1/10/15/16번에서 VODE-Adams/BDF와 VigilODE를 비교한다. SciPy wrapper/Python callback 비용을 표기한다. 설치된 Julia와 고정 Project/Manifest가 있을 때만 --julia-project를 제공한다. Julia/diffsol이 없거나 모델 adapter가 없으면 NOT_RUN으로 남기며 다른 solver나 옛 fixture를 대신 쓰지 않는다. 비교대상은 VODE/DVODE, diffsol, Julia로 한정한다.

8. stage 병렬 연구는 build_local.py --example reconsideration_bench로 별도 build 영수증을 만든 뒤 stage_campaign.py로 실행한다. endpoint-only 24모델과 출력 계약이 다르므로 표를 분리한다. standard/data/ensemble/stage의 스레드 예산을 중첩하지 않는다. 물리 코어 1/2/4/8/12 수준만 사용하며 허용 코어 부족은 명시적으로 skip한다.

9. 조건부 기하/continuation 후보는 local/CONDITIONAL_CAMPAIGN_KO.md의 별도 빌드와 conditional_campaign.py로 시험한다. conditional_chart_sweep, conditional_continuation_sweep, structured_residual_sweep, invariant_module_bench를 build_local.py --example로 각각 빌드하고 개별 성공 영수증을 사용한다. 정확도·잔차·refusal/fallback·강건성 및 비용 증가를 함께 기록하며, 조금 느려도 안정성/정확도 이점이 검증되면 조건부 사용 후보로 남긴다. kernel 결과를 24개 전체 ODE 속도 순위에 섞지 않는다. 그 다음 analyze 결과와 plot_campaign.py로 정확도 적격 완료 반복만 시각화한다. RHS/JVP, projection/factor/linear solve, rejection/controller, allocation/memory, pool/synchronization, process/IO overhead를 직접 계측과 미계측으로 구분한다. profile 실행을 깨끗한 timing ratio에 넣지 않는다. 없는 내부 시간을 합계 잔차로 지어내지 않는다.

10. 최종 산출물은 실행 identity, build receipt, PLAN/PREFLIGHT, 모든 raw stdout/stderr/resource/perf와 RECEIPT, 분석 JSON/Markdown, 가능한 그림, 미실행 및 실패 원인이다. 어떤 문제에서 어떤 정확도 조건으로 속도/안정성 이점이 있었는지와 추가 최적화 후보를 정리한다. local benchmark 완료를 broad production 승격과 동일시하지 않는다. 요청 없이 source 수정/추가 연구/원격 push는 하지 않는다.
```
