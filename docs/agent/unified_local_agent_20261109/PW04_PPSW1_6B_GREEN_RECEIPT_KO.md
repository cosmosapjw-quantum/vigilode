# PW04 PPSW1-6b GREEN 영수증 (OD-23..26 반영)

**상태:** 계약 테스트 GREEN (37/37). 기계 판독본/해시: `PW04_PPSW1_6B_GREEN_RECEIPT.json`.

## 변경 (허용 목록만)
- 신규 `crates/rodas5p-integrators/src/ppsw1.rs` (1138줄, sha256 `f2650c79...a25d`).
- `src/lib.rs`: `pub mod ppsw1;` 한 줄 추가. cargo fmt가 알파벳 순서(`polynomial_resolvent` 다음)를 요구해 그 위치에 둠. 한 줄을 제거하면 blob SHA-1 `dc734d29...`(PW02 GREEN 후 값)이 복원된다. 현재 blob `6bb0975e...`.
- 테스트 파일 `tests/ppsw1_6b_contracts.rs` 확장(sha256 `a4e2f053...83a7`). 기존 27개 테스트의 assertion은 변경 없음. 변경은: `#[ignore]` 플레이스홀더 삭제(대체 테스트 h1-h8, i1-i2 추가), `use std::sync::{Arc, Mutex}`. 새 테스트 10개 -> 총 37개.
- 다른 파일/Cargo/ptsw2b/candidates 수정 없음. git 명령 사용 없음.

## 구현 요약
- 계수: DD(double-double)로 `Theta=VSPV^-1`, `Gamma=-gamma Theta`, `B=(I+gamma E)Theta+sigma beta E`, `A=beta+gamma Theta` 계산 후 f64 1회 반올림. gamma는 38자리 상수(60자리는 테스트 b2가 정확 정수로 인증).
- OD-23 starter: 격자 기준 t0=t_init+h0, 전방 5단계 Radau IIA 한 스텝(`2 h0`), 노드와 Lagrange 적분 tableau를 모듈 내에서 구성(Legendre 근 bisection), frozen T의 단순 Newton, 5차 collocation 다항식으로 6개 노드값. Newton은 starter에만 있다.
- OD-24: `jacobian_norm_bound`(무한노름), `select_h0(norm, h_max)=min(h_max,1/norm)`, 비정상 입력은 `InvalidInput`; 사용자 h0도 `start`가 검증.
- OD-25: `estimator_weights/estimator_vector`(정확 Lagrange, 점 c=-1/2,0,2/5,4/5,1, x=1+sigma; sigma=1에서 [512/117,-30,625/9,-1250/13,160/3] 확인), RMS 노름, 지수 1/5, `rounding_floor_warning`(sum|w| eps |Y| > 0.1*tol), `Session::try_step`.
- OD-26 (OWNER_CHOICE, 문헌 인증 아님): `propose_ratio` = min{cap, max{0.5, 0.85 Err^(-1/5)}}, [0.95,1.05] 유지, 최근 6 ratio 성장 이력 규칙, NaN이면 0.5 거부.
- sigma>1.10은 `InvalidInput`(기존 e4 테스트 유지).

## 검증 (한 번에 하나, CARGO_TARGET_DIR=/mnt/extdrive/vigilode-t02-20261110/target)
| 명령 | exit | 결과 |
|---|---|---|
| `cargo test --locked --release -p rodas5p-integrators --test ppsw1_6b_contracts` | 0 | 37 passed |
| `--test ptsw2b_contracts` | 0 | 24 passed |
| `--test unified_candidate_contracts` | 0 | 13 passed |
| `--lib ppsw1` | 0 | 4 passed (120 filtered; 전체 lib NOT_RUN) |
| `cargo fmt -p rodas5p-integrators -- --check` | 0 | 출력 없음 (1차는 mod 순서 diff로 exit 1, lib.rs 위치 수정 후 통과) |
| `cargo clippy --locked -p rodas5p-integrators --all-targets --release --no-deps` (-D 없음) | 0 | ppsw1.rs/ppsw1_6b_contracts.rs/lib.rs 진단 0건; `source_owned_graph_schur.rs` 9건은 기존. 1차의 ppsw1.rs 5건(needless_range_loop)은 반복자로 고쳤고 allow 억제는 쓰지 않음 |
| 보호 경로 | - | PW04 RED의 g3 테스트가 14개 sha256 + lib.rs(한 줄 제거 시 원본)를 검증(통과). blob SHA-1 재계산: lib.rs만 +1줄 |
sha256(stdout/stderr 앞 16자)은 JSON에 있다.

## End-to-end (합성, qualification 아님; i1, i2)
PR lambda=-1, y=cos(10t), [0,2], 실제 starter+estimator+controller, tol 1e-2..1e-6: 오차 1.77e-3, 5.56e-4, 3.10e-5, 3.27e-6, 2.23e-7 (수락 스텝 61, 65, 79, 116, 188, 기각 0,0,0,0,1). 오차-스텝 적합 기울기 7.65(사전 점근 영역 혼합, 참고치). Newton은 starter에만(session nonlinear_iterations 0), 스텝당 LU 1회.
강경직 3x3 (lambda -1,-50,-1000, 회전 A, y=g(t)): tol 1e-2..1e-6 오차 5.56e-5(3개 동일), 1.02e-5, 1.10e-6, 5.08e-8 (스텝 250..318). 스텝 수가 1.10 성장 cap과 h0~1/|lambda|로 결정되어 적합 차수 27.9는 의미 없음(참고만); 테스트는 단조 감소만 요구한다. 이 두 실험으로 전역 차수를 주장하지 않는다.
- 초기 e2e 시도: 같은 실험이 tol에 무관하게 같은 오차를 내어(문제가 너무 쉬움, 주파수 4/3/5) 주파수를 올렸다.

## 자체 감사 10문항
1 worktree/브랜치 research/unified-t02-20261110 (git 미사용). 2 근거: PW04 검증, 소유자 OD-23..26. 3 유형: opt-in Peer-W. 4 방법 사실: 6 stage, 공통 W, 변보폭 차수 5, 노드행 V. 5 파생 정보: ExactJacobianOrJvpFrozenAtStepStart. 6 잔차: 단계 대수 잔차와 order 조건 테스트; 전역 오차 한정 없음. 7 오차 authority: estimator는 OWNER_CHOICE(보간 결함), 인증된 error bound 아님. 8 charge: jacobian_builds(+jvp), direct_factorizations, linear_solves, direct_solve_calls, rhs_calls, nonlinear_* (starter). 9 허용 경로 준수(ppsw1.rs 신규, lib.rs 1줄, 테스트 파일). 10 중단 조건: 계약 테스트 실패 또는 보호 sha 변경.

## 코드 자체 점검
unsafe 0 (crate forbid), 테스트 모듈 밖 unwrap/expect/panic 0 (`unwrap_or`만), 테스트 모듈 unwrap 2(radau_nodes), 입력 오류는 CoreError.

## UNRESOLVED / NOT_RUN
강경직 전역 차수(PR -1000, LIN3) NOT_RUN; W 허용 구간 0.8-1.0 NOT_RUN; controller/estimator는 OWNER_CHOICE이며 PPSW1 문헌 인증 아님; 시작기 정확도 O(h0^5)는 노드 테스트로만 확인(h0 선택이 전역 오차를 좌우); 마지막 스텝 clipping(작은 sigma) 안정성, t_end<2h0, event, matrix-free, mass matrix, vigilode 기본 라우팅, 전체 `--lib` NOT_RUN; i128/DD 외 구현 독립 리뷰 NOT_RUN.
