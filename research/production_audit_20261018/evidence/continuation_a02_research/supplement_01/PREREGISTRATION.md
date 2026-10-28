# A02-CONTRACT-SUPPLEMENT-01: 순수 실행 전 사전등록

상태는 HOLD_PENDING_INDEPENDENT_SUPPLEMENT_REVIEW이며 `implementation_approved=false`다. 이 파일에는 수치 실행 결과를 추가하지 않는다. 이전 D1–D5 READY/C1·C2 HOLD 판정은 PRIOR_REVIEW_HANDOFF.json의 parent handoff로 보존한다. 독립 보완 검토가 계약을 채택한 다음에만 구현한다.

Source HEAD: `7e2873887ed6038b47b667c976435810851c5dba`, tree `9275e35d3d5afe1ae4e40e1c0d9aca1034994ca6`, branch `audit/production-readiness-20261018`.

동결 입력:

- `FROZEN_ACCEPTANCE_INPUTS.json`: `5a867fc2ee80db7afe29834681f50819305d59cb37d9b802d48223a14a31837d`
- `SOURCE_BINDINGS.json`: `1170fad132e157c3a6c8aae085d3abe1791700e41ada31dbda21f389f25b0639`
- `PRIOR_REVIEW_HANDOFF.json`: `ffbb97a899f7cec88291b7d9841b9d8655f59d0be577d892142f1adfcb4c09a4`
- `../../../A02_VERIFIED_TRAJECTORY_CONTRACT.md`: `566130e113b172a7352072dfc0dc71fd92960e6b24ae72e2a4684cde0d048e78`

C1: AT01–AT12별 model/config/grid/uncertainty/oracle/mutations/schedule/limits/tolerance source를 manifest에 동결한다. T_FIXED=1e-5/1e-5, T_INHERITED=1e-4/1e-4, T_REJECTION=2e-4/2e-4를 변경하지 않는다. A01 local unequal oracle는 물리 tolerance를 정의하지 않았다. AT02의 global trajectory gate에는 별도 fixed-grid source의 변경 없는 T_FIXED를 이 계약에서 명시적으로 적용한다. 이는 기존 local 물리 tolerance의 재사용이라는 주장이 아니다.

C2: window별 시작 t,c,r,e,E_global,E_center, static model/curve/config/grid identity, physical end/exact span/H와 generation을 immutable로 보존한다. Global query는 그 seed를 사용하며 live state를 historical seed로 대체하지 않는다. epoch0에서 P0/Q0 준비, P0 commit, window0 내부1/32 출력 보존, Q0 stale refusal, 새 P1 commit, epoch2 뒤 window0 재query를 실행한다. 이미 사용한 P0 replay도 거절한다.

증명 보존: §2–6의 SHA-256 `ecc8841db6ad2b2f40c7245102372e31e31120e38ecdb68b91ed0cfa4ddcdcb5`는 이전과 동일하다. AT05는 finite binary64를 non-dyadic이라고 부르지 않는다. frozen exact rational offset witness는 manifest에 기록했다.

미래 실행: 코드·사전등록 checkpoint를 원격 게시해 EXEC_SHA를 확정한 다음, 해당 source와 입력 hash 및 source-derived counter tally를 실행 입력으로 고정한다. Targeted binary 1800초, unequal Fraction oracle 600초, analytic oracle 120초, 각 fixture max_steps1000. Solver order/degree/sweeps/tolerance/oracle order는 동결값을 사용한다. 원시 첫 실패를 별도 log로 보존하고 수치·입력·환경·identity 실패를 구별한다. Allocation clone 양성 대조군 검출 전 zero-allocation PASS를 부여하지 않는다. 미실행과 typed refusal는 numerical inclusion으로 집계하지 않는다. 역사적 L-0124를 수정하지 않는다.

현재 허가 범위는 계약 보완과 metadata readback이다. Rust 코드, 테스트, Cargo 그래프, shared DAG, ledger, build·solver 실행은 이 단위에 포함하지 않는다. 다음 단계는 이 보완의 독립 read-only 검토다.
