# 로컬 검증 결과 — 2026-10-18

- 저장소/브랜치: `https://github.com/cosmosapjw-quantum/vigilode` / `research/cloud-completion-20261018`
- 검증 source: `4cb795d2b4d3d1cc43ac1ede37d48e4f4daaac3b` (원격 인계 ref와 동기화된 source)
- Cargo.lock SHA-256: `04c7d6c147bf19ebb1a627463705d9dcf27337a3ff228dd1169f19eb73c71400`; source manifest 31개 항목 일치: `True`
- 환경: Rust/Cargo 1.94.1, Python 3.12.3, offline local registry cache. Cargo manifest/lock 수정 없음.

## 실행 판정

| 단계 | 결과 | 주요 실행 |
|---|---|---|
| check | `PASS` (nonallocation commands `True`) | offline all-target compile + core/integrator contracts + fraction oracles + allocation probe |
| smoke | `PASS` (nonallocation commands `True`) | verified packet 16 modes/32 windows + PR101 101-output run |
| bench | `PASS` (nonallocation commands `True`) | measurement build + 32 modes, 5 repeats per arm and tolerance |

## 수치 및 계측

- Allocation positive control: **PASS**. Library model clone의 세 Vec 할당을 검출한 뒤, warmed `step_into` 10회는 추가 allocation 0을 확인했다. 이는 해당 native step 범위의 증거이며 전체 실행/일반 solver에 확대하지 않는다.
- Bench: 30개 행, 30/30 successful; 모든 행에서 empirical accuracy gate와 allocation calibration이 통과했다. 두 해상도 RK4 참조는 empirical이다.
- `rtol=1e-06, atol=1e-09`: structured median `392591 ns`, dense median `906713 ns`, dense/structured `2.310x`; 측정 최대 normalized WRMS `0.0443240665` / `0.0443241635`; empirical gate upper `0.0443240971` / `0.0443241941`. 속도는 DIAGNOSTIC_ONLY.
- `rtol=1e-08, atol=1e-11`: structured median `808557 ns`, dense median `1187002 ns`, dense/structured `1.468x`; 측정 최대 normalized WRMS `0.403531089` / `0.403535594`; empirical gate upper `0.403534149` / `0.403538653`. 속도는 DIAGNOSTIC_ONLY.
- `rtol=1e-10, atol=1e-13`: structured median `1756527 ns`, dense median `2895899 ns`, dense/structured `1.649x`; 측정 최대 normalized WRMS `0.43454907` / `0.435028395`; empirical gate upper `0.434855041` / `0.435334366`. 속도는 DIAGNOSTIC_ONLY.

## 유지한 판정과 남은 항목

- 전체 production 및 일반 solver accuracy 승격: **HOLD**. 이번 작업은 검토된 opt-in scope의 로컬 컴파일/실행 검증이다.
- 측정 speed superiority: **HOLD**; 이 32-mode diagnostic campaign은 동일 정확도 production 속도 주장을 세우지 않는다.
- L-0124의 기존 report hash mismatch는 전달 baseline에도 있던 `INHERITED_UNCHANGED` 이슈다. `evidence/publication_checks.json`을 참조했으며 과거 ledger나 원본 보고서는 수정하지 않았다.
- 이번 실행에서 신규 수치/컴파일 실패 없음. 미실행: 다른 modes, 다른 feature 조합, full workspace test campaign, ledger checker 재실행.

원시 로그 및 JSON: `evidence/local_runs/{check,smoke,bench}/`. 상세 명령·exit code: `executions.json`. Machine DAG: `LOCAL_DAG.json`.

## 최신 HEAD 재실행 — 2026-10-18

- 새 checkout: `412856893c3451917ea19c38ec2965f8790d3a1c` (fetch한 전달 브랜치 tip). 인계 commit `4cb795d…` 이후 게시 source `b237965…` 및 기록 commit이 반영되어 있었다. SOURCE_IDENTITY의 31개 코드·검토 항목 hash가 모두 일치했고 Cargo.lock hash도 기록과 같다.
- Rust/Cargo 1.94.1, Python 3.12.3, `--locked --offline`; `LD_LIBRARY_PATH`는 현 Rust sysroot lib 경로 설정 상태였다. Cargo.toml/Cargo.lock 및 solver 소스 변경은 없다.
- 새 raw evidence는 `evidence/local_codex_rerun_20261018/{check,smoke,bench}/`에 보존했다. check 7개 command, smoke 2개, bench build/run 2개가 모두 exit 0이다. Bench는 30행(각 arm 3 tolerance × 5회), 모두 성공하고 empirical accuracy gate가 통과했다.
- Allocator positive control은 bench 행마다 library clone 3회를 관측해 통과했다. warmed attempt allocation counter는 structured 37, dense_fast 70 calls였다. 이 값은 해당 비교 실행에 한정한다.
- Profile `measurement`, 32 modes, output 11개, 같은 입력/grid/tolerance. 각 5회 median (structured / dense_fast): `rtol=1e-6, atol=1e-9`: 329854 / 527790 ns; `1e-8, 1e-11`: 573487 / 902671 ns; `1e-10, 1e-13`: 1329028 / 2115698 ns. 최대 normalized empirical WRMS는 각 tolerance에서 두 arm 모두 1 미만이다. 독립 RK4 두 해상도는 경험적 기준이며 timing은 DIAGNOSTIC_ONLY, speed promotion HOLD다.
- 전체 production은 계속 HOLD. 이전 L-0124 report-hash mismatch는 이 재실행에서 새 ledger checker를 돌려 고치지 않았으며, 원 과거 자료를 수정하지 않았다. 미실행: 다른 mode 수, 다른 feature 조합, 전체 workspace test, certified accuracy/performance campaign.
- 재실행 machine record: `evidence/local_codex_rerun_20261018/LOCAL_RERUN_20261018.json`.
