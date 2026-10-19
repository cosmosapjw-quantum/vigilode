#!/usr/bin/env python3
"""Parity of the Rust triage problems with the NumPy models used for the references: PARITY.json.

For every CORPUS problem (or `--problem`), `rodas5p triage-parity --problem P` is expected to print

    {"schema": "vigilode-triage-parity-v1", "problem": P, "dimension": n, "y0_hex": [...] (optional),
     "samples": [{"t_hex": h, "y_hex": [...], "f_hex": [...], "jacobian_hex": [[row 0], ..., [row n-1]]}]}

The NumPy right-hand side and (densified) Jacobian of tools/triage/triage_models.py are evaluated at each
sample (t, y), and every entry is compared as

    rel = |rust - numpy| / max(|rust|, tiny),   tiny = smallest positive normal binary64,

so an entry that is exactly zero in Rust must be exactly zero in NumPy. A problem passes iff every rel <= limit
(and `y0_hex`, when printed, equals the model's initial state bit for bit). The limit is THRESHOLDS
`parity.max_relative_error` when that key exists, else PARITY_LIMIT of tools/stiff_benchmark_scipy.py
(1e-13); its source is recorded. Problems without a NumPy model are `not_checked` with reason
`no_numpy_model` (reported, not failed). Exit 1 when any problem fails or errors.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import triage_common as tc  # noqa: E402
import triage_models as tm  # noqa: E402

SCHEMA = "vigilode-triage-parity-report-v1"
TINY = sys.float_info.min


def rel_errors(rust: list[float], numpy_values) -> float:
    worst = 0.0
    for r, n in zip(rust, numpy_values):
        n = float(n)
        err = abs(r - n) / max(abs(r), TINY)
        if err != err:  # NaN on either side is a failure, never a pass
            return float("inf")
        worst = max(worst, err)
    return worst


def check_problem(binary: str, problem: dict, limit: float) -> dict:
    import numpy as np

    pid = problem["id"]
    out = {"problem": pid, "status": None, "reason": None, "samples": 0, "rhs_max_rel": None,
           "jacobian_max_rel": None, "y0_match": None, "model_source": None}
    model = tm.model_for(problem)
    if model is None:
        out.update(status="not_checked", reason="no_numpy_model")
        return out
    out["model_source"] = model.source
    proc = subprocess.run([binary, "triage-parity", "--problem", pid], capture_output=True, text=True)
    if proc.returncode != 0:
        out.update(status="error", reason=f"triage-parity: {tc.describe_exit(proc.returncode)}: {tc.tail(proc.stderr, 400)}")
        return out
    doc = tc.parse_json_output(proc.stdout)
    if doc is None or doc.get("schema") != tc.PARITY_INPUT_SCHEMA or doc.get("problem") != pid:
        out.update(status="error", reason="triage-parity printed no matching vigilode-triage-parity-v1 document")
        return out
    n = model.dimension
    try:
        if doc.get("y0_hex") is not None:
            y0 = [tc.unhex64(v) for v in doc["y0_hex"]]
            out["y0_match"] = len(y0) == n and all(tc.hex64(a) == tc.hex64(b) for a, b in zip(y0, model.y0))
        worst_f = worst_j = 0.0
        samples = doc.get("samples") or []
        for s in samples:
            t = tc.unhex64(s["t_hex"])
            y = np.asarray([tc.unhex64(v) for v in s["y_hex"]], dtype=np.float64)
            f = [tc.unhex64(v) for v in s["f_hex"]]
            jac = tc.hex_matrix(s["jacobian_hex"])
            if len(y) != n or len(f) != n or len(jac) != n or any(len(r) != n for r in jac):
                raise tc.TriageError(f"sample shape differs from dimension {n}")
            worst_f = max(worst_f, rel_errors(f, model.rhs(t, y)))
            dense = np.asarray(model.dense_jacobian(t, y), dtype=np.float64)
            worst_j = max(worst_j, rel_errors([v for r in jac for v in r], dense.reshape(-1)))
    except (tc.TriageError, KeyError, TypeError) as exc:
        out.update(status="error", reason=f"malformed parity document: {exc}")
        return out
    if not samples:
        out.update(status="error", reason="no samples")
        return out
    out.update(samples=len(samples), rhs_max_rel=worst_f if worst_f != float("inf") else None,
               jacobian_max_rel=worst_j if worst_j != float("inf") else None)
    failed = []
    if not worst_f <= limit:
        failed.append(f"rhs max rel {worst_f!r} > {limit!r}")
    if not worst_j <= limit:
        failed.append(f"jacobian max rel {worst_j!r} > {limit!r}")
    if out["y0_match"] is False:
        failed.append("y0 differs from the NumPy model's initial state")
    out.update(status="fail" if failed else "pass", reason="; ".join(failed) or None)
    return out


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--binary", required=True, help="rodas5p binary with `triage-parity`")
    parser.add_argument("--corpus", required=True, type=Path)
    parser.add_argument("--thresholds", type=Path, default=tc.DEFAULT_NODE / "THRESHOLDS.json")
    parser.add_argument("--output", required=True, type=Path, help="PARITY.json")
    parser.add_argument("--problem", action="append")
    parser.add_argument("--force", action="store_true")
    args = parser.parse_args(argv)
    tc.refuse_existing(args.output, args.force)
    try:
        tm.require_numpy()
        corpus = tc.load_json(args.corpus)
        thresholds = tc.load_json(args.thresholds) if args.thresholds.exists() else {}
    except (tc.TriageError, tm.ModelsUnavailable) as exc:
        print("error:", exc, file=sys.stderr)
        return 2
    parity = thresholds.get("parity") if isinstance(thresholds, dict) else None
    if isinstance(parity, dict) and tc.finite(parity.get("max_relative_error")):
        limit, source = float(parity["max_relative_error"]), f"{args.thresholds.name}: parity.max_relative_error"
    else:
        limit, source = float(tm.scipy_tool().PARITY_LIMIT), "tools/stiff_benchmark_scipy.py: PARITY_LIMIT"
    results = []
    for pid, problem in sorted(tc.corpus_problems(corpus).items()):
        if args.problem is None or pid in args.problem:
            results.append(check_problem(args.binary, problem, limit))
            print(f"{pid}: {results[-1]['status']} {results[-1]['reason'] or ''}", file=sys.stderr)
    doc = {"schema": SCHEMA, "generator": "tools/triage/triage_parity.py", "limit": limit, "limit_source": source,
           "tiny": TINY, "runtime": tm.versions(), "binary_sha256": tc.sha256_file(Path(args.binary)),
           "corpus_sha256": tc.sha256_file(args.corpus), "problems": results}
    tc.write_text(args.output, tc.dumps(doc) + "\n", args.force)
    return 1 if any(r["status"] in ("fail", "error") for r in results) else 0


if __name__ == "__main__":
    sys.exit(main())
