#!/usr/bin/env python3
"""Generate or verify the independent ScientificCorpusV2 numerical references.

`--generate` is deliberately explicit because it runs eight tight integrations for the two
large holdouts.  `--self-check` never integrates: it validates the pinned runtime, manifest,
raw artifact bytes, source definitions, grids, checksums, and convergence evidence already on
disk.  This script does not import or invoke the Rust repository or vendor Fortran code.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import struct
import sys
from pathlib import Path
from typing import Any, Callable

# These must be established before NumPy/SciPy loads any BLAS/OpenMP runtime.  The manifest and
# self-check record the exact single-thread rule as part of generated-evidence reproducibility.
BLAS_THREAD_ENVIRONMENTS = (
    "OPENBLAS_NUM_THREADS",
    "OMP_NUM_THREADS",
    "MKL_NUM_THREADS",
    "VECLIB_MAXIMUM_THREADS",
)
for _thread_environment in BLAS_THREAD_ENVIRONMENTS:
    os.environ[_thread_environment] = "1"

import numpy as np
import scipy
from scipy.integrate import solve_ivp
from scipy.sparse import csc_matrix


MANIFEST_SCHEMA = "vigilode-numerical-reference-manifest-v1"
ARTIFACT_SCHEMA = "vigilode-numerical-reference-artifact-v1"
GENERATION_MODE = "explicit-generate; self-check-never-regenerates"
EXPECTED_RUNTIME = {"python": "3.12", "numpy": "2.4.2", "scipy": "1.17.0"}
WRMS_SCALE = {"absolute": 1.0e-10, "relative": 1.0e-8}
REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
SEMILINEAR_ORACLE_PATH = REPOSITORY_ROOT / "fixtures" / "scientific_corpus_v2_1_semilinear_oracle.json"


class ReferenceValidationError(RuntimeError):
    """Raised for an invalid or non-reproducible reference input/output."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ReferenceValidationError(message)


def require_runtime_pins() -> None:
    require(sys.version_info[:2] == (3, 12), "requires Python 3.12.x")
    require(np.__version__ == EXPECTED_RUNTIME["numpy"], "requires NumPy 2.4.2")
    require(scipy.__version__ == EXPECTED_RUNTIME["scipy"], "requires SciPy 1.17.0")
    require(
        all(os.environ.get(name) == "1" for name in BLAS_THREAD_ENVIRONMENTS),
        "requires deterministic single-thread BLAS/OpenMP settings",
    )


def bits(value: float) -> bytes:
    return struct.pack("<Q", struct.unpack("<Q", struct.pack("<d", float(value)))[0])


def same_bits(left: float, right: float) -> bool:
    return bits(left) == bits(right)


def sha256_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def grid_checksum(times: list[float]) -> str:
    payload = bytearray(b"vigilode-reference-grid-v1\0")
    payload.extend(struct.pack("<Q", len(times)))
    for time in times:
        payload.extend(bits(time))
    return sha256_bytes(bytes(payload))


def state_checksum(states: list[list[float]]) -> str:
    payload = bytearray(b"vigilode-reference-states-v1\0")
    payload.extend(struct.pack("<Q", len(states)))
    for state in states:
        payload.extend(struct.pack("<Q", len(state)))
        for value in state:
            payload.extend(bits(value))
    return sha256_bytes(bytes(payload))


def artifact_set_checksum(entries: list[dict[str, Any]]) -> str:
    payload = bytearray(b"vigilode-reference-artifact-set-v1\0")
    ordered = sorted(entries, key=lambda entry: entry["problem"]["problem_id"])
    payload.extend(struct.pack("<Q", len(ordered)))
    for entry in ordered:
        for field in (
            entry["problem"]["problem_id"],
            entry["artifact_sha256"],
            entry["grid_sha256"],
            entry["state_sha256"],
        ):
            encoded = field.encode("utf-8")
            payload.extend(struct.pack("<Q", len(encoded)))
            payload.extend(encoded)
    return sha256_bytes(bytes(payload))


HOLDOUTS: dict[str, dict[str, Any]] = {
    "oregonator": {
        "problem_id": "oregonator-holdout-v2",
        "dimension": 3,
        "t_span": [0.0, 360.0],
        "mandatory_breakpoints": [],
        "source": {
            "source_definition_id": "bari-stiff-ode/orego.f@aa58d9090f1f581f2e60e29b02b409466197981f5399120ce66bfb2d34f41c27",
            "source_repository": "Bari stiff ODE test set",
            "source_revision": "orego.f file identity",
            "source_path": "orego.f",
            "source_blob": None,
            "source_sha256": "aa58d9090f1f581f2e60e29b02b409466197981f5399120ce66bfb2d34f41c27",
        },
    },
    "pollution": {
        "problem_id": "pollution-holdout-v2",
        "dimension": 20,
        "t_span": [0.0, 60.0],
        "mandatory_breakpoints": [],
        "source": {
            "source_definition_id": "bari-stiff-ode/pollu.f@2aba777ee6de34e0ee074951375e029ad5171e937dabb7ab4c6461c0736e6c20",
            "source_repository": "Bari stiff ODE test set",
            "source_revision": "pollu.f file identity",
            "source_path": "pollu.f",
            "source_blob": None,
            "source_sha256": "2aba777ee6de34e0ee074951375e029ad5171e937dabb7ab4c6461c0736e6c20",
        },
    },
    "medical-akzo": {
        "problem_id": "medical-akzo-holdout-v2",
        "dimension": 400,
        "t_span": [0.0, 20.0],
        "mandatory_breakpoints": [5.0],
        "source": {
            "source_definition_id": "bari-stiff-ode/medakzo.f@3b5a4aa80769cd752e17a64a2ae15b4b07ba2a15f037aed48b7c2158d739861a",
            "source_repository": "Bari stiff ODE test set",
            "source_revision": "medakzo.f file identity",
            "source_path": "medakzo.f",
            "source_blob": None,
            "source_sha256": "3b5a4aa80769cd752e17a64a2ae15b4b07ba2a15f037aed48b7c2158d739861a",
        },
    },
    "brusselator-2d": {
        "problem_id": "brusselator-2d-holdout-v2",
        "dimension": 512,
        "t_span": [0.0, 11.5],
        "mandatory_breakpoints": [1.1],
        "source": {
            "source_definition_id": "sciml-scimlsensitivity/brusselator.md@fea9aaa141f224a97f112e024082966a1a5ee6c2",
            "source_repository": "SciML/SciMLSensitivity.jl",
            "source_revision": "63a13a7301a17feb8cb5e3a4b3ccef4487ae0c52",
            "source_path": "docs/src/examples/pde/brusselator.md",
            "source_blob": "fea9aaa141f224a97f112e024082966a1a5ee6c2",
            "source_sha256": "688e4642b669e4181cca67d0d7cd9d663e2322d70923daf0240e5a995627351e",
        },
    },
}


def requested_times(problem: dict[str, Any]) -> list[float]:
    """Use D's scalar `t0 + (tf-t0)*index/100` operation order exactly."""
    t0, tf = problem["t_span"]
    times = [t0 + (tf - t0) * float(index) / 100.0 for index in range(101)]
    for breakpoint in problem["mandatory_breakpoints"]:
        if not any(same_bits(time, breakpoint) for time in times):
            times.append(float(breakpoint))
    times.sort()
    return times


def validate_problem(problem: dict[str, Any]) -> None:
    expected = HOLDOUTS.get(problem.get("family"))
    require(expected is not None, "unknown holdout family")
    require(problem.get("problem_id") == expected["problem_id"], "problem id pin mismatch")
    require(problem.get("dimension") == expected["dimension"], "dimension pin mismatch")
    require(problem.get("uniform_output_points") == 101, "requires exactly 101 uniform outputs")
    require(
        len(problem.get("t_span", [])) == 2
        and all(same_bits(left, right) for left, right in zip(problem["t_span"], expected["t_span"])),
        "time span pin mismatch",
    )
    require(
        len(problem.get("mandatory_breakpoints", [])) == len(expected["mandatory_breakpoints"])
        and all(
            same_bits(left, right)
            for left, right in zip(problem["mandatory_breakpoints"], expected["mandatory_breakpoints"])
        ),
        "breakpoint pin mismatch",
    )
    require(problem.get("source") == expected["source"], "source-definition pin mismatch")


def validate_method(method: dict[str, Any], label: str, solver: str) -> None:
    require(method.get("label") == label and method.get("method") == solver, "method pin mismatch")
    require(
        isinstance(method.get("rtol"), (float, int))
        and isinstance(method.get("atol"), (float, int))
        and math.isfinite(float(method["rtol"]))
        and math.isfinite(float(method["atol"]))
        and method["rtol"] > 0.0
        and method["atol"] > 0.0,
        "method tolerance must be finite and positive",
    )


def validate_manifest(manifest: dict[str, Any], require_checksums: bool) -> None:
    require(manifest.get("schema_version") == MANIFEST_SCHEMA, "manifest schema mismatch")
    require(manifest.get("generation_mode") == GENERATION_MODE, "manifest generation mode mismatch")
    generator = manifest.get("generator")
    require(isinstance(generator, dict), "missing generator pins")
    for name, value in EXPECTED_RUNTIME.items():
        require(generator.get(name) == value, f"runtime pin mismatch: {name}")
    require(generator.get("blas_threads") == 1, "BLAS thread pin mismatch")
    ladder = generator.get("radau_ladder")
    require(isinstance(ladder, list) and len(ladder) == 3, "requires L0/L1/L2 Radau ladder")
    for label, method in zip(("L0", "L1", "L2"), ladder):
        validate_method(method, label, "Radau")
    require(
        all(
            ladder[index + 1]["rtol"] < ladder[index]["rtol"]
            and ladder[index + 1]["atol"] < ladder[index]["atol"]
            for index in range(2)
        ),
        "Radau ladder must become strictly tighter",
    )
    require(ladder[2]["rtol"] <= 1.0e-10 and ladder[2]["atol"] <= 1.0e-12, "L2 is not tight enough")
    validate_method(generator.get("tight_lsoda", {}), "tight-lsoda", "LSODA")
    entries = manifest.get("artifacts")
    require(isinstance(entries, list) and len(entries) == 4, "manifest must contain four artifacts")
    families = set()
    for entry in entries:
        require(isinstance(entry, dict), "manifest entry is not an object")
        validate_problem(entry.get("problem", {}))
        family = entry["problem"]["family"]
        require(family not in families, "duplicate manifest holdout")
        families.add(family)
        path = Path(entry.get("artifact_path", ""))
        require(not path.is_absolute() and ".." not in path.parts, "artifact path must be manifest-relative")
        require(entry.get("canonical_method") == ladder[2], "canonical source must be tight Radau L2")
        if require_checksums:
            for field in ("artifact_sha256", "grid_sha256", "state_sha256"):
                value = entry.get(field)
                require(
                    isinstance(value, str) and len(value) == 64 and all(char in "0123456789abcdef" for char in value),
                    f"invalid manifest {field}",
                )
    require(families == set(HOLDOUTS), "manifest holdout set differs from ScientificCorpusV2")
    if require_checksums:
        require(
            manifest.get("artifact_set_sha256") == artifact_set_checksum(entries),
            "aggregate artifact-set checksum mismatch",
        )


def assert_close(left: float, right: float, message: str) -> None:
    scale = max(abs(left), abs(right), 1.0)
    require(abs(left - right) <= 128.0 * sys.float_info.epsilon * scale, message)


def validate_convergence(convergence: dict[str, Any]) -> None:
    fields = (
        "d0_max_grid_wrms",
        "d1_max_grid_wrms",
        "q",
        "richardson_uncertainty_wrms",
        "method_disagreement_wrms",
        "reference_uncertainty_wrms",
    )
    require(all(math.isfinite(float(convergence.get(field, math.nan))) for field in fields), "nonfinite convergence field")
    d0 = float(convergence["d0_max_grid_wrms"])
    d1 = float(convergence["d1_max_grid_wrms"])
    q = float(convergence["q"])
    require(d0 > d1 >= 0.0, "requires D0 > D1 >= 0")
    require(0.0 <= q <= 0.5, "requires finite q <= 0.5")
    assert_close(q, d1 / d0, "q != D1/D0")
    richardson = d1 * q / (1.0 - q)
    assert_close(float(convergence["richardson_uncertainty_wrms"]), richardson, "bad Richardson uncertainty")
    require(float(convergence["method_disagreement_wrms"]) >= 0.0, "negative method disagreement")
    assert_close(
        float(convergence["reference_uncertainty_wrms"]),
        richardson + float(convergence["method_disagreement_wrms"]),
        "bad total reference uncertainty",
    )
    scale = convergence.get("wrms_scale", {})
    require(
        scale == WRMS_SCALE,
        "WRMS scale differs from the declared fixed scale",
    )


def validate_artifact(
    artifact: dict[str, Any], entry: dict[str, Any], generator: dict[str, Any]
) -> None:
    require(artifact.get("schema_version") == ARTIFACT_SCHEMA, "artifact schema mismatch")
    require(artifact.get("problem") == entry["problem"], "artifact problem identity mismatch")
    validate_problem(artifact["problem"])
    require(artifact.get("canonical_method") == generator["radau_ladder"][2], "artifact canonical method mismatch")
    require(artifact.get("independent_method") == generator["tight_lsoda"], "artifact LSODA method mismatch")
    expected_times = requested_times(artifact["problem"])
    times = artifact.get("requested_times")
    require(isinstance(times, list) and len(times) == len(expected_times), "requested time count mismatch")
    require(
        all(same_bits(left, right) for left, right in zip(times, expected_times)),
        "requested times missing, reordered, or altered",
    )
    require(
        all(math.isfinite(float(time)) for time in times)
        and all(times[index] < times[index + 1] for index in range(len(times) - 1)),
        "requested times must be finite and strictly increasing",
    )
    for breakpoint in artifact["problem"]["mandatory_breakpoints"]:
        require(any(same_bits(time, breakpoint) for time in times), "mandatory breakpoint absent")
    states = artifact.get("states")
    require(isinstance(states, list) and len(states) == len(times), "state/grid length mismatch")
    dimension = artifact["problem"]["dimension"]
    require(
        all(
            isinstance(state, list)
            and len(state) == dimension
            and all(math.isfinite(float(value)) for value in state)
            for state in states
        ),
        "state shape or finiteness mismatch",
    )
    checksums = artifact.get("checksums", {})
    require(checksums.get("grid_sha256") == grid_checksum(times), "grid checksum mismatch")
    require(checksums.get("state_sha256") == state_checksum(states), "state checksum mismatch")
    require(entry["grid_sha256"] == checksums["grid_sha256"], "manifest grid checksum mismatch")
    require(entry["state_sha256"] == checksums["state_sha256"], "manifest state checksum mismatch")
    validate_convergence(artifact.get("convergence", {}))


def oregonator_rhs(_time: float, y: np.ndarray) -> np.ndarray:
    s = 77.27
    q = 8.375e-6
    w = 0.161
    return np.array(
        [
            s * (y[1] + y[0] * (1.0 - q * y[0] - y[1])),
            (y[2] - (1.0 + y[0]) * y[1]) / s,
            w * (y[0] - y[2]),
        ],
        dtype=np.float64,
    )


def oregonator_jac(_time: float, y: np.ndarray) -> np.ndarray:
    s = 77.27
    q = 8.375e-6
    w = 0.161
    return np.array(
        [
            [s * (1.0 - 2.0 * q * y[0] - y[1]), s * (1.0 - y[0]), 0.0],
            [-y[1] / s, -(1.0 + y[0]) / s, 1.0 / s],
            [w, 0.0, -w],
        ],
        dtype=np.float64,
    )


POLLUTION_K = np.array(
    [
        0.35,
        26.6,
        12300.0,
        8.6e-4,
        8.2e-4,
        15000.0,
        1.3e-4,
        24000.0,
        16500.0,
        9000.0,
        0.022,
        12000.0,
        1.88,
        16300.0,
        4.8e6,
        3.5e-4,
        0.0175,
        1.0e8,
        4.44e11,
        1240.0,
        2.1,
        5.78,
        0.0474,
        1780.0,
        3.12,
    ],
    dtype=np.float64,
)


def pollution_rates(y: np.ndarray) -> np.ndarray:
    k = POLLUTION_K
    return np.array(
        [
            k[0] * y[0],
            k[1] * y[1] * y[3],
            k[2] * y[4] * y[1],
            k[3] * y[6],
            k[4] * y[6],
            k[5] * y[6] * y[5],
            k[6] * y[8],
            k[7] * y[8] * y[5],
            k[8] * y[10] * y[1],
            k[9] * y[10] * y[0],
            k[10] * y[12],
            k[11] * y[9] * y[1],
            k[12] * y[13],
            k[13] * y[0] * y[5],
            k[14] * y[2],
            k[15] * y[3],
            k[16] * y[3],
            k[17] * y[15],
            k[18] * y[15],
            k[19] * y[16] * y[5],
            k[20] * y[18],
            k[21] * y[18],
            k[22] * y[0] * y[3],
            k[23] * y[18] * y[0],
            k[24] * y[19],
        ],
        dtype=np.float64,
    )


def pollution_rate_derivatives(y: np.ndarray, v: np.ndarray) -> np.ndarray:
    k = POLLUTION_K
    return np.array(
        [
            k[0] * v[0],
            k[1] * (v[1] * y[3] + y[1] * v[3]),
            k[2] * (v[4] * y[1] + y[4] * v[1]),
            k[3] * v[6],
            k[4] * v[6],
            k[5] * (v[6] * y[5] + y[6] * v[5]),
            k[6] * v[8],
            k[7] * (v[8] * y[5] + y[8] * v[5]),
            k[8] * (v[10] * y[1] + y[10] * v[1]),
            k[9] * (v[10] * y[0] + y[10] * v[0]),
            k[10] * v[12],
            k[11] * (v[9] * y[1] + y[9] * v[1]),
            k[12] * v[13],
            k[13] * (v[0] * y[5] + y[0] * v[5]),
            k[14] * v[2],
            k[15] * v[3],
            k[16] * v[3],
            k[17] * v[15],
            k[18] * v[15],
            k[19] * (v[16] * y[5] + y[16] * v[5]),
            k[20] * v[18],
            k[21] * v[18],
            k[22] * (v[0] * y[3] + y[0] * v[3]),
            k[23] * (v[18] * y[0] + y[18] * v[0]),
            k[24] * v[19],
        ],
        dtype=np.float64,
    )


def assemble_pollution(r: np.ndarray) -> np.ndarray:
    return np.array(
        [
            -r[0] - r[9] - r[13] - r[22] - r[23] + r[1] + r[2] + r[8] + r[10] + r[11] + r[21] + r[24],
            -r[1] - r[2] - r[8] - r[11] + r[0] + r[20],
            -r[14] + r[0] + r[16] + r[18] + r[21],
            -r[1] - r[15] - r[16] - r[22] + r[14],
            -r[2] + 2.0 * r[3] + r[5] + r[6] + r[12] + r[19],
            -r[5] - r[7] - r[13] - r[19] + r[2] + 2.0 * r[17],
            -r[3] - r[4] - r[5] + r[12],
            r[3] + r[4] + r[5] + r[6],
            -r[6] - r[7],
            -r[11] + r[6] + r[8],
            -r[8] - r[9] + r[7] + r[10],
            r[8],
            -r[10] + r[9],
            -r[12] + r[11],
            r[13],
            -r[17] - r[18] + r[15],
            -r[19],
            r[19],
            -r[20] - r[21] - r[23] + r[22] + r[24],
            -r[24] + r[23],
        ],
        dtype=np.float64,
    )


def pollution_rhs(_time: float, y: np.ndarray) -> np.ndarray:
    return assemble_pollution(pollution_rates(y))


def pollution_jac(_time: float, y: np.ndarray) -> np.ndarray:
    jacobian = np.empty((20, 20), dtype=np.float64)
    direction = np.zeros(20, dtype=np.float64)
    for column in range(20):
        direction[column] = 1.0
        jacobian[:, column] = assemble_pollution(pollution_rate_derivatives(y, direction))
        direction[column] = 0.0
    return jacobian


MEDICAL_N = 200
MEDICAL_H = 0.005
MEDICAL_K = 100.0
MEDICAL_C = 4.0


def medical_coefficients(index: int) -> tuple[float, float]:
    zeta = float(index + 1) * MEDICAL_H
    a = 2.0 * (zeta - 1.0) ** 3 / (MEDICAL_C * MEDICAL_C)
    b = (zeta - 1.0) ** 4 / (MEDICAL_C * MEDICAL_C)
    return a, b


def medical_rhs_with_phi(y: np.ndarray, phi: float) -> np.ndarray:
    out = np.empty(2 * MEDICAL_N, dtype=np.float64)
    for index in range(MEDICAL_N):
        offset = 2 * index
        u = y[offset]
        v = y[offset + 1]
        um = phi if index == 0 else y[offset - 2]
        up = u if index + 1 == MEDICAL_N else y[offset + 2]
        a, b = medical_coefficients(index)
        reaction = MEDICAL_K * u * v
        out[offset] = a * (up - um) / (2.0 * MEDICAL_H) + b * (um - 2.0 * u + up) / (MEDICAL_H * MEDICAL_H) - reaction
        out[offset + 1] = -reaction
    return out


def medical_rhs(time: float, y: np.ndarray) -> np.ndarray:
    return medical_rhs_with_phi(y, 2.0 if time <= 5.0 else 0.0)


def medical_jac_sparse(_time: float, y: np.ndarray) -> csc_matrix:
    rows: list[int] = []
    columns: list[int] = []
    values: list[float] = []
    for index in range(MEDICAL_N):
        offset = 2 * index
        a, b = medical_coefficients(index)
        u = y[offset]
        v = y[offset + 1]
        diagonal_u = -2.0 * b / (MEDICAL_H * MEDICAL_H) - MEDICAL_K * v
        if index + 1 == MEDICAL_N:
            diagonal_u += a / (2.0 * MEDICAL_H) + b / (MEDICAL_H * MEDICAL_H)
        rows.extend((offset, offset, offset + 1, offset + 1))
        columns.extend((offset, offset + 1, offset, offset + 1))
        values.extend((diagonal_u, -MEDICAL_K * u, -MEDICAL_K * v, -MEDICAL_K * u))
        if index > 0:
            rows.append(offset)
            columns.append(offset - 2)
            values.append(-a / (2.0 * MEDICAL_H) + b / (MEDICAL_H * MEDICAL_H))
        if index + 1 < MEDICAL_N:
            rows.append(offset)
            columns.append(offset + 2)
            values.append(a / (2.0 * MEDICAL_H) + b / (MEDICAL_H * MEDICAL_H))
    return csc_matrix((values, (rows, columns)), shape=(2 * MEDICAL_N, 2 * MEDICAL_N))


def medical_jac_banded(time: float, y: np.ndarray) -> np.ndarray:
    sparse = medical_jac_sparse(time, y).tocoo()
    lower = upper = 2
    packed = np.zeros((lower + upper + 1, 2 * MEDICAL_N), dtype=np.float64)
    for row, column, value in zip(sparse.row, sparse.col, sparse.data):
        packed[upper + row - column, column] = value
    return packed


BRUSSELATOR_SIDE = 16
BRUSSELATOR_PLANE = BRUSSELATOR_SIDE * BRUSSELATOR_SIDE
BRUSSELATOR_A = 3.4
BRUSSELATOR_B = 1.0
BRUSSELATOR_ALPHA = 10.0
BRUSSELATOR_H = 1.0 / 15.0


def brusselator_offset(i: int, j: int) -> int:
    return i + BRUSSELATOR_SIDE * j


BRUSSELATOR_FORCING_MASK = np.array(
    [
        [
            (float(i) * BRUSSELATOR_H - 0.3) ** 2
            + (float(j) * BRUSSELATOR_H - 0.6) ** 2
            <= 0.01
            for i in range(BRUSSELATOR_SIDE)
        ]
        for j in range(BRUSSELATOR_SIDE)
    ],
    dtype=np.bool_,
)


def brusselator_rhs_with_forcing(y: np.ndarray, forcing_enabled: bool) -> np.ndarray:
    # `index = i + 16*j` is C-order `(j, i)` storage.  The operation sequence below is the
    # source's left + right + down + up - 4*center stencil, evaluated componentwise in f64.
    u = y[:BRUSSELATOR_PLANE].reshape((BRUSSELATOR_SIDE, BRUSSELATOR_SIDE))
    v = y[BRUSSELATOR_PLANE:].reshape((BRUSSELATOR_SIDE, BRUSSELATOR_SIDE))
    diffusion = BRUSSELATOR_ALPHA / (BRUSSELATOR_H * BRUSSELATOR_H)
    lap_u = np.roll(u, 1, axis=1) + np.roll(u, -1, axis=1)
    lap_u += np.roll(u, 1, axis=0)
    lap_u += np.roll(u, -1, axis=0)
    lap_u -= 4.0 * u
    lap_v = np.roll(v, 1, axis=1) + np.roll(v, -1, axis=1)
    lap_v += np.roll(v, 1, axis=0)
    lap_v += np.roll(v, -1, axis=0)
    lap_v -= 4.0 * v
    uv2 = u * u * v
    forcing = 5.0 * BRUSSELATOR_FORCING_MASK if forcing_enabled else 0.0
    out = np.empty(2 * BRUSSELATOR_PLANE, dtype=np.float64)
    out[:BRUSSELATOR_PLANE] = (
        diffusion * lap_u + BRUSSELATOR_B + uv2 - (BRUSSELATOR_A + 1.0) * u + forcing
    ).ravel()
    out[BRUSSELATOR_PLANE:] = (diffusion * lap_v + BRUSSELATOR_A * u - uv2).ravel()
    return out


def brusselator_rhs(time: float, y: np.ndarray) -> np.ndarray:
    return brusselator_rhs_with_forcing(y, time >= 1.1)


def brusselator_jac_sparse(_time: float, y: np.ndarray) -> csc_matrix:
    rows: list[int] = []
    columns: list[int] = []
    values: list[float] = []
    diffusion = BRUSSELATOR_ALPHA / (BRUSSELATOR_H * BRUSSELATOR_H)
    for j in range(BRUSSELATOR_SIDE):
        jm = BRUSSELATOR_SIDE - 1 if j == 0 else j - 1
        jp = 0 if j + 1 == BRUSSELATOR_SIDE else j + 1
        for i in range(BRUSSELATOR_SIDE):
            im = BRUSSELATOR_SIDE - 1 if i == 0 else i - 1
            ip = 0 if i + 1 == BRUSSELATOR_SIDE else i + 1
            index = brusselator_offset(i, j)
            u = y[index]
            v = y[BRUSSELATOR_PLANE + index]
            u_row = index
            v_row = BRUSSELATOR_PLANE + index
            for neighbor in (
                brusselator_offset(im, j),
                brusselator_offset(ip, j),
                brusselator_offset(i, jm),
                brusselator_offset(i, jp),
            ):
                rows.extend((u_row, v_row))
                columns.extend((neighbor, BRUSSELATOR_PLANE + neighbor))
                values.extend((diffusion, diffusion))
            rows.extend((u_row, u_row, v_row, v_row))
            columns.extend((index, BRUSSELATOR_PLANE + index, index, BRUSSELATOR_PLANE + index))
            values.extend(
                (
                    -4.0 * diffusion + 2.0 * u * v - BRUSSELATOR_A - 1.0,
                    u * u,
                    BRUSSELATOR_A - 2.0 * u * v,
                    -4.0 * diffusion - u * u,
                )
            )
    return csc_matrix((values, (rows, columns)), shape=(2 * BRUSSELATOR_PLANE, 2 * BRUSSELATOR_PLANE))


def brusselator_jac_dense(time: float, y: np.ndarray) -> np.ndarray:
    return brusselator_jac_sparse(time, y).toarray()


def initial_state(family: str) -> np.ndarray:
    if family == "oregonator":
        return np.array([1.0, 2.0, 3.0], dtype=np.float64)
    if family == "pollution":
        return np.array(
            [0.0, 0.2, 0.0, 0.04, 0.0, 0.0, 0.1, 0.3, 0.01, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.007, 0.0, 0.0, 0.0],
            dtype=np.float64,
        )
    if family == "medical-akzo":
        state = np.zeros(2 * MEDICAL_N, dtype=np.float64)
        state[1::2] = 1.0
        return state
    if family == "brusselator-2d":
        state = np.zeros(2 * BRUSSELATOR_PLANE, dtype=np.float64)
        for j in range(BRUSSELATOR_SIDE):
            yy = float(j) * BRUSSELATOR_H
            for i in range(BRUSSELATOR_SIDE):
                x = float(i) * BRUSSELATOR_H
                index = brusselator_offset(i, j)
                state[index] = 22.0 * (yy * (1.0 - yy)) ** 1.5
                state[BRUSSELATOR_PLANE + index] = 27.0 * (x * (1.0 - x)) ** 1.5
        return state
    raise ReferenceValidationError(f"no initial state for {family}")


def rhs_and_jacobians(family: str) -> tuple[Callable[..., np.ndarray], Callable[..., Any], Callable[..., Any] | None, tuple[int, int] | None]:
    if family == "oregonator":
        return oregonator_rhs, oregonator_jac, oregonator_jac, None
    if family == "pollution":
        return pollution_rhs, pollution_jac, pollution_jac, None
    if family == "medical-akzo":
        # LSODA estimates its banded Jacobian independently; the sparse analytic Jacobian remains
        # required for the tight Radau lane.
        return medical_rhs, medical_jac_sparse, None, (2, 2)
    if family == "brusselator-2d":
        # Keep the independent LSODA lane derivative-independent; Radau uses sparse analytic J.
        return brusselator_rhs, brusselator_jac_sparse, None, None
    raise ReferenceValidationError(f"no equation definition for {family}")


def solve_piecewise(
    problem: dict[str, Any],
    method: dict[str, Any],
    rhs: Callable[..., np.ndarray],
    radau_jacobian: Callable[..., Any],
    lsoda_jacobian: Callable[..., Any] | None,
    lsoda_band: tuple[int, int] | None,
) -> np.ndarray:
    all_times = requested_times(problem)
    t0, tf = problem["t_span"]
    bounds = [t0, *problem["mandatory_breakpoints"], tf]
    state = initial_state(problem["family"])
    output: list[np.ndarray] = []
    output_times: list[float] = []
    for segment_index, (start, end) in enumerate(zip(bounds, bounds[1:])):
        segment_times = [time for time in all_times if start <= time <= end]
        require(segment_times and same_bits(segment_times[-1], end), "piece has no exact endpoint output")
        options: dict[str, Any] = {
            "method": method["method"],
            "rtol": method["rtol"],
            "atol": method["atol"],
            "t_eval": np.asarray(segment_times, dtype=np.float64),
        }
        if method["method"] == "Radau":
            options["jac"] = radau_jacobian
        else:
            require(method["method"] == "LSODA", "unexpected independent solver")
            if lsoda_jacobian is not None:
                options["jac"] = lsoda_jacobian
            if lsoda_band is not None:
                options["lband"], options["uband"] = lsoda_band
        # A collocation stage at a piece endpoint must not sample the opposite side of a jump.
        # The endpoint state is propagated once and emitted once; the following piece starts from
        # that continuous state under its right-hand branch.
        if problem["family"] == "medical-akzo":
            segment_rhs = lambda _time, values, phi=2.0 if segment_index == 0 else 0.0: medical_rhs_with_phi(values, phi)
        elif problem["family"] == "brusselator-2d":
            segment_rhs = lambda _time, values, enabled=segment_index != 0: brusselator_rhs_with_forcing(values, enabled)
        else:
            segment_rhs = rhs
        result = solve_ivp(segment_rhs, (start, end), state, **options)
        require(result.success, f"{problem['family']} {method['label']} failed: {result.message}")
        require(result.y.shape[1] == len(segment_times), "solver omitted requested output")
        state = result.y[:, -1].copy()
        start_index = 0 if segment_index == 0 else 1
        output.extend(result.y[:, start_index:].T.copy())
        output_times.extend(segment_times[start_index:])
    require(
        len(output_times) == len(all_times)
        and all(same_bits(left, right) for left, right in zip(output_times, all_times)),
        "piecewise integration changed requested grid",
    )
    result = np.asarray(output, dtype=np.float64)
    require(result.shape == (len(all_times), problem["dimension"]), "unexpected solution table shape")
    require(np.isfinite(result).all(), "solver produced nonfinite canonical state")
    return result


def source_spot_assertions() -> None:
    """Cheap source-definition guards that are independent of any numerical artifact."""
    forced_indices = np.flatnonzero(BRUSSELATOR_FORCING_MASK.ravel()).tolist()
    require(
        forced_indices == [132, 133, 147, 148, 149, 164, 165],
        "Brusselator source forcing mask differs from the pinned seven-point definition",
    )
    trial = np.zeros(2 * MEDICAL_N, dtype=np.float64)
    require(
        not np.array_equal(medical_rhs_with_phi(trial, 2.0), medical_rhs_with_phi(trial, 0.0)),
        "Medical Akzo source jump branches are not distinct",
    )
    validate_semilinear_oracle(SEMILINEAR_ORACLE_PATH)


def semilinear_grid_shape(dimension: int) -> tuple[int, int]:
    grids = {96: (8, 12), 384: (16, 24), 1536: (32, 48)}
    require(dimension in grids, f"no scientific-corpus-v2.1 grid for dimension {dimension}")
    return grids[dimension]


def semilinear_exact_state(nx: int, ny: int, time: float) -> np.ndarray:
    hx = 1.0 / float(nx + 1)
    hy = 1.0 / float(ny + 1)
    decay = math.exp(-time)
    state = np.empty(nx * ny, dtype=np.float64)
    for j in range(ny):
        # Match the Rust callback's operation association exactly.  This bit
        # identity matters because the tight L2 trajectory is also the sole
        # WRMS scale anchor in the v2 reference wire format.
        y = float(j + 1) * hy
        sy = math.sin(math.pi * y)
        for i in range(nx):
            x = float(i + 1) * hx
            state[i + nx * j] = decay * math.sin(math.pi * x) * sy
    return state


def semilinear_ramp(time: float) -> tuple[float, float]:
    value = math.tanh((time - 0.5) / 0.08)
    return 0.5 * (1.0 + value), 0.5 * (1.0 - value * value) / 0.08


def semilinear_operator(nx: int, ny: int, time: float, values: np.ndarray) -> np.ndarray:
    hx = 1.0 / float(nx + 1)
    hy = 1.0 / float(ny + 1)
    ramp, _ = semilinear_ramp(time)
    advection = 0.5 + 3.5 * ramp
    out = np.empty(nx * ny, dtype=np.float64)
    for j in range(ny):
        for i in range(nx):
            index = i + nx * j
            center = float(values[index])
            left = 0.0 if i == 0 else float(values[index - 1])
            right = 0.0 if i + 1 == nx else float(values[index + 1])
            down = 0.0 if j == 0 else float(values[index - nx])
            up = 0.0 if j + 1 == ny else float(values[index + nx])
            laplacian = (left - 2.0 * center + right) / (hx * hx)
            laplacian += (down - 2.0 * center + up) / (hy * hy)
            backward = (center - left) / hx + (center - down) / hy
            out[index] = 0.002 * laplacian - advection * backward - center
    return out


def semilinear_rhs(nx: int, ny: int, time: float, state: np.ndarray) -> np.ndarray:
    phi = semilinear_exact_state(nx, ny, time)
    defect = state - phi
    ramp, _ = semilinear_ramp(time)
    nonlinear = 2.0 + 48.0 * ramp
    return semilinear_operator(nx, ny, time, defect) - phi + nonlinear * (state * state - phi * phi)


def semilinear_jacobian(nx: int, ny: int, time: float, state: np.ndarray) -> csc_matrix:
    hx = 1.0 / float(nx + 1)
    hy = 1.0 / float(ny + 1)
    ramp, _ = semilinear_ramp(time)
    advection = 0.5 + 3.5 * ramp
    nonlinear = 2.0 + 48.0 * ramp
    rows: list[int] = []
    columns: list[int] = []
    values: list[float] = []
    for j in range(ny):
        for i in range(nx):
            index = i + nx * j
            rows.append(index)
            columns.append(index)
            values.append(
                -0.004 / (hx * hx)
                - 0.004 / (hy * hy)
                - advection / hx
                - advection / hy
                - 1.0
                + 2.0 * nonlinear * float(state[index])
            )
            if i > 0:
                rows.append(index)
                columns.append(index - 1)
                values.append(0.002 / (hx * hx) + advection / hx)
            if i + 1 < nx:
                rows.append(index)
                columns.append(index + 1)
                values.append(0.002 / (hx * hx))
            if j > 0:
                rows.append(index)
                columns.append(index - nx)
                values.append(0.002 / (hy * hy) + advection / hy)
            if j + 1 < ny:
                rows.append(index)
                columns.append(index + nx)
                values.append(0.002 / (hy * hy))
    return csc_matrix((values, (rows, columns)), shape=(nx * ny, nx * ny))


def semilinear_partial_t(nx: int, ny: int, time: float, state: np.ndarray) -> np.ndarray:
    hx = 1.0 / float(nx + 1)
    hy = 1.0 / float(ny + 1)
    ramp, dramp = semilinear_ramp(time)
    phi = semilinear_exact_state(nx, ny, time)
    defect = state - phi
    dadvection = 3.5 * dramp
    operator_t = np.empty(nx * ny, dtype=np.float64)
    for j in range(ny):
        for i in range(nx):
            index = i + nx * j
            left = 0.0 if i == 0 else float(defect[index - 1])
            down = 0.0 if j == 0 else float(defect[index - nx])
            operator_t[index] = -dadvection * (
                (float(defect[index]) - left) / hx + (float(defect[index]) - down) / hy
            )
    nonlinear = 2.0 + 48.0 * ramp
    dnonlinear = 48.0 * dramp
    return (
        operator_t
        + semilinear_operator(nx, ny, time, phi)
        + phi
        + dnonlinear * (state * state - phi * phi)
        + 2.0 * nonlinear * phi * phi
    )


def validate_semilinear_oracle(path: Path) -> None:
    oracle = read_json(path)
    require(
        oracle.get("schema_version") == "scientific-corpus-v2.1-semilinear-mpmath-oracle-v1",
        "semilinear oracle schema mismatch",
    )
    require(
        oracle.get("origin") == {"engine": "mpmath", "version": "1.3.0", "decimal_digits": 80},
        "semilinear oracle origin mismatch",
    )
    cases = oracle.get("cases")
    require(isinstance(cases, list) and len(cases) == 3, "semilinear oracle must contain three grids")
    seen: set[int] = set()
    for expected in cases:
        dimension = int(expected["dimension"])
        require(dimension not in seen, "duplicate semilinear oracle dimension")
        seen.add(dimension)
        nx, ny = semilinear_grid_shape(dimension)
        require([nx, ny] == [expected["nx"], expected["ny"]], "semilinear oracle grid mismatch")
        time = float(expected["time"])
        phi = semilinear_exact_state(nx, ny, time)
        state = phi + np.asarray(
            [0.01 * math.cos(0.17 * float(index + 1)) for index in range(dimension)],
            dtype=np.float64,
        )
        direction = np.asarray(
            [math.sin(0.23 * float(index + 1)) for index in range(dimension)],
            dtype=np.float64,
        )
        rhs_at_phi = semilinear_rhs(nx, ny, time, phi)
        rhs = semilinear_rhs(nx, ny, time, state)
        jacobian = semilinear_jacobian(nx, ny, time, state)
        jvp = jacobian @ direction
        partial_t = semilinear_partial_t(nx, ny, time, state)
        coo = jacobian.tocoo()
        bandwidth = int(np.max(np.abs(coo.row - coo.col)))
        require(bandwidth == expected["expected_half_bandwidth"], "semilinear Jacobian bandwidth mismatch")
        require(
            float(jacobian[nx - 1, nx]) == float(expected["seam_cross_jacobian"]),
            "semilinear Jacobian crosses an x seam",
        )
        for sample in expected["samples"]:
            index = int(sample["index"])
            actual_bits = struct.unpack(">Q", struct.pack(">d", float(phi[index])))[0]
            require(
                actual_bits == int(sample["phi_f64_bits"]),
                f"semilinear {dimension} exact-state bit identity mismatch at {index}",
            )
            comparisons = (
                (phi[index], sample["phi"], "phi"),
                (rhs_at_phi[index], sample["rhs_at_phi"], "rhs(phi)"),
                (rhs[index], sample["rhs_perturbed"], "rhs(perturbed)"),
                (jvp[index], sample["jvp"], "Jv"),
                (partial_t[index], sample["partial_t"], "partial_t"),
            )
            for actual, decimal, label in comparisons:
                expected_value = float(decimal)
                scaled_error = abs(float(actual) - expected_value) / max(1.0, abs(expected_value))
                require(
                    scaled_error <= 5.0e-14,
                    f"semilinear {dimension} {label} differs from shared mpmath oracle",
                )
    require(seen == {96, 384, 1536}, "semilinear oracle dimension set mismatch")


def max_grid_wrms(left: np.ndarray, right: np.ndarray) -> float:
    weights = WRMS_SCALE["absolute"] + WRMS_SCALE["relative"] * np.maximum(np.abs(left), np.abs(right))
    values = np.sqrt(np.mean(np.square((left - right) / weights), axis=1))
    value = float(np.max(values))
    require(math.isfinite(value) and value >= 0.0, "nonfinite WRMS comparison")
    return value


def build_artifact(entry: dict[str, Any], generator: dict[str, Any]) -> dict[str, Any]:
    problem = entry["problem"]
    family = problem["family"]
    rhs, radau_jacobian, lsoda_jacobian, lsoda_band = rhs_and_jacobians(family)
    ladder = generator["radau_ladder"]
    levels = [
        solve_piecewise(problem, method, rhs, radau_jacobian, lsoda_jacobian, lsoda_band)
        for method in ladder
    ]
    independent = solve_piecewise(
        problem,
        generator["tight_lsoda"],
        rhs,
        radau_jacobian,
        lsoda_jacobian,
        lsoda_band,
    )
    d0 = max_grid_wrms(levels[0], levels[1])
    d1 = max_grid_wrms(levels[1], levels[2])
    q = d1 / d0
    require(math.isfinite(q) and q <= 0.5, f"{family}: q={q:.17g} violates q <= 0.5")
    richardson = d1 * q / (1.0 - q)
    disagreement = max_grid_wrms(levels[2], independent)
    convergence = {
        "d0_max_grid_wrms": d0,
        "d1_max_grid_wrms": d1,
        "q": q,
        "richardson_uncertainty_wrms": richardson,
        "method_disagreement_wrms": disagreement,
        "reference_uncertainty_wrms": richardson + disagreement,
        "wrms_scale": WRMS_SCALE,
    }
    times = requested_times(problem)
    states = [[float(value) for value in state] for state in levels[2]]
    artifact = {
        "schema_version": ARTIFACT_SCHEMA,
        "problem": problem,
        "requested_times": times,
        "states": states,
        "canonical_method": ladder[2],
        "independent_method": generator["tight_lsoda"],
        "convergence": convergence,
        "checksums": {
            "grid_sha256": grid_checksum(times),
            "state_sha256": state_checksum(states),
        },
    }
    validate_artifact(artifact, {**entry, **artifact["checksums"]}, generator)
    return artifact


def read_json(path: Path) -> dict[str, Any]:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ReferenceValidationError(f"cannot read {path}: {error}") from error


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, allow_nan=False, sort_keys=True, separators=(",", ":")) + "\n",
        encoding="utf-8",
    )


def generate(manifest_path: Path) -> None:
    require_runtime_pins()
    source_spot_assertions()
    manifest = read_json(manifest_path)
    validate_manifest(manifest, require_checksums=False)
    generated: list[tuple[dict[str, Any], dict[str, Any], bytes]] = []
    for entry in manifest["artifacts"]:
        artifact = build_artifact(entry, manifest["generator"])
        payload = (json.dumps(artifact, allow_nan=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")
        generated.append((entry, artifact, payload))
    for entry, artifact, payload in generated:
        artifact_path = manifest_path.parent / entry["artifact_path"]
        artifact_path.parent.mkdir(parents=True, exist_ok=True)
        artifact_path.write_bytes(payload)
        entry["artifact_sha256"] = sha256_bytes(payload)
        entry["grid_sha256"] = artifact["checksums"]["grid_sha256"]
        entry["state_sha256"] = artifact["checksums"]["state_sha256"]
    manifest["artifact_set_sha256"] = artifact_set_checksum(manifest["artifacts"])
    write_json(manifest_path, manifest)
    self_check(manifest_path)
    print(f"GENERATED {len(generated)} canonical numerical reference artifacts")


def self_check(manifest_path: Path) -> None:
    require_runtime_pins()
    source_spot_assertions()
    manifest = read_json(manifest_path)
    validate_manifest(manifest, require_checksums=True)
    for entry in manifest["artifacts"]:
        artifact_path = manifest_path.parent / entry["artifact_path"]
        try:
            payload = artifact_path.read_bytes()
        except OSError as error:
            raise ReferenceValidationError(f"missing artifact {artifact_path}: {error}") from error
        require(sha256_bytes(payload) == entry["artifact_sha256"], f"raw artifact checksum mismatch: {artifact_path}")
        try:
            artifact = json.loads(payload)
        except json.JSONDecodeError as error:
            raise ReferenceValidationError(f"invalid artifact JSON {artifact_path}: {error}") from error
        validate_artifact(artifact, entry, manifest["generator"])
    print(f"SELF_CHECK PASS {len(manifest['artifacts'])} canonical numerical reference artifacts")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--generate", action="store_true")
    mode.add_argument("--self-check", action="store_true")
    args = parser.parse_args()
    try:
        if args.generate:
            generate(args.manifest)
        else:
            self_check(args.manifest)
    except ReferenceValidationError as error:
        print(f"REFERENCE_VALIDATION_ERROR: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
