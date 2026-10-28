# 로컬 재실행 추가 보고




## 재현성 수정 후 최신 HEAD 재실행 — 2026-10-18

- 최신 전달 ref `ac89a9a…`에서 이어받아 runner 직렬화 수정 commit `f522eacb2d433d8628da194fb128cf7a84fc2e1e`에서 실행했다. solver/source 31개 hash와 Cargo.lock hash는 published source manifest와 모두 일치했다. 실행 시 worktree clean, Rust/Cargo 1.94.1, Python 3.12.3, `--locked --offline`였다.
- 처음 지정된 출력 폴더가 과거 산출물을 포함해 runner가 시작 전 exit 2로 거부했다. 과거 자료를 보존하고 고유 출력 경로를 사용했다. 이후 첫 check의 `core_contracts`에서 전역 Leja `OnceLock` 초기화 경쟁으로 1개 테스트가 exit 101이었다. 그 테스트 단독 실행과 같은 계약 묶음의 직렬 실행은 모두 통과했다.
- `run_local.py`는 해당 core Leja 테스트 묶음만 `--test-threads=1`로 실행하도록 조정했다. 수치 코드, 모델, tolerance, `Cargo.toml`, `Cargo.lock`은 바꾸지 않았다. 실패 로그와 수정 후 실행은 `evidence/local_codex_replay_20261018/`에 보존했다.
- clean commit에서 check 7/7, smoke 2/2, bench build/run 2/2가 exit 0이다. check의 allocation positive control이 library clone 3개 Vec 할당을 실제 검출했다. 명시적 warmed `step_into` allocation contract도 통과했으며 적용 범위는 해당 계약의 warmed step이다.
- bench는 32 modes, 11 output points, 3 tolerance 수준, 5 repeats/arm으로 30/30 행 성공했다. 모든 행에서 empirical accuracy gate와 allocator calibration 통과. median structured/dense_fast ns는 각각 `338327/542335` (`rtol=1e-6, atol=1e-9`), `591126/937518` (`1e-8, 1e-11`), `1330387/2416491` (`1e-10, 1e-13`)이다. 두 해상도 RK4는 empirical reference이므로 시간 우위는 `DIAGNOSTIC_ONLY`; measured speed promotion은 계속 HOLD다.
- 전체 production 및 일반 solver 정확도 승격은 HOLD다. 다른 mode 수, 다른 feature 조합, full workspace test campaign, certified accuracy/speed campaign은 실행하지 않았다.
- machine execution identity와 결과 요약: `evidence/local_codex_replay_20261018/EXECUTION_RECORD.json`. 원시 check/smoke/bench stdout·stderr 및 JSON은 같은 경로의 하위 폴더에 있다.

### Postflight provenance checks

- `python3 tools/check-authority-refs.py`: exit 0.
- `python3 tools/check-research-node.py`: exit 1 for historical/generated-output mismatches only. L-0124 has the already known `CONTINUATION_DAG.json` and `CONTINUATION_FINDINGS.json` mismatches. L-0138 expects the report bytes from commit `78ed1c7`; the fetched base `ac89a9a` already contains later report bytes introduced by `161a27a`. The new L-0139 execution row's inputs and outputs hash-verify. No historical ledger row or report was edited to suppress these findings. Raw checker output and exit codes are in `POSTFLIGHT_CHECKS.json` and adjacent stdout/stderr files.
