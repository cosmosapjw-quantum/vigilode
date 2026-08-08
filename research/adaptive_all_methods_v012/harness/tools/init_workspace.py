#!/usr/bin/env python3
from __future__ import annotations

import argparse
import shutil
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TEMPLATES = ROOT / "templates" / "state"
STATE = ROOT / "state"


def main() -> int:
    parser = argparse.ArgumentParser(description="Initialize research state files.")
    parser.add_argument("--project", default="Untitled Research Project")
    parser.add_argument("--force", action="store_true", help="Overwrite existing state files")
    args = parser.parse_args()

    STATE.mkdir(parents=True, exist_ok=True)
    for src in sorted(TEMPLATES.glob("*.md")):
        dst = STATE / src.name
        if dst.exists() and not args.force:
            print(f"keep  {dst.relative_to(ROOT)}")
            continue
        text = src.read_text(encoding="utf-8")
        text = text.replace("{{PROJECT_NAME}}", args.project)
        text = text.replace("{{DATE}}", date.today().isoformat())
        dst.write_text(text, encoding="utf-8")
        print(f"write {dst.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
