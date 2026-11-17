#!/usr/bin/env python3
"""Renders PORT_PLAN_KO.md (loop L1 node V12) from port_contracts.json and SOURCE_STATUS.json.

usage: python3 -I verify_sources.py && python3 -I render_plan.py
The table rows are read from the two JSON files; this script holds the fixed Korean text only.
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))

STATUS_KO = {
    "PORTED_AND_MEASURED": "이식됨·실측됨",
    "PORTED_NOT_MEASURED": "이식됨·판별 실험 없음",
    "PORTED_NOT_WIRED": "이식됨·미연결",
    "PORTED_BLOCKED": "이식됨·선행 조건 대기",
    "TEST_ONLY": "test 안에만 있음",
    "NOT_PORTED": "미이식",
    "DESIGN_ONLY": "설계만 있음",
    "SUPERSEDED": "head에 후속 구현 있음",
    "ARCHIVE": "archive",
}
ARRIVAL_KO = {
    "at_head": "head에 있음",
    "at_head_but_follow_up_commits_unmerged": "head에 있음 (후속 branch commit 미병합)",
    "partly_at_head": "일부만 head에 있음",
    "not_at_head": "head에 없음",
    "no_code_path_declared": "코드 경로 없음",
}


def cell(text):
    return str(text).replace("|", "\\|").replace("\n", " ")


def main():
    with open(os.path.join(HERE, "port_contracts.json"), encoding="utf-8") as f:
        contracts = json.load(f)
    with open(os.path.join(HERE, "SOURCE_STATUS.json"), encoding="utf-8") as f:
        status = json.load(f)
    arrival = {it["id"]: it for it in status["items"]}
    out = []
    w = out.append
    w("# 미이식 수학 이식 계획 (loop L1 node V12)")
    w("")
    w("이 문서는 `render_plan.py`가 `port_contracts.json`(계약)과 `SOURCE_STATUS.json`(git으로 확인한 출처·도착 상태, `verify_sources.py` 출력)에서 생성한다.")
    w("")
    w(f"기준 head: `{status['head'][:8]}`. 상태: {contracts['status']}")
    w("")
    w("## 1. 한눈에 보기")
    w("")
    counts = {}
    for it in contracts["items"]:
        counts[it["status_20261117"]] = counts.get(it["status_20261117"], 0) + 1
    w("| 상태 | 항목 수 | 뜻 |")
    w("|---|---|---|")
    for key, n in sorted(counts.items(), key=lambda kv: -kv[1]):
        w(f"| {STATUS_KO[key]} | {n} | {cell(contracts['status_vocabulary'][key])} |")
    w("")
    w("우선순위: P1 = 이번 loop의 node가 사용, P2 = 다음 loop, P3 = 선행 조건이 풀릴 때까지 보류.")
    w("")
    w("## 2. 항목별 계약")
    w("")
    w("| ID | P | 주제 | 기제 | 상태 | 도착 (git 확인) | 이번 loop 결과 | 다음 판별 실험 |")
    w("|---|---|---|---|---|---|---|---|")
    for it in contracts["items"]:
        arr = ARRIVAL_KO[arrival[it["id"]]["arrival"]]
        nxt = it["discriminator"] if it["next_discriminator"] == "as written" else it["next_discriminator"]
        w(f"| {it['id']} | {it['priority']} | {cell(it['theme'])} | {cell(it['name'])} | {STATUS_KO[it['status_20261117']]} | {arr} | {cell(it['result_this_loop'])} | {cell(nxt)} |")
    w("")
    w("## 3. 출처와 대상")
    w("")
    w("| ID | 출처 | 대상 | 지켜야 할 불변량 |")
    w("|---|---|---|---|")
    for it in contracts["items"]:
        w(f"| {it['id']} | {cell(it['source'])} | {cell(it['target'])} | {cell(it['invariant'])} |")
    w("")
    w("## 4. 후속 branch 확인 (612063db)")
    w("")
    w(contracts.get("lineage_check_20261117", ""))
    w("")
    for it in contracts["items"]:
        if "follow_up_612063db" in it:
            w(f"- {it['id']}: {it['follow_up_612063db']}")
    w("")
    w("## 5. 읽는 법")
    w("")
    w("- '이식됨'은 코드가 loop branch head에 있다는 뜻이고, 이득이 실측됐다는 뜻이 아니다. 이득 주장은 '다음 판별 실험'을 실행한 뒤에만 한다.")
    w("- 계약의 규칙: " + contracts["rule"])
    w("- 다시 만들기: `python3 -I research/production_loop_20261116/port_plan/verify_sources.py && python3 -I research/production_loop_20261116/port_plan/render_plan.py`.")
    w("")
    with open(os.path.join(HERE, "PORT_PLAN_KO.md"), "w", encoding="utf-8") as f:
        f.write("\n".join(out))
    print("PORT_PLAN_KO.md written,", len(out), "lines")


if __name__ == "__main__":
    main()
