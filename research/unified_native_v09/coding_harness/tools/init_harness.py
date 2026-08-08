#!/usr/bin/env python3
from __future__ import annotations

import argparse
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FILES = {
    "RUN_STATE.md": """# RUN_STATE.md\n\nDATE: {date}\n\nCURRENT_LAYER: diagnose\n\nTASK:\n\nCURRENT_HYPOTHESIS:\n\nREPRODUCTION_OR_ACCEPTANCE:\n\nLIKELY_EDIT_LOCATIONS:\n\nBLOCKERS:\n\nLAST_VALIDATION:\n\nNEXT_MINIMAL_ACTION:\n""",
    "DECISION_LOG.md": "# DECISION_LOG.md\n",
    "FAILURE_LOG.md": "# FAILURE_LOG.md\n",
}


def main() -> int:
    parser = argparse.ArgumentParser(description="Initialize coding harness state files.")
    parser.add_argument("--force", action="store_true")
    args = parser.parse_args()

    for rel, template in FILES.items():
        path = ROOT / rel
        if path.exists() and path.stat().st_size > 0 and not args.force:
            print(f"keep  {rel}")
            continue
        path.write_text(template.format(date=date.today().isoformat()), encoding="utf-8")
        print(f"write {rel}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
