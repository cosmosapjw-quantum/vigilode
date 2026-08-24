from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def first_position(text: str, patterns: tuple[str, ...]) -> int:
    positions = [text.find(pattern) for pattern in patterns if text.find(pattern) >= 0]
    require(bool(positions), f"none of the required patterns found: {patterns}")
    return min(positions)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--r5-dir", required=True)
    root = Path(parser.parse_args().r5_dir).resolve()
    script_path = root / "publish_pm4_task1.sh"
    state_path = root / "STATE.json"
    helper_path = root / "validate_vendor_source.py"
    require(script_path.is_file(), f"missing {script_path}")
    require(state_path.is_file(), f"missing {state_path}")
    require(helper_path.is_file(), f"missing {helper_path}")

    script = script_path.read_text(encoding="utf-8")
    state_text = state_path.read_text(encoding="utf-8")
    state = json.loads(state_text)

    for literal in ('== "262"', "expected 262", 'offline_vendor_packages": 262', "exact 262-package"):
        require(literal not in script, f"stale exact-count gate remains in script: {literal}")
        require(literal not in state_text, f"stale exact-count claim remains in state: {literal}")

    require("validate_vendor_source.py" in script, "publication script does not call the helper")
    require("cargo metadata" in script and "--frozen" in script and "--format-version 1" in script,
            "frozen Cargo metadata gate missing")
    require("--force" not in script and "force-with-lease" not in script,
            "force push token present")
    require(not re.search(r"\bgh\s+pr\s+merge\b", script), "merge command present")

    metadata_pos = first_position(script, ("cargo metadata",))
    test_pos = first_position(script, ("cargo test -p rodas5p-integrators --test v38d_performance_probe_contracts",))
    push_pos = first_position(script, ("git push --porcelain", "git push "))
    require(metadata_pos < test_pos < push_pos, "required ordering metadata < tests < push is not satisfied")

    require(state.get("schema") == "vigilode-pm4-task1-publication-kit-r5",
            "STATE schema must identify R5")
    require(state.get("source_patch_unchanged") is True,
            "STATE must seal unchanged Task-1 patch")
    require(state.get("exact_vendor_package_count_enforced") is False,
            "STATE must reject exact vendor-count authority")

    print("PASS: no exact vendor-count gate")
    print("PASS: frozen Cargo metadata precedes tests and push")
    print("PASS: force/merge tokens absent")
    print("PASS: R5 state and unchanged source patch declared")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
