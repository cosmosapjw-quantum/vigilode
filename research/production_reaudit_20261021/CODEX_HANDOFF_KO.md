# Codex handoff — Codex 6 Luna low 실행 전용

아래 내용을 Codex 6 Luna low 세션에 전달한다.

---

너의 작업은 이미 구현된 VigilODE의 검증·고정 벤치마크 실행·원본 결과 반환이다. 추가 이론 연구, solver/API 설계, 코드 수정, tolerance 조정은 맡지 않는다. 저장소는 `cosmosapjw-quantum/vigilode`, 브랜치는 **`audit/production-reaudit-20261021`**이다. 게시된 최신 브랜치의 깨끗한 checkout에서 시작하고 실행을 시작한 실제 `HEAD`를 기록한다. 이 문서에 고정 SHA를 삽입하거나 문서를 포함한 commit의 SHA를 다시 문서에 써 넣지 않는다.

현재 broad production·전역 정확도·속도 승격은 **HOLD**다. 정상 종료나 tests PASS만으로 바꾸지 않는다. 공개 `Tier::Performance`는 여전히 Protected 경로를 실행한다. 이 세션은 source/config를 읽고 실행 결과를 checkout 밖에 남기는 작업이며, 새 branch·commit·push·package 설치·NCP provisioning을 수행하지 않는다.

## 먼저 읽을 파일

먼저 이 문서와 `research/production_reaudit_20261021/NCP_RUNBOOK.md`, `DECISION.json`, `EXECUTION.json`, `NCP_CONFIG.json`만 읽는다. 저예산 실행에 모든 연구 보고서를 다시 읽을 필요는 없다. 오류가 특정 노드와 관련될 때만 `NCP_IMPLEMENTATION.json`, `NEXT_N01_IMPLEMENTATION.json`, `NEXT_N02_IMPLEMENTATION.json`, `NEXT_N03_IMPLEMENTATION.json`, `NEXT_N05_IMPLEMENTATION.json`, `NEXT_N06_N07_IMPLEMENTATION.json`, `N08_IMPLEMENTATION.json` 중 해당 문서를 확인한다. 개별 receipt의 오래된 pending/부재 관측은 당시 source/환경의 기록이다. 최종 통합 실행은 현재 package의 `EXECUTION.json`, NCP host 준비 여부는 새 `PREFLIGHT.json`을 따른다. 역사적 미구현 제안이나 next_development를 이번 세션의 코딩 지시로 해석하지 않는다.

| 노드 | 이미 들어간 구현 | 이번 실행에서 확인할 범위 |
| --- | --- | --- |
| N01 | facade 소유 Stats 22종, 61 counters, v4 JSON 보존, 384조합 지원 조회 | 고정 regression/API/schema 검증 |
| N02 | build-source closure와 source/corpus/binary 검사, RADAU5/CVODE 전 refinement 계층 | 실제 native link/test 및 strict reference work-precision |
| N03 | sealed model/run에 묶인 fixed-L2 global replay, initial/jump/query 반올림 charge | PR101 provider/replay의 원래 budget과 전체 비용 |
| N04 | outward integer allocation 및 bounded refusal | uniform/optimized algebraic witness; ODE 실행으로 간주하지 않음 |
| N05 | owned frozen-linear Cayley/Bernstein planning/assessment, diagonal staleness 경로 | 등록된 bounded/resolvent/diagonal full-cost 비교 |
| N06 | protected/Krylov/Leja/Laguerre/Chebyshev/router 비교 client | 동일 endpoint budget, 선택 backend/fallback 및 모든 비용 |
| N07 | nonlinear direct/continuation 및 actual-k 비교 client | original-ODE endpoint와 stage-functional 인증의 구분 |
| N08 | source/PLAN/build/runtime/raw 결속 promotion reader, corpus-lock·실행 파일 path 검사 | frozen receipt chain과 scoped 판정 확인; broad production은 HOLD |

이미 감사된 published 증명과 과거 benchmark raw를 일괄 재실행하지 않는다. 아래 runner에 고정된 검증과 **새 비교 campaign**을 실행한다.

## 시작 조건과 package 확인

실행 host에는 Rust/Cargo **1.94.1**, 두 Cargo.lock과 일치하는 offline cache/vendor, `/usr/bin/time`, `taskset`, GCC/GFortran/binutils, BLAS, SUNDIALS headers/libraries, Python pytest/NumPy/SciPy/mpmath가 준비되어 있어야 한다. 최소 8개 허용 logical CPU와 config의 8-GiB admission 조건이 필요하다. 다른 heavy job과 겹치지 않는 host를 사용한다. CPU pinning 자체가 physical-core 독점이나 고정 clock을 증명하지는 않는다.

저장소 root에서 다음을 확인한다. `HEAD`와 방금 fetch한 remote branch tip이 다르거나 작업 트리가 dirty이면 실행하지 말고 그 상태를 반환한다. checkout을 강제 reset하거나 변경을 삭제하지 않는다.

```bash
git fetch origin audit/production-reaudit-20261021
git branch --show-current
git status --short
git rev-parse HEAD
git rev-parse FETCH_HEAD
git rev-parse 'HEAD^{tree}'
```

branch 이름이 정확하고 source가 clean이며 두 commit이 일치할 때, 동일한 Bash 세션에서 실행 identity와 외부 결과 위치를 만든다. 이후 branch/HEAD/source를 변경하지 않는다.

```bash
VIGILODE_RUN_COMMIT="$(git rev-parse HEAD)"
VIGILODE_RECEIPT_DIR="$(mktemp -d "$(dirname "$PWD")/vigilode-ncp-XXXXXXXX")"
VIGILODE_RUN_DIR="$VIGILODE_RECEIPT_DIR/campaign"
git rev-parse HEAD > "$VIGILODE_RECEIPT_DIR/EXPECTED_COMMIT.txt"
git rev-parse 'HEAD^{tree}' > "$VIGILODE_RECEIPT_DIR/EXPECTED_TREE.txt"
python3 research/production_reaudit_20261021/verify_package.py > "$VIGILODE_RECEIPT_DIR/PACKAGE_VERIFY.json" 2> "$VIGILODE_RECEIPT_DIR/PACKAGE_VERIFY.stderr.log"
VIGILODE_VERIFY_EXIT=$?
printf '%s\n' "$VIGILODE_VERIFY_EXIT" > "$VIGILODE_RECEIPT_DIR/PACKAGE_VERIFY.exit"
```

`PACKAGE_VERIFY.exit = 0`이고 JSON `status = PASS`일 때만 계속한다. 실패하면 manifest를 재생성하거나 digest를 고치지 않고 원본을 반환한다. 이 검사는 file identity/DAG coverage 검사이며 수치 solver 인증이 아니다. 결과 디렉터리의 실제 절대 경로를 보고한다.

## 정해진 실행

기본 선택은 다음 **한 명령**이다. runner가 sequential verification, release build, 고정 benchmark를 실행한다. 별도의 손수 작성한 Cargo 명령이나 임의 테스트 subset으로 대체하지 않는다.

```bash
python3 tools/run_ncp_reaudit.py --mode benchmark --output "$VIGILODE_RUN_DIR"
```

검증까지만 먼저 끝내야 하면 위 명령 대신 다음 두 단계를 사용한다. 첫 단계의 `SUMMARY.json.status = VERIFIED`를 확인한 후, 같은 host boot·도구·환경·CPU·source/config에서 두 번째 명령을 실행한다.

```bash
python3 tools/run_ncp_reaudit.py --mode verify-only --output "$VIGILODE_RUN_DIR"
python3 tools/run_ncp_reaudit.py --mode benchmark --output "$VIGILODE_RUN_DIR" --resume
```

다른 shell/tool call에서 이어갈 때는 기록된 결과 절대 경로로 `VIGILODE_RECEIPT_DIR`와 `VIGILODE_RUN_DIR`를 복원하고, `VIGILODE_RUN_COMMIT="$(cat "$VIGILODE_RECEIPT_DIR/EXPECTED_COMMIT.txt")"`로 최초 commit을 읽는다. 새 HEAD나 새 임시 폴더를 같은 campaign의 identity로 대체하지 않는다.

사전 관측만 필요한 경우 `--mode dry-run --output "$VIGILODE_RECEIPT_DIR/preview"`를 별도로 실행한다. dry-run 결과 폴더를 실제 campaign으로 재사용하지 않는다. 기본 `--cpus auto`는 허용된 첫 8개 logical CPU를 택한다. host 담당자가 특정 CPU set을 지정한 경우 최초 명령과 resume에 동일한 `--cpus`를 전달하고 중간에 바꾸지 않는다.

정확한 protocol은 변경하지 않은 `NCP_CONFIG.json`이 정한다. 현재 1/2/4/8 threads에서 2 warmups + 21 recorded repetitions이며, 비교·bound-client 3,220 rows(280 warmups, 2,940 measurements)와 multi-arm work-precision 1개를 합쳐 **3,221 required IDs**다. 모든 arm과 반복을 포함하며 실패·refusal·timeout도 결과다. 제한은 validation/build 단계당 4시간, 비교 process당 120초, work-precision 6시간, invocation당 benchmark 24시간이다.

runner는 `--locked --offline -j1`, 한 test thread, 독립된 test/bench/clippy target과 explicit thread 설정을 관리한다. shared target에서 다른 Cargo job을 함께 실행하지 않는다. runner를 다시 작성하거나 기존 cache를 `cargo clean`으로 지우지 않는다.

## HOLD·실패·resume 처리

runner exit 2 또는 `SUMMARY.json.status = HOLD`이면 `issues`, `PREFLIGHT.json`, 실패 command의 `RECEIPT.json`/stderr를 읽고 원인과 그대로 반환한다. source 변경, 부족한 prerequisite, native compile/link/test 실패, numerical refusal, resource admission 실패를 구별한다. 없는 dependency 버전을 유사 버전으로 대체하거나 Cargo.lock을 다시 생성하지 않는다. package 설치나 작은 코드 수정도 이번 실행 세션에서 하지 않는다.

`--resume`은 동일 source/config/tool/package/environment/host-boot identity와 보존된 artifact hash가 일치할 때만 사용한다. 성공한 receipt를 재사용하며 **실패한 receipt를 삭제·수정·자동 재시도하지 않는다**. prerequisite나 환경을 담당자가 별도로 수정했다면 이전 campaign을 보존하고 새 결과 디렉터리에서 별도 campaign으로 실행한다. timeout이나 refused row를 빼고 반복 수를 다시 세거나 threshold를 완화하지 않는다.

## 별도 promotion reader

benchmark가 끝나거나 HOLD로 종료된 뒤 `SUMMARY.json`이 있으면, 처음 기록한 commit으로 다음 reader를 실행한다. 현재 HEAD가 달라졌다면 그 변경을 먼저 보고하고 측정 identity를 새 HEAD로 바꾸지 않는다.

```bash
python3 tools/production_promotion_gate.py --summary "$VIGILODE_RUN_DIR/SUMMARY.json" --corpus crates/vigilode-bench/protocols/n02_independent_reference_v1.json --expected-commit "$VIGILODE_RUN_COMMIT" --out "$VIGILODE_RUN_DIR/GATE.json"
```

**gate process exit 0은 promotion PASS가 아니다.** `GATE.json.receipt_integrity`, 각 case의 status/reasons, `broad_production`을 실제로 읽어 반환한다. 현재 reader는 broad production을 HOLD로 유지하고 자동 routing/release 변경을 하지 않는다. gate 자체가 실패하면 stdout/stderr와 exit code를 보존하며 빈 결과나 수작업 PASS로 대신하지 않는다.

원래 corpus의 RADAU5/CVODE refinement 3 levels가 모두 성공해야 한다. 한 coarse 실패를 finer 성공으로 숨기지 않는다. 독립 reference의 grid 일치는 exact/global enclosure가 아니다. polynomial/homotopy endpoint 성공은 full curve 인증이 아니며 actual-k는 stage functional만 인증한다. allocation row는 대수적 witness로 ODE steps가 0이다. 이번 global-budget row만 sealed PR101 계약 안에서 full-curve 판정을 가진다.

`elapsed_ns`와 outer `process_elapsed_ns`를 섞지 않는다. unknown RSS는 0으로 바꾸지 않으며 HOLD다. 관측 RSS는 complete process-tree 상한이나 OS 메모리 제한이 아니다. 후보 실패·fallback·rollback·최종 certificate와 runtime 비용을 모두 남긴다. fallback 뒤 얻은 시간을 최초 polynomial family의 성능으로 귀속하지 않는다.

## 반환할 결과

외부 결과 디렉터리를 보존하고 `PACKAGE_VERIFY.json/.exit`, `EXPECTED_COMMIT.txt`, `EXPECTED_TREE.txt`, campaign의 `PLAN.json`, `PREFLIGHT.json`, `SUMMARY.json`, `GATE.json`(생성된 경우), `BENCHMARK_CASES.json`, `WORK_PRECISION.json`, 모든 `runs/*/{RECEIPT.json,stdout.log,stderr.log,gnu_time.txt}`를 반환한다. 일부 파일이 실행 단계에 도달하지 않아 없으면 **NOT_RUN**으로 기록한다. runner가 생성한 machine-readable receipt와 raw hash를 수작업으로 덮어쓰지 않는다.

최종 답변에는 실제 commit/tree, 결과 경로, package/validation/measurement/gate 상태, 성공·실패·미실행 구분, exact blocker 및 재개 가능한 조건만 짧게 적는다. 구현을 추가로 약속하거나 미래 연구 과제를 이 low 세션에 배정하지 않는다.

다음은 **이번 작업의 코딩 목록이 아니라 남아 있는 HOLD 범위**다: sequential/matrix-free vector atol 등의 지원 불가 facade 조합, in-loop event 종료, 일반 callback에 대한 global certificate, 일반 nonlinear stage-tube/drift authority, 일반 nonnormal polynomial action, arbitrary nonlinear full-curve 인증, band/low-rank resolvent extension, broad production 및 matched-accuracy speed 승격. 해당 범위는 새 증거가 없는 한 그대로 유지한다.
