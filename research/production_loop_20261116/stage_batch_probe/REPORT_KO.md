# V32 stage-batch 동시성 probe: 상한 있는 rendezvous (OD-L1-28)

이 문서는 `parse_results.py`가 `logs/`의 원 출력에서 만든다. 숫자는 모두 그 출력에서 읽었다. wall time은 진단값이며 어떤 판정에도 쓰지 않는다. 기준 commit d4e2772f, 파싱 시점 head `4a90ac87`.

**정정 고지 (2026-11-25, 독립 리뷰 `REVIEW.json`, 판정 accept_with_corrections).** 리뷰어가 원 보고 1080개를 다시 세어 같은 판정을 얻었다(이전 6/680 false, 새 0/400). 아래 네 가지를 바로잡는다. 이 고지는 `parse_results.py --report-only`로 커밋된 `RESULTS.json`에서 렌더링했다.

1. 같은 원인의 flaky가 기존 테스트 `stage_batch_contracts::local_rayon_pool_executes_rhs_rows_concurrently_and_preserves_order`에도 있었다(새 4-thread pool, 2 ms 작업 8개, 대기 없이 최대 >= 2를 단언; 리뷰어의 LD_PRELOAD shim에서 3/3 실패). 이 노드의 허용 경로 밖이어서 builder는 손대지 않았다. 병합 때 orchestrator가 같은 상한 있는 대기를 그 테스트의 병렬 단계에만 넣었다(OD-L1-28, OD-L1-30).
2. 반복 실행에 쓴 증거 실행 파일(probe-bins의 late_workers_test-old/-new, contracts_test-old/-new)은 rustfmt가 줄을 나누기 전의 소스로 빌드되었다. 그래서 커밋된 소스로 바이트 단위 재빌드가 되지 않는다(build-id와 panic 위치 줄 번호만 다르다). 리뷰어가 커밋된 소스로 다시 빌드한 결과의 동작은 같다(이전 10/10 실패, 새 10/10 통과).
3. late-worker 테스트의 `late >= 3` 조건은 probe pool 자신의 worker가 늦었다는 증명이 아니다(벤치마크 pool 둘만으로 1 + 3 = 4). 지금은 모든 보고가 7이고 이전 probe가 실패하므로 유효하다. 의존성 갱신 뒤를 위해서는 `late == 7`로 묶는 편이 안전하다.
4. 요약의 '6/680'은 분모가 섞였다. 바쁜 loop 조건의 strace 실행은 340회(hog1 3/160, hog4 3/180)이고, 680은 이전 CLI 실행 전체다. 과제 문안의 조건만으로는 0/160이었다. 판정은 결정적 증거(late-worker 테스트와 리뷰어의 shim: 이전 CLI false 6/6)에 기댄다.

## 1. 원인

`run_concurrency_probe`는 새 local rayon pool(4 thread)을 만들고 곧바로 작업 8개(각 1 ms sleep)를 넣는다. rayon의 `build`는 worker가 시작되기를 기다리지 않는다. 먼저 돈 worker가 작업 전체를 받아 자기 deque에 나누고, 다른 worker는 실행된 뒤에야 훔쳐 갈 수 있다. 다른 worker가 약 8 ms 안에 돌지 못하면(추적기 아래의 늦은 thread 시작, 부하) 한 worker가 8개를 모두 혼자 실행하고 최대 동시 작업 수는 1, `parallel-observed=false`가 되어 checksum이 `577161eb…`(hosted 두 번째 실행 값)으로 바뀐다.

## 2. 변경

각 작업은 활성으로 표시한 뒤, 동시에 활성인 작업이 2개 이상 관측되었거나 마감이 지날 때까지 100 µs씩 잠들며 기다린다. 마감은 첫 작업이 도착한 시각 + 2 s로 한 번 정해지고 모든 작업이 공유한다. 두 작업을 동시에 돌릴 수 없는 pool은 한 번만 기다린 뒤 지금처럼 1을 보고한다. checksum 구성, 보고서 schema, 작업 8개 × 1 ms, 4 thread pool은 그대로다. `stage_batch.rs` blob `c2f44c32` → `fb68321b` (CHECKPOINT 재고정은 OD-L1-28에 따라 orchestrator가 한다).

출처: 로그에 적힌 `stage_batch.rs` blob과 로그 — `1a44222f`: tests/new_code_new_tests.log; `c2f44c32`: tests/old_code_new_tests.log; `fb68321b`: checks/cli_contracts.log, checks/clippy.log, checks/fmt.log, checks/integrators_stage_batch_tests.log, checks/loop_checker.log, checks/loop_checker_rest.log. 유발 실험의 binary — b1 old: `rodas5p-old-d4e2772f` (sha256 `1dc4cb0a8741…`); b1h old: `rodas5p-old-d4e2772f` (sha256 `1dc4cb0a8741…`); b2 old: `rodas5p-old-d4e2772f` (sha256 `1dc4cb0a8741…`); b2 new: `rodas5p-new-02eb3afb` (sha256 `5056821c371d…`). 새 binary 이름의 commit(02eb3afb)의 blob은 `1a44222f`이고, 그 뒤 최종 blob까지의 차이는 probe 안 주석 문구(2줄)뿐이다(`git diff 02eb3afb f55a1d69 -- crates/rodas5p-integrators/src/stage_batch.rs`).

## 3. 판정

| 판정 항목 | 측정 | 판정 |
|---|---|---|
| old failure provoked | CLI, task-text conditions strace1/strace2: 0/160 false; all CLI conditions: 6/680 false (in hog1, hog4), each with checksum ['577161eb']; late-worker test executable: 20/20 runs failed with 'probe observed 1' | 통과 |
| new code never shows the failure under the same provocation | CLI: 0/400 false (interleaved with the old binary in batch b2); late-worker test executable: 20/20 runs passed | 통과 |
| checksum unchanged | new CLI checksums {'4079849b6d3c0702f14e6dfb977319f6f1f6891c046786a8bd7aff1f1d39c950': 400}; old CLI checksums other than the false value {'4079849b6d3c0702f14e6dfb977319f6f1f6891c046786a8bd7aff1f1d39c950': 674}; test checksums ['4079849b6d3c0702f14e6dfb977319f6f1f6891c046786a8bd7aff1f1d39c950']; composition mismatches 0 | 통과 |
| suites | cli_contracts: exit 0, 25 passed/0 failed; integrators_stage_batch_tests: exit 0, 10 passed/0 failed; clippy: exit 0; fmt: exit 0 | 통과 |

## 4. CLI 유발 실험 (`provoke.sh`)

b1, b1h는 이전 binary만, b2는 같은 조건에서 이전/새 binary를 run마다 번갈아 실행했다(같은 시각의 host 부하를 함께 받는다). CPU 8(과 9)에 고정했다. 과제 문장의 조건(strace1, strace2)만으로는 실패가 나오지 않아 추적기(strace)와 같은 CPU에 바쁜 loop를 두는 조건(hog1, hog4)과 추적기 공유(shared1), strace 없는 대조(hogonly1)를 더했다.

바쁜 loop에 관한 정정: `provoke.sh`의 `with_hogs`는 run마다 loop 2개(hog4는 4개)를 시작하고 run 뒤에 kill하지만, kill은 감싸는 subshell에만 닿고 `timeout 30 sh -c 'while :; do :; done'`은 30초를 다 돈다. 그래서 이어지는 run의 loop가 CPU 8에 쌓인다(스크립트 결함, 모든 hog run에 똑같이 해당하고 b2에서는 이전/새 binary가 같은 상태를 번갈아 받는다). 아래 표의 'CPU 8의 바쁜 loop 수'는 runs.tsv의 시작 시각으로 다시 센 값이다.

| binary | 조건 | batch:실행 | 실행 | false | 관측 최대 동시 작업 분포 | checksum | CPU 8의 바쁜 loop 수 (시작 시, 중앙값 [최소, 최대]) | wall 중앙값 s (진단) |
|---|---|---|---|---|---|---|---|---|
| old | hog1: strace1 + 2 busy loops started on CPU 8 per run | b1:40, b2:120 | 160 | 3 | 1: 3, 2: 18, 3: 14, 4: 125 | 4079849b… 157, 577161eb… 3 | 30.0 [2, 40] | 1.9464 |
| old | hog4: strace1 + 4 busy loops started on CPU 8 per run | b1h:60, b2:120 | 180 | 3 | 1: 3, 2: 37, 3: 35, 4: 105 | 4079849b… 177, 577161eb… 3 | 44.0 [24, 50] | 2.8887 |
| old | hogonly1: taskset -c 8 + 2 busy loops started on CPU 8 per run, no strace | b1h:60, b2:40 | 100 | 0 | 4: 100 | 4079849b… 100 | 22.0 [2, 30] | 3.0058 |
| old | shared1: one strace -f on CPU 8 traces 4 concurrent runs | b1:40, b2:40 | 80 | 0 | 2: 7, 3: 5, 4: 68 | 4079849b… 80 | 3.0 [0, 28] | 0.4378 |
| old | strace1: taskset -c 8 strace -f -qq -e trace=none (task text) | b1:40, b2:40 | 80 | 0 | 3: 1, 4: 79 | 4079849b… 80 | 0.0 [0, 0] | 0.0767 |
| old | strace2: taskset -c 8,9 strace -f -qq -e trace=none (task text) | b1:40, b2:40 | 80 | 0 | 2: 2, 3: 8, 4: 70 | 4079849b… 80 | 0.0 [0, 0] | 0.0249 |
| new | hog1: strace1 + 2 busy loops started on CPU 8 per run | b2:120 | 120 | 0 | 2: 8, 3: 13, 4: 99 | 4079849b… 120 | 30.0 [22, 32] | 2.1535 |
| new | hog4: strace1 + 4 busy loops started on CPU 8 per run | b2:120 | 120 | 0 | 2: 13, 3: 21, 4: 86 | 4079849b… 120 | 44.0 [34, 48] | 2.8641 |
| new | hogonly1: taskset -c 8 + 2 busy loops started on CPU 8 per run, no strace | b2:40 | 40 | 0 | 4: 40 | 4079849b… 40 | 20.0 [4, 28] | 2.9538 |
| new | shared1: one strace -f on CPU 8 traces 4 concurrent runs | b2:40 | 40 | 0 | 4: 40 | 4079849b… 40 | 0.0 [0, 0] | 0.2369 |
| new | strace1: taskset -c 8 strace -f -qq -e trace=none (task text) | b2:40 | 40 | 0 | 4: 40 | 4079849b… 40 | 0.0 [0, 0] | 0.0767 |
| new | strace2: taskset -c 8,9 strace -f -qq -e trace=none (task text) | b2:40 | 40 | 0 | 2: 1, 3: 3, 4: 36 | 4079849b… 40 | 0.0 [0, 0] | 0.0254 |

false가 나온 run (binary: old 6):

| batch | binary | 조건 | run | 관측 최대 | CPU 8의 바쁜 loop 수 | checksum |
|---|---|---|---|---|---|---|
| b1 | old | hog1 | 40 | 1 | 32 | 577161eb… |
| b1h | old | hog4 | 22 | 1 | 40 | 577161eb… |
| b2 | old | hog1 | 79 | 1 | 28 | 577161eb… |
| b2 | old | hog1 | 97 | 1 | 28 | 577161eb… |
| b2 | old | hog4 | 13 | 1 | 48 | 577161eb… |
| b2 | old | hog4 | 18 | 1 | 44 | 577161eb… |

기술 통계(판정에 쓰지 않음): b2(번갈아 실행)의 hog 조건에서 이전 4/240, 새 0/240; 단측 Fisher 정확 검정 p = 0.0617. 새 binary 전체 400회에서 false가 0회이면 false 비율의 단측 95% 상한(Clopper-Pearson) = 0.746%.

## 5. 늦은 worker test (`tests/stage_batch_probe_late_workers.rs`)

test binary의 global allocator가 index ≥ 1인 모든 rayon worker를 첫 할당(작업을 찾기 시작할 때, crossbeam-epoch 등록)에서 300 ms 재운다. worker 0만 정상으로 시작하므로 probe pool에서 worker 0이 300 ms 동안 혼자 돈다. 같은 test를 이전 probe와 새 probe에 대해 build해 번갈아 실행했다(`repeat_tests.sh`).

| probe | 실행 | 통과 | 실패 | 비고 |
|---|---|---|---|---|
| 이전 (d4e2772f) | 20 | 0 | 20 | 'probe observed 1' 20회 |
| 새 (V32) | 20 | 20 | 0 | 보고서 80개: 늦은 worker {'7': 80}, 관측 최대 {'4': 80} |

cargo로 돌린 같은 test (이전 probe / 새 probe):

| probe | stage_batch.rs blob | exit | passed | failed | 실패 test |
|---|---|---|---|---|---|
| old | c2f44c32 | 101 | 1 | 1 | probe_observes_concurrency_when_pool_workers_start_late |
| new | 1a44222f | 0 | 2 | 0 | - |

## 6. 검사 (`run_checks.sh`)

| 검사 | 명령 | exit | 결과 |
|---|---|---|---|
| cli_contracts | `cargo_locked test --locked -j 4 --release -p rodas5p-cli --test cli_contracts --no-fail-fast` | 0 | 25 passed, 0 failed |
| clippy | `cargo_locked clippy --locked -j 4 --release -p rodas5p-integrators --no-deps --lib --test stage_batch_probe_contracts --test stage_batch_probe_late_workers --test stage_batch_probe_latency -- -D warnings` | 0 |  |
| fmt | `cargo +1.94.1 fmt -p rodas5p-integrators -- --check` | 0 |  |
| integrators_stage_batch_tests | `cargo_locked test --locked -j 4 --release -p rodas5p-integrators --test stage_batch_contracts --test generic_scope_contracts --test stage_batch_probe_contracts --test stage_batch_probe_late_workers --test stage_batch_probe_latency --no-fail-fast -- --nocapture` | 0 | 10 passed, 0 failed |
| loop_checker | `python3 -I research/production_loop_20261116/check_checkpoint.py` | 1 | Traceback (most recent call last): /   File "/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116-w4-v32/research/production_loop_20261116/check_checkpoint.py", line 95, in <module> /     main() /   File "/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116-w4-v32/research/production_loop_20261116/check_checkpoint.py", line 77, in main /     require(git('hash-object', path) == expected, 'protected file changed: ' + path) /   File "/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116-w4-v32/research/production_loop_20261116/check_checkpoint.py", line 28, in require /     raise ValueError(message) / ValueError: protected file changed: crates/rodas5p-integrators/src/stage_batch.rs |
| loop_checker_rest | `python3 -I research/production_loop_20261116/stage_batch_probe/loop_checker_rest.py` | 0 | expected complaint (not raised): protected file changed: crates/rodas5p-integrators/src/stage_batch.rs fb68321b94dd4caf8a895ea7e2f9aad5363b80f2 != c2f44c327c53d7c6dfe7ba80b26cd43486d56b43 / PASS: strict JSON, task/DAG structure, ancestry, protected and evidence identities / Scope: checkpoint reproducibility. Production HOLD; no new numerical qualification. |

loop checker(`check_checkpoint.py`)는 첫 불일치에서 멈춘다. 불평은 예상된 `stage_batch.rs`의 보호 blob 하나이며 (OD-L1-28에 따라 orchestrator가 재고정), `loop_checker_rest.py`는 그 한 줄만 보고로 바꾸어 나머지 검사(증거 identity, strict JSON, 변경 경로 allowlist)를 돌린다.

## 7. 실행 시간 영향과 상한 (진단)

정상인 경우 첫 작업은 두 번째 작업이 활성이 될 때까지만 기다린다. 늦은 worker가 있으면 그 worker가 시작할 때까지 기다리며, 이것이 예전에 실패하던 경우다. 두 번째 작업이 끝내 오지 않으면 보고서당 한 번 최대 2 s가 든다. 아래는 새 pool마다 첫 작업이 기다린 시간(첫 작업 도착부터 두 번째 작업이 동시에 활성이 될 때까지)이다. probe와 같은 구성(새 `ParallelExecution::rayon(4)`, 작업 8개, 같은 rendezvous)을 무시된 test `stage_batch_probe_latency`가 재현하고 `latency.sh`가 조건별로 돌렸다. 진단값이며 판정에 쓰지 않는다.

| 조건 | pool 수 | 두 번째 작업 없음 | 최소 ms | 중앙값 ms | p90 ms | p99 ms | 최대 ms |
|---|---|---|---|---|---|---|---|
| plain: 고정과 추적기 없음 | 200 | 0 | 0.000 | 0.001 | 0.004 | 0.014 | 0.018 |
| strace1: taskset -c 8 strace -f -qq -e trace=none | 200 | 0 | 0.004 | 0.013 | 0.063 | 0.297 | 0.651 |
| hog30: strace1 + CPU 8에 바쁜 loop 30개 (run 동안 유지, 뒤에 정지) | 200 | 0 | 0.005 | 0.016 | 8.956 | 32.019 | 33.329 |

CLI run 전체의 wall 중앙값(4절 표)도 진단값이다.

## 8. 남는 점

- 과제 문장의 조건(strace1, strace2; 1개/2개 CPU에서 strace -f)만으로는 이 host에서 실패가 나오지 않았다. 실패는 추적기와 같은 CPU에 바쁜 loop를 둔 조건(hog1, hog4)과 늦은 worker test에서만 나왔다. 같은 바쁜 loop에 strace가 없으면(hogonly1) 실패가 없었다: thread 생성 때의 ptrace 정지가 필요한 요소다.
- CLI 유발 비율이 낮아(4절) CLI 비교만으로는 약한 증거다(단측 Fisher p는 기술 통계). 결정적인 증거는 늦은 worker test다(이전 probe에서 매번 실패, 새 probe에서 매번 통과).
- hosted runner의 추적 방식(`strace -ff --seccomp-bpf -y -e trace=<파일 syscall>`)을 그대로 재현하지는 않았다. 여기서는 `strace -f -qq -e trace=none`(모든 syscall 정지)을 썼다.
- `provoke.sh`의 바쁜 loop 결함(4절)은 고치지 않고 기록했다. 모든 hog run이 같은 결함을 갖고, b2에서는 이전/새 binary가 같은 상태를 번갈아 받았다.
- 2 s 상한: 두 번째 worker가 2 s 넘게 돌지 못하는 host에서는 여전히 1을 보고한다(설계상 상한, 7절의 대기 분포와 비교).
- 독립 검토와 CHECKPOINT의 `stage_batch.rs` 보호 blob 재고정(OD-L1-28)이 완료 조건이다.

## 9. 재현

```
# 이전/새 CLI binary (CARGO_TARGET_DIR의 probe-bins/에 복사해 둔다)
cargo +1.94.1 build --locked -j 4 --release -p rodas5p-cli
# 유발 실험 (b2: 번갈아 실행)
BINARIES="old=<rodas5p-old> new=<rodas5p-new>" ./provoke.sh b2 40 strace1 strace2 shared1 hogonly1
BINARIES="old=<rodas5p-old> new=<rodas5p-new>" ./provoke.sh b2 120 hog1 hog4
python3 -I pack_logs.py logs/b2
# 늦은 worker test executable, 이전/새 probe로 build한 것을 번갈아 20회
./repeat_tests.sh logs/tests/late_workers_repeat.log 20 old=<exe-old> new=<exe-new>
# rendezvous 대기 진단
./latency.sh <stage_batch_probe_latency exe> 200 plain strace1 hog30
# 검사, 결과
CARGO_TARGET_DIR=... LOCK=... ./run_checks.sh
python3 -I parse_results.py
```
