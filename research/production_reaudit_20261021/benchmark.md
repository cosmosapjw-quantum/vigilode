# 최신 벤치마크의 적대적 재감사

판정은 **기존 E1b의 INCONCLUSIVE를 유지**한다. production 비교 실험은 전보다 좋아졌지만, 이 데이터로 전역 오차 보증이나 일반적인 속도 우위를 승격할 수는 없다. 이것은 알고리즘의 불가능성 판정이 아니라, 관측된 오차·측정 환경·reference의 권한을 구별한 판정이다.

## 이번에 확인한 실제 결과

기준 소스는 측정 commit `33e60cf387be58b9cfa341f2d6143a7a28457f42`, 최신 통합 기준은 `f2f20c1d8cd606e70ddbd8409c0f3b4529e928a1`이다. 두 commit 사이의 numerical dependency diff를 직접 읽었다. `crates/vigilode/src/{error,lib,solution}.rs` 변경은 문서와 오류 메시지이며, core/integrator/Krylov 수치 함수의 변경은 없다. 그러므로 원래 실험을 기준선의 empirical evidence로 재사용했다. 이번 감사 수정 이후의 소스에 대한 새 속도 실험으로 부르지는 않는다.

`research/bench_wave3_20261021/results_v2.json`의 SHA-256은 `3e9d063714e9248cf3fffa42a2259d3e6acc4dc7cd278af977175b71527fe36b`이다. 별도의 Python consumer로 row 완전성, 중복, 각 row의 21회 반복, median, callback/allocation 집계, own/common 오차의 정규화 관계, reference 차원, 모든 750개 interpolation record를 재계산했다. **132개 row와 750개 record 모두 일치**했다. 원래 solver 실행을 반복하지 않았다.

- 132개 row 중 131개 성공, DVODE van der Pol 한 row 실패. 실패 row는 보존되어 있다.
- dense row 30개 모두 output-clipped steps = 0. 프로토콜 위반 0.
- dense row 20/30에서 관측된 global grid WRMS error가 1을 초과한다. 최댓값은 Brusselator의 **9.000451844925761**이다.
- dense matched-error level은 130/375이다. 나머지 245개는 원래 규칙대로 diagnostic-only다.
- 21회 반복 모두 같은 status/state/callback을 냈다는 producer 기록은 있지만, 구버전 JSON은 각 반복의 status와 states를 개별 보존하지 않는다. callback은 독립 재계산할 수 있으나 state determinism을 raw 자료만으로 재구축할 수는 없다.
- 측정 host의 높은 부하와 주파수 미고정 때문에 모든 timing authority는 diagnostic-only다. `host.json`의 “no other heavy job” 문구와 별개로 `WAVE3_REPORT.md`가 udev/modprobe 부하를 공개한다. 기존 자료를 수정해서 이 차이를 없애지 않았다.

## 적대적 finding과 수정

| ID | finding | 영향 | 처리 |
|---|---|---|---|
| B-F01 | `max_wrms`의 `atol + rtol |reference|`가 overflow하면 유한한 오차를 0으로 계산할 수 있다 | malformed/extreme input의 거짓 정확도 통과 | finite·positive scale를 강제하고 invalid metric은 `None`으로 처리. `hypot`으로 제곱 overflow를 줄이고, 반대 부호 극값의 normalized difference는 먼저 나누어 계산 |
| B-F02 | reference rtol, crosscheck ladder, budget, dispute threshold가 검증되지 않음. NaN threshold는 비교를 무력화할 수 있고 같은 구현의 다른 arm을 독립 crosscheck로 지정 가능 | 생성된 reference-disputed 판단의 입력 계약 누락 | `reference_policy.rs`로 수치 계약, tightest-first, 독립 implementation family를 검사. duplicate problem IDs/arms, empty corpus, tolerance scaling under/overflow도 거절 |
| B-F03 | 첫 반복 이후 실패 상태·work를 raw repetitions에서 잃음 | 실패 원인·비용 재구성 불가 | 앞으로 모든 Rep에 status/grid_points_completed/work를 보존; aggregate failed_repetitions 추가, determinism에 work equality도 포함 |
| B-F04 | `within_tol_units` 설명이 local interior-control acceptance처럼 읽힘 | global error 누적을 local estimator failure로 오해 | 구 필드와 값은 유지하면서 future JSON에 `observed_global_grid_error_not_local_acceptance` 명시; report 문구 교정 |
| B-F05 | provenance가 실행 시점 HEAD와 bench crate의 dirty 여부만 기록 | 오래된 binary·수정된 path dependency를 깨끗한 소스처럼 기록할 여지 | **미해결**. 다음 build provenance task를 구체화. 기존 measured source가 틀렸다는 주장은 하지 않음 |
| B-F06 | reference는 VigilODE + Diffsol decisive agreement ≤1e-8. parent plan의 independent RADAU5/CVODE ≤1e-10 tier와 다름 | stricter production reference tier를 충족하지 않음 | **미해결**. 원래 preregistration의 판정은 유지, 새 tier는 별도 preregistration으로 수행 |

B-F01 실제 재현: `a=1, reference=f64::MAX, atol=1, rtol=2`에서 수정 전 `Some(0.0)`, 수정 후 `None`. `a=f64::MAX, reference=-f64::MAX, atol=f64::MAX, rtol=0`에서 수정 전 `Some(inf)`, 수정 후 `Some(2.0)`. finite-range 계산 오류를 고친 것이며 기존 132개 정상 입력의 scientific verdict를 뒤집지 않는다. 전자는 표현 불가능한 weight에 대해 metric을 제공하지 않는 계약이다.

## 오차 해석과 다음 accuracy 연구의 구성적 결과

고정된 양의 weight vector `w`에서 grid norm을 `E_w(x)=max_k sqrt(mean_i((x_ki/w_ki)^2))`로 정의하면 삼각 부등식으로

`max(0, E_w(y-r)-δ) ≤ E_w(y-y*) ≤ E_w(y-r)+δ`

가 성립한다. 가정은 **정확해에 대한 reference enclosure** `E_w(r-y*)≤δ`이다. 증명은 `y-y*=(y-r)+(r-y*)`에 삼각 부등식을 적용하고, 역방향은 두 항을 바꾸는 것으로 끝난다. 이는 알려진 norm 성질의 직접 적용이며 새 정리의 우선권을 주장하지 않는다. 중요한 구현 결과는 requested-grid acceptance를 `observed_error + certified_reference_radius ≤ budget`으로 만들 수 있다는 점이다. `δ`를 단순한 두 코드의 불일치로 대체하면 가정이 성립하지 않는다.

현재 cross-code 최대 관측 불일치는 Robertson 1.1638e-10, HIRES 2.1010e-9, van der Pol 1.4614e-9, Brusselator 5.3629e-10이다. 이 값들은 reference error bound가 아니다. HIRES의 dense matched level 하나가 그 관측 spread의 10배 이내여서, 새 실험에서는 reference resolution 대비 matched-error level을 명시해야 한다. 다른 129개 matched level의 간격이 크다는 사실 역시 exact-solution certificate는 아니다.

local estimator와 global grid error를 구분하려면 각 accepted step을 기록한 동일 state/time에서 더 엄격한 독립 solver로 한 step을 재시작하여 local endpoint defect와 dense defect를 분리한다. 이후 defect 전파 bound를 통해 global controller의 예산을 배분한다. 지금의 101-point global metric만으로는 embedded estimate 과소평가와 전역 누적을 분리할 수 없다. 따라서 기존 tolerance를 완화하거나 “2 unit”을 새 성공 문턱으로 바꾸지 않는다.

## 실행과 검증 범위

- Rust 1.94.1로 실제 `metrics.rs`의 5개 unit test와 `reference_policy.rs`의 4개 unit test: **9 PASS**.
- 원본/수정 metric 실제 native 재현: `evidence/benchmark/metric_before_after.log`.
- 독립 raw-data replay: **132 rows / 750 interpolation records PASS**; `evidence/benchmark/audit_retained.py`, `retained_replay.json`.
- full bench cargo test는 최초 offline `diffsol` 부재로 실패했으며 로그를 보존했다. 첨부 17에서 정확한 버전의 29개 dependency를 복원한 뒤에도 `diffsol-la/nl` 0.1.1을 포함한 41개 locked package version이 없어서 metadata부터 실패했다. 대체 버전이나 lock 변경으로 통과시키지 않았다. 그러므로 수정된 전체 bench integration의 Rust typecheck도 미완료다. 별도로 GNU Fortran compiler와 SUNDIALS development dependency도 현 환경에 없다. native comparator linking/execution을 PASS로 보고하지 않는다.
- reference-policy validation은 pure Rust module 자체를 실제 실행한 것이며, 전체 FFI solver의 검증을 대신하지 않는다. `tests/report.rs`의 새 integration contracts는 해당 native 환경에서 실행해야 한다.
- 기존 E1b JSON, report, preregistration, host 자료 및 ledger 판정은 변경하지 않았다.

## 구체적 다음 개발 단계

1. **B-N01, 측정 provenance**: build.rs에서 workspace path dependencies의 실제 source·Cargo locks·`include_str!`/`include_bytes!` 자산·native bridge source를 hash하고 binary에 내장한다. 실행 직전/직후 현재 source hash와 비교하고 실행 binary hash·compiler flags를 보존한다. bench crate 밖 source 수정과 stale binary에서 거절하는 hostile test를 완료한다. 측정 setup 밖에서 수행하여 timing window를 오염시키지 않는다.
2. **B-N02, reference tier**: RADAU5/CVODE를 독립 계산으로 삼는 preregistration을 새로 만들고 refinement ladder와 agreement metric을 고정한다. `1e-10` 목표가 정규화 오차인지 absolute error인지 parent plan에서 명확히 대응한다. exact/certified reference가 필요한 범위는 enclosure를 추가한다. 이전 E1b를 재채점하지 않는다.
3. **B-N03, local/global 분해**: 저장한 accepted-step trace에 independent local solve를 대응하고 endpoint defect, interior defect, flow propagation을 각각 측정한다. 이 결과로 density controller와 global error-per-unit-time controller를 선택한다. 같은 문제·같은 허용오차·같은 mesh를 유지한다.
4. **B-N04, speed 재측정**: source/host/reference가 고정된 상태에서 paired process runs, warmup, randomized arm order와 complete failed work를 포함한다. 두 arm이 achieved-error budget을 통과한 level만 비교하고 confidence interval을 보고한다. 지금의 diagnostic ratios를 일반적인 우위로 승격하지 않는다.
5. **B-N05, 수정 bench native validation**: offline vendor + gfortran + SUNDIALS + BLAS가 있는 로컬 환경에서 `cargo test --manifest-path crates/vigilode-bench/Cargo.toml --locked --offline -- --test-threads=1`를 실행한다. 새 corpus refusal, per-repetition retention, report traceability가 모두 통과해야 한다. 정확한 실행 명령·입력 hash·실패 로그를 반환한다.

## 연구 루프 대응

R1–R3: 최신 E1b 질문/기준/claim의 범위를 고정하고 preregistration과 raw rows를 대조했다. R4–R5: scale overflow·reference 정책·local/global 혼동을 구체적 반례와 norm 식으로 분석했다. R6–R7: source diff, 독립 Python replay, native metric 재현과 9개 Rust 검증을 수행했다. R8–R10: 기존 INCONCLUSIVE를 유지하고 변경 코드, 실행 한계, 다음 acceptance contract를 보존했다.

C1–C4: measured/current 소스 identity, JSON→report 경로, 실패 집계, 오차 계산을 추적했다. C5–C8: 위 네 수정과 adversarial contracts를 구현하고 실행 가능한 module tests/replay를 수행했다. C9: 변경 diff를 root의 독립 검토에 제공했다. C10: machine-readable `benchmark.json`과 로컬 native validation 재개점을 함께 제공한다. full crate linking의 미실행 상태를 폐쇄하지 않는다.
