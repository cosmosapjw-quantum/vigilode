#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REQUIRED = [
    "PROJECT_INSTRUCTIONS.md",
    "state/RESEARCH_STATE.md",
    "state/EVIDENCE_LEDGER.md",
    "state/HYPOTHESIS_GRAPH.md",
    "state/DECISION_LOG.md",
    "state/NEGATIVE_RESULTS.md",
    "state/CLOSEOUT.md",
    "prompts/00_integrated_work_run.md",
]


def main() -> int:
    errors: list[str] = []
    for rel in REQUIRED:
        path = ROOT / rel
        if not path.is_file() or not path.read_text(encoding="utf-8").strip():
            errors.append(f"missing or empty: {rel}")

    for skill_md in ROOT.glob(".agents/skills/*/SKILL.md"):
        text = skill_md.read_text(encoding="utf-8")
        if not re.search(r"^---\s*\nname:\s*\S+\s*\ndescription:\s*.+?\n---", text, re.S):
            errors.append(f"invalid skill frontmatter: {skill_md.relative_to(ROOT)}")

    if errors:
        print("Workspace validation failed:", file=sys.stderr)
        for err in errors:
            print(f"- {err}", file=sys.stderr)
        return 1

    print("Research harness validation passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
