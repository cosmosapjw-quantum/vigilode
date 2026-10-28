# VigilODE adapter·switcher 프로파일링 감사 packet

이 packet은 `research/adapter-integration-20261027`(`dda8d2f1`)과 후속 노드 L-0172(`f4e971f5`, `381a9cc2`) 이후에 남은 개선 지점을 가벼운 벤치마크와 프로파일링으로 찾아 정리한 외부 감사용 자료다. 결론 목록은 **[ISSUES_KO.md](ISSUES_KO.md)**, 기계 판독 목록은 [ISSUES.json](ISSUES.json)이다.

## 출처와 환경

| 항목 | 값 |
|---|---|
| 기준 소스 | `46939bbec73738585789490b53175be1d100c107` (branch `research/adapter-checkpoint-followup-20261027`), 측정 시 추적 파일 변경 0 (`data_v2/SOURCE.txt`; untracked packet 디렉터리 제외). packet 문서·자료는 그 다음 commit에 추가했다 |
| 라이브러리 변경 | 없음. 이 packet은 진단 example(`crates/vigilode/examples/adapter_checkpoint_trace.rs`의 `bench`, `repeat-arm`, `rodas-fd` arm)과 도구만 추가했다 |
| 바이너리 | `bench_bin` sha256 `593fd146…`(measurement profile), `prof_bin` `c4bbb4f1…`(measurement + `debug=line-tables-only` + `-C force-frame-pointers=yes`, 진단 전용) — `data_v2/BIN_SHA256.txt` |
| Host | AMD Ryzen 9 5900X, 24 logical CPU, `taskset -c 11`, solver thread 1, NCP 없음 |
| Toolchain | Rust 1.94.1, `--locked --offline`, `Cargo.lock` 미변경 |
| 시간 권한 | 모든 벽시계 시간 `diagnostic_only`. 비율·몫은 callgrind Ir과 결정론적 counter를 우선한다 |

## 방법

1. **가벼운 벤치마크** (`tools/light_bench_v2.py`): 10개 fixture에서 선택기 arm과 관련 직접 경로 1–4개만 측정했다. 전수 조합은 하지 않았다. solver 호출만 시간에 넣고, 요약은 시간 밖에서 한 번 만든다. 할당은 cold 첫 호출과 warm 호출을 따로 기록한다. 50 ms 이상 7 batch의 최소·중앙값·spread를 기록하고 spread가 5%를 넘으면 최대 2회 재실행한다.
2. **프로파일링** (`tools/profile_v2.sh`): 29개 cell에 perf(frame-pointer call graph, self/inclusive)와 callgrind(Ir)를, 그중 9개 cell에 heaptrack(n=1과 n=21 차분으로 정상상태 할당)을 적용했다. `data_v2/CALLGRIND_PER_SOLVE.json`은 1회성 계수표 parse(약 0.51M Ir)를 뺀 solve당 Ir이다. n=1 cell 5개는 parse가 annotate 기본 임계값 아래로 잘려 `--threshold=100`으로 다시 찾아 뺐다(`coef_parse_note`). 할당 기울기는 `data_v2/ALLOC_SWEEP.json`(시도 수별 warm 할당)에 있다.
3. **분석·검증 workflow**: 5개 영역(선택기 제어, 비교 경제성, matrix-free 커널, dense·facade 설정, 정확도·거버넌스)을 각 분석 agent가 분석하고, 영역마다 별도 agent가 수치·코드 위치·수정 가능성을 적대적으로 재확인했다. 종합 agent가 중복을 합쳐 30개 항목을 만들고, 완전성 비평 agent가 누락·과장 9건(C1–C9)과 수치·순위 정정 7건(C10–C16)을 지적했다. 원 기록은 `workflow/WORKFLOW_RESULT.json`이다. agent들은 읽기 전용이었고 timing 측정은 하지 않았다. 모두 같은 세션의 Claude subagent이므로 AGENTS.md §4의 이종 모델 독립 감사에는 해당하지 않는다.
4. **재측정과 정정**: 비평이 지적한 harness 결함(switching arm만 JSON 생성이 timed loop 안에 있음, cold 할당)과 출처 결손 때문에 harness v2를 커밋한 뒤 전 cell을 다시 측정했다. 요약표의 Ir·할당·WU·시간 비는 v2로 재확인했다. 구성요소 분해 일부(C3·C5·C6·C7의 Ir, B3·B4의 반사실 replay)는 v1 workflow 값이며 "(v1)"로 표시했다.
5. **출판 전 packet 검증**: 별도 agent 2개(수치, 주장·코드)가 이 문서를 검증해 42건(ERROR 8, MISLEADING 20, 참고 14)을 지적했고 모두 반영했다(`workflow/PACKET_VERIFICATION.json`).

## 주요 측정값 (v2)

| cell | 선택기 / 직접 Ir | 시간 최소값 비 | warm 할당 (선택기 / 직접) |
|---|---:|---:|---:|
| decay adaptive / DOPRI | 1.59 | 1.86 | 78 / 16 |
| rotation adaptive / DOPRI | 1.36 | 1.58 | 478 / 16 |
| gyro-tight adaptive / DOPRI | 1.11 | 1.19 | 1,100 / 16 |
| stiff-forced adaptive / RODAS | 3.33 | 3.56 | 342 / 63 |
| relaxation adaptive / RODAS | 1.58 | 1.65 | 364 / 63 |
| case5 adaptive / MF RODAS | 0.84 | 0.85 | 595 / 189 |
| transition-39 adaptive / MF RODAS | 0.65 | 0.63 | 3,041 / 3,315 |
| stale-return sentinel / adaptive | 0.78 | 0.79 | 429 / 342 |

작은 n의 provider 비교(같은 RHS 함수): case5 MF 4.60M, dense 0.67M, FD-dense 0.74M Ir/solve, gyro-tight MF 2,160M, FD-dense 332M Ir/solve. MF와 dense는 시도·RHS 호출·오차가 같다. FD-dense는 차분 때문에 RHS 호출이 더 많고(case5 409 vs 307, gyro-tight 139,252 vs 75,314), gyro-tight 시도는 9,452 vs 9,454로 근사 동일하다.

## 주의 사항

- **선택기 arm은 opt-in preset이다.** 모두 `SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly)`(evidence arm은 `evidence_driven`)다. 기본 `SwitchingConfig::default()`는 DOPRI 전용(fallback Forbidden)이고 pilot·back-off·재확인을 하지 않으므로 B1·B3·B7·B8·C4·C9는 이 preset에만 해당한다.
- **profile.** 모든 Ir·시간은 lto=false, codegen-units=16인 measurement profile 기준이다. release profile(fat LTO, cgu=1)에서는 호출·inline 고정비(C2·C5·C7·C8)가 줄 수 있으므로 해당 순위는 release에서 재확인해야 한다.
- 등록 fixture는 모두 n ≤ 6이고 RHS가 싸다. 고정 오버헤드의 몫은 큰 n이나 비싼 RHS에서 줄어든다(ISSUES D6).
- v2에서도 34 cell 중 9개가 재실행 후 spread 5%를 넘었다(rotation adaptive 41.7%, relaxation adaptive 26.4%, case5 rodas-fd 23.8% 등). 시간 비는 batch 최소값을 썼고 결론은 Ir과 counter에 의존한다.
- perf inclusive는 frame pointer가 없는 libm·std 경계에서 caller frame을 잃을 수 있다. 몫 판단은 callgrind inclusive를 따른다.
- `prof_bin`은 frame pointer와 line table을 켠 진단 build다. fixture RHS에는 tracing이 꺼진 `rec(t)` atomic load 1회가 남아 약 1%로 보인다.
- callgrind와 heaptrack 수치에는 사용자 fixture callback 비용(`Model::apply`, `sincos`)이 포함된다. 이는 solver 비용이 아니다.
- 로컬 절대 경로(/home/<user>)는 `~`로 바꿨다(수치·내용 변경 없음).
- 모델 라우팅: 분석·검증·종합·비평·출판 전 검증은 이 세션의 Claude subagent가 수행했다. codex(gpt-6-astra/luna)는 사용량 한도로 2026-11-02까지 쓸 수 없었고, Gemini CLI는 snap cgroup 오류로 실행되지 않았다. 따라서 이 packet에 대한 이종 모델 감사는 아직 없으며, codex 한도 해제 후 수행해야 한다.

## 파일

| 경로 | 내용 |
|---|---|
| `ISSUES_KO.md`, `ISSUES.json` | 최종 잔여 문제 목록(31개, 4등급), 해결·반박 기록 |
| `data_v2/LIGHT_BENCH_v2.json`, `light_bench.log` | v2 가벼운 벤치마크 |
| `data_v2/CALLGRIND_PER_SOLVE.json` | cell별 정상상태 Ir/solve와 주요 함수 몫 |
| `data_v2/ALLOC_SWEEP.json` | MF·FD·dense 경로의 시도 수별 warm 할당(A2·A3 근거) |
| `data_v2/perf/`, `data_v2/callgrind/` | perf self/inclusive 표, callgrind 상위 표, 실행 로그 |
| `data_v2/heaptrack/` | 할당 위치 상위 300줄, `STEADY_ALLOCATIONS.txt` |
| `data_v1_superseded/` | 폐기된 v1 자료(사유: `NOTE.md`). workflow 분석의 입력이었다 |
| `workflow/WORKFLOW_RESULT.json` | 영역별 분석·검증, 종합, 비평 원문 |
| `workflow/PACKET_VERIFICATION.json` | 출판 전 문서 검증 42건 원문 |
| `tools/` | v1·v2 benchmark/profile 도구(실행한 그대로) |

무거운 원자료(perf `.data`, callgrind `.out`, heaptrack `.zst`, 바이너리)는 로컬 보관소 `~/vigilode-local-archive/adapter-profile-audit-20261027/`에 `SHA256SUMS.txt`(sha256 `f7c05f17…`, 552개 파일)와 함께 있다.

## 재현

```bash
git checkout 46939bbec73738585789490b53175be1d100c107
CARGO_TARGET_DIR=$T cargo build --profile measurement --locked --offline -p vigilode --example adapter_checkpoint_trace
CARGO_TARGET_DIR=$TP CARGO_PROFILE_MEASUREMENT_DEBUG=line-tables-only RUSTFLAGS="-C force-frame-pointers=yes" \
  cargo build --profile measurement --locked --offline -p vigilode --example adapter_checkpoint_trace
python3 external_reports/adapter_profile_audit_20261027/tools/light_bench_v2.py $T/measurement/examples/adapter_checkpoint_trace OUT/LIGHT_BENCH_v2.json
bash external_reports/adapter_profile_audit_20261027/tools/profile_v2.sh $TP/measurement/examples/adapter_checkpoint_trace OUT/prof
```
