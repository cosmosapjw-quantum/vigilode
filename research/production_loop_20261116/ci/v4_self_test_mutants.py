"""Mutation check of the v4 self-test (node V33, OD-L1-29): each mutant breaks one mechanism of
tools/ci/select_tests.py; the self-test must fail on every mutant.

  python3 -I v4_self_test_mutants.py <repository root> <output log>

A mutant is written to <root>/tools/ci/_mutant/select_tests.py (removed after its run), so that its
self-test still reads the real workflows of the repository."""
import os, shutil, subprocess, sys
root, out = sys.argv[1], sys.argv[2]
src = os.path.join(root, "tools", "ci", "select_tests.py")
text = open(src, encoding="utf-8").read()
MUTANTS = {
    "decide ignores the units not run": ("    for item in not_run:\n        if not isinstance(item, dict):", "    for item in ():\n        if not isinstance(item, dict):"),
    "plan ignores run_last": ('later = [s["kind"] != "build" and bool(last.intersection(s["units"])) for s in steps]', 'later = [False for s in steps]'),
    "stream ignores the deadline": ('            if deadline is not None and clock() >= deadline:\n                stopped = True', '            if False:\n                stopped = True'),
    "run keeps the entries of selected targets": ('carried = {k: v for k, v in previous["targets"].items() if k not in keys}', 'carried = dict(previous["targets"])'),
    "write_json is not atomic": ('    fd, tmp = tempfile.mkstemp(dir=directory, prefix="." + os.path.basename(path) + ".", suffix=".tmp")', '    fd, tmp = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o644), path'),
    "baseline rejects incomplete outcomes": ('    if not outcome.get("complete") and "not_run" not in outcome:', '    if not outcome.get("complete"):'),
    "baseline drops run_last": ('run_last=deferred_units(outcome), environment', 'run_last=[], environment'),
    "outcome written only at the end": ('            outcome["running"] = None\n            save()\n    except', '            outcome["running"] = None\n    except'),
    "no outcome before a command": ('            save(with_map=False)\n', ''),
    "read_outcome accepts a malformed not_run": ('        if not isinstance(value, list) or len(unit_pairs(value)) != len(value):', '        if False:'),
    "load_map accepts malformed sets": ('            if not isinstance(values, list) or not all(isinstance(v, str) for v in values):', '            if False:'),
    "a stopped run exits 0": ('            outcome["stop"] = dict(stop, minutes=round((clock() - started) / 60.0, 2))\n            status = 1', '            outcome["stop"] = dict(stop, minutes=round((clock() - started) / 60.0, 2))'),
    "check_workflows accepts !cancelled() saves": ('if "uses: actions/upload-artifact@" in step and not re.search(r"^\\s*if: always\\(\\)\\s*$", step, re.M):', 'if False:'),
}
results = []
for name, (old, new) in MUTANTS.items():
    if text.count(old) != 1:
        results.append((name, f"PATTERN COUNT {text.count(old)}"))
        continue
    mutant_dir = os.path.join(root, "tools", "ci", "_mutant")
    os.makedirs(mutant_dir, exist_ok=True)
    path = os.path.join(mutant_dir, "select_tests.py")
    with open(path, "w", encoding="utf-8") as f:
        f.write(text.replace(old, new))
    try:
        r = subprocess.run([sys.executable, "-I", "-u", path, "self-test"], cwd=root, capture_output=True, text=True, timeout=120)
        verdict = "killed (self-test failed)" if r.returncode != 0 else "SURVIVED"
        tail = (r.stdout + r.stderr).strip().splitlines()[-1:] 
    except subprocess.TimeoutExpired:
        verdict, tail = "killed (self-test timed out after 120 s)", []
    finally:
        shutil.rmtree(mutant_dir, ignore_errors=True)
    results.append((name, verdict + (" | " + tail[0][:220] if tail else "")))
with open(out, "w", encoding="utf-8") as f:
    for name, verdict in results:
        line = f"{name}: {verdict}"
        print(line)
        f.write(line + "\n")
print(sum(1 for _, v in results if v.startswith("killed")), "of", len(results), "mutants killed")
