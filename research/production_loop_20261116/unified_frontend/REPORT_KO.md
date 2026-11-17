# V03 통합 frontend (part A) 보고

이 문서는 `build_report.py`가 `RECEIPT.json`에서 만든 것이다. 숫자는 모두 `logs/*.log`(실행된 명령의 원 출력)에서 `build_receipt.py`가 파싱했다. 손으로 적은 숫자는 없다.

- base: `c468478e73123a4625c6d20f7459a9f79cbf33de`
- 측정에 쓰인 코드 commit: `b52f79cd8d72c697945dde37b36c2da3c2ec2669`
- 사전등록: `PREDECLARED.json` (첫 실행 전에 commit `00d32764`)
- wall time 주장은 없다. 로그의 경과 시간은 `diagnostic_only`다.

## 1. 결론

- 도달 단계: **Phase B**.
  `GeneralOde::select`의 본문이 method catalog에서 유도한 선택(`MethodRegistry::general_selection`)으로 바뀌었고, 이전 본문은 `GeneralOde::select_reference`로 남아 테스트 oracle이 된다. 기본 라우팅의 결정은 한 건도 달라지지 않았다.
- 동결 판별 기준(PREREGISTRATION.md V03 행)의 결과:

| 기준 | 측정 | 판정 |
|---|---|---|
| D1 `route(General)` = `GeneralOde::select`, 전수 격자, callback 0 | 221184 / 221184 일치, golden 파일과 221184 / 221184 일치, callback 0회 | PASS |
| D2 adapter가 직접 호출과 bit 동일 | adapter 8 / 8, 사례 25 / 25 | PASS |
| D3 거부는 어떤 callback보다 먼저 | `route` 405504회 (계획 66672, 거부 338832) callback 0회; 거부된 `solve` 68회 callback 0회 | PASS |
| D4 기본 경로 golden 불변 | 두 번 연속 실행 모두 exit 0/0, golden 파일 9개 byte 동일, base 대비 diff 0줄 | PASS |

- Phase B 조건: catalog 유도 선택과 이전 본문이 확장 격자 331776 / 331776건, golden 격자 221184건에서 일치하고 callback은 0회다. 판정 PASS.
- PeerOde 결함(COD-02, COD-04) 재현 테스트 8건: base 소스에서 8건 실패, 수정 후 8건 통과.
- facade 전체 suite: 497 passed, 0 failed, 0 ignored (테스트 바이너리 57개, exit 0).
- clippy exit 0, fmt exit 0, rustdoc exit 0, loop checker exit 0.
- 기존 테스트(`unified_method_capabilities.rs`, `unified_method_sessions.rs`) 수정: 없음. 두 파일은 그대로 통과한다.
- 허용 경로 밖의 변경: 없음. 보호 경로의 base 대비 diff: 없음.

## 2. API

진입점은 하나다. `route`가 선언된 사실만 읽어 계획 또는 타입이 있는 거부를 돌려주고(사용자 callback 0회), `solve`가 계획을 실행하고, `view()`가 방법과 무관한 결과를 빌려준다. 아래 예는 `MethodChoice` 문서의 doctest이며 facade suite에서 컴파일되고 실행된다.

```rust
use vigilode::{
    CallbackError, DerivativeFallback, GeneralOdePolicy, Jacobian, MethodChoice,
    MethodRegistry, Options, Problem, ProblemRegime,
};

let problem = Problem::new(2, |_t, y, dy| {
    dy[0] = -1000.0 * y[0] + y[1];
    dy[1] = -y[1];
    Ok::<(), CallbackError>(())
})
.jacobian(Jacobian::dense(|_t, _y, jac| {
    jac.copy_from_slice(&[-1000.0, 1.0, 0.0, -1.0]);
    Ok(())
}))
.autonomous();
let options = Options::default();

// today's default decision, as a plan (no callback is made)
let policy = GeneralOdePolicy {
    regime: ProblemRegime::Stiff,
    derivatives: DerivativeFallback::ProvidedOnly,
    ..GeneralOdePolicy::default()
};
let plan = MethodRegistry::route(&problem, &options, &MethodChoice::General(policy))?;
assert_eq!(plan.route.id(), "rodas5p/dense-fast");

// execute it and read the method-independent view
let solution = MethodRegistry::solve(&problem, (0.0, 1.0), &[1.0, 1.0], &options, &plan)?;
let view = solution.view();
assert!(view.is_success());
assert_eq!(view.times(), &[0.0, 1.0]);
assert!(view.work().rhs_calls > 0);

// any registered method by id; a refusal is typed and precedes every callback
let radau = MethodRegistry::route(&problem, &options, &MethodChoice::Explicit("radau-iia3"))?;
assert_eq!(radau.route.id(), "radau-iia3/explicit-jacobian-dense-newton");
assert!(MethodRegistry::route(&problem, &options, &MethodChoice::Explicit("no-such-method")).is_err());
```

| 항목 | 내용 |
|---|---|
| `MethodChoice::General(policy)` | 오늘의 기본 결정. `GeneralOde::select`와 같다 |
| `MethodChoice::Explicit(id)` | catalog의 방법 하나를 id로 지정 |
| `MethodChoice::Auto(policy)` | opt-in 순위 결정. 기본값이 아니다 |
| `MethodRegistry::route` / `route_with` | 순수 함수. `MethodRoutePlan` 또는 `MethodRouteRefusal` |
| `MethodRegistry::solve` | 계획 실행. 입력 오류는 callback 전에 `MethodSolveError` |
| `SolutionView` | `times()`, `states()`, `output_grid()`, `work()`, `route()`/`route_id()`(실행 기록에서 읽은 경로), `failure()`(타입이 있는 사유), `at(t)` |
| `MethodRegistry::catalog` / `entry` | 방법 표(아래) |
| `MethodParams` | PPSW1 옵션, Lawson의 선언된 선형 flow |

기존 frontend(`Rodas5p`, `Dopri5`, `GeneralOde`, `AutomaticOde`, `SwitchingSolver`, `ProductionSolver`, `PeerOde`, `LawsonDopri5`, `ExplicitSession`, `Dopri5Ensemble`)는 그대로 동작한다. `lib.rs`는 수정하지 않았다.

## 3. 방법 표

`MethodRegistry::catalog()`의 내용이다(테스트 `catalog_extends_the_registry_and_states_what_is_unavailable`가 출력). 앞의 6개는 U06 `descriptors()`와 같고, 뒤의 3개가 이번에 facade에서 도달 가능해진 방법이다.

| id | 실행 | Jacobian 형태 | 지원 기능 | band 선언 | vector atol | 단계/차수 | qualification | Auto 순위 대상 |
|---|---|---|---|---|---|---|---|---|
| `dopri5` | 가능 | 불필요 (RHS만) | 없음 | 예 | 예 | 7/5 | production | 예 |
| `rodas5p` | 가능 | Dense, Banded, Jvp, SparseJvp(band), SparseJvp(일반), FD | mass, event, dense output | 예 | 예 | 8/5 | production | 예 |
| `ppsw1-6b-schmitt-weiner-singly-implicit-peer-w` | 가능 | Dense, Jvp | 없음 | 아니오 | 아니오 | 6/5 | scoped calibration (production 아님) | 예 |
| `ppsw1-6b-matrix-free-common-w` | 불가 | - | 없음 | 아니오 | 아니오 | 6/5 | 구현됨, 미자격 | - |
| `bdf1-bdf2-variable` | 가능 | Dense, Jvp | 없음 | 아니오 | 아니오 | 1/2 | 구현됨, 미자격 | 예 |
| `radau-iia3` | 가능 | Dense, Jvp | 없음 | 아니오 | 아니오 | 3/5 | 참조 구현 전용 | 예 |
| `pexprb54s4-fused` | 가능 | Jvp | 없음 | 아니오 | 아니오 | 4/5 | 구현됨, 미자격 | 예 |
| `rodas5p-lagged-richardson-v1` | 가능 | Dense | 없음 | 아니오 | 아니오 | 8/5 | 구현됨, 미자격 | 예 |
| `lawson-dopri5-declared-damped-rotation` | 가능 | 불필요 (RHS만) | 없음 | 예 | 예 | 7/5 | 구현됨, 미자격 | 아니오 |

RODAS5P 항목의 정확한 경로 variant(`MethodEntry::route_variants`). U06 descriptor에는 권한 variant 3개만 있고 banded와 sequential이 없었다(COD-03). 계획의 variant 이름과 general 선택의 matrix-free 권한, dense 차원 상한 판정이 이 표를 읽는다.

| variant | n x n 행렬 저장 | matrix-free | mass | event | dense output |
|---|---|---|---|---|---|
| `dense-fast` | 예 | 아니오 | 아니오 | 아니오 | 아니오 |
| `banded-fast` | 아니오 | 아니오 | 아니오 | 아니오 | 아니오 |
| `matrix-free` | 아니오 | 예 | 아니오 | 아니오 | 아니오 |
| `fast-dense` | 예 | 아니오 | 아니오 | 예 | 예 |
| `fast-banded-dense` | 아니오 | 아니오 | 아니오 | 예 | 예 |
| `sequential-dense` | 예 | 아니오 | 예 | 예 | 예 |

adapter가 bit 동일해야 하는 직접 호출과 D2 결과:

| 방법 | 직접 호출 | 사례 | bit 동일 | 판정 |
|---|---|---|---|---|
| `rodas5p` | `vigilode::Rodas5p::solve(problem, span, y0, options)` | 7 | 7 | PASS |
| `dopri5` | `vigilode::Dopri5::solve(problem, span, y0, options)` 및 `vigilode::GeneralOde::solve(problem, span, y0, options, policy)` | 3 | 3 | PASS |
| `bdf1-bdf2-variable` | `rodas5p_integrators::integrate_bdf_adaptive_observed(ode, span, y0, BdfConfig::default(), adaptive, [t0, t1])` | 3 | 3 | PASS |
| `radau-iia3` | `rodas5p_integrators::integrate_radau_adaptive_observed(ode, span, y0, RadauConfig::default(), adaptive, [t0, t1])` | 3 | 3 | PASS |
| `pexprb54s4-fused` | `rodas5p_integrators::integrate_pexprb54s4_fused_adaptive_observed(ode, span, y0, adaptive, [t0, t1], FusedPhiKrylovConfig::default(), sequential)` | 1 | 1 | PASS |
| `rodas5p-lagged-richardson-v1` | `rodas5p_integrators::integrate_rodas5p_lagged_observed(ode, span, y0, adaptive, [t0, t1], Rodas5pLaggedOptions::default())` | 3 | 3 | PASS |
| `ppsw1-6b-schmitt-weiner-singly-implicit-peer-w` | `vigilode::PeerOde::solve(problem, span, y0, peer_options, allowed)` | 4 | 4 | PASS |
| `lawson-dopri5-declared-damped-rotation` | `vigilode::LawsonDopri5::solve(flow, residual_problem, span, y0, options)` | 1 | 1 | PASS |

비교한 양: 모든 출력 시각과 상태의 bit, 성공 여부, 수락/거절 step 수, attempts, driver work counter 전체. RODAS5P는 facade 경로 6개(dense-fast, banded-fast, matrix-free, fast-dense, fast-banded-dense, sequential-dense)를 모두 포함한다.

## 4. Auto가 하는 것과 하지 않는 것

하는 것:

- 후보는 adapter가 있고, 선언된 사실과 호출자가 허용한 `CapabilitySet` 아래에서 admission을 통과한 catalog 항목이다.
- 순서: (1) regime 적합성(stiff로 선언하면 stiff용 방법만, 그 외에는 explicit 방법이 먼저), (2) qualification, (3) catalog 순서. 계획에 catalog 항목마다 한 행씩 순위 또는 거부 사유가 들어 있다.
- 예(Robertson, stiff, 전체 허용, 미자격 포함): `rodas5p` > `ppsw1-6b-schmitt-weiner-singly-implicit-peer-w` > `bdf1-bdf2-variable` > `rodas5p-lagged-richardson-v1` > `radau-iia3`.
- 같은 문제에 기존 `GeneralOdePolicy`의 권한만 주면: `rodas5p` (PPSW1 starter는 Newton 권한이 없어 거부, 미자격 방법은 제외).
- 실측 증거는 호출자가 넘긴 `CostEvidence`만 쓴다. 정책이 `use_measured_evidence`를 켰을 때만, 같은 등급 안에서만 순서를 바꾸며, 그 증거를 얻는 데 든 callback 단위는 결과에 기록된다 (예: `rodas5p` > `ppsw1-6b-schmitt-weiner-singly-implicit-peer-w` > `rodas5p-lagged-richardson-v1` > `bdf1-bdf2-variable` > `radau-iia3`, 기록된 callback 단위 18).
- fallback은 정책이 켰을 때만 한다. 실패한 시도 다음 순위의 방법을 `t0`에서 다시 실행하고, 모든 시도를 결과에 남긴다. 실패한 시도의 work는 합계에 포함된다.

fallback 예(Robertson, regime unknown, 시도 예산 500):

| 시도 | 경로 | 성공 | 사유 | prefix 끝 | RHS 호출 | 수락 step |
|---|---|---|---|---|---|---|
| 1 | `dopri5/rhs-only` | 아니오 | DOPRI5 attempt budget exhausted (AttemptBudget) | 0.7103 | 3001 | 476 |
| 2 | `rodas5p/dense-fast` | 예 | - | 40 | 279 | 34 |
| 합계 | | | | | 3280 | 510 |

하지 않는 것:

- 기본값이 아니다. `GeneralOde`, `Rodas5p` 등 어떤 frontend도 Auto를 부르지 않는다(OD-L1-3).
- `route`는 아무것도 측정하지 않는다. stiffness 탐지나 timing pilot이 없다.
- 호출자가 허용하지 않은 capability를 쓰는 방법으로 넘어가지 않는다.
  (미분 권한이 없으면 fallback 후보가 없어 시도는 1번으로 끝난다.)
- 실패한 시도의 상태를 이어 쓰지 않는다(방법 전환이 아니라 `t0` 재시작).
- Lawson처럼 문제의 우변 의미를 바꾸는 방법은 순위에 넣지 않는다(명시 선택만).
- Auto가 더 빠르거나 정확하다는 주장은 이 노드에 없다.

## 5. 사용할 수 없는 것과 이유

- `ppsw1-6b-matrix-free-common-w`: adapter 없음. catalog에 적힌 사유: "no end-to-end driver: the matrix-free common-W backend is a stage backend of ppsw1::Session; PeerOde lowers the dense backend only and there is no matrix-free starter". 즉 matrix-free common-W backend는 `ppsw1::Session`의 stage backend일 뿐이고, 끝에서 끝까지 도는 driver와 matrix-free starter가 없다.
- BDF, Radau: `Jacobian::Dense`와 `Jacobian::Jvp`만 내린다. FD, banded, sparse-JVP, band 선언, mass, event, dense output, vector atol은 타입이 있는 거부다.
- PEXPRB54s4: `Jacobian::Jvp`인 autonomous 문제만. 그 외는 거부.
- lagged RODAS5P: `Jacobian::Dense`만.
- PPSW1: `Jacobian::Dense` 또는 `Jacobian::Jvp`, band 선언 없음, scalar atol.
- Lawson: 선언된 flow(`MethodParams::lawson_flow`)가 없으면 거부.
- 이번 후보 목록에 없어 여전히 facade에서 닿지 않는 것: small/batch RODAS5P, SABR, homotopy, transactional q1/q2, PTSW2B, `vigilode-coupled`.
- 다른 방법으로 조용히 대체하는 adapter는 없다.

## 6. 라우팅 권위: Phase A와 Phase B

- Phase A: `route(General)`은 `GeneralOde::select`를 호출한다. golden oracle의 결정 격자(221184건)를 같은 순서로 다시 열거해 `select`, golden 파일과 비교했다.
- Phase B: COD-03이 요구한 요청 확장을 `MethodFacts`로 넣었다(차원, provider 형태, band 선언, autonomy, 시간 미분, mass, event, dense output, interior-error 모드, atol 종류). catalog의 RODAS5P 항목에는 facade의 driver 계열 6개를 경로 variant로 넣었다(banded-fast, sequential-dense 포함, 3절 표). general 선택의 receipt(`implicit_route`)는 이전과 같이 facade의 `Rodas5p::capabilities`가 답한 경로다.
- 확장 격자는 golden 격자에 sparse-JVP provider 두 형태를 더한 331776건이다. 두 구현이 읽는 모든 술어의 곱이므로 전수 비교다. 결과 종류 23가지.
- debug profile 실행(debug assertion 켜짐): exit 0, 2 passed. catalog 유도 경로가 reference 본문으로 물러난 적이 없다.

COD-03의 13개 probe 사례(불일치 9건 포함)를 새 경로로 다시 실행한 결과:

| 사례 | `select` | `route(General)` 일치 | 같은 권한의 Auto |
|---|---|---|---|
| c01_banded_provider_n1000_stiff_provided | `Ok(GeneralSelection { method: Rodas5p, regime: Stiff, derivatives: ProvidedOnly, implicit_route: Some(BandedFast) })` | 예 | `rodas5p/banded-fast` |
| c02_fd_declared_band_n1000_stiff_allowfd | `Ok(GeneralSelection { method: Rodas5p, regime: Stiff, derivatives: AllowFiniteDifference, implicit_route: Some(BandedFast) })` | 예 | `rodas5p/banded-fast` |
| c03_jvp_n3_stiff_allowfd_no_matrix_free | `Err(MatrixFreeNotEnabled)` | 예 | 거부 |
| c04_dense_n3_nonautonomous_no_ft_stiff_provided | `Err(MissingTimeDerivative)` | 예 | 거부 |
| c05_dense_n3_event_stiff_provided | `Err(Events)` | 예 | 거부 |
| c06_dense_n3_mass_stiff_provided | `Err(MassMatrix)` | 예 | `rodas5p/sequential-dense` |
| c07_dense_n3_regime_unknown_provided | `Ok(GeneralSelection { method: Dopri5, regime: Unknown, derivatives: ProvidedOnly, implicit_route: None })` | 예 | `dopri5/rhs-only` |
| c08_dense_n3_dense_output_stiff_provided | `Err(DenseOutput)` | 예 | `rodas5p/fast-dense` |
| c09_jvp_n3_vector_atol_stiff_provided_matrix_free | `Err(Implicit(VectorAtolOnMatrixFreeRoute))` | 예 | 거부 |
| c10_dense_provider_declared_band_n100_control | `Err(DenseDimensionLimit)` | 예 | 거부 |
| c11_dense_n3_stiff_provided_control | `Ok(GeneralSelection { method: Rodas5p, regime: Stiff, derivatives: ProvidedOnly, implicit_route: Some(DenseFast) })` | 예 | `rodas5p/dense-fast` |
| c12_banded_provider_n10_label | `Ok(GeneralSelection { method: Rodas5p, regime: Stiff, derivatives: ProvidedOnly, implicit_route: Some(BandedFast) })` | 예 | `rodas5p/banded-fast` |
| c13_dense_n3_event_dense_output | `Err(Events)` | 예 | `rodas5p/fast-dense` |

mass, event, dense output 세 경우는 general surface 자체의 제한이다. 방법(RODAS5P)은 지원하므로 Auto와 Explicit에서는 admit된다. 나머지 거부는 양쪽이 같다.

D3 격자에서 `route(Explicit("rodas5p"))`의 계획은 18432 / 18432건이 실행되는 driver 계열과 같고, 그 variant 행은 요청한 mass, event, dense output을 지원한다. 거부는 `Rodas5p::capabilities`의 거부와 같다.

남긴 것: `MethodRequest`, `MethodRegistry::descriptors()`, `query()`(U06 표)는 바꾸지 않았다. `MethodRequest`의 struct literal을 허용 경로 밖의 `tests/geometry_mol_contracts.rs`가 쓰고, U06 테스트가 6개 항목을 고정하기 때문이다. `query()`는 provider 형태를 모르는 거친 권한 표로 남고, 라우팅은 `route`가 한다.

## 7. PeerOde 결함 수정 (COD-02, COD-04)

`ppsw1.rs`는 보호 파일이라 건드리지 않았다. 수정은 `selection/peer.rs`(adapter)에만 있다. 각 결함은 감사 probe에서 만든 테스트로 먼저 실패를 확인했다(`logs/failing_first_peer_defects.log`).

| 재현 테스트 | base 소스 | 수정 후 |
|---|---|---|
| `cod02_p3_fast_forcing_20_is_error_controlled_or_refused` | FAILED | ok |
| `cod02_p3b_fast_forcing_200_is_error_controlled_or_refused` | FAILED | ok |
| `cod02_p5_invalid_h0_is_refused_before_any_callback` | FAILED | ok |
| `cod04a_p1_jvp_provider_needs_the_jvp_permission` | FAILED | ok |
| `cod04b_p2a_starter_endpoint_above_t_end_is_a_success_with_exact_endpoint` | FAILED | ok |
| `cod04c_p4_recoverable_jacobian_error_is_reported_as_itself` | FAILED | ok |
| `cod04cd_p6_fatal_rhs_error_is_reported_as_a_fatal_callback` | FAILED | ok |
| `cod04e_p7_declared_band_is_refused_by_admission_too` | FAILED | ok |

| 사례 | base: 성공, h0, 오차(허용오차 단위) | 수정 후: 성공, h0, 오차(허용오차 단위), Peer step |
|---|---|---|
| y' = 20 cos 20t | 예, 0.5, 9.007e+06 | 예, 0.01562, 0.01551, 363 |
| y' = 200 cos 200t | 예, 0.5, 5.688e+07 | 예, 0.001953, 0.03488, 3437 |

수정 내용:

- starter 구간 오차 제어: step doubling. `h0`의 starter 상태를 `h0/2` starter 두 번과 비초기 node 5개에서 비교하고(`ppsw1::error_norm`, 호출자 허용오차), 추정이 1 이하일 때만 쓴다. 아니면 `h0`를 반으로 줄인다(최대 40회, 그 뒤 타입이 있는 실패). 추정이지 상계가 아니다.
- `h0`가 유한한 양수가 아니면 callback 전에 `InvalidInput`.
- admission이 provider 형태를 읽는다. JVP provider는 JVP 권한이 있어야 하고, band 선언은 admission에서 거부된다.
- 실패 사유는 `PeerFailure`(타입)이고, callback 오류는 종류(recoverable/fatal), callback, 메시지를 그대로 가진다. `failure` 문자열은 그 표시 형식으로 남겼다(기존 호출자 호환).
- Jacobian/JVP의 recoverable 오류는 그 사유로 멈춘다(선형화 점이 step 크기와 무관해 반감으로 고칠 수 없다).
- 끝점은 `t_end`에 정확히 착지시키고, `success`는 착지 여부다.

탐색적 회귀 확인: U07 계약 문제 4개 설정 중 4개에서 수락 궤적(시각과 상태의 FNV-1a digest)이 수정 전후 bit 동일하다. starter 제어가 첫 `h0`를 받아들였기 때문이다. startup work는 제어용 starter 2회만큼 늘었다(Jacobian build 2 -> 4, RHS 16 -> 48). 동결 기준이 아니다.

## 8. 실행한 명령

| 로그 | exit | passed | failed | ignored |
|---|---|---|---|---|
| `logs/contracts.log` | 0 | 31 | 0 | 0 |
| `logs/existing_registry_tests.log` | 0 | 23 | 0 | 0 |
| `logs/default_path_golden_run1.log` | 0 | 11 | 0 | 0 |
| `logs/default_path_golden_run2.log` | 0 | 11 | 0 | 0 |
| `logs/full_suite.log` | 0 | 497 | 0 | 0 |
| `logs/clippy.log` | 0 | - | - | - |
| `logs/fmt_check.log` | 0 | - | - | - |
| `logs/doc.log` | 0 | - | - | - |
| `logs/check_checkpoint.log` | 0 | - | - | - |
| `logs/failing_first_peer_defects.log` | 101 | 1 | 8 | 0 |
| `logs/peer_defects_after_fix.log` | 0 | 21 | 0 | 0 |
| `logs/decision_grid_debug_profile.log` | 0 | 2 | 0 | 0 |

`logs/failing_first_peer_defects.log`의 exit 101은 의도한 결과다(수정 전 소스에서 재현 테스트가 실패). 각 로그의 첫 줄들에 명령, commit, 시각, load average가 있다.

## 9. 발견한 것과 한계

- U06 registry의 `radau-iia3`는 2단계/3차라고 적혀 있었다. `radau.rs`의 기본 설정은 3단계 Radau IIA 5차(내장 3차 추정)다. descriptor를 코드에 맞게 고쳤고 테스트가 코드 값과 비교한다.
- BDF, Radau, PEXPRB, lagged driver는 실패했을 때 마지막 수락 상태를 내주지 않는다. 그래서 view의 prefix는 도달한 출력 표본(끝점 일정이면 `t0`)뿐이다. fatal callback 오류에서는 driver가 기록 없이 오류만 돌려주므로, adapter가 타입이 있는 사유와 callback 경계에서 센 호출 수를 남긴다. 더 나은 prefix는 `rodas5p-integrators` 수정이 필요하다(이 노드 범위 밖).
- 새로 확인한 facade 결함(고치지 않음, `capabilities.rs`와 `solve.rs`의 RODAS5P 경로는 이 노드 범위 밖): 일반(비 band) sparse-JVP provider에서 `Rodas5p::capabilities`는 항상 `MatrixFree`라고 답하지만, `Rodas5p::solve`는 dense output이면 fast dense driver를, vector atol이면 dense fast driver를 실행하고 Jacobian을 JVP 열 n개로 조립한다. 실행 결과:

  | 요청 | capability query | 계획한 경로 | 실행된 경로 | entry point | Jacobian build | JVP 호출 |
  |---|---|---|---|---|---|---|
  | plain | MatrixFree | `rodas5p/matrix-free` | `rodas5p/matrix-free` | `integrate_rodas5p_routed_observed` | 0 | 1023 |
  | dense output | MatrixFree | `rodas5p/fast-dense` | `rodas5p/fast-dense` | `integrate_rodas5p_fast_dense_observed` | 8 | 128 |
  | vector atol | MatrixFree | `rodas5p/dense-fast` | `rodas5p/dense-fast` | `integrate_rodas5p_fast_observed_with_atol` | 8 | 128 |

  registry의 계획은 실제로 실행되는 driver와 그 dense-matrix 필요를 적는다(view는 실행 기록에서 경로를 다시 읽어 확인한다). 그래서 Auto는 dense-matrix 권한 없이 이 driver로 가지 않는다. 격자에서 capability query가 실행되지 않는 driver를 답한 요청은 704건이다.
- 같은 결함의 기본 경로 쪽 귀결(바꾸지 않음, 기본 라우팅 불변): 일반 sparse-JVP + vector atol에서 `GeneralOde::select`는 `Some(MatrixFree)`를 고르고 dense 차원 상한을 적용하지 않는다(예: n = 16, 상한 4). 실제로는 `rodas5p/dense-fast`가 실행되어 n x n 행렬을 만든다. 이전 본문도 같은 결정을 한다. 소유자 결정이 필요하다.
- dense provider에 band를 선언하면 보호 router가 `(t0, y0)`에서 callback으로 band를 검증한다. 그 거부(`SolveError::StructureRefused`)는 라우팅 거부가 아니라 직접 호출과 같은 solve 단계 오류다.
- `GeneralOde::select`는 이제 호출마다 catalog를 만든다(작은 heap 할당). 수치 workspace 할당이나 callback은 없다. hot loop에서 불리지 않는다.
- Auto와 fallback에는 성능 측정이 없다. 이 노드의 증거는 결정 동등성과 bit 동등성뿐이다.
- 출력 격자를 호출자가 지정하는 기능은 넣지 않았다(직접 호출과의 bit 동등성을 유지하기 위해 각 방법의 고유 격자를 그대로 보고).

## 10. 재현

```sh
sh research/production_loop_20261116/unified_frontend/run_checks.sh \
    contracts existing golden1 golden2 debuggrid suite clippy fmt doc checker diff
python3 -I research/production_loop_20261116/unified_frontend/build_receipt.py
python3 -I research/production_loop_20261116/unified_frontend/build_report.py
```
