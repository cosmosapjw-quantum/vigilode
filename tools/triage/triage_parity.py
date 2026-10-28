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

`--diffsol-binary PATH` (tools/benchmarks/diffsol/triage `diffsol-triage`) also checks the Diffsol models
against the same Rust documents, with the same limit: the Rust document is written to a temporary file,
`diffsol-triage parity --input FILE` evaluates f, the Diffsol-assembled dense Jacobian and J v (the operators
the Diffsol solvers use) at the same t_hex/y_hex samples, and f (always), the Jacobian (when Rust has one) and
J v (when Rust has `jvp_hex`) are compared entrywise; the samples must be the same bits, sample 0 must equal
the Diffsol initial state bit for bit, t_span and atol_scale must equal CORPUS exactly, the input digest must
match and Rust must have no mass matrix. The results go to the report key `diffsol` (`{binary_sha256, problems:
[...]}`, problems the harness does not implement are `not_checked`); NumPy results are unchanged.
`--diffsol-only` skips the NumPy models (report `problems` is [] and `numpy_models` says so) and needs NumPy
but not SciPy; the limit must then come from THRESHOLDS.
"""

from __future__ import annotations

import argparse
import math
import subprocess
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import triage_common as tc  # noqa: E402
import triage_models as tm  # noqa: E402

SCHEMA = "vigilode-triage-parity-report-v1"
TINY = sys.float_info.min
DIFFSOL_LIST_SCHEMA = "vigilode-diffsol-triage-list-v1"


def decode(values) -> "np.ndarray":
    import numpy as np

    return np.asarray([tc.unhex64(v) for v in values], dtype=np.float64)


class Worst:
    """Largest relative entry error per kind, with the entry that produced it."""

    def __init__(self, other: str = "numpy"):
        self.other = other  # name of the compared side in the worst-entry record
        self.by_kind: dict[str, float] = {}
        self.entry: dict | None = None

    def update(self, kind: str, sample: int, rust, model, scale=None) -> None:
        import numpy as np

        rust = np.asarray(rust, dtype=np.float64).reshape(-1)
        model = np.asarray(model, dtype=np.float64).reshape(-1)
        if rust.shape != model.shape:
            raise tc.TriageError(f"{kind}: {rust.size} Rust entries vs {model.size} {self.other} entries")
        with np.errstate(invalid="ignore", over="ignore"):
            rel = np.abs(rust - model) / np.maximum(np.abs(rust), TINY)
        rel = np.where(np.isnan(rel), np.inf, rel)
        k = int(np.argmax(rel)) if rel.size else 0
        value = float(rel[k]) if rel.size else 0.0
        self.by_kind[kind] = max(self.by_kind.get(kind, 0.0), value)
        if self.entry is None or value > self.entry["rel"]:
            self.entry = {"kind": kind, "sample": sample, "index": k, "rust": fval(rust[k]),
                          self.other: fval(model[k]), "rel": value, "rust_hex": tc.hex64(rust[k]),
                          f"{self.other}_hex": tc.hex64(model[k]),
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


def rust_parity_document(binary: str, pid: str) -> tuple[dict | None, str | None, str | None]:
    """(document, stdout text, None) of `rodas5p triage-parity --problem P`, or (None, None, reason)."""
    proc = subprocess.run([binary, "triage-parity", "--problem", pid], capture_output=True, text=True)
    if proc.returncode != 0:
        return None, None, f"triage-parity: {tc.describe_exit(proc.returncode)}: {tc.tail(proc.stderr, 400)}"
    doc = tc.parse_json_output(proc.stdout)
    if doc is None or doc.get("schema") != tc.PARITY_INPUT_SCHEMA or doc.get("problem") != pid:
        return None, None, "triage-parity printed no matching vigilode-triage-parity-v1 document"
    return doc, proc.stdout, None


def check_problem(binary: str, problem: dict, limit: float, fetched: tuple | None = None) -> dict:
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
    doc, _, reason = fetched if fetched is not None else rust_parity_document(binary, pid)
    if doc is None:
        out.update(status="error", reason=reason)
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


def diffsol_supported(binary: str) -> tuple[set | None, str | None]:
    """Problem ids of `diffsol-triage list`, or (None, reason)."""
    try:
        proc = subprocess.run([binary, "list"], capture_output=True, text=True, timeout=120)
    except (OSError, subprocess.TimeoutExpired) as exc:
        return None, f"diffsol-triage list failed: {exc}"
    doc = tc.parse_json_output(proc.stdout) if proc.returncode == 0 else None
    if doc is None or doc.get("schema") != DIFFSOL_LIST_SCHEMA:
        return None, f"diffsol-triage list: {tc.describe_exit(proc.returncode)}: {tc.tail(proc.stderr, 400)}"
    return {p.get("id") for p in doc.get("problems", [])}, None


def check_diffsol(binary: str, problem: dict, limit: float, fetched: tuple, supported: set | None,
                  supported_reason: str | None) -> dict:
    """Diffsol models against the Rust parity document (module docstring)."""
    pid = problem["id"]
    out = {"problem": pid, "status": None, "reason": None, "samples": 0, "compared": [], "rhs_max_rel": None,
           "jacobian_max_rel": None, "jvp_max_rel": None, "y0_match": None, "definition_match": None,
           "worst": None, "model_source": None}
    if supported is None:
        out.update(status="error", reason=supported_reason)
        return out
    if pid not in supported:
        out.update(status="not_checked", reason="problem not implemented in the Diffsol harness")
        return out
    doc, text, reason = fetched
    if doc is None:
        out.update(status="error", reason=reason)
        return out
    with tempfile.TemporaryDirectory(prefix="triage_parity_diffsol_") as tmp:
        path = Path(tmp) / f"{pid}.json"
        path.write_text(text)
        proc = subprocess.run([binary, "parity", "--input", str(path)], capture_output=True, text=True)
    if proc.returncode != 0:
        out.update(status="error", reason=f"diffsol-triage parity: {tc.describe_exit(proc.returncode)}: "
                                          f"{tc.tail(proc.stderr, 400)}")
        return out
    ddoc = tc.parse_json_output(proc.stdout)
    if ddoc is None or ddoc.get("schema") != tc.PARITY_INPUT_SCHEMA or ddoc.get("problem") != pid:
        out.update(status="error", reason="diffsol-triage parity printed no matching vigilode-triage-parity-v1 document")
        return out
    failed: list[str] = []
    worst = Worst("diffsol")
    try:
        meta = ddoc["diffsol"]
        out["model_source"] = meta.get("source")
        n = doc["dimension"]
        if ddoc.get("dimension") != n:
            raise tc.TriageError(f"Rust dimension {n} != Diffsol dimension {ddoc.get('dimension')}")
        if meta.get("input_sha256") != tc.sha256_bytes(text.encode()):
            failed.append("diffsol-triage read a different document (input_sha256)")
        if doc.get("mass_hex") is not None:
            failed.append("Rust has a mass matrix, the Diffsol harness has none")
        if ddoc.get("autonomous") != doc.get("autonomous"):
            failed.append(f"autonomous {ddoc.get('autonomous')!r} != Rust {doc.get('autonomous')!r}")
        if ddoc.get("jvp_direction_hex") != doc.get("jvp_direction_hex"):
            failed.append("J v direction differs")
        definition = []
        if [float(x) for x in meta.get("t_span", [])] != [float(x) for x in problem.get("t_span", [])]:
            definition.append(f"t_span {meta.get('t_span')!r} != CORPUS {problem.get('t_span')!r}")
        if not tc.finite(meta.get("atol_scale")) or float(meta["atol_scale"]) != float(problem["atol_scale"]):
            definition.append(f"atol_scale {meta.get('atol_scale')!r} != CORPUS {problem.get('atol_scale')!r}")
        out["definition_match"] = not definition
        failed.extend(definition)
        samples, dsamples = doc.get("samples") or [], ddoc.get("samples") or []
        if not samples or len(samples) != len(dsamples):
            raise tc.TriageError(f"{len(samples)} Rust samples vs {len(dsamples)} Diffsol samples")
        out["y0_match"] = meta.get("y0_hex") == samples[0]["y_hex"]
        if not out["y0_match"]:
            failed.append("sample 0 state differs from the Diffsol initial state (bitwise)")
        for index, (rs, ds) in enumerate(zip(samples, dsamples)):
            if rs["t_hex"] != ds["t_hex"] or rs["y_hex"] != ds["y_hex"]:
                failed.append(f"sample {index}: Diffsol evaluated at different (t, y) bits")
                continue
            if rs.get("f_error") is not None or rs.get("f_hex") is None:
                failed.append(f"sample {index}: Rust f failed ({rs.get('f_error')}) but the Diffsol f succeeded")
                continue
            worst.update("rhs", index, decode(rs["f_hex"]), decode(ds["f_hex"]))
            if rs.get("jacobian") is not None:
                worst.update("jacobian", index, rust_jacobian(rs["jacobian"], n), rust_jacobian(ds["jacobian"], n))
            if rs.get("jvp_hex") is not None:
                worst.update("jvp", index, decode(rs["jvp_hex"]), decode(ds["jvp_hex"]))
        out["samples"] = len(samples)
    except (tc.TriageError, KeyError, TypeError, ValueError) as exc:
        out.update(status="error", reason=f"malformed parity document: {exc}")
        return out
    for kind, key in (("rhs", "rhs_max_rel"), ("jacobian", "jacobian_max_rel"), ("jvp", "jvp_max_rel")):
        if kind in worst.by_kind:
            value = worst.by_kind[kind]
            out[key] = value if math.isfinite(value) else None
            out["compared"].append(kind)
            if not value <= limit:
                failed.append(f"{kind} max rel {value!r} > {limit!r}")
    if worst.entry is not None:
        worst.entry["rel"] = fval(worst.entry["rel"])
    out["worst"] = worst.entry
    out.update(status="fail" if failed else "pass", reason="; ".join(failed) or None)
    return out


def runtime_versions(diffsol_only: bool) -> dict:
    """tm.versions(), with scipy null when only the Diffsol models are checked (SciPy is not needed then)."""
    if not diffsol_only:
        return tm.versions()
    import platform

    import numpy as np

    try:
        import scipy
        scipy_version = scipy.__version__
    except ImportError:
        scipy_version = None
    return {"python": platform.python_version(), "numpy": np.__version__, "scipy": scipy_version,
            "machine": platform.machine()}


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--binary", required=True, help="rodas5p binary with `triage-parity`")
    parser.add_argument("--corpus", required=True, type=Path)
    parser.add_argument("--thresholds", type=Path, default=tc.DEFAULT_NODE / "THRESHOLDS.json")
    parser.add_argument("--output", required=True, type=Path, help="PARITY.json")
    parser.add_argument("--problem", action="append")
    parser.add_argument("--diffsol-binary", help="tools/benchmarks/diffsol/triage `diffsol-triage`: also check the "
                                                 "Diffsol models against the Rust documents")
    parser.add_argument("--diffsol-only", action="store_true",
                        help="with --diffsol-binary: skip the NumPy models (needs NumPy, not SciPy)")
    parser.add_argument("--force", action="store_true")
    args = parser.parse_args(argv)
    if args.diffsol_only and not args.diffsol_binary:
        parser.error("--diffsol-only needs --diffsol-binary")
    tc.refuse_existing(args.output, args.force)
    try:
        if args.diffsol_only:
            try:
                import numpy  # noqa: F401
            except ImportError as exc:
                raise tm.ModelsUnavailable(f"numpy unavailable: {exc}") from None
        else:
            tm.require_numpy()
        corpus = tc.load_json(args.corpus)
        thresholds = tc.load_json(args.thresholds) if args.thresholds.exists() else {}
    except (tc.TriageError, tm.ModelsUnavailable) as exc:
        print("error:", exc, file=sys.stderr)
        return 2
    parity = thresholds.get("parity") if isinstance(thresholds, dict) else None
    if isinstance(parity, dict) and tc.finite(parity.get("max_relative_error")):
        limit, source = float(parity["max_relative_error"]), f"{args.thresholds.name}: parity.max_relative_error"
    elif args.diffsol_only:
        print("error: --diffsol-only needs THRESHOLDS parity.max_relative_error (the fallback limit needs SciPy)",
              file=sys.stderr)
        return 2
    else:
        limit, source = float(tm.scipy_tool().PARITY_LIMIT), "tools/stiff_benchmark_scipy.py: PARITY_LIMIT"
    diffsol = args.diffsol_binary
    if diffsol is not None and not Path(diffsol).is_file():
        print(f"error: --diffsol-binary {diffsol!r} is not a file", file=sys.stderr)
        return 2
    supported, supported_reason = diffsol_supported(diffsol) if diffsol else (None, None)
    results, diffsol_results = [], []
    for pid, problem in sorted(tc.corpus_problems(corpus).items()):
        if args.problem is None or pid in args.problem:
            fetched = rust_parity_document(args.binary, pid) if diffsol else None
            if not args.diffsol_only:
                results.append(check_problem(args.binary, problem, limit, fetched))
                r = results[-1]
                print(f"{pid}: {r['status']} rhs {r['rhs_max_rel']} jac {r['jacobian_max_rel']} "
                      f"jvp {r['jvp_max_rel']} {r['reason'] or ''}", file=sys.stderr, flush=True)
            if diffsol:
                diffsol_results.append(check_diffsol(diffsol, problem, limit, fetched, supported, supported_reason))
                d = diffsol_results[-1]
                print(f"{pid} (diffsol): {d['status']} rhs {d['rhs_max_rel']} jac {d['jacobian_max_rel']} "
                      f"jvp {d['jvp_max_rel']} {d['reason'] or ''}", file=sys.stderr, flush=True)
    doc = {"schema": SCHEMA, "generator": "tools/triage/triage_parity.py", "limit": limit, "limit_source": source,
           "tiny": TINY, "runtime": runtime_versions(args.diffsol_only),
           "binary_sha256": tc.sha256_file(Path(args.binary)), "corpus_sha256": tc.sha256_file(args.corpus),
           "problems": results}
    if args.diffsol_only:
        doc["numpy_models"] = "skipped (--diffsol-only)"
    if diffsol:
        doc["diffsol"] = {"binary_sha256": tc.sha256_file(Path(diffsol)), "problems": diffsol_results}
    tc.write_text(args.output, tc.dumps(doc) + "\n", args.force)
    return 1 if any(r["status"] in ("fail", "error") for r in results + diffsol_results) else 0


if __name__ == "__main__":
    sys.exit(main())
