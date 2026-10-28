# 로컬 A01–A12 인계 검증 — 2026-10-18

## 실행 범위와 source identity

- 브랜치: `research/all-dag-local-validation-20261018`
- 테스트 source: `33e45625be0803771156704263c3b1c1f92d2b40` (tree `8cc5c76544b4cfa5c288f4d63a066cc6a4578eb9`)
- Rust/Cargo: `rustc 1.94.1 (e408947bf 2026-03-25)` / `cargo 1.94.1 (29ea6fb6a 2026-03-24)`; Python 3.12.3
- 환경: AMD Ryzen 9 5900X 12-Core Processor, 24 logical CPUs; locked dependencies, offline
- A01–A12 source manifest: 414 files, `5a461ee10fc178e188631e242705d992978d68292d4a10f0ee3f856854cea916`; DAG `38f2d5c2a5ba44dcf93120bbd0c9a1c98937d20767f74065a5e2372003c68063`. LOCAL-02 전후 모두 일치. Cargo.lock 변경 없음.

## 실행 결과

- LOCAL-01 `python3 tools/validate_all_dag_completion.py`: exit 0. 12개 node ID와 414개 manifest file이 모두 맞고 errors는 없었습니다. 이 검사는 source/evidence 실행 자체를 대신하지 않습니다.
- LOCAL-02 `python3 tools/validate_all_dag_completion.py --execute --output research/all_dag_completion_20261018/local_validation_01`: exit 0. core, integrator, homotopy unit, 두 compile-fail doc 그룹, hostile Python contract 총 6개 command group이 모두 exit 0입니다. 개별 명령·로그 해시는 `local_validation_01/RESULT.json`에 있습니다.
- LOCAL-03의 필수 `nonlinear_homotopy_run`과 handoff의 추가 client examples 5개(packet adaptive, damped packet, verified residual, Gray–Scott, actual-K homotopy): 모두 exit 0. 원시 stdout/stderr와 각 command의 exit code는 `local_example_01/`에 보존했습니다.
- LOCAL-04 `tools/all_dag_benchmark.py run --modes 8 32 128 --repeats 5 --processes 5`: exit 0, 450/450 rows 성공, schema/source binding과 empirical achieved-error gate가 전부 통과했습니다. 450행 전부 allocator positive control에서 clone 3회를 검출했습니다. 이는 calibration PASS입니다. timed arms의 allocation_calls는 38–1857로 관측되어 full-run zero-allocation은 주장하지 않습니다.

## Accuracy·timing 판정

Dense/structured paired median 시간비는 dense ÷ structured로 8 mode에서 0.437, 1.558, 8.918 (32, 128 mode 순)였습니다. 8 mode에서는 structured가 빨랐고 32/128 mode에서는 느렸습니다. 이 크기 의존 crossover, 작은 CPU 고정 campaign, allocator instrumentation, empirical RK4 기준 때문에 timing은 `DIAGNOSTIC_ONLY`; 일반 speed promotion은 `HOLD`입니다.

각 arm은 동일한 출력 grid와 tolerance를 사용했고 모든 실패 arm은 보존하는 runner를 사용했습니다. 별도 목표 CPU/thread 설정, 전체 workspace/features 조합, certified reference campaign은 실행하지 않았습니다.

## 노드와 production 판정

원래 DAG A01–A12 구현과 독립 review는 전달 source package의 scoped 결과로 유지됩니다. 이번 실행은 변경 없는 해당 source에 대한 local contract/build/client 확인입니다. A12 final gate가 허용한 scoped certificate API 판정 이외로 확대하지 않습니다. broad production, arbitrary callback global accuracy, matched-accuracy speed는 계속 `HOLD`; 기존 validated generic RODAS5P의 문제별 사용 범위는 유지됩니다.

LOCAL-01–LOCAL-05 결과 및 파일 해시는 `LOCAL_VALIDATION_RECEIPT.json`, `local_validation_01/`, `local_example_01/`, `local_timing_01/`을 참조하십시오.
