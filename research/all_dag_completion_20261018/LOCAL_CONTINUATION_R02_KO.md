# A01–A12 local continuation R02 — 2026-10-18

## 왜 continuation이 한 번 멈췄는가

이전 응답은 LOCAL-02의 6개 command group과 `nonlinear_homotopy_run`을 실제 실행했지만, 새 결과를 receipt/manifest에 연결하지 않았다. 이는 작업 성공과 DAG closeout을 같은 것으로 취급한 controller 실수였다. 기존 A01–A12 cloud 구현·증명·독립심사를 다시 수행하지 않고, 최신 branch source에서 빠진 local evidence accounting을 마감한다.

## 최신 source에서 실행한 검증

- source commit `dfc96b54e1f157d9998e7422dc4a58b86b66e983`, tree `2b36c4cc1a4f6c4ac4fb9bc7ae1832d0ee19f8c2`.
- `python3 tools/validate_all_dag_completion.py`: exit 0; 12 node ID, 414 source entries, source-manifest SHA256 `5a461ee10fc178e188631e242705d992978d68292d4a10f0ee3f856854cea916`, DAG SHA256 `38f2d5c2a5ba44dcf93120bbd0c9a1c98937d20767f74065a5e2372003c68063`; error 없음.
- `python3 tools/validate_all_dag_completion.py --execute --output research/all_dag_completion_20261018/local_validation_02`: exit 0; core contracts, integrator contracts, homotopy unit, Leja compile-fail docs, polynomial compile-fail docs, Python tool contracts 모두 exit 0. 실행 전후 source/DAG identity가 같음.
- `cargo run --locked --offline --profile measurement -p rodas5p-integrators --example nonlinear_homotopy_run`: exit 0; requested endpoints 8개, weighted tolerance `1e-5`, 최대 weighted radius `1.8976595905312864e-9`. 이 실행은 speed comparison이 아니며 후보 roundoff certificate도 주장하지 않음.

실행별 argv, 시간, exit code와 stdout/stderr hash는 `local_validation_02/RESULT.json`, `local_validation_02/nonlinear_homotopy_run.json`, `local_validation_02/static_check.json`에 있다. 원시 파일은 같은 directory에 보존했다.

## 전체 DAG·기존 증거를 대조한 결과

현재 `DAG.json`은 원래 12개 ID, acceptance 및 dependency를 유지하며 A01–A12 각각 `IMPLEMENTED_SCOPED_VALIDATION_COMPLETE`다. 기존 LOCAL-01–05 완료 기록도 대조했다. LOCAL-04의 450행 source-bound benchmark는 이미 기록된 실행을 재사용했다. `33e45625…`부터 현재 source까지 `crates/`, `tools/`, `Cargo.lock` diff가 없어 benchmark의 code input은 동일하다. 이번 revision에서 전체 timing campaign을 재실행하지 않았다.

닫힌 범위: 원래 12개 노드의 전달된 scoped 구현·증거·독립 심사; 최신 source에 대한 LOCAL-01/02 contract check; nonlinear homotopy client smoke. 원래 DAG와 별도 local task DAG에 미완료 `READY`/`RUNNING` 작업은 없다.

계속 `HOLD`: broad production/general solver accuracy, matched-accuracy speed, arbitrary callback global certificate, full-run zero allocation, 다른 목표 hardware/thread configuration 및 모든 feature 조합. A03 carrier=0, A04/A08 EstimateOnly 경로, A06 exact represented sum(weights)=1, A09 stage target/ODE error 경계는 원래대로 보존한다.

## 환경·게시 경계

Rust `1.94.1`, Cargo `1.94.1`, Python `3.12.3`; Linux x86_64, AMD Ryzen 9 5900X, 24 logical CPUs; Cargo는 `--locked --offline`이다. Cargo.lock SHA256은 `04c7d6c147bf19ebb1a627463705d9dcf27337a3ff228dd1169f19eb73c71400`이다. 현재 local supplement는 exact source/build/config에 묶였고, promotion 권한을 추가하지 않는다.
