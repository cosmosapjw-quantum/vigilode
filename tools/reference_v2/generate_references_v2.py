#!/usr/bin/env python3
"""Create, partition, resume, assemble, or verify ScientificCorpusV2.1 references.

This is a new wire path.  The v1 manifest/artifact bytes and loader semantics
remain historical and unchanged.  This producer never imports or invokes the
Rust implementation.  A checked-in manifest may remain ``not-run``; only an
explicit, create-new assembly after all 22 physical artifacts validates as
``complete``.
"""

from __future__ import annotations

import argparse
import base64
import csv
import hashlib
import importlib.metadata
import json
import math
import os
import resource
import struct
import sys
import time
from pathlib import Path
from typing import Any, Callable

THREAD_ENVIRONMENTS = (
    "OPENBLAS_NUM_THREADS",
    "OMP_NUM_THREADS",
    "MKL_NUM_THREADS",
    "VECLIB_MAXIMUM_THREADS",
)
for _name in THREAD_ENVIRONMENTS:
    os.environ[_name] = "1"

import numpy as np
import scipy
from scipy.integrate import solve_ivp
from scipy.integrate._ivp import radau as scipy_radau
from scipy.sparse import csc_matrix

import generate_references as legacy


MANIFEST_SCHEMA = "vigilode-numerical-reference-manifest-v2"
ARTIFACT_SCHEMA = "vigilode-numerical-reference-artifact-v2"
FAILURE_SCHEMA = "vigilode-numerical-reference-failure-v2"
CHECKPOINT_SCHEMA = "vigilode-numerical-reference-checkpoint-v2"
CORPUS_VERSION = "scientific-corpus-v2.1"
WRMS_FORMULA_ID = "wrms-tight-radau-l2-anchor-v1"
GENERATION_MODE = "partition-resume-create-new; self-check-never-regenerates"
ZERO_SHA256 = "0" * 64
RTOL_LABELS = ((1.0e-4, "1e-4"), (1.0e-6, "1e-6"), (1.0e-8, "1e-8"))
REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
CALIBRATION_ORACLE_PATH = (
    REPOSITORY_ROOT / "fixtures" / "scientific_corpus_v2_1_calibration_oracle.json"
)

GENERATOR = {
    "python": "3.12",
    "numpy": "2.4.2",
    "scipy": "1.17.0",
    "blas_threads": 1,
    "radau_ladder": [
        {"label": "L0", "method": "Radau", "rtol": 1.0e-8, "atol": 1.0e-10},
        {"label": "L1", "method": "Radau", "rtol": 1.0e-10, "atol": 1.0e-12},
        {"label": "L2", "method": "Radau", "rtol": 1.0e-12, "atol": 1.0e-14},
    ],
    "tight_lsoda": {
        "label": "tight-lsoda",
        "method": "LSODA",
        "rtol": 3.0e-14,
        "atol": 3.0e-16,
    },
}

RUNTIME = {
    "python_executable": "/usr/bin/python3.12",
    "python_version": "3.12.3",
    "python_sha256": "a92f0f95e883390c7256b2e441484aac06b1002dbe1d924141a77c8d82f96223",
    "numpy_record_sha256": "41e1145d39013f7d909361f1fd4e74c46493bcf426797898b0fb499f670204c5",
    "numpy_record_verified_file_count": 916,
    "numpy_git_revision": "c81c49f77451340651a751e76bca607d85e4fd55",
    "scipy_record_sha256": "81c576349363842874f8638770240686a20ef21499da9987901f94f8e2179ac2",
    "scipy_record_verified_file_count": 1425,
    "scipy_git_revision": "8c75ae75176236f233824e9a0483c26a69e6dfec",
    "scipy_release": True,
    "scipy_version_module_sha256": "d6a223e725b2f146a5f6d4bc578e5ff77c7165f0a70351e1d1ea3ca1bf95d61a",
    "scipy_radau_module_sha256": "d0aa4593431ef39ee07825db6ef0324e4a9bacef0e23fda42d377318ba6a6256",
    "blas_libraries": [
        {
            "role": "numpy-ilp64",
            "basename": "libscipy_openblas64_-096271d3.so",
            "version": "0.3.31.dev",
            "configuration": "USE64BITINT DYNAMIC_ARCH NO_AFFINITY Haswell MAX_THREADS=64",
            "sha256": "c0f0784c075afdeb2d57cb78e6225221f7c97ef8d03e512b3c98e105054e73c2",
        },
        {
            "role": "scipy-lp64",
            "basename": "libscipy_openblas-6cdc3b4a.so",
            "version": "0.3.30",
            "configuration": "DYNAMIC_ARCH NO_AFFINITY Haswell MAX_THREADS=64",
            "sha256": "8fb864c29cac4b25f6e2c139491ea96f2724dde42d51394f84e9c4a622e34790",
        },
    ],
    "thread_environment": {name: "1" for name in sorted(THREAD_ENVIRONMENTS)},
}


class V2ReferenceError(RuntimeError):
    pass


class ReferenceRunFailed(V2ReferenceError):
    def __init__(self, message: str, evidence: dict[str, Any]):
        super().__init__(message)
        self.evidence = evidence


class ArtifactGenerationFailed(V2ReferenceError):
    def __init__(self, message: str, run_evidence: list[dict[str, Any]]):
        super().__init__(message)
        self.run_evidence = run_evidence


def require(condition: bool, message: str) -> None:
    if not condition:
        raise V2ReferenceError(message)


def sha256_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def valid_sha256(value: Any) -> bool:
    return isinstance(value, str) and len(value) == 64 and all(c in "0123456789abcdef" for c in value)


def append_field(payload: bytearray, value: str) -> None:
    encoded = value.encode("utf-8")
    payload.extend(struct.pack("<Q", len(encoded)))
    payload.extend(encoded)


def append_grid_shape(payload: bytearray, shape: list[int] | None) -> None:
    if shape is None:
        payload.append(0)
    else:
        payload.append(1)
        payload.extend(struct.pack("<Q", int(shape[0])))
        payload.extend(struct.pack("<Q", int(shape[1])))


def append_optional_field(payload: bytearray, value: str | None) -> None:
    if value is None:
        payload.append(0)
    else:
        payload.append(1)
        append_field(payload, value)


def problem_definition_checksum(entries: list[dict[str, Any]]) -> str:
    payload = bytearray(b"vigilode-reference-problem-definitions-v2\0")
    ordered = sorted(entries, key=lambda entry: entry["problem"]["problem_id"])
    payload.extend(struct.pack("<Q", len(ordered)))
    for entry in ordered:
        problem = entry["problem"]
        for value in (problem["problem_id"], problem["family"], problem["partition"]):
            append_field(payload, value)
        payload.extend(struct.pack("<Q", problem["dimension"]))
        append_grid_shape(payload, problem["grid_shape"])
        for time_value in problem["t_span"]:
            payload.extend(legacy.bits(time_value))
        payload.extend(struct.pack("<Q", problem["uniform_output_points"]))
        payload.extend(struct.pack("<Q", len(problem["mandatory_breakpoints"])))
        for breakpoint in problem["mandatory_breakpoints"]:
            payload.extend(legacy.bits(breakpoint))
        source = problem["source"]
        for value in (source["source_repository"], source["source_revision"], source["source_path"]):
            append_field(payload, value)
        append_optional_field(payload, source["source_blob"])
        append_optional_field(payload, source["source_sha256"])
        append_field(payload, source["license_or_terms"])
        append_optional_field(payload, source["interpretation_note"])
    return sha256_bytes(bytes(payload))


def binding_checksum(binding: dict[str, Any], entry: dict[str, Any]) -> str:
    payload = bytearray(b"vigilode-reference-case-binding-v2\0")
    for value in (
        binding["case_id"],
        binding["problem_id"],
        entry["artifact_sha256"],
        entry["grid_sha256"],
        entry["state_sha256"],
    ):
        append_field(payload, value)
    append_grid_shape(payload, entry["problem"]["grid_shape"])
    return sha256_bytes(bytes(payload))


def artifact_set_checksum(entries: list[dict[str, Any]]) -> str:
    payload = bytearray(b"vigilode-reference-artifact-set-v2\0")
    ordered = sorted(entries, key=lambda entry: entry["problem"]["problem_id"])
    payload.extend(struct.pack("<Q", len(ordered)))
    for entry in ordered:
        for value in (
            entry["problem"]["problem_id"],
            entry["artifact_sha256"],
            entry["grid_sha256"],
            entry["state_sha256"],
        ):
            append_field(payload, value)
        append_grid_shape(payload, entry["problem"]["grid_shape"])
    return sha256_bytes(bytes(payload))


def binding_set_checksum(bindings: list[dict[str, Any]]) -> str:
    payload = bytearray(b"vigilode-reference-binding-set-v2\0")
    ordered = sorted(bindings, key=lambda binding: binding["case_id"])
    payload.extend(struct.pack("<Q", len(ordered)))
    for binding in ordered:
        for value in (
            binding["case_id"],
            binding["problem_id"],
            binding["reference_checksum_sha256"],
        ):
            append_field(payload, value)
    return sha256_bytes(bytes(payload))


def grid_checksum(times: list[float]) -> str:
    return legacy.grid_checksum(times)


def state_checksum(states: list[list[float]]) -> str:
    return legacy.state_checksum(states)


def calibration_source(family: str) -> dict[str, Any]:
    if family == "semilinear-advection-diffusion-ramped":
        note = (
            "VigilODE scientific-corpus-v2.1 manufactured equation: x-fast rectangular interior grid; "
            "zero Dirichlet boundary; five-point D=0.002 diffusion; backward-upwind x/y advection "
            "a(t)=0.5+3.5*r(t); mu(t)=2+48*r(t); exact phi=exp(-t)*sin(pi*x)*sin(pi*y)"
        )
    else:
        note = (
            "scientific-corpus-v2.1 retains the v2 prefix-stable base-two radical-inverse parameter "
            "diversification; legacy atlas behavior remains separate"
        )
    return {
        "source_repository": "VigilODE",
        "source_revision": CORPUS_VERSION,
        "source_path": "crates/rodas5p-integrators/src/scientific_corpus_v2.rs",
        "source_blob": None,
        "source_sha256": None,
        "license_or_terms": "repository license",
        "interpretation_note": note,
    }


def holdout_source(family: str) -> dict[str, Any]:
    table = {
        "oregonator": (
            "Bari stiff ODE test set", "orego.f file identity", "orego.f", None,
            "aa58d9090f1f581f2e60e29b02b409466197981f5399120ce66bfb2d34f41c27",
            "clean mathematical reimplementation; source distribution terms apply", None,
        ),
        "pollution": (
            "Bari stiff ODE test set", "pollu.f file identity", "pollu.f", None,
            "2aba777ee6de34e0ee074951375e029ad5171e937dabb7ab4c6461c0736e6c20",
            "clean mathematical reimplementation; source distribution terms apply", None,
        ),
        "medical-akzo": (
            "Bari stiff ODE test set", "medakzo.f file identity", "medakzo.f", None,
            "3b5a4aa80769cd752e17a64a2ae15b4b07ba2a15f037aed48b7c2158d739861a",
            "clean mathematical reimplementation; source distribution terms apply",
            "mandatory t=5 split is source-declared",
        ),
        "brusselator-2d": (
            "SciML/SciMLSensitivity.jl", "63a13a7301a17feb8cb5e3a4b3ccef4487ae0c52",
            "docs/src/examples/pde/brusselator.md", "fea9aaa141f224a97f112e024082966a1a5ee6c2",
            "688e4642b669e4181cca67d0d7cd9d663e2322d70923daf0240e5a995627351e", "MIT",
            "f64 translation of the source Float32 executable grid; h=1/15 follows its inclusive range; mandatory t=1.1 split is a VigilODE corpus policy for the discontinuous forcing, not a source tstops declaration",
        ),
    }
    repository, revision, path, blob, digest, terms, note = table[family]
    return {
        "source_repository": repository,
        "source_revision": revision,
        "source_path": path,
        "source_blob": blob,
        "source_sha256": digest,
        "license_or_terms": terms,
        "interpretation_note": note,
    }


def physical_problems() -> list[dict[str, Any]]:
    calibration = (
        ("robertson-ramped", (0.0, 0.1)),
        ("hires-ramped", (0.0, 1.0)),
        ("van-der-pol-ramped", (0.0, 1.0)),
        ("rotating-nonnormal", (0.0, 1.0)),
        ("nonautonomous-stiff-forcing", (0.0, 1.0)),
        ("semilinear-advection-diffusion-ramped", (0.0, 1.0)),
    )
    problems: list[dict[str, Any]] = []
    for family, span in calibration:
        for dimension in (96, 384, 1536):
            shape = {96: [8, 12], 384: [16, 24], 1536: [32, 48]}[dimension] if family.startswith("semilinear") else None
            if family.startswith("semilinear"):
                problem_id = f"{family}-n{dimension}-grid{shape[0]}x{shape[1]}-v2.1"
            else:
                problem_id = f"{family}-n{dimension}-v2"
            problems.append({
                "problem_id": problem_id,
                "family": family,
                "partition": "calibration",
                "dimension": dimension,
                "grid_shape": shape,
                "t_span": list(span),
                "uniform_output_points": 101,
                "mandatory_breakpoints": [],
                "source": calibration_source(family),
            })
    for family, dimension, span, breaks, problem_id in (
        ("oregonator", 3, (0.0, 360.0), [], "oregonator-holdout-v2"),
        ("pollution", 20, (0.0, 60.0), [], "pollution-holdout-v2"),
        ("medical-akzo", 400, (0.0, 20.0), [5.0], "medical-akzo-holdout-v2"),
        ("brusselator-2d", 512, (0.0, 11.5), [1.1], "brusselator-2d-holdout-v2"),
    ):
        problems.append({
            "problem_id": problem_id,
            "family": family,
            "partition": "holdout",
            "dimension": dimension,
            "grid_shape": None,
            "t_span": list(span),
            "uniform_output_points": 101,
            "mandatory_breakpoints": breaks,
            "source": holdout_source(family),
        })
    return sorted(problems, key=lambda problem: problem["problem_id"])


def case_id(problem: dict[str, Any], tolerance_label: str) -> str:
    family = problem["family"]
    dimension = problem["dimension"]
    if problem["grid_shape"] is not None:
        nx, ny = problem["grid_shape"]
        return f"{family}-n{dimension}-grid{nx}x{ny}-rtol-{tolerance_label}-v2.1"
    return f"{family}-n{dimension}-rtol-{tolerance_label}-v2.1"


def build_not_run_manifest() -> dict[str, Any]:
    artifacts = [
        {
            "problem": problem,
            "artifact_path": f"artifacts/{problem['problem_id']}.json",
            "artifact_sha256": ZERO_SHA256,
            "grid_sha256": ZERO_SHA256,
            "state_sha256": ZERO_SHA256,
            "canonical_method": GENERATOR["radau_ladder"][2],
        }
        for problem in physical_problems()
    ]
    by_problem = {entry["problem"]["problem_id"]: entry for entry in artifacts}
    bindings = [
        {
            "case_id": case_id(entry["problem"], label),
            "problem_id": entry["problem"]["problem_id"],
            "reference_checksum_sha256": ZERO_SHA256,
        }
        for entry in artifacts
        for _, label in RTOL_LABELS
    ]
    bindings.sort(key=lambda binding: binding["case_id"])
    for binding in bindings:
        binding["reference_checksum_sha256"] = binding_checksum(binding, by_problem[binding["problem_id"]])
    return {
        "schema_version": MANIFEST_SCHEMA,
        "corpus_version": CORPUS_VERSION,
        "generation_status": "not-run",
        "generation_mode": GENERATION_MODE,
        "generator": GENERATOR,
        "runtime": RUNTIME,
        "producer": {
            "script_path": "tools/reference_v2/generate_references_v2.py",
            "script_sha256": sha256_file(Path(__file__).resolve()),
            "implementation_revision": "NOT_RUN",
            "problem_definition_sha256": problem_definition_checksum(artifacts),
        },
        "wrms_policy": {"formula_id": WRMS_FORMULA_ID, "absolute": 1.0e-10, "relative": 1.0e-8},
        "artifacts": artifacts,
        "bindings": bindings,
        "artifact_set_sha256": artifact_set_checksum(artifacts),
        "binding_set_sha256": binding_set_checksum(bindings),
    }


def requested_times(problem: dict[str, Any]) -> list[float]:
    t0, tf = problem["t_span"]
    times = [t0 + (tf - t0) * float(index) / 100.0 for index in range(101)]
    for breakpoint in problem["mandatory_breakpoints"]:
        if not any(legacy.same_bits(value, breakpoint) for value in times):
            times.append(float(breakpoint))
    times.sort()
    return times


def refresh_manifest_digests(manifest: dict[str, Any]) -> None:
    by_problem = {entry["problem"]["problem_id"]: entry for entry in manifest["artifacts"]}
    for binding in manifest["bindings"]:
        binding["reference_checksum_sha256"] = binding_checksum(binding, by_problem[binding["problem_id"]])
    manifest["artifact_set_sha256"] = artifact_set_checksum(manifest["artifacts"])
    manifest["binding_set_sha256"] = binding_set_checksum(manifest["bindings"])


def validate_manifest_layout(manifest: dict[str, Any]) -> None:
    expected = build_not_run_manifest()
    require(manifest.get("schema_version") == MANIFEST_SCHEMA, "v2 manifest schema mismatch")
    require(manifest.get("corpus_version") == CORPUS_VERSION, "v2 corpus version mismatch")
    require(manifest.get("generation_status") in ("not-run", "complete"), "v2 generation status mismatch")
    require(manifest.get("generation_mode") == GENERATION_MODE, "v2 generation mode mismatch")
    require(manifest.get("generator") == GENERATOR, "v2 generator pins mismatch")
    require(manifest.get("runtime") == RUNTIME, "v2 exact runtime identity mismatch")
    producer = manifest.get("producer", {})
    require(producer.get("script_path") == "tools/reference_v2/generate_references_v2.py", "v2 producer path mismatch")
    require(producer.get("script_sha256") == sha256_file(Path(__file__).resolve()), "v2 producer bytes mismatch")
    require(producer.get("problem_definition_sha256") == problem_definition_checksum(manifest.get("artifacts", [])), "v2 problem-definition digest mismatch")
    revision = producer.get("implementation_revision")
    if manifest.get("generation_status") == "not-run":
        require(revision == "NOT_RUN", "NOT_RUN manifest has an implementation revision")
    else:
        require(isinstance(revision, str) and len(revision) == 40 and all(c in "0123456789abcdef" for c in revision), "complete manifest lacks a canonical implementation revision")
    require(manifest.get("wrms_policy") == expected["wrms_policy"], "v2 WRMS policy mismatch")
    artifacts = manifest.get("artifacts")
    bindings = manifest.get("bindings")
    require(isinstance(artifacts, list) and len(artifacts) == 22, "v2 requires 22 artifacts")
    require(isinstance(bindings, list) and len(bindings) == 66, "v2 requires 66 bindings")
    require(
        [entry.get("problem", {}).get("problem_id") for entry in artifacts]
        == [entry["problem"]["problem_id"] for entry in expected["artifacts"]],
        "v2 artifacts are not in canonical order",
    )
    require(
        [binding.get("case_id") for binding in bindings]
        == [binding["case_id"] for binding in expected["bindings"]],
        "v2 bindings are not in canonical order",
    )
    expected_entries = {entry["problem"]["problem_id"]: entry for entry in expected["artifacts"]}
    expected_problems = {
        problem_id: entry["problem"] for problem_id, entry in expected_entries.items()
    }
    by_problem: dict[str, dict[str, Any]] = {}
    for entry in artifacts:
        problem = entry.get("problem", {})
        problem_id = problem.get("problem_id")
        require(problem_id in expected_problems and problem == expected_problems[problem_id], "v2 problem metadata mismatch")
        require(
            entry.get("artifact_path") == expected_entries[problem_id]["artifact_path"],
            "v2 artifact path mismatch",
        )
        require(problem_id not in by_problem, "duplicate v2 physical problem")
        by_problem[problem_id] = entry
        path = Path(entry.get("artifact_path", ""))
        require(not path.is_absolute() and ".." not in path.parts, "v2 artifact path is not relative")
        require(entry.get("canonical_method") == GENERATOR["radau_ladder"][2], "v2 canonical method is not L2")
        require(all(valid_sha256(entry.get(field)) for field in ("artifact_sha256", "grid_sha256", "state_sha256")), "bad v2 artifact digest")
        digests = [entry[field] for field in ("artifact_sha256", "grid_sha256", "state_sha256")]
        if manifest.get("generation_status") == "not-run":
            require(all(digest == ZERO_SHA256 for digest in digests), "NOT_RUN artifact has materialized digests")
        else:
            require(all(digest != ZERO_SHA256 for digest in digests), "complete artifact retains NOT_RUN digest sentinel")
    expected_bindings = {
        binding["case_id"]: binding["problem_id"] for binding in expected["bindings"]
    }
    seen: set[str] = set()
    indegree = {problem_id: 0 for problem_id in by_problem}
    for binding in bindings:
        identity = binding.get("case_id")
        require(identity in expected_bindings and identity not in seen, "missing/extra/duplicate v2 case binding")
        seen.add(identity)
        problem_id = binding.get("problem_id")
        require(problem_id == expected_bindings[identity], "v2 case misbinding")
        require(binding.get("reference_checksum_sha256") == binding_checksum(binding, by_problem[problem_id]), "v2 binding checksum mismatch")
        indegree[problem_id] += 1
    require(seen == set(expected_bindings), "v2 binding set mismatch")
    require(all(value == 3 for value in indegree.values()), "v2 physical artifact indegree is not three")
    require(manifest.get("artifact_set_sha256") == artifact_set_checksum(artifacts), "v2 artifact-set checksum mismatch")
    require(manifest.get("binding_set_sha256") == binding_set_checksum(bindings), "v2 binding-set checksum mismatch")


def canonical_json(value: dict[str, Any]) -> bytes:
    return (json.dumps(value, allow_nan=False, sort_keys=True, separators=(",", ":")) + "\n").encode()


def atomic_create(path: Path, payload: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.tmp-{os.getpid()}")
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o644)
    try:
        with os.fdopen(descriptor, "wb", closefd=True) as handle:
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
        os.link(temporary, path)
    except FileExistsError as error:
        raise V2ReferenceError(f"create-new target already exists: {path}") from error
    finally:
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass


def read_json(path: Path) -> dict[str, Any]:
    try:
        return json.loads(path.read_bytes())
    except (OSError, json.JSONDecodeError) as error:
        raise V2ReferenceError(f"cannot read {path}: {error}") from error


def create_template(path: Path) -> None:
    manifest = build_not_run_manifest()
    validate_manifest_layout(manifest)
    atomic_create(path, canonical_json(manifest))


def diversity_multiplier(index: int) -> float:
    value = index + 1
    fraction = 0.5
    radical_inverse = 0.0
    while value:
        if value & 1:
            radical_inverse += fraction
        value >>= 1
        fraction *= 0.5
    return 0.9 + 0.2 * radical_inverse


def smooth_ramp(time_value: float, center: float, width: float) -> tuple[float, float]:
    value = math.tanh((time_value - center) / width)
    return 0.5 * (1.0 + value), 0.5 * (1.0 - value * value) / width


def sparse_from_columns(dimension: int, columns: list[tuple[int, int, float]]) -> csc_matrix:
    rows = [row for row, _, _ in columns]
    cols = [column for _, column, _ in columns]
    values = [value for _, _, value in columns]
    return csc_matrix((values, (rows, cols)), shape=(dimension, dimension))


def robertson_runtime(dimension: int):
    blocks = dimension // 3

    def rhs(time_value: float, state: np.ndarray) -> np.ndarray:
        ramp, _ = smooth_ramp(time_value, 0.045, 0.010)
        activity = 0.05 + 0.95 * ramp
        out = np.empty(dimension)
        for block in range(blocks):
            i = 3 * block
            scale = diversity_multiplier(block)
            k1, k2, k3 = 0.04 * scale, 1.0e4 * activity * scale, 3.0e7 * activity * scale
            y1, y2, y3 = state[i : i + 3]
            out[i] = -k1 * y1 + k2 * y2 * y3
            out[i + 1] = k1 * y1 - k2 * y2 * y3 - k3 * y2 * y2
            out[i + 2] = k3 * y2 * y2
        if 3 * blocks < dimension:
            out[3 * blocks :] = -20.0 * activity * state[3 * blocks :]
        return out

    def jac(time_value: float, state: np.ndarray) -> csc_matrix:
        ramp, _ = smooth_ramp(time_value, 0.045, 0.010)
        activity = 0.05 + 0.95 * ramp
        values: list[tuple[int, int, float]] = []
        for block in range(blocks):
            i = 3 * block
            scale = diversity_multiplier(block)
            k1, k2, k3 = 0.04 * scale, 1.0e4 * activity * scale, 3.0e7 * activity * scale
            y2, y3 = state[i + 1], state[i + 2]
            values += [
                (i, i, -k1), (i, i + 1, k2 * y3), (i, i + 2, k2 * y2),
                (i + 1, i, k1), (i + 1, i + 1, -k2 * y3 - 2.0 * k3 * y2),
                (i + 1, i + 2, -k2 * y2), (i + 2, i + 1, 2.0 * k3 * y2),
            ]
        return sparse_from_columns(dimension, values)

    initial = np.zeros(dimension)
    initial[0 : 3 * blocks : 3] = 1.0
    return rhs, jac, initial, 2


def hires_runtime(dimension: int):
    blocks = dimension // 8

    def rhs(time_value: float, state: np.ndarray) -> np.ndarray:
        ramp, _ = smooth_ramp(time_value, 0.45, 0.08)
        activity = 0.1 + 0.9 * ramp
        out = np.empty(dimension)
        for block in range(blocks):
            i = 8 * block
            scale = diversity_multiplier(block)
            y1, y2, y3, y4, y5, y6, y7, y8 = state[i : i + 8]
            q = 280.0 * activity * y6 * y8
            out[i] = scale * (-1.71 * y1 + 0.43 * y2 + 8.32 * y3 + 0.0007)
            out[i + 1] = scale * (1.71 * y1 - 8.75 * y2)
            out[i + 2] = scale * (-10.03 * y3 + 0.43 * y4 + 0.035 * y5)
            out[i + 3] = scale * (8.32 * y2 + 1.71 * y3 - 1.12 * y4)
            out[i + 4] = scale * (-1.745 * y5 + 0.43 * y6 + 0.43 * y7)
            out[i + 5] = scale * (-q + 0.69 * y4 + 1.71 * y5 - 0.43 * y6 + 0.69 * y7)
            out[i + 6] = scale * (q - 1.81 * y7)
            out[i + 7] = scale * (-q + 1.81 * y7)
        return out

    def jac(time_value: float, state: np.ndarray) -> csc_matrix:
        ramp, _ = smooth_ramp(time_value, 0.45, 0.08)
        activity = 0.1 + 0.9 * ramp
        values: list[tuple[int, int, float]] = []
        for block in range(blocks):
            i = 8 * block
            scale = diversity_multiplier(block)
            y6, y8 = state[i + 5], state[i + 7]
            q6 = 280.0 * activity * y8
            q8 = 280.0 * activity * y6
            rows = (
                (-1.71, 0.43, 8.32, 0, 0, 0, 0, 0),
                (1.71, -8.75, 0, 0, 0, 0, 0, 0),
                (0, 0, -10.03, 0.43, 0.035, 0, 0, 0),
                (0, 8.32, 1.71, -1.12, 0, 0, 0, 0),
                (0, 0, 0, 0, -1.745, 0.43, 0.43, 0),
                (0, 0, 0, 0.69, 1.71, -0.43 - q6, 0.69, -q8),
                (0, 0, 0, 0, 0, q6, -1.81, q8),
                (0, 0, 0, 0, 0, -q6, 1.81, -q8),
            )
            for row, entries in enumerate(rows):
                for column, value in enumerate(entries):
                    if value != 0.0:
                        values.append((i + row, i + column, scale * value))
        return sparse_from_columns(dimension, values)

    initial = np.zeros(dimension)
    initial[0 : 8 * blocks : 8] = 1.0
    initial[7 : 8 * blocks : 8] = 0.0057
    return rhs, jac, initial, 7


def vdp_runtime(dimension: int):
    blocks = dimension // 2

    def rhs(time_value: float, state: np.ndarray) -> np.ndarray:
        ramp, _ = smooth_ramp(time_value, 0.5, 0.08)
        mu = 10.0 + 490.0 * ramp
        out = np.empty(dimension)
        for block in range(blocks):
            i = 2 * block
            local_mu = mu * diversity_multiplier(block)
            out[i] = state[i + 1]
            out[i + 1] = local_mu * (1.0 - state[i] * state[i]) * state[i + 1] - state[i]
        return out

    def jac(time_value: float, state: np.ndarray) -> csc_matrix:
        ramp, _ = smooth_ramp(time_value, 0.5, 0.08)
        mu = 10.0 + 490.0 * ramp
        values = []
        for block in range(blocks):
            i = 2 * block
            local_mu = mu * diversity_multiplier(block)
            values += [
                (i, i + 1, 1.0),
                (i + 1, i, -2.0 * local_mu * state[i] * state[i + 1] - 1.0),
                (i + 1, i + 1, local_mu * (1.0 - state[i] * state[i])),
            ]
        return sparse_from_columns(dimension, values)

    initial = np.zeros(dimension)
    initial[0 : 2 * blocks : 2] = 2.0
    return rhs, jac, initial, 1


def rotating_shape(dimension: int, time_value: float, derivative: int) -> np.ndarray:
    out = np.empty(dimension)
    for index in range(dimension):
        frequency = (1.0 + float(index % 7)) * diversity_multiplier(index // 2)
        if derivative == 0:
            out[index] = 0.4 * math.sin(frequency * time_value) + 0.2 * math.cos(0.5 * frequency * time_value)
        elif derivative == 1:
            out[index] = 0.4 * frequency * math.cos(frequency * time_value) - 0.1 * frequency * math.sin(0.5 * frequency * time_value)
        else:
            out[index] = -0.4 * frequency * frequency * math.sin(frequency * time_value) - 0.05 * frequency * frequency * math.cos(0.5 * frequency * time_value)
    return out


def rotating_operator(time_value: float, values: np.ndarray) -> np.ndarray:
    ramp, _ = smooth_ramp(time_value, 0.5, 0.08)
    stiffness0 = 20.0 + 480.0 * ramp
    eta = 0.1 + 0.8 * ramp
    theta0 = 8.0 * time_value + 0.4 * math.sin(4.0 * time_value)
    out = np.empty_like(values)
    for block in range(len(values) // 2):
        i = 2 * block
        scale = diversity_multiplier(block)
        stiffness, theta = stiffness0 * scale, theta0 * scale
        cosine, sine = math.cos(theta), math.sin(theta)
        xr0 = cosine * values[i] + sine * values[i + 1]
        xr1 = -sine * values[i] + cosine * values[i + 1]
        ar0 = -stiffness * xr0 + eta * stiffness * xr1
        ar1 = -0.35 * stiffness * xr1
        out[i] = cosine * ar0 - sine * ar1
        out[i + 1] = sine * ar0 + cosine * ar1
    return out


def rotating_runtime(dimension: int):
    def rhs(time_value: float, state: np.ndarray) -> np.ndarray:
        phi = rotating_shape(dimension, time_value, 0)
        dphi = rotating_shape(dimension, time_value, 1)
        ramp, _ = smooth_ramp(time_value, 0.6, 0.06)
        return rotating_operator(time_value, state - phi) + dphi + 40.0 * ramp * (state * state - phi * phi)

    def jac(time_value: float, state: np.ndarray) -> csc_matrix:
        ramp, _ = smooth_ramp(time_value, 0.6, 0.06)
        nonlinear = 40.0 * ramp
        values: list[tuple[int, int, float]] = []
        for block in range(dimension // 2):
            i = 2 * block
            for column in range(2):
                basis = np.zeros(dimension)
                basis[i + column] = 1.0
                image = rotating_operator(time_value, basis)
                for row in range(2):
                    value = float(image[i + row])
                    if row == column:
                        value += 2.0 * nonlinear * state[i + row]
                    values.append((i + row, i + column, value))
        return sparse_from_columns(dimension, values)

    initial = rotating_shape(dimension, 0.0, 0)
    return rhs, jac, initial, 1


def forcing_runtime(dimension: int):
    def rhs(time_value: float, state: np.ndarray) -> np.ndarray:
        ramp, _ = smooth_ramp(time_value, 0.45, 0.07)
        stiffness = 30.0 + 470.0 * ramp
        frequency = 2.0 + 28.0 * ramp
        out = np.empty(dimension)
        for index in range(dimension):
            scale = diversity_multiplier(index)
            argument = frequency * time_value + float(index % 11) * 0.17
            phi = math.sin(argument)
            defect = state[index] - phi
            out[index] = -scale * stiffness * defect + scale * frequency * math.cos(argument) + 20.0 * ramp * defect * defect
        return out

    def jac(time_value: float, state: np.ndarray) -> csc_matrix:
        ramp, _ = smooth_ramp(time_value, 0.45, 0.07)
        stiffness = 30.0 + 470.0 * ramp
        frequency = 2.0 + 28.0 * ramp
        diagonal = np.empty(dimension)
        for index in range(dimension):
            scale = diversity_multiplier(index)
            phi = math.sin(frequency * time_value + float(index % 11) * 0.17)
            diagonal[index] = -scale * stiffness + 40.0 * ramp * (state[index] - phi)
        return csc_matrix((diagonal, (np.arange(dimension), np.arange(dimension))), shape=(dimension, dimension))

    initial = np.asarray([math.sin(float(index % 11) * 0.17) for index in range(dimension)])
    return rhs, jac, initial, 0


def semilinear_runtime(dimension: int):
    nx, ny = legacy.semilinear_grid_shape(dimension)
    rhs = lambda time_value, state: legacy.semilinear_rhs(nx, ny, time_value, state)
    jac = lambda time_value, state: legacy.semilinear_jacobian(nx, ny, time_value, state)
    initial = legacy.semilinear_exact_state(nx, ny, 0.0)
    return rhs, jac, initial, nx


def packed_banded(jacobian: csc_matrix | np.ndarray, lower: int, upper: int) -> np.ndarray:
    dense = jacobian.toarray() if hasattr(jacobian, "toarray") else np.asarray(jacobian)
    packed = np.zeros((lower + upper + 1, dense.shape[1]), dtype=np.float64)
    for column in range(dense.shape[1]):
        for row in range(max(0, column - upper), min(dense.shape[0], column + lower + 1)):
            packed[upper + row - column, column] = dense[row, column]
    return packed


def problem_runtime(problem: dict[str, Any]):
    family, dimension = problem["family"], problem["dimension"]
    if family == "robertson-ramped":
        return robertson_runtime(dimension)
    if family == "hires-ramped":
        return hires_runtime(dimension)
    if family == "van-der-pol-ramped":
        return vdp_runtime(dimension)
    if family == "rotating-nonnormal":
        return rotating_runtime(dimension)
    if family == "nonautonomous-stiff-forcing":
        return forcing_runtime(dimension)
    if family == "semilinear-advection-diffusion-ramped":
        return semilinear_runtime(dimension)
    if family == "oregonator":
        return legacy.oregonator_rhs, legacy.oregonator_jac, legacy.initial_state(family), None
    if family == "pollution":
        return legacy.pollution_rhs, legacy.pollution_jac, legacy.initial_state(family), None
    if family == "medical-akzo":
        return legacy.medical_rhs, legacy.medical_jac_sparse, legacy.initial_state(family), 2
    if family == "brusselator-2d":
        return legacy.brusselator_rhs, legacy.brusselator_jac_sparse, legacy.initial_state(family), None
    raise V2ReferenceError(f"unknown v2 physical family {family}")


def validate_source_equation_oracles() -> None:
    """Bind both language implementations to common high-precision observations."""
    # This validator also checks exact f64 bits for selected manufactured phi
    # values, preventing Python/Rust operation association from drifting.
    legacy.validate_semilinear_oracle(legacy.SEMILINEAR_ORACLE_PATH)

    oracle = read_json(CALIBRATION_ORACLE_PATH)
    require(
        oracle.get("schema_version")
        == "scientific-corpus-v2.1-calibration-mpmath-oracle-v1",
        "calibration equation oracle schema mismatch",
    )
    require(
        oracle.get("origin")
        == {"engine": "mpmath", "version": "1.3.0", "decimal_digits": 80},
        "calibration equation oracle origin mismatch",
    )
    require(oracle.get("state") == "y0+0.01*cos(0.17*(p+1))", "oracle state mismatch")
    require(oracle.get("direction") == "sin(0.23*(p+1))", "oracle direction mismatch")
    expected_indices = {
        "robertson-ramped": [*range(0, 3), *range(48, 51), *range(93, 96)],
        "hires-ramped": [*range(0, 8), *range(48, 56), *range(88, 96)],
        "van-der-pol-ramped": [0, 1, 48, 49, 94, 95],
        "rotating-nonnormal": [0, 1, 48, 49, 94, 95],
        "nonautonomous-stiff-forcing": [0, 48, 95],
    }
    cases = oracle.get("cases")
    require(isinstance(cases, list) and len(cases) == 5, "calibration oracle case count mismatch")
    problems = {
        problem["family"]: problem
        for problem in physical_problems()
        if problem["partition"] == "calibration" and problem["dimension"] == 96
    }
    seen: set[str] = set()
    time_value = float(oracle["time"])
    for expected in cases:
        family = expected.get("family")
        require(family in expected_indices and family not in seen, "unknown/duplicate oracle family")
        seen.add(family)
        require(expected.get("dimension") == 96, "calibration oracle dimension mismatch")
        samples = expected.get("samples")
        require(
            isinstance(samples, list)
            and [sample.get("index") for sample in samples] == expected_indices[family],
            f"{family} oracle does not cover every selected block component",
        )
        rhs, jacobian, initial, _ = problem_runtime(problems[family])
        state = initial + np.asarray(
            [0.01 * math.cos(0.17 * float(index + 1)) for index in range(96)],
            dtype=np.float64,
        )
        direction = np.asarray(
            [math.sin(0.23 * float(index + 1)) for index in range(96)],
            dtype=np.float64,
        )
        actual_rhs = rhs(time_value, state)
        actual_jvp = jacobian(time_value, state) @ direction
        for sample in samples:
            index = int(sample["index"])
            for actual, decimal, label in (
                (initial[index], sample["y0"], "y0"),
                (actual_rhs[index], sample["rhs"], "RHS"),
                (actual_jvp[index], sample["jvp"], "Jv"),
            ):
                expected_value = float(decimal)
                scaled = abs(float(actual) - expected_value) / max(1.0, abs(expected_value))
                require(scaled <= 1.0e-13, f"{family} {label} differs from shared mpmath oracle")
    require(seen == set(expected_indices), "calibration oracle family set mismatch")


def locate_distribution_record(name: str) -> Path:
    distribution = importlib.metadata.distribution(name)
    path = Path(distribution._path) / "RECORD"  # exact installed wheel evidence
    require(path.is_file(), f"missing {name} RECORD")
    return path


def verify_distribution_record(name: str, expected_count: int) -> Path:
    distribution = importlib.metadata.distribution(name)
    record = locate_distribution_record(name)
    verified = 0
    with record.open(newline="", encoding="utf-8") as handle:
        for relative, digest_field, size_field in csv.reader(handle):
            if not digest_field.startswith("sha256="):
                continue
            path = Path(distribution.locate_file(relative))
            require(path.is_file(), f"{name} RECORD member missing: {relative}")
            payload = path.read_bytes()
            encoded = digest_field.removeprefix("sha256=")
            expected_digest = base64.urlsafe_b64decode(encoded + "=" * (-len(encoded) % 4))
            require(hashlib.sha256(payload).digest() == expected_digest, f"{name} RECORD hash mismatch: {relative}")
            require(size_field.isdigit() and len(payload) == int(size_field), f"{name} RECORD size mismatch: {relative}")
            verified += 1
    require(verified == expected_count, f"{name} verified RECORD count mismatch")
    return record


def require_exact_runtime() -> None:
    require(str(Path(sys.executable).resolve()) == RUNTIME["python_executable"], "wrong Python executable")
    require(".".join(map(str, sys.version_info[:3])) == RUNTIME["python_version"], "wrong Python patch version")
    require(sha256_file(Path(sys.executable).resolve()) == RUNTIME["python_sha256"], "wrong Python bytes")
    require(np.__version__ == GENERATOR["numpy"] and scipy.__version__ == GENERATOR["scipy"], "wrong NumPy/SciPy version")
    numpy_record = verify_distribution_record("numpy", RUNTIME["numpy_record_verified_file_count"])
    scipy_record = verify_distribution_record("scipy", RUNTIME["scipy_record_verified_file_count"])
    require(sha256_file(numpy_record) == RUNTIME["numpy_record_sha256"], "wrong NumPy distribution")
    require(sha256_file(scipy_record) == RUNTIME["scipy_record_sha256"], "wrong SciPy distribution")
    require(getattr(np.version, "git_revision", None) == RUNTIME["numpy_git_revision"], "wrong NumPy revision")
    require(scipy.version.git_revision == RUNTIME["scipy_git_revision"] and scipy.version.release is True, "wrong SciPy revision")
    require(sha256_file(Path(scipy.version.__file__)) == RUNTIME["scipy_version_module_sha256"], "wrong SciPy version module")
    radau_path = Path(scipy_radau.__file__)
    require(sha256_file(radau_path) == RUNTIME["scipy_radau_module_sha256"], "wrong SciPy Radau module")
    site = Path(np.__file__).resolve().parent.parent
    library_roots = (site / "numpy.libs", site / "scipy.libs")
    for expected in RUNTIME["blas_libraries"]:
        matches = [root / expected["basename"] for root in library_roots if (root / expected["basename"]).is_file()]
        require(len(matches) == 1, f"missing or ambiguous BLAS library {expected['basename']}")
        require(sha256_file(matches[0]) == expected["sha256"], f"wrong BLAS library {expected['basename']}")
    require({name: os.environ.get(name) for name in sorted(THREAD_ENVIRONMENTS)} == RUNTIME["thread_environment"], "wrong BLAS thread environment")
    validate_source_equation_oracles()


def branch_fixed_rhs(problem: dict[str, Any], segment_index: int, rhs: Callable[..., np.ndarray]):
    if problem["family"] == "medical-akzo":
        phi = 2.0 if segment_index == 0 else 0.0
        return lambda _time, state: legacy.medical_rhs_with_phi(state, phi)
    if problem["family"] == "brusselator-2d":
        enabled = segment_index != 0
        return lambda _time, state: legacy.brusselator_rhs_with_forcing(state, enabled)
    return rhs


def solve_reference_run(
    problem: dict[str, Any], method: dict[str, Any]
) -> tuple[np.ndarray, dict[str, Any]]:
    rhs, jacobian, initial, bandwidth = problem_runtime(problem)
    times = requested_times(problem)
    bounds = [problem["t_span"][0], *problem["mandatory_breakpoints"], problem["t_span"][1]]
    state = np.asarray(initial, dtype=np.float64)
    output: list[np.ndarray] = []
    output_times: list[float] = []
    counters = {"nfev": 0, "njev": 0, "nlu": 0}
    started = time.perf_counter()
    for segment_index, (start, end) in enumerate(zip(bounds, bounds[1:])):
        segment_times = [value for value in times if start <= value <= end]
        require(segment_times and legacy.same_bits(segment_times[-1], end), "segment lacks exact endpoint")
        options: dict[str, Any] = {
            "method": method["method"],
            "rtol": method["rtol"],
            "atol": method["atol"],
            "t_eval": np.asarray(segment_times, dtype=np.float64),
        }
        if method["method"] == "Radau":
            options["jac"] = jacobian
        else:
            require(method["method"] == "LSODA", "unsupported reference method")
            if bandwidth is None:
                def dense_jacobian(t: float, y: np.ndarray) -> np.ndarray:
                    matrix = jacobian(t, y)
                    return matrix.toarray() if hasattr(matrix, "toarray") else np.asarray(matrix)

                options["jac"] = dense_jacobian
            else:
                options["lband"] = bandwidth
                options["uband"] = bandwidth
                options["jac"] = lambda t, y: packed_banded(jacobian(t, y), bandwidth, bandwidth)
        try:
            result = solve_ivp(
                branch_fixed_rhs(problem, segment_index, rhs),
                (start, end),
                state,
                **options,
            )
        except Exception as error:
            evidence = {
                "label": method["label"],
                "status": "failed",
                "wall_seconds": time.perf_counter() - started,
                **counters,
                "process_peak_rss_bytes_at_run_end": int(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss) * 1024,
                "message": f"{type(error).__name__}: {error}",
            }
            raise ReferenceRunFailed(evidence["message"], evidence) from error
        counters["nfev"] += int(result.nfev)
        counters["njev"] += int(result.njev)
        counters["nlu"] += int(result.nlu)
        if not result.success:
            evidence = {
                "label": method["label"],
                "status": "failed",
                "wall_seconds": time.perf_counter() - started,
                **counters,
                "process_peak_rss_bytes_at_run_end": int(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss) * 1024,
                "message": str(result.message),
            }
            raise ReferenceRunFailed(
                f"{problem['problem_id']} {method['label']} failed: {result.message}", evidence
            )
        require(result.y.shape[1] == len(segment_times), "reference solver omitted output")
        state = result.y[:, -1].copy()
        first = 0 if segment_index == 0 else 1
        output.extend(result.y[:, first:].T.copy())
        output_times.extend(segment_times[first:])
    require(len(output_times) == len(times) and all(legacy.same_bits(a, b) for a, b in zip(output_times, times)), "reference grid changed")
    states = np.asarray(output, dtype=np.float64)
    require(states.shape == (len(times), problem["dimension"]) and np.isfinite(states).all(), "bad reference state table")
    evidence = {
        "label": method["label"],
        "status": "complete",
        "wall_seconds": time.perf_counter() - started,
        **counters,
        "process_peak_rss_bytes_at_run_end": int(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss) * 1024,
        "message": None,
    }
    return states, evidence


def anchor_wrms(left: np.ndarray, right: np.ndarray, tight_l2: np.ndarray) -> float:
    require(left.shape == right.shape == tight_l2.shape, "anchor WRMS shape mismatch")
    weights = 1.0e-10 + 1.0e-8 * np.abs(tight_l2)
    rows = np.sqrt(np.mean(np.square((left - right) / weights), axis=1))
    value = float(np.max(rows))
    require(math.isfinite(value) and value >= 0.0, "nonfinite anchor WRMS")
    return value


def build_artifact(problem: dict[str, Any]) -> dict[str, Any]:
    levels: list[np.ndarray] = []
    run_evidence: list[dict[str, Any]] = []
    try:
        for method in GENERATOR["radau_ladder"]:
            states, evidence = solve_reference_run(problem, method)
            levels.append(states)
            run_evidence.append(evidence)
        independent, evidence = solve_reference_run(problem, GENERATOR["tight_lsoda"])
    except ReferenceRunFailed as error:
        raise ArtifactGenerationFailed(str(error), [*run_evidence, error.evidence]) from error
    run_evidence.append(evidence)
    tight = levels[2]
    d0 = anchor_wrms(levels[0], levels[1], tight)
    d1 = anchor_wrms(levels[1], tight, tight)
    require(d0 > d1 >= 0.0, "reference ladder does not satisfy D0>D1")
    q = d1 / d0
    require(math.isfinite(q) and q <= 0.5, f"reference q={q} exceeds 0.5")
    richardson = d1 * q / (1.0 - q)
    disagreement = anchor_wrms(tight, independent, tight)
    times = requested_times(problem)
    states = [[float(value) for value in row] for row in tight]
    grid_sha = grid_checksum(times)
    state_sha = state_checksum(states)
    artifact = {
        "schema_version": ARTIFACT_SCHEMA,
        "problem_id": problem["problem_id"],
        "requested_times": times,
        "states": states,
        "canonical_method": GENERATOR["radau_ladder"][2],
        "independent_method": GENERATOR["tight_lsoda"],
        "convergence": {
            "d0_max_grid_wrms": d0,
            "d1_max_grid_wrms": d1,
            "q": q,
            "richardson_uncertainty_wrms": richardson,
            "method_disagreement_wrms": disagreement,
            "reference_uncertainty_wrms": richardson + disagreement,
            "wrms_basis": {
                "formula_id": WRMS_FORMULA_ID,
                "absolute": 1.0e-10,
                "relative": 1.0e-8,
                "anchor_state_sha256": state_sha,
            },
        },
        "checksums": {"grid_sha256": grid_sha, "state_sha256": state_sha},
        "run_evidence": run_evidence,
    }
    validate_artifact(artifact, problem)
    return artifact


def validate_artifact(artifact: dict[str, Any], problem: dict[str, Any]) -> None:
    require(artifact.get("schema_version") == ARTIFACT_SCHEMA, "v2 artifact schema mismatch")
    require(artifact.get("problem_id") == problem["problem_id"], "v2 artifact problem mismatch")
    require(artifact.get("canonical_method") == GENERATOR["radau_ladder"][2], "v2 artifact canonical source is not L2")
    require(artifact.get("independent_method") == GENERATOR["tight_lsoda"], "v2 artifact independent method mismatch")
    times = artifact.get("requested_times")
    expected_times = requested_times(problem)
    require(isinstance(times, list) and len(times) == len(expected_times) and all(legacy.same_bits(a, b) for a, b in zip(times, expected_times)), "v2 artifact grid mismatch")
    states = artifact.get("states")
    require(isinstance(states, list) and len(states) == len(times), "v2 artifact state count mismatch")
    require(all(isinstance(row, list) and len(row) == problem["dimension"] and all(math.isfinite(float(value)) for value in row) for row in states), "v2 artifact state shape/finiteness mismatch")
    checksums = artifact.get("checksums", {})
    require(checksums.get("grid_sha256") == grid_checksum(times), "v2 artifact grid checksum mismatch")
    require(checksums.get("state_sha256") == state_checksum(states), "v2 artifact state checksum mismatch")
    convergence = artifact.get("convergence", {})
    basis = convergence.get("wrms_basis", {})
    require(basis == {
        "formula_id": WRMS_FORMULA_ID,
        "absolute": 1.0e-10,
        "relative": 1.0e-8,
        "anchor_state_sha256": checksums["state_sha256"],
    }, "v2 artifact WRMS basis mismatch")
    d0, d1 = float(convergence.get("d0_max_grid_wrms", math.nan)), float(convergence.get("d1_max_grid_wrms", math.nan))
    q = float(convergence.get("q", math.nan))
    require(d0 > d1 >= 0.0 and 0.0 <= q <= 0.5, "v2 artifact convergence invalid")
    legacy.assert_close(q, d1 / d0, "v2 q != D1/D0")
    richardson = d1 * q / (1.0 - q)
    legacy.assert_close(float(convergence["richardson_uncertainty_wrms"]), richardson, "v2 Richardson mismatch")
    disagreement = float(convergence["method_disagreement_wrms"])
    legacy.assert_close(float(convergence["reference_uncertainty_wrms"]), richardson + disagreement, "v2 uncertainty mismatch")
    evidence = artifact.get("run_evidence")
    expected_labels = [method["label"] for method in GENERATOR["radau_ladder"]] + [GENERATOR["tight_lsoda"]["label"]]
    require(isinstance(evidence, list) and sorted(run["label"] for run in evidence) == sorted(expected_labels), "v2 run evidence labels mismatch")
    for run in evidence:
        require(run.get("status") == "complete" and run.get("message") is None, "v2 artifact contains failed run")
        require(math.isfinite(float(run.get("wall_seconds", math.nan))) and run["wall_seconds"] >= 0.0, "bad v2 run timing")
        require(all(isinstance(run.get(field), int) and run[field] >= 0 for field in ("nfev", "njev", "nlu", "process_peak_rss_bytes_at_run_end")), "bad v2 run counters")


def validate_artifact_file(path: Path, problem: dict[str, Any]) -> tuple[dict[str, Any], bytes]:
    try:
        payload = path.read_bytes()
        artifact = json.loads(payload)
    except (OSError, json.JSONDecodeError) as error:
        raise V2ReferenceError(f"cannot read v2 artifact {path}: {error}") from error
    validate_artifact(artifact, problem)
    return artifact, payload


def generate_partition(
    manifest_path: Path,
    partition_index: int,
    partition_count: int,
    checkpoint_path: Path,
    resume: bool,
    artifact_builder: Callable[[dict[str, Any]], dict[str, Any]] = build_artifact,
) -> None:
    require_exact_runtime()
    require(partition_count > 0 and 0 <= partition_index < partition_count, "invalid partition index/count")
    manifest = read_json(manifest_path)
    validate_manifest_layout(manifest)
    require(manifest["generation_status"] == "not-run", "partition generation requires NOT_RUN manifest")
    selected = [
        entry
        for index, entry in enumerate(sorted(manifest["artifacts"], key=lambda item: item["problem"]["problem_id"]))
        if index % partition_count == partition_index
    ]
    started = time.perf_counter()
    completed: list[str] = []
    skipped: list[str] = []
    failures: list[dict[str, Any]] = []
    for entry in selected:
        problem = entry["problem"]
        destination = manifest_path.parent / entry["artifact_path"]
        if destination.exists():
            require(resume, f"artifact already exists without --resume: {destination}")
            validate_artifact_file(destination, problem)
            skipped.append(problem["problem_id"])
            continue
        try:
            artifact = artifact_builder(problem)
            validate_artifact(artifact, problem)
            atomic_create(destination, canonical_json(artifact))
            completed.append(problem["problem_id"])
        except ArtifactGenerationFailed as error:
            failure = {
                "schema_version": FAILURE_SCHEMA,
                "problem": problem,
                "partition": {"index": partition_index, "count": partition_count},
                "message": str(error),
                "run_evidence": error.run_evidence,
            }
            failure_path = manifest_path.parent / "failures" / f"{problem['problem_id']}.json"
            atomic_create(failure_path, canonical_json(failure))
            failures.append({"problem_id": problem["problem_id"], "failure_path": str(failure_path.relative_to(manifest_path.parent))})
        except Exception as error:
            failure = {
                "schema_version": FAILURE_SCHEMA,
                "problem": problem,
                "partition": {"index": partition_index, "count": partition_count},
                "message": f"{type(error).__name__}: {error}",
                "run_evidence": [],
            }
            failure_path = manifest_path.parent / "failures" / f"{problem['problem_id']}.json"
            atomic_create(failure_path, canonical_json(failure))
            failures.append({"problem_id": problem["problem_id"], "failure_path": str(failure_path.relative_to(manifest_path.parent))})
    checkpoint = {
        "schema_version": CHECKPOINT_SCHEMA,
        "manifest_sha256": sha256_file(manifest_path),
        "partition_index": partition_index,
        "partition_count": partition_count,
        "selected_problem_ids": [entry["problem"]["problem_id"] for entry in selected],
        "completed_problem_ids": completed,
        "resumed_problem_ids": skipped,
        "failures": failures,
        "wall_seconds": time.perf_counter() - started,
        "process_peak_rss_bytes_at_partition_end": int(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss) * 1024,
    }
    atomic_create(checkpoint_path, canonical_json(checkpoint))
    require(not failures, f"partition preserved {len(failures)} failed artifacts")


def assemble_complete_manifest(not_run_path: Path, output_path: Path) -> None:
    require_exact_runtime()
    manifest = read_json(not_run_path)
    validate_manifest_layout(manifest)
    require(manifest["generation_status"] == "not-run", "assembly source must be NOT_RUN")
    for entry in manifest["artifacts"]:
        artifact_path = not_run_path.parent / entry["artifact_path"]
        artifact, payload = validate_artifact_file(artifact_path, entry["problem"])
        entry["artifact_sha256"] = sha256_bytes(payload)
        entry["grid_sha256"] = artifact["checksums"]["grid_sha256"]
        entry["state_sha256"] = artifact["checksums"]["state_sha256"]
    manifest["generation_status"] = "complete"
    revision = os.environ.get("VIGILODE_CODE_REVISION")
    require(isinstance(revision, str) and len(revision) == 40 and all(c in "0123456789abcdef" for c in revision), "assembly requires exact VIGILODE_CODE_REVISION")
    manifest["producer"]["implementation_revision"] = revision
    refresh_manifest_digests(manifest)
    validate_manifest_layout(manifest)
    atomic_create(output_path, canonical_json(manifest))
    self_check(output_path)


def self_check(manifest_path: Path) -> None:
    require_exact_runtime()
    manifest = read_json(manifest_path)
    validate_manifest_layout(manifest)
    require(manifest["generation_status"] == "complete", "full self-check requires complete manifest")
    for entry in manifest["artifacts"]:
        artifact_path = manifest_path.parent / entry["artifact_path"]
        artifact, payload = validate_artifact_file(artifact_path, entry["problem"])
        require(sha256_bytes(payload) == entry["artifact_sha256"], f"raw artifact checksum mismatch: {artifact_path}")
        require(artifact["checksums"]["grid_sha256"] == entry["grid_sha256"], "manifest/artifact grid mismatch")
        require(artifact["checksums"]["state_sha256"] == entry["state_sha256"], "manifest/artifact state mismatch")


def parse_partition(value: str) -> tuple[int, int]:
    try:
        index_text, count_text = value.split("/", 1)
        return int(index_text), int(count_text)
    except (ValueError, TypeError) as error:
        raise argparse.ArgumentTypeError("partition must be INDEX/COUNT") from error


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--init", action="store_true")
    mode.add_argument("--template-check", action="store_true")
    mode.add_argument("--generate-partition", type=parse_partition)
    mode.add_argument("--assemble", action="store_true")
    mode.add_argument("--self-check", action="store_true")
    parser.add_argument("--resume", action="store_true")
    parser.add_argument("--checkpoint", type=Path)
    parser.add_argument("--output-manifest", type=Path)
    args = parser.parse_args()
    try:
        if args.init:
            create_template(args.manifest)
        elif args.template_check:
            validate_manifest_layout(read_json(args.manifest))
        elif args.generate_partition is not None:
            require(args.checkpoint is not None, "--checkpoint is required for partition generation")
            generate_partition(args.manifest, *args.generate_partition, args.checkpoint, args.resume)
        elif args.assemble:
            require(args.output_manifest is not None, "--output-manifest is required for assembly")
            assemble_complete_manifest(args.manifest, args.output_manifest)
        else:
            self_check(args.manifest)
    except V2ReferenceError as error:
        print(f"REFERENCE_V2_ERROR: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
