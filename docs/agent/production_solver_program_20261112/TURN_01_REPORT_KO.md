# Turn 1 보고서 — `T01_GLOBAL_ERROR_AUTHORITY` (글로벌 오차 권위 및 matched achieved-error 프로토콜 동결)

- **기준 체크포인트**: `92bf561e11cd3940614aa77a325c375b38957a04` (`research/production-solver-unification-20261112`)
- **공식 G0 상태**: 불변 (`23 nodes / 34 edges`, 판정 `HOLD`) — 본 턴에서 `crates/**`, `tools/**`, `fixtures/**`, `Cargo.toml`, `Cargo.lock` 무수정 (`G0_DOCS_ONLY`).
- **홀드아웃 격리**: `holdout` 4개 문제 및 `n=384`/`n=1536` 문제는 전혀 생성/열람/실행하지 않음 (`NOT_RUN`).

---

## 1. 실행 요약 및 증거 등급

| 항목 | 상태 / 수치 | 증거 등급 | 근거 파일 |
| :--- | :--- | :--- | :--- |
| `rodas5p-fair-ab` 릴리스 계약 테스트 | 11개 타깃 **59 passed / 0 failed / 0 ignored** | `IMPLEMENTATION_VERIFIED` | [REFERENCE_REPLAY_RECEIPT.json](REFERENCE_REPLAY_RECEIPT.json) |
| `ScientificCorpusV2.1` `cal96` 참조 해 6종 생성 | `n=96` 6개 보정 문제 L2 Radau + tight-LSODA 검증 완료 | `NUMERICALLY_CHECKED` | [REFERENCE_REPLAY_RECEIPT.json](REFERENCE_REPLAY_RECEIPT.json) |
| `tools/reference_v2` 런타임 핀 검증 | NumPy/SciPy raw `RECORD`, `scipy.radau`, OpenBLAS 2종 **일치**; `/usr/bin/python3.12` 바이너리 SHA-256 불일치(`e50d468e...` vs `a92f0f95...`) | `UNRESOLVED` (Python patch drift) | [REFERENCE_REPLAY_RECEIPT.json](REFERENCE_REPLAY_RECEIPT.json) |
| `cal96` 18케이스 `v2` / `v3` 구체 리플레이 | `v2`: **18/18 `output-policy-dominated`**; `v3`: **14 pass / 1 `interpolant-dominated` / 3 `global-error-exceeds-budget`** | `NUMERICALLY_CHECKED` | [REFERENCE_REPLAY_RECEIPT.json](REFERENCE_REPLAY_RECEIPT.json) |
| Adaptive / Fixed Pareto Smoke 리플레이 | Adaptive 80 runs(pair 6 admissible / 34 dominated), Fixed 30 runs(182/182 fronts `comparative_reading=forbidden`) | `NUMERICALLY_CHECKED` | [MATCHED_ERROR_PROTOCOL.json](MATCHED_ERROR_PROTOCOL.json) |
| `ppsw1` startup vs per-step 카운터 분리 | startup: `nonlinear_solves=1, iters=2..3, lu=1`; stepping(5스텝): `nonlinear_solves=0, lu=5, linear_solves=30` | `IMPLEMENTATION_VERIFIED` | [REFERENCE_REPLAY_RECEIPT.json](REFERENCE_REPLAY_RECEIPT.json) |
| 메모리 분리 계측 (`VmHWM` vs `stored_state_bytes`) | `cal96` peak RSS `11,172 KiB` (~`77,568 B` 대비 ~147배); `pareto` `6,516 KiB` (`11,632 KiB` 종료); `ppsw1` `4,116 KiB` | `IMPLEMENTATION_VERIFIED` | [REFERENCE_REPLAY_RECEIPT.json](REFERENCE_REPLAY_RECEIPT.json) |

---

## 2. 구체적 수치 발견 및 부정적 대조군 보존 (`H. Negative Controls Preserved`)

1. **`cal96` `v2` vs `v3` 출력 정책 민감도**:
   - 동결된 v2 규칙(`gap <= 0.1 * dense_wrms`)에서는 **18/18 케이스 전부**가 `output-policy-dominated`로 분류되었다. 이는 `clipped` 암이 출력 격자 간격(`0.01`, robertson `0.001`)에 맞춰 ~100스텝 강제 클리핑되는 반면, `dense` 암은 자연 매크로스텝(`12~615`) 후 보간하기 때문이다.
   - `two-arm-output-admissibility-v3`(`B=10.0`, `E+U<=B / E-U>B`, 삼각부등식 무결성, 동일스텝 `interpolant_delta<=2.0`)에서는 **14/18 통과**, **4/18 실패**가 명확히 보존되었다:
     - `robertson-ramped-n96-rtol-1e-6-v2.1`: `interpolant-dominated` (`interpolant_delta = 3.3069 > 2.0`).
     - `semilinear-advection-diffusion-ramped-n96-grid8x12-v2.1`: `rtol=1e-4, 1e-6, 1e-8` 모두 `global-error-exceeds-budget` (`dense_case_wrms`가 각각 `21.37`, `53.22`, `207.46 > 10.0`; `1e-8`에서는 `clipped_case_wrms = 20.13 > 10.0` 동반 초과).
   - 18/18 케이스에서 `v3_attribution_reading = "method-or-order-reduction"` (`r_inner ~ 1.0`), 즉 내부 Krylov 오차가 아니라 외부 시간 이산화/차수 감소가 지배적임을 실측 확인하였다.
   - `replay_t01_runner` 실행은 `VIGILODE_CODE_REVISION` 미설정 상태로 빌드된 `rodas5p-fair-ab`를 사용했으므로 `authority = "synthetic-ci-smoke"`로 기록된다. 정식 보정 동결(Turn 6) 시에는 깨끗한 커밋 리비전을 명시해 `CanonicalV2Runner`로 실행해야 한다.

2. **동일 `rtol`, 동일 스텝, `B=10` 통과가 Matched Achieved Error가 아님을 실증**:
   - `robertson rtol=1e-4`: 동일 허용오차에서 `clipped`는 `max_grid_wrms=16.295` (`33,641` JVP), `dense`는 `4664.68` (`4,690` JVP)로 약 **286배** 오차 차이.
   - `hires-ramped-n96` Dense: `rtol=1e-4` (`dense_case_wrms=0.3922`)와 `rtol=1e-8` (`0.4093`) 모두 `v3` `B=10`을 통과하지만 실제 앵커 스케일 WRMS 오차는 `3922.1` vs `0.4093` (**9,583배** 차이).
   - `Fixed Pareto PR-l-20-m1` 동일 `h=0.04`: `bdf1` (`1.394e-4`) ~ `rodas5p` (`2.796e-11`)로 동일 스텝수에서 **6.7 자릿수** 오차 차이. `bdf1` (`[3.684e-5, 1.394e-4]`)과 `rodas5p` (`[2.465e-14, 2.796e-11]`)는 달성 오차 구간이 전혀 겹치지 않으므로(`NOT_COMPARABLE_NO_OVERLAP`) 외삽 비교가 금지된다. 반면 겹치는 구간 `[5.729e-14, 2.796e-11]` 내 `E* = 1e-12`에서 log-log 보간하면 `rodas5p` ~ `77.22` RHS vs `radau-iia3` ~ `165.85` RHS가 산출된다(단 `radau-iia3-fixed`가 `reference-implementation-only`이므로 `comparative_reading = forbidden` 유지).

3. **`ppsw1` Startup vs Per-Step 분리 및 Rollback 탐침**:
   - `PR-l-1000-m0`/`m1`에서 `ppsw1::start()`의 5스테이지 Radau IIA 뉴턴 초기화(`nonlinear_solves=1, nonlinear_iterations=2..3, direct_factorizations=1, rhs=16..21`)와 `Session::try_step` 5스텝(`nonlinear_solves=0, direct_factorizations=5, linear_solves=30, rhs=30`)을 분리 계측하였다.
   - `h0=0.001, tol=1e-15`에서 실행한 rejection 탐침은 해당 스텝 오차(`error_norm=0.0830 < 1.0`)가 작아 수락(`triggered=false`)되었음을 사실대로 기록하며, 명시적 거부/롤백 계약은 필터링 테스트 `ppsw1_6b_contracts::h8_try_step_rejects_on_estimator_and_commits_on_acceptance` (`1 passed, 0 failed, 36 filtered out`) 통과로 검증됨을 병기한다.

---

## 3. Turn 2 (`T02_PPSW1_6B_QUALIFICATION`) 핸드오프

- 다음 턴에서는 [MATCHED_ERROR_PROTOCOL.json](MATCHED_ERROR_PROTOCOL.json)을 준수하여 **PPSW1-6b 독립 검증**을 수행한다:
  1. 논문(Schmitt & Weiner 2004 SINUM 42(1)) 식/표 기반 계수 독립 재유도 및 종이/코드/오너 선택 분리.
  2. 가변 스텝 차수, stiff/nonnormal Prothero-Robinson, LIN3, step-ratio/`W`-허용오차 스캔, starter 차수 및 `h0` 과도현상 해상도, 고증폭 추정기 가중치/반올림, 거부/클리핑, `t_end < 2 h0`, 이벤트 및 질량행렬 계약 검증.
  3. 반드시 startup 뉴턴 비용과 per-step 무뉴턴 비용을 분리하고, matched achieved-error `[E_min, E_max]` 구간 내에서만 RODAS5P/참조솔버와 비교할 것.
