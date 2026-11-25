#!/usr/bin/env python3
"""The loop checker of research/production_loop_20261116/check_checkpoint.py, run without stopping at its
first complaint: its `require` collects the messages instead of raising. Prints every complaint as one
V31_CHECKER_JSON line. Node V31 uses it to show that the only complaints are the pinned implementation
files the node is allowed to change. The checker file itself is read, not modified. (A copy of the
wrapper of node V29, research/production_loop_20261116/assembly_default/checker_all.py, with the tag of
this node.)

    python3 -I research/production_loop_20261116/assembly_wide_band/checker_all.py
"""
import json
from pathlib import Path

CHECKER = Path(__file__).resolve().parent.parent / "check_checkpoint.py"
ORIGINAL = "def require(condition, message):\n    if not condition:\n        raise ValueError(message)"
COLLECTING = "COMPLAINTS = []\ndef require(condition, message):\n    if not condition:\n        COMPLAINTS.append(message)"


def main():
    source = CHECKER.read_text()
    if ORIGINAL not in source:
        raise SystemExit("the checker's require() is not the expected one; nothing run")
    namespace = {"__file__": str(CHECKER), "__name__": "checker_all"}
    exec(compile(source.replace(ORIGINAL, COLLECTING), str(CHECKER), "exec"), namespace)
    namespace["main"]()
    complaints = namespace["COMPLAINTS"]
    print("V31_CHECKER_JSON " + json.dumps({"complaints": complaints, "count": len(complaints)}))


if __name__ == "__main__":
    main()
