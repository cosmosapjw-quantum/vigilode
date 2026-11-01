# VigilODE → REI Bianchi IGM | 원본 5개 광자 source 시각을 사용하는 native RODAS5P

2026-11-01. 실제 science owner는 `cosmosapjw-quantum/rei_bianchi`, solver/method owner는 `cosmosapjw-quantum/vigilode`이다. 이 단위는 이전 [VigilODE PR #173](https://github.com/cosmosapjw-quantum/vigilode/pull/173)의 실제 Native 3-moment photoionization-only RODAS5P 코드를 확장한다. 새로운 arbitrary-solver 구현이 아니다. 원래 26-event MPRK22/SDIRK2, frozen 실패 기록과 수용 기준은 변하지 않았다.

## 실제 원본 입력

- REI [PR #332](https://github.com/cosmosapjw-quantum/rei_bianchi/pull/332) 실제 46,080개 공간 노드의 광자 source 및 3 shared neutral sums, 4 photon groups. Actions ZIP SHA-256 `83d94b5383ac7c1f36d89486e83bf8b4a56dd1993a25af3ae1f8fed06a695ba9`.
- REI [PR #342](https://github.com/cosmosapjw-quantum/rei_bianchi/pull/342) 동일 tick160 부모점에서 original source forcing을 다섯 개 proper-time 위치로 평가한 결과. Actions ZIP SHA-256 `88f7e5bb1c6c44c0ca0c7f2ef2c81ac70098cc7ed773096868b1e7b4db25c035`.
- REI [PR #320](https://github.com/cosmosapjw-quantum/rei_bianchi/pull/320) original accepted tick160 H/He/xe/T *구간* 데이터. Actions ZIP SHA-256 `48ad5c6f0922ad946a10a89ce17fefd2c7241ac956b6bac194892a42810312a7`.
- Source parent array SHA-256 `cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01`; original frozen failure tar `a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b`. Original source ZIPs are **not** Git에 복제되지 않으며 전체 로컬 과학 백업에 따로 포함된다.

## 새 과학

원래 original tick은 `h=4.87184874916828e9 s`. 기존 frozen photon current와 비교하여 원본 5개 시각에서 original group current의 **위쪽 구간 경계의 비율**과 frozen-parent 종/외부 absorption weights를 수집했다. 원래 begin의 source current는 PR332의 동일 source 평가를 사용한다. 나머지 시각과 시각 사이에서는 이 current 비율 및 4채널 weights를 **proper-time 선형보간**한다. 이는 명시적인 새로운 forcing 근사이며 원래 REI owner의 연속 forcing과 동일하다는 주장은 하지 않는다.

이 축약계는 H I, He I, He II 중성종의 global population moment 3개와 누적 광자 흡수량 3개, 외부 sink 4개, photon emission 4개, 화학적 결합 에너지 1개, 합계 **15 state**를 진화시킨다. 원래 photon-source 3-moment 경쟁 분모와 정확한 analytic JVP를 그대로 쓴다. 재결합, 충돌 이온화, 26-event OTS photon owner, implicit thermal root, Bianchi geometry는 진화하지 않는다. Chemical binding energy는 NIST ionization thresholds에 의한 이온화 비용이지 gas heating 혹은 radiation escape budget이 아니다.

`SCIENCE_RESULT.json`은 **실제로 재계산한** source-frozen/5-site-forcing 비교이며, Radau와 DOP853 출력을 별도 검증한다. `ORIGINAL_REI_46080_CELL_SOURCE_TIME_RESPONSE.csv`는 원래 46,080-cell photon source spatial weights를 사용한 source-conditional local photoelectron response다. The original accepted state marginal bounds of xHII/HeII/HeIII provide only outer electron/H bounds, not unique correlated parent realizations.

Native Rust `rei_igm_five_clock_source.rs`는 PR #173과 동일한 **기존** `rodas5p-integrators::integrate_sequential_matrix_free_adaptive_dense_observed`를 호출하며, frozen 및 time-varying forcing 두 개를 진짜 native 15-state IVP로 실행한다. Source fixture와 SciPy reference는 먼저 Git에 고정하며, native budget=5e-10은 실행 전에 선언됐다. 성공 판정에는 CI 원본 로그와 native artifact readback이 필요하다.

## 재현

원본 source ZIP 3개를 `/mnt/data` 아래에 해당 이름으로 둔 상태에서:

```bash
python prepare_sources.py
python make_reference.py
python run_science.py
python -m unittest discover -s tests -v
python -O -m unittest discover -s tests -v
python ci_reference.py
```

Git checkout만 있고 원본 source ZIP가 없는 경우 `ci_reference.py`/`test_ci_core.py` 및 native source fixture replay는 가능하지만, 원본 source 전체의 SHA·셀별 provenance를 새로 검증했다고 주장하지 않는다.

VigilODE repository root에서:

```bash
cargo run --locked --release -p rodas5p-integrators --no-default-features --example rei_igm_five_clock_source > science/NATIVE_TIMEFORCED_PHYSICAL.json
cd exploratory/rei_igm_timeforced_photoionization_20261101
python verify_native.py "$GITHUB_WORKSPACE/science/NATIVE_TIMEFORCED_PHYSICAL.json"
```

## 정지 조건과 drift watchdog

새로운 **원본 5-clock forcing 물리 이력과 cellwise photoelectron/binding atlas**가 과학 산출물이며, 9-point atlas 반복이나 일반 최적화가 아니다. `VIGILODE=7/23`, successor `0/12`, `REI_CENTRAL=28/88`, `WORKING=33/88`, canonical delta 0. 중간 26-event 실제 모델과 바꾸어치기 금지. Original tick160→161 역사적 `2.1245050576368385e-4 >2e-4` unchanged; M1 full first interval BLOCKED. REI original production chemistry 권한 없음; NO_MAIN_MERGE, NO_FORCE_PUSH, AUTO_NEXT_NODE=false.

적대적 C9 finding: 원본 다섯 forcing source 값의 inter-knot 구간은 **not owner-validated**, PR342 current upper-bound ratios are not exact central J(t), source-frozen spatial p are not jointly reachable all-X. RODAS5P is not a positivity theorem. Downstream native full-26-event coupled kinetics and original first interval acceptance are NOT_RUN.
