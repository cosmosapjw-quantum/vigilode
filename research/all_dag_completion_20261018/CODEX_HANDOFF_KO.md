# Local Codex에 전달할 prompt

Vigilode의 `research/all-dag-completion-20261018` 브랜치를 이어받아라. 저장소는 https://github.com/cosmosapjw-quantum/vigilode 이다. 이번 작업은 원래 DAG의 A01–A12 전체에 대한 병렬 구현·증명·독립 검토이며, 한 노드만 완료한 이전 handoff와 다르다.

먼저 `research/all_dag_completion_20261018/{REPORT_KO.md,DAG.json,SOURCE_MANIFEST.json,HANDOFF_TASKS.json,nodes/A12/DECISION.json}`을 읽어라. `DAG.json`의 12개 id, original acceptance, dependency를 유지하라. 이전 루트 DAG/과거 continuation 상태로 이번 scope를 덮어쓰지 마라. 현재 증거의 canonical 경로는 이 폴더이다.

## 현재 완료 상태

- A01 immutable weighted packet owner와 실제 반환 중심 radius.
- A02 transactional adaptive global trajectory, 불변 tolerance/requested mesh, 실패 작업량 보존, A10 optional fallback.
- A03 κ≥0 common damping native endpoint/multiwindow packet.
- A04 original-budget Leja admission token.
- A05 PR101/collective/GrayScott generated owner를 받는 generic continuous-residual driver와 interval Horner 수정.
- A06 native all-stage rank-two baseline, certified Schur inverse fallback, opt-in residual witness.
- A07 fixed-protocol 비교 도구와 실패·source·allocation·achieved-error gate.
- A08 Leja/Laguerre/Chebyshev guarded routing.
- A09 실제 전체 α/β를 보존한 H5 stage client와 별도의 nonlinear window homotopy.
- A10 actual frozen residual의 Bernstein refinement.
- A11 실제 GrayScott tube/provider, full μ₂ bound, 임시 callback 배열 제거.
- A12 별도 agent의 독립 심사. 좁은 인증 API는 PASS, broad production/speed는 HOLD.

Rust 1.94.1로 이 환경에서 실제 컴파일·테스트·예제를 실행했다. pseudocode를 local에서 다시 구현하는 작업은 남기지 않았다. 원시 성공/실패 로그와 독립 reference는 이미 저장소에 있다. 최신 upstream cloud `86afb40…`, audit `59d95eb…`도 통합했고 신규 legacy Leja oracle의 218 comparisons도 실행했다.

## 순서대로 수행할 작업

1. 현재 작업 트리를 검사하고 사용자 미완료 변경을 덮어쓰지 말라. 원격 branch를 fetch하여 전용 checkout/worktree에서 이어라. default branch로 임의 merge하거나 force-push하지 말라.
2. 먼저 컴파일 없이 아래 검증을 실행하라. 12개 node coverage와 현재 code/asset hash가 맞아야 한다.

```bash
python3 tools/validate_all_dag_completion.py
```

3. 프로젝트에 제공된 Rust/toolchain/vendor를 사용하라. `Cargo.lock`을 재생성하거나 dependency version을 임의로 올리지 말라. 필요한 경우 local 환경에 맞는 PATH/CARGO_HOME/vendor 경로만 설정하라. 원래 함수·모델·허용오차는 바꾸지 말라.
4. local compiler/allocator 차이를 확인할 필요가 있으면 아래 하나의 runner로 변경된 native scope만 실행하라. output은 새 비어 있는 폴더를 사용한다. 이 명령은 과거 전체 campaign을 반복하지 않는다.

```bash
python3 tools/validate_all_dag_completion.py --execute --output research/all_dag_completion_20261018/local_validation_01
```

5. 실제 사용할 client 예제를 선택해 release 또는 프로젝트 `measurement` profile로 실행하라. 아래 예제는 이미 구현되어 있다.

```bash
cargo run --locked --offline --profile measurement -p rodas5p-integrators --example packet_adaptive_run
cargo run --locked --offline --profile measurement -p rodas5p-integrators --example damped_packet_run
cargo run --locked --offline --profile measurement -p rodas5p-integrators --example verified_residual_compare
cargo run --locked --offline --profile measurement -p rodas5p-integrators --example gray_scott_verified_run -- 4
cargo run --locked --offline --profile measurement -p rodas5p-integrators --example nonlinear_homotopy_run
cargo run --locked --offline --profile measurement -p rodas5p-integrators --example actual_k_homotopy_run
```

6. 목표 CPU/thread/dimension의 성능 판정이 목적이면 다음 source-bound 도구를 사용하고 raw 실패를 보존하라. requested tolerance만 같다는 이유로 speed를 판정하지 말라. achieved-error gate, 동일 출력과 setup boundary, allocator calibration을 확인하라.

```bash
python3 tools/all_dag_benchmark.py run --modes 8 32 128 --repeats 5 --processes 5 --output research/all_dag_completion_20261018/local_timing_01
```

위 명령은 원격/외부 업로드를 하지 않는다. 이번 cloud의 8-mode 측정은 structured가 dense보다 느렸다. 좋은 결과만 선택하거나 N을 바꾸어 기존 실패를 덮어쓰지 말고, 크기별 crossover를 기록하라. 이 도구는 broad speed를 자동 promote하지 않는다.

## 판단할 때 반드시 보존할 계약

- A03은 현재 carrier=0 경로이다. 고차 confluent continuous damped representation은 최신 연구 문서에 있는 선택 확장이며 현재 구현인 것처럼 표시하지 말라.
- A04/A08 complete-error admitted 결과와 legacy/Krylov EstimateOnly를 혼동하지 말라.
- A05/A11은 generated immutable model owner의 tube 권한이다. 임의 callback에 caller가 준 상수로 certificate를 만들지 말라. segment를 나중의 임의 시각에 evaluate한 결과는 원래 requested grid와 달리 호출자가 budget을 검사해야 한다.
- A06은 exact represented sum(weights)=1 조건을 유지한다. 다른 노드가 actual S를 지원한다는 이유로 이 조건을 제거하지 말라.
- A09 stage H5는 actual stage target의 root 인증이고 ODE truncation 인증은 아니다. window client는 요청 endpoint를 원래 ODE에 대해 인증하지만 전체 continuous output 인증은 아니다. floating represented K를 nilpotent로 가정하지 말라.
- A10 refinement는 원래 source/tail/tolerance를 유지한다. residual threshold 성공 횟수와 실제 window accept 횟수를 구분하라.
- published unchanged 연구는 다시 증명하지 말라. source/model/assumption이 바뀐 경우에만 해당 의존 부분을 재검증하라.

실패하면 원시 command/source/config/log를 남기고, 바뀐 환경과 해당 코드 경로만 최소 수정하라. 허용오차 완화·출력 mesh 축소·의도치 않은 모델 교체로 통과시키지 말라. 수정 후에는 영향받는 검사만 재실행하고 node receipt/manifest를 새 revision으로 갱신하여 이전 실패를 보존하라. hash를 새로 적는 행위 자체는 재검증이 아니다.

최종 답변에는 12개 노드 전체 상태, local에서 실제 실행한 명령과 exit status, source identity, 남은 하드웨어 한계, scope별 판정을 포함하라. 노드 하나만 작업해 완료로 보고하거나 한 노드만의 결과를 push하지 말라. 추가 publication은 이 handoff와 사용자 지시가 허용한 branch/scope를 유지하고 모든 변경의 정확한 묶음을 검토한 뒤 수행하라.
