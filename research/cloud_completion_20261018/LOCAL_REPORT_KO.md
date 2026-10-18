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
