#!/usr/bin/env python3
"""Mutation tests for tools/evidence_schema_v2.py and its wiring into the ALG04/ALG05/ALG06 checkers.

Re-audit node AS03 (finding F104). Every mutation is applied to a temporary copy of the committed inputs; no
committed BASE/RUNS/RESULTS file is written and no solver is run. The four malformed full-checker copies recorded
in research/reaudit_accuracy_speed_20261010/CHECKER_PROBES.json (all PASS before) are rebuilt byte for byte (their
recorded SHA-256 is checked) and must now be INVALID. Valid copies of the committed historical inputs must
schema-check and reproduce the committed RESULTS (ALG04 FAIL, ALG05 FAIL, ALG06 PASS).

Run: python3 -m unittest discover -s tools -p test_evidence_schema_v2.py   (and again with python3 -O)
"""

from __future__ import annotations

import ast
import copy
import hashlib
import importlib.util
import json
import os
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
TOOLS = ROOT / "tools"
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))
import evidence_schema_v2 as es  # noqa: E402

CHECKERS = {
    "ALG04": TOOLS / "alg04_coupled_target_v2_check.py",
    "ALG05": TOOLS / "alg05_controller_v2_check.py",
    "ALG06": TOOLS / "alg06_guard_v2_check.py",
}
INPUTS = {
    "ALG04": {"base": "research/alg01_coupled_stage_target_20261008/BASE.json",
              "runs": "research/alg04_coupled_target_v2_20261010/RUNS.json",
              "alg01-runs": "research/alg01_coupled_stage_target_20261008/RUNS.json"},
    "ALG05": {"base": "research/alg05_predictive_controller_v2_20261010/BASE.json",
              "runs": "research/alg05_predictive_controller_v2_20261010/RUNS.json",
              "native": "research/stiff_native_benchmark_20261001/NATIVE.json"},
    "ALG06": {"base": "research/alg03_stage_budget_guard_20261008/BASE.json",
              "runs": "research/alg06_guard_v2_20261010/RUNS.json",
              "alg03-runs": "research/alg03_stage_budget_guard_20261008/RUNS.json"},
}
RESULTS = {
    "ALG04": "research/alg04_coupled_target_v2_20261010/RESULTS.json",
    "ALG05": "research/alg05_predictive_controller_v2_20261010/RESULTS.json",
    "ALG06": "research/alg06_guard_v2_20261010/RESULTS.json",
}
EXPECTED_VERDICTS = {"ALG04": "FAIL", "ALG05": "FAIL", "ALG06": "PASS"}
PROBES = ROOT / "research/reaudit_accuracy_speed_20261010/CHECKER_PROBES.json"
PROTECTED = sorted({p for d in INPUTS.values() for p in d.values()} | set(RESULTS.values())
                   | {str(PROBES.relative_to(ROOT))})

_HASHES_BEFORE: dict[str, str] = {}
_DOCS: dict[str, dict] = {}


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def bits(value: float) -> str:
    return struct.pack(">d", value).hex()


def committed(kind: str, role: str) -> dict:
    """The committed document, parsed once; tests deep-copy before mutating."""
    key = f"{kind}:{role}"
    if key not in _DOCS:
        _DOCS[key] = json.loads((ROOT / INPUTS[kind][role]).read_text())
    return _DOCS[key]


def setUpModule():  # noqa: N802
    for rel in PROTECTED:
        _HASHES_BEFORE[rel] = digest(ROOT / rel)


def tearDownModule():  # noqa: N802
    changed = [rel for rel in PROTECTED if digest(ROOT / rel) != _HASHES_BEFORE[rel]]
    if changed:
        raise AssertionError(f"committed evidence changed during the tests: {changed}")


def load_checker(kind: str):
    spec = importlib.util.spec_from_file_location("as03_checker_" + kind, CHECKERS[kind])
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class CheckerRun:
    """Run a checker on files in a fresh temporary directory (with the interpreter's own -O level)."""

    def __init__(self, kind: str, tmp: Path, files: dict[str, Path | None], cwd: Path = ROOT):
        self.output = tmp / "RESULT.json"
        command = [sys.executable, *(["-O"] * sys.flags.optimize), str(CHECKERS[kind])]
        for role, path in files.items():
            command += [f"--{role}", str(path if path is not None else tmp / "NOT_REQUESTED.json")]
        command += ["--output", str(self.output)]
        env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1")
        self.process = subprocess.run(command, cwd=cwd, text=True, capture_output=True, timeout=300, env=env)
        self.report = json.loads(self.output.read_text()) if self.output.exists() else None


def copy_inputs(kind: str, tmp: Path, overrides: dict[str, dict] | None = None,
                optional: bool = False) -> dict[str, Path | None]:
    """Byte copies of the committed inputs; overridden roles are re-serialised as the probes did."""
    files: dict[str, Path | None] = {}
    for role, rel in INPUTS[kind].items():
        if role.endswith("-runs") and not optional:
            files[role] = None  # optional reproduction input not requested (as checker_probes.py)
            continue
        dest = tmp / f"{role.upper()}.json"
        if overrides and role in overrides:
            dest.write_text(json.dumps(overrides[role], allow_nan=False) + "\n")
        else:
            shutil.copyfile(ROOT / rel, dest)
        files[role] = dest
    return files


class InvalidAssertions(unittest.TestCase):
    def assert_invalid(self, run: CheckerRun, *needles: str):
        self.assertEqual(run.process.returncode, es.INVALID_EXIT_CODE, run.process.stdout + run.process.stderr)
        self.assertIsNotNone(run.report, run.process.stderr)
        self.assertEqual(run.report["verdict"], es.INVALID)
        self.assertIsNone(run.report["gate"])
        self.assertEqual(run.report["evidence_schema"], es.VERSION)
        self.assertGreater(run.report["reasons_total"], 0)
        text = "\n".join(run.report["reasons"])
        for needle in needles:
            self.assertIn(needle, text)
        self.assertIn("verdict: INVALID", run.process.stdout)

    def assert_validation_error(self, fn, *needles: str):
        with self.assertRaises(es.ValidationError) as ctx:
            fn()
        text = "\n".join(ctx.exception.reasons)
        for needle in needles:
            self.assertIn(needle, text)
        return ctx.exception


# --------------------------------------------------------------------------------- recorded probes
class RecordedAlg06Probes(InvalidAssertions):
    """The four recorded malformed full-checker copies of CHECKER_PROBES.json: PASS before, INVALID now."""

    CANDIDATE = ("D1", "brusselator-1d-160", 1e-4)
    TWIN = ("D2", "robertson-4e10", 1e-5)
    EXPECT = {
        "truncated_candidate": ("length 1 != required 320",),
        "nan_candidate": ("non-finite binary64 value 7ff8000000000000",),
        "wrong_twin": ("twin is not bound to BASE",),
        "duplicate_row": ("duplicate raw key ('D1', 'brusselator-1d-160', 0.0001)",),
    }

    @staticmethod
    def mutate(case: str) -> dict:
        """The exact mutation of checker_probes.end_to_end_probes."""
        base, runs = committed("ALG06", "base"), committed("ALG06", "runs")
        mutated = copy.deepcopy(runs)
        key = lambda row: (row["group"], row["case"], row["rtol"])  # noqa: E731
        selected = next(row for row in mutated["rows"] if key(row) == RecordedAlg06Probes.CANDIDATE)
        if case == "truncated_candidate":
            selected["arms"]["b3"]["y_last"] = base["references"][selected["case"]]["y"][:1]
        elif case == "nan_candidate":
            state = list(base["references"][selected["case"]]["y"])
            state[1] = bits(float("nan"))
            selected["arms"]["b3"]["y_last"] = state
        elif case == "wrong_twin":
            selected = next(row for row in mutated["rows"] if key(row) == RecordedAlg06Probes.TWIN)
            selected["twin"]["y_last"][0] = bits(0.0)
        elif case == "duplicate_row":
            mutated["rows"].append(copy.deepcopy(selected))
        else:
            raise ValueError(case)
        return mutated

    def test_recorded_probes_were_pass_and_are_now_invalid(self):
        recorded = {p["mutation"]["case"]: p for p in json.loads(PROBES.read_text())["end_to_end_probes"]}
        self.assertEqual(set(recorded), set(self.EXPECT))
        for case, needles in self.EXPECT.items():
            with self.subTest(case=case), tempfile.TemporaryDirectory(prefix="as03_probe_") as d:
                tmp = Path(d)
                files = copy_inputs("ALG06", tmp, {"runs": self.mutate(case)})
                # The copy is the recorded malformed copy, byte for byte, which the old checker passed.
                self.assertEqual(digest(files["runs"]), recorded[case]["copied_runs_sha256"])
                self.assertTrue(recorded[case]["malformed_copy_passed"])
                self.assertEqual(recorded[case]["checker_result"]["verdict"], "PASS")
                run = CheckerRun("ALG06", tmp, files)
                self.assert_invalid(run, *needles)


# ------------------------------------------------------------------------------------ ALG05 cases
class Alg05Cases(InvalidAssertions):
    CELL = ("van-der-pol-mu1000", "PREDcap2")  # first PREDcap2 van der Pol row (dimension 2)

    @classmethod
    def setUpClass(cls):
        cls.checker = load_checker("ALG05")
        native = json.loads((ROOT / INPUTS["ALG05"]["native"]).read_text())
        cls.reference = native["references"]["van-der-pol-mu1000"]["final_state"]

    def test_helper_rejects_without_assert(self):
        ref2 = [0.0, 1.0]
        self.assertEqual(self.checker.endpoint_error([bits(0.0), bits(1.0)], ref2), 0.0)
        cases = {
            "truncated": ([bits(0.0)], ref2, "metric length mismatch: state 1 vs reference 2"),
            "nan_second": ([bits(0.0), bits(float("nan"))], [0.0, 0.0], "metric state[1] is not a finite float"),
            "infinity_second": ([bits(0.0), bits(float("inf"))], [0.0, 0.0], "metric state[1] is not a finite"),
            "empty": ([], [], "metric state must be a nonempty list"),
            "nan_reference": ([bits(0.0)], [float("nan")], "metric reference[0] is not a finite float"),
        }
        for case, (state, ref, needle) in cases.items():
            with self.subTest(case=case):
                self.assert_validation_error(lambda: self.checker.endpoint_error(state, ref), needle)

    def test_helper_rejects_under_python_O(self):
        """The former length assert vanished under -O; the typed check must not."""
        code = (
            "import importlib.util, struct, sys\n"
            "sys.dont_write_bytecode = True\n"
            f"spec = importlib.util.spec_from_file_location('m', {str(CHECKERS['ALG05'])!r})\n"
            "m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)\n"
            "b = lambda v: struct.pack('>d', v).hex()\n"
            "out = []\n"
            "for state, ref in (([b(0.0)], [0.0, 1.0]), ([b(0.0), b(float('nan'))], [0.0, 0.0])):\n"
            "    try:\n"
            "        m.endpoint_error(state, ref); out.append('returned')\n"
            "    except m.evidence.ValidationError:\n"
            "        out.append('ValidationError')\n"
            "print(sys.flags.optimize, ' '.join(out))\n"
        )
        process = subprocess.run([sys.executable, "-O", "-c", code], text=True, capture_output=True, timeout=60,
                                 env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1"))
        self.assertEqual(process.stdout.strip(), "1 ValidationError ValidationError", process.stderr)

    def mutated_runs(self, mutate) -> dict:
        runs = copy.deepcopy(committed("ALG05", "runs"))
        row = next(r for r in runs["rows"] if r["problem"] == self.CELL[0] and r["arm"] == self.CELL[1])
        mutate(runs, row)
        return runs

    def run_mutation(self, mutate, *needles):
        with tempfile.TemporaryDirectory(prefix="as03_alg05_") as d:
            tmp = Path(d)
            run = CheckerRun("ALG05", tmp, copy_inputs("ALG05", tmp, {"runs": self.mutated_runs(mutate)}))
            self.assert_invalid(run, *needles)

    def test_full_checker_nan_second(self):
        def mutate(runs, row):
            state = [bits(v) for v in self.reference]  # equal to the reference ...
            state[1] = bits(float("nan"))  # ... except a NaN in the second component
            row["final_state"] = state
            row["error"] = 0.0
        self.run_mutation(mutate, "final_state[1]: non-finite binary64 value 7ff8000000000000")

    def test_full_checker_truncated_state(self):
        def mutate(runs, row):
            row["final_state"] = [bits(self.reference[0])]
        self.run_mutation(mutate, "final_state: length 1 != required 2")

    def test_full_checker_recorded_error_unbound(self):
        def mutate(runs, row):
            row["error"] = row["error"] / 2.0
        self.run_mutation(mutate, "!= recomputed")

    def test_full_checker_duplicate_row(self):
        def mutate(runs, row):
            runs["rows"].append(copy.deepcopy(row))
        self.run_mutation(mutate, "duplicate raw key")

    def test_full_checker_bool_as_int(self):
        def mutate(runs, row):
            row["rejected"] = False
        self.run_mutation(mutate, "rejected: expected int, got bool")


# ------------------------------------------------------------------------------------ ALG04 cases
class Alg04Cases(InvalidAssertions):
    CANDIDATE = ("C1", "brusselator-1d-160", 1e-6)
    TWIN = ("C2", "robertson", 1e-9)

    def run_mutation(self, mutate, *needles):
        runs = copy.deepcopy(committed("ALG04", "runs"))
        base = committed("ALG04", "base")
        key = lambda row: (row["group"], row["case"], row["rtol"])  # noqa: E731
        rows = {key(r): r for r in runs["rows"]}
        mutate(runs, rows, base)
        with tempfile.TemporaryDirectory(prefix="as03_alg04_") as d:
            tmp = Path(d)
            run = CheckerRun("ALG04", tmp, copy_inputs("ALG04", tmp, {"runs": runs}))
            self.assert_invalid(run, *needles)

    def test_truncated_candidate(self):
        def mutate(runs, rows, base):
            rows[self.CANDIDATE]["arms"]["coupled_guarded2"]["y_last"] = \
                base["references"][self.CANDIDATE[1]]["y"][:1]
        self.run_mutation(mutate, "length 1 != required 320")

    def test_nan_candidate(self):
        def mutate(runs, rows, base):
            state = list(base["references"][self.CANDIDATE[1]]["y"])
            state[1] = bits(float("nan"))
            rows[self.CANDIDATE]["arms"]["coupled_guarded2"]["y_last"] = state
        self.run_mutation(mutate, "non-finite binary64 value 7ff8000000000000")

    def test_wrong_twin(self):
        def mutate(runs, rows, base):
            rows[self.TWIN]["twin"]["y_last"][0] = bits(0.0)
        self.run_mutation(mutate, "twin is not bound to BASE")

    def test_wrong_lu_rung(self):
        def mutate(runs, rows, base):
            runs["ladders"][0]["lu"]["rel_max_norm_error"] *= 2.0
        self.run_mutation(mutate, "LU reference rung is not bound to BASE")

    def test_duplicate_row(self):
        def mutate(runs, rows, base):
            runs["rows"].append(copy.deepcopy(rows[self.CANDIDATE]))
        self.run_mutation(mutate, "duplicate raw key ('C1', 'brusselator-1d-160', 1e-06)")

    def test_missing_arm(self):
        def mutate(runs, rows, base):
            del rows[self.CANDIDATE]["arms"]["proj_l2"]
        self.run_mutation(mutate, "arms: key set differs (missing ['proj_l2']")


# ------------------------------------------------------------------------- direct validator rules
class ValidatorRules(InvalidAssertions):
    """Rule-level mutations through validate_alg06 (the smallest artifact family)."""

    def validate(self, runs_text: str | None = None, runs: dict | None = None, base_bytes: bytes | None = None,
                 pins=es.REGISTERED_SHA256):
        with tempfile.TemporaryDirectory(prefix="as03_rules_") as d:
            tmp = Path(d)
            base_path, runs_path = tmp / "BASE.json", tmp / "RUNS.json"
            base_path.write_bytes(base_bytes if base_bytes is not None
                                  else (ROOT / INPUTS["ALG06"]["base"]).read_bytes())
            if runs_text is None:
                runs_text = json.dumps(runs if runs is not None else committed("ALG06", "runs"))
            runs_path.write_text(runs_text)
            return es.validate_alg06(base_path, runs_path, None, pins=pins)

    def runs_with(self, mutate) -> dict:
        runs = copy.deepcopy(committed("ALG06", "runs"))
        mutate(runs)
        return runs

    def test_valid_control(self):
        evidence = self.validate()
        self.assertEqual(evidence.status, es.VALID)
        self.assertEqual(evidence.sha256["base"], es.REGISTERED_SHA256["alg03_base"])

    def test_strict_json(self):
        text = json.dumps(committed("ALG06", "runs"))
        first = '"accepted": '
        i = text.index(first) + len(first)
        j = text.index(",", i)
        cases = {
            "nan_literal": (text[:i] + "NaN" + text[j:], "non-standard JSON constant NaN"),
            "overflow": (text[:i] + "1e999" + text[j:], "is not a finite binary64 value"),
            "duplicate_object_key": (text[:i] + "1, " + first + "2" + text[j:], "duplicate object key 'accepted'"),
            "not_json": ("{", "strict JSON"),
        }
        for case, (mutated, needle) in cases.items():
            with self.subTest(case=case):
                self.assert_validation_error(lambda: self.validate(runs_text=mutated), needle)

    def test_types(self):
        def first_b3(runs):
            return runs["rows"][0]["arms"]["b3"]
        cases = {
            "bool_as_int": (lambda r: first_b3(r).__setitem__("accepted", True), "accepted: expected int, got bool"),
            "int_as_bool": (lambda r: first_b3(r).__setitem__("ok", 1), "ok: expected bool, got int"),
            "int_as_float": (lambda r: r["rows"][0].__setitem__("rtol", 1), "rtol: expected finite float, got int"),
            "negative_count": (lambda r: first_b3(r)["counters"].__setitem__("jvp_vectors", -1),
                               "jvp_vectors: expected int >= 0"),
            "string_count": (lambda r: first_b3(r)["counters"].__setitem__("jvp_vectors", "7"),
                             "jvp_vectors: expected int, got str"),
            "float_statistic_as_int": (lambda r: first_b3(r)["stage_statistics"].__setitem__("nu_max", 1),
                                       "nu_max: expected finite float, got int"),
            "uppercase_hex": (lambda r: first_b3(r)["y_last"].__setitem__(0, "3FF0000000000000"),
                              "not a 16-digit lowercase binary64 hex string"),
            "spaced_hex": (lambda r: first_b3(r)["y_last"].__setitem__(0, "3ff00000 00000000"),
                           "not a 16-digit lowercase binary64 hex string"),
            "infinite_time": (lambda r: first_b3(r)["t"].__setitem__(0, bits(float("inf"))),
                              "non-finite binary64 value 7ff0000000000000"),
            "empty_state": (lambda r: first_b3(r).__setitem__("y_last", []), "y_last: empty array"),
            "charge_row_short": (lambda r: next(a for row in r["rows"] for a in row["arms"].values()
                                                if a.get("charges"))["charges"][0].pop(),
                                 "charge row length 5 != 6"),
            "extra_key": (lambda r: first_b3(r).__setitem__("note", "x"), "unexpected ['note']"),
            "charge_columns": (lambda r: r["charge_columns"].reverse(), "runs.charge_columns: expected"),
            "missing_d5_cell": (lambda r: r["rows"].pop(), "D5 cell set differs"),
            "dimension_unbound": (lambda r: r["rows"][0].__setitem__("dimension", 321), "!= BASE 320"),
        }
        for case, (mutate, needle) in cases.items():
            with self.subTest(case=case):
                self.assert_validation_error(lambda: self.validate(runs=self.runs_with(mutate)), needle)

    def test_base_is_bound_to_its_registered_bytes(self):
        base = json.loads((ROOT / INPUTS["ALG06"]["base"]).read_text())
        twin = base["rows"][3]["twin"]
        twin["y_last"][0] = bits(0.0)
        runs = self.runs_with(lambda r: r["rows"][3]["twin"]["y_last"].__setitem__(0, bits(0.0)))
        tampered = (json.dumps(base) + "\n").encode()
        # Base and RUNS changed together: only the immutable-base binding can catch it.
        self.assert_validation_error(lambda: self.validate(runs=runs, base_bytes=tampered),
                                     "is not the registered immutable input")
        # Same bytes re-serialised (no pin): the structure is valid, so the pin is the operative rule.
        self.validate(runs=runs, base_bytes=tampered, pins=None)

    def test_dense_reference_bound_to_its_recorded_run(self):
        base = json.loads((ROOT / INPUTS["ALG06"]["base"]).read_text())
        ref = base["references"]["robertson-4e10"]
        ref["y"][0] = bits(1.0)
        tampered = (json.dumps(base) + "\n").encode()
        self.assert_validation_error(lambda: self.validate(base_bytes=tampered, pins=None),
                                     "dense reference is not the state of its recorded dense_1e-13 run")

    def test_require_metric_pair(self):
        es.require_metric_pair([1.0, 2.0], [1.0, 2.0])
        for state, ref, needle in (([1.0], [1.0, 2.0], "length mismatch"),
                                   ([1.0, float("nan")], [1.0, 2.0], "state[1] is not a finite float"),
                                   ([True], [1.0], "state[0] is not a finite float"),
                                   ([1], [1.0], "state[0] is not a finite float"),
                                   ([], [], "nonempty")):
            with self.subTest(state=state, ref=ref):
                self.assert_validation_error(lambda: es.require_metric_pair(state, ref), needle)

    def test_missing_input_is_invalid(self):
        with tempfile.TemporaryDirectory(prefix="as03_missing_") as d:
            self.assert_validation_error(
                lambda: es.validate_alg06(Path(d) / "BASE.json", Path(d) / "RUNS.json"), "cannot read")


# ------------------------------------------------------------------------------- historical inputs
class HistoricalInputs(unittest.TestCase):
    """Valid copies of the committed inputs schema-check once and reproduce the committed verdicts."""

    def test_schema_check_once(self):
        validators = {"ALG04": lambda f: es.validate_alg04(f["base"], f["runs"], f["alg01-runs"]),
                      "ALG05": lambda f: es.validate_alg05(f["base"], f["runs"], f["native"]),
                      "ALG06": lambda f: es.validate_alg06(f["base"], f["runs"], f["alg03-runs"])}
        for kind, validate in validators.items():
            with self.subTest(kind=kind), tempfile.TemporaryDirectory(prefix="as03_valid_") as d:
                files = copy_inputs(kind, Path(d), optional=True)
                evidence = validate(files)
                self.assertEqual(evidence.status, es.VALID)
                self.assertEqual(evidence.kind, kind)
                for role, rel in INPUTS[kind].items():
                    self.assertEqual(evidence.sha256[role.replace("-", "_")], digest(ROOT / rel))

    def test_copies_reproduce_committed_results(self):
        for kind, expected in EXPECTED_VERDICTS.items():
            with self.subTest(kind=kind), tempfile.TemporaryDirectory(prefix="as03_hist_") as d:
                tmp = Path(d)
                files = copy_inputs(kind, tmp, optional=True)
                run = CheckerRun(kind, tmp, files)
                self.assertEqual(run.process.returncode, 0, run.process.stderr)
                committed_result = json.loads((ROOT / RESULTS[kind]).read_text())
                self.assertEqual(committed_result["verdict"], expected)
                self.assertEqual(run.report["verdict"], expected)
                gate = "gates" if kind == "ALG05" else "gate"
                self.assertEqual(run.report[gate], committed_result[gate])
                # Identical report apart from the input paths (temporary copies); digests are identical.
                ours, theirs = dict(run.report), dict(committed_result)
                ours_inputs, theirs_inputs = ours.pop("inputs"), theirs.pop("inputs")
                self.assertEqual(ours, theirs)
                if kind == "ALG05":
                    self.assertEqual({k: v["sha256"] for k, v in ours_inputs.items()},
                                     {k: v["sha256"] for k, v in theirs_inputs.items()})
                else:
                    self.assertEqual(sorted(ours_inputs.values()), sorted(theirs_inputs.values()))

    def test_committed_paths_rerun_byte_identical(self):
        """With the recorded command's relative paths, the hardened checker writes the committed bytes."""
        for kind in EXPECTED_VERDICTS:
            with self.subTest(kind=kind), tempfile.TemporaryDirectory(prefix="as03_bytes_") as d:
                tmp = Path(d)
                files = {role: Path(rel) for role, rel in INPUTS[kind].items() if not role.endswith("-runs")}
                run = CheckerRun(kind, tmp, files, cwd=ROOT)
                self.assertEqual(run.process.returncode, 0, run.process.stderr)
                self.assertEqual(run.output.read_bytes(), (ROOT / RESULTS[kind]).read_bytes())

    def test_output_stays_immutable(self):
        with tempfile.TemporaryDirectory(prefix="as03_immutable_") as d:
            tmp = Path(d)
            (tmp / "RESULT.json").write_text("{}\n")
            run = CheckerRun("ALG06", tmp, copy_inputs("ALG06", tmp))
            self.assertNotEqual(run.process.returncode, 0)
            self.assertIn("immutable output exists", run.process.stderr)
            self.assertEqual((tmp / "RESULT.json").read_text(), "{}\n")


class NoAssertAuthority(unittest.TestCase):
    def test_no_assert_statements(self):
        for path in [TOOLS / "evidence_schema_v2.py", *CHECKERS.values()]:
            with self.subTest(path=path.name):
                tree = ast.parse(path.read_text())
                asserts = [n.lineno for n in ast.walk(tree) if isinstance(n, ast.Assert)]
                self.assertEqual(asserts, [])


if __name__ == "__main__":
    unittest.main()
