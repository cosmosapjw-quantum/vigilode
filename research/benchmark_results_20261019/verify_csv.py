#!/usr/bin/env python3
"""Check the two archived VigilODE result CSVs, without re-running solvers."""
import csv
import hashlib
import json
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent
EXPECTED = {
    "run1/endpoint_results.csv": (154, "8a78b5a9c9b9e37dcc599efc7a348457437f606399d2bf236759b7e02d14909c"),
    "run2/ct_results_2700.csv": (2700, "03eda7909f39c467d1ddce7990dd2707b94a4dbb9bde5e036e21f1cf9435e2fa"),
}

def load(path):
    expected_n, expected_sha = EXPECTED[path]
    data = (ROOT / path).read_bytes()
    assert hashlib.sha256(data).hexdigest() == expected_sha, f"SHA mismatch: {path}"
    rows = list(csv.DictReader(data.decode("utf-8").splitlines()))
    assert len(rows) == expected_n, f"Row-count mismatch: {path}"
    return rows

def main():
    a = load("run1/endpoint_results.csv")
    assert Counter(row["status"] for row in a) == {"completed": 154}
    assert all(row["deterministic"] == "True" for row in a)
    max_wrms = max(float(row["endpoint_wrms_plus_empirical_spread_bound"]) for row in a)
    assert abs(max_wrms - 0.9917799399710141) < 1e-12
    b = load("run2/ct_results_2700.csv")
    status = Counter(row["status"] for row in b)
    assert status == {"ok": 2639, "refused": 54, "solver_failure": 7}
    successful = [row for row in b if row["status"] == "ok"]
    thresholds = {str(t): sum(float(row["R_up"]) <= t for row in successful) for t in (1, 10, 100)}
    assert thresholds == {"1": 2329, "10": 2580, "100": 2626}
    pi = [row for row in successful if row["arm"] == "fast-ctl-pi"]
    assert len(pi) == 144 and all(float(row["R_up"]) <= 1 for row in pi)
    print(json.dumps({"result": "PASS_SCOPED_CSV_INTEGRITY", "run1_rows": len(a),
                      "run1_max_endpoint_wrms_plus_spread": max_wrms,
                      "run2_rows": len(b), "run2_status": dict(status),
                      "run2_success_R_up_threshold_counts": thresholds,
                      "run2_fast_ctl_pi_R_up_le_1": len(pi)}, indent=2, sort_keys=True))

if __name__ == "__main__":
    main()
