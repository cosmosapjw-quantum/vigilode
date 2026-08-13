#!/usr/bin/env python3
from __future__ import annotations
import hashlib, json, sys
from pathlib import Path
from typing import Any

def project(value: Any) -> Any:
    if isinstance(value, dict):
        out = {}
        for key in sorted(value):
            if "wall" in key.lower() or key == "failure":
                continue
            out[key] = project(value[key])
        return out
    if isinstance(value, list):
        return [project(item) for item in value]
    return value

def digest(value: Any) -> str:
    payload = json.dumps(
        project(value), sort_keys=True, separators=(",", ":"), allow_nan=False
    ).encode()
    return hashlib.sha256(payload).hexdigest()

def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit("usage: verify_g3_scientific_identity.py LEFT RIGHT")
    left = json.loads(Path(sys.argv[1]).read_text())
    right = json.loads(Path(sys.argv[2]).read_text())
    dl, dr = digest(left), digest(right)
    print(json.dumps({"left": dl, "right": dr, "scientific_equal": dl == dr}, indent=2))
    if dl != dr:
        raise SystemExit(1)

if __name__ == "__main__":
    main()
