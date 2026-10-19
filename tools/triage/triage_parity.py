#!/usr/bin/env python3
"""Parity of the Rust triage problems with the NumPy models used for the references: PARITY.json.

For every CORPUS problem (or `--problem`), `rodas5p triage-parity --problem P` prints (crates/rodas5p-cli/src/
triage.rs::parity_json)

    {"schema": "vigilode-triage-parity-v1", "problem", "dimension", "autonomous",
     "jacobian": "analytic" | "jvp", "jvp_direction_hex": [v], "mass_hex": null | [[row], ...],
     "samples": [{"t_hex", "y_hex", "f_hex" | null, "f_error": null | "message",
                  "jacobian": null | {"format": "dense", "rows_hex": [[..]]}
                                   | {"format": "coo" (or "triplets"), "nonzeros_hex": [[i, k, hex], ...]},
                  "jvp_hex": null | [J v]}]}

Samples are f(t, y) at y0 and a perturbation of it (stiff_benchmark::parity_states) at t0 and the span
midpoint; sample 0 is y0 at t0, so the model's initial state is compared with it bit for bit. Per sample:

* f is always compared. An `f_error` sample must also fail in the model (exception or non-finite value);
  a failure on one side only is a mismatch.
* The Jacobian (dense rows, or coo nonzeros with every other entry +0) is compared entrywise when present.
* Otherwise `jvp_hex` is compared with the model J v (the model JVP when it has one, else its Jacobian
  times v), v = `jvp_direction_hex`.
* `mass_hex` is compared with the model's constant mass matrix; a mass matrix on one side only is a mismatch.

Entry error: rel = |rust - numpy| / max(|rust|, tiny), tiny = smallest positive normal binary64, so an entry
that is exactly zero in Rust must be exactly zero in NumPy. A problem passes iff every rel <= limit. The limit
is THRESHOLDS `parity.max_relative_error` when present, else PARITY_LIMIT of tools/stiff_benchmark_scipy.py,
and its source is recorded. The worst entry of each problem (kind, sample, index, both values) is recorded,
and for a JVP also |J| |v| at that index (the rounding scale of the product). Problems without a NumPy model
are `not_checked` (`no_numpy_model`). Exit 1 when any problem fails or errors.
"""

from __future__ import annotations

import argparse
import math
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import triage_common as tc  # noqa: E402
import triage_models as tm  # noqa: E402

SCHEMA = "vigilode-triage-parity-report-v1"
TINY = sys.float_info.min


def decode(values) -> "np.ndarray":
    import numpy as np

    return np.asarray([tc.unhex64(v) for v in values], dtype=np.float64)


class Worst:
    """Largest relative entry error per kind, with the entry that produced it."""

    def __init__(self):
        self.by_kind: dict[str, float] = {}
        self.entry: dict | None = None

    def update(self, kind: str, sample: int, rust, model, scale=None) -> None:
        import numpy as np

        rust = np.asarray(rust, dtype=np.float64).reshape(-1)
        model = np.asarray(model, dtype=np.float64).reshape(-1)
        if rust.shape != model.shape:
            raise tc.TriageError(f"{kind}: {rust.size} Rust entries vs {model.size} NumPy entries")
        with np.errstate(invalid="ignore", over="ignore"):
            rel = np.abs(rust - model) / np.maximum(np.abs(rust), TINY)
        rel = np.where(np.isnan(rel), np.inf, rel)
        k = int(np.argmax(rel)) if rel.size else 0
        value = float(rel[k]) if rel.size else 0.0
        self.by_kind[kind] = max(self.by_kind.get(kind, 0.0), value)
        if self.entry is None or value > self.entry["rel"]:
            self.entry = {"kind": kind, "sample": sample, "index": k, "rust": fval(rust[k]), "numpy": fval(model[k]),
                          "rel": value, "rust_hex": tc.hex64(rust[k]), "numpy_hex": tc.hex64(model[k]),
                          "abs_j_abs_v": None if scale is None else fval(np.asarray(scale).reshape(-1)[k])}


def fval(x) -> float | None:
    x = float(x)
    return x if math.isfinite(x) else None


def model_f(model, t, y):
    import numpy as np

    try:
        out = np.asarray(model.rust_rhs(t, y), dtype=np.float64)
    except Exception as exc:  # noqa: BLE001 - any model failure is the model side of an f_error comparison
        return None, f"{type(exc).__name__}: {exc}"
    if not np.isfinite(out).all():
        return None, "non-finite model f"
    return out, None


def rust_jacobian(spec: dict, n: int):
    import numpy as np

    fmt = spec.get("format")
    if fmt == "dense":
        rows = spec["rows_hex"]
        if len(rows) != n or any(len(r) != n for r in rows):
            raise tc.TriageError(f"dense Jacobian is not {n} x {n}")
        return np.asarray([[tc.unhex64(v) for v in r] for r in rows], dtype=np.float64)
    if fmt in ("coo", "triplets"):
        dense = np.zeros((n, n))
        for entry in spec.get("nonzeros_hex", spec.get("triplets", [])):
            i, k, value = entry
            dense[int(i), int(k)] = tc.unhex64(value)
        return dense
    raise tc.TriageError(f"unknown Jacobian format {fmt!r}")


def check_problem(binary: str, problem: dict, limit: float) -> dict:
    import numpy as np

    pid = problem["id"]
    out = {"problem": pid, "status": None, "reason": None, "samples": 0, "compared": [], "rhs_max_rel": None,
           "jacobian_max_rel": None, "jvp_max_rel": None, "mass_max_rel": None, "f_error_samples": 0,
           "y0_match": None, "worst": None, "model_source": None, "numpy_model": problem.get("numpy_model")}
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
    failed: list[str] = []
    worst = Worst()
    try:
        if doc.get("dimension") != n:
            raise tc.TriageError(f"Rust dimension {doc.get('dimension')} != NumPy model dimension {n}")
        v = decode(doc["jvp_direction_hex"])
        if doc.get("mass_hex") is not None or model.mass is not None:
            if doc.get("mass_hex") is None or model.mass is None:
                failed.append("mass matrix on one side only")
            else:
                worst.update("mass", -1, [[tc.unhex64(x) for x in r] for r in doc["mass_hex"]], model.mass)
                out["compared"].append("mass")
        samples = doc.get("samples") or []
        for index, sample in enumerate(samples):
            t = tc.unhex64(sample["t_hex"])
            y = decode(sample["y_hex"])
            if len(y) != n:
                raise tc.TriageError(f"sample {index}: state length {len(y)} != {n}")
            if index == 0:
                out["y0_match"] = bool(len(model.y0) == n and all(tc.hex64(a) == tc.hex64(b)
                                                                  for a, b in zip(y, model.y0)))
            f_model, f_model_error = model_f(model, t, y)
            if sample.get("f_error") is not None or sample.get("f_hex") is None:
                out["f_error_samples"] += 1
                if f_model is not None:
                    failed.append(f"sample {index}: Rust f failed ({sample.get('f_error')}) but the model f succeeded")
                continue
            if f_model is None:
                failed.append(f"sample {index}: model f failed ({f_model_error}) but Rust f succeeded")
                continue
            worst.update("rhs", index, decode(sample["f_hex"]), f_model)
            if sample.get("jacobian") is not None:
                worst.update("jacobian", index, rust_jacobian(sample["jacobian"], n), model.rust_dense_jacobian(t, y))
            elif sample.get("jvp_hex") is not None:
                scale = None
                if model.jvp is None:
                    jac = (model.parity_jac or model.jac)(t, y)
                    scale = abs(jac) @ np.abs(v)
                worst.update("jvp", index, decode(sample["jvp_hex"]), model.rust_jvp(t, y, v), scale)
        out["samples"] = len(samples)
        if not samples:
            raise tc.TriageError("no samples")
    except (tc.TriageError, KeyError, TypeError, ValueError) as exc:
        out.update(status="error", reason=f"malformed parity document or model: {exc}")
        return out
    for kind, key in (("rhs", "rhs_max_rel"), ("jacobian", "jacobian_max_rel"), ("jvp", "jvp_max_rel"),
                      ("mass", "mass_max_rel")):
        if kind in worst.by_kind:
            value = worst.by_kind[kind]
            out[key] = value if math.isfinite(value) else None
            if kind not in out["compared"]:
                out["compared"].append(kind)
            if not value <= limit:
                failed.append(f"{kind} max rel {value!r} > {limit!r}")
    if worst.entry is not None:
        worst.entry["rel"] = fval(worst.entry["rel"])
    out["worst"] = worst.entry
    if out["y0_match"] is False:
        failed.append("sample 0 state differs from the NumPy model's initial state")
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
            r = results[-1]
            print(f"{pid}: {r['status']} rhs {r['rhs_max_rel']} jac {r['jacobian_max_rel']} jvp {r['jvp_max_rel']} "
                  f"{r['reason'] or ''}", file=sys.stderr, flush=True)
    doc = {"schema": SCHEMA, "generator": "tools/triage/triage_parity.py", "limit": limit, "limit_source": source,
           "tiny": TINY, "runtime": tm.versions(), "binary_sha256": tc.sha256_file(Path(args.binary)),
           "corpus_sha256": tc.sha256_file(args.corpus), "problems": results}
    tc.write_text(args.output, tc.dumps(doc) + "\n", args.force)
    return 1 if any(r["status"] in ("fail", "error") for r in results) else 0


if __name__ == "__main__":
    sys.exit(main())
