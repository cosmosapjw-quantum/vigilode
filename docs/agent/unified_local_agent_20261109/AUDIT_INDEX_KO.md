# 외부 감사 색인 (AUDIT INDEX) — VigilODE unified DAG, T02 / PW 트랙

- 브랜치: `research/unified-t02-20261110` (베이스 `bd82eff9d1d967ef489ac4eb0c302ef6a4102f31`, 고정 코드 소스 C5 `569ce026…`, `crates/` 트리 동일)
- 성격: 이 브랜치는 **연구 증거 + opt-in 모듈 2개**이다. 기본값, 보호 솔버, tableau, 임계값, G0는 바꾸지 않았다. **Production HOLD, G0 미충족, 릴리스 제안 아님.**
- 원 G0 불변: 23 nodes / 34 edges, 7/23, successor 0/12. 목표(전역 차수 ≥4.8, accepted-step ≤1.2, 속도 ≥1.15, fast lane J/Newton 0, depth p95≤6, fallback≤0.1)는 **하나도 입증되지 않았다**(PW06/T09 미실행).

## 1. 먼저 읽을 파일
1. `docs/agent/unified_local_agent_20261109/OWNER_DECISIONS_20261110.json` — 오너 결정 OD-1..27과 미결정 목록
2. `docs/agent/unified_local_agent_20261109/VIGILODE_UNIFIED_DAG_20261109.json` — 노드별 status/evidence/result_note/receipt
3. 노드별 보고서 (아래 §3), 영수증 JSON은 명령, exit code, stdout/stderr sha256, 스크립트 원문 포함
4. `docs/agent/unified_local_agent_20261109/PACKAGE_MANIFEST.json` — 파일 sha256 (재생성 가능)

## 2. 코드 변경 (전부 opt-in, 기본 라우팅 미연결)
| 경로 | git blob SHA-1 |
|---|---|
| `crates/rodas5p-integrators/src/ptsw2b.rs` | `f491b0a3f5f351d0fe933b06bf9ea30a0c730922` |
| `crates/rodas5p-integrators/src/ppsw1.rs` | `d7386d03b0db085c1116f56c9f2fdc10a52c1e61` |
| `crates/rodas5p-integrators/src/lib.rs` | `6bb0975eb926f74de889705e895ba50e2d50d076` |
| `crates/rodas5p-integrators/tests/ptsw2b_contracts.rs` | `b9a5d3eab54d05909f289441dce3359302257d3c` |
| `crates/rodas5p-integrators/tests/ppsw1_6b_contracts.rs` | `efe42fdcc2f3155fc8f07556700218fd951ad6fa` |

- `lib.rs`는 `pub mod ptsw2b;`, `pub mod ppsw1;` 두 줄만 추가(`git diff bd82eff9 -- crates/rodas5p-integrators/src/lib.rs`로 확인). `candidates.rs`의 PeerW는 Deferred 그대로. 보호 소스 sha256 가드 테스트가 두 계약 테스트에 있다.
- 새 모듈 본문(테스트 모듈 제외): `unsafe`, `unwrap`, `expect`, `panic!`, `#[allow]` 0건(정적 계수, 조정자 확인).

## 3. 노드 상태 (DAG에서 생성)
| node | status | evidence |
|---|---|---|
| T00_LOCK_AND_PLAN | COMPLETE_SCOPED | REMOTE_READBACK_VERIFIED |
| T01_HISTORICAL_LINEAGE_AND_SOURCE_REPLAY | COMPLETE_SCOPED | REMOTE_METADATA_AND_SELECTED_SOURCE_READBACK |
| T01A_INTEGRATED_PORTFOLIO_ARCHITECTURE | COMPLETE_SCOPED | REMOTE_READBACK_VERIFIED |
| T02_CURRENT_GLOBAL_ERROR_AUTHORITY_REPLAY | PARTIAL_SCOPED | NUMERICALLY_CHECKED |
| T03_C5_COST_ATTRIBUTION_AND_LONG_PATH_PREREG | PLANNED | NOT_RUN |
| T04_C5_LONG_PATH_NATIVE_FULL_IVP_HOLDOUT | PLANNED | NOT_RUN |
| T05_C5_WORK_EFFICIENT_ORDERED_SCAN | CONDITIONAL_GATE | NOT_RUN |
| T06_BDF_RADAU_AND_PEER_METHOD_QUALIFICATION | PLANNED | NOT_RUN |
| PW00_PEER_W_SOURCE_AND_API_AUDIT | COMPLETE_SCOPED | REMOTE_READBACK_AND_SYMBOLIC_CHECKS |
| PW01_PTSW2C_ORDER3_FIRST_PEER_IMPLEMENTATION | PLANNED | NOT_RUN |
| PW02_PTSW2B_LSTABLE_ORDER2_CONTROL | PARTIAL_SCOPED | IMPLEMENTATION_VERIFIED |
| PW03_G0_ORDER5_PEER_W_TABLEAU_SOURCE_RECOVERY | PARTIAL_SCOPED | DERIVED_AND_NUMERICALLY_CHECKED |
| PW04_PEER_HISTORY_STARTUP_ERROR_AND_SESSION | PARTIAL_SCOPED | IMPLEMENTATION_VERIFIED |
| PW05_RHS_ONLY_IMPLICIT_PEER_HOMOTOPY | CONJECTURAL_RESEARCH_PLANNED | UNRESOLVED |
| PW06_PEER_MATCHED_FULL_IVP_AND_G0_GATE | PLANNED | NOT_RUN |
| PW07_COMPOSITIONAL_ERROR_CERTIFICATE_AND_MOCK_EXPERIMENT | COMPLETE_SCOPED_RUST_NOT_RUN | PYTHON_SYMPY_REPLAYED_REMOTE_READBACK |
| T07_CAPABILITY_FRONTEND_AND_METHOD_PLUGIN | PLANNED | NOT_RUN |
| T08_SPATIAL_MOL_SPHERE_AND_3D_GEOMETRY | PLANNED | NOT_RUN |
| T09_MATCHED_FULL_IVP_BENCHMARK_AND_RELEASE_REVIEW | PLANNED | NOT_RUN |

보고서: T02 `GLOBAL_ERROR_AUTHORITY_REPLAY_KO.md`; PW01 `PW01_PW02_READINESS_AUDIT_KO.md`, `PW01_CONTROLLER_CONSTANTS_RECOVERY_KO.md`, `PW01_THESIS_PODHAISKY2002_RECOVERY_KO.md`; PW02 `PW02_SCAN_AND_RED_TEST_KO.md`, `PW02_GREEN_RECEIPT_KO.md`, `PW02_LINT_CLEANUP_RECEIPT_KO.md`; PW03 `PW03_PEERW_P5_LITERATURE_RECOVERY_KO.md`, `PW03A_PPSW2_6_DERIVATION_KO.md`, `PW03B_PPSW2_6_QUALIFICATION_EVIDENCE_KO.md`; PW04 `PW04_MIPEER6_AND_W_TOLERANCE_KO.md`, `PW04_COMMON_W_ALTERNATIVE_SEARCH_KO.md`, `PW04_PPSW1_6BC_VERIFICATION_KO.md`, `PW04_PPSW1_STARTER_ESTIMATOR_LITERATURE_KO.md`, `PW04_PPSW1_6B_RED_TEST_KO.md`, `PW04_PPSW1_6B_GREEN_RECEIPT_KO.md`.

## 4. 검증 수준의 구분 (감사자가 가장 먼저 확인할 것)
**코디네이터가 직접 재실행/대조한 것**: 소스 blob 28개와 4개 T02 blob, 통합/부모 DAG 검증기, fair-ab 계약 테스트 44개(T02), PW02 `ptsw2b_contracts` 24, `unified_candidate_contracts` 13, `ppsw1_6b_contracts` 37, `--lib ppsw1` 4, `cargo fmt --check`, clippy(진단 0건: 새 파일), PW03a 유도 스크립트, PW03B z2/z6, PW04 m1~m4, PPSW1 검증 s02/s03, 추정기 라그랑주 가중치(정확 유리수), 박사논문 p.45-46의 컨트롤러 상수와 PTSW2B 노드, SINUM Table 2와 γ 값.
**에이전트 보고만 있는 것(재실행 안 함)**: PW04 m5/m6(W-허용 창), PPSW1 s04/s05(σ≤1.10 안정성, 합성 전역 차수), PW03B z1/z3-z5/z7, 설계 A starter 프로토타입 수치, 웹 문헌 요약 일부(EPPEER, Klinge arXiv:1902.01161 등은 웹 요약 기반), end-to-end 통합 테스트의 전역 차수 수치 해석.
**독립성 한계**: 테스트, 임시 오라클, 구현이 같은 에이전트 계열이 작성했다. 외부 감사의 적대적 리뷰(T09)가 필요하다.

## 5. 알려진 미해결 / NOT_RUN / 한계
- 전체 G0 게이트(차수 4.8, ≤1.2 steps, 속도 1.15, depth, fallback)와 PW06 매칭 오차 전체 IVP 벤치마크: **NOT_RUN**.
- 강성 전역 차수(PR λ=−1000, 강성 3×3)는 NOT_CONCLUSIVE. 스텝 수가 1.10 성장 cap에 묶여 적합 차수가 의미 없음.
- W-허용: T=cJ는 c=1.0/1.2 수렴, 0.8/0.6 발산(0.8–1.0 미실행). G0의 "fast lane explicit-J 0"과 충돌할 수 있음(오너 결정 OD-14: G0 우선, 문제 시 W-TOLERANCE-FALLBACK 명시).
- PPSW1-6b: 오차추정기는 원문 서술(차수 s−2 예측자)의 해석, 제어기는 OWNER_CHOICE(박사논문 PTSW 제어기), starter는 원문에 없어 설계 A를 오너가 승인. 시작 구간이 h0 해상도 요건을 가진다(h0·|λ|≲1).
- PPSW2-6 및 mipeer6은 multi-implicit(스텝당 분해 6회)이라 대안 문서로만 보존. PPSW1-6c는 검증됐으나 구현하지 않음.
- 문서 정정(이력): PTSW2B/C 박사논문 상수 전부 SOURCE_PRINTED, 인증서 상수는 원래 SECONDARY였음. PPSW2는 multi-implicit(앞선 일부 요약의 공통 W 전제는 PTSW/PPSW1에만 해당).
- 부모 패키지 검증기 불일치(T02-F4)는 데이터 쪽을 교정했고(검증기 완화 없음, 이력은 git에 보존) 검증기 PASS.
- clippy `-D warnings`는 기존 `source_owned_graph_schur.rs`(보호, 고정 blob 5d2c4319…)의 8건 때문에 이 베이스에서 어떤 변경으로도 통과 불가(변경 없음).
- 워크스페이스 전체 `cargo check/test`: 결과는 `WORKSPACE_CHECK_RECEIPT.json` 참조(실행하지 못한 항목은 NOT_RUN).
- 박사논문과 논문 PDF 3편은 저작권 자료라 **repo에 포함하지 않음**(루트 경로 참조만; 해시는 영수증에 있음).

## 6. 재현 명령 (worktree 루트)
```
python3 -I docs/agent/unified_local_agent_20261109/validate_unified_dag.py
python3 -I docs/agent/solver_consolidation_20261108/validate_package.py
export CARGO_TARGET_DIR=<scratch>
cargo test --locked --release -p rodas5p-integrators --test ptsw2b_contracts      # 24
cargo test --locked --release -p rodas5p-integrators --test ppsw1_6b_contracts    # 37
cargo test --locked --release -p rodas5p-integrators --test unified_candidate_contracts  # 13
cargo test --locked --release -p rodas5p-integrators --lib ppsw1                  # 4
cargo test --locked --release -p rodas5p-fair-ab --test global_error_contracts    # 9 (T02 set: 6 test binaries, 44 tests)
cargo fmt -p rodas5p-integrators -- --check
cargo clippy --locked -p rodas5p-integrators --all-targets --release --no-deps --message-format=short
```
스크립트 기반 검증은 각 영수증 JSON의 `script_text`를 `python3 -I`로 재실행하고 `stdout_sha256`와 비교한다.

## 7. 이 브랜치의 커밋 (베이스 이후)
```
0536eb3e 2026-11-10 docs(agent): add unified local-agent package and T02 global-error authority replay
cfffdabd 2026-11-10 docs(agent): fix parent validate_package.py mismatch (T02-F4) data-side
6c84410f 2026-11-10 docs(agent): PW01/PW02 pre-implementation readiness audit
c519cec2 2026-11-10 docs(agent): PW03 Peer-W p>=5 literature recovery; update PW01/02/03 DAG nodes
058ce4c9 2026-11-10 docs(agent): PW03a PPSW2-6 derivation, owner decisions OD-1..4, DAG update
e58864e3 2026-11-10 test(peer): PW02 PTSW2B RED contract tests + scan, oracle GREEN check (scratch)
78c2610a 2026-11-11 feat(peer): opt-in PTSW2B order-2 L-stable two-step Peer-W control (PW02 GREEN)
f56e9fd8 2026-11-11 test(peer): lint-only cleanup of ptsw2b contract tests (3 clippy warnings)
019f4455 2026-11-11 docs(agent): PW01 controller-constant literature recovery (negative result)
02ffeaba 2026-11-11 docs(agent): PW03B PPSW2-6 qualification evidence; DAG PW01/PW03 notes
18c3f456 2026-11-11 docs(agent): PW01 controller constants recovered from Podhaisky 2002 thesis
397c2cfa 2026-11-11 docs(agent): PW04 candidate selection - mipeer6 check and W-tolerance analysis
dcc50cb0 2026-11-12 docs(agent): record owner decisions OD-15..18 for PW04
aa7111a5 2026-11-12 docs(agent): PW04 common-W search: PPSW1-6b/6c singly implicit order-5 candidate
62ed1f31 2026-11-12 docs(agent): owner decisions OD-20..22 for PPSW1-6b/c
60eea115 2026-11-12 docs(agent): PW04 PPSW1-6b/6c verification (two routes, order, stability, global order)
fbb5b82f 2026-11-12 test(peer): PW04 PPSW1-6b RED contract tests (27 + 1 ignored)
183213ea 2026-11-12 docs(agent): PW04 PPSW1-6b starter, estimator and controller literature + prototypes
1e99b1d6 2026-11-12 docs(agent): owner decisions OD-23..27 (PPSW1-6b starter, h0, estimator, controller, audit push)
dcd7f7e8 2026-11-12 feat(peer): opt-in PPSW1-6b singly implicit order-5 two-step Peer-W (PW04 GREEN)
(+ 이 감사 색인 커밋: `docs(agent): external audit package ...`, 푸시 후 `git log -1`로 확인)
```

## 8. 감사자에게 요청하는 점검
1. 위 §4의 "에이전트 보고만 있는 것"을 독립 재현.
2. PPSW1-6b: 추정기 해석(4차 예측자), 시작기 설계 A의 정확도 주장, σ≤1.10 안정성, h0 규칙의 적절성.
3. 보호 경로 불변(blob 비교)과 opt-in 격리(기본 라우팅 불변, PeerW Deferred).
4. 증거 등급 표기(NUMERICALLY_CHECKED vs IMPLEMENTATION_VERIFIED)와 NOT_RUN을 PASS로 오기한 곳이 없는지.
5. 오너 결정 OD-1..27이 임계값/기본값/G0를 변경하지 않았는지.
