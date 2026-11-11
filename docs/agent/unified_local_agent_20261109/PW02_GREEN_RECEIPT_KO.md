# PW02 GREEN 영수증 (PTSW2B order-2 L-stable Peer-W CONTROL)

**상태:** 계약 테스트 GREEN. clippy는 기존(pre-existing) 파일의 오류로 실패(exit 101). 상세/해시: `PW02_GREEN_RECEIPT.json`.

## 변경
- 신규 `crates/rodas5p-integrators/src/ptsw2b.rs`.
- `crates/rodas5p-integrators/src/lib.rs`: `pub mod ptsw2b;` 한 줄만 추가(`mod problems;` 다음). 이 줄을 제거하면 원본 blob `9d427381...`이 정확히 복원된다. 현재 blob `dc734d29...`.
- blobs.json 30개 경로 중 lib.rs만 달라졌다. 테스트 파일과 candidates.rs 등 보호 파일은 불변.

## owner 결정 반영
파생 정보 class `ExactJacobianOrJvpFrozenAtStepStart`, startup S5(모듈 내부 ROS2 starter, RODAS5P/DOPRI5 미호출), reject 의미(accepted 불변, rejected+1, 실패 작업 charge, history/controller 불변), 계수는 정확한 Q(√3) 형태가 진실이고 런타임 f64는 정수 isqrt bracket 재계산(양 끝이 같은 f64로 반올림될 때만 인정, 모듈 내 테스트로 인증), 상한 1.10.

## 검증 (실행한 것만)
| id | 명령 | exit | 결과 |
|---|---|---|---|
| a | `cargo test --locked --release -p rodas5p-integrators --test ptsw2b_contracts` | 0 | 24 passed / 0 failed |
| b | 같은 명령 `--test unified_candidate_contracts` | 0 | 13 passed |
| c | `cargo fmt -p rodas5p-integrators -- --check` | 0 | 출력 없음 |
| d | `cargo clippy --locked -p rodas5p-integrators --all-targets --release --no-deps -- -D warnings` | 101 | 실패. 모든 오류가 `source_owned_graph_schur.rs`(변경 없음)에 있고 ptsw2b.rs/lib.rs를 지목하는 진단은 없다. base blob에서의 재실행은 하지 않았다. 억제(suppress)는 추가하지 않았다 |
| e | blob SHA-1 30개 재계산 | - | lib.rs만 변경 |
| f | `cargo test --locked --release -p rodas5p-integrators --lib ptsw2b` | 0 | 5 passed. 전체 `--lib`(120개) 중 ptsw2b 필터만 실행, 전체는 NOT_RUN |

stdout/stderr sha256과 24개 테스트 이름은 JSON에 있다.

## 코드 자체 점검 (정적, 독립 리뷰 아님)
unsafe 0(crate가 `forbid(unsafe_code)`), 테스트 모듈 밖 `unwrap`/`expect`/`panic!` 0. 공개 API 목록은 JSON `code_review_self_check`.

## 자체 감사 10문항
1. 저장소: 위 worktree, 브랜치 research/unified-t02-20261110 (HEAD/tree는 git 미사용으로 조회 안 함).
2. 근거 blob: blobs.json 30개. 사용 API는 읽기 전용(`dense_jacobian`, `eval_rhs`, `LuFactorization`, `map_ordered`).
3. 유형: 일반 opt-in 방법(Peer-W control).
4. 방법 사실: CERT 2.1~2.3 tableau, order 2, 두 stage 독립·공통 W, 자체 estimator/controller, S5 startup, Session의 history/rejection. dense output·event·mass는 지원 안 함.
5. 파생 정보 허용: ExactJacobianOrJvpFrozenAtStepStart.
6. 잔차: stage 대수 잔차를 테스트가 1e-12로 확인. 절단 오차나 전역 오차는 한정하지 않는다.
7. 오차 authority: T02 fair-ab 미사용, achieved-error 주장 없음.
8. charge: jacobian_builds(+jvp_calls), direct_factorizations, rhs_calls, linear_solves, direct_solve_calls. 실패 시도도 charge.
9. 허용 경로: ptsw2b.rs 신규, lib.rs 한 줄, docs. 나머지는 바이트 동일.
10. 중단 조건: 계약 테스트 실패 또는 보호 blob 변경.

## UNRESOLVED / NOT_RUN
σ 허용 도메인(`advance`는 유한 양수 비율이면 허용, controller clip만 [0.5,2]), L(90) sector 증명(source-reported), 실제 문제·matched-error·속도 벤치마크 NOT_RUN, t_end<t1·startup 구간 event·starter 비용 모델, clippy 기존 실패(base 재실행 없음), 전체 `--lib` NOT_RUN, vigilode 기본 라우팅 NOT_RUN, matrix-free/shared-Krylov 미구현(주장 없음).
