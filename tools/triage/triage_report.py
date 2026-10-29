#!/usr/bin/env python3
"""Generate REPORT_KO.md of the component triage from RESULTS.json and CLASSIFICATION.json only.

Both inputs are validated first (tools/evidence_schema_v2.py::validate_triage; an INVALID input writes nothing,
exit 2). The report embeds the SHA-256 of both inputs and of every recorded input digest, a label summary, one
table per family, the SPECIALIZE predicates with their holdout verdicts, the criteria status per component, the
rule path of every decision, the insufficient-evidence reasons and the excluded-comparison reasons by kind.
Nothing in the report is computed from any other source, and the report is never edited by hand.

`--check` (PA-06 guard) regenerates the report in memory and compares it byte for byte with the existing file:
exit 0 when identical, 1 with a unified diff otherwise. Without `--check` an existing output is not overwritten
unless `--force`.
"""

from __future__ import annotations

import argparse
import difflib
import sys
from collections import Counter, defaultdict
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import triage_common as tc  # noqa: E402

es = tc.evidence
LABELS = ("PROMOTE", "SPECIALIZE", "OPTIMIZE", "PRUNE")
OUTCOMES = ("GAIN", "NEUTRAL", "LOSS", "capability_gain", "capability_loss", "excluded")


def cell(value) -> str:
    if value is None:
        return "—"
    if isinstance(value, bool):
        return "예" if value else "아니오"
    if isinstance(value, float):
        return f"{value:.4g}"
    return str(value).replace("|", "\\|").replace("\n", " ")


def table(header: list[str], rows: list[list]) -> list[str]:
    out = ["| " + " | ".join(header) + " |", "|" + "|".join("---" for _ in header) + "|"]
    out += ["| " + " | ".join(cell(v) for v in row) + " |" for row in rows]
    return out


def reason_kind(reason: str | None) -> str:
    if not reason:
        return "(none)"
    return reason.split(":", 1)[0].strip()


def render(results: dict, classification: dict, results_sha: str, classification_sha: str) -> str:
    comps = sorted(classification["components"], key=lambda e: (e["family"], e["slot"], e["id"]))
    lines = ["# 컴포넌트 트리아지 보고서 (자동 생성)", "",
             "> 이 문서는 `tools/triage/triage_report.py`가 `RESULTS.json`과 `CLASSIFICATION.json`만으로 생성한다. "
             "손으로 편집하지 않는다 (PA-06). `--check`가 재생성 결과와 바이트 단위로 비교한다.", "",
             f"- 캠페인: `{classification['campaign']}`",
             f"- RESULTS.json SHA-256: `{results_sha}`",
             f"- CLASSIFICATION.json SHA-256: `{classification_sha}`",
             f"- 생성기: `{results['generator']}`", "", "## 입력", ""]
    rows = []
    for role in sorted(results["inputs"]):
        value = results["inputs"][role]
        for d in (value if isinstance(value, list) else [value]):
            rows.append([role, None if d is None else d["name"], None if d is None else f"`{d['sha256']}`"])
    lines += table(["역할", "파일", "SHA-256"], rows)
    summary = classification["summary"]
    lines += ["", "## 요약", ""]
    lines += table(["라벨", "개수"], [[label, summary[label]] for label in LABELS]
                   + [["잠정 (provisional)", summary["provisional"]], ["전체", summary["components"]]])
    lines += ["", "판정 규칙과 임계값은 `RESULTS.json`의 `thresholds`(THRESHOLDS.json 사본)를 따른다. "
              "비율 = cost(컴포넌트 arm) / cost(상대 arm), 비용 = PROFILE `ir_warm` (h0 = 1 셀).", ""]
    by_family = defaultdict(list)
    for e in comps:
        by_family[e["family"]].append(e)
    lines += ["## 패밀리별 분류", ""]
    for family in sorted(by_family):
        lines += [f"### {family}", ""]
        rows = []
        for e in by_family[family]:
            o = e["evidence"]["outcomes"]
            rows.append([f"`{e['id']}`", e["slot"], e["label"] + (" (relocate)" if e["prune_kind"] else ""),
                         e["provisional"], e["loc"], o["GAIN"], o["NEUTRAL"], o["LOSS"], o["capability_gain"],
                         o["capability_loss"], o["excluded"], e["evidence"]["max_ratio"],
                         ", ".join(e["flags"]) or None])
        lines += table(["컴포넌트", "슬롯", "라벨", "잠정", "LOC", "GAIN", "NEUTRAL", "LOSS", "역량 우위",
                        "역량 손실", "제외", "최대 비율", "플래그"], rows)
        lines.append("")
    special = [e for e in comps if e["predicate"] is not None]
    lines += ["## SPECIALIZE 술어", ""]
    if special:
        rows = []
        for e in special:
            p = e["predicate"]
            if p.get("kind") == "applicability":  # API requirement of the arms (SPECIALIZE_APPLICABILITY)
                arms = ", ".join(r["arm"] for r in p["rule"])
                wins = f", 승 {p['win_cells']}/{p['min_positives']}" if "win_cells" in p else ""  # Amendment 8
                rows.append([f"`{e['id']}`", "applicability", "API", f"{arms} (coverage {p['coverage']:.3g}{wins})",
                             None, None, "deterministic", None, None])
                continue
            value = p["values"] if p["op"] == "in" else p["threshold"]
            h = p["holdout"]
            rows.append([f"`{e['id']}`", p["feature"], p["op"], value, p["calibration"]["wins_covered"],
                         p["calibration"]["wins"], h["verdict"], h["n_positives"], h["holdout_wins_recommended"]])
        lines += table(["컴포넌트", "특징", "연산", "임계값/값", "보정 승 (포함)", "보정 승 (전체)", "홀드아웃 판정",
                        "홀드아웃 양성", "홀드아웃 승 포함"], rows)
    else:
        lines.append("SPECIALIZE 라벨 없음.")
    lines += ["", "## 기준 상태", ""]
    rows = []
    for e in comps:
        crit = e["criteria"]
        rows.append([f"`{e['id']}`"] + [crit[k]["status"] or "null" for k in
                                         ("c1", "c2", "c3", "allocation", "promote", "specialize", "optimize",
                                          "timing")])
    lines += table(["컴포넌트", "C1", "C2", "C3", "할당", "PROMOTE", "SPECIALIZE", "OPTIMIZE", "타이밍"], rows)
    lines += ["", "## 규칙 경로", ""]
    for e in comps:
        lines.append(f"### `{e['id']}` → {e['label']}{' (잠정)' if e['provisional'] else ''}")
        lines.append("")
        lines += [f"{i}. `{step['rule']}` ({step['kind']}): {cell(step['detail'])}"
                  for i, step in enumerate(e["rule_path"], 1)]
        if e["blocked_by"]:
            lines.append(f"- blocked_by: {', '.join(e['blocked_by'])}")
        lines.append("")
    lines += ["## 증거 부족 (insufficient_evidence)", ""]
    missing = [(e["id"], r) for e in comps for r in e["insufficient_evidence"]]
    lines += [f"- `{cid}`: {cell(r)}" for cid, r in missing] or ["없음."]
    gaps = [g for g in results["external_gaps"] if g["gap"] is not None]
    lines += ["", "## 외부 피어 counted-work 격차 (OPTIMIZE b)", ""]
    rows = []
    for e in comps:
        eg = e["criteria"]["optimize"]["external_gap"]
        if eg["peers"]:
            rows.append([f"`{e['id']}`", ", ".join(eg["peers"]), eg["matched_cells"], eg["cells"], eg["median_gap"],
                         eg["reason"]])
    lines += table(["컴포넌트", "외부 피어", "일치 셀", "전체 셀", "중앙 격차", "사유"], rows) if rows \
        else ["external_peers가 지정된 컴포넌트 없음."]
    lines += ["", f"일치한 외부 격차 셀 {len(gaps)}개 / 전체 {len(results['external_gaps'])}개.", ""]
    lines += ["## 제외된 비교의 사유", ""]
    kinds = Counter(reason_kind(r["reason"]) for r in results["comparisons"] if r["outcome"] == "excluded")
    lines += table(["사유", "개수"], [[k, kinds[k]] for k in sorted(kinds)]) if kinds else ["없음."]
    totals = Counter(r["outcome"] for r in results["comparisons"])
    lines += ["", "## 비교 집계", ""]
    lines += table(["결과", "개수"], [[o, totals.get(o, 0)] for o in OUTCOMES])
    lines += ["", f"행 {len(results['rows'])}개, 그룹 {len(results['groups'])}개, 비교 {len(results['comparisons'])}개, "
              f"다른 캠페인 행 {results['ignored_rows_other_campaigns']}개 무시.", ""]
    return "\n".join(lines)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--results", required=True, type=Path)
    parser.add_argument("--classification", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path, help="REPORT_KO.md")
    parser.add_argument("--check", action="store_true", help="compare a regenerated report with --output")
    parser.add_argument("--force", action="store_true")
    args = parser.parse_args(argv)
    try:
        ev = es.validate_triage(args.results, args.classification)
    except es.ValidationError as exc:
        print(f"INVALID inputs ({len(exc.reasons)} reason(s)):", file=sys.stderr)
        for reason in exc.reasons[:30]:
            print("  ", reason, file=sys.stderr)
        return 2
    text = render(ev.docs["results"], ev.docs["classification"], ev.sha256["results"], ev.sha256["classification"])
    if args.check:
        if not args.output.exists():
            print(f"--check: {args.output} does not exist", file=sys.stderr)
            return 1
        current = args.output.read_text()
        if current == text:
            print(f"--check: {args.output} matches the regenerated report", file=sys.stderr)
            return 0
        sys.stdout.writelines(difflib.unified_diff(current.splitlines(True), text.splitlines(True),
                                                   str(args.output), "regenerated"))
        print(f"--check: {args.output} differs from the regenerated report", file=sys.stderr)
        return 1
    tc.write_text(args.output, text, args.force)
    print(f"wrote {args.output}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
