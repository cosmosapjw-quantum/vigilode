#!/usr/bin/env python3
"""REPORT_KO.md of node V22 from RESULTS.json (every number comes from that file).

usage: build_report.py
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))


def fmt(value, digits=3):
    return "-" if value is None else f"{value:.{digits}f}"


def main():
    with open(os.path.join(HERE, "RESULTS.json")) as f:
        r = json.load(f)
    a = r["stage_a_width_scan"]
    width = a["W"]
    conf = r["small_n_confirmation"]
    d4 = r["stage_b"]["D4_facade_speed"]
    tests = r["stage_b"]["tests"]
    out = []
    out.append("# V22: facade banded 경로의 커널 선택 (소유자 결정 OD-L1-9)\n")
    out.append("- 사전등록: `PREDECLARED.json`, 수정 조항 `AMENDMENT_1.json` (확인 셀 반복 실행). 수치는 전부 `RESULTS.json`에서 읽었다.")
    out.append("- 두 커널(Indexed, Slices)은 같은 부동소수점 연산을 같은 순서로 한다. 바뀌는 것은 속도와 경로 라벨뿐이다.\n")
    out.append("## 1. 결론\n")
    if width is None:
        out.append("- 판정 기준을 만족하는 폭 W가 정해지지 않았다. facade는 바꾸지 않았다.")
    else:
        out.append(f"- 폭 W = {width}. 아래 띠와 위 띠가 모두 {width} 이상이면 Slices, 아니면 지금처럼 Indexed.")
        out.append("- 한쪽 띠만 좁은 경우는 바꾸지 않았다(Indexed 유지).")
    if conf is not None and "holds" in conf:
        out.append(f"- small-n 확인({conf['problem']}): instructions {fmt(conf['instructions_ratio'])}, cycles {fmt(conf['cycles_ratio'])} ({conf['cycles_label']}). {'통과' if conf['holds'] else '불통과'}.")
    if d4.get("measurements"):
        m = next(e for e in d4["measurements"] if e["measurement"] == d4["deciding_measurement"])
        out.append(f"- facade 수준(D4, Allen-Cahn 2D 64): 직접 Indexed 호출 대비 instructions {fmt(m['instructions_ratio_facade_to_direct'])} (기준 {d4['thresholds']['instructions']}), cycles {fmt(m['cycles_ratio_facade_to_direct'])} ({m['cycles_label']}). 횟수와 상태 hash 동일: {m['equal_counts_and_state_hash']}.")
    out.append("")
    out.append("## 2. 폭 scan (Slices / Indexed, attempt당)\n")
    out.append(f"유효성(모든 실행에서 횟수, 연산 수, 상태 hash 동일): {a['validity_equal_counts_operations_and_state_hash_in_every_run']}. 측정 셀 {a['cells_measured']}개.\n")
    out.append("| 반띠폭 | Allen-Cahn strip: instr | cycles | 라벨 | Brusselator strip: instr | cycles | 라벨 | 자격 |")
    out.append("|---|---|---|---|---|---|---|---|")
    for w in sorted(a["by_half_band_width"], key=int):
        row = a["by_half_band_width"][w]
        cells = []
        for family in ("acstrip", "brstrip"):
            e = row.get(family)
            if e is None:
                cells += ["-", "-", "-"]
            else:
                cells += [fmt(e["instructions_ratio"]), fmt(e["cycles_ratio"]),
                          "권위" if e["cycles_label"] == "authoritative" else "진단"]
        out.append(f"| {w} | " + " | ".join(cells) + f" | {'예' if a['width_qualifies'][w] else '아니오'} |")
    out.append("")
    out.append("자격: 그 폭에서 측정한 모든 계열이 instructions와 cycles 모두 0.95 이하이고 cycles가 권위 측정일 때. W는 그 폭 이상 전부가 자격을 갖는 가장 작은 폭.")
    if a["rises_above_the_trend_bound"]:
        out.append(f"\n추세 점검에서 상승 발견: {json.dumps(a['rises_above_the_trend_bound'], ensure_ascii=False)}")
    out.append("")
    out.append("## 3. 구현 검증\n")
    for key, title in (("D1_identity_of_the_twins", "D1 커널 인자를 받는 진입점 = 기존 진입점 (bit 단위)"),
                       ("D2_facade_bits_and_labels", "D2 facade 기록: 라벨 두 필드만 변경"),
                       ("D3_no_change_elsewhere", "D3 다른 곳 불변 (golden, sentinel, suite, lint)")):
        t = tests.get(key)
        out.append(f"### {title}\n")
        if t is None:
            out.append("- 로그 없음 (미실행).\n")
            continue
        out.append(f"- 로그: `{t['file']}`")
        for line in t["lines"][:40]:
            out.append(f"  - `{line[:230]}`")
        out.append("")
    out.append("## 4. 바뀐 것과 안 바뀐 것\n")
    out.append("- 바뀐 것: `Rodas5p::solve`의 banded 경로 네 개(끝점/dense output, scalar/vector atol)가 넓은 띠에서 Slices를 쓴다. 그때 경로 기록의 진입점 이름과 driver 식별자가 실제로 실행된 것으로 바뀐다(`..._with_kernel`, `-slices`).")
    out.append("- 안 바뀐 것: 결과 bit, 횟수, counter. 좁은 띠의 경로와 라벨. 커널 자체. 기존 진입점과 그 호출자(CLI, 예제, 벤치, `BandedStageLu`).")
    out.append("- 다루지 않은 것: dense Jacobian에 띠를 선언해 router(규칙 1)로 가는 경로는 Indexed 그대로다. 띠 2 전용 fixed-width 커널은 쓰지 않았다.")
    out.append("")
    if r["exploratory_series_not_used_for_the_decision"]:
        out.append("## 5. 탐색 측정 (판정에 쓰지 않음)\n")
        out.append("host가 바쁠 때 잰 값이다. instructions는 결정적이라 참고가 되고, cycles는 진단용이다.\n")
        out.append("| 계열 | 반띠폭 | instr | cycles |")
        out.append("|---|---|---|---|")
        for e in sorted(r["exploratory_series_not_used_for_the_decision"], key=lambda e: (e["family"], e["half_band_width"])):
            out.append(f"| {e['family']} | {e['half_band_width']} | {fmt(e['instructions_ratio'])} | {fmt(e['cycles_ratio'])} |")
        out.append("")
    with open(os.path.join(HERE, "REPORT_KO.md"), "w") as f:
        f.write("\n".join(out) + "\n")
    print("REPORT_KO.md written")


if __name__ == "__main__":
    main()
