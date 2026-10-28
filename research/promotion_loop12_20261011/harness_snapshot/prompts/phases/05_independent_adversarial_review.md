# Phase 5 — Adversarial Review

고정된 후보·claim·raw evidence를 실제 분리된 reviewer에게 주고 evidence, logic, physical consistency, novelty/relabeling, assumptions, testability, robustness, alternatives, tractability를 집중 검토한다. reviewer 독립성의 범위를 기록한다. 같은 실행자가 검토하면 `OWNER_SELF_REVIEW`다.

원래 동기와 후보의 가장 강한 형태를 대상으로 한다. 우아함과 증거, fitting과 설명, 계산 가능성과 물리적 타당성을 구분한다. 대안을 strawman으로 만들지 않는다.

결과 역할: `DEFENDED / TRADEOFF / INFORMATIVE_FAILURE / REJECTED_SHORTCUT / NEEDS_MORE_EVIDENCE`. 발견 항목에는 구체 근거, 영향받는 claim, 필요한 최소 보완을 붙인다. 리뷰의 리뷰를 자동 생성하지 않는다. owner는 기존 범위에서 수정과 관련 확인을 완료한다.
