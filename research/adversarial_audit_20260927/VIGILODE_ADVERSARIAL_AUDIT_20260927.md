# VigilODE 적대적 감사 보고서와 개선안

**작성일:** 2026-09-28 (KST)
**대상 저장소:** `cosmosapjw-quantum/vigilode`
**감사 대상:** Draft PR #42 head `b3e8165c8dc3b5016702821d280daea1a3f1feb7` (Rust 소스는 PR #40 head `426d37c`와 동일) + 미공개 stage-certificate overlay(정적 검토)
**기계 판독 ledger:** `VIGILODE_ADVERSARIAL_AUDIT_LEDGER_20260927.json` (schema: `VIGILODE_ADVERSARIAL_AUDIT_LEDGER_SCHEMA_20260927.json`)
**원시 증거:** `~/.local/state/vigilode/adversarial-audit/20260927T121000Z/` (Git 밖)

이 보고서의 모든 finding ID(`F-###`), 실험 ID(`E-##`)는 ledger와 1:1로 대응한다. 줄 번호는 감사 대상 ref 기준이다.

---

## 1. 요약

### 1.1 결론

solver의 수치 핵심은 공격을 견뎠다. RODAS5P tableau는 5차 조건 17개를 모두 만족하고(`E-07`), stage 방정식의 W-form과 mass matrix 처리는 정확하며(`E-09`), Krylov solver가 보고하는 residual은 재계산한 true residual과 일치한다(`E-05`). 계수 하나를 1e-8만큼 바꾸면 테스트 10개가 실패한다(`E-10`).

문제는 그 주변에 있다. 확인된 결함은 94개이고 그중 P1이 4개다. P0는 없다.

| 구분 | 수 |
|---|---:|
| 확인된 finding (CONFIRMED) | 94 |
| P1 | 4 |
| P2 | 41 |
| P3 | 49 |
| 반박되어 제외된 finding | 10 |
| 중복으로 병합된 finding | 5 |
| 심어 둔 decoy 중 반박자가 잡아낸 수 | 13 / 13 |
| 실행한 실험 | 10 |

핵심은 네 가지다.

1. **v2 검증 프로토콜이 통과할 수 없는 기준에 걸려 있다 (`F-007`, P1).** clipped/dense 두 궤적의 차이가 dense 오차의 0.1배 이하여야 한다는 기준은 독립적으로 적분한 두 궤적의 오차 벡터가 점별로 약 10% 이내로 일치하기를 요구한다. 커밋된 54행 전부가 실패했고, 같은 코드 경로의 허용 가능한 정책 쌍도 89–94%가 실패하며, SciPy Radau 쌍도 88–94%가 실패한다(`E-01`, `E-03`). 54/54 nonpass는 solver에 대한 정보를 담고 있지 않다.
2. **production inner-forcing rule이 고정-h 수렴을 깨고, 엄격한 tolerance에서 첫 stage를 거부한다 (`F-008` P2, `F-009` P1).** stage residual 목표가 h와 무관한 절대 예산이라 h를 줄여도 오차가 줄지 않는다. 바닥은 Prothero-Robinson에서 약 `0.1–0.3·rtol`(`rtol=1e-8`에서는 최대 `1.3·rtol`), advection-diffusion에서 `0.01–0.8·rtol`이다. `rtol ≤ 1e-8`이고 |λ|≈1e6인 입력에서는 큰 고정 h에서 GMRES를 한 번도 돌리지 않고 step 0에서 오류를 반환한다(`rtol=1e-10`은 시험한 모든 h, `rtol=1e-8`은 h ≥ 1/256; `E-04`). 적응 경로는 h를 줄여 회복하지만 고정-h 사용은 실패한다.
3. **GCRO-DR가 이전 해를 검증 없이 재사용하다 abort한다 (`F-010`, P1).** 차원이 바뀐 시스템에 같은 state를 넘기면 `copy_from_slice`에서 panic이 나고 release 프로파일에서는 프로세스가 종료된다(`E-05`; 반박자의 재실행에서 exit 134).
4. **φ-action의 happy breakdown이 틀린 결과를 오차 0으로 인증한다 (`F-011`, P1).** 임계값 `64·√eps`(≈9.5e-7)가 느슨하고 breakdown 시 `error_estimate = 0`을 보고한다. 이 수치 예시는 Rust 실행이 아니라 numpy 복제본에서 나왔다.

여기에 측정된 사실 하나를 더한다. v2 corpus의 n=96 18행 중 12행에서 dense arm의 전역 오차가 해당 case의 tolerance를 넘고(최대 207배, semilinear advection-diffusion), 같은 tolerance에서 SciPy Radau보다 17/18 행에서 오차가 크다(행별 비 중앙값 5.3배, 최대 438배). solver의 embedded 오차 추정은 36개 arm 모두에서 1 이하였다(`E-02`, `E-03`, `F-004`, `F-033`). 원인은 이 감사에서 확정하지 못했다.

### 1.2 월요일 목록

| # | 조치 | 관련 | 규모 | 해제되는 것 |
|---|---|---|---|---|
| 1 | `rodas5p_inner_forcing_target`의 step-0 오류 반환을 제거하고 residual floor로 대체한다 | `F-009`, `F-031` | S | `rtol ≤ 1e-8` stiff 실행과 고정-h 수렴 ladder |
| 2 | v2 ledger에 추가 기록을 남긴다: 0.1 기준은 대조군 solver도 통과하지 못하며, dense 오차가 12/18 행에서 case tolerance를 넘는다 | `F-007`, `F-004` | XS | 54/54 결과의 오독 방지, 프로토콜 재설계 착수 |
| 3 | GCRO-DR에서 차원·operator·preconditioner identity가 바뀌면 `previous_solution`과 recycle 공간을 초기화하고 typed error를 반환한다 | `F-010` | S | `GcrodrState` 재사용의 안전성 |
| 4 | φ-action에서 증강 열과 Krylov seed를 ‖v‖로 정규화하고, happy breakdown 시 `error_estimate = 0` 대신 residual 추정값을 보고한다 | `F-011`, `F-040` | M | 물리 단위에 무관한 φ 정확도, 지수 경로 인증의 신뢰성 |
| 5 | dirty worktree에만 있는 stage-certificate 바이트를 원격 ref로 올린다 | `F-025` | XS | M0 시작 상태의 재현 가능성 |

sealed v3.7 replay 테스트의 17 vs 18 불일치 bisect(`F-003`)를 포함한 전체 개선안은 7장에 있다.

### 1.3 읽는 순서

3장이 발견, 4장이 가설 판정과 실험, 5장이 공격했으나 정상으로 확인된 항목, 6장이 반박된 발견, 7장이 개선안, 8장이 이 감사의 한계다.

---

## 2. 감사 범위와 방법

### 2.1 감사 대상

| 항목 | 값 |
|---|---|
| 저장소 | `cosmosapjw-quantum/vigilode` |
| 감사 대상 ref | Draft PR #42 head `b3e8165c8dc3b5016702821d280daea1a3f1feb7` (tree `d579f48`) |
| Rust 소스 기준 | PR #40 head `426d37c` 와 Rust 소스 동일 (PR #41/#42는 docs/tools만 변경) |
| 비교 기준 | `origin/main` = `8d0c791` |
| 미공개 overlay | `~/.local/share/vigilode-worktrees/stage-certificate-20260831T141627Z` 의 dirty 바이트(읽기 전용 복사본, 정적 검토만) |
| 규모 | Rust 약 80k LoC, `#[test]` 487개 |

로컬 `main`(`2e83cd1`)은 origin/main보다 88 commit 뒤처져 있고 `.cargo/config.toml`이 존재하지 않는 vendor 디렉토리를 가리켜 빌드되지 않는다. 그래서 감사는 로컬 clone을 새로 만들고 GitHub에서 대상 브랜치를 받아 수행했다. 사용자 저장소의 refs, 작업 트리, dirty worktree는 실행 전후 바이트 동일함을 확인했다.

### 2.2 절차

1. **Baseline**: 대상 ref에서 전체 workspace 테스트(기본 + `audit2-research` + `audit2-bateman-authority`), `clippy -D warnings`, `fmt`, measurement 프로파일 빌드.
2. **매핑**: 5개 shard(core+krylov, integrators A/B, fair-ab+cli+tools, overlay+formal)로 file:line 수준 공격면 지도 작성.
3. **Finder**: 11개 lens(D1–D11)가 독립적으로 결함 후보를 제출. 실험 결과를 받은 뒤 D1/D4, D2는 2차 라운드 수행.
4. **실험**: E-01 ~ E-10. 트리 밖 별도 harness crate에서 measurement 프로파일, `RAYON_NUM_THREADS=1`로 실행. 명령·입력·출력 해시 기록.
5. **반박 검증**: 모든 finding을 파일 경로별 13개 shard로 묶어 반박 전담 에이전트가 공격. `SURVIVES` 판정에는 실행·코드 추적·문서 인용 중 하나의 구체적 artifact가 필수. 각 shard에 고의로 틀린 decoy finding 1개를 섞었다.
6. **상위 집합 재검증**: 상위 11개 finding은 심각도 판정, 기존 인지 여부(provenance) 조사, 독립적인 2차 반박을 추가로 받았다.
7. **개선안**: 6개 그룹별 수정 설계, 로드맵 변경안, 완전성 비평.

### 2.3 심각도 기준 (고정)

| 등급 | 정의 |
|---|---|
| P0 | 공개된 claim/receipt/결과가 거짓이 되거나 실험 결과가 무효가 됨 |
| P1 | 합법적 입력에서 조용히 틀린 수치 결과, abort/panic, 또는 비인과·참조·holdout 정보가 결정에 유입 |
| P2 | 미검증/미지원 경로, 공정성·방법론 약점, 문서-코드 drift, 미집계 작업 |
| P3 | 위생 |

### 2.4 상태 기준

- `CONFIRMED`: 반박 시도가 실패했고 반박자가 구체적 artifact를 제시함.
- `PLAUSIBLE`: 판정 불가이거나 2차 반박자가 반박함.
- `REFUTED`: 반박됨. 사유와 함께 6장과 ledger `refuted[]`에 보존.

---

## 3. 발견

각 블록의 `한정` 항목은 2차 반박에서 좁혀진 주장이나 심각도 이견을 기록한다.

### 3.1 P1 발견

#### F-007 · P1 · D2 · protocol-unattainable — v2 output-policy 허용 규칙(gap ≤ 0.1 × dense 오차)은 사실상 도달 불가
- **주장**: v2 output-policy admissibility 규칙(clipped/dense gap ≤ 0.1 × dense max-grid error)은 독립적으로 step을 밟은 두 궤적의 오차 벡터가 약 10% 이내로 점별 일치할 것을 요구한다. 커밋된 54/54 행에서 실패하고(최소 비율 0.1245), 동일 코드 경로 admissible pair의 89-94%, SciPy Radau pair의 88-94%에서도 실패한다.
- **위치**: `crates/rodas5p-fair-ab/src/global_error.rs:240-284`, `crates/rodas5p-fair-ab/src/global_error.rs:701-720`, `crates/rodas5p-fair-ab/src/scientific_validity_v2_campaign.rs:708-747`
- **증거·재현**: `global_error.rs:715`의 `if output_policy_discrepancy_wrms > 0.1 * dense_max_grid_wrms { Dominated }`에 대해 E-01 요약의 gap/dense_max는 min 0.1245, median 0.944, max 1.094이다. 기대는 올바른 solver가 도달할 수 있는 기준(O(1) × dense 오차 또는 tolerance 대비)이고, 관측은 `c_max/d_max < 0.9`인 행(54개 중 41개 이상)이 하한만으로 Dominated가 되며 가장 좋은 행도 0.125 > 0.1이다.
- **영향**: scientific-validity-v2 gate는 후보 solver가 통과할 수 없으므로 'complete-nonpassing' 결과가 solver에 대한 정보를 담지 않고, 이 행들에서 유도되는 holdout threshold(campaign 1026/1084)도 도달되지 않는다(모든 행이 Pass여야 함, 1012). (관련: H1a, M12)
- **반박 시도**: `exp/E-01/E-01_rows.csv`를 재계산해 삼각부등식 하한 `gap >= d_max - c_max` 위반 0/54, `c_max < 0.9*d_max`인 행 47/54, min gap/d_max = 0.1245를 확인했으며 수학과 데이터 모두 반박되지 않았다.
- **기존 인지**: doc — `research/scientific_validity_v2_20260829/CANONICAL_EXECUTION_EVIDENCE.md:42-43` (비율 범위 0.1245...~1.0941...과 모든 행의 0.1 한계 초과를 기록).
- **수정안** (M, 위험 medium): `global_error.rs:715`의 scale-free 규칙을 case tolerance weight `w = atol_case + rtol_case*|y_ref|`로 평가하는 3부 admissibility 기준으로 교체한다. arm별 reference-aware budget은 reference 불확도 U에 대해 `E_a + U <= B`이면 WithinBudget, `E_a - U > B`이면 ExceedsBudget, 그 사이는 ReferenceDominated로 분류한다. 검증: `gap <= E_c + E_d` property test(한 arm의 grid가 한 index 밀리면 발화)와 세 band 분류표 unit test를 추가한다.
- **한정**: refuter는 'unattainable'을 'clipped-vs-dense 쌍에서 사실상 unattainable'로 좁혔다. E-03에서 h0-pair 2/18, max_step pair 1/18이 규칙을 만족했고 clipped-vs-clipped 쌍은 16/18 만족했으므로, 두 arm이 grid에 고정된 step sequence를 공유하면 규칙은 달성 가능하다.

#### F-009 · P1 · D2 · wrong-formula — roundoff-floor guard가 합법적 stiff 입력에서 stage 0을 중단시킴
- **주장**: `rodas5p_inner_forcing_target`은 `unclamped_eta < 64 eps`이고 `output_weight_l1*tau > 0.1`일 때, 즉 `rhs_wrms > 0.1/(4.9455*64*eps) = 1.42e12`일 때 Err를 반환한다. `rhs_wrms`는 `h*|lambda|*|K|/(rtol*|y|)`처럼 스케일하므로 `|lambda| ~ 1e6`, `rtol <= 1e-8`의 통상적 step size에서 임계를 넘어 fixed-step API는 step 0에서 중단되고 adaptive driver에서는 h*=0.2 rejection으로 바뀐다.
- **위치**: `crates/rodas5p-integrators/src/g4_s5b0_inner_tolerance.rs:64-77`, `crates/rodas5p-integrators/src/g4_s5b0_inner_tolerance.rs:19-19`, `crates/rodas5p-integrators/src/sequential.rs:365-369`
- **증거·재현**: E-04 `table.md` 15-22행(p1 forcing:1e-10, h = 1/8 ~ 1/1024의 8개 모두)이 'failed step 0 t=0: linear solve failed: RODAS5P inner-forcing roundoff floor exceeds the stage-residual heuristic allocation'으로 실패한다. 기대는 stage solve 성공(같은 입력에서 direct arm 오차 1e-15) 또는 수렴한 inexact solve이고, 관측은 P1PR에서 마지막 통과 h(1/64)의 `rhs_wrms` 5.0e11, 실패 h에서 1.4e12 초과이다.
- **영향**: tight tolerance의 stiff 문제에서 protected matrix-free lane의 fixed-step entry point가 합법 입력에 중단되고, adaptive driver는 정확도 실패가 아닌 기준으로 h를 시도당 5배 줄여(`adaptive.rs:291`) fair-ab harness가 solver failure로 보고하는 `rejected_steps`/`linear_solve_failures`가 부풀려진다. (관련: H2, M07, M09)
- **반박 시도**: prebuilt harness `e04_order_krylov p1 3 4 forcing:1e-10`에서 h=1/8, 1/16 모두 status 'failed', `lin_it=0`, `steps_completed=0`으로 재현되었고 guard 대수도 코드 읽기로 확인되어 반박에 실패했다.
- **기존 인지**: 없음 (`crates/rodas5p-integrators/src/g4_s5b0_inner_tolerance.rs:73`의 오류 메시지만 존재).
- **수정안** (S, 위험 low): 정확도 실패가 아닌 이 기준으로 Err를 반환하는 분기를 고친다(입력의 요약문은 중간에서 잘려 있음). 검증 항목 기준으로는 `rodas5p_inner_forcing_target_v3(flow=1e10, rhs=1e13, l1=4.9455, h=1/8, None)`이 Ok와 `tau = 64 eps * 1e13 = 0.142`, `floor_active=true`를 반환해야 한다. 검증: 현재 test가 없는 Err 분기에 위 unit test를 추가하고 `e04_order_krylov`를 rebuild해 재실행한다.
- **한정**: mechanism 한 가지가 과장되었다. stage 0에서는 `gmix`가 0이므로(`sequential.rs:350-354`) `h*J*gmix` 팽창은 적용되지 않으며, P1의 step-0 중단은 f 자체가 manifold 밖에서 O(lambda)인 데서 온다.

#### F-010 · P1 · D3 · panic — GCRO-DR가 차원 변경 후 previous_solution을 재사용해 panic
- **주장**: n=32 system에서 수렴한 `GcrodrState`로 n=16 operator(`x0=None`)에 `solve_gcrodr`를 호출하면 Err 반환 대신 `gcrodr.rs:463`에서 'copy_from_slice: source slice length (32) does not match destination slice length (16)'으로 process가 abort된다.
- **위치**: `crates/rodas5p-krylov/src/gcrodr.rs:416-423`, `crates/rodas5p-krylov/src/gcrodr.rs:462-464`, `crates/rodas5p-krylov/src/lgmres.rs:68-78`
- **증거·재현**: E-05 `e05_raw.json` children[0]이 `signal_or_abort=true`와 위 panic 메시지를 기록한다(`harness/src/bin/e05_krylov_stress.rs:343-358`). 기대는 `Err(CoreError::Dimension)` 또는 LGMRES(`lgmres.rs:75`)와 같은 warm-start reset이고, 관측은 panic이며 LGMRES 동일 시나리오는 144 iteration으로 Ok를 반환한다.
- **영향**: 크기가 다른 문제에 `GcrodrState`를 재사용하는 library 사용자(fair-ab SolverSession, custom driver)는 crash하며, 같은 n에서 operator가 바뀌어도 다른 선형계의 stale `previous_solution`이 warm start로 쓰인다(정확성은 certificate가 보호, 비용은 영향). (관련: M08)
- **반박 시도**: `e05_krylov_stress --child-dimchange gcrodr`가 exit 134(SIGABRT)로 재현되었고, 차원 간 재사용을 금지하는 문서화된 contract, `validate_system`의 guard(`x0`만 검사), in-tree caller(integrator는 n 고정)를 찾는 세 공격 모두 library API panic을 반박하지 못했다.
- **기존 인지**: 없음 (sibling solver인 `crates/rodas5p-krylov/src/lgmres.rs:69-77`에만 reset이 있음, ab8fbcd에서 도입).
- **수정안** (S, 위험 low): `gcrodr.rs:420`은 recycle basis만 `len()==n`으로 거르고 `previous_solution`은 검사하지 않으므로, LGMRES 규칙(`lgmres.rs:68-78`)을 따라 system identity가 None이거나 `state.system_identity`와 다르면 `previous_solution`(및 images)을 버린다. 검증: `krylov_contracts.rs`에 `gcrodr_dimension_change_returns_dimension_error_and_preserves_state` test를 추가해 E-05 child_dimchange 시나리오(n=32 후 n=16)를 고정한다.

#### F-011 · P1 · D4 · wrong-formula — phi-action happy breakdown이 error_estimate 0으로 결과를 인증
- **주장**: `v = e1 + 4.5e-7*e2`, `A = diag(-1,-3)`에서 fused/legacy Arnoldi 규칙 `h_{m+1,m} <= 64*sqrt(eps)*max(1,|h_im|)`이 m=1에서 발화해 `converged=true`, `error_estimate=0.0`을 반환하지만 `exp(A)v`의 실제 상대오차는 3.9e-7이다(KIOPS estimator는 5.7e-7). 따라서 약 1e-6 미만의 `relative_tolerance`는 조용히 강제되지 않는다.
- **위치**: `crates/rodas5p-integrators/src/exponential.rs:1078-1099`, `crates/rodas5p-integrators/src/exponential.rs:1136-1150`, `crates/rodas5p-integrators/src/exponential.rs:881-911`
- **증거·재현**: `exp/D4/happy_breakdown.py`의 출력과 `exponential.rs:1136-1150`의 `error_estimate: if happy_breakdown || full_space_exact { 0.0 }`이 근거이다. 기대는 rtol 1e-10에서 보고 오차 >= 실제 오차 또는 수렴 미선언이고, 관측은 delta=4.5e-7 scale=1에서 reported_error=0, TRUE_rel_err=3.9e-7, delta=1e-7 scale=3에서 실제 1.0e-7, 보고 0이다.
- **영향**: `adaptive_exponential`의 `phi_error_proxy`(`adaptive_exponential.rs:83-94, 264-265`)가 0을 받아 `total_error`가 time_error만으로 결정되므로, 거의 분리된 stiff mode 등 invariant subspace 근처에서 tight tolerance의 exponential lane 결과가 제어되지 않은 Krylov 오차와 함께 수락될 수 있다.
- **반박 시도**: numpy/scipy로 one-pass MGS Arnoldi를 독립 재현(`refute/scratch_05/f011_check.py`)한 결과 h21=9.000e-07 <= 9.537e-07로 happy=True, 상대오차 3.891e-07이 그대로 나왔고, 이를 막는 caller, test, 문서를 찾지 못했다.
- **기존 인지**: 없음 (`research/scientific_validity_v2_20260829/CLAIM_SCOPE_AND_INVALIDATION.md:57`은 scale-invariant Krylov breakdown/certification을 IMPLEMENTED로 기록).
- **수정안** (M, 위험 medium): 세 Arnoldi loop(`exponential.rs:881,905-911; 1078,1094-1099; 1563-1564`)가 같은 규칙으로 happy breakdown을 선언한 뒤 이미 계산된 residual을 0으로 덮어쓰므로(`exponential.rs:1136-1150`), residual estimate를 덮어쓰지 않도록 한다(요약문의 나머지 항목은 입력에서 잘려 있음). 검증: `exp/D4/happy_breakdown.py`를 Rust test로 옮겨 위 사례(rtol=1e-10)에서 `converged=false` 또는 `error_estimate >= 3.9e-7`을 요구한다.
- **한정**: `max(1,.)` floor 때문에 `||scale*H|| < 1`이면 규칙은 더 느슨해진다. 영향 lane은 guarded exponential candidate(exprb32/43, pexprb54s4 fused/prefix)이며 protected RODAS5P solver는 아니다.

### 3.2 P2 발견 — 수치 핵심 (D1–D4)

#### F-002 · P2 · D1 · unverified-assumption — RODAS5P dense output 내부 오차는 tolerance 제어를 받지 않음
- **주장**: RODAS5P dense output의 내부(보간) 오차에는 tolerance 제어가 없다. 수락은 endpoint embedded estimate로만 결정되고, stiff 문제에서 interpolant 정확도를 측정하는 in-tree test가 없다.
- **위치**: `crates/rodas5p-integrators/src/dense_output_v2.rs:63-114`, `crates/rodas5p-integrators/src/dense_output_v2.rs:440-458`, `crates/rodas5p-integrators/src/output.rs:337-373`
- **증거·재현**: `exp/D1/dense_stiff_check.py`에서 PR lam=-1e5의 interior max WRMS는 rtol 1e-4/1e-6/1e-8에서 17.5 / 610 / 206이고 endpoint max WRMS는 0.002/0.095/0.136이며, step별 interior/endpoint 비는 최대 7.5e3이다. 기대는 dense arm grid 값이 tolerance 제어될 경우 interior WRMS <= O(1)이다.
- **영향**: dense arm의 모든 소비자(fair-ab dense grid error, E-01 `dense_max_wrms`, H1b)는 solver 오차가 아닌 interpolant 오차를 측정하며, interpolant 변환 자체는 올바르므로(nonstiff order 4) 이는 잘못된 공식이 아니라 누락된 제어이다. (관련: H1b, H1a, M12)
- **반박 시도**: `README.md:36-44`와 `CLAIM_SCOPE_AND_INVALIDATION.md:41,63`이 54행 전부 output-policy-dominated이며 v2 claim을 인정하지 않는다고 명시한 점으로 공격했으나, dense arm grid 값이 interpolant 지배적이고 `dense_output_v2.rs`에 caveat이 없다는 핵심 관찰은 남았다.
- **기존 인지**: doc — `research/audit2_two_loop_20260829/README.md:7` (nonstiff 설정의 quartic continuous output local defect O(h^5)를 기술하고 stiff limit은 별개라고 명시).
- **수정안** (L, 위험 medium): `DenseErrorControl {Off, Report, Enforce}`를 추가해, 요청 output time을 포함하는 accepted step마다 theta=0.5에서 interpolant defect를 평가하고(rhs 1회 추가) 이미 분해된 W로 필터링한(back-solve 1회) WRMS를 interior estimate로 삼는다. Report가 기본값으로 값만 기록하고 Enforce는 opt-in 제어이다. 검증: 새 test `adaptive_dense_interior_wrms_on_stiff_pr`(PR lambda=-1e5, rtol {1e-4,1e-6,1e-8})에서 Report mode의 기록값이 실제 interior WRMS의 factor 10 이내임을 확인한다.
- **한정**: refuter는 claim이 endpoint order를 과장했고 tree가 dense arm에 의존하는 claim을 이미 모두 보류하고 있다고 지적했다. wrong formula가 아닌 methodology/documentation gap이며, 근거는 Python replica뿐이다.

#### F-018 · P2 · D1 · untested-path — 'fifth-order' gate는 stiff 문제에서 order를 측정하지 않음
- **주장**: slope >= 4.8 기준(`homotopy_order_policy.rs:533-545`)은 problem_id 'manufactured-vector-order'에만 적용되고, stiff trajectory는 control 대비 5x regression check만 받는다(547-562). 따라서 README의 'fifth-order recovery' gate(`README.md:19`, `unified_gates.rs:610-612`)는 nonstiff 전용 증거이며, E-04/E-09가 측정한 direct-LU order는 4.0(P1PR), 2.95-3.25(PR lambda=-1e4), 3.2-4.8(diagmass eps=1e-3)이다.
- **위치**: `crates/rodas5p-integrators/src/homotopy_order_policy.rs:531-548`, `crates/rodas5p-integrators/src/unified_gates.rs:610-612`, `crates/rodas5p-integrators/src/adaptive.rs:27-33`
- **증거·재현**: E-04 `results.json`의 p1pr direct slopes_max는 [4.01,4.03,4.03,3.63,3.42]이고 `consecutive_halvings_ge4.5=0`, E-09의 PR lambda=-1e4 analytic_ft slope는 [2.95,3.03,3.11,3.25]이며, E-07은 17개 classical order-5 조건이 모두 성립하고 P1NS의 관측 order가 5.4-5.9임을 보인다. 기대는 stiff ladder에서 slope >= 4.5인 연속 halving 2회 이상과 이를 assert하는 gate이고, 관측은 그런 gate가 없고 stiff order가 3-4이다.
- **영향**: ledger/README의 'fifth-order recovery', 'global fifth-order gate' 진술은 nonstiff manufactured problem에서만 인증되며, stiff 문제에서는 exact linear algebra에서도 order 3-4로 동작한다(classical ROW/W-method order reduction). (관련: H2, M09)
- **반박 시도**: `native_gates.rs:122-155`와 `unified_gates.rs:361-416`에도 observed-order 계산이 있음을 찾았으나 모두 nonstiff 문제여서 핵심 주장을 반박하지 못하고 오히려 보강했다.
- **기존 인지**: 없음
- **수정안** (M, 위험 low): outer tolerance가 h와 독립인 fixed-h order test 파일(`stiff_order_contracts.rs`)을 추가하고, 기존 h^6 결합 test를 `forced_fixed_step_with_h6_coupled_rtol_retains_order_five`로 rename하며 'Defect caught' 주석을 정정한다. 검증: fixed h = 1/2^k, slope = log2(e_k/e_{k+1}), error > 1e-12인 행만 pre-floor로 세는 test plan(T1 `direct_fixed_step_nonstiff_order_is_five` 등)을 실행한다.
- **한정**: 'tree 내 유일한 observed-order pass criterion'이라는 표현은 과장이다. 다른 두 gate도 observed order를 계산하지만 대상이 nonstiff 문제이다.

#### F-006 · P2 · D2 · wrong-formula — clipped 수락 시 controller가 requested step을 동결
- **주장**: output-clipped step이 수락되면 controller는 requested step을 그대로 반환한다(`adaptive.rs:279-282`). v2 campaign은 uniform 101-point grid의 모든 점에서 hard-stop하므로 clipped arm은 사실상 fixed-step h = span/100 integrator이다.
- **위치**: `crates/rodas5p-integrators/src/adaptive.rs:269-293`, `crates/rodas5p-integrators/src/integrate.rs:757-761`, `crates/rodas5p-integrators/src/integrate.rs:861-869`
- **증거·재현**: `exp/E-01/E-01_rows.csv`에서 26/54 행이 `(c_rej,c_clip,c_int)=(0,98,100)`이고 54행 모두 `c_clip`이 {98,99}에 속한다. 기대는 `max_step=span`, rtol 1e-4..1e-8에서 tolerance 크기의 step을 밟거나 clipped 수락 후 `requested_h`가 커지는 것이고, 관측은 clipped arm이 tolerance를 사용하지 않는 것이다(`clipped_max/dense_max` median 0.211).
- **영향**: clipped(output-observed) lane의 모든 소비자(`integrate_adaptive_observed*`, `integrate_homotopy_adaptive_observed`, `integrate_sequential_matrix_free_adaptive_observed`, transactional lane)가 해당되며, v2 campaign의 clipped arm은 adaptive run이 아니라 grid 간격의 fixed-step run이다. (관련: H1a)
- **반박 시도**: `adaptive.rs:279-282`, `integrate.rs:757-761`, `:861-869`의 trace를 단계별로 재확인했으나 모두 성립했고, 정확히 100 internal / 98 clipped step인 zero-rejection 26행이 그 fingerprint이다.
- **기존 인지**: doc — `crates/rodas5p-integrators/src/adaptive.rs:264-266` (forced output landing은 scheduling artifact이며 clipped trial 성공 시 unclipped request를 복원하고 PI history를 건드리지 않는다는 주석).
- **수정안** (M, 위험 high): pre-clip h를 기억하는 의도는 유지하되 sample이 이를 갱신하게 한다. `candidate = trial_h * factor(error)`를 계산하고 sample이 informative(`trial_h/requested_h >= 0.5`)이면 PI history에 기록한다. 검증: `adaptive_controller_contracts.rs:69-84`는 bit-for-bit로 유지되어야 하고 `dense_output_v2_contracts.rs:343-360`은 재작성해야 한다.

#### F-008 · P2 · D2 · wrong-formula — inner-forcing target tau = 0.1/||b||_1은 h와 무관한 절대 예산
- **주장**: `rhs_wrms >= flow_wrms`이면 `tau = eta*rhs_wrms = 0.1/output_weight_l1`이 되며(`g4_s5b0_inner_tolerance.rs:58-66`), 이는 모든 stiff stage에서 성립하므로 stage당 허용 residual은 모든 h와 rtol에서 outer-WRMS 단위 상수 0.02022(=0.1/4.9455)이다. 따라서 step당 inexact-solve 오차는 h와 무관하게 O(0.1*tol)이고 production step의 fixed-h 오차는 약 0.1 tol 아래로 내려가지 못한다.
- **위치**: `crates/rodas5p-integrators/src/g4_s5b0_inner_tolerance.rs:58-66`, `crates/rodas5p-integrators/src/g4_s5b0_inner_tolerance.rs:18-20`, `crates/rodas5p-integrators/src/sequential.rs:359-369`
- **증거·재현**: E-04 `table.md`에서 `tau_max = 0.02022028690921782`가 h=1/8..1/1024와 rtol 1e-4..1e-10 전부에서 동일하고, P1PR floor는 forcing:1e-4에서 1.20e-05(0.12 rtol), 1e-6에서 9.96e-08(0.10 rtol), 1e-8에서 1.98e-09(0.20 rtol)이며 7회 halving 동안 slope는 약 0(-0.43..1.15)이다. 기대는 허용 stage residual이 local truncation error(O(h^5)) 또는 최소한 h처럼 스케일하는 것이다.
- **영향**: `integrate_sequential_matrix_free_adaptive_observed`의 모든 소비자(v2 campaign dense/clipped arm, CLI protected lane)에서 정확도가 step size와 무관하게 약 0.1-0.3 tol에서 포화되고, production step의 fixed-h 수렴 연구는 order 약 0을 보인다. (관련: H2, M04, M05, M07, M09)
- **반박 시도**: `inner_forcing_v2_contracts.rs:104-107,127-139`의 h^6 결합 설명과 문서화된 설계라는 점으로 공격했으나, 이 tree에서 claim은 문자 그대로 참이고 E-04 데이터가 상수 tau와 slope 약 0을 확인한다.
- **기존 인지**: nonclaim — `research/scientific_validity_v2_20260829/PROTOCOL.md:147-148` (WRMS inner policy는 per-stage true-residual heuristic이며 `0.1` 할당은 `W^-1`의 독립 bound 없이는 endpoint contamination을 bound하지 않는다고 기술).
- **수정안** (M, 위험 medium): residual target은 outer-WRMS 절대 단위로 유지하여(GMRES는 `gmres.rs:210`의 `max(atol, rtol*||rhs||)=tau`에서 정지) F-031의 K-form rhs 팽창이 들어오지 않게 하되, 상수 cap 대신 reference error에 따라 줄어드는 target으로 바꾼다. 검증: `rodas5p_inner_forcing_target_v3`가 `err_ref=1e-3`, `h=h_ref`에서 `tau = cap*1e-3^1.2 = 5.1e-6`(cap=0.02022), reference None에서 cap을 반환하고 `err_ref`와 h에 대해 monotone non-increasing임을 unit test로 확인한 뒤 E-04 harness를 rebuild한다.

#### F-029 · P2 · D2 · scale-dependence — v2 평가 WRMS는 고정 (1e-10, 1e-8) basis라 tolerance 정규화가 아님
- **주장**: v2 campaign 행 metric `max_grid_wrms`는 case rtol(1e-4/1e-6/1e-8)과 무관하게 weight `1e-10 + 1e-8*|y_ref|`를 쓰는 반면, solver의 수락 test는 `atol_case + rtol_case*max(|y_old|,|y_new|)`를 쓴다. 따라서 rtol 1e-6에서 보고된 'WRMS 310'은 3.1 tolerance unit이고, E-01 headline 'median 310, >1 in 48/54'는 tolerance 초과를 과대 표시한다(재정규화: median 2.15, >1 in 36/54).
- **위치**: `crates/rodas5p-fair-ab/src/numerical_reference.rs:763-764`, `crates/rodas5p-fair-ab/src/numerical_reference.rs:811-815`, `crates/rodas5p-fair-ab/src/global_error.rs:117-127`
- **증거·재현**: `exp/D2/renormalized_e01.txt`의 `d_max*1e-8/rtol` 열은 median 2.15, q25 0.72, q75 4.2, n>1 36/54이며, E-01에서 hires-ramped-n96 rtol 1e-4의 `d_max` 3923(basis unit)은 0.39 tol unit이다. 기대는 case tolerance를 쓰는 metric 또는 단위를 명시한 ledger이고, 관측은 54행 최대값인 `conservative_threshold_wrms`가 basis-unit 값이 1e4배 부풀려진 rtol=1e-4 행(semilinear 3.8e5)에 지배되는 것이다.
- **영향**: `calibration_all_cases_compact.json`이나 E-01 요약을 tolerance 초과로 읽는 독자가 오해하게 되고, calibration 최대값으로 정한 holdout pass threshold는 rtol arm 간에 비교 불가능한 행을 섞는다. (관련: H1b)
- **반박 시도**: `E-01_rows.csv`를 재계산해 `d_max*1e-8/rtol` median 2.1506, >1 36/54, basis-unit median 310.42를 그대로 얻었고 수치는 반박되지 않았다.
- **기존 인지**: doc — `scientific_validity_v2_campaign.rs:257`과 `numerical_reference.rs:761-764`가 고정 1e-10/1e-8 basis를 선언하고, `PROTOCOL.md:56-63`이 tolerance unit을 주장하지 않은 채 max-over-rows 유도를 문서화.
- **수정안** (M, 위험 medium): F-007과 동일한 수정안이다. `global_error.rs:715`의 규칙을 case tolerance weight `w = atol_case + rtol_case*|y_ref|`로 평가하는 3부 admissibility 기준으로 교체한다. 검증: `gap <= E_c + E_d` property test와 세 band 분류표 unit test를 추가한다.
- **한정**: 고정 tight basis는 의도된 것이고 policy id에 문서화되어 있으므로 'doc-drift' 부분은 tree가 아니라 감사의 FINDER_BRIEF('1.0 == exactly at tolerance')에 해당한다. 실질적으로 남는 것은 fixed basis의 max-over-54-rows 단일 threshold가 rtol=1e-4 행에 지배된다는 점이다.

#### F-030 · P2 · D2 · untested-path — forcing rule order 보존 test가 rtol = h^6에 묶여 floor를 검출하지 못함
- **주장**: `forced_fixed_step_refinement_retains_order_five_before_roundoff`(`inner_forcing_v2_contracts.rs:128-140`)는 'Defect caught: an h-independent inner residual tolerance creates a global error floor'라고 적고 있으나, `forced_fixed_endpoint`가 `rtol = step^6`, `atol = 1e-2*rtol`(107-108행)로 설정하므로 상수 tau(0.02 outer-WRMS unit)가 절대 단위로 h^6처럼 줄어든다. outer tolerance를 고정한 production 상황에서는 같은 step이 7회 halving 동안 slope 약 0을 보인다(E-04).
- **위치**: `crates/rodas5p-integrators/tests/inner_forcing_v2_contracts.rs:104-108`, `crates/rodas5p-integrators/tests/inner_forcing_v2_contracts.rs:128-140`, `crates/rodas5p-integrators/tests/inner_forcing_v2_contracts.rs:93-93`
- **증거·재현**: E-04 `README.md` slope table에서 p1pr forcing:1e-4/1e-6/1e-8의 fixed rtol slope는 -0.43..1.15, floor는 0.10-0.32 rtol이며, `baseline/test_results_summary.txt`의 1074 passed(`--ignored` 실행 제외)에 이 test가 포함된다. 기대는 'retains order five' test가 outer tolerance를 고정하고도 slope >= 4.8을 관측하는 것이고, 관측은 fixed rtol에서 오차가 h = 1/8..1/1024에 걸쳐 1.0e-7..3.2e-7로 평탄한 것이다.
- **영향**: test suite가 production 설정에 없는 성질(production inner-forcing rule 하의 order 유지)을 인증하므로 1075-passing baseline에 의존하는 reviewer는 F-008의 floor를 놓치며, 사용된 문제(stiffness 20, n=8)도 `rhs_wrms >> flow_wrms`인 stiff 영역과 거리가 멀다. (관련: H2, M09)
- **반박 시도**: test 자체 주석(104-106행)이 h^6 설계와 adaptive API의 차이를 공개적으로 밝힌다는 점을 완화 요인으로 들었으나, `rtol=h^6`에서는 tolerance 비례 inner forcing이 모두 h^6로 스케일해 floor가 보일 수 없으므로 test가 판별력을 갖지 못한다는 주장은 그대로 성립했다.
- **기존 인지**: doc — `inner_forcing_v2_contracts.rs:104-106` inline 주석이 h^6 rtol 결합을 의도된 test 설계로 인정.
- **수정안** (M, 위험 low): F-018과 동일한 수정안이다. outer tolerance가 h와 독립인 fixed-h order test 파일을 추가하고 기존 test를 `forced_fixed_step_with_h6_coupled_rtol_retains_order_five`로 rename하며 'Defect caught' 주석을 정정한다. 검증: fixed h = 1/2^k, error > 1e-12인 행만 pre-floor로 세는 test plan(T1 `direct_fixed_step_nonstiff_order_is_five` 등)을 실행한다.

#### F-031 · P2 · D2 · scale-dependence — K-form stage 식의 rhs norm이 O(h|lambda|)만큼 팽창
- **주장**: `rhs_i = h f_i + h J (sum_j gamma_ij K_j) + h^2 gamma_i f_t`(`sequential.rs:359-364`)와 `W = I - h gamma J`에서 stiff mode의 rhs norm은 `K_i = W^-1 rhs_i` norm의 약 `|h lambda|`배이므로, 상대 residual eta는 non-stiff 방향에서 최대 `eta*|h lambda_max|*|K|`의 절대 stage 오차를 낸다. P1/P1PR에서 `rhs_wrms`는 1e9-1e12 outer-WRMS unit에 이르며, 이 양이 F-008의 상수 tau와 F-009의 abort를 모두 유발한다.
- **위치**: `crates/rodas5p-integrators/src/sequential.rs:350-364`, `crates/rodas5p-integrators/src/sequential.rs:355-358`, `crates/rodas5p-integrators/src/sequential.rs:366-369`
- **증거·재현**: E-04 `p1pr_forcing_1e-6.jsonl` k=3에서 `flow_wrms_mean` 5.17e7, `rhs_wrms_mean` 2.05e9(h=1/8에서 flow의 40배)이고 p1pr forcing:1e-10 k=6에서 `rhs_wrms` 5.03e11이며, fixed-eta GMRES 오차항은 `C_eta*eta`(C_eta ~ 1e2-1e3)로 eta=1e-12에서도 h*=1/32에 direct와 crossover한다. 기대는 rhs norm이 stage increment와 비슷해 상대 residual이 비슷한 상대 stage 오차로 대응되는 것이다.
- **영향**: forcing rule이 stage 오차를 대표하지 않는 norm에 대해 calibrate되어 있다는 점이 F-008과 F-009의 설계상 근본 원인이며, U-form 대비 stage당 JVP 1회가 더 들어(`jvp_calls`에 계산됨) matrix-free 비용 비교가 RODAS5P에 step당 약 7 JVP만큼 불리하게 치우친다. (관련: H2, M09)
- **반박 시도**: `sequential.rs:350-364`와 `g4_s5b0_inner_tolerance.rs:59-66`을 다시 읽고 E-04 jsonl을 재파싱해(39.8x) 모든 문자적 주장을 확인했으며, production rule에서는 tau가 outer WRMS 상수 `0.1/||b||_1`로 붕괴한다는 부분적 반론만 남았다.
- **기존 인지**: doc — `g4_s5b0_inner_tolerance.rs:23-29`의 `RODAS5P_INNER_FORCING_CLAIM_SCOPE = StageResidualHeuristicRequiresResolventBound`와 `scientific_validity_v2_campaign.rs:183-186`의 `inner_solve_claim_scope`('residual-only; endpoint contamination requires an independent W-inverse resolvent certificate').
- **수정안** (M, 위험 medium): F-008과 동일한 수정안이다. residual target을 outer-WRMS 절대 단위로 유지해 K-form rhs 팽창이 target에 들어오지 않게 한다. 검증: `rodas5p_inner_forcing_target_v3` unit test(`tau = cap*1e-3^1.2 = 5.1e-6`, cap=0.02022)와 E-04 harness rebuild로 확인한다.

#### F-033 · P2 · D2 · unverified-assumption — semilinear 행이 tolerance의 17-261배 밖인데 embedded estimator는 수락
- **주장**: semilinear-advection-diffusion-ramped family에서 case tolerance로 재정규화한 dense arm max-grid error는 17-261이다(rtol 1e-8: n=96/384/1536에서 207/261/169, accepted step 60-66개). 이는 약 60 step 동안 1 tolerance unit 이하의 local error가 누적된 것으로는 설명되지 않으므로, solver가 제어하는 양(같은 inexact stage의 `btilde*K` WRMS, `sequential.rs:583-585`)이 평가가 측정하는 양(tight Radau 대비 global error)을 bound하지 못한다.
- **위치**: `crates/rodas5p-integrators/src/sequential.rs:583-586`, `crates/rodas5p-integrators/src/sequential.rs:366-370`, `crates/rodas5p-integrators/src/sequential.rs:507-513`
- **증거·재현**: `exp/E-01/E-01_rows.csv` 재정규화 값에서 다른 다섯 family의 dense max는 hires 0.39-0.96, robertson 0.27-3.7, van-der-pol 0.29-2.55, rotating 1.4-4.5, nonautonomous 2.1-4.25로 global-vs-local 초과로 설명 가능한 범위이다. 기대는 tolerance의 작은 배수 이내의 global error 또는 초과를 표시하는 solver 측 진단이고, 관측은 dense arm 최대 261x, fixed-step arm 최대 60x이며 campaign 행은 'output-policy-dominated'만 기록한다.
- **영향**: PDE류 문제에서 matrix-free protected lane의 정확도 claim 전반이 해당되며, 행 status가 output-policy 판정에 먼저 소비되므로 v2 campaign은 'protocol artifact'와 'solver가 tolerance 밖'을 구분하지 못한다. (관련: H1b, H2, M09)
- **반박 시도**: semilinear 행을 다시 읽어 `d_max` 207.48/260.78/169.07(rtol 1e-8), 53.2/71.0/92.8(rtol 1e-6), 17.0/22.8/38.3(rtol 1e-4)과 code anchor가 정확함을 확인했으나, exact-stage(Direct) semilinear arm은 실행할 수 없어(해당 prebuilt binary 없음, cargo 금지) 귀속은 검증하지 못했다.
- **기존 인지**: doc — `scientific_validity_v2_campaign.rs:183-186`의 `inner_solve_claim_scope`, `g4_s5b0_inner_tolerance.rs:23-29`의 claim scope enum, E-09 `results.json` notes[1]의 order reduction 언급.
- **수정안** (M, 위험 medium): semilinear 초과에 고유한 행 status와 attribution arm을 부여한다. 교체 기준에서 n=96 semilinear dense 세 행(17, 53, 207 case-tolerance unit; 같은 문제의 scipy Radau 0.24, 1.6, 0.47)은 GlobalErrorExceedsBudget로 분류되며, 진단 전용 세 번째 arm 'dense-tight-inner'를 행마다 추가한다. 검증: E-02 harness(`harness/src/bin/e0203_v2rows.rs`)에 이 arm을 semilinear n=96 세 행과 control family(hires)에 추가하고, 3행 중 2행 이상에서 `R_inner > 2`일 때 'inexact stages are the cause'라고 판정하는 규칙을 사전 선언한다.
- **한정**: 제목과 관측의 'accepts every step'은 사실이 아니며 dense arm은 행당 3-8 step을 reject했다. mechanism 귀속(inexact Krylov stage / dense interpolant / W-method order reduction)은 이 발견에서 확립되지 않았고, 병합된 발견의 제목은 원인을 RODAS5P truncation error(~h^5.6 스케일)와 non-conservative embedded estimator로 기술하며 inexact stage나 interpolation이 아니라고 적고 있다.

#### F-034 · P2 · D3 · unverified-assumption — scalar Krylov config가 NaN/Inf tolerance를 수용
- **주장**: `GmresConfig::validate`, `LinearSolverConfig::validate`, `LgmresConfig`, `GcrodrConfig`는 음수 tolerance만 거부한다. `atol=+Inf`(또는 b!=0에서 `rtol=+Inf`)이면 threshold가 +Inf가 되어 `solve_gmres`가 0회 iteration 후 `Ok(converged=true, x=x0 또는 0)`을 반환하고, `rtol=NaN`은 `f64::max`가 NaN을 버리므로 조용히 absolute-only 기준으로 바뀐다.
- **위치**: `crates/rodas5p-krylov/src/gmres.rs:40-44`, `crates/rodas5p-krylov/src/gmres.rs:209-210`, `crates/rodas5p-krylov/src/gmres.rs:218-233`
- **증거·재현**: `gmres.rs:40-44`의 `self.rtol < 0.0 || self.atol < 0.0`과 `gmres.rs:284-289`의 최종 certificate `residual_norm > threshold`가 threshold=+Inf에서 false인 점이 근거이다. 기대는 `gmres_givens.rs:304-308` / `block_gmres.rs:38-48`과 같은 InvalidInput 오류이고, 관측은 validate 통과 후 x=0으로 `converged=true`를 보고하는 것이다.
- **영향**: non-finite tolerance를 만들 수 있는 library caller와 config 경로는 인증되었지만 틀린 해를 받으며, fail-closed인 두 신규 kernel과 일관되지 않는다. CLI JSON은 Inf/NaN을 encode할 수 없어 production 경로는 영향받지 않는다.
- **반박 시도**: `gmres.rs:40-44`, `:209-210`, `:218-233`, `:284-289`의 산술 trace를 따라갔으나 주장은 문자 그대로 참이었고, lgmres/gcrodr는 부호 검사도 없어 발견이 기술한 것보다 더 약하다.
- **기존 인지**: 없음
- **수정안** (S, 위험 low): `common.rs`에 NaN, +/-Inf, 음수 rtol/atol을 `CoreError::InvalidInput`으로 거부하는 helper와, stopping threshold를 만들고 non-finite 결과를 거부하는 두 번째 helper를 두어 tolerance 검증을 한곳으로 모은다. `LgmresConfig`와 `GcrodrConfig`에는 `validate()`를 추가한다. 검증: table-driven test `krylov_contracts.rs::scalar_kernels_reject_non_finite_or_negative_tolerances`로 `solve_gmres`, `solve_lgmres`, `solve_gcrodr`, `solve_gmres_givens` 각각에 대해 rtol/atol이 {NaN, +Inf, -Inf, -1.0}인 경우를 확인한다.
- **한정**: Rust repro는 실행하지 못했고(cargo 사용 불가) 근거는 산술 trace이다.

#### F-036 · P2 · D3 · untested-path — gmres_givens.rs는 test 전용 kernel이며 production gmres.rs와 결과가 다름
- **주장**: integrator, fair-ab, CLI source 어디에서도 `solve_gmres_givens*`를 호출하지 않는다. E-05 s1/jacobi에서 두 kernel의 성공 여부가 엇갈려, gmres는 rhs scale 1e0에서만 실패(4000 소진)하고 gmres_givens는 1e-14와 1e14에서 실패하며 1e0에서 성공(3739 it)한다.
- **위치**: `crates/rodas5p-krylov/src/gmres_givens.rs:436-442`, `crates/rodas5p-krylov/src/gmres_givens.rs:214-226`, `crates/rodas5p-krylov/src/gmres.rs:177-183`
- **증거·재현**: `grep -rln solve_gmres_givens crates`는 `rodas5p-krylov/src`, krylov tests, `integrators/tests/scientific_corpus_v2_spectral_contracts.rs`만 반환하고, E-05 `e05_raw.json` s1 행에서 gmres/jacobi는 1e-14 ok(3200), 1e0 fail, 1e14 ok(3440)이다. 기대는 production caller 또는 `gmres.rs`와의 문서화된 equivalence contract이고, 관측은 production caller 0개와 행별 성공 여부 반전이다.
- **영향**: in-cycle projected-residual 종료가 production 비용을 줄인다는 진술은 이 target에서 뒷받침되지 않으며, Givens kernel의 rejected certification은 `report.matvecs`에서 제외되는 Diagnostic matvec이다(495). (관련: M11)
- **반박 시도**: grep으로 `integrators/src`, `fair-ab/src`, `cli/src`에 hit가 없음을 확인했고 `sequential.rs:401-419`가 `LinearMethod::Gmres`를 `solve_gmres`/`solve_gmres_with_residual_scale`로만 dispatch함을 확인해 반박에 실패했다.
- **기존 인지**: doc — `research/a1_inner_tolerance_audit_20260825/CLAIM_SCOPE_AND_INVALIDATION.md:129` (within-cycle convergence / incremental Givens를 production claim이 아닌 future work A2/A3으로 기재).
- **수정안** (S, 위험 medium): `scientific_corpus_v2_spectral_contracts.rs:244-258`의 probe를 동일한 `GmresConfig`의 `solve_gmres`로 바꾸고 고정된 iteration count를 production kernel에서 다시 유도하며, Givens 수치가 필요하면 명시적으로 label된 두 번째 probe로 남긴다. 검증: 전환 후 `grep -rn solve_gmres_givens crates/rodas5p-integrators`가 corpus calibration 값을 만드는 test에서 hit를 내지 않고, spectral contract test가 `solve_gmres`로 재생성한 iteration count로 통과해야 한다.
- **한정**: refuter는 발견이 기술한 것보다 상황이 나쁘다고 적었다. `scientific_corpus_v2_spectral_contracts.rs:244-258`이 `solve_gmres_givens`를 v2 corpus dimension-sensitivity claim을 calibrate하는 probe로 쓰므로, 그 corpus 증거는 production integrator가 실행하지 않는 kernel에 의존한다. F-038과 중복은 아니다.

#### F-037 · P2 · D3 · untested-path — seeded GMRES가 preconditioned RHS ≈ 0일 때 true residual 없이 수렴 인증
- **주장**: `solve_seeded_gmres`는 `max_i ||M^-1 b_i||_2 <= f64::MIN_POSITIVE`이면 `block_gmres.rs:492-511`에서 solutions=0, residual_norms=0, `converged=true`를 반환한다. 따라서 nontrivial kernel을 가진 Preconditioner(b!=0, M^-1 b=0)는 거짓 certificate를 받으며, true residual을 다시 계산하는 block GMRES rank-0 경로(301-313)와 다르다.
- **위치**: `crates/rodas5p-krylov/src/block_gmres.rs:486-511`, `crates/rodas5p-krylov/src/block_gmres.rs:301-313`, `crates/rodas5p-krylov/src/block_gmres.rs:556-596`
- **증거·재현**: `block_gmres.rs:493-497`의 `if seed_norm <= f64::MIN_POSITIVE { ... converged: true, residual_norms: vec![0.0; rhs_count]`가 근거이다. 기대는 Err(certificate failure) 또는 `residual_norms = ||b_i||`이고, 관측은 코드 읽기(492-511)상 `converged=true`와 전부 0.0인 `residual_norms`이다.
- **영향**: 거짓 certificate는 custom preconditioner에서만 도달 가능하며(built-in Identity/Jacobi/Direct는 nonsingular), degenerate seed 분기를 다루는 test가 없다.
- **반박 시도**: singular preconditioner를 금지하는 trait contract(`operator.rs:286-294`에 없음)와 in-tree 도달 가능성(nonsingular PC만 연결됨)으로 공격했으나, custom Preconditioner로 도달 가능한 untested 경로라는 주장은 유지되었다.
- **기존 인지**: 없음
- **수정안** (S, 위험 low): degenerate-seed 분기의 무조건 성공 반환을 rank-0 block 경로(`:301-313`)와 같은 certificate로 교체한다. x=0이면 true residual이 정확히 `b_i`이므로 operator 적용 없이 `residual_norms[i]=safe_l2(b_i)`를 `atol.max(rtol*||b_i||)`와 비교하고, 실패하는 행이 있으면 `CoreError::LinearSolve`를 반환한다. 검증: `block_gmres_contracts.rs`에 세 test를 추가하며, 그중 `seeded_gmres_rejects_preconditioner_that_annihilates_nonzero_rhs`는 test-local `ZeroingPreconditioner`와 nonzero RHS 두 행으로 Err를 요구한다.
- **한정**: 관측은 실행이 아니라 코드 읽기에 근거한다.

#### F-039 · P2 · D4 · uncounted-work — 실패한 adaptive-exponential trial의 work가 counter에서 누락
- **주장**: kernel은 local `WorkCounters`(`exponential.rs:2402`)에 누적하고 Err를 `?`/`require_fused`로 전파한다. adaptive loop는 Err에서 `counters.rejected_steps`만 세고(`adaptive_exponential.rs:252`) `report.work`는 Ok에서만 누적하므로(258), 실패한 trial의 JVP/RHS evaluation은 exponential lane의 work 합계에 없다.
- **위치**: `crates/rodas5p-integrators/src/adaptive_exponential.rs:225-258`, `crates/rodas5p-integrators/src/exponential.rs:2402-2403`, `crates/rodas5p-integrators/src/exponential.rs:1309-1322`
- **증거·재현**: `adaptive_exponential.rs:227-253`은 `rejected_steps`만 갱신하고 work를 누적하지 않으며, 258행의 `counters.accumulate(report.work)`는 Ok에서만 실행된다. 기대는 시도된 work가 두 arm 모두에서 누적되는 것이고, 관측은 Ok에서만 누적되는 것이다.
- **영향**: exponential candidate와 RODAS5P의 비용 비교는 candidate의 실패 trial work를 누락하며, 편향 방향은 exponential lane에 유리하다. (관련: H4)
- **반박 시도**: Err arm이 telemetry에 `trial_work: None`을 기록하고 kernel의 local `WorkCounters`가 `?`와 `require_fused`의 Err로 버려짐을 확인했으며, 이를 누적하는 경로를 찾지 못해 반박에 실패했다.
- **기존 인지**: 없음
- **수정안** (M, 위험 low): level-1 prefix의 local `WorkCounters`(`exponential.rs:2402`)가 모든 실패 경로(`require_fused`의 `Err(LinearSolve)`, `exponential.rs:2131-2140`, 그리고 `?`로 전파되는 NonFinite)에서 버려지므로, 실패 시에도 work가 adaptive loop의 counter에 전달되도록 한다(요약문의 구체적 방법은 입력에서 잘려 있음). 검증: `FusedPhiKrylovConfig{maximum_dimension: 2, maximum_substeps: 1}`로 `require_fused`가 실패하는 stiff 문제에서, rejected trial 1회 후 driver의 `counters.jvp_calls`와 `rhs_evaluations`가 kernel-local counter와 같음을 assert한다.
- **한정**: rejected이지만 점수 계산이 가능한 trial은 집계된다(`:258`의 accumulate가 accept test보다 앞섬). 누락은 점수 계산이 불가능한 실패(NonFinite/LinearSolve)로 한정된다.

#### F-040 · P2 · D4 · scale-dependence — phi-action이 beta를 정규화 없이 Pade-13 augmented 행렬에 넣음
- **주장**: k>=1인 모든 Krylov phi_k action에서 projected oracle의 상대오차는 `ceil(log2(||v||_2/5.37))`개의 추가 squaring과 함께 커져 `||v||_2=1e8`에서 약 1e-8에 이른다(E-06). `projected_action`이 `reduced_input=[beta,0,..]`를 `dense_phi_action`에 넘기고, 이 함수가 벡터를 그대로 augmented matrix에 넣으며 그 1-norm이 squaring 횟수를 정하기 때문이다.
- **위치**: `crates/rodas5p-integrators/src/exponential.rs:805-829`, `crates/rodas5p-integrators/src/exponential.rs:858-858`, `crates/rodas5p-integrators/src/exponential.rs:989-1009`
- **증거·재현**: E-06 `results.json`에서 `cases_failing_any_k=18/54`이고 모두 `||v||=1e8`이며 `worst_relerr_k1_to_4=1.58e-8`이다. 기대는 모든 k에서 상대 편차 <= 1e3*eps = 2.2e-13이고, 관측은 k>=1에서 2.7e-9..1.6e-8(matrix type과 `||A||`에 무관), k=0과 `matrix_exp_pade13` 자체는 3.5e-14 이하, `||v||=1`과 1e-8은 통과이다.
- **영향**: 모든 non-fused exponential candidate(exprb32/43, `projected_action`을 거치는 pexprb54s4)와 KIOPS-style residual estimator가 h나 Krylov tolerance가 아닌 사용자 rhs의 단위에 의존하는 phi-action 상대오차 floor(약 `2^ceil(log2(beta/5.37))*eps`; beta=1e5에서 약 1e-11, beta=1e8에서 약 1e-8)를 물려받는다.
- **반박 시도**: `exponential.rs:858`, `:820-822`, `:989-1005`와 `matrix_functions.rs:152-158`, `:80-84`를 따라가 unfused production 경로가 beta를 정규화하지 않고 넘김을 확인했고, mechanism과 측정값 모두 반박되지 않았다.
- **기존 인지**: 없음
- **수정안** (S, 위험 low): oracle에서 고친다. augmentation column을 `1/||v||_inf`(또는 `1/||v||_2`)로 scale한 뒤 exponentiate하고, 추출한 column에 `||v||`를 곱한다(phi_k는 v에 대해 선형). 검증: E-06 harness(`harness/src/bin/e06_phi_action.rs`, `exp/E-06/cases.json`)를 patch된 crate로 재실행해 54 case 모두 k=0..4에서 `relerr_k <= 2.2e-13`을 만족해야 한다(현재 worst 1.58e-8).
- **한정**: F-042와 근본 원인(`dense_phi_action` + `matrix_exp_pade13`에 augmented column 정규화와 overscaling guard가 없음)을 공유한다. trigger가 작은 h가 아니라 `||v||`이고 code site가 달라 duplicate로 표시하지 않았다.

#### F-041 · P2 · D4 · untested-path — exponential order 증거가 scalar y'=y^2 test에만 의존
- **주장**: `declared_orders_are_observed_on_nonlinear_problem`은 `square_problem`(dimension 1)을 쓰므로 pexprb54s4 >= 4.7, exprb43 >= 3.7 assertion은 Krylov truncation, 비가환 J/N 상호작용, stiff order reduction을 검출할 수 없다. exact phi function을 쓴 감사 측정에서 nonstiff 2-D 문제의 local order는 6/5(main/embedded)이지만 lambda=-1e4인 stiff nonlinear 2-D 문제의 global order는 약 4.1이다.
- **위치**: `crates/rodas5p-integrators/tests/exponential_contracts.rs:9-20`, `crates/rodas5p-integrators/tests/exponential_contracts.rs:131-135`, `crates/rodas5p-integrators/src/exponential.rs:774-793`
- **증거·재현**: `exp/D4/pexprb54s4_order.py`의 출력에서 nonstiff local slope는 5.7-5.9(main), 4.8-5.0(embedded)이고 stiff lam=-1e4 global slope는 4.15/4.09/4.08(main), 4.2/4.1/4.0(embedded)이다. 기대는 stiff하고 비가환인 문제에서의 order-5 증거이다.
- **영향**: receipt/ledger의 pexprb54s4 'order 5' label은 nonstiff, exact-phi 영역에서만 뒷받침되며, RODAS5P(order 5, stiffly accurate) 대비 stiff 성능 claim은 test suite로 입증되지 않는다. (관련: H2)
- **반박 시도**: `exponential_contracts.rs:9-28`, `:40-70`, `:131-135`와 `fused_exponential_contracts.rs:33-71`을 확인하고 pexprb54s4를 참조하는 나머지 10개 test 파일을 grep했으나, order assertion은 scalar y'=y^2 문제를 쓰는 두 곳뿐이어서 반박에 실패했다.
- **기존 인지**: 없음
- **수정안** (S, 위험 low): Krylov truncation과 stiff order reduction을 볼 수 있도록 scalar 문제 외에 감사가 사용한 2-D nonstiff 및 stiff(lambda=-1e4) 문제에 대한 order test를 추가한다(요약문의 구체 항목은 입력에서 잘려 있음). 검증: `exp/D4/pexprb54s4_order.py`로 기대 slope(nonstiff local 5.7-5.9 main / 4.8-5.0 embedded, stiff global 4.08-4.15)를 고정하고, 새 test가 exact-phi config에서 이를 0.2 이내로 재현해야 한다.
- **한정**: `exp/D4/pexprb54s4_order.py`는 mpmath로 작성한 독립 재구현이며 Rust 코드를 실행한 측정이 아니다.

#### F-042 · P2 · D4 · scale-dependence — fused phi 조합의 scale^k 나눗셈으로 작은 h에서 정확도 저하
- **주장**: pexprb54s4 endpoint 항(`D_i = O(h^2)`인 phi_3/phi_4)에서, `b_k = c_k v_k / h^k`로 만든 augmented matrix에 Rust algorithm(1-norm squaring count, `||A^k||^{1/k}` test 없음)을 적용하면 상대 정확도가 h에 따라 단조롭게 나빠진다(h=0.1에서 7.5e-15, 1e-3에서 6.5e-13, 1e-4에서 9.0e-12; 같은 행렬의 scipy expm은 1e-16). repository test는 scale 0.1/0.17만 다룬다.
- **위치**: `crates/rodas5p-integrators/src/exponential.rs:1351-1361`, `crates/rodas5p-integrators/src/exponential.rs:1174-1222`, `crates/rodas5p-integrators/src/exponential.rs:985-1010`
- **증거·재현**: `exp/D4/pade_overscaling.py`의 출력과 `matrix_functions.rs:80-85`의 `(norm / PADE_13_THETA).log2().ceil()`(Al-Mohy-Higham 2009 보정 없음)이 근거이다. 기대는 h와 무관한 약 1e-15 상대오차이고, 관측은 h가 0.1에서 1e-4로 갈 때 squaring 8 -> 17, rel_err 7.5e-15 -> 9.0e-12이다.
- **영향**: 작은 step이나 큰 nonlinear remainder에서 exponential lane의 정확도는 projected-space 'oracle'이 시사하는 것보다 나쁘고, prefix parity test가 쓰는 rtol 1e-12에서는 oracle 오차가 이미 tolerance 수준이다. rtol >= 1e-10에서는 아직 잘못된 결과가 아니다.
- **반박 시도**: `exponential.rs:1351-1361`, `:1174-1222`, `:1278`, `:992`와 `matrix_functions.rs:80-84`를 따라가 mechanism을 확인했고, 작은 scale에서 test되지 않았다는 점도 반박하지 못했다.
- **기존 인지**: 없음
- **수정안** (M, 위험 medium): `fused_phi_linear_combination`이 각 항을 `scale^k`로 나누어(`exponential.rs:1357-1361`) augmented operator의 B block norm O(c/h)가 projected Hessenberg의 1-norm을 지배하고 `matrix_exp_pade13`이 과도하게 squaring하므로, 이 over-squaring을 막도록 oracle을 고친다(요약문의 구체 방법은 입력에서 잘려 있음). 검증: `exp/D4/pade_overscaling.py`의 augmented matrix(h=1e-4에서 `||M||_1 = 4.06e5`)를 rodas5p-core test로 옮겨 h {1e-1,1e-2,1e-3,1e-4}에서 고정밀 reference 대비 rel err <= 1e-14를 assert한다.
- **한정**: 근거는 3x3 model의 Python replica이며 Rust는 실행되지 않았다. 크기는 예시적이고 Krylov Hessenberg에서 측정한 값이 아니다. F-040과 근본 원인을 공유하되 trigger가 다르다(`1/scale^k` 증폭 대 `||v||`).

#### F-043 · P2 · D4 · unverified-assumption — fused Krylov threshold가 augmented start norm 기준이고 proxy는 WRMS가 아님
- **주장**: `threshold = atol + rtol*max(||current||_2, beta)`에서 `beta = ||[b0;0;..;1]||_2 >= 1`(start vector 1216-1218)이므로 `||f0|| << 1`이면 phi action은 사실상 절대 tolerance `rtol*1`로 인증된다. 또한 `adaptive_exponential`은 L2 augmented residual 합을 `h*sum/||scale||_2`(componentwise WRMS 아님)로 바꾼 뒤 WRMS time error와 max를 취한다.
- **위치**: `crates/rodas5p-integrators/src/exponential.rs:1131-1138`, `crates/rodas5p-integrators/src/exponential.rs:1216-1218`, `crates/rodas5p-integrators/src/adaptive_exponential.rs:83-94`
- **증거·재현**: `exponential.rs:1135`의 `safe_l2(&current).max(beta)`와 `adaptive_exponential.rs:92`의 `h.abs()*estimate/safe_l2(scale)`이 근거이다. 기대는 물리적 action에 상대적인 phi 오차 제어이고, 관측은 물리적 크기와 무관한 threshold floor `rtol*1`이며 `||f0||<<1` 또는 `>>1`을 다루는 test가 없다.
- **영향**: exponential lane의 오차 제어는 RHS 크기에 scale-dependent이며, accept/reject 결정의 phi-error 성분은 max를 취하는 대상인 time error와 같은 단위가 아니다.
- **반박 시도**: `exponential.rs:1216-1218`, `:1053`, `:1134-1135`, `:1595-1596`(prefix session)과 `adaptive_exponential.rs:83-94`, `:263-265`를 확인했으나 인용된 행은 모두 정확했고 반박되지 않았다.
- **기존 인지**: 없음
- **수정안** (M, 위험 medium): augmented start norm 때문에 `rtol*1`로 floor되는 fused Krylov threshold(`exponential.rs:1134-1135, 1595-1596`)와 `h*sum/||scale||_2` proxy(`adaptive_exponential.rs:83-94`)를 물리적 action 크기와 WRMS 단위에 맞춘다(요약문의 구체 방법은 입력에서 잘려 있음). 검증: stiff 2-D 문제와 그 rescale 사본(`y -> c*y`, `f -> c*f`, `atol -> c*atol`, c는 {1e-3, 1e3})에서 adaptive pexprb54s4 driver가 동일한 accepted step sequence와 Krylov dimension을 내는 scale-invariance test를 추가한다.
- **한정**: KIOPS의 column balancing이 없는 KIOPS-style augmented-norm 기준이며, proxy는 scale vector가 uniform일 때만 차원상 WRMS에 해당한다. 이를 소비하는 것은 guarded pexprb54s4 adaptive candidate뿐이고, finder와 refuter 모두 실행은 하지 않았다.

### 3.3 P2 발견 — 게이트·회계·참조·audit2 (D5–D8)

#### F-045 · P2 · D5 · unverified-assumption — atlas의 'unsafe' 라벨은 후보의 자체 오차 추정이며 참조해가 없음
- **주장**: `run_exponential_shadow`는 `admissible = total_error <= 1.0`으로 판정하며, `total_error`는 `pexprb54s4`의 embedded `error_estimate`의 WRMS와 h·(Krylov phi 오차 추정 합)/||scale|| 중 큰 값이고 스칼라 atol을 쓴 `error_scale(y, y_new, &[atol], rtol)`로 계산된다. `unsafe_recommendations`는 `!shadow_full_e_locally_admissible`인 추천 행을 세므로 'unsafe' 판정에 독립 참조가 개입하지 않는다.
- **위치**: `crates/rodas5p-integrators/src/g4_s5b0_regime_atlas.rs:1540-1600`, `crates/rodas5p-integrators/src/g4_s5b0_regime_atlas.rs:4576-4581`, `research/generic_enforced_prefix_budget_v35/results/RESULT_SUMMARY.json:39-47`
- **증거·재현**: `g4_s5b0_regime_atlas.rs:1572`에서 `total_error=time_error.max(phi_error)`, `:1598`에서 `admissible = total_error<=1.0`이며, v35 `RESULT_SUMMARY.json`의 `audit_unsafe_events=1`(hires, total_error 1.1351)이 v3.5 'discriminating evidence' 게이트 전체의 유일한 positive control이다. 기대는 참조 궤적 또는 protected R-JF step 결과에 고정된 안전 라벨이고, 관측은 후보가 자기 보고한 embedded 오차이며 추정기가 오차를 과소추정하면 safe로 표시된다.
- **영향**: v3.5/v3.6/v3.7의 `unsafe_recommendations = 0`과 positive control `audit_unsafe_events = 1 (14.32 > tau)`는 후보의 자체 추정에 의존하므로 실제 step 오차에 대한 안전성 진술을 뒷받침하지 못하고, 단일 사건(n=1)이라 게이트의 민감도도 낮다.
- **반박 시도**: 라벨에 참조 궤적이나 R-JF 비교가 들어가는 경로를 찾으려 했으나 없었다. `V35_FRESH_BUDGET_SAFETY_HOLDOUT_CONTRACT.md:38`이 positive control을 'audit-full-E inadmissible'로 조작적으로 정의하므로 ledger 수치 자체는 내부적으로 일관된다.
- **기존 인지**: 없음
- **수정안** (L, 위험 medium): 자체 추정과 안전 라벨을 분리하고 holdout의 최소 증거 기준을 둔다. 현재 양은 `self_estimate_inadmissible`이라는 이름으로 유지하고, 모든 추천 행에 대해 같은 (t, y, h)를 tight-tolerance protected solve로 진행시켜 문제의 선언된 atol 벡터 기준 WRMS 차이를 재는 reference-anchored local error를 추가한다('unsafe'는 reference error > 1). 검증: unit test `self_estimate_and_reference_label_are_independent_fields`에서 embedded 추정 0.5, 참조 차이 3 WRMS인 stub 보고가 `reference_unsafe = true`, `self_estimate_inadmissible = false`로 표시되는지 확인한다.

#### F-046 · P2 · D5 · unsourced-constant — 정책 상수 다수가 저장소 내 유도 근거 없이 사용됨
- **주장**: `V25_ERROR_DROP_THRESHOLD=0.012790399606947056`과 `V29_STAGE_GROWTH_BASELINE=3.24`는 `g4_s5b0_regime_atlas.rs`에만 나타나 출처가 없다(unsourced). `V29_PREFIX_RESERVE_JVP=80`과 `V29_PREFIX_BUDGET_FRACTION=0.25`는 v3.5 audit에 유도 없이 선언되어 있고(declared), `V36_FROZEN_ZETA34_TAU`는 fitted 값이다(`FROZEN_ZETA34_POLICY.json`이 calibration192의 24개 후보 / 23개 eligible 사건에서 tau를 선택).
- **위치**: `crates/rodas5p-integrators/src/g4_s5b0_regime_atlas.rs:1875-1875`, `crates/rodas5p-integrators/src/g4_s5b0_regime_atlas.rs:2942-2955`, `crates/rodas5p-integrators/src/g4_s5b0_regime_atlas.rs:377-379`
- **증거·재현**: `grep -rn '0.0127903996' research crates tools`는 `g4_s5b0_regime_atlas.rs:1875`만, `3.24` 검색은 `:2955`만 반환한다. `FROZEN_ZETA34_POLICY.json`은 candidate_count 24, eligible_events 23, final.tau 13.39706618860016, unsafe 0이며, v35 `PHYS_MATH_AUDIT.md:7`은 80과 0.25를 유도 없이 기술한다. 기대는 각 상수가 calibration artifact, 인용, 또는 명시된 설계 선택으로 추적되는 것이다.
- **영향**: 분류는 `V25_ERROR_DROP_THRESHOLD`=unsourced(유효숫자 14자리는 fit artifact가 트리에 없는 fitted quantile임을 시사), `V29_STAGE_GROWTH_BASELINE`=unsourced, B_abs=80과 delta=0.25=declared, tau=fitted(calibration192, n=23), 1.15x=declared이며, tau fit의 선택 기준이 'calibration에서 unsafe==0'이어서 calibration 집합이 통과하도록 선택된 값이다.
- **반박 시도**: `research/`, `crates/`, `tools/`에서 유도 artifact를 찾으려 했으나 v2.5/v2.9 ledger 디렉터리가 없어 실패했다.
- **기존 인지**: doc — `crates/rodas5p-integrators/src/g4_s5b0_regime_atlas.rs:377-379` doc comment와 `research/generic_enforced_prefix_budget_v35/contracts/V35_ENFORCED_PREFIX_BUDGET_CONTRACT.md:9`가 tau를 sealed calibration output으로 선언한다(tau 항목만 해당).
- **수정안** (S, 위험 low): 상수 provenance 표 `docs/CONSTANTS_PROVENANCE.toml`과 이를 소스와 동기화하는 테스트를 도입한다. 각 행은 name, value(정확한 literal), location, class(derived | fitted | declared | cited | unsourced), source_artifact 경로와 sha256, fit_dataset과 n, selection_criterion, sensitivity, valid_scope, status를 가진다. 검증: 테스트 `constants_provenance_table_matches_source`가 TOML을 파싱해 각 값을 `pub const` registry의 상수와 bitwise 비교하고, atlas에서 `V<digits>_` 패턴의 정책 상수가 표에 없으면 실패한다.
- **한정**: tau는 숨겨진 fit이 아니다. `atlas.rs:377-379`와 V35 contract `:9`가 frozen calibration192 threshold임을 명시적으로 선언한다.

#### F-047 · P2 · D5 · leakage — frozen k=1 comparator가 N=512 holdout의 step index를 하드코딩함
- **주장**: `G4S5B0PrefixProbePolicy::FrozenK1Comparator`는 `frozen_k1_decision`에 (dimension ∈ {128,512}, family)별로 박힌 literal accepted-step index에서 발화하며 관측된 error-drop feature를 무시한다. 512 행은 128 행과 다르므로(예: semilinear `[8,20,27,34]` vs `[9,15]`) 이전 N=512(`Holdout512`) 궤적에서 읽어낸 값일 수밖에 없다.
- **위치**: `crates/rodas5p-integrators/src/g4_s5b0_regime_atlas.rs:1877-1898`, `crates/rodas5p-integrators/src/g4_s5b0_regime_atlas.rs:1915-1938`, `crates/rodas5p-integrators/src/g4_s5b0_regime_atlas.rs:86-94`
- **증거·재현**: `g4_s5b0_regime_atlas.rs:1877-1898`은 (dimension,family) → `&[usize]` literal match이고 `:1929-1931`의 정책 분기는 `feature`를 무시한다. `main.rs:514`의 `--prefix-policy frozen-k1`과 `main.rs:538`의 `--profile holdout`으로 N=512 표에 CLI에서 도달할 수 있으며, `tests/g4_s5b0_regime_atlas_contracts.rs:195-246`은 `Calibration128`만 시험한다. 기대는 step별 인과적 관측의 함수인 comparator 또는 calibration 차원으로 제한된 표이다.
- **영향**: `Holdout512`에서 `K3Development`를 `FrozenK1Comparator`와 비교하면 live 정책을 같은 holdout에 맞춘 사후 replay와 비교하는 것이므로 holdout 증거로 부를 수 없다.
- **반박 시도**: 이 comparator를 소비하는 공개 ledger를 찾으려 했으나 `research/` 아래에서 'frozen-k1-comparator'나 'holdout-512' 참조가 없어, 공개된 결과로의 누출은 확인되지 않았다.
- **기존 인지**: 없음
- **수정안** (S, 위험 low): `frozen_k1_decision`에서 N=512 행을 삭제하고 정책 이름을 replayed k=1 table로 바꾸며, holdout profile과 결합하면 CLI를 포함해 typed error를 반환한다. `K3Development`와 같은 error-drop feature에 latch 길이 1로 발화하는 causal k=1 comparator를 추가한다. 검증: 테스트 `replayed_k1_table_is_rejected_on_holdout_profile`(Holdout512 + replay 정책이 `InvalidInput` 반환)과 `causal_k1_comparator_fires_only_on_observed_feature`를 추가한다.
- **한정**: 'N=512 궤적에서 읽어냈다'는 부분은 추론이다. 다만 128/512 표가 서로 다르고 트리 내 유도 근거가 없다. 영향은 CLI/실험적 사용에 한정된다.

#### F-048 · P2 · D6 · uncounted-work — `jvp_calls`의 의미가 generic 경로와 matrix-free 경로에서 다름
- **주장**: generic 경로(`build_step_context` → `OdeProblem::linearize`)에서는 JVP 기반 `ClosureOperator`가 기본 `application_work()`(0)를 상속하므로 Krylov/진단 JVP 적용이 `linear_matvecs`/`diagnostic_matvecs`만 증가시키고 `jvp_calls`는 실제 JVP callback을 과소집계한다. matrix-free 경로(`new_counted_jvp`)는 apply마다 `jvp_calls`를 부과하여 atomic count와 정확히 일치한다.
- **위치**: `crates/rodas5p-integrators/src/problem.rs:239-245`, `crates/rodas5p-core/src/operator.rs:123-125`, `crates/rodas5p-core/src/operator.rs:504-506`
- **증거·재현**: E-08(`$RUN/exp/E-08/results.json`)에서 generic GMRES lane은 Prothero-Robinson에서 `jvp_calls` 140 vs atomic 760(`linear_matvecs` 460, diagnostic 160), robertson n=96에서 8841 vs 74897이고, matrix-free 12개 arm 모두에서 차이는 0이다. 기대는 모든 경로에서 `jvp_calls` == 사용자 JVP callback 수이며, explicit Jacobian이 있는 경우에는 JVP callback이 0회인데 `jvp_calls=140`으로 관측되었다.
- **영향**: generic-lane 후보와 matrix-free 후보 사이에서 `jvp_calls`(또는 `jvp_vectors`)를 비교하는 ledger, gate, 보고서는 서로 다른 양을 비교하며, generic lane은 JVP 작업량이 실제보다 5-8배 저렴해 보인다.
- **반박 시도**: `jvp_calls`를 'Krylov 밖의 explicit JVP callback'으로 정의한 문서를 찾으려 했으나 트리에 없어, lane 간 의미 차이는 설계 선택이 아니라 문서화되지 않은 drift로 남는다. 실행 증거와 코드가 정확히 일치한다.
- **기존 인지**: 없음
- **수정안** (M, 위험 medium): 서로 합산하지 않는 두 축의 `WorkCounters` 계약을 채택한다. PROVENANCE 축(`jvp_calls`/`jvp_vectors` = 모든 lane에서 실제 실행된 사용자 JVP callback 호출/벡터 수, explicit dense J*v는 새 `jacobian_matvecs`)과 ROLE 축(`linear_matvecs`, `recycle_refresh_matvecs`, `diagnostic_matvecs`, `block_matvecs`)으로 나누고, provenance를 호출 위치가 아닌 operator의 속성으로 만든다. 검증: E-08 harness(`harness/src/bin/e08_work_counters.rs`)를 24개 arm 모두에서 `counters.jvp_calls == atomic.jvp` 규칙으로 재실행하여 generic GMRES arm이 140/8841 대신 760(PR), 74897(robertson n=96)을 읽는지 확인한다.

#### F-049 · P2 · D6 · fairness — `load_rodas5p_coefficients()`가 매 step·매 dense 샘플마다 JSON 파싱과 8x8 역행렬을 수행함
- **주장**: 모든 RODAS5P step context(`sequential.rs:154`, `:202`)와 모든 내부 dense-output 평가(`dense_output_v2.rs:82`)가 `load_rodas5p_coefficients()`를 호출한다. 이 함수는 4,430바이트 `include_str!` fixture를 serde_json으로 파싱하고 약 110개의 십진 문자열을 재파싱한 뒤 inverse(Gamma^-1)와 8x8 matmul 3회를 `OnceLock`/`LazyLock` 캐시 없이 계산하여, comparator(BDF/Radau)가 내지 않는 O(10-40 us) 상수를 step과 출력 샘플마다 더한다.
- **위치**: `crates/rodas5p-core/src/coefficients.rs:102-161`, `crates/rodas5p-integrators/src/sequential.rs:154-154`, `crates/rodas5p-integrators/src/sequential.rs:202-202`
- **증거·재현**: `coefficients.rs:103-105`의 `serde_json::from_str(include_str!(...))`, `:138-142`의 `inverse(&gamma_inv)`와 matmul이 호출마다 무조건 실행되고, `dense_output_v2.rs:66-71`은 theta==0/1만 load 전에 short-circuit한다. 기대는 tableau 비용을 한 번만 지불하는 것이고, 관측은 모든 step 시도(accepted, rejected, failed)와 모든 dense 샘플에서 지불된다.
- **영향**: RODAS5P lane의 모든 wall-clock 수치(BDF/Radau 대비 fair-ab authoritative timing front `global_error.rs:1722-1792`, v3.x shadow-wall gamma = wall/sum|h|, dense-vs-clipped wall 비교)가 오염되고, 양쪽 arm이 같은 가산 비용을 내므로 후보 대 baseline speedup 비율이 1 쪽으로 압축된다.
- **반박 시도**: (1) 캐시를 찾았으나 `rodas5p-core/src`와 `rodas5p-integrators/src`에서 `OnceLock|LazyLock|lazy_static|once_cell` 검색 결과가 0건이었고, (2) comparator의 동등한 비용을 찾았으나 Radau는 closed-form 3x3 tableau만 재구성하고 BDF는 없어 비대칭이 실재한다.
- **기존 인지**: 없음
- **수정안** (S, 위험 low): RODAS5P tableau를 프로세스당 한 번 `std::sync::OnceLock`에서 파싱·유도하여 `&'static Rodas5pCoefficients`로 제공하고, `load_rodas5p_coefficients()`는 테스트와 CLI용 cloning 호환 wrapper로 남긴다. `sequential.rs:154`/`:202`, `dense_output_v2.rs:82`, `stage_batch.rs:173`(및 `homotopy.rs`, `common_w_gate.rs`)을 캐시된 accessor로 전환한다. 검증: `rodas5p_coefficients()`가 같은 포인터를 두 번 반환하고 새 파싱 결과와 `to_bits()`로 필드별 일치하는지 unit test로 확인하며, fair-ab fixed/adaptive screen의 scientific checksum이 전후로 불변인지 확인한다.

#### F-050 · P2 · D6 · uncounted-work — legacy atlas `run_trajectory`가 rejected/failed 시도의 RHS/JVP 작업량을 누락함
- **주장**: `g4_s5b0_regime_atlas::run_trajectory`는 매 시도의 `jacobian_builds`와 `direct_factorizations`만 합산한다(1678-1679). `Err(NonFinite|LinearSolve)` 또는 embedded rejection에서는 `G4S5B0StepRow` push 전에 `continue`하므로, rejected/failed 시도의 `rhs_evaluations`/`jvp_vectors`/`linear_matvecs`가 `run_g4_s5b0_regime_atlas`의 atlas summary에 들어가지 않는다.
- **위치**: `crates/rodas5p-integrators/src/g4_s5b0_regime_atlas.rs:1678-1690`, `crates/rodas5p-integrators/src/g4_s5b0_regime_atlas.rs:1697-1710`, `crates/rodas5p-integrators/src/g4_s5b0_regime_atlas.rs:1736-1749`
- **증거·재현**: 1683-1686의 `rejected += 1; h *= min_factor; continue`와 1697-1710의 rejected 분기 `continue`가 1736-1749의 `rows.push` 이전에 실행되며, `README.md:23`은 failed/speculative 작업량이 ledger에 유지된다고 약속한다. 기대는 행 작업량의 합 == 실제 JVP callback 수이고, 코드상 관측은 accepted 행만 counter를 가진다.
- **영향**: legacy runner의 atlas step별 작업량 행과 그로부터 유도한 'R-JF cost' baseline은 rejection이 발생하는 stiff 전이 구간에서 RODAS5P 비용을 과소평가하며, v3.x attempt-row runner는 이 결함을 공유하지 않으므로 runner 세대 간 비용 수치를 비교할 수 없다.
- **반박 시도**: 예산 내에서 rejection을 강제한 atlas 바이너리 실행은 하지 못했으나 제어 흐름이 무조건적이어서 반박되지 않았다. `research/` 아래 어떤 ledger도 `run_g4_s5b0_regime_atlas` 출력을 소비하지 않아 영향은 제한적이다.
- **기존 인지**: 없음
- **수정안** (S, 위험 low): 모든 시도(accepted, embedded 추정에 의한 rejected, NonFinite/LinearSolve로 중단)가 자신의 `WorkCounters` 전체를 trajectory 합계에 부과하고 rollback하지 않는 규칙을 적용한다(E-08이 production lane에서 이미 관측한 동작). `run_trajectory`에서 trial 반환 직후, 어떤 `continue`보다 앞서 `step_counters` 전체를 trajectory 합계에 merge한다. 검증: h0를 크게 강제해 최소 1회 reject되는 profile에서 rhs/jvp closure를 `AtomicU64`로 감싸 `summary.total_rhs_evaluations == atomic rhs`, `summary.total_jvp_vectors == atomic jvp`를 확인하는 테스트를 추가한다.

#### F-051 · P2 · D6 · uncounted-work — Block GMRES가 JVP 8회를 `linear_matvec` 1회로, LU solve 8회를 `preconditioner_app` 1회로 보고함
- **주장**: `solve_gmres`로 푸는 s*n `BlockOperator`에서 각 Krylov apply는 물리적으로 s=8회의 JVP를 수행하면서 `linear_matvecs`를 1만 증가시키고, 각 `BlockDirectPc` apply는 LU back-solve 8회를 수행하지만 `preconditioner_app` 1회로 집계된다. 따라서 `BlockSolveReport`의 `matvecs`/`preconditioner_apps`와 block_* counter는 sequential lane 대비 벡터 작업량을 8배 과소집계한다(`jvp_calls`/`jvp_vectors`는 `application_work`로 올바르게 부과됨).
- **위치**: `crates/rodas5p-integrators/src/block.rs:616-633`, `crates/rodas5p-integrators/src/block.rs:644-653`, `crates/rodas5p-integrators/src/block.rs:555-577`
- **증거·재현**: `block.rs:622-633`의 `application_work{jvp_calls:8,jvp_vectors:8,block_matvecs:1}`, `operator.rs:233`의 `linear_matvecs += 1`, `block.rs:571-577`의 보고 필드 `matvecs=d.linear_matvecs`, `preconditioner_apps=d.preconditioner_apps`가 근거이다. 기대는 sequential lane과 같은 단위(벡터 하나 = 1)의 counter이고, 관측은 8-벡터 block apply당 1씩 증가한다.
- **영향**: block-GMRES 후보(SABR, homotopy)와 sequential GMRES 사이의 `linear_matvecs`, `preconditioner_apps`, `block_preconditioner_apps` 비교는 block 후보에 유리하게 8배 편향되며, unified_screen의 batch_vectors = 8*block_linear_solves는 집계가 아닌 가정이다.
- **반박 시도**: 주 비교 지표의 오류 여부를 확인했으나 `jvp_vectors`는 8씩 올바르게 증가하므로, 편향은 2차 matvec/preconditioner counter와 그로부터 유도된 `WorkLedger` 필드에 국한된다.
- **기존 인지**: 없음
- **수정안** (M, 위험 low): `linear_matvecs`와 `preconditioner_apps`는 solver-level 적용 횟수로 유지하고, n-벡터 등가 단위로 부과하는 `linear_matvec_vectors`와 `preconditioner_vectors`를 추가한다(block apply당 s=8, sequential apply당 1). 단위는 호출자가 아닌 객체에서 오도록 `OperatorApplicationWork`에 `state_vectors`를 추가한다. 검증: `block_gmres_charges_every_stage_jvp_and_mass_action_at_the_operator_site`를 `BlockPreconditioner::Direct`로 확장하여 `delta.linear_matvec_vectors == s*(linear_matvecs+diagnostic_matvecs) == delta.jvp_vectors`를 assert한다.

#### F-052 · P2 · D6 · fairness — 내부 BDF/Radau comparator가 고정 Newton 허용오차와 solve마다의 새 Jacobian+LU로 불리함
- **주장**: comparator의 Newton solve는 외부 rtol과 무관한 고정 절대 임계값(`NewtonConfig` 기본 atol 1e-12/rtol 1e-10, Radau IIA3 1e-14/1e-12)까지 수렴하고, `solve_dense_newton` 호출마다 시작 시 dense Jacobian+LU를 새로 만들며 step 간 LU를 재사용하지 않는다. 그 결과 accepted step당 rhs_calls, direct_factorizations, wall이 production implicit solver 대비 부풀려진다.
- **위치**: `crates/rodas5p-integrators/src/nonlinear.rs:20-31`, `crates/rodas5p-integrators/src/nonlinear.rs:154-165`, `crates/rodas5p-integrators/src/nonlinear.rs:233-243`
- **증거·재현**: `nonlinear.rs:20-31`의 기본값, `radau.rs:118-126`의 override, `nonlinear.rs:158-165`의 solve 시작 시 factorize, `global_error.rs:1548`의 `ParetoCostMetric::WallSeconds`가 근거이다. 기대는 Newton 정지 허용오차가 외부 허용오차에 비례(kappa*rtol)하는 것이고, 코드상 관측은 모든 외부 허용오차에서 1e-12/1e-10(Radau는 1e-14/1e-12)에서 정지한다.
- **영향**: fixed/adaptive fair-ab screen의 RhsCalls, DirectFactorizations, WallSeconds 기준 RODAS5P 대 comparator Pareto front와 attainment는 RODAS5P 쪽으로 기울어 있어 성능 정보를 담지 못한다(관련 가설 H4). `README.md:27`이 경쟁적 해석을 금지하므로 공개된 주장이 반증되지는 않는다.
- **반박 시도**: 하위 주장을 개별 확인했으나 모두 문자 그대로 참이었다. 전용 바이너리 없이 comparator sweep은 실행하지 못했으나 코드 경로가 무조건적이다.
- **기존 인지**: doc — `README.md:27`('reference implementations, not competitive production baselines'); `research/scientific_validity_v2_20260829/CLAIM_SCOPE_AND_INVALIDATION.md:86`
- **수정안** (L, 위험 medium): 2단계 comparator 정책을 둔다. Tier A(front를 성능으로 읽기 전 필수인 최소 공정성)는 Newton 정지 허용오차를 외부 허용오차로 스케일하고(SciPy Radau 규칙 kappa = max(10*eps/rtol, min(0.03, sqrt(rtol))), BDF는 CVODE식 0.1) Jacobian/LU를 Newton 반복과 step 간에 재사용한다. 검증: Tier B(XS, 먼저 적용)로 모든 comparator 행이 `comparator_fidelity`를 직렬화하고, reference-only comparator를 받은 gate가 Promote/Block 대신 `NotEvaluated`를 반환하는지 schema test로 확인한다.

#### F-053 · P2 · D6 · fairness — 어떤 timing 결정도 산포 추정을 쓰지 않으며 1.15x gate가 host noise에 넘어갈 수 있음
- **주장**: fair-ab adaptive screen은 output mode당 `Instant` 샘플 하나를 `execution.map_ordered` 안에서 기록하고, fixed screen의 병렬 경로는 non-authoritative 샘플 하나만 저장하며, authoritative timing은 마지막 warmup 샘플만으로 batch 크기를 보정한다. CLI Tier-L 결정은 CI 없이 `median(candidate/reference wall) >= 1.15`이고 smoke는 repetitions=1/warmups=0이다.
- **위치**: `crates/rodas5p-fair-ab/src/adaptive_global_error.rs:732-753`, `crates/rodas5p-fair-ab/src/adaptive_global_error.rs:1122-1123`, `crates/rodas5p-fair-ab/src/global_error.rs:1799-1809`
- **증거·재현**: `global_error.rs:1737-1751`(마지막 warmup 샘플로 batch 보정), `main.rs:783-784`(`TIER_L/TIER_N_REQUIRED_WALL_SPEEDUP=1.15`)가 근거이며, timing 보고서에 host metadata(CPU/governor/load)가 기록되지 않는다. 기대는 산포/CI 추정과 최소 검출 가능 효과 규칙이 동반된 승격 임계값이고, 관측은 1-3회 반복의 median-of-medians를 고정값 1.15와 비교하며 N=384에서 측정된 pair spread가 0.55-4.4이다.
- **영향**: Tier-L/Tier-N의 Promote/Block 판정과 stage-batch 1.15x 승격은 재현 가능한 결정이 아니며, 같은 프로토콜 약점이 CLI의 모든 wall 기반 gate에 적용된다.
- **반박 시도**: `IntegratorTimingReport::from_samples`(`global_error.rs:778-803`)가 `wall_q25`/`q75`를 기록한다는 점으로 반박을 시도했으나, 어떤 결정도 이를 소비하지 않고 smoke profile은 반복이 1회여서 q25=q75=median이므로 핵심 주장은 유지된다.
- **기존 인지**: doc — `research/generic_frozen_full_e_shadow_v36/reports/RESULT.md:56-58`(v3.6은 campaign이 host noise에 지배된다는 이유로 상수 승격을 명시적으로 보류)
- **수정안** (M, 위험 medium): `median ratio >= 1.15`를 산포를 고려한 paired timing 프로토콜로 교체한다. case당 최소 2회 warmup(batch 크기는 최소 warmup 샘플로 보정) 후 고정된 단일 thread에서 seeded random ABBA 순서로 최소 30개의 candidate/reference pair를 교차 측정하고, 통계량은 pair별 log(reference/candidate)의 median이며 seeded case-clustered bootstrap(B=10000, percentile 95% CI)을 쓰는 3값 결정으로 한다. 검증: 합성 샘플 unit test로 참 비율 1.30에 5% noise → Promote, 참 비율 1.15에 v3.6 기록 spread(pair 비율 0.547-4.43) → Inconclusive, 동일 arm → A/A CI가 1.0을 포함함을 확인한다.
- **한정**: 제목의 'No timing path produces a dispersion estimate'는 과장이다. q25/q75는 기록되지만 결정에 쓰이지 않는다는 것이 정확한 범위이다.

#### F-056 · P2 · D7 · fairness — RadauIIA3 comparator가 오차 추정용으로 trial마다 추가 dense LU를 만들고 그 비용이 부과됨
- **주장**: `radau.rs`는 오차 추정마다 `error_operator = (mu/h)M - J`를 만들고 `counters.direct_factorizations += 1`과 함께 `LuFactorization::new`를 실행하며, 이는 3n stage-system factorization에 추가된다. SciPy의 Radau는 추정에 LU_real을 재사용하므로 comparator의 집계된 direct 작업량이 충실한 구현 대비 부풀려진다.
- **위치**: `crates/rodas5p-integrators/src/radau.rs:403-414`
- **증거·재현**: `radau.rs:410-414`의 `error_operator = mass.scale(mu/h).combine(&jacobian,-1.0)`, `counters.direct_factorizations += 1`, `LuFactorization::new(&error_operator)`가 근거이다. 기대는 이미 factorize된 stage operator로 오차 추정을 푸는 것이고, 관측은 추정을 위해 trial마다 새 dense LU를 만든다.
- **영향**: 'frozen-radau-iia3' 대비 exponential/RODAS 후보의 작업량 비교(G3 gate, unified gate)는 comparator의 direct-factorization 비용을 과대평가하며, 내부 comparator가 상대 성능 주장의 근거가 될 수 없다는 H4를 지지한다.
- **반박 시도**: stage solve의 LU 재사용 가능성으로 반박을 시도했으나, stage solve는 3n dense Newton LU여서 재사용할 LU_real이 없고 이 count가 fair-ab Pareto cost metric에 들어가므로 반박되지 않았다.
- **기존 인지**: doc — `radau.rs:30-54`의 `RadauTransformLimitation::RealOnlyDenseLu`가 eigen-transform 부재를 인정한다(이중 factorization 집계는 다루지 않음).
- **수정안** (L, 위험 medium): F-052와 같은 2단계 comparator 정책이다. Tier A에서 Newton 정지 허용오차를 외부 허용오차로 스케일하고 Jacobian/LU를 Newton 반복과 step 간에 재사용한다. 검증: Tier B(XS, 먼저 적용)로 모든 comparator 행이 `comparator_fidelity`를 직렬화하고 reference-only comparator를 받은 gate가 `NotEvaluated`를 반환하는지 schema test로 확인한다.
- **한정**: flop 기준으로 추가 n^3 LU는 27 n^3 stage LU의 약 4%이다. 부풀림은 주로 count(trial당 +1)에 나타난다.

#### F-014 · P2 · D8 · leakage — reusable-preconditioner transaction의 후보 commit이 외부 참조 상태를 사용함
- **주장**: audit2 transactional attempt는 `y_new`와 외부에서 공급된 참조 상태 사이의 conservative L2 거리(선언된 불확도 포함)가 budget 이내일 때만 후보를 commit한다. 따라서 commit 결정이 solver가 runtime에는 가질 수 없는 참조 정보를 소비한다.
- **위치**: `crates/rodas5p-integrators/src/audit2_reusable_transaction_research.rs:985-999`, `crates/rodas5p-integrators/src/audit2_reusable_transaction_research.rs:565-600`
- **증거·재현**: `:985-999`에서 `output_error_l2 = audit2_conservative_l2_difference_upper(&step.y_new,&reference.state)`이고 `:591-593`에서 `output_error_upper_l2 <= output_budget_l2`가 accepted 조건에 들어간다. 기대는 인과적 solver 양(embedded 추정, residual)만 쓰는 정책 결정이고, 관측은 `committed = step.accepted && (output_accepted && embedded_accepted && original_target_accepted)`이며 `output_accepted`가 외부 참조에서 계산된다.
- **영향**: Bateman/reusable-preconditioner receipt의 disposition(Candidate vs ProtectedFallback vs Rejected)은 solver 단독 결과가 아니므로 runtime 후보 품질의 증거로 인용할 수 없다(관련 blocker: `M05`, `M06`, `M08`, `M12`).
- **반박 시도**: 영향 범위에 대해 반박을 시도했다. admission 규칙이 reference-aware로 사전등록되어 있고 Bateman authority proof가 `candidate_executions == 0`을 요구한다는 점은 확인되었으나, commit 결정이 참조 상태를 소비한다는 문자 그대로의 주장은 유지된다.
- **기존 인지**: nonclaim — `research/audit2_reusable_preconditioner_transactional_step_20260830/README.md:91-92`("This is attempt-level research plumbing. It is not a production controller or an end-to-end integration transaction.")
- **수정안** (S, 위험 low): commit 결정에서 참조를 제거하고 `committed`를 인과적 solver 양(step accepted, budget 내 embedded 추정, budget 내 original-target residual과 contraction)만으로 계산한다. reference-aware 평가는 계속 계산·기록하되 다음으로 넘기는 상태를 바꿀 수 없는 사후 audit 필드로 둔다. 검증: 테스트 `transaction_commit_is_invariant_to_reference_state`(feature `audit2-research`)에서 `reference.state`를 1e3 x `output_budget_l2`만큼 옮겨 두 번 실행하고 committed, disposition, 넘겨진 상태가 bitwise 동일한지 assert한다.
- **한정**: feature-gated(`audit2-research`) research 전용 진단 코드이다.

#### F-057 · P2 · D8 · untested-path — `max_arnoldi`가 순서만 검증되고 trace 행별로는 강제되지 않음
- **주장**: `verify_trace`는 `stage.work.arnoldi_iterations`를 `provenance.iteration_limit`으로만 제한한다(line 413). 따라서 krylov GMRES는 `total_iterations >= max_arnoldi`에서 오류를 내는데도 `arnoldi_iterations`가 (max_arnoldi, iteration_limit] 범위인 완료 행이 통과한다.
- **위치**: `overlay/untracked/crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs:378-382`, `overlay/untracked/crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs:409-423`, `crates/rodas5p-krylov/src/gmres_givens.rs:349-351`
- **증거·재현**: `audit2_stage_certificate_research.rs:413`의 `stage.work.arnoldi_iterations > u64::from(provenance.iteration_limit)`, `gmres_givens.rs:349`의 `if total_iterations >= config.max_arnoldi { return Err(`, `EXECUTION_CONTRACT.json:400-408`의 independent_of_iteration_limit=true, max_plus_one_rejects=true가 근거이다. 기대는 거부(contract의 MAX_ARNOLDI_EXCEEDED)이고, 관측은 행이 `verify_trace`를 통과해 receipt가 생성된다.
- **영향**: receipt가 solver가 생성할 수 없는 trace를 인증할 수 있고, repair contract의 MAX_ARNOLDI_* mutant가 fail-closed가 아니다(관련 blocker: `M03`).
- **반박 시도**: `validate_provenance_policy`(`:375-392`)가 행별 상한을 강제하는지 확인했으나 restart<=max_arnoldi<=iteration_limit 순서만 검사하므로 반박되지 않았다.
- **기존 인지**: doc — `overlay/predecessor_reviews/software_contract_review.json` finding P1 lines 409-414; `tree/research/audit2_stage_certificate_repair_20260831/EXECUTION_CONTRACT.json:158` TRACE_MAX_ARNOLDI_UNENFORCED, `:437-438` MAX_ARNOLDI_EXCEEDED/EQUALITY 필수 테스트; `CLAIM_LEDGER.md:8-9`
- **한정**: overlay 전용 파일이다. 선행 review가 이미 blocking P1으로 등록했고 closeout이 STOP_INVALID이므로 이 바이트에서 나온 receipt는 유효한 것으로 공개되지 않는다.

#### F-059 · P2 · D8 · untested-path — contract test의 유일한 accept가 evaluator가 검출 못 하는 거짓 kappa 전제에 의존함
- **주장**: fixture는 W = diag(2,4)에 대해 `kappa_upper = 0.25`를 공급하지만 참값 ||W^-1||_2는 0.5이며, `evaluate_audit2_stage_certificate`는 `kappa_upper`를 finite·nonnegative로만 검증한다(269-270). 따라서 `strict_lower_fixture_recomputes_a_safe_accept`(98-129, 두 weight 벡터를 0으로 두어 theta = 0)의 `SyntheticConsistentAccept`는 유효한 전제로 F03->F04->F05 chain을 시험하지 않는다.
- **위치**: `overlay/untracked/crates/rodas5p-integrators/tests/audit2_stage_certificate_contracts.rs:24-30`, `overlay/untracked/crates/rodas5p-integrators/tests/audit2_stage_certificate_contracts.rs:80-95`, `overlay/untracked/crates/rodas5p-integrators/tests/audit2_stage_certificate_contracts.rs:98-129`
- **증거·재현**: `audit2_stage_certificate_contracts.rs:26`의 `vec![vec![2.0,0.0],vec![0.0,4.0]]`와 `:87`의 `kappa_upper: 0.25`, `audit2_stage_certificate_research.rs:269-270`의 검증이 근거이다. 기대는 fixture kappa >= 0.5와 nonzero weight의 accept test이고, 관측은 kappa 0.25이며 zero weight로 theta가 0으로 강제된다.
- **영향**: 13개 테스트 suite는 certificate의 soundness 논증을 end-to-end로 검증하지 못하며, kappa는 검사되지 않는 호출자 입력(F03 전제)이다(관련 blocker: `M04`, `M06`).
- **반박 시도**: 참 kappa로 accept가 유지되는지 계산했으나, kappa=0.5와 r=(2,8)에서 kappa*||r|| = 4.123 > `caller_product_upper` 3.0이 되어 `DownwardRoundedBound`가 반환되므로 accept 자체가 거짓 전제에 의존함이 확인되었다.
- **기존 인지**: 없음
- **한정**: overlay 전용 파일이며 certificate는 `SyntheticSchemaConsistencyOnly` authority로 표시되어 있어(`contracts.rs:118-120`) 파급 범위가 제한된다.

#### F-060 · P2 · D8 · wrong-formula — stage certificate가 한 번의 linear solve residual bound를 전 stage에 복제하고 stage 수와 차원을 혼동함
- **주장**: `evaluate_audit2_stage_certificate`는 단일 (operator, rhs, approximate_solution)을 받아 모든 i에 대해 q_i = `q_upper`로 두고(line 300), strict_lower, weights, stage_traces의 길이가 `rhs.len()`이기를 요구한다(258-268). 따라서 산출되는 majorant z=(I-T)^-1 q는 stage별 q_i를 쓰는 M04/M05 bound |m|^T (I-T)^-1 q가 아니며 RODAS5P s=6, n!=6 시스템을 표현할 수 없다.
- **위치**: `overlay/untracked/crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs:258-268`, `overlay/untracked/crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs:300-301`, `overlay/untracked/crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs:598-612`
- **증거·재현**: `:300`의 `let q = vec![q_upper; dimension];`, `:268`의 `input.trace.stage_traces.len() != dimension`, `FORMAL_SCOPE.md:18-24`의 벡터 q를 쓰는 z_i=q_i+sum(j<i,A_ij z_j)가 근거이다. 기대는 stage별 solve에서 얻은 stage별 residual bound q_i와 n에 독립인 s x s T이고, 관측은 모든 stage에 하나의 `q_upper`가 쓰이고 s가 n과 같도록 강제된다.
- **영향**: 이 바이트에서 나온 모든 SyntheticConsistentAccept/Reject receipt는 stage 2..s를 stage 1의 solve에서만 계산된 양으로 제한하며, formal fixture q=(1/10,1/5,3/10)은 API로 표현할 수 없다(관련 blocker: `M04`, `M05`, `M06`).
- **반박 시도**: module header와 receipt authority(`SyntheticSchemaConsistencyOnly`)가 synthetic schema로 범위를 제한한다는 점으로 반박을 시도했으나, s=n 강제와 `q_upper` 복제라는 주장 자체는 유지된다.
- **기존 인지**: doc — overlay SC:1-5 synthetic-only header; `overlay/predecessor_reviews/formal_scope_review.json` F04 P1 'one three-stage fixture'; `overlay/predecessor_terminal/formal_adjudication.json` unmet_obligations F01/F03/F04; `PREREGISTRATION.md:104-106`
- **한정**: 선행 formal-scope review가 F04를 fixture-only로 이미 실패 처리했다.

#### F-061 · P2 · D8 · wrong-formula — directed rounding이 마지막 곱/합에만 적용되고 residual과 norm은 round-to-nearest임
- **주장**: `residual()`(516-522)과 `l2()`(532-537)는 round-to-nearest 산술로 누적하고 `next_up_nonnegative_product`/sum만 사후에 1 ulp를 더한다. 따라서 q_upper = ||x|| + kappa||b-Wx||는 cancellation이 큰 residual에서 정확한 양의 인증된 상계가 아니다.
- **위치**: `overlay/untracked/crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs:504-529`, `overlay/untracked/crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs:531-541`, `overlay/untracked/crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs:543-552`
- **증거·재현**: `:516`의 `let next = sum + entry * value;`(오차 항 없음), `:533`의 `norm.hypot(*value)` fold, `EXECUTION_CONTRACT.json:385`의 "directed_rounding_bridge_required": true가 근거이다. 기대는 receipt residual_l2 >= 정확한 ||r||이고, Python 복제 실행(`refute/scratch_04/f061_residual_undershoot2.txt`)에서 W=[[3]], x=[0.1], b=fl(0.3)일 때 코드의 ||r||=0.0, bound=0.0인데 정확값(mpmath 400-bit)은 2.78e-17로 bound < exact가 관측되었다.
- **영향**: 참 구간이 1에 걸쳐 있을 때 SyntheticConsistentAccept가 나올 수 있어 F03->F04 chain이 rigorous bound 지위를 잃는다(관련 blocker: `M04`, `M06`).
- **반박 시도**: 사후 +1 ulp가 상계를 보장하는지 실행으로 확인했으나, exact-zero 보존(next_up(0)=0) 때문에 반올림된 residual이 0으로 상쇄되면 undershoot가 전부 남아 반박되지 않았다. 테스트 fixture는 정확한 작은 정수를 써서 이를 잡지 못한다.
- **기존 인지**: doc — `tree/research/audit2_stage_certificate_repair_20260831/EXECUTION_CONTRACT.json:385` directed_rounding_bridge_required=true 및 `:430-434` policy CORRECTLY_ROUNDED_TOWARD_POSITIVE_INFINITY(contract는 요구사항을 인정하나 결함은 인정하지 않음)
- **한정**: overlay 전용 파일이다(target tree에 없음).

#### F-062 · P2 · D8 · unverified-assumption — M08 reuse certificate epsilon+delta<1이 어디서도 계산되지 않음
- **주장**: target tree의 어떤 Rust 소스도 DeltaW/preconditioner-defect certificate를 계산하지 않는다. reusable cache는 `ExactOperatorIdentity` + frozen-W sha + 선언된 identity + provider bit-match만으로 committed preconditioner를 재사용하고(246-262), transactional receipt의 유일한 defect 유사량은 caller budget(997-999)과 비교되는 residual contraction ratio final/initial(950-958)이다.
- **위치**: `crates/rodas5p-integrators/src/audit2_reusable_transaction_research.rs:246-262`, `crates/rodas5p-integrators/src/audit2_reusable_transaction_research.rs:950-958`, `crates/rodas5p-integrators/src/audit2_reusable_transaction_research.rs:995-999`
- **증거·재현**: `grep -rn -i 'epsilon *+ *delta|DeltaW|delta_w|\bdW\b' tree/crates`는 0건이고 `MATH_BLOCKER_LEDGER.json:83`만 "reuse certificate epsilon+delta<1"을 언급한다. 기대는 재사용 전에 epsilon(preconditioner defect)과 delta(DeltaW proxy)로 epsilon+delta<1을 검사하는 runtime 양이고, 관측은 재사용이 bit-identity 검사이며(같은 W → delta = 0이 자명) Jacobi map에 대한 epsilon은 계산되지 않고 changed-W 재사용은 단순히 무효화된다(264-278).
- **영향**: 이 바이트가 생성하는 어떤 receipt로도 `M08`을 평가할 수 없고, ledger의 CLOSED_CONDITIONAL 상태에는 runtime witness 경로가 없으며, 'changed-W' 시나리오는 cache 무효화만 시험하고 인증된 재사용은 시험하지 않는다.
- **반박 시도**: `tree/crates`에서 epsilon 또는 delta를 계산하는 코드를 검색했으나 0건이어서 반박되지 않았다.
- **기존 인지**: doc — `tree/research/audit2_bateman_local_validation_20260831/MATH_BLOCKER_LEDGER.json:84` remaining_evidence에 'DeltaW proxy', 'preconditioner defect'가 나열됨; `CLAIM_LEDGER.md:12-16`은 M09/M11 closure를 주장하지 않으며 ceiling은 EXPLORATORY_NONAUTHORITATIVE

#### F-064 · P2 · D8 · untested-path — provenance digest가 결정에 쓰이는 T, weights, kappa, ehat, x를 묶지 않음
- **주장**: `verify_provenance`(433-463)는 coefficient, operator, preconditioner, rhs bit만 digest하고, frozen_plan 문서는 hash 검사만 되며(255) 파싱되지 않는다. 따라서 receipt는 provenance와 plan hash가 바뀌지 않은 채 임의의 strict_lower/endpoint_weights/estimator_weights/kappa_upper/ehat/approximate_solution을 담을 수 있다.
- **위치**: `overlay/untracked/crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs:433-463`, `overlay/untracked/crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs:255-256`
- **증거·재현**: `:452-458`에서 네 개의 digest만 비교되고 `:300-306`에서 T/alpha/beta/ehat이 결정을 좌우한다. 기대는 `ProvenanceMismatch` 또는 plan 내용 불일치이고, 관측은 동일한 provenance에서 결정이 바뀐다.
- **영향**: receipt digest로 결정을 frozen plan에 귀속시킬 수 없어 선행 작업의 receipt-integrity 주장이 뒷받침되지 않는다(관련 blocker: `M06`).
- **반박 시도**: `verify_frozen_json`(`:360-366`)이 plan 내용을 검증하는지 확인했으나 canonical-sha256 동등성 검사일 뿐 문서를 파싱하지 않아 반박되지 않았다.
- **기존 인지**: doc — `overlay/predecessor_terminal/bounded_closeout.json` decision STOP_INVALID; overlay SC header lines 1-5 'neither dispatches an integrator nor makes a real-client or production claim'
- **한정**: finding의 위치 경로(TREE 기준)는 정확하지 않다. 해당 파일은 overlay/untracked에만 있고 PR #42의 tree에는 없으며, authority label은 `SyntheticSchemaConsistencyOnly`이다.

### 3.4 P2 발견 — 견고성·테스트·프로세스 (D9–D11)

#### F-003 · P2 · D10 · doc-drift — 봉인된 v3.7 consumed-N=192 replay 테스트 실패(17 vs 18), CI 미실행
- **주장**: `v37_exhaustion_is_a_charged_abstention_without_endpoint_or_failure_label`은 `completed.target_attempt_index == 18`(`research/generic_policy_redesign_v33` ledger에서 고정한 값)을 단언하지만, 대상 코드는 dev와 measurement 프로파일 모두에서 17을 낸다. 이 테스트는 `#[ignore]` 상태이고 `--ignored`를 넘기는 `.github` workflow가 없어 sealed replay 주장은 어떤 CI에서도 검사되지 않는다.
- **위치**: `crates/rodas5p-integrators/tests/v37_continuation_transaction_contracts.rs:69-71`, `crates/rodas5p-integrators/tests/v37_continuation_transaction_contracts.rs:104-112`, `research/generic_policy_redesign_v33/results/calibration192_shards/semilinear.json:1247-1247`
- **증거·재현**: `baseline/ignored_failure_note.md`에 `:110` 단언의 left=17, right=18이 두 프로파일 모두에서 기록되어 있고, `.github/workflows`에서 `ignored|v37|v3.7` grep 결과가 없다. 기대는 통과(sealed replay가 ledger의 attempt index 18을 재현)이고 관측은 `assert_eq!` 실패(left=17, right=18)이다.
- **영향**: 동결 증거로 공개된 sealed v3.3/v3.5/v3.7 ledger가 대상 코드에서 재현되지 않거나 봉인 후 코드가 drift한 것이며, ignore 속성 때문에 CI와 1075-passed 헤드라인에는 드러나지 않는다.
- **반박 시도**: 테스트 바이너리를 `--ignored --exact`로 독립 실행했으나 exit 101과 함께 `:110`에서 같은 단언 실패(30.8 s)가 재현되어 반박에 실패했고, ignore 사유가 언급하는 'replay gate'는 트리 어디에도 없다.
- **기존 인지**: doc — `research/a1_inner_tolerance_audit_20260825/CLAIM_SCOPE_AND_INVALIDATION.md:21-33`
- **수정안** (M, 위험 medium): 18을 17로 고치지 않는다. 먼저 `84a3b0f..b3e8165` 구간을 measurement 프로파일의 ignored 테스트로 bisect하여 첫 분기 commit을 찾고(로그상 후보는 `f4807d1`, `0b8b359`, `6cb005c`, `b712cde`이나 원인은 확정되지 않음), 그 다음 trajectory-pinned literal을 tombstone과 함께 퇴역시킨다. 검증: bisect 결과를 tombstone에 `sealed_commit`, `first_divergent_commit`, `observed_diff {field: target_attempt_index, sealed: 18, head: 17}`, `cause_class`(intended-numerical-change, defect, unknown 중 하나)로 기록한다.
- **한정**: refuter의 severity 관련 지적에 따르면 `README.md:43-44`와 a1 `CLAIM_SCOPE:22-33`이 v3.7 receipt를 원래의 inner-solve 정책으로 이미 한정하고 있으므로, receipt가 대상 코드에서 유효하다고 공표된 것은 아니다.

#### F-020 · P2 · D10 · untested-path — stiff order·dense output·mass·FD f_t·preconditioner 경로 테스트 부재
- **주장**: 487개 테스트 중 다음에 대한 수치 정확도 또는 convergence-order 테스트가 없다: 고정 h의 stiff 문제에서 RODAS5P order, stiff 문제에서 dense-output 오차 대 tolerance, `mass_matrix` Some의 order/정확도, FD `partial_t` 정확도, `sequential_step`을 거치는 `LinearSolverConfig.preconditioner`=Jacobi/Direct. 기존 dense-output 테스트는 PR lambda=-2, theta=0.37(order>3.8)뿐이고 preconditioner를 설정하는 테스트는 0건이다.
- **위치**: `crates/rodas5p-integrators/tests/dense_output_v2_contracts.rs:118-126`, `crates/rodas5p-integrators/tests/transactional_q1_q2_adaptive.rs:87-118`, `crates/rodas5p-integrators/tests/inner_forcing_v2_contracts.rs:98-98`
- **증거·재현**: `exp/D10/test_inventory.csv`와 `inventory_counts.json` 기준 487 tests(a=76 b=100 c=262 d=47 e=2; integrators a=49 b=65)이다. E-09에서 diagmass eps=1e-3 slope는 [3.22, 3.6, 4.78, 2.79], PR lambda=-1e4 slope는 약 3.0-3.25이고 FD f_t가 h=1/256부터 지배하며, E-04/E-09는 해당 문제에서 order 약 3-4를 보인다. 기대는 load-bearing 경로마다 order/정확도 단언 1개 이상이고, 관측은 없음이다(stiff PR -1e4는 order 단언 없는 adaptive rejection-recovery 및 transactional 테스트에만 등장).
- **영향**: stiff 문제의 order reduction(고전적 ROW 거동으로 문서화되어 있으나 README는 order 5를 주장)과 mass/FD/preconditioner 경로가 검출 없이 regress할 수 있다.
- **반박 시도**: `grep -rn 'PreconditionerKind::' crates/*/tests | grep -v None`은 0건이고, integrators 테스트의 `prothero_robinson_problem(-1` 사용처 4곳은 어느 것도 order를 단언하지 않으며 유일한 transactional order 테스트(`:137-171`)는 lambda=-20을 쓰므로 반박에 실패했다.
- **기존 인지**: 없음
- **수정안** (M, 위험 low): outer tolerance가 h와 독립인 fixed-h order 테스트 파일(`stiff_order_contracts.rs`)을 추가하여 adaptive driver가 실제로 쓰는 구성에서 production forcing rule을 검사한다. 기존 h^6-coupled 테스트는 `forced_fixed_step_with_h6_coupled_rtol_retains_order_five`로 이름을 바꾸고 'Defect caught' 주석을 정정한다(고정 rtol에서 E-04는 7회 halving 동안 slope 약 0, floor 0.1-0.8 rtol을 측정). 검증: 모든 테스트를 h = 1/2^k, slope = log2(e_k/e_{k+1})로 두고 error > 1e-12인 행만 pre-floor로 집계하며, T1 `direct_fixed_step_nonstiff_order_is_five`는 축소된 n의 `semilinear_advection_diffusion_problem`과 `constant_affine_mass_problem`을 k=3..6에서 검사한다.
- **한정**: NARROWED — '모든 mass_matrix Some order/정확도가 counter 단언뿐'이라는 부분은 comparator에 대해서는 거짓이다(BDF/Radau mass gate가 `mass_error_l2 <= floor`를 단언, `native_gates.rs:188-202`). RODAS5P 자체에 대해서는 참이며, H1b/H2 주장에는 이쪽이 해당한다.

#### F-022 · P2 · D10 · uncounted-work — generic lane에서 `jvp_calls`가 실제 JVP callback 수를 과소 집계
- **주장**: generic `integrate_adaptive_observed_with_config`+GMRES 경로에서 `jvp_calls`는 실제 JVP callback 수보다 적게 집계된다. 원인은 `OdeProblem::linearize`의 `ClosureOperator`가 `application_work`를 0으로 두기 때문이며, 이 lane을 고정하는 테스트는 없다.
- **위치**: `crates/rodas5p-integrators/tests/operator_accounting_contracts.rs:36-36`, `crates/rodas5p-core/tests/work_counter_contracts.rs:100-100`
- **증거·재현**: E-08에서 generic 경로는 `jvp_calls`=140 대 atomic 760이고, explicit-Jacobian 경우는 callback 0건에 `jvp_calls`=140이다. 기대는 모든 lane에서 `jvp_calls` == atomic count이고, 관측은 generic lane 불일치이며 기존 테스트 중 실패하는 것은 없다.
- **영향**: lane 간에 `jvp_calls`를 읽는 fairness 비교는 서로 다른 양을 비교하게 된다.
- **반박 시도**: `exp/E-08/e08_out.jsonl`을 재확인한 결과 generic arm은 140 vs 760, robertson-ramped-n96은 8841 vs 74897, PR-with-jac은 `jvp_calls`=140에 atomics.jvp=0인 반면 `mf_adaptive_reject` arm은 760=760, 29385=29385로 일치하여 핵심 주장을 반박하지 못했다.
- **기존 인지**: 없음
- **수정안** (M, 위험 medium): 서로 합산하지 않는 두 축을 가진 단일 `WorkCounters` 계약을 채택한다. PROVENANCE 축은 `jvp_calls`/`jvp_vectors`를 모든 lane에서 실제 실행된 사용자 JVP callback 호출/벡터 수로 정의하고 explicit dense J*v는 새 `jacobian_matvecs` counter로 보내며, ROLE 축은 `linear_matvecs`, `recycle_refresh_matvecs`, `diagnostic_matvecs`, `block_matvecs`이다. provenance는 call site가 아니라 operator의 속성으로 강제한다. 검증: E-08 harness(`harness/src/bin/e08_work_counters.rs`)를 `counters.jvp_calls == atomic.jvp` 규칙으로 24개 arm 전체에 재실행하여 generic GMRES arm이 140/8841 대신 760(PR)과 74897(robertson n=96)을 읽는지 확인한다.
- **한정**: NARROWED — 'integrator 수준의 유일한 `jvp_calls` 단언'이라는 표현은 거짓이다(10개 테스트 파일이 `jvp_calls`를 단언). 다만 generic GMRES lane을 고정하는 테스트는 없으므로 untested-lane 주장은 유지된다.

#### F-004 · P2 · D11 · doc-drift — v2 ledger가 dense arm global error의 tolerance 초과를 기록하지 않음
- **주장**: commit된 v2 문서 중 어느 것도, case-tolerance 가중치로 본 dense-arm global error가 n=96의 18개 행 중 12개에서 1을 초과한다는 점(semilinear에서 최대 207x)을 기술하지 않는다. solver가 accept한 embedded norm은 <=1이며, ledger는 "every case completed ... not a solver-execution crash"로만 보고한다.
- **위치**: `research/scientific_validity_v2_20260829/CANONICAL_EXECUTION_EVIDENCE.md:49-51`, `research/scientific_validity_v2_20260829/CLAIM_SCOPE_AND_INVALIDATION.md:56-63`, `README.md:36-44`
- **증거·재현**: E-02의 case-tolerance 절에서 dense max grid error <= 1인 행은 6/18이고 solver max accepted norm <= 1은 36/36 arm이며, E-03의 side observation은 17/18 행이 scipy Radau보다 나쁘고 median이 5.3x(재계산값)라고 기록한다. 기대는 failure-preserving ledger가 자기 campaign의 global-error-vs-tolerance 결과를 기록하는 것이고, 관측은 tight reference basis의 WRMS와 dominance ratio만 보고된 것이다.
- **영향**: README와 ledger의 독자는 protocol만 실패했다고 결론짓게 되며, 과학적으로 조치 가능한 결과(local control이 만족된 상태에서의 global error 증가, 특히 semilinear)가 기록에 남지 않는다.
- **반박 시도**: `exp/E-03/e03a_rows.json`의 `max_grid_wrms_casetol > 1`을 재집계하여 12/18(최대 207.48 semilinear rtol 1e-8, 1e-6에서 53.19)을 확인했고, sv2 node의 prose와 data에서 해당 표현을 찾지 못해 누락 주장을 반박하지 못했다.
- **기존 인지**: nonclaim — `research/scientific_validity_v2_20260829/CLAIM_SCOPE_AND_INVALIDATION.md:86-90`
- **수정안** (S, 위험 low): commit된 세 문서를 failure-preserving 문장으로 개정하되 숫자는 삭제하지 않고 54/54 분류는 frozen rule의 결과로 기록에 남긴다. `CANONICAL_EXECUTION_EVIDENCE.md:49-51`의 'It is a scientific output-policy invalidation'은, frozen rule `gap <= 0.1*dense error`가 독립적으로 step한 두 O(tol) 궤적(그 차이 자체가 O(tol))을 비교한다는 점과 SciPy Radau도 같은 rule을 위반한다는 독립 감사 결과를 밝히는 문장으로 교체한다. 검증: doc contract test로 ledger에 basis 선언, '12/18', '36/54', semilinear 값, 범위 한정자 'n=96' 문자열이 포함되어 있는지 단언한다.
- **한정**: CAVEAT — ledger는 solver 품질에 대한 긍정 주장을 공표하지 않는다(`README:43` 'no v2 performance, scaling, ranking…'). SINGLE-SOURCE — scipy Radau 대비 17/18, median 약 5.3x(재계산값)는 E-03 자체 표에서 가져온 값이며 독립적으로 재계산되지 않았다.

#### F-023 · P2 · D11 · fairness — v3.5 holdout 통과가 one-rank margin 안의 unsafe event 1건에 의존
- **주장**: N=320 holdout에는 inadmissible event가 1건(HIRES, zeta34=14.32)뿐이며, 이 값은 `tau_final`=13.397(frozen rule)과 `tau_selected`=16.88(calibration-optimal rule) 사이에 있다. 따라서 "SURVIVES this discriminating holdout" 판정은 post-selection one-rank backoff만으로 결정되었고 selected threshold에서는 fail이었을 것이며, positive 1건으로는 discrimination이 성립하지 않는다.
- **위치**: `research/generic_policy_redesign_v33/results/calibration_analysis/FROZEN_ZETA34_POLICY.json:1-1`, `research/generic_enforced_prefix_budget_v35/reports/PHYS_MATH_AUDIT.md:33-37`, `research/generic_enforced_prefix_budget_v35/reports/PLOT_CRAG_AUDIT.md:17-17`
- **증거·재현**: `exp/D11/v35_holdout320_tau_sensitivity.json`에서 rank_of_unsafe는 28 중 14이고 `tau_selected`는 unsafe event를 recommend한다. 기대는 one-rank threshold 섭동에 판정이 둔감할 만큼 positive가 충분한 holdout이고, 관측은 positive 정확히 1건, `tau_final`과 `tau_selected` 사이에서 판정 반전, N=384(v3.4)는 positive 0건이다.
- **영향**: zeta34 safety rule은 calibration(discovery에서 48 events / 5 unsafe) 밖에서 unsafe event 2건 이상에 대해 검증된 적이 없으므로, "SURVIVES"를 v3.6/v3.7 receipt의 검증된 safety witness로 이월해서는 안 된다.
- **반박 시도**: `research/generic_enforced_prefix_budget_v35/results/fresh_holdout320/*.json`에서 독립 재집계했으나 eligible 28, unsafe는 `hires.json`(zeta34=14.3201, err=1.1351) 1건으로 동일했고 그 값이 (`tau_final`, `tau_selected`] 안에 있어 반박에 실패했다.
- **기존 인지**: doc — `FROZEN_ZETA34_POLICY.json`의 `statistical_coverage_claim=false` 및 `safety_margin='one-preceding-distinct-zeta34-rank'`; v35 `PLOT_CRAG_AUDIT.md:19` 'Generic safety theorem: NOT CLAIMED'; v34 `PLOT_CRAG_AUDIT.md:5` 'safety discrimination is therefore untested rather than proven'(N384)
- **수정안** (L, 위험 medium): self-estimate와 safety label을 분리하고 holdout의 최소 증거 기준을 정한다. 현재 양은 `self_estimate_inadmissible`이라는 이름으로 유지하고, 모든 recommended row에 대해 같은 (t, y, h)를 tight-tolerance protected solve로 진행시킨 reference-anchored local error를 문제의 선언된 atol 벡터로 WRMS 측정하여 추가하며, 'unsafe'는 reference error > 1로 정의한다. 검증: unit test `self_estimate_and_reference_label_are_independent_fields`에서 embedded estimate 0.5, reference 차이 3 WRMS인 stub exponential report가 `reference_unsafe = true`, `self_estimate_inadmissible = false`로 라벨되는지 확인한다.

#### F-024 · P2 · D11 · unverified-assumption — v3.6 이후 process accretion: 결과 admit 0건, candidate 실행 0건
- **주장**: v3.6 이후 repository에 research node 8개(`audit2_*` 7개 + `scientific_validity_v2`)가 추가되어 Markdown 4744행과 서로 다른 schema/receipt 식별자 문자열 83개가 생겼으나, admit된 과학적 주장과 real-client candidate 실행은 0건이다. Sep-4 spiral design은 첫 과학 실행 전에 five-node V0/R/C/J/D 구조, Lean/Rocq/Wolfram/Sage/Singular 의무, 14개 task를 추가하며, 이는 자체 `STALL_PROCESS_ACCRETION` 규칙(연속 2회 process-only cycle)을 초과한다(H3 SUPPORTED).
- **위치**: `planning-branch:docs/superpowers/specs/2026-09-04-vigilode-research-coding-spiral-design.md:30-36`, `planning-branch:docs/superpowers/plans/2026-09-04-vigilode-m0-stage-certificate-closeout.md:36-40`, `research/audit2_bateman_local_validation_20260831/README.md:3-8`
- **증거·재현**: `exp/D11/evidence_ratio_summary.json`, `exp/D11/schema_families.txt`(서로 다른 문자열 83개), `exp/D11/loc_ratio.json`이 근거이다. 기대는 design 3.1에 따라 node마다 실질적 delta(theorem, measurement, counterexample, decision) 1개 이상이고, 관측은 실행된 campaign 1건(v2, protocol artifact), two-loop node 1개, 수치 결과가 없는 control/authority/handoff node 6개이며 md:json 비율은 0.005(v3.1-v3.7)에서 audit2 디렉터리 기준 약 1.06(3912 md vs 3691 json)으로 올랐다.
- **영향**: v3.6 `PASS_DESCRIPTIVE_ECONOMICS` 이후 pipeline이 admit한 과학적 결과가 없고, 계획은 E-02/E-03/E-04에 대응할 측정보다 governance(M0)를 앞에 둔다.
- **반박 시도**: 트리에서 grep과 `wc -l`로 재집계했으나 schema 문자열 83개, md 793+389+925+542+186+379+698+832 = 4744행, 14 tasks, five-node 구조, candidate 실행 0건, sv2 NOT_ADMITTED가 모두 재현되어 반박에 실패했다.
- **기존 인지**: doc — `research/audit2_stage_certificate_repair_20260831/README.md:3-6`('control-only successor ... contains no repaired science, raw backend output, candidate run, or claim promotion'); `CODEX_START_HERE.md:93-94`가 `STALL_PROCESS_ACCRETION` 규칙을 on-tree로 채택; spiral spec 3.1(planning branch)이 규칙의 원 출처
- **수정안** (M, 위험 medium): node별 receipt family를 단일 append-only ledger로 대체하고 prose에 hard cap을 두되 failure preservation은 유지한다. `research/LEDGER.jsonl`은 단일 schema(id, date, commit, command, profile, inputs_sha256, outputs_sha256, claim, verdict, supersedes)를 가지며, 실패·무효 행은 삭제하지 않고 tombstone도 하나의 행으로 기록한다. 검증: CI의 `tools/check-research-node.py`가 `research/` 아래 새 디렉터리에 pre-registration, 수치 행을 가진 results 파일 1개 이상, 그 파일을 재현하는 command와 commit을 가진 ledger 행이 없으면 실패하도록 한다.
- **한정**: CAVEAT — refuter는 caveat 2건을 언급했다. 확인 가능한 내용은 spiral spec/plan이 off-tree planning branch에 있어 fetch된 `github/` ref로만 읽을 수 있고 detached tree에는 없다는 점과 STALL 규칙의 채택일이 Sep 4라는 점이며, 나머지 서술은 입력에서 잘려 있다.

#### F-025 · P2 · D11 · untested-path — stage-certificate 모듈과 Lean/Rocq 증명이 로컬 dirty worktree에만 존재
- **주장**: M0 plan은 `audit2_stage_certificate_research.rs`와 관련 증명이 PR #42 head에 publish되어 있지 않아 보존된 dirty worktree에서 복구해야 한다고 명시한다. overlay map은 predecessor P1 3건과 P2 2건의 결함이 그곳에 byte 단위로 그대로 남아 있음을 확인하므로, 프로젝트의 가장 최근 과학적 상태는 어떤 remote ref로도 재현할 수 없다.
- **위치**: `planning-branch:docs/superpowers/plans/2026-09-04-vigilode-m0-stage-certificate-closeout.md:36-44`, `planning-branch:docs/superpowers/specs/2026-09-04-vigilode-research-coding-spiral-design.md:18-26`
- **증거·재현**: `map/overlay_formal.json`의 notes(predecessor P1/P2 발견이 여전히 존재, F01-F05는 3x3만 수정)와 planning-branch M0 plan `:38-40`이 근거이다. 기대는 scientific continuation authority가 push된 commit에 묶이는 것이고, 관측은 authority가 로컬 dirty worktree이며 design이 RED evidence가 생기기 전까지 Git commit을 금지한다는 것이다(plan:209).
- **영향**: 독립 감사자가 M0의 시작 상태를 재구성할 수 없고, worktree가 유실되면 그 상태도 유실된다.
- **반박 시도**: fetch된 remote branch들에 대해 `git ls-tree -r --name-only`로 해당 모듈을 찾아 반박하려 했으나 어느 branch에도 없었고, `overlay_formal.json`의 notes도 target에 contract만 있고 Rust/formal byte는 없다고 독립적으로 기술한다.
- **기존 인지**: doc — planning-branch `docs/superpowers/plans/2026-09-04-vigilode-m0-stage-certificate-closeout.md`의 'Source Authority Note'(lines 36-40)가 byte의 미공개를 인정하고 복구를 V0 gate로 둔다
- **수정안** (XS, 위험 low): 보존된 dirty worktree를 있는 그대로, unreviewed 상태이며 predecessor P1 3건과 P2 2건의 결함이 남아 있음을 message에 밝힌 work-in-progress branch에 commit하여 push하고, sha256을 붙인 git bundle을 기기 밖에 보관한다. RED evidence 전 commit을 금지하는 plan 규칙은 폐기한다(commit은 저장이고, authority는 push된 SHA를 인용하는 tag 또는 ledger 행에서 나온다). 검증: remote의 fresh clone에서 `git cat-file -e <sha>:overlay/untracked/crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs`가 성공하고 파일 sha256이 worktree에서 기록한 값과 일치하는지 확인한다.

#### F-067 · P2 · D9 · doc-drift — fair-ab `build.rs`가 git worktree를 필수로 요구, dirty flag가 untracked 파일에 반응
- **주장**: `build.rs`는 git을 호출하고 실패 시 `expect`/`assert`로 중단하므로 `rodas5p-fair-ab`와 `rodas5p-cli`는 source tarball에서나 PATH에 git이 없는 환경에서 빌드할 수 없다. `VIGILODE_SOURCE_DIRTY_AT_BUILD`는 untracked 파일(예: scratch note) 하나로도 true가 되며, README는 두 요구사항 어느 것도 문서화하지 않는다.
- **위치**: `crates/rodas5p-fair-ab/build.rs:6-15`, `crates/rodas5p-fair-ab/build.rs:87-101`
- **증거·재현**: `build.rs:6-15, 87-101`이 근거이고 `README.md`와 `docs/*.md`에서 `requires git|build.rs|source tarball` grep 결과가 없다. 기대는 revision 'unknown'으로 빌드 성공 또는 문서화된 오류이고, 관측은 코드 읽기상 build script가 `build.rs:11-12`에서 panic하는 것이며 실행으로 확인하지는 않았다(이 감사에서 cargo 사용 불가).
- **영향**: tarball/crates.io로부터의 downstream 재현이 불가능하고, 'clean' provenance bit가 작업 디렉터리의 무관한 파일에 의존한다.
- **반박 시도**: `build.rs`를 재검토했으나 git 호출 각각이 실패 시 `.expect`/`assert!`이고 `build.rs:87-101`이 `status --porcelain=v1 --untracked-files=normal`을 쓰며 `.gitignore`는 `/target/`, `.worktrees/`, `*.tmp`, `*.profraw`, `artifacts/generated/`, `/vendor/`만 무시하므로 반박에 실패했다.
- **기존 인지**: doc — `crates/rodas5p-fair-ab/src/scientific_validity_v2_campaign.rs:163-166`(dirty overlay 거부는 의도된 동작); `docs/superpowers/plans/2026-08-23-v37-timing-authority-validator-implementation.md:151`이 'Dirty tree ... fail capture'를 언급
- **수정안** (S, 위험 low): build script는 panic 대신 degrade하도록 하고 엄격성은 evidence를 기록하는 지점으로 옮긴다. revision은 `VIGILODE_CODE_REVISION`, git, 문자열 'unknown' 순으로 결정하고, dirty flag는 tri-state(true, false, unknown)로 하여 untracked 파일을 무시하고 build input으로 한정하며, canonical evidence writer는 revision이 unknown이거나 tree가 dirty이면 run time에 receipt 발행을 거부한다. 검증: CI에서 `git archive HEAD | tar -x -C "$RUNNER_TEMP/src"` 후 그 디렉터리에서 `cargo build -p rodas5p-fair-ab -p rodas5p-cli --locked`가 revision 'unknown'으로 성공하는지 확인한다.
- **한정**: NARROWED — `build.rs`의 git 호출은 5회가 아니라 6-7회이고, `rodas5p-cli`는 자체 `build.rs`가 없으며 fair-ab 의존성을 통해 요구사항을 물려받는다. dirty 거부는 canonical v2 runner의 의도된 provenance 정책이다(`scientific_validity_v2_campaign.rs:163-166`).

### 3.5 P3 발견 (위생)

| ID | 차원 | 유형 | 내용 | 위치 | 수정 규모 |
|---|---|---|---|---|---|
| F-019 | D1 | uncounted-work | 매 step·매 dense output 평가마다 계수 snapshot 재파싱 및 8x8 행렬 재역산 | `crates/rodas5p-integrators/src/dense_output_v2.rs:82` | - |
| F-069 | D1 | doc-drift | doc 주석은 'third-degree'이나 다항식은 theta 4차, 측정된 interpolant 차수도 4 | `crates/rodas5p-integrators/src/dense_output_v2.rs:58` | - |
| F-070 | D1 | unverified-assumption | 계수 snapshot provenance hash가 코드·테스트 어디서도 검증되지 않음 | `crates/rodas5p-core/src/coefficients.rs:143` | - |
| F-071 | D1 | untested-path | mass-matrix 문제 `constant_affine_mass_problem`에 exact solution 없음 | `crates/rodas5p-integrators/src/problems.rs:245` | - |
| F-072 | D1 | unsourced-constant | FD `partial_t` step이 sqrt(eps) (central); autonomous도 `ft_calls` 증가 | `crates/rodas5p-integrators/src/problem.rs:160` | - |
| F-021 | D10 | untested-path | `VIGILODE_CODE_REVISION` 없으면 smoke test가 NOT_RUN 통과; audit2 테스트 기본 제외 | `crates/rodas5p-fair-ab/tests/scientific_validity_v2_campaign_contracts.rs:138` | - |
| F-073 | D10 | untested-path | speed-up·order gate가 양수 여부 또는 단일 window만 assert (느슨한 assertion) | `crates/rodas5p-integrators/tests/g3_fused_adaptive_gate_contracts.rs:16` | - |
| F-027 | D11 | unverified-assumption | ledger가 approximate-Jacobian order를 미해결로 취급; W-method/ROK 문헌 미인용 | `/home/cosmosapjw/vigilode/VIGILODE_NONCODING_MATH_BLOCKERS_20260831.md:1437` | - |
| F-074 | D11 | unsourced-constant | README 예제 'accuracy check' 예산 1e-6이 예제 rtol 1e-8보다 100배 느슨함 | `crates/rodas5p-integrators/examples/solve_stiff.rs:12` | - |
| F-075 | D11 | doc-drift | CLI validate가 toolchain·backend 버전 문자열을 hard-code | `crates/rodas5p-cli/src/main.rs:1612` | - |
| F-076 | D2 | unsourced-constant | `initial_step = span/100`이 출력 간격과 같고 tolerance 비의존; `max_step = span` | `crates/rodas5p-fair-ab/src/scientific_validity_v2_campaign.rs:179` | - |
| F-077 | D2 | untested-path | `rodas5p_inner_forcing_target`의 roundoff-floor Err 분기 미테스트 | `crates/rodas5p-integrators/src/g4_s5b0_inner_tolerance.rs:69` | - |
| F-078 | D2 | unverified-assumption | reject 후 growth cap 없음: 직후 첫 accept에서 h가 `max_factor=5`까지 증가 가능 | `crates/rodas5p-integrators/src/adaptive.rs:199` | - |
| F-079 | D2 | wrong-formula | `fixed_point_error`(O(h^5) 아님)가 ^(-1/5) 지수 적용 전 embedded error에 가산 | `crates/rodas5p-integrators/src/integrate.rs:30` | - |
| F-035 | D3 | uncounted-work | 중복 true-residual matvec이 `LinearSolveReport.matvecs`에서 제외 | `crates/rodas5p-krylov/src/gmres.rs:218` | - |
| F-038 | D3 | scale-dependence | restarted GMRES(40)에 stagnation 검출 없음; E-05 s1/jacobi가 rhs scale에 의존 | `crates/rodas5p-krylov/src/gmres.rs:218` | - |
| F-080 | D3 | doc-drift | 모든 solver에서 `LinearSolveReport.converged`는 literal true, `info`는 항상 0 | `crates/rodas5p-krylov/src/gmres.rs:292` | - |
| F-081 | D3 | uncounted-work | GCRO-DR이 계수되는 `dot()` kernel 밖에서 O(n) 내적을 계산 | `crates/rodas5p-krylov/src/gcrodr.rs:505` | - |
| F-082 | D3 | untested-path | `KrylovState::invalidate_products`의 호출자가 tree 어디에도 없음 | `crates/rodas5p-integrators/src/sequential.rs:32` | - |
| F-083 | D3 | untested-path | `DirectPreconditioner`가 apply마다 LU 재분해; production 경로에서는 미사용 | `crates/rodas5p-core/src/operator.rs:370` | - |
| F-084 | D3 | wrong-formula | `GmresPrefixSession::prediction`: PC residual과 비-PC L2 threshold 혼용 | `crates/rodas5p-krylov/src/gmres.rs:446` | - |
| F-085 | D3 | unsourced-constant | Harmonic-Ritz recycle 갱신이 eigensolver 오류를 삼키고 출처 없는 절대 threshold 사용 | `crates/rodas5p-krylov/src/gcrodr.rs:233` | - |
| F-087 | D3 | wrong-formula | `safe_l2`가 all-NaN 벡터에 0.0 반환; 허위 happy breakdown·certificate 통과 가능 | `crates/rodas5p-core/src/norms.rs:3` | - |
| F-088 | D3 | stale-cache | LGMRES retain/truncate가 길이 혼합 direction 상태에서 image를 잘못 재짝지을 수 있음 | `crates/rodas5p-krylov/src/lgmres.rs:79` | - |
| F-089 | D3 | unverified-assumption | GCRO-DR 직교화가 recycle image C에는 CGS 1회, V에는 MGS 2회로 비대칭 | `crates/rodas5p-krylov/src/gcrodr.rs:578` | - |
| F-090 | D3 | unverified-assumption | solver별 breakdown 검사의 NaN/subnormal 처리 상이; LGMRES 비정규화 direction 저장 | `crates/rodas5p-krylov/src/lgmres.rs:115` | - |
| F-091 | D4 | untested-path | `dense_phi_action` 테스트가 ‖v‖ <= ~1.1만 다뤄 ‖v‖ 의존 정확도 손실이 CI에 안 보임 | `crates/rodas5p-core/tests/matrix_function_contracts.rs:45` | - |
| F-044 | D5 | doc-drift | `unified_screen`: Rejected row도 `scientifically_certified=true` literal | `crates/rodas5p-integrators/src/unified_screen.rs:731` | - |
| F-092 | D5 | unsourced-constant | G3 `legacy_to_fused_phi_action_ratio`는 측정값이 아닌 literal 15.0/5.0 | `crates/rodas5p-integrators/src/g3_fused_adaptive_gate.rs:757` | - |
| F-093 | D5 | doc-drift | Holdout profile 2종을 v3.6/v3.7이 재소비; N=2048 sealed는 profile 부재로만 강제 | `crates/rodas5p-integrators/src/g4_s5b0_regime_atlas.rs:3622` | - |
| F-094 | D6 | uncounted-work | `partial_t` 작업이 없는 autonomous 문제에서도 `ft_calls` 증가 | `crates/rodas5p-integrators/src/problem.rs:160` | - |
| F-095 | D6 | uncounted-work | `fast_accepts`가 embedded error test 전에 계상되어 embedded-reject step 이중 계수 | `crates/rodas5p-integrators/src/transactional_q1_q2.rs:806` | - |
| F-096 | D6 | uncounted-work | fixed fair-ab screen이 spec당 clipped candidate를 2회 실행하고 첫 실행 work만 보고 | `crates/rodas5p-fair-ab/src/global_error.rs:1341` | - |
| F-097 | D6 | untested-path | Jacobian-free·budget ledger invariant가 debug_assert 전용 (측정 build 미검사) | `crates/rodas5p-integrators/src/sequential.rs:697` | - |
| F-098 | D7 | doc-drift | in-tree holdout reference는 v1 schema; v2 manifest는 not-run, digest 0 | `tools/reference_v2/artifacts/oregonator-holdout-v2.json:1` | - |
| F-099 | D7 | fairness | SciPy Radau arm과 RODAS5P v2 arm이 tolerance는 같으나 step-controller 설정 불일치 | `tools/scientific_validity_v2/external_evidence.py:1066` | - |
| F-100 | D7 | unverified-assumption | Bateman oracle 정확성이 파일 byte-identity에 의존; Rust는 verifier 미실행 | `crates/rodas5p-integrators/src/audit2_bateman_real_client_research.rs:580` | - |
| F-058 | D8 | doc-drift | formal source가 F01/F03/F04는 고정 3x3·수치 fixture만 증명, F05만 quantified | `overlay/untracked/research/audit2_stage_certificate_telemetry_20260831/formal/lean/StageCertificate.lean:10` | - |
| F-063 | D8 | untested-path | Bateman Jacobi-GMRES <=2 iteration kill test(M03) 미구현, receipt로 관측 불가 | `crates/rodas5p-integrators/src/audit2_bateman_real_client_research.rs:176` | - |
| F-065 | D8 | fairness | protected fallback이 candidate에 적용되는 reference/budget check 없이 commit | `crates/rodas5p-integrators/src/audit2_reusable_transaction_research.rs:1091` | - |
| F-101 | D8 | doc-drift | rounding helper가 exact인 nonzero 값도 1 ulp 이동; contract test가 이 bump를 고정 | `overlay/untracked/crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs:561` | - |
| F-102 | D8 | untested-path | receipt readback 테스트가 in-memory struct만 비교, serde_json round trip 없음 | `overlay/untracked/crates/rodas5p-integrators/tests/audit2_stage_certificate_contracts.rs:280` | - |
| F-103 | D8 | doc-drift | audit2 'result-independent'·'matching trial stages' flag가 literal | `crates/rodas5p-integrators/src/audit2_research.rs:444` | - |
| F-104 | D8 | untested-path | directed-rounding 테스트에 exact-zero·overflow 경계 누락 (선행 P2 잔존) | `overlay/untracked/crates/rodas5p-integrators/tests/audit2_stage_certificate_contracts.rs:157` | - |
| F-068 | D9 | untested-path | fair-ab receipt의 non-finite f64가 JSON null로 직렬화되어 재읽기 불가 | `crates/rodas5p-fair-ab/src/adapters.rs:19` | - |
| F-105 | D9 | doc-drift | `OdeProblem::new`가 positional 인자 10개 (bare None 4개, bare bool 1개 포함) | `crates/rodas5p-integrators/examples/solve_stiff.rs:25` | - |
| F-106 | D9 | panic | `ShiftedOperator::new`가 W 조립에 expect 사용; non-finite h*gamma*J에서 panic | `crates/rodas5p-core/src/operator.rs:449` | - |
| F-107 | D9 | doc-drift | G3 gate row가 실패 run의 `endpoint_l2_error`를 true endpoint 기준으로 기록 | `crates/rodas5p-integrators/src/g3_fused_adaptive_gate.rs:304` | - |
| F-108 | D9 | untested-path | immutable-output publish가 filesystem 의존적 hard_link semantics에 의존, 미테스트 | `crates/rodas5p-cli/src/main.rs:828` | - |

---

## 4. 가설 판정과 실험

### 4.1 사전 등록 가설 판정

| 가설 | 판정 | 정제된 진술 | 근거 |
|---|---|---|---|
| H1a | SUPPORTED | 0.1 output-policy 기준은 독립적으로 적분한 두 O(tol) 궤적에 대해 사실상 통과 불가다. 커밋된 54/54 행 실패(최소 비율 0.1245), 같은 코드 경로 쌍 89–94% 실패, SciPy Radau 쌍 88–94% 실패. "불가능"은 과장이고, 비교 종류에 따라 18쌍 중 1–2쌍은 통과했다 | `E-01`, `E-02`, `E-03`, `F-007` |
| H1b | PARTIAL | `E-03`의 사전 등록 규칙(interpolated/accepted 비가 5배를 넘는 행이 다수)만으로는 REFUTED다(0/18). max-grid 오차가 endpoint 오차의 약 42배라는 전제는 맞지만 원인이 interpolant라는 귀속은 campaign corpus에서 반박됐다. interpolated 점 오차는 accepted-step 오차의 0.70–2.43배(중앙값 1.08)다. 남는 것은 dense output 내부 오차가 tolerance로 제어되지 않는다는 구조적 사실이다 | `E-03`, `F-002` |
| H2 | SUPPORTED | inexact Krylov stage solve에서 고정-h 5차 수렴이 유지되지 않는다. 예측한 메커니즘은 수정됐다: η는 h가 줄수록 느슨해지지만(h^-1.1 ~ h^-1.9) 0.5 clamp에는 도달하지 않았고, 원인은 h와 무관한 절대 residual 예산이다 | `E-04`, `F-008`, `F-009`, `F-031` |
| H3 | SUPPORTED | tree 전체에 schema family 문자열이 83개 있고, v3.6 이후 추가된 노드 8개의 산문이 4744줄이지만 이 노드들의 admitted 결과와 candidate 실행은 0이다(v3.7 replay receipt는 집계에서 제외). 핵심 과학 바이트가 dirty worktree에만 있다 | `F-024`, `F-025`, `F-003` |
| H4 | PARTIAL | 내부 BDF/Radau comparator는 불리한 조건이라 상대 성능 주장이 불가하고 저장소도 이를 금지한다. 계획서의 "매 Newton 반복마다 J+LU"는 반박됐다(modified Newton, solve당 LU 1회). 불리함의 크기는 측정하지 않았다 | `F-052`, `F-056`, `E-03` |
| H5 | SUPPORTED | `rodas5p-fair-ab/build.rs`가 git 명령을 `expect`/`assert`로 호출해 tarball 빌드가 불가하다. 정적 검토만 했고 실제 tarball 빌드는 시도하지 않았다 | `F-067` |

### 4.2 실험 결과

| ID | 판정 | 핵심 결과 |
|---|---|---|
| `E-01` | EVIDENCE | 커밋된 v2 54행 재분석. 54/54 `output-policy-dominated`. gap/dense 중앙값 0.944, 최소 0.1245. 54행은 (family, rtol) 18쌍의 차원 replica 3개씩이다 |
| `E-02` | PASS | n=96 18행을 대상 ref에서 재실행. arm checksum 36/36, discrepancy 18/18, step 수와 RHS/JVP counter가 커밋된 기록과 비트 동일. accepted embedded norm은 36개 arm 모두 1 이하. case tolerance 기준 전역 오차는 dense 12/18, clipped 7/18 행에서 1 초과(최대 207배) |
| `E-03` | EVIDENCE | 같은 dense 경로의 두 정책 쌍이 0.1 기준을 16/18, 17/18에서 위반. SciPy Radau 쌍은 17/18, 15/17에서 위반. interpolated/endpoint 오차 비 중앙값 1.08. VigilODE dense 오차가 SciPy Radau보다 17/18 행에서 큼(행별 비 중앙값 5.3배, 최대 438배 = semilinear `rtol=1e-8`) |
| `E-04` | EVIDENCE | advection-diffusion(n=512)에서 direct LU(5.33, 5.27, 5.15, 5.05)와 GMRES η=1e-12(5.33, 5.27, 5.13, 4.94)만 4회 halving 동안 기울기 4.5 이상 유지. η=1e-9는 1회. production rule의 h와 무관한 바닥은 Prothero-Robinson에서 약 `0.1–0.3·rtol`(`rtol=1e-8`은 최대 `1.3·rtol`), advection-diffusion에서 `0.01–0.8·rtol`. step-0 오류는 P1 `rtol=1e-10`의 모든 h, P1 `rtol=1e-8`의 h ≥ 1/256, Prothero-Robinson `rtol=1e-10`의 h ≥ 1/32. Prothero-Robinson에서는 direct LU도 약 4차(order reduction). 적응 경로의 err/rtol은 0.01–0.29 |
| `E-05` | FAIL | 보고 residual과 true residual의 비는 42행 모두 1±2e-15. 수렴 거짓 보고 0건. GCRO-DR가 operator 변경 후에도 `previous_solution`을 재사용하고, 차원 32→16 변경 시 `gcrodr.rs:463`에서 abort. LGMRES는 preconditioner 교체 후 정체하다 오류로 종료 |
| `E-06` | FAIL | `dense_phi_action`이 ‖v‖에 대해 scale-invariant하지 않음. ‖v‖=1e8인 18개 case 전부에서 k=1..4 상대오차 2.7e-9 ~ 1.6e-8(기준 2.2e-13). ‖v‖≤1은 최대 3.5e-14, `matrix_exp_pade13`은 최대 3.5e-14 |
| `E-07` | PASS | mpmath 50자리 재구성. 5차까지의 차수 조건 17/17 만족, 최대 잔차 5.5e-15(기준 1e-13). embedded는 정확히 4차. stiffly accurate 확인. `b`의 f64 반올림 약 800 ulp(cond 4.1e4)이나 차수 잔차에는 영향 없음 |
| `E-08` | FAIL | `rhs_calls`, `ft_calls`는 성공한 실행 전부에서 실제 호출 수와 일치(24개 arm 중 22개; hard error로 끝난 `fixed_direct` arm 2개는 counter를 버림). `jvp_calls`는 strict matrix-free 경로에서 일치(760=760)하지만 generic 경로에서는 140 대 760(robertson 8841 대 74897). 지수 방법 arm은 실행하지 못함 |
| `E-09` | FAIL (차수 규칙) | mass matrix W는 Direct와 GMRES 경로 모두 동작(오차 차이 1.4e-15 이하). nonstiff mass 문제는 5차. stiff case는 3–4차로 차수 규칙 실패, tolerance 규칙은 통과. FD `f_t`는 h=1/256부터 해석적 `f_t` 대비 오차 지배 |
| `E-10` | PASS | tableau 항 하나를 1e-8 상대 섭동하면 테스트 10개 실패, 1e-4면 21개 실패. controller 지수 mutant는 실행하지 않음 |

`E-08`과 `E-09`의 FAIL은 사전 등록한 판정 규칙에 따른 것이다. `E-09`의 차수 규칙은 stiff 문제의 알려진 order reduction을 고려하지 않은 규칙이었다.

### 4.3 Baseline

| 항목 | 결과 |
|---|---|
| 기본 + feature 테스트 suite | 1074 passed, 0 failed, 6 ignored |
| `clippy --workspace --all-targets -- -D warnings` | 통과 |
| `cargo fmt --check` | 통과 |
| `--ignored` 테스트 | `v37_exhaustion_is_a_charged_abstention_without_endpoint_or_failure_label` 실패 (17 vs 18), dev와 measurement 프로파일 모두 |

### 4.4 테스트 인벤토리

`#[test]` 487개를 분류한 결과다. 분류 기준과 전체 목록은 `exp/D10/test_inventory.csv`에 있다.

| 범주 | 수 |
|---|---:|
| (a) 해석해 정확도 | 76 |
| (b) 수렴 차수 | 100 |
| (c) self-consistency / regression / 결정성 | 262 |
| (d) contract / schema | 47 |
| (e) policy 동작 | 2 |

이 분류는 에이전트 한 명이 수행했고 독립 검증을 받지 않았다.

---

## 5. 감사가 확인한 정상 동작

- RODAS5P tableau는 order 5, embedded order 4 주장을 만족한다 (`/home/cosmosapjw/.local/state/vigilode/adversarial-audit/20260927T121000Z/exp/E-07/results.json`)
- mass matrix를 포함한 W-form stage equation이 올바르다 (`sequential.rs:334-365, operator.rs:475-498`)
- stage equation은 표준 k-form ROW scheme이다 (`sequential.rs:335-363`)
- direct 경로와 matrix-free GMRES 경로 모두에서 mass-matrix W = M - h*gamma*J이다 (`E-09 results.json direct_vs_gmres_agreement; operator.rs:450-475 (first round)`)
- direct-LU RODAS5P step의 nonstiff observed order는 5이다 (`E-04 p2 direct slopes 5.33,5.27,5.15,5.05; E-07 P1NS 5.44,5.73,5.87; E-09 constant_affine 5.18,5.64,…`)
- 'estimator order: 5' metadata와 controller exponent 1/5가 일치한다 (`adaptive.rs:27-33; adaptive.rs:226-229 (map integrators_A.json:41)`)
- WRMS norm과 error scale 정의가 올바르다 (`tree/crates/rodas5p-core/src/norms.rs:27-58`)
- GMRES threshold는 `||b||`를 사용한다(decoy 'rtol*||r0||'는 반박됨) (`gmres.rs:209-210, lgmres.rs:88-89, gcrodr.rs:459-460, gmres_givens.rs:312-313`)
- stopping criterion과 certificate는 선택된 norm의 unpreconditioned true residual이다 (`gmres.rs:218-233, 275-289; lgmres.rs:93-106, 179-193; gcrodr.rs:470-482, 663-677; gmres_givens.rs:32…`)
- forced-stage WRMS residual은 solve 후 forcing target과 대조된다 (`sequential.rs:506-521`)
- stagnation/budget exhaustion은 fail-closed이며 silent하지 않다 (`gmres.rs:235-240; gcrodr.rs:484-488; lgmres.rs:92 loop + 189-193; gmres_givens.rs:348-353`)
- Krylov solver의 residual 보고는 정직하다 (`exp/E-05/README.md; gcrodr.rs/gmres.rs true-residual certificate`)
- adaptive driver는 forcing-rule Err를 rejection으로 처리하고 Krylov recycle state를 복원한다(silent acceptance 없음) (`tree/crates/rodas5p-integrators/src/integrate.rs:773-812`)
- `rhs_calls`와 `ft_calls`는 정확하고, matrix-free `jvp_calls`도 정확하며, forced rejection/GMRES failure는 계수되고 roll back되지 않는다 (`$RUN/exp/E-08/results.json summary_metrics (24 arms, dev and measurement identical)`)
- transactional common-W batch(`map_ordered` with `?`)는 work를 누락하지 않는다 (`crates/rodas5p-integrators/src/transactional_q1_q2.rs:267-292`)
- v2 calibration threshold freeze는 calibration 행만 사용한다 (`crates/rodas5p-fair-ab/src/scientific_validity_v2_campaign.rs:1006-1027`)
- v3.5 N=320 holdout profile과 tolerance pair는 출력 이전에 predeclare되었다 (`tree/research/generic_enforced_prefix_budget_v35/contracts/V35_FRESH_BUDGET_SAFETY_HOLDOUT_CONTRACT.…`)
- E-02는 committed campaign 행을 bitwise로 재현한다 (`exp/E-02/results.json (36/36 checksums, 18/18 gaps, identical counters)`)
- Rayon 사용은 order-preserving이고 counter는 per-task이다 (`parallel.rs:1-40; common_w_gate.rs:23,130`)
- wall-clock front는 fair-ab scientific checksum에서 제외된다 (`crates/rodas5p-fair-ab/src/global_error.rs:1693-1700`)

---

## 6. 반박된 발견

입력 자료의 반박 사유와 근거 문자열은 일부가 잘린 상태로 제공되었다. 아래는 확인 가능한 범위만 요약한 것이다.

- **F-013** fair-ab linear runner가 기본값으로 매 solve를 이전 case의 exact oracle 해로 seed한다 — 반박 사유: 코드 독해는 정확하나 leakage라는 규정은 성립하지 않는다. 이전 case의 oracle과 계산된 해의 차이는 상대 ~3e-8(solve tolerance)인 반면, warm start가 흡수하는 case 간 oracle drift는 O(0.1)이다(`scenarios.rs:134-141`). 근거: `refute/scratch_02`에서 `rodas5p trace --kind slow-drift` 후 `rodas5p benchmark --repetitions 3`을 `--zero-guess` 유무로 실행(`bench_oracle.json` / `bench_zero.json`). `operator_total_median`은 gmres/off 2909 vs 3214, lgmres/off 3231 vs 3440.
- **F-016** RODAS5P tableau에 대한 Rosenbrock order-condition 테스트가 없다 (차수는 본 감사가 외부에서 검증) — 반박 사유: 저장소에 대수적 order-condition 테스트가 없다는 첫 절은 문자 그대로 사실이다. 그러나 "구조를 보존한 C/A/H 항목 오염이 suite를 통과한다"는 주장은 거짓이며, bit-identity 테스트는 snapshot의 단일 항목 변경에도 실패한다. 근거: `crates/rodas5p-core/tests/core_contracts.rs:40-256`의 `coefficient_snapshot_is_bit_identical_to_the_authoritative_sciml_literals`가 gamma, 모든 A 항목(54-117), C matrix(118-181), c(182-191), dense_h(192-256)를 `to_bits()` 동등성으로 고정한다.
- **F-017** `mass_matrix: Some(M)` stage solve에 대한 non-degenerate integration test가 0개다 — 반박 사유: 발견이 사용한 grep 패턴 `Some(mass`가 `Some(m.clone())`와 `Some(mass.clone())`를 놓쳤다. RODAS5P W-form stage solve는 integration test에서 non-degenerate 2x2 mass matrix로, `ShiftedOperator::apply`는 core test에서 3x3 mass로 실행된다. 근거: `crates/rodas5p-integrators/tests/transactional_q1_q2_adaptive.rs:87-110`의 `matrix_free_shifted_applications_are_folded_into_physical_jvp_and_mass_work`가 `constant_affine_mass_problem()`에 대해 `sequential_matrix_free_step`을 호출한다. `problems.rs:246`의 m=[[2,1],[0,3]].
- **F-026** README의 "complete speculative-work accounting"은 generic path에서 거짓이다 (`jvp_calls`가 5배 과소 계수) — 반박 사유: 전제 수치는 사실이나 결론은 거짓이다. generic path에서 모든 JVP callback은 `jvp_calls`+`linear_matvecs`+`diagnostic_matvecs`에 걸쳐 정확히 한 번씩 계수되고(합이 atomic count와 일치), speculative/실패 work는 두 경로 모두에서 유지된다. 근거: `exp/E-08/results.json`의 `summary_metrics.generic_path_jvp_calls_vs_atomic`. PR-jvp-only generic은 jvp=140, linear_matvecs=460, diagnostic_matvecs=160으로 140+460+160 = 760 == atomic 760이며, robertson generic row도 합이 atomic count와 정확히 일치한다.
- **F-032** `adaptive_exponential`: phi_error proxy가 L2 augmented-space residual을 `||scale||_2`(차원 의존)로 나누고, method metadata 대신 order 5를 hard-code한다 — 반박 사유: 핵심 주장(성분별 residual 고정 시 n이 2배가 되면 phi_error가 sqrt(2)만큼 줄어든다)은 거짓이다. Krylov residual estimate가 beta = `||augmented start vector||_2`를 포함하고 이 값이 같은 sqrt(2)만큼 커지기 때문이다. 남는 것은 non-uniform scale에서의 L2-ratio 대 WRMS 단위 불일치이다. 근거: `tree/crates/rodas5p-integrators/src/exponential.rs:1493`의 `beta = safe_l2(&initial)`(augmented start vector, L2), `:975-1011`의 `projected_exponential_action_with_residual_estimate`가 beta를 받는다(`reduced_input[0]=beta`).
- **F-054** Holdout numerical reference: oregonator/pollution/medical-akzo/brusselator-2d의 Rust corpus RHS가 Python `problem_runtime`과 교차 검증되지 않으며, `anchor_state_sha256`은 동일한 L2 state table을 다시 hash할 뿐이다 — 반박 사유: loader 관련 절반은 문자 그대로 사실이다. 그러나 "네 holdout family 중 어느 drift도 모든 loader check와 테스트를 통과한다"는 결론과 제안된 repro(holdout rate constant 변경)는 거짓이며, Oregonator 상수 또는 component 0-4에 들어가는 pollution 상수를 변경하면 테스트가 실패한다. 근거: `crates/rodas5p-integrators/tests/scientific_corpus_v2_contracts.rs:206-262`의 `source_equation_spot_checks_include_both_sides_of_discontinuities`가 oregonator y0=[1,2,3]에서 Rust `eval_rhs`를 평가하고 세 component 모두를 assert한다.
- **F-055** Richardson-ladder uncertainty model이 in-tree holdout artifact 네 개 모두에서 독립 LSODA lane과 경험적으로 불일치한다 (불일치가 Richardson의 4.5배~430배) — 반박 사유: 인용된 비율(333/431/9.3/4.5)은 맞다. 그러나 "geometric ladder model이 L2 reference error를 bound하지 못한다"는 추론은 비율이 가장 큰 family에서 반증된다. 다음 Radau rung은 Richardson이 예측한 위치에 있고, LSODA가 Radau보다 ~50배 덜 정확할 뿐이다. 근거: `refute/scratch_02/orego_ladder.py`(+ `.out`). Oregonator holdout(t in [0,360], 101 points, y0 [1,2,3])에서 scipy Radau L0/L1/L2(1e-8/1e-10/1e-12), 추가 rung L3(rtol 1e-13, atol 1e-15), LSODA 3e-14 및 1e-12, BDF 1e-12를 실행했다.
- **F-066** E-04가 h-independent error floor를 보인 production inner-forcing path를 다루는 audit2 certificate(M04-M07)가 없다 — 반박 사유: anchor 네 개 중 두 개가 존재하지 않는다. `StageCertificateInput`/`ehat`/`kappa` symbol과 `audit2_stage_certificate_research.rs`는 이 tree에 없고, `research/audit2_stage_certificate_telemetry_20260831` doc 폴더만 있다. 잔여 사실 내용은 audit2 arm이 고정 1e-11 stage tolerance로 실행된다는 점이다. 근거: `grep -rln`으로 `StageCertificateInput` / `ehat` / `kappa`를 `tree/crates`에서 검색한 결과 0 hits. `crates/rodas5p-integrators/src/`에는 `audit2_bateman_real_client_research.rs`, `audit2_matrix_free_research.rs`, `audit2_research.rs` 등이 있고 `audit2_stage_certificate_research.rs`는 없다. 참고: 3.5절 F-101의 위치는 `audit2_stage_certificate_research.rs:561`을 가리키므로 두 기록이 상충하며, 입력만으로는 어느 쪽이 맞는지 판정할 수 없다.
- **F-086** E-05 stale-image 시나리오는 결론이 나지 않았고 `ClosureOperator` instance-token identity는 여전히 실행되지 않았다 — 반박 사유: 발견은 `e05_raw.json`의 선행 stale row(모두 ok=false, images=0)에만 의존한다. RUN 증거에는 수렴하는 행렬에서의 재실행이 이미 포함되어 있으며, PC-swap 시나리오는 image 8개로 수렴했고 mutated-same-token 시나리오도 end to end로 실행되었다. 근거: `$RUN/exp/E-05/e05_stale_raw.json`. S1-lgmres-pc-swap-same-op는 step 0:jacobi에서 ok=True, iterations=1332, state_images_present=8(step 2, 3도 ok)이고, S1-gcrodr-pc-swap-same-op는 6개 step 모두 ok=True, state_images_present=8이다.
- **F-109** `matrix_exp_pade13`의 squaring count가 saturating float->u32 cast를 사용해 무한대 1-norm을 ~4.3e9회 squaring으로 바꾼다 — 반박 사유: cast 산술은 맞다(`inf.log2().ceil() as u32 == u32::MAX`, `u32::MAX as i32 == -1`이므로 행렬은 2배로 scale된다). 그러나 결과(4.29e9 squarings / hang)는 거짓이며, Pade numerator/denominator가 NaN/Inf guard를 가진 `DenseMatrix::combine` -> `DenseMatrix::new`를 거쳐 조립된다. 근거: `tree/crates/rodas5p-core/src/matrix_functions.rs:112-116`에서 numerator=v.add(&u), denominator=v.sub(&u)가 `:115`의 `for _ in 0..squarings` loop보다 먼저 실행된다. `matrix.rs:157-178`의 `combine()`은 `DenseMatrix::new`로 생성하고, `matrix.rs:25-27`이 Err를 반환한다.

---

## 7. 개선안

### 7.1 즉시 조치 (이번 주)

| # | 조치 | 관련 발견 | 규모 | 해제되는 것 |
|---|---|---|---|---|
| 1 | 보존된 dirty-worktree 바이트(`audit2_stage_certificate_research.rs`, 해당 contract test, Lean/Rocq 소스)를 reset/stash/rebase 없이, PR #42 publication whitelist를 건드리지 않고 원격 archival ref(non-PR 브랜치 또는 SHA-256이 붙은 signed git bundle)로 push한다. M0 plan Task 1에는 'recover from local worktree' 대신 그 ref를 기록한다. | F-025, F-067 | XS | M0 Task 1이 원격 ref에서 재현 가능해진다. 검토자가 M0 시작 상태를 다시 빌드할 수 있고, stage-certificate 코드와 증명의 유일한 사본이 단일 디스크에만 있는 손실 위험이 없어진다. |
| 2 | `rodas5p_inner_forcing_target`(`g4_s5b0_inner_tolerance.rs:64-76`)의 step-0 hard abort를 제거한다: `64*eps` 대 K-form rhs guard를 rhs scale로 표현한 residual floor와 `roundoff_floor_active` 플래그 기록으로 바꾸고, 수락 여부는 embedded estimate가 결정하게 한다(fix design G2-2). 빠져 있는 Err 분기 unit test를 추가하고 메시지에 해당 stage norm을 넣는다. | F-009, F-077, F-031, E-04 | S | `\|lambda\|~1e6` 문제에서 protected matrix-free lane의 tight-rtol(1e-8..1e-10) 실행, tight rtol의 fixed-h convergence ladder(E-04는 P1 rtol=1e-10을 어떤 h에서도 실행하지 못했다), 그리고 비정확도 기준의 5x shrink retry로 부풀려진 `rejected_steps` / `linear_solve_failures` counter의 정상화. |
| 3 | `adaptive_next_step_after_attempt`(`adaptive.rs:269`)를 고친다: output landing으로 짧아진 accepted step도 수락을 기록하고 controller factor를 clip 이전의 요청 step에 적용해야 한다. clipped arm의 accepted step 열이 rtol에 따라 달라짐을 확인하는 regression test를 추가하고, campaign `initial_step`을 `span/100` 대신 tolerance에 따라 정한다. | F-006, F-076, E-01, E-03 | S | 실제로 adaptive하게 동작하는 clipped arm. 그 전까지 scientific-validity-v2의 모든 clipped-lane 수치는 `h=span/100` fixed-step 실행이며 dense-vs-clipped 비교는 해석할 수 없다. |
| 4 | GCRO-DR: 차원 또는 operator/preconditioner identity가 바뀌면 `previous_solution`과 recycle space를 reset하고, `copy_from_slice`에서 panic하는 대신 typed Dimension error를 반환한다(`gcrodr.rs:416-464`; fix design G3-1). `recycle_dim>1` invariant test와 E-05 방식의 stale-state test를 추가한다(G3-2). | F-010, E-05 | S | fair-ab `SolverSession`과 외부 driver가 `GcrodrState`를 안전하게 재사용할 수 있다(현재는 `panic=abort`에서 프로세스 abort). E-05에서 측정된 352 대 264 iteration의 stale warm start가 없어진다. |
| 5 | sealed v3.7 consumed-N=192 replay(`v37_continuation_transaction_contracts.rs:69`)를 실행하고 `target_attempt_index` 17 대 18 불일치를 seal 이후 코드 변경 또는 잘못된 pin 중 하나로 bisect한다. 결과는 sealed ledger를 수정하지 않고 v3.7 ledger 옆에 날짜가 붙은 typed failure note로 기록하며, `--ignored` replay test와 `audit2-research` feature를 실행하는 CI job을 추가하고 `VIGILODE_CODE_REVISION`이 없을 때 campaign smoke test가 NOT_RUN 보고 대신 실패하게 한다. | F-003, F-021, F-097 | S | v3.3/v3.5/v3.7 sealed ledger의 신뢰 상태. '1074 passed(`--ignored` 실행 제외)' 헤드라인이 실행되지 않고 실패하는 sealed replay를 가리지 않게 된다. |
| 6 | scientific-validity-v2 ledger에 날짜가 붙은 addendum을 append-only로 추가한다. 내용: (a) case-tolerance weight 기준 dense-arm global error가 n=96 18행 중 12행에서 1을 넘고 semilinear advection-diffusion에서 최대 207x이다; (b) VigilODE는 18행 중 17행에서 scipy Radau에 뒤지며 median 약 5.3x(재계산값)이다; (c) 공개된 `max_grid_wrms`는 고정 (1e-10,1e-8) basis이고 renormalized table은 median 2.15, 54행 중 36행에서 >1이다; (d) `gap<=0.1*dense_error` 규칙은 구성상 도달 불가능하고 scipy Radau도 18쌍 중 17쌍에서 실패하므로 'complete-nonpassing'은 solver 정보를 담지 않는다. | F-004, F-029, F-033, F-007, E-01, E-03 | XS | 이력을 다시 쓰지 않고 P1 protocol artifact를 기록에서 제거한다. 실행 가능한 결과(local control이 만족된 상태에서의 global error 증가)가 기록에 남아 다음 node가 이를 대상으로 삼을 수 있다. |
| 7 | RED test를 먼저 작성한다: (1) Prothero-Robinson과 advection-diffusion에서 production forcing rule에 대한 fixed-rtol, fixed-h halving ladder로, matrix-free error가 direct-LU error를 명시된 factor 이내로 따라가는지 assert한다(현재 test는 rtol을 h^6에 묶어 floor를 볼 수 없다). (2) stiff observed-order test는 5를 assert하지 않고 측정된 order(PR lambda=-1e4에서 3-4)를 pinned expectation으로 기록한다. | F-030, F-018, F-020, E-04, E-09 | S | forcing rule의 test-driven 교체(구조적 변경 G2-1). E-04가 stiff PR에서는 정확한 선형대수로도 slope>=4.5에 도달할 수 없음을 보였으므로 pass 기준이 'direct LU 대비'가 된다. |
| 8 | Exponential lane P1/P2 수치: augmented Pade-13 이전에 augmentation column / Krylov seed를 `\|\|v\|\|`로 정규화하고(G4-FIX-1), happy breakdown에서는 `error_estimate=0` 대신 Sidje/KIOPS 방식의 residual estimate를 보고한다(G4-FIX-2). `\|\|v\|\|=1e8` 경우와 near-invariant-subspace 경우(`v=e1+4.5e-7 e2`)를 test로 추가한다. | F-040, F-011, E-06 | M | 물리 단위에 의존하지 않는 phi_k 정확도(E-06: `\|\|v\|\|=1e8`에서 relative error 2.7e-9..1.6e-8, threshold는 2e-13). `adaptive_exponential`에서 약 1e-6 미만의 relative tolerance를 강제할 수 있게 된다. |
| 9 | 입력 검증 sweep: scalar Krylov config에서 NaN/Inf rtol/atol을 거부하고(현재 `atol=+Inf`는 x=0을 certify한다), `safe_l2`가 0.0을 반환하지 않고 NaN을 전파하게 한다. seeded shared-basis GMRES에서 가장 큰 preconditioned RHS가 ~0일 때 true-residual certificate를 요구하고, `ShiftedOperator::new`의 expect를 typed error로 바꾼다. | F-034, F-087, F-037, F-106 | XS | non-finite 또는 degenerate 입력이 passing certificate가 되는 남은 경로를 닫는다. `converged`라는 이름의 receipt field를 신뢰하기 위한 선행 조건이다. |
| 10 | RODAS5P coefficient snapshot을 매 step과 매 dense-output sample마다 JSON 재파싱 및 8x8 행렬 재역산하지 않고 `OnceLock`으로 한 번만 load·invert한다. 그 단일 load 시점에 carried provenance hash를 검증한다. | F-049, F-019, F-070 | XS | 모든 small-n timing을 RODAS5P에 불리하게 치우치게 하는 고정 per-step overhead가 없어진다. provenance hash가 carried text가 아니라 실행되는 check가 된다. |

### 7.2 구조적 변경

| # | 변경 | 관련 발견 | 규모 | 근거 |
|---|---|---|---|---|
| 1 | production inner-forcing rule `tau = 0.1/\|\|b\|\|_1`(`rhs_wrms >= flow_wrms`이면 h와 rtol에 무관한 절대 stage-residual budget)을 embedded error estimate / local error target에 맞춰 scale되는 forcing term으로 교체하여, 허용되는 stage residual이 truncation error와 함께 줄어들게 한다(fix design G2-1). 세 호출 지점(`integrate.rs:774`, `dense_output_v2.rs:585`, `g4_s5b0_regime_atlas.rs:1631`)에 하나의 함수로 적용한다. | F-008, F-005, F-031, F-033, E-04 | M | E-04: 현재 rule에서 fixed-h error는 0.1-0.3*rtol의 floor이고 5회 이상의 halving에서 slope <= 0이며 eta는 h^-1.9로 커진다. v3.5-v3.7의 모든 receipt와 v2 campaign이 이 rule을 거쳤고, embedded estimator는 inexact-solve error를 볼 수 없다(F-033). scipy Radau 대비 5.3x gap의 가장 유력한 기여 요인이며, 그 위에 certificate를 쌓기 전에 고쳐야 한다. |
| 2 | K-form stage equation은 당분간 유지하되, 모든 relative inner criterion을 `\|\|rhs\|\|`가 아니라 solution-scale 기준(`\|\|K_i\|\|` 또는 error scale)에 대해 서술한다. `\|\|rhs\|\|`는 stiff mode에서 O(h\|lambda\|)만큼 크다. U-form(rhs에 `h*J*sum gamma_ij K_j`가 없는 형태)은 별도의 이후 lane으로 decision note에 기록한다(fix design G2-3). | F-031, F-008, F-009 | S | E-04는 h=1/8에서 `rhs_wrms/flow_wrms = 40`, `rhs_wrms` 최대 5e11을 측정했으므로 fraction-of-rhs 기준은 그 factor만큼 느슨하다. U-form 재작성은 tableau replay, dense output, JVP accounting을 건드리므로(effort L, high risk) forcing-rule 수정과 묶지 않는다. |
| 3 | output-policy admissibility protocol을 재설계한다: 각 arm을 case 고유의 (atol, rtol) weight로 reference에 대해 선언된 상수와 함께 판정하고(error <= C*tol), policy equivalence는 의미가 있는 경우(같은 accepted-step 열, dense evaluation 대 landed output)에만 test한다. 거의 동일한 3개 dimension replica를 합쳐 54행을 18개 독립 case로 보고한다. | F-007, F-029, F-006, E-01, E-03 | M | 삼각부등식에 의해 `clipped_max < 0.9*dense_max`인 행은 구성상 Dominated이다. E-01은 54행 전부에서 gap/dense >= 0.1245를 확인했고 E-03은 scipy Radau가 18쌍 중 17쌍에서 같은 규칙에 실패함을 보였다. 이 gate는 두 O(tol) realisation의 차이를 재며 solver 품질을 재지 않고, 여기서 유도된 holdout threshold에는 도달할 수 없다. |
| 4 | stage-certificate evaluator를 client에 쓰기 전에 고친다: stage별 residual bound q_i(한 linear solve의 bound를 전 stage에 broadcast하지 않음), stage count와 state dimension의 분리, 마지막 product/sum만이 아니라 residual과 norm 계산에 대한 directed rounding, 실제로 계산되는 M08 reuse quantity epsilon+delta, trace row별 `max_arnoldi` 강제, evaluator가 확인할 수 있는 kappa premise, 그리고 T, weights, kappa, ehat, x를 bind하는 digest. | F-060, F-061, F-062, F-059, F-057, F-064, F-101, F-104 | M | certificate가 계산하는 내용 자체의 결함이므로 Lean/Rocq 정리의 일반성과 무관하게 certified accept가 틀릴 수 있다. M0 plan은 소프트웨어 항목 세 개(`max_arnoldi`, serde round trip, upward rounding)를 닫지만 F-060, F-062, F-064는 닫지 않는다. |
| 5 | reusable-preconditioner transaction에서 reference-in-the-loop acceptance를 제거한다: commit 결정은 runtime 양(embedded estimate와 contamination bound)만 사용하고 reference distance는 post-hoc 평가 column이 된다. protected fallback에도 같은 평가를 적용하여 두 disposition을 동일하게 판정하고, Rust가 checked-in 파일과의 byte identity를 assert하는 대신 Bateman oracle verifier를 실행하게 한다. | F-014, F-065, F-100 | S | reference를 소비한 Candidate/ProtectedFallback/Rejected disposition은 runtime 품질의 증거로 인용할 수 없다. rubric의 P1급 leakage 패턴이며 현재는 `audit2-research` feature gate로만 격리되어 있다. |
| 6 | work-counter contract를 하나로 정의하고 두 lane 모두에서 test로 고정한다: `jvp_calls` = JVP callback 횟수이며 `linear_matvecs`와 별도로 보고한다; block GMRES는 8 JVP / 8 LU solve를 8로 계상한다; 실패·거부된 attempt(legacy atlas `run_trajectory`, exponential trial)의 work를 유지한다; true-residual matvec과 GCRO-DR inner product를 센다; fair-ab screen은 두 실행을 모두 보고한다. | F-048, F-022, F-051, F-050, F-039, F-035, F-081, F-095, F-096, F-094, E-08 | M | E-08: `rhs_calls`와 `ft_calls`는 24개 arm 전부에서 정확하지만 `jvp_calls`는 generic 경로와 matrix-free 경로에서 의미가 달라 현재 cross-lane cost ratio는 비교할 수 없다. design section 7이 마지막에 두는 performance study보다 먼저 이루어져야 한다. |
| 7 | Dense output: interpolant를 nonstiff order 4 / stiff order ~3으로 문서화하고, 선택적 interior error check(embedded solution 또는 midpoint defect sample과 비교)와 결과의 typed `dense_uncontrolled` 플래그를 추가하며, stiff dense-output accuracy test를 추가한다. | F-002, F-069, F-020, E-03 | M | F-002는 Prothero-Robinson lambda=-1e5에서 endpoint는 통과하면서 interior error가 tolerance의 17x-610x임을 측정했다. E-03은 v2 campaign 행에서 interpolation이 지배적 오차라는 가설을 반박하므로(grid/accepted ratio median 1.08, max 2.43) forcing rule과 protocol보다 순위가 낮지만, 매우 stiff한 문제에서 실제로 빠져 있는 control이다. |
| 8 | Exponential lane: augmented norm이 아니라 operator norm에서 Pade squaring 횟수를 고르는 scale-aware fused phi combination(G4-FIX-3); Krylov threshold와 phi error proxy를 time error와 같은 WRMS 단위로 통일(G4-FIX-4); stiff, non-scalar, non-commuting order test(G4-FIX-6); method 자신의 embedded estimate를 atlas 'unsafe' label로 쓰는 것을 중단한다. | F-042, F-043, F-041, F-045, F-091, E-06, E-08 | M | `pexprb54s4`의 order 증거는 Krylov space가 exact인 scalar `y'=y^2` test에 의존한다. E-06은 단위 의존적 정확도를 보였고, E-08은 exponential arm이 non-autonomous 문제를 거부하여 두 audit 문제 어느 쪽에서도 측정할 수 없었음을 보였다. |
| 9 | GMRES/LGMRES/GCRO-DR이 공유하는 단일 Krylov state-validity model: warm start, recycle basis, augmentation direction을 모두 (dimension, operator identity, preconditioner identity)로 key하고 불일치 시 폐기한다. restarted GMRES에 stagnation detection을 추가하고 breakdown/NaN 처리를 통일하며, `gmres_givens.rs`는 삭제하거나 production kernel로 만든다. | F-010, F-088, F-082, F-038, F-089, F-090, F-085, F-036, E-05 | M | E-05: residual 보고는 정직하고 모든 solver가 fail closed이므로(converged 42/42 행이 재계산 residual과 일치) 결함은 state reuse에 있다. operator를 변경한 GCRO-DR은 4126 대 271 matvec이고, LGMRES는 preconditioner 교체 후 fresh state라면 수렴하는 6개 solve 중 3개에서 실패한다. |
| 10 | Comparator fairness: 내부 BDF/Radau comparator에 tolerance-proportional Newton tolerance와 Jacobian/LU reuse를 적용한다. ported error estimate의 추가 factorization을 명시 없이 RadauIIA3에 계상하지 않고, SciPy arm과 step-controller 설정을 맞추며, 1.15x Tier-L rule 적용 전에 모든 timing 경로가 반복 paired run의 dispersion estimate를 보고한다. | F-052, F-056, F-053, F-099, F-073 | M | 이것들이 없으면 내부 comparator는 handicap을 지고 1.15x promotion threshold는 host noise로 넘을 수 있으므로, 어떤 speed claim도 공개 시 P0에 취약하다. |

### 7.3 연구 로드맵 변경

1. **변경**: inner-forcing rule과 fixed-step order-preservation 실험(design section 7, items 1-2)을 M1 Bateman real-client 실행 앞으로 옮긴다. 새 순서는 forcing-rule RED test -> forcing-rule 교체 -> direct LU 대비 fixed-h ladder -> M1이다.
   - 근거: E-04는 production rule이 h에 무관한 error floor를 가지며 legal input에서 abort함을 보였고, 지금 만든 Bateman certificate는 h가 줄수록 커지는 tolerance로 푼 stage를 certify하게 된다(design section 7은 'when newly observed evidence creates a higher-priority blocker'일 때의 재배치를 허용한다).
   - 현재 계획과의 충돌: design section 6('M1 opens only if M0 is verified. It must not be replaced by another planning/audit node')과 이 lane들을 real-client evidence 뒤에 두는 section 7 기본 순서와 충돌하며, plan Task 14(M1 Bateman preflight)가 연기된다.
   - 관련 id: F-005, F-008, F-009, F-030, E-04
2. **변경**: output-policy protocol 재설계와 clipped-controller 수정을 추가 campaign, calibration, holdout 실행보다 앞에 둔다. `gap<=0.1*dense_error` gate는 append-only invalidation note로 폐기하고 현재 규칙으로는 v2 holdout을 열지 않는다.
   - 근거: 이 gate는 두 번 실행한 어떤 adaptive solver에도 도달 불가능하고(scipy Radau는 18쌍 중 17쌍에서 실패), clipped arm은 fixed-step 실행이며 metric basis가 case tolerance가 아니므로, 이 protocol에 holdout을 쓰면 정보 없이 소모된다.
   - 현재 계획과의 충돌: design section 7 item 5(transactional retry / event / dense-output semantics는 기본 순서에서 다섯 번째)와 'improve output-policy agreement'라는 v2 ledger 지침(duplicate finding F-001)과 충돌한다.
   - 관련 id: F-007, F-006, F-029, F-076, E-01, E-03
3. **변경**: blocker M09를 'NOT_GENERICALLY_CLOSED, needs a method-specific W-order proof'에서 'answered by existing theory; remaining work is an empirical order campaign'으로 재분류한다. M09의 `remaining_evidence`에서 proof 선택지를 빼고 perturbation/order campaign만 남긴다: exact J 대 stale J 대 Krylov-projected J, fixed h, direct-LU arm 대비 측정.
   - 근거: RODAS5P는 order 5가 exact Jacobian을 가정하는 Rosenbrock(ROW) method이며, W-method order condition(Hairer-Wanner vol. II, sec. IV.7)과 Rosenbrock-Krylov 이론(Tranquilli and Sandu, SISC 36, 2014)이 Jacobian 근사의 비용을 이미 서술하고, E-07은 tableau가 exact J에서 order 5(4)임을 확인하므로 증명할 generic fifth-order arbitrary-J 정리는 없다(인용 세부는 audit brief와 F-027에서 가져온 것이며 이번 pass에서 원문과 대조하지 않았다).
   - 현재 계획과의 충돌: blocker ledger M09(priority P0, `remaining_evidence` 'method-specific W-order proof or perturbation/order campaign')와 design section 7 item 3(theory node로서의 'approximate/stale Jacobian effects and Rosenbrock/W/Krylov order conditions')과 충돌한다.
   - 관련 id: F-027, F-018, E-04, E-07, E-09
4. **변경**: blocker M07(fifth-order algebraic tolerance scaling)을 다시 서술한다. 가설 'RHS = O(h)'는 stiff mode의 K-form stage equation에서 성립하지 않으며(rhs가 O(h|lambda|)만큼 크다), acceptance criterion은 'slope >= 4.5'가 아니라 'inexact arm within a stated factor of the direct-LU arm over the ladder'여야 한다.
   - 근거: E-04 P1PR에서 direct-LU arm 자체가 order ~4(slope 4.01, 4.03, 4.03, 3.63)이고 E-09는 PR lambda=-1e4에서 order ~3, stiff diagonal mass matrix에서 3.2-4.8을 보였으며, 이는 classical stiff order reduction이므로 fifth-order slope 기준은 모든 arm에서 실패하여 forcing rule을 판별하지 못한다.
   - 현재 계획과의 충돌: blocker ledger M07(theorem 'if RHS=O(h), relative residual O(h^5)'; remaining evidence 'fixed-h refinement, uniform amplification window')과 충돌한다.
   - 관련 id: F-031, F-018, F-008, E-04, E-09
5. **변경**: M0의 formal scope를 줄이고 방향을 바꾼다. F01/F03/F04에 대해 arbitrary n의 universal authority는 하나(Lean)만 유지하고, 중복된 Rocq track과 Wolfram/Sage/Singular cross-check record(plan Task 6, 13 backend-role records)를 뺀다. 회수한 노력은 정리가 다루지 않는 evaluator 결함 F-060/F-061/F-062에 쓴다.
   - 근거: F01/F03/F04는 기초적인 유한차원 사실(strict-lower nilpotency, Neumann inverse, nonnegative majorant recursion)이어서 두 번 증명해도 solver에 대한 증거가 늘지 않으며, certified accept를 틀리게 만들 결함은 Rust evaluator에 있다(전 stage에 broadcast된 하나의 residual bound, round-to-nearest residual과 norm, 계산되지 않는 reuse quantity).
   - 현재 계획과의 충돌: design section 5('Lean/mathlib and Rocq remain universal authorities ... Wolfram/Sage/Singular remain role-limited exact cross-checks')와 plan Tasks 3-6, 그리고 M0가 나열된 C01 항목 세 개만 닫는다는 plan constraint와 충돌한다.
   - 관련 id: F-058, F-060, F-061, F-062, F-024
6. **변경**: Bateman client를 'first real-client scientific execution'에서 kill test로 낮춘다. 첫 scientific execution은 inexactness가 실제로 작동하는 client, 즉 corpus에 이미 있는 semilinear advection-diffusion family와 Prothero-Robinson ladder로 한다.
   - 근거: ledger 자신의 M03 정리에 의해 Bateman에서 Jacobi-preconditioned GMRES는 <= 2 iteration에 종료하므로 stage residual이 roundoff 근처여서 contamination certificate가 stress를 받지 않고, <=2-iteration kill test는 구현되어 있지도 receipt에 보이지도 않으며(F-063), 측정된 결함은 다른 곳(semilinear 행이 모든 step이 accept된 채 case tolerance의 17-207x 밖)에 있다.
   - 현재 계획과의 충돌: design section 6('Primary target: the already-constructed Bateman client lane')과 ledger X01(P0 non-mathematical blocker로서의 six-case Bateman candidate run)과 충돌한다.
   - 관련 id: F-063, F-100, F-014, F-033, E-03, E-04
7. **변경**: forcing-rule 수정 바로 뒤에 새 node 'global error under satisfied local control'을 넣는다. n=96 18행(E-02가 bitwise로 재현한다)을 direct LU, 새 forcing rule, scipy Radau로 다시 실행하여 5.3x median / 최대 438x(semilinear) gap을 inexact stage, stiff order reduction, controller 설정 중 어디에 귀속시킬지 정한다.
   - 근거: 이는 audit이 산출한 유일한 solver-quality 결과이며 plan에 없고, E-03이 interpolation을 원인에서 배제했으며 모든 arm에서 embedded norm이 <= 1이므로 원인은 controller의 상류에 있다.
   - 현재 계획과의 충돌: Sep-4 design과 plan에 없는 node로 M0와 M1 사이에 놓이게 되며, plan constraint 'Candidate executions: exactly 0 throughout M0'는 이 node가 M0 이후 별도 node로 실행될 때만 지켜진다.
   - 관련 id: F-004, F-033, F-052, E-02, E-03
8. **변경**: M11(nonnormal GMRES)을 convergence-theory 문제에서 engineering node로 rescope한다: stagnation detection, receipt의 true-residual history, fallback policy. 위치(design section 7 item 4)는 유지하되, 정직한 residual을 가진 채 실패하는 client가 나오지 않는 한 field-of-values / pseudospectral probe는 뺀다.
   - 근거: ledger는 universal fast-convergence 정리가 없음을 이미 기록하고 있고, E-05는 solver가 true residual을 올바르게 보고하고 fail closed임을 보였으므로 빠진 것은 이론이 아니라 stagnation 감지와 안전한 state reuse이다.
   - 현재 계획과의 충돌: ledger M11의 `remaining_evidence`('FOV or pseudospectral probe') 및 P0 priority와 충돌하며, design의 순서와는 일치한다.
   - 관련 id: F-038, F-010, E-05
9. **변경**: reuse quantity epsilon+delta가 runtime에 계산되고 acceptance가 reference를 읽지 않게 될 때까지 M08(cross-step / retry reuse)과 모든 reusable-preconditioner claim을 보류한다. 순서는 forcing rule과 Krylov state-validity model 뒤로 둔다.
   - 근거: 현재 reuse는 identity equality만으로 허용되고, E-05는 preconditioner 변경을 넘는 reuse가 수렴하는 solve를 실패로 바꿀 수 있음을 보였으며, 평가되지 않는 reuse certificate는 claim을 뒷받침할 수 없다.
   - 현재 계획과의 충돌: ledger M08 status CLOSED_CONDITIONAL(정리는 유효하나 runtime evidence가 없다) 및 PR #38/#39 substrate framing과 충돌한다.
   - 관련 id: F-062, F-014, F-065, E-05
10. **변경**: v3.5의 'SURVIVES this discriminating holdout' 결과를 'one unsafe event, inside the one-rank margin, non-discriminating'으로 낮추고, Holdout512 / Holdout320 / Holdout384를 consumed로 표시하며, `FrozenK1Comparator`를 holdout evidence에서 뺀다. v2.5/v2.9 policy constant는 각각 in-repo derivation이 생길 때까지 확장하지 않으며, 새 policy-safety claim 전에는 fresh holdout이 필요하다.
    - 근거: positive 하나로는 판별할 수 없고, frozen k=1 comparator는 N=512 holdout에서만 나올 수 있는 step index를 hard-code하며, holdout profile은 v3.6/v3.7에서 같은 이름으로 다시 소비된다.
    - 현재 계획과의 충돌: sealed v3.5 ledger의 문구 및 이를 validated safety witness로 인용하는 v3.6/v3.7 receipt와 충돌하며, sealed 파일은 수정하지 않고 note를 append한다.
    - 관련 id: F-023, F-047, F-093, F-046
11. **변경**: 단기 로드맵에서 exponential-lane promotion과 atlas recommendation label, production polyalgorithm selection, 모든 speed comparison을 뺀다. performance study는 design대로 마지막에 두되, 비용이 작은 선행 작업(counter contract, coefficient caching, timing dispersion)은 performance claim이 아니라 accounting의 정확성이므로 앞으로 옮긴다.
    - 근거: exponential lane에는 P1 error-estimate 결함과 단위 의존적 정확도가 있고, `jvp_calls`가 하나의 의미를 가질 때까지 lane 간 cost ratio는 비교할 수 없다.
    - 현재 계획과의 충돌: accounting 수정을 M0 안에 넣으면 plan constraint 'No performance, speedup, scalability ... work belongs to M0'와 충돌하므로 M0 밖에 일정을 잡으며, design section 7 item 7과는 일치한다.
    - 관련 id: F-045, F-011, F-040, F-041, F-048, F-053, E-06, E-08

### 7.4 프로세스 변경

1. design 자신의 section 3.1 규칙을 repo 안의 counter로 강제한다: 각 node는 numerical measurement, counterexample, merged code change 중 하나를 산출해야 하고, M0는 evaluator 수리를 포함하지 않으면 process cycle로 센다. node의 prose는 `RESULT.md` 하나와 receipt 하나로 제한하고, schema family는 하나를 폐기하지 않고는 새로 만들지 않는다. (관련 id: F-024, F-025)
2. gate를 freeze하기 전에 attainability check를 한다: 제안된 기준을 (a) control solver(scipy Radau)와 (b) 같은 code path의 perturbation 쌍(h0 대 0.7*h0)에 실행하고, control이 실패하는 gate는 설계 단계에서 기각한다. 이 check를 pre-registration에 기록한다. (관련 id: F-007, F-030, E-03, E-04)
3. node별 수동 verification bundle을 하나의 CI matrix로 대체한다: default test, `--ignored` sealed replay, `audit2-research` feature, 그리고 `debug_assert` invariant를 checked assertion으로 승격한 measurement-profile 실행. 실패한 sealed replay는 CI가 ledger 옆에 쓰는 typed failure file이 된다. (관련 id: F-003, F-021, F-097, F-020)
4. literal receipt field를 금지한다: receipt의 모든 boolean 또는 ratio(`scientifically_certified`, `result_independent`, `matching_trial_stages`, `converged`, `info`, `legacy_to_fused` ratio)는 run data에서 계산하거나 제거한다. receipt constructor에서 literal `true` / 상수 ratio를 grep하는 lint test를 추가한다. (관련 id: F-044, F-103, F-080, F-092, F-107)
5. node별 CLAIM_SCOPE 문서를 README의 claims table 하나(claim, evidence file, units, status)로 대체한다. 모든 error 수치는 case 고유의 (atol, rtol) weight로 보고하고 order claim은 'nonstiff 5, stiff 3-4 observed'로 적으며, table에 행이 없는 claim은 하지 않는다. (관련 id: F-029, F-004, F-074, F-018)
6. holdout consumption register(JSON 파일 하나: profile, first-opened revision, consumers)를 둔다. 소비된 holdout은 코드에서 'replay-N'으로 이름을 바꾸고 다시 holdout evidence로 인용하지 않으며, in-tree reference artifact는 generator identity와 `wrms_basis`를 갖거나 legacy로 표시한다. (관련 id: F-093, F-047, F-023, F-098)
7. publish-first 및 rebuildable-from-remote 규칙: 어떤 plan step도 local worktree에만 있는 바이트에 의존하지 않는다. `build.rs`는 git이 없으면 'provenance unknown'으로 degrade하고 dirty를 tracked file에서만 계산하며, receipt는 test에서 `serde_json` round-trip을 거치고 non-finite 값을 typed error로 거부하며 decision-driving input을 digest한다. (관련 id: F-025, F-067, F-068, F-102, F-064, F-108)
8. constants register: 모든 policy 또는 numerical constant(V25/V29 threshold, `tau=13.397`, FD step `sqrt(eps)`, harmonic-Ritz threshold, controller growth cap, controller의 `fixed_point_error` 항)에 한 줄짜리 출처(derivation, 문헌, 또는 'tuned on <dataset>')를 붙인다. tuned constant는 자신이 본 데이터를 명시한다. (관련 id: F-046, F-072, F-085, F-078, F-079)

---

## 8. Nonclaims와 한계

### 8.1 이 감사가 주장하지 않는 것

- 감사는 P0를 찾지 못했다: VigilODE의 공개된 claim, receipt, result 중 거짓으로 드러난 것은 없다. 확정 발견은 P1 4, P2 42, P3 49이다. 동시에 감사는 receipt를 certify하지 않는다: bitwise로 재현된 것은 n=96 v2 18행뿐이다(E-02).
- VigilODE의 wall-clock performance, efficiency, SciPy 또는 다른 solver 대비 순위에 대해서는 아무 진술도 하지 않는다. timing 실험은 실행되지 않았고 comparator handicap은 정량화되지 않았으며(F-052, F-056, F-053), tree 자체가 그런 claim을 금지한다.
- RODAS5P tableau와 stage equation은 결함이 없다. stiff Prothero-Robinson과 mass-matrix 문제에서 관측된 order 3-4는 classical Rosenbrock order reduction이다(E-07 PASS: order condition 17개가 2e-15 이내; E-04/E-07/E-09에서 nonstiff order 5 관측; E-09 tolerance rule 통과).
- production inner-forcing rule이 adaptive 경로에서 tolerance 밖의 결과를 낸다는 것은 보이지 않았다. E-04 adaptive err/rtol은 두 문제 모두 0.01-0.29이며, 결함은 fixed-h convergence 손실과 abort이다(F-008, F-009).
- H1b의 반박은 dense output이 tolerance-controlled라는 뜻이 아니며, F-002는 campaign의 dense-arm error를 설명하지 않는다. E-03c(campaign corpus)와 F-002(synthetic PR, Python replica)는 서로 다른 regime을 다룬다.
- F-011, F-002, F-042는 Rust 코드 실행이 아니라 Rust 알고리즘의 Python replica에서 시연되었다.
- F-010(GCRO-DR panic)과 F-037은 in-tree integrator 경로에서 도달할 수 없다. F-011, F-039-F-043은 protected RODAS5P solver가 아니라 candidate exponential lane에 영향을 준다.
- exponential integrator의 stiff order, work-counter exactness, prefix-resume equivalence에 대해서는 아무것도 주장하지 않는다. E-08 exponential arm은 실행되지 않았고 E-04에는 exponential arm이 없으며 nonstiff order만 확인되었다.
- finite-difference JVP 사용자와 controller-formula test adequacy에 대해서는 아무것도 주장하지 않는다. E-04 FD-JVP arm과 E-10 mutant B는 실행되지 않았다.
- v3.7 replay 불일치(17 대 18)의 원인은 알 수 없다. 감사는 v3.7 ledger가 생성 당시 틀렸다고도, drift가 무해하다고도 주장하지 않는다(F-003은 bisect되지 않았다).
- overlay 발견(D8 stage certificate, Lean/Rocq)은 untracked local 바이트의 static reading이며 formal proof는 재검증되지 않았다(PLAN limitation 'overlay static only'; F-025).
- v2 campaign에 대한 통계는 54개 독립 행이 아니라 18개 effective case를 가리킨다. E-01: 54행 = 18개 (family, rtol) 쌍 x 거의 동일한 dimension replica 3개.
- 반박된 발견(10건)은 서술된 claim이 성립하지 않았다는 뜻이지 해당 영역이 positive하게 검증되었다는 뜻이 아니며, verified_ok 항목은 spot check이지 증명이 아니다. 예: reference uncertainty model(F-054/F-055 반박)은 positive verification이 없고, `deny_unknown_fields` 항목은 coverage 통계이다.
- 상위 발견 11건 중 7건은 finder가 P1으로 제출했으나 judge가 P2로 낮췄다. 보고서의 P1은 4건이다(F-007, F-009, F-010, F-011).
- E-04의 P1PR 결과는 exploratory이며(P1을 본 뒤 추가된 variant), E-09의 FAIL은 PLAN의 tolerance criterion이 아닌 order rule을 사용한다(E-04 README:54, E-09 README:13 대 PLAN:116,121).

### 8.2 계획 대비 미실행·변경된 실험

| 실험 | 계획 | 실제 | 결론에 미치는 비용 |
|---|---|---|---|
| E-02 | `output_policy_discrepancy_wrms`의 bitwise identity를 포함한 54-case v2 campaign 전체 재실행 | n=96 18행만 실행. `run_scientific_validity_v2_case`를 대상 tree에서 호출할 수 없어 `execute_arm`을 복제한 harness를 사용 | bitwise 재현은 n=96에 대해서만 성립한다. n=384/1536 행(F-033의 n 의존성 논거에 사용)은 재현되지 않은 committed data이며, 복제 harness는 drift의 두 번째 원천이다. |
| E-04 (P1PR variant) | y0=g(0)+1인 P1과 P2, 4회 이상 halving에서 slope>=4.5로 판정 | P1 max-norm error는 transient가 지배하여(모든 arm에서 5e-2..1e-1) 사용할 수 없었다. P1을 본 뒤 P1PR(y0=g(0))을 추가했고, direct arm 자체가 P1PR에서 order ~4이므로 4.5 slope rule 대신 'direct' 대비로 판정 | P1PR 결과는 pre-registered가 아닌 exploratory이다. H2 SUPPORTED는 pre-specified인 P2(gmres eta=1e-9는 1 halving, production rule은 2 halving 유지)가 떠받쳐야 한다. P1PR floor(0.1-0.3 rtol)와 F-009의 abort threshold는 post-hoc 관측이며 독립적인 문제가 필요하다. |
| E-04 (FD-JVP arm) | P1에서 FD-JVP arm | 실행되지 않음(README/results verdict에 언급 없음) | finite-difference JVP noise가 forcing rule 또는 `64*eps` roundoff guard와 어떻게 상호작용하는지에 대한 증거가 없다. analytic JVP가 없는 사용자는 다뤄지지 않았고 map note 'FD-JVP none'은 미검증으로 남는다. |
| E-05 | cond 1e2..1e8의 nonnormal matrix; 같은 token 아래 preconditioner swap 후의 stale image | main suite는 N-scale s in {0,1,10,100}을 사용. stale suite는 s=10에서 수렴하는 solver가 없어 s=1(cond 1e2)에서만 재실행. stale-image scenario는 inconclusive로 판정(F-086 반박)되었고 `ClosureOperator` token identity는 exercise되지 않음 | FAIL은 residual 오보고가 아니라 GCRO-DR panic과 LGMRES stall에서 나온다(42/42 정직). high-condition stale reuse는 test되지 않았다. |
| E-06 | `dense_phi_action` 대 mpmath | 계획대로이나 dense kernel만 | production Krylov phi-action을 다루지 않는다. F-040/F-042/F-011은 reading/replica로 이를 확장한 것이다. |
| E-07 | order condition + embedded + dense-output condition(Hairer-Wanner IV.7) residual | main과 embedded condition은 확인. dense-output condition은 대수적으로 확인되지 않음(exp/D1의 empirical slope만) | interpolant order 4(nonstiff)는 empirical 관측이며, F-069의 'degree 4'는 code-read이다. |
| E-08 (exponential arm) | lane 전반에 걸친 `WorkCounters` 대 atomic ground truth | exponential arm 미실행: `pexprb54s4`는 non-autonomous 문제를 거부하고 두 test 문제가 모두 non-autonomous이다. transactional, block, comparator lane도 계측되지 않음 | F-039(실패한 exponential trial work 누락), `phi_krylov_vectors`, F-051(block 8->1)은 ground-truth 측정이 없다. 'PASS for rhs/ft'는 RODAS5P generic과 matrix-free lane에만 적용된다. |
| E-09 | analytic solution 대비 error가 tolerance를 넘으면 FAIL | tolerance rule은 20개 ladder 전부에서 PASS. FAIL label은 PLAN이 아니라 experiment brief에서 도입된 order rule(연속 두 slope >= 4.5)에서 나온다 | 보고서는 두 verdict를 모두 보여야 한다. FAIL은 classical ROW order reduction(알려진 이론)이며 구현의 결함이 아니다. |
| E-10 (mutant B) | tableau entry +-1e-8과 controller exponent 하나 | tableau mutant A1(1e-8)과 A2(1e-4)만 실행. mutant B는 미실행(session budget). 1e-8에서의 검출은 fixture bit-identity test가 지배 | E-10 PASS는 D2의 controller/test adequacy에 대해 아무것도 말하지 않는다. behavioural test만으로는 tableau 손상을 ~1e-4에서만 검출하며, 1e-8에서의 검출은 사실상 checksum이다. |
| baseline step 12/15 | ignored 포함 모든 test green | ignored test 하나가 dev와 measurement profile에서 실패 | F-003이 다룬다. 원인은 미해결이다. |

### 8.3 단일 출처에 의존하는 결론

- 전체: top_set과 group file의 모든 항목은 refuter가 정확히 하나이며, 확정 발견 95건 중 55건(대부분 P3, 그리고 D8 P2인 F-057..F-065)은 어느 slim group file에도 없다. 'Confirmed'는 finder + refuter 1명을 뜻하고, decoy catch rate 13/13은 finder recall이 아니라 refuter vigilance를 잰다.
- F-011 (P1): finder와 refuter 모두 one-pass MGS Arnoldi의 독립 numpy/scipy replica를 사용했고(`exp/D4/happy_breakdown.py`, `refute/scratch_05/f011_check.py`), Rust phi-action은 triggering input에서 실행되지 않았다. P1은 `exponential.rs:1078-1150`에 대한 replica 충실도에 의존하며, 두 replica가 일치하므로 위험은 낮으나 Rust 실행 증거는 아니다.
- F-002 (P2): synthetic PR lambda=-1e5에 대한 Python 재구현(`exp/D1/dense_stiff_check.py`)만 있다. impact 서술('every consumer of the dense arm measures interpolant error')은 campaign corpus에서 E-03c(median 1.08, max 2.43)와 모순되므로, 구조적 claim(interpolant의 tolerance control 부재)은 유지하고 campaign 귀속은 뺀다.
- F-009 (P1): E-04 harness뿐이며 refuter는 같은 prebuilt `e04_order_krylov` binary를 재실행했다. abort는 P1과 post-hoc P1PR variant에서만 보였고 campaign corpus 문제에서는 보이지 않았다. guard 대수는 reading으로 검증되었으며 이것이 독립적인 근거이다.
- F-008 (P2): 같은 단일 harness(E-04)와 code 대수이며, tau 상수 0.02022는 한 table에서 읽었다. F-005, F-030, F-031, F-018도 같은 E-04 실행을 인용하므로 H2 cluster 전체(발견 6건)의 실험적 뿌리는 하나이다.
- F-010 (P1): E-05 child scenario이며 refuter는 같은 binary를 재실행했다(exit 134). abort는 명확하여 위험은 낮으나 in-tree integrator에서 도달할 수 없고, P1은 public-API-legal-input 해석에 의존한다.
- F-007 (P1): E-01(committed data 재분석) + E-03(새 실행, n=96만) + 삼각부등식 논거. 가장 잘 뒷받침된 상위 발견이지만 'unattainable'은 'practically unattainable for independently stepped arms'로 써야 한다(same-path 쌍 2/18, 1/18이 통과했고 clipped-vs-clipped는 16/18 통과).
- F-006 (P2): code trace와 data fingerprint(54행 중 26행이 rejection 0, clipped 98, step 100)이며 clipped arm의 h 열을 직접 기록한 실행은 없다. E-02 harness의 per-step h trace가 있으면 닫힌다.
- F-004 (P2): refuter는 E-03 행에서 12/18을 다시 세었으나 'trails scipy Radau ~2.6x'(정정 전 표현) 수치는 E-03의 서술문에서 가져와 재계산하지 않았다. 최종 점검의 재계산 결과 행별 비 중앙값은 5.3배, 최대 438배였고 2.6배는 재현되지 않았다. 2.6x는 단일 출처이며 efficiency ratio가 아니라 equal-tolerance error ratio이고, scipy와 VigilODE는 controller 설정이 다르다(F-099).
- F-033 (P2): committed campaign data뿐이다. refuter는 Direct-stage semilinear arm을 실행하지 못했고 Krylov-inexactness 귀속이 'is not established'라고 적었으며, title의 'accepts every step'은 거짓이다(행당 rejection 3-8회). 17-261x 초과는 실재하나 원인은 미해결이고 title은 수정해야 한다.
- F-003 (P2): 실행으로 두 번 재현되었으나(baseline dev+measurement, refuter) root cause는 밝혀지지 않았다. severity는 미지의 원인에 달려 있다.
- F-040/F-042 (P2): E-06은 `rodas5p_core::dense_phi_action`만 측정했고 production Krylov phi 경로(`exponential.rs:805`, `:1351`)는 읽기만 했다. F-042의 크기는 Python 3x3 model에서 나왔으며, production exponential lane에서의 정확도 손실 크기는 추론이다.
- F-052/F-056/F-053 (P2): code reading뿐이며 comparator sweep이나 timing dispersion 측정은 실행되지 않았다. handicap의 존재는 확인되었으나 크기는 아니며 H4는 정량화할 수 없다.
- F-039/F-050/F-037/F-034/F-047/F-043 (P2): code-trace뿐이고 실행이 없다(E-08은 exponential arm을 실행하지 못했고 forced rejection이 있는 atlas 실행도 없다). control-flow claim은 그럴듯하고 반박되지 않았으나 run artifact가 없다.
- F-023/F-024/F-025 (P2): exp/D11 파일의 count와 rank 분석을 한 reader가 수행하고 한 refuter가 재현했으며, spiral design은 off-tree planning branch에 있다. H3는 건강한 비율의 baseline 없이 descriptive count에 의존한다.
- F-067 (P2): `build.rs`의 static reading(git 명령에 대한 expect)이며 git 없는 빌드나 tarball 빌드는 시도되지 않았다. 실패 양상은 runtime panic이 아니라 compile time의 build-script panic이다.
- F-014 (P2): code trace이며 design 자체가 reference-aware admission rule을 인정한다. preregistered research-only code이므로 finder-P1이 P2로 낮춰졌고, 보고서는 이것이 by design임을 밝혀야 한다.

### 8.4 점검되지 않은 공격면

- D10/D11 — sealed v3.7 replay 불일치(`target_attempt_index` 17 대 18)의 root cause: F-003은 실패와 CI lane 부재를 기록하지만 sealing revision과 b3e8165 사이의 어느 변경이 index를 옮겼는지 bisect한 사람이 없다. F-003의 severity(P2)는 drift가 무해하다고 가정하며, behaviour regression(P1) 또는 invalidated ledger(P0)일 가능성은 미정이다.
- D1 — protected RODAS5P lane에서 stage 및 step 간 W / Jacobian reuse: 발견도 verified_ok 항목도 없다. reuse 하의 W staleness는 Rosenbrock method가 조용히 order를 잃는 고전적 경로이며, 감사는 그것이 없다고 말할 수 없다.
- D2 — WRMS norm에 tau/time-augmented 성분이 포함되는지 여부 대 atlas slice: 발견도 verified_ok도 없다(F-045는 scalar atol만 다룬다). v3.x atlas의 모든 'unsafe'/'admissible' label이 이 미지수를 물려받는다.
- D2 — branch-fixed segment가 dense mode에서 적용되는지: branch-fixed segment semantics를 다룬 항목이 없다. branch switch가 있는 문제(ramped family)에서 dense-arm 유효성에 영향을 주며 비용은 low-to-medium이다.
- D3 — blocker M11: F-036의 related_blocker tag로만 등장하며, M11이 open인지 closed인지 mis-stated인지에 대한 진술이 없어 보고서는 M11의 status를 줄 수 없다.
- D4 — Krylov prefix-resume equivalence(resumed phi/GMRES prefix 대 fresh solve): F-084(latent P3)만 있고 equivalence check는 실행되지도 read-verify되지도 않았다. v3.6/v3.7 'retained level-2 resumption' receipt가 resume == fresh에 의존하며, 17 대 18 replay drift의 후보 원인이기도 하다.
- D4 — `pexprb54s4` stiff (Z=0 weak-form) order-5 condition: verified_ok는 nonstiff order만 다루고 F-041은 in-tree evidence가 scalar 문제라고 한다. stiffness 하의 exponential-lane order는 UNTESTED이며 검증도 반박도 되지 않았다.
- D5 — calibration/tuning code에서 `holdout_specs()`의 reachability: 발견과 verified_ok에 언급이 없고 fair-ab `holdout_specs` call graph는 추적되지 않았다. verified_ok의 'README:23 holdout not used to retune' 항목은 call-graph trace가 아니라 prose(v34 NEXT_NODE)에 의존한다.
- D6 — threads>1인 Rayon pool 안에서의 timing, per-step coefficient reload의 측정 비용: verified_ok는 CLI default threads=1만 보인다. F-049의 '10-40 us per step'은 code reading에 의한 추정이고 profile은 취해지지 않았으므로 F-049/F-019의 fairness 영향은 정량화되지 않았다.
- D7 — `numerical_reference.rs`의 tolerance ladder와 Richardson + method-disagreement uncertainty model: 두 공격 모두 반박되었고(F-054, F-055) verified_ok는 load-time arithmetic validation만 다룬다. reference uncertainty의 적정성은 가정된 상태이며, E-03c는 별도의 tight Radau reference(uncertainty <= 4e-3)를 사용하여 H1b에 한해서만 이를 완화한다.
- D9 — receipt의 f64 bit-exactness(`to_bits`)와 `deny_unknown_fields` coverage: `to_bits`는 항목이 없다. `deny_unknown_fields`는 coverage 42/108(fair-ab), 3/184(integrators)로 verified_ok에 있으나 이는 coverage 통계이지 pass가 아니므로 발견(P3)으로 옮기거나 다시 label해야 한다.
- D10 — controller-formula mutation adequacy와 tolerance-proportionality test: E-10 mutant B는 미실행이고 F-020은 tolerance proportionality를 나열하지 않는다. 잘못된 controller exponent나 F-006 clipped-step freeze의 변경을 잡을 test가 있다는 증거가 없으며, `adaptive_controller_contracts.rs:80-83`은 실제로 그 freeze를 pin한다.
- D11 — forking paths(48 events / 5 unsafe, v3.0->v3.2 sequential search)와 v3.4 post-hoc budget change: forking-path 분석은 언급이 없다. `tau=13.397`과 `B_abs=80`/`delta=0.25`(F-046은 unsourced라고 한다)의 selection bias는 평가되지 않았으며, 보고서는 v3.x threshold가 multiplicity에 대해 감사되었다고 암시해서는 안 된다.
- D8 — overlay(untracked stage-certificate module, Lean/Rocq 소스)의 동적 동작: static reading과 prebuilt feature-gated test binary뿐이며 증명은 Lean/Rocq로 재검증되지 않았다. F-057..F-065는 slim group file에 없어 refuter artifact가 critic에게 보이지 않았고, 모든 D8 발견은 dirty local worktree에만 있는 바이트에 대한 read-only claim이다(F-025).

### 8.5 가치가 큰 후속 점검 (각 1시간 이내)

1. F-003 bisect: ignored v3.7 replay test를 v3.7 ledger를 seal한 commit과 b3e8165에서 실행하고(`git log --` 대상: `sequential.rs`, `g4_s5b0_regime_atlas.rs`, `gmres.rs`), semilinear N=192 실행의 accepted-attempt 열을 diff한다. F-003이 doc/test drift(P2)인지 behaviour regression(P1)인지 invalidated ledger(P0)인지 결정하고 prefix-resume gap도 test한다. (60분)
2. tree 밖의 Rust micro-harness(bin 하나): (a) A=diag(-1,-3), `v=e1+4.5e-7*e2`, rtol 1e-10으로 Krylov phi-action을 호출하여 converged/error_estimate/true error를 출력하고, (b) Prothero-Robinson lambda=-1e5에서 `integrate_rodas_adaptive_dense`를 실행하여 interior 대 endpoint WRMS를 출력한다. F-011(P1)과 F-002를 replica 기반에서 run 기반 증거로 바꾼다. (45분)
3. E-10 mutant B와 하나 더: controller exponent를 1/5 -> 1/4로 바꾸고, 별도로 clipped-accept branch가 growth를 적용하게 하여(F-006을 되돌림) isolated worktree에서 default suite를 실행한다. 미실행 E-10 arm을 닫고 D2 test adequacy를 측정하며 `adaptive_controller_contracts`가 freeze를 pin하는지 확인한다. (45분)
4. E-04 harness 확장: (a) campaign의 accepted step 열에서 semilinear advection-diffusion ramped family에 대한 Direct-stage arm과 gmres eta=1e-12 arm, (b) P1PR에서의 FD-JVP arm, (c) production forcing rule에 대한 독립 stiff 문제 하나(예: Robertson 또는 van der Pol). 17-261x semilinear 초과(F-033, H1b 잔여)를 귀속시키고 FD-JVP arm을 닫으며 H2/F-008/F-009에 post-hoc P1PR 외의 두 번째 pre-specified 문제를 준다. (60분)
5. autonomous 문제(autonomous Robertson 또는 linear y'=Ay)에서 atomic counter와 forced failed trial을 갖춘 E-08 exponential arm. 같은 session에서 `git archive b3e8165 | tar -x`를 temp dir에 풀고 git이 PATH에 있을 때와 없을 때 `cargo build -p rodas5p-fair-ab`를 시도한다. F-039와 `phi_krylov_vectors`를 ground-truth화하고(H4 accounting) H5/F-067을 static에서 run 증거로 바꾼다. (50분)

### 8.6 최종 점검에서 정정한 내용

보고서 1장과 4장의 수치 152개를 원자료와 대조했고, 발견 블록 18개의 위치를 tree에서 다시 열어 확인했다. 아래 항목을 정정했다.

| 위치 | 정정 전 | 정정 후 | 분류 |
|---|---|---|---|
| 1.1 결론, '여기에 측정된 사실 하나를 더한다' 문단 | SciPy Radau보다 17/18 행에서 오차가 크다(중앙값 2.6배) | 같은 tolerance에서 SciPy Radau보다 17/18 행에서 오차가 크다(행별 비 중앙값 5.3배, 최대 438배). | wrong |
| 4.2 실험 결과 표, E-03 행 | VigilODE dense 오차가 SciPy Radau보다 17/18 행에서 큼(중앙값 2.6배, 최대 70배) | VigilODE dense 오차가 SciPy Radau보다 17/18 행에서 큼(행별 비 중앙값 5.3배, 최대 438배 = semilinear rtol 1e-8; semilinear rtol 1e-4는 70배) | wrong |
| 4.2 실험 결과 표, E-07 행 | 5차 조건 17/17 최대 잔차 1.3e-15 | 5차까지의 차수 조건 17/17 만족, 최대 잔차 5.5e-15(5차 트리 9개만 보면 1.2e-15, 기준 1e-13) | wrong |
| 4.2 실험 결과 표, E-06 행 | ‖v‖≤1은 최대 2.44e-14 | ‖v‖≤1은 최대 3.5e-14, `matrix_exp_pade13`은 최대 3.5e-14 | wrong |
| 4.3 Baseline 표, '기본 + feature 테스트 suite' 행 | 1075 passed, 0 failed | 기본 + feature 테스트 suite / 1074 passed, 0 failed (`--ignored` 실행의 1 passed, 1 failed를 합치면 1075 passed, 1 failed) | wrong |
| 4.2 실험 결과 표, E-08 행 | `rhs_calls`, `ft_calls`는 24개 arm 모두 실제 호출 수와 일치 | `rhs_calls`, `ft_calls`는 성공한 실행 전부에서 실제 호출 수와 일치(24개 arm 중 22개 일치, hard error로 끝난 fixed_direct arm 2개는 rhs 1회, ft 1회를 보고하지 않음) | imprecise |
| 4.2 실험 결과 표, E-09 행 | 오차 차이 1e-15 이하 | 오차 차이 1.4e-15 이하 | imprecise |
| 4.2 실험 결과 표, E-04 행 / 1.1 결론 항목 2 / 4.1 H2 | production rule은 `0.1–0.3·rtol` 바닥 (advection-diffusion(n=512) 문장에 이어서 서술); 1.1: 오차가 `0.1–0.3·rtol`에서 바닥을 친다 | production rule은 Prothero-Robinson에서 약 0.1–0.3·rtol(rtol=1e-8, 1e-10에서는 최대 1.3·rtol, 0.74·rtol), advection-diffusion에서 0.01–0.8·rtol의 h와 무관한 바닥 | imprecise |
| 1.1 결론 항목 2 | `rtol ≤ 1e-8`의 stiff 입력에서는 GMRES를 한 번도 돌리지 않고 step 0에서 오류를 반환한다 | `rtol ≤ 1e-8`의 stiff 입력(/λ/≈1e6)에서는 큰 고정 h에서 GMRES를 한 번도 돌리지 않고 step 0에서 오류를 반환한다(P1 rtol=1e-10은 모든 h, rtol=1e-8은 h ≥ 1/256) | imprecise |
| 4.2 실험 결과 표, E-04 행 | direct LU와 GMRES η=1e-12만 4회 halving 동안 기울기 4.5 이상 유지(5.33, 5.27, 5.15, 5.05) | direct LU(5.33, 5.27, 5.15, 5.05)와 GMRES η=1e-12(5.33, 5.27, 5.13, 4.94)만 4회 halving 동안 기울기 4.5 이상 유지 | imprecise |
| 1.1 결론 항목 3 | release 프로파일에서는 프로세스가 종료된다(`E-05`, exit 134) | panic=abort인 release/measurement 프로파일에서는 프로세스가 종료된다(`E-05`에서 SIGABRT, 반박자 재실행에서 exit 134) | imprecise |
| 4.1 사전 등록 가설 판정 표, H1b 행 | H1b / PARTIAL | H1b / PARTIAL (E-03의 사전 등록 규칙만으로는 REFUTED: interpolated/accepted 비가 5배를 넘는 행 0/18. 구조적 사실을 포함한 종합 판정이 PARTIAL) | imprecise |
| 4.1 사전 등록 가설 판정 표, H1a 행 | 18쌍 중 2쌍은 통과했다 | 비교 종류에 따라 18쌍 중 1–2쌍은 통과했다(h0 쌍 2/18, max_step 쌍 1/18, SciPy 쌍 1/18과 2/17) | imprecise |
| 4.1 사전 등록 가설 판정 표, H3 행 | v3.6 이후 schema family 83개와 산문 4744줄이 늘었으나 admitted 결과와 candidate 실행은 0이다 | tree 전체에 schema family 문자열이 83개 있고, v3.6 이후 추가된 노드 8개(audit2 7개 + scientific_validity_v2)의 산문이 4744줄이지만 이 노드들의 admitted 결과와 candidate 실행은 0이다(v3.7 replay receipt는 집계에서 제외… | imprecise |
| `F-057` 위치 | `crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs:378-382` | `overlay/untracked/crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs:378-382` | wrong |
| `F-060` 위치 | `crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs:258-268` | `overlay/untracked/crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs:258-268` | wrong |

가장 큰 정정은 SciPy Radau 대비 오차 비다. 실험 에이전트의 README 서술에 있던 "중앙값 2.6배"는 행 데이터로 재현되지 않았다. 18행을 다시 계산한 값은 중앙값 5.3배, 최대 438배, 최소 0.90배다. 이 수치는 `F-004`와 개선안 문장에도 퍼져 있었고 모두 정정했다.

미공개 overlay 파일을 가리키는 위치 23개는 경로 앞의 `overlay/untracked/`가 빠져 있었다. 감사 도구의 중복 제거 단계가 경로를 정규화하면서 생긴 오류이며 모두 복원했다.

위치 점검은 표본 검사다. 표본에 들지 않은 블록의 줄 번호는 다시 확인하지 않았다.

---

## 부록 A. 재현 방법

모든 경로는 `RUN=~/.local/state/vigilode/adversarial-audit/20260927T121000Z` 기준이다.

| 대상 | 위치 |
|---|---|
| 감사 tree (읽기 전용 clone, `b3e8165`) | `$RUN/tree` |
| cargo wrapper (flock, offline, locked) | `$RUN/bin/cargo-wrapped.sh` |
| 실험 harness crate | `$RUN/harness` (bin: `e0203_v2rows`, `e04_order_krylov`, `e05_krylov_stress`, `e06_phi_action`, `e07_coeff_print` 외) |
| 실험 결과 | `$RUN/exp/E-##/results.json`, `README.md` |
| Baseline 로그 | `$RUN/baseline/` |
| 공격면 지도 | `$RUN/map/*.json` |
| finder 원본 | `$RUN/findings/D*.json` |
| 반박 verdict | `$RUN/refute/verdicts_*.json`, `lens_*.json`, `narrowings.json` |
| 수정 설계 | `$RUN/fixes/*.json` |
| 미공개 overlay 복사본 | `$RUN/overlay/` |

빌드 캐시 `$RUN/cargo-target`(약 24 GB)과 `$RUN/cargo-target-mut`(약 9 GB)은 삭제해도 증거가 손실되지 않는다.

## 부록 B. 무결성

| 항목 | 결과 |
|---|---|
| 사용자 저장소 refs, status, HEAD (실행 전후) | 동일 |
| dirty worktree status, diff, HEAD (실행 전후) | 동일 |
| 감사 tree의 `git status` | clean |

스냅샷은 `$RUN/integrity/`에 있다.

## 부록 C. 출처가 없는 상수

| 상수 | 값 | 위치 | 출처 분류 | 용도 |
|---|---|---|---|---|
| `V36_FROZEN_ZETA34_TAU` | `13.39706618860016` | `crates/rodas5p-integrators/src/g4_s5b0_regime_atlas.rs:379` | fitted | quadratic-drift zeta34 <= tau recommends full-E continuation |
| `allocation_audit defaults` | `reps 3, warmups 1, restart 20,…` | `crates/rodas5p-cli/src/bin/allocation_audit.rs:112` | unsourced | clap defaults |
| `CLI trace/benchmark defaults` | `dimension 48, steps 4, stages …` | `crates/rodas5p-cli/src/main.rs:375` | unsourced | clap defaults |
| `TIER_L/TIER_N_REQUIRED_WALL_SPEEDUP` | `1.15` | `crates/rodas5p-cli/src/main.rs:783` | unsourced | promotion threshold |
| `unified linear configs` | `smoke dim 8 steps 1 stiffness …` | `crates/rodas5p-cli/src/main.rs:1458` | unsourced | linear Tier-L traces |
| `unified FairSolveConfig` | `rtol 1e-9 atol 1e-12 restart 2…` | `crates/rodas5p-cli/src/main.rs:1506` | unsourced | linear solver settings |
| `validate command literals` | `rust_toolchain_lock 1.94.1, li…` | `crates/rodas5p-cli/src/main.rs:1612` | unsourced | validation JSON |
| `Jacobi zero-diagonal tolerance` | `f64::EPSILON * max(max_i|d_i|,…` | `crates/rodas5p-core/src/operator.rs:339` | unsourced | reject Jacobi preconditioner with (near-)zero diagonal |
| `LinearSolverConfig::default` | `method Direct, rtol 1e-11, ato…` | `crates/rodas5p-core/src/solver_types.rs:36` | unsourced | public solver config default |
| `GCRO-DR rank_tol` | `1e-12` | `crates/rodas5p-fair-ab/src/adapters.rs:130` | unsourced | recycle subspace rank tolerance |
| `g1 corpus extra problems` | `complex_dahlquist(16,120,180,0…` | `crates/rodas5p-fair-ab/src/adaptive_global_error.rs:341` | unsourced | G1 screen corpus |
| `tolerance_ladder` | `smoke {1e-4,1e-6}; canonical {…` | `crates/rodas5p-fair-ab/src/adaptive_global_error.rs:376` | unsourced | adaptive screen rtol ladder |
| `adaptive atol` | `0.01*rtol` | `crates/rodas5p-fair-ab/src/adaptive_global_error.rs:386` | unsourced | absolute tolerance rule |
| `adaptive min_step/max_attempts` | `1e-12 / 200000` | `crates/rodas5p-fair-ab/src/adaptive_global_error.rs:389` | unsourced | controller limits |
| `homotopy config` | `HomotopyPathConfig::new(1.0,7,…` | `crates/rodas5p-fair-ab/src/adaptive_global_error.rs:517` | unsourced | homotopy candidate parameters |
| `TIMING_SEED` | `20260808` | `crates/rodas5p-fair-ab/src/global_error.rs:21` | unsourced | PRNG seed for shuffled timing order |
| `SMOKE_WARMUPS/SMOKE_REPETITIONS` | `1/3` | `crates/rodas5p-fair-ab/src/global_error.rs:22` | unsourced | smoke timing protocol |
| `CANONICAL_WARMUPS/CANONICAL_REPETITIONS` | `1/5` | `crates/rodas5p-fair-ab/src/global_error.rs:24` | unsourced | canonical timing protocol |
| `MIN_TIMING_SAMPLE_SECONDS` | `2.0e-3` | `crates/rodas5p-fair-ab/src/global_error.rs:26` | unsourced | minimum batch duration for a timing sample |
| `MAX_TIMING_BATCH_ITERATIONS` | `10000` | `crates/rodas5p-fair-ab/src/global_error.rs:27` | unsourced | cap on batch iterations |
| `uniform grid divisibility tolerance` | `128*eps*max(|start|,|end|,1)` | `crates/rodas5p-fair-ab/src/global_error.rs:64` | unsourced | CommonOutputGrid::uniform |
| `matching_index tolerance` | `64*eps*max(|a|,|b|,1)` | `crates/rodas5p-fair-ab/src/global_error.rs:283` | unsourced | output-time matching |
| `output-policy dominance ratio` | `0.1` | `crates/rodas5p-fair-ab/src/global_error.rs:715` | unsourced | gap > 0.1*dense error => Dominated |
| `analytic ExternalErrorScale` | `absolute 1e-10, relative 1e-8` | `crates/rodas5p-fair-ab/src/global_error.rs:1130` | unsourced | WRMS weights for analytic references (also adaptive_global_error.rs:293, numeric… |
| `fixed-anchor corpus` | `scalar_linear(-2,1) h{.04,.02,…` | `crates/rodas5p-fair-ab/src/global_error.rs:1155` | unsourced | problems and step ladders |
| `fixed-anchor RODAS5P linear tolerances` | `atol 1e-12, rtol 1e-10 (positi…` | `crates/rodas5p-fair-ab/src/global_error.rs:1208` | unsourced | inner tolerances for sequential fixed-step candidate |
| `default_targets` | `MaxGridL2 in {1e-2,1e-4,1e-6,1…` | `crates/rodas5p-fair-ab/src/global_error.rs:1603` | unsourced | target attainment thresholds |
| `reference dominance ratio` | `0.1` | `crates/rodas5p-fair-ab/src/numerical_reference.rs:392` | unsourced | uncertainty > 0.1*measured error => Dominated |
| `radau ladder pins` | `L0 1e-8/1e-10; L1 1e-10/1e-12;…` | `crates/rodas5p-fair-ab/src/numerical_reference.rs:615` | unsourced | reference generator tolerances |
| `q upper bound` | `0.5` | `crates/rodas5p-fair-ab/src/numerical_reference.rs:965` | unsourced | Richardson admissibility |
| `same_f64 tolerance` | `128*eps*scale` | `crates/rodas5p-fair-ab/src/numerical_reference.rs:1642` | unsourced | convergence field equality |
| `requested grid` | `101 uniform points + mandatory…` | `crates/rodas5p-fair-ab/src/numerical_reference.rs:1664` | unsourced | canonical output grid |
| `holdout table` | `oregonator n3 [0,360]; polluti…` | `crates/rodas5p-fair-ab/src/numerical_reference.rs:1700` | unsourced | expected holdout identities and source sha256 |
| `scenario base matrix` | `diag 1+stiffness*(0.01+x^2); s…` | `crates/rodas5p-fair-ab/src/scenarios.rs:65` | unsourced | synthetic operator |
| `scenario drift/abrupt/rotating` | `1+0.01*step; givens 0.55 & dia…` | `crates/rodas5p-fair-ab/src/scenarios.rs:101` | unsourced | sequence kinds |
| `oracle_vector` | `sin(pi x)+0.35 sin(phase) cos(…` | `crates/rodas5p-fair-ab/src/scenarios.rs:133` | unsourced | exact solutions |
| `SCIENTIFIC_VALIDITY_V2_MAX_ATTEMPTS_PER_ARM` | `200000` | `crates/rodas5p-fair-ab/src/scientific_validity_v2_campaign.rs:35` | unsourced | attempt budget per arm |
| `SOLVER_PROTOCOL_ID` | `rodas5p;gmres-restart32-max256…` | `crates/rodas5p-fair-ab/src/scientific_validity_v2_campaign.rs:39` | unsourced | frozen v2 solver protocol string hashed into campaign binding |
| `campaign_config numeric fields` | `restart 32, max_arnoldi 256, i…` | `crates/rodas5p-fair-ab/src/scientific_validity_v2_campaign.rs:190` | unsourced | v2 arms configuration |
| `A1 profile / invalidated run id` | `EnforcedBudgetHoldout320 / 329…` | `crates/rodas5p-integrators/src/a1_two_arm_receipt.rs:18` | unsourced | receipt cell |
| `RODAS5P estimator order` | `5` | `crates/rodas5p-integrators/src/adaptive.rs:31` | unsourced | exponent denominator for RODAS5P (embedded order 4 → local error O(h^5)) |
| `AdaptiveStepConfig::default.atol` | `1.0e-9` | `crates/rodas5p-integrators/src/adaptive.rs:78` | unsourced | default absolute tolerance |
| `AdaptiveStepConfig::default.rtol` | `1.0e-6` | `crates/rodas5p-integrators/src/adaptive.rs:79` | unsourced | default relative tolerance |
| `AdaptiveStepConfig::default.initial_step` | `1.0e-3` | `crates/rodas5p-integrators/src/adaptive.rs:80` | unsourced | default h0 |
| `AdaptiveStepConfig::default.min_step` | `1.0e-14` | `crates/rodas5p-integrators/src/adaptive.rs:81` | unsourced | loop break threshold |
| `AdaptiveStepConfig::default.max_attempts` | `100000` | `crates/rodas5p-integrators/src/adaptive.rs:83` | unsourced | attempt budget (silent success=false) |
| `AdaptiveStepConfig::default.min_factor` | `0.2` | `crates/rodas5p-integrators/src/adaptive.rs:85` | unsourced | minimum step ratio; also used directly after solver-error rejection |
| `AdaptiveStepConfig::default.max_factor` | `5.0` | `crates/rodas5p-integrators/src/adaptive.rs:86` | unsourced | maximum growth (also returned when error==0) |
| `AdaptiveStepConfig::default.reject_max_factor` | `0.9` | `crates/rodas5p-integrators/src/adaptive.rs:87` | unsourced | cap on factor after rejection |
| `legacy_rodas.min_step` | `f64::MIN_POSITIVE` | `crates/rodas5p-integrators/src/adaptive.rs:175` | unsourced | legacy lanes effectively have no minimum step |
| `PI exponents` | `-0.7/order, +0.4/order` | `crates/rodas5p-integrators/src/adaptive.rs:227` | unsourced | PI controller |
| `Integral exponent` | `-1.0/order` | `crates/rodas5p-integrators/src/adaptive.rs:229` | unsourced | I controller |
| `error floor` | `1.0e-16` | `crates/rodas5p-integrators/src/adaptive.rs:249` | unsourced | floor for stored/used error (also 289; integrate.rs:254; adaptive_exponential.rs… |
| `step-doubling denominator` | `2^p - 1` | `crates/rodas5p-integrators/src/adaptive.rs:356` | unsourced | Richardson error estimate |
| `phi_error_proxy denominator floor` | `f64::MIN_POSITIVE` | `crates/rodas5p-integrators/src/adaptive_exponential.rs:93` | unsourced | avoid division by zero in //scale//_2 |
| `adaptive_exponential loop tolerance` | `10 eps max(|tf|,1)` | `crates/rodas5p-integrators/src/adaptive_exponential.rs:190` | unsourced | termination/success tolerance |
| `pexprb adaptive controller order` | `5 (literal)` | `crates/rodas5p-integrators/src/adaptive_exponential.rs:315` | unsourced | propose_factor order for pexprb54s4 (also line 323) |
| `Bateman constants` | `FAST_RATE 1000.0, SLOW_RATE 1.…` | `crates/rodas5p-integrators/src/audit2_bateman_real_client_research.rs:67` | unsourced | authority manifest |
| `Bateman decoded bits` | `FAST_RATE 1000.0 (0x408f4000_0…` | `crates/rodas5p-integrators/src/audit2_bateman_real_client_research.rs:67` | unsourced | frozen client + reference |
| `Bateman budgets` | `outer_atol 1e-4, outer_rtol 1e…` | `crates/rodas5p-integrators/src/audit2_bateman_real_client_research.rs:498` | unsourced | transactional admission |
| `Bateman proof receipt literals` | `declared_reference_l2_uncertai…` | `crates/rodas5p-integrators/src/audit2_bateman_real_client_research.rs:664` | unsourced | admission string/number equalities |
| `Audit2MatrixFreeCommonWConfig::default` | `restart 24, max_arnoldi 192, r…` | `crates/rodas5p-integrators/src/audit2_matrix_free_research.rs:30` | unsourced | GMRES |
| `AUDIT2_STRUCTURE_PROJECTION_TOLERANCE` | `64*eps` | `crates/rodas5p-integrators/src/audit2_research.rs:22` | unsourced | coefficient leakage allowance |
| `BDF2_ZERO_STABILITY_RATIO_MAX` | `2.414213562373095` | `crates/rodas5p-integrators/src/bdf.rs:174` | unsourced | variable BDF2 ratio guard |
| `SABR block GMRES default (BlockMethod::Gmres via s…` | `rtol 1e-11 atol 1e-13 restart …` | `crates/rodas5p-integrators/src/block.rs:596` | unsourced | block solve |
| `RefinedRootConfig::default` | `max_iterations 10, residual_rt…` | `crates/rodas5p-integrators/src/certification.rs:33` | unsourced | C3 Newton |
| `RHS_COUNT` | `8` | `crates/rodas5p-integrators/src/common_w_gate.rs:22` | unsourced | RHS batch size |
| `common-W operator` | `omega=0.35*stiffness; mass dia…` | `crates/rodas5p-integrators/src/common_w_gate.rs:111` | unsourced | synthetic operator |
| `common-W solver configs` | `GMRES restart 32 max_arnoldi 1…` | `crates/rodas5p-integrators/src/common_w_gate.rs:337` | unsourced | solver tolerances |
| `TARGET_SAMPLE_SECONDS / MAX_BATCH_ITERATIONS / rep…` | `5e-3 / 4096 / 3|5 / 1.15` | `crates/rodas5p-integrators/src/common_w_gate.rs:490` | unsourced | timing |
| `RODAS5P stage count` | `8 (from fixture c.len(); asser…` | `crates/rodas5p-integrators/src/dense_output_v2.rs:75` | unsourced | dense output shape check |
| `radau_iia3_weights c1,c2` | `(4∓√6)/10` | `crates/rodas5p-integrators/src/dense_output_v2.rs:124` | unsourced | Radau IIA3 collocation nodes |
| `Radau estimator orders / ids` | `IIA1: 2 'radau-iia1-step-doubl…` | `crates/rodas5p-integrators/src/dense_output_v2.rs:1170` | unsourced | controller order for comparators |
| `step-doubling guard` | `0.5*h <= f64::MIN_POSITIVE` | `crates/rodas5p-integrators/src/dense_output_v2.rs:1177` | unsourced | Radau/BDF adaptive dense loops break (also 1308) |
| `ExponentialKrylovConfig::default` | `min 4, max 24, incr 2, rtol 1e…` | `crates/rodas5p-integrators/src/exponential.rs:27` | unsourced | legacy Krylov phi defaults |
| `FusedPhiKrylovConfig::default` | `min 4, max 24, incr 2, rtol 1e…` | `crates/rodas5p-integrators/src/exponential.rs:88` | unsourced | fused phi defaults |
| `quadratic drift scaling` | `D_j/c_j^2 − D_i/c_i^2` | `crates/rodas5p-integrators/src/exponential.rs:396` | unsourced | zeta23/zeta34 telemetry |
| `breakdown_tolerance` | `64*sqrt(f64::EPSILON) (~9.5e-7…` | `crates/rodas5p-integrators/src/exponential.rs:881` | unsourced | happy-breakdown detection (relative to max(1,max/h_{i,col}/)); also 1078, 1563 |
| `substep doubling` | `substeps*2, capped at maximum_…` | `crates/rodas5p-integrators/src/exponential.rs:1325` | unsourced | phi-action restart schedule |
| `predict_krylov_dimension contraction clamp` | `[1e-3, 0.999]; uses last 2 rat…` | `crates/rodas5p-integrators/src/exponential.rs:1404` | unsourced | advisory cost prediction |

전체 227개 (literature/derived 포함)는 ledger `constants_table` 참조. 위 표는 unsourced/fitted 217개 중 최대 80개.
