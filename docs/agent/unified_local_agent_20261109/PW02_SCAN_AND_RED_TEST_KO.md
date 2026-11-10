# PW02 (PTSW2B order-2 L-stable Peer-W CONTROL) 스캔과 RED 테스트

**상태: RED 확정, oracle GREEN 확인(24/24).** 테스트 파일 작성 완료. RED는 PASS-as-RED (5절). oracle(scratch, 구현 미전달) 24 passed / 0 failed, exit 0. 상세 기계 판독본: `PW02_SCAN_AND_RED_TEST.json`.

## 1. Owner 결정 반영
accepted-step 상한 <=1.10x RODAS5P, PTSW2C와 구별되는 opt-in order-2 CONTROL, 자체 startup/estimator/controller(RODAS 가중치 재사용 금지), step 내 stage 병렬(공통 W), shared-Krylov 주장 없음.

## 2. 스캔 결과
- 형태: 새 파일 `src/ptsw2b.rs` + `lib.rs`의 `pub mod ptsw2b;` 한 줄(기존 파일 수정, owner 결정). crate 루트 재export 없음. 테스트 파일은 Cargo 자동 발견(Cargo.toml 수정 불필요).
- `candidates.rs`의 PeerW Deferred 스텁과 `unified_candidate_contracts.rs`는 변경 금지(PW02는 catalog에 등록하지 않음).
- 작업량 charge는 sequential.rs 관례: step당 `direct_factorizations`+1, solve당 `linear_solves`/`direct_solve_calls`+1, `jacobian_builds`+1, stage당 `rhs_calls`+1.
- OdeProblem은 Jacobian 또는 JVP 콜백을 반드시 가진다(`new`가 강제). `dense_jacobian`은 둘 다 처리(JVP면 n회 JVP 호출 charge). FD-JVP는 facade가 Jacobian 콜백으로 감쌀 때만 존재.
- 병렬 실행: `ParallelExecution::map_ordered` 재사용(두 번째 pool 금지).

## 3. 파생 정보 class 제안 (owner 결정 PENDING)
`ExactJacobianOrJvpFrozenAtStepStart`: `T_m = J(t_m,u_m)`를 step 시작에 한 번 얻고(해석적 J 또는 정확 JVP), dense LU 하나를 두 stage가 공유한다. derivative-free 아님. matrix-free 경로는 v1에서 제외.

## 4. 제안 API (`rodas5p_integrators::ptsw2b`)
상수: METHOD_ID, STAGES, FORMAL_ORDER, ACCEPTED_STEP_CEILING_VS_RODAS5P=1.10, SAFETY, RATIO_CLIP, HYSTERESIS_BAND, HYSTERESIS_HISTORY.
함수: gamma, nodes, b, a_matrix, gamma_matrix, v, b_embedded, v_embedded, stiff_limit_matrix, startup_plan, stage_inputs, solve_stages, embedded_error_vector, error_norm, accepts, propose_ratio, advance, identity.
타입: State, StageInput, StageSolve, StepOutput, Config, ControllerState, Attempt, Session, Identity, DerivativeClass, StartupPlan.

## 5. RED 영수증
- 1차: exit 101, stdout sha256 `e3b0c442...b855`(빈 출력), stderr sha256 `156f1ab186f2ba13a60528f27449d8b2be7d0f004859b5b2bb11fc6579e924bf`. 에러: `E0432 unresolved imports rodas5p_integrators::ptsw2b`(의도한 RED) 1개와 `E0689 powi ambiguous float` 6개(테스트 버그).
- 수정: `for sf in [0.5, ...]` -> `[0.5_f64, ...]`(타입 지정만, assertion 변경 없음). rustfmt 통과.
- 2차 재실행(수정 후): exit 101, stdout sha256 `e3b0c442...b855`(빈 출력), stderr sha256 `c13e247572f4b7bc997c07b442e8961eccca9415195692f5903a82b1ff49f7b9`. 유일한 에러는 `E0432 unresolved imports rodas5p_integrators::ptsw2b`. 판정: **PASS-as-RED**. (lock 경합은 해소됨.)
- oracle GREEN 검증: worktree 밖 scratch 복사본(fixtures 포함)에 폐기용 `src/ptsw2b.rs`와 `pub mod ptsw2b;`를 두고 실행: `24 passed; 0 failed`, exit 0, 출력 sha256 `303a93d6...1e141`. 테스트 수정은 `0.5_f64` 타입 지정뿐, assertion 약화 없음. 이 구현은 worktree에 복사하지 않았다. 테스트 oracle 일관성만 검증하며 구현 증거가 아니다.

## 6. 테스트 매트릭스
JSON `tests[]` 참조(24개). worktree에서는 RED(컴파일 에러)만 실행, scratch oracle에서 24개 모두 PASS. h1/h2는 구현 없이도 통과할 보호 가드이고 나머지는 모듈이 있어야 GREEN.

## 7. startup 옵션 (owner 결정 PENDING)
S1 호출자 제공 State(테스트 사용), S2 DOPRI5 forward starter(비강성 한정, 정확도 미검증), S3 보호된 RODAS5P 읽기 전용 호출, S4 crate 내 radau/bdf(reference-only 라벨), S5 ptsw2b 내부 자체 starter. 제외: `u1=u0, k0=u0'`(CERT 4가 비권장).

## 8. allowlist 및 변경 금지
허용(구현 단계): `src/ptsw2b.rs`(신규), `lib.rs` 한 줄, 본 테스트 파일. 변경 금지 목록과 시작 시점 blob SHA-1은 JSON(`protected_byte_identical`, `blob_sha1_at_start`). 보호 파일 sha256은 테스트 h2에 고정.

## 9. UNRESOLVED
σ 허용 도메인(clip [0.5,2] 밖), L(90) 섹터 증명(source-reported), t_end<t1 및 startup 구간 event/비용 charge, reject 시 "work state 불변"의 의미(테스트는 accepted 불변, rejected+1, 실패 작업 charge로 해석), vigilode 쪽 기본 라우팅 불변 검사(NOT_RUN), lib.rs 한 줄 수정 허용 여부.
