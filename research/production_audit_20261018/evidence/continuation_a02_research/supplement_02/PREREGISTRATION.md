# A02-CONTRACT-SUPPLEMENT-02 순수 실행 전 사전등록

상태는 `READY_FOR_FRESH_DELTA_REVIEW`, `implementation_approved=false`다.
현재 단위는 parent가 전달한 AT08 admission-before-multiplication finding의
계약 보완과 DC1/AT06 선택 동결이다. 기존 supplement_01와 D1–D5 증명,
물리 tolerance, reference 차수와 자원 제한을 보존한다.

계약은 `AMENDMENT_KO.md`, 입력은 `FROZEN_ACCEPTANCE_INPUTS_V2.json`,
기존/신규 input 해시는 `INPUT_HASHES.json`, source는 `SOURCE_IDENTITY.json`과
`SOURCE_BINDINGS.json`, 미래 구현 권한을 보류한 판정은 `DECISION.json`이다.
이 문서에는 향후 수치 결과를 붙이지 않는다.

이번 단위에는 Rust 코드·테스트·Cargo graph·shared DAG·ledger 수정,
build·solver·reference 실행이 없다. metadata 생성·문서 readback·바이트
identity 확인만 수행한다. 수식의 exact integer algebra는 문서의 직접 유도다.
metadata 명령의 성공은 수치 검증이나 scientific admission으로 쓰지 않는다.

다음 단계는 새 v2 입력과 제한 overlay의 fresh read-only independent review다.
READY 후 구현과 source/preregistration checkpoint를 원격 게시하여 EXEC_SHA를
확정하고, 해당 source/input hashes와 source-derived work tally를 동결한 뒤
원래 AT01–AT12만 실행한다. Targeted test 1800초, unequal Fraction oracle
600초, analytic oracle 120초, fixture max_steps1000, solver 및 oracle 차수,
tolerance는 supplement_01 값 그대로다. 이전 raw 실패와 입력은 덮어쓰지 않는다.
