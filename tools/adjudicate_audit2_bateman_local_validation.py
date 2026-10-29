#!/usr/bin/env python3
"""Independently adjudicate a sealed Bateman package without a candidate run."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import pathlib
import platform
import pwd
import shutil
import subprocess
import sys
from typing import Any, Callable, Mapping, Sequence

PROTOCOL_VERSION = "vigilode-audit2-bateman-local-execution-orchestrator/v1"
EXECUTION_SOURCE_HEAD = "6b00a886c4eb38d3fe199e3d77852cc1eb35eb39"
EXECUTION_SOURCE_TREE = "4a9ede5c442514f1ae86d018419a2afeee5b6d01"
IMPLEMENTATION_HEAD = "cac7d1b7337a6dff25a60072009658f6ddf155d9"
IMPLEMENTATION_TREE = "c23abbee0d47e2dbe002e01516bf34e2481bc333"
BASE_HEAD = "f954e39130e5141256731d0745666a872c0267ea"
BASE_TREE = "4314da2f9e1533737d4169526ebd2d84515ab19d"

MANIFEST_PATH = "research/audit2_real_client_authority_construction_20260830/authority_manifest.json"
VERIFIER_PATH = "research/audit2_real_client_authority_construction_20260830/verify_authority_manifest.py"
PROOF_PATH = "research/audit2_real_client_authority_construction_20260830/evidence/AUTHORITY_VERIFICATION_RECEIPT.json"
VALIDATOR_PATH = "research/audit2_real_client_authority_construction_20260830/verify_local_six_case_receipt.py"
EXAMPLE_PATH = "crates/rodas5p-integrators/examples/audit2_bateman_local_six_case.rs"
READINESS_PATH = "tools/check-audit2-readiness.sh"
CODEX_START_PATH = "research/audit2_real_client_authority_construction_20260830/CODEX_START_HERE.md"
HANDOFF_PATH = "research/audit2_real_client_authority_construction_20260830/handoff.json"

EXPECTED_SOURCE_HASHES: dict[str, str] = {
    MANIFEST_PATH: "673045bf6b9e723fceb6a3b8df8e9e9e9075c942cf1c438f0ebd03574dbac360",
    VERIFIER_PATH: "542715ca749efbf2060d608f2089ee8457e32f9c61fd0d35f613d5ecec26487d",
    PROOF_PATH: "057cceba92fed0d707db1d586b53adebee5aed00583b224811d091f1d453ab12",
    VALIDATOR_PATH: "8391e03e6f94f305f2675799c923d787547cad662cef8d8f8384a8c1bbe94e67",
    EXAMPLE_PATH: "0873ed8189a7e0f77ebd4eef05ce6067f84958e7f118aa3e686654e7dc3c48f9",
    READINESS_PATH: "74dc27607ff1fc764e3ea89912b333418c86a1dfb5e3c14764481b94821b7521",
    "rust-toolchain.toml": "f53198ae4fdecfd87da36fe431c771b54c51e975d01c0e99f653bc14d5d48211",
    "Cargo.lock": "d9255cd442dfbca2890152549ae7edc60e890aa062a1046d8f0b8e44678d678a",
    "Cargo.toml": "86e27546665f923265a8addd3c464ac6017fe35558ab95fe0af7248cd99fb73b",
    CODEX_START_PATH: "ce96761b5cd067fe21e8d01e52a74767a65a8d9eaaa8c2c18ed1db8ca47de776",
    HANDOFF_PATH: "391861375a01a772e918aad28cfee887600b929cb7ed6b00b555a8bbc2aadb91",
}

EXPECTED_RUNNER_SHA256 = "f53f5bc2ea77721adc562c2640a58d24ae975f14795f7401c750c900c2980f29"
EXPECTED_PYTHON_VERSION = "3.12.13"
EXPECTED_NUMPY_VERSION = "2.3.5"
EXPECTED_MPMATH_VERSION = "1.3.0"
EXPECTED_RUST_VERSION = "1.94.1"

REPORT_SCHEMA = "vigilode-audit2-bateman-local-six-case-report/v1"
REPORT_CLAIM_SCOPE = "LOCAL_ONLY_EXPLORATORY_NONAUTHORITATIVE_REAL_CLIENT_VALIDATION"
CLIENT_ID = "bateman-two-timescale-parent-stable-daughter-v1"
GLOBAL_CLAIM_CEILING = (
    "EXPLORATORY_NONAUTHORITATIVE_REUSABLE_PRECONDITIONER_TRANSACTIONAL_STEP_SUBSTRATE"
)
CANONICAL_STATE_ROOT = (
    pathlib.Path(pwd.getpwuid(os.getuid()).pw_dir)
    / ".local/state/vigilode/bateman-local-six-case-v1"
)
ACCEPT_VERDICT = "ACCEPT_BOUNDED_EXACT_BATEMAN_SIX_SCENARIO_RECEIPT"
REJECT_VERDICT = "REJECT_BOUNDED_EXACT_BATEMAN_SIX_SCENARIO_CLAIM"
RUNNER_ACCEPT_VERDICT = (
    "ACCEPT_BOUNDED_EXACT_BATEMAN_CLIENT_MANIFEST_TWO_OPERATOR_CASES_SIX_SCENARIOS_ONLY"
)

EXPECTED_SCENARIOS = (
    ("same-live-context-reuse", "nominal-h1e-3", "same-live-context-cache-probe", "cache-reuse-observed"),
    ("changed-w-invalidation", "changed-w-h5e-4", "changed-w-cache-probe", "changed-w-invalidation-observed"),
    ("nominal-independent-budget", "nominal-h1e-3", "transactional-nominal", "candidate"),
    ("over-strict-budget-fallback", "nominal-h1e-3", "transactional-strict-fallback", "protected-fallback"),
    ("late-preconditioner-failure", "nominal-h1e-3", "transactional-late-apply-failure", "protected-fallback"),
    ("terminal-rejection", "nominal-h1e-3", "transactional-terminal-rejection", "rejected"),
)
CANDIDATE_ARGV = (
    "cargo",
    "run",
    "--locked",
    "-p",
    "rodas5p-integrators",
    "--features",
    "audit2-bateman-authority",
    "--example",
    "audit2_bateman_local_six_case",
)

EXPECTED_COMMANDS = tuple(
    [
        (f"source_pre_{suffix}", "source")
        for suffix in (
            "head",
            "tree",
            "parent",
            "implementation_tree",
            "implementation_parent",
            "base_tree",
            "execution_ancestry",
            "implementation_ancestry",
            "status",
        )
    ]
    + [
        ("python_dependencies", "environment"),
        ("rustc_version", "environment"),
        ("cargo_version", "environment"),
        ("authority_verification", "preflight"),
        ("python_authority_tests", "preflight"),
        ("python_local_receipt_tests", "preflight"),
        ("rust_authority_contracts", "preflight"),
        ("readiness", "preflight"),
        ("clippy", "preflight"),
        ("fmt", "preflight"),
        ("diff_check", "preflight"),
        ("cargo_build_candidate", "preflight"),
    ]
    + [
        (f"source_prelaunch_{suffix}", "source")
        for suffix in (
            "head",
            "tree",
            "parent",
            "implementation_tree",
            "implementation_parent",
            "base_tree",
            "execution_ancestry",
            "implementation_ancestry",
            "status",
        )
    ]
    + [("candidate", "candidate"), ("validator", "validator")]
    + [
        (f"source_post_{suffix}", "source")
        for suffix in (
            "head",
            "tree",
            "parent",
            "implementation_tree",
            "implementation_parent",
            "base_tree",
            "execution_ancestry",
            "implementation_ancestry",
            "status",
        )
    ]
)

PRESERVED_ENVIRONMENT = frozenset(
    {"USER", "LOGNAME", "TMPDIR"}
)
FIXED_ENVIRONMENT = {
    "LANG": "C",
    "LC_ALL": "C",
    "TZ": "UTC",
    "CARGO_NET_OFFLINE": "true",
    "CARGO_TERM_COLOR": "never",
    "PYTHONDONTWRITEBYTECODE": "1",
    "RUST_BACKTRACE": "0",
}

SCIENTIFIC_IMPLEMENTATION_PROVENANCE = {
    "execution_source_head": EXECUTION_SOURCE_HEAD,
    "execution_source_tree": EXECUTION_SOURCE_TREE,
    "implementation_head": IMPLEMENTATION_HEAD,
    "implementation_tree": IMPLEMENTATION_TREE,
    "implementation_parent": BASE_HEAD,
    "implementation_parent_tree": BASE_TREE,
}

EXPECTED_DECLARATIONS = {
    "execution_policy": "HOST_CODEX_ONLY",
    "local_llm_used": False,
    "local_llm_used_attestation": "SELF_DECLARED_NOT_HOST_ATTESTED",
    "holdout_access": "NOT_OPENED_OR_EXECUTED",
    "holdout_access_attestation": "SELF_DECLARED_AND_ABSENT_FROM_FIXED_COMMAND_PLAN",
    "remote_write": "NOT_PERFORMED_BY_RUNNER_COMMAND_PLAN",
    "remote_write_attestation": "FIXED_COMMAND_PLAN_ONLY_NOT_HOST_WIDE_ATTESTATION",
    "claim_ceiling": GLOBAL_CLAIM_CEILING,
}

FULL_MANIFEST_KEYS = frozenset(
    {
        "schema",
        "verdict",
        "protocol_version",
        "run_id",
        "utc_start",
        "utc_end",
        "source_pre",
        "source_prelaunch",
        "source_post",
        "source_hashes",
        "scientific_implementation_provenance",
        "tool_versions",
        "authority_preflight",
        "candidate",
        "validator",
        "pre_candidate_stop",
        "commands",
        "declarations",
        "math_blocker_coverage",
        "package",
    }
)
FULL_COMMAND_ROW_KEYS = frozenset(
    {
        "ordinal",
        "name",
        "phase",
        "argv",
        "cwd",
        "environment_overrides",
        "stdout_path",
        "stderr_path",
        "started_at_utc",
        "finished_at_utc",
        "launch_status",
        "exit_code",
        "stdout_sha256",
        "stderr_sha256",
    }
)
SOURCE_HASH_ROW_KEYS = frozenset({"expected", "observed", "match"})
CANDIDATE_SUMMARY_KEYS = frozenset(
    {
        "invocation_count",
        "launch_status",
        "exit_code",
        "argv",
        "result_summary_path",
        "result_summary_sha256",
        "stderr_path",
        "guard_key_sha256",
        "binary_sha256_prelaunch",
        "binary_sha256_post",
    }
)
VALIDATOR_SUMMARY_KEYS = frozenset(
    {
        "attempt_count",
        "launch_status",
        "exit_code",
        "argv",
        "stdout_path",
        "stdout_sha256",
        "stderr_path",
        "stderr_sha256",
        "parsed_status",
    }
)
TOOL_VERSION_KEYS = frozenset(
    {
        "executables",
        "platform",
        "controller",
        "environment_policy",
        "python",
        "rustc_version",
        "cargo_version",
        "candidate_binary",
    }
)
EXECUTABLE_IDENTITY_KEYS = frozenset(
    {"path", "realpath", "sha256", "size_bytes"}
)
PLATFORM_KEYS = frozenset({"system", "release", "machine", "python_runtime"})
CONTROLLER_KEYS = frozenset({"path", "sha256_pre", "sha256_post"})
ENVIRONMENT_POLICY_KEYS = frozenset(
    {"mode", "forbidden_ambient_detected", "cargo_config_files_present", "effective"}
)
CANDIDATE_BINARY_KEYS = frozenset(
    {"path", "sha256_prelaunch", "sha256_post", "size_bytes"}
)
ATTEMPT_LOCK_KEYS = frozenset(
    {
        "schema",
        "protocol_version",
        "key_sha256",
        "key_material",
        "created_at_utc",
        "run_id",
    }
)
ATTEMPT_KEY_MATERIAL_KEYS = frozenset(
    {
        "implementation_head",
        "authority_manifest_sha256",
        "example_target",
        "protocol_version",
    }
)
READINESS_OUTPUT_FILES = frozenset(
    {
        "readiness-output/solve-stiff.json",
        "readiness-output/solve-stiff-budget-exhausted.json",
    }
)


def expected_math_blocker_coverage(eligible_receipt: bool) -> dict[str, str]:
    return {
        "X01": (
            "EVALUATED_BY_ELIGIBLE_SEALED_RUN"
            if eligible_receipt
            else "NOT_ESTABLISHED"
        ),
        "M01": "PREEXISTING_MATHEMATICAL_AUTHORITY",
        "M02": (
            "PARTIALLY_EVALUABLE_FROM_COMPACT_RECEIPT"
            if eligible_receipt
            else "NOT_EVALUATED"
        ),
        "M03": (
            "PARTIALLY_EVALUABLE_IF_PER_SOLVE_TELEMETRY_PRESENT"
            if eligible_receipt
            else "NOT_EVALUATED"
        ),
        **{
            identifier: "NOT_EVALUATED"
            for identifier in (
                "M04",
                "M05",
                "M06",
                "M07",
                "M08",
                "M09",
                "M10",
                "M11",
                "M12",
                "X02",
            )
        },
    }


def expected_source_identity() -> dict[str, Any]:
    return {
        "head": EXECUTION_SOURCE_HEAD,
        "tree": EXECUTION_SOURCE_TREE,
        "parent": IMPLEMENTATION_HEAD,
        "implementation_tree": IMPLEMENTATION_TREE,
        "implementation_parent": BASE_HEAD,
        "base_tree": BASE_TREE,
        "execution_ancestry": True,
        "implementation_ancestry": True,
        "status": "",
        "valid": True,
    }


class AdjudicationInputError(ValueError):
    """An unsafe or ambiguous adjudication input."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def strict_json_bytes(value: bytes) -> Any:
    def pairs(items: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, item in items:
            if key in result:
                raise ValueError(f"duplicate JSON key: {key}")
            result[key] = item
        return result

    def constant(name: str) -> Any:
        raise ValueError(f"non-finite JSON constant: {name}")

    parsed = json.loads(value, object_pairs_hook=pairs, parse_constant=constant)
    return parsed


def strict_json_file(path: pathlib.Path) -> Any:
    return strict_json_bytes(path.read_bytes())


def write_bytes_new(path: pathlib.Path, value: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("xb") as stream:
        stream.write(value)
        stream.flush()
        os.fsync(stream.fileno())


def write_json_new(path: pathlib.Path, value: Any) -> None:
    write_bytes_new(path, (json.dumps(value, indent=2, sort_keys=True) + "\n").encode())


def inside(path: pathlib.Path, parent: pathlib.Path) -> bool:
    return path == parent or parent in path.parents


def expected_cargo_target_dir(run_id: Any) -> pathlib.Path | None:
    """Return the runner's one and only state-root-bound build location."""

    if not isinstance(run_id, str) or not run_id:
        return None
    return (
        CANONICAL_STATE_ROOT.absolute()
        / "build-cache"
        / EXECUTION_SOURCE_HEAD
        / run_id
    )


def trusted_host_executable(
    name: str, package: pathlib.Path, source: pathlib.Path
) -> str:
    """Resolve a host tool independently of every sealed package byte."""

    located = shutil.which(name)
    if located is None:
        raise AdjudicationInputError(f"trusted host executable unavailable: {name}")
    try:
        resolved = pathlib.Path(located).resolve(strict=True)
    except OSError as error:
        raise AdjudicationInputError(
            f"trusted host executable unresolved: {name}: {error}"
        ) from error
    if not resolved.is_file() or inside(resolved, package) or inside(resolved, source):
        raise AdjudicationInputError(f"trusted host executable unsafe: {name}")
    return str(resolved)


def trusted_adjudicator_python(package: pathlib.Path, source: pathlib.Path) -> str:
    """Use the current frozen interpreter, never a package-recorded Python."""

    if platform.python_version() != EXPECTED_PYTHON_VERSION:
        raise AdjudicationInputError("independent Python version mismatch")
    try:
        resolved = pathlib.Path(sys.executable).resolve(strict=True)
    except OSError as error:
        raise AdjudicationInputError(
            f"independent Python executable unresolved: {error}"
        ) from error
    if not resolved.is_file() or inside(resolved, package) or inside(resolved, source):
        raise AdjudicationInputError("independent Python executable unsafe")
    return str(resolved)


def independent_validator_environment(python_executable: str) -> dict[str, str]:
    """Build the adjudicator-owned environment for the source validator."""

    account_home = pwd.getpwuid(os.getuid()).pw_dir
    return {
        "HOME": account_home,
        "PATH": os.pathsep.join(
            dict.fromkeys(
                [str(pathlib.Path(python_executable).parent), "/usr/bin", "/bin"]
            )
        ),
        "LANG": "C",
        "LC_ALL": "C",
        "TZ": "UTC",
        "PYTHONDONTWRITEBYTECODE": "1",
    }


def regular_files(package: pathlib.Path) -> set[str]:
    result: set[str] = set()
    for path in package.rglob("*"):
        if path.is_symlink():
            raise ValueError(f"symlink forbidden: {path}")
        if path.is_file():
            result.add(path.relative_to(package).as_posix())
        elif path.exists() and not path.is_dir():
            raise ValueError(f"special file forbidden: {path}")
    return result


def full_candidate_file_allowlist(
    commands: Sequence[Mapping[str, Any]],
) -> set[str]:
    """Return the closed file set emitted by a complete runner execution."""

    result = {
        "SHA256SUMS",
        "execution_manifest.json",
        "events.jsonl",
        "validator_attempt.json",
        "authority_bundle_sha256.txt",
        "attempt_lock.json",
        "candidate_launch.json",
        *READINESS_OUTPUT_FILES,
    }
    for row in commands:
        for key in ("stdout_path", "stderr_path"):
            relative = row.get(key)
            if isinstance(relative, str):
                result.add(relative)
    return result


def candidate_free_file_contract(
    commands: Sequence[Mapping[str, Any]], manifest: Mapping[str, Any]
) -> tuple[set[str], set[str]]:
    """Return the required and allowed sealed files for a stopped prefix.

    Candidate-free outcomes vary by the reached stage, but their file universe
    is still closed.  The readiness script is the sole producer of its two
    named outputs and may not have reached a successful write before a failed
    readiness command, so those files are allowed only at that stage and
    required only after a successful readiness command.
    """

    required = {
        "SHA256SUMS",
        "execution_manifest.json",
        "events.jsonl",
        "validator_attempt.json",
        "authority_bundle_sha256.txt",
    }
    allowed = set(required)
    for row in commands:
        for key in ("stdout_path", "stderr_path"):
            relative = row.get(key)
            if isinstance(relative, str):
                required.add(relative)
                allowed.add(relative)

    readiness = next(
        (row for row in commands if row.get("name") == "readiness"), None
    )
    if readiness is not None:
        allowed.update(READINESS_OUTPUT_FILES)
        if (
            readiness.get("launch_status") == "LAUNCHED"
            and readiness.get("exit_code") == 0
        ):
            required.update(READINESS_OUTPUT_FILES)

    environment_policy = manifest.get("tool_versions")
    forbidden_environment = (
        environment_policy.get("environment_policy", {}).get(
            "forbidden_ambient_detected"
        )
        if isinstance(environment_policy, dict)
        and isinstance(environment_policy.get("environment_policy"), dict)
        else None
    )
    if not commands and isinstance(forbidden_environment, list) and forbidden_environment:
        required.add("forbidden_environment.json")
        allowed.add("forbidden_environment.json")
    if manifest.get("verdict") == "INCONCLUSIVE_RUNNER_INTERNAL_FAILURE":
        required.add("runner_internal_failure.json")
        allowed.add("runner_internal_failure.json")
    if manifest.get("verdict") == "INCONCLUSIVE_ONE_SHOT_ALREADY_CONSUMED":
        required.add("attempt_lock.json")
        allowed.add("attempt_lock.json")
    return required, allowed


def verify_seal(package: pathlib.Path) -> tuple[bool, list[str]]:
    errors: list[str] = []
    sums = package / "SHA256SUMS"
    if not sums.is_file() or sums.is_symlink():
        return False, ["SHA256SUMS missing or unsafe"]
    try:
        lines = sums.read_text(encoding="ascii").splitlines()
        entries: dict[str, str] = {}
        for line in lines:
            digest, separator, relative = line.partition("  ")
            if not separator or len(digest) != 64 or any(c not in "0123456789abcdef" for c in digest):
                raise ValueError(f"malformed SHA256SUMS line: {line!r}")
            candidate = pathlib.PurePosixPath(relative)
            if candidate.is_absolute() or ".." in candidate.parts or relative in {"", "SHA256SUMS"}:
                raise ValueError(f"unsafe SHA256SUMS path: {relative!r}")
            if relative in entries:
                raise ValueError(f"duplicate SHA256SUMS path: {relative}")
            entries[relative] = digest
        if list(entries) != sorted(entries):
            errors.append("SHA256SUMS paths are not sorted")
        actual = regular_files(package) - {"SHA256SUMS"}
        if set(entries) != actual:
            errors.append("SHA256SUMS file set mismatch")
        for relative, expected in entries.items():
            path = package / relative
            if not path.is_file() or path.is_symlink() or sha256_file(path) != expected:
                errors.append(f"hash mismatch: {relative}")
    except (OSError, UnicodeError, ValueError) as error:
        errors.append(str(error))
    return not errors, errors


def default_command_runner(
    argv: Sequence[str], *, cwd: pathlib.Path, env: Mapping[str, str] | None = None
) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        list(argv), cwd=cwd, env=None if env is None else dict(env),
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
    )


def source_identity(
    source: pathlib.Path,
    runner: Callable[..., subprocess.CompletedProcess[bytes]],
    *,
    git_executable: str = "git",
    environment: Mapping[str, str] | None = None,
) -> tuple[bool, dict[str, Any], list[str]]:
    commands = {
        "toplevel": [git_executable, "rev-parse", "--show-toplevel"],
        "head": [git_executable, "rev-parse", "HEAD"],
        "tree": [git_executable, "rev-parse", "HEAD^{tree}"],
        "parent": [git_executable, "rev-parse", "HEAD^"],
        "implementation_tree": [
            git_executable,
            "rev-parse",
            f"{IMPLEMENTATION_HEAD}^{{tree}}",
        ],
        "implementation_parent": [
            git_executable,
            "rev-parse",
            f"{IMPLEMENTATION_HEAD}^",
        ],
        "base_tree": [git_executable, "rev-parse", f"{BASE_HEAD}^{{tree}}"],
        "execution_ancestry": [
            git_executable,
            "merge-base",
            "--is-ancestor",
            IMPLEMENTATION_HEAD,
            EXECUTION_SOURCE_HEAD,
        ],
        "implementation_ancestry": [
            git_executable,
            "merge-base",
            "--is-ancestor",
            BASE_HEAD,
            IMPLEMENTATION_HEAD,
        ],
        "status": [git_executable, "status", "--porcelain=v1", "--untracked-files=all"],
    }
    observed: dict[str, Any] = {}
    errors: list[str] = []
    for name, argv in commands.items():
        completed = runner(argv, cwd=source, env=environment)
        text = completed.stdout.decode("utf-8", "replace").strip()
        if name.endswith("ancestry"):
            observed[name] = completed.returncode == 0
        else:
            observed[name] = text if completed.returncode == 0 else None
        if completed.returncode != 0:
            errors.append(f"source query failed: {name}")
    expected = {
        "toplevel": str(source),
        "head": EXECUTION_SOURCE_HEAD,
        "tree": EXECUTION_SOURCE_TREE,
        "parent": IMPLEMENTATION_HEAD,
        "implementation_tree": IMPLEMENTATION_TREE,
        "implementation_parent": BASE_HEAD,
        "base_tree": BASE_TREE,
        "execution_ancestry": True,
        "implementation_ancestry": True,
        "status": "",
    }
    for name, value in expected.items():
        if observed.get(name) != value:
            errors.append(f"source identity mismatch: {name}")
    return not errors, observed, errors


def expected_command_contract(
    source: pathlib.Path,
    package: pathlib.Path,
    executables: Mapping[str, str],
    environment: Mapping[str, str],
) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []

    def add(
        name: str,
        phase: str,
        argv: Sequence[str],
        stdout_path: str,
        stderr_path: str,
        *,
        extra_environment: Mapping[str, str] | None = None,
    ) -> None:
        command_environment = dict(environment)
        if extra_environment:
            command_environment.update(extra_environment)
        rows.append(
            {
                "name": name,
                "phase": phase,
                "argv": list(argv),
                "cwd": str(source),
                "environment_overrides": command_environment,
                "stdout_path": stdout_path,
                "stderr_path": stderr_path,
            }
        )

    def add_source(prefix: str) -> None:
        git = executables["git"]
        commands = (
            ("head", (git, "rev-parse", "HEAD")),
            ("tree", (git, "rev-parse", "HEAD^{tree}")),
            ("parent", (git, "rev-parse", "HEAD^")),
            (
                "implementation_tree",
                (git, "rev-parse", f"{IMPLEMENTATION_HEAD}^{{tree}}"),
            ),
            (
                "implementation_parent",
                (git, "rev-parse", f"{IMPLEMENTATION_HEAD}^"),
            ),
            ("base_tree", (git, "rev-parse", f"{BASE_HEAD}^{{tree}}")),
            (
                "execution_ancestry",
                (
                    git,
                    "merge-base",
                    "--is-ancestor",
                    IMPLEMENTATION_HEAD,
                    EXECUTION_SOURCE_HEAD,
                ),
            ),
            (
                "implementation_ancestry",
                (
                    git,
                    "merge-base",
                    "--is-ancestor",
                    BASE_HEAD,
                    IMPLEMENTATION_HEAD,
                ),
            ),
            ("status", (git, "status", "--porcelain=v1", "--untracked-files=all")),
        )
        for suffix, argv in commands:
            name = f"{prefix}_{suffix}"
            add(
                name,
                "source",
                argv,
                f"logs/{name}.stdout.log",
                f"logs/{name}.stderr.log",
            )

    add_source("source_pre")
    add(
        "python_dependencies",
        "environment",
        (
            executables["python"],
            "-c",
            "import json,platform,numpy,mpmath; print(json.dumps({'python':platform.python_version(),'numpy':numpy.__version__,'mpmath':mpmath.__version__},sort_keys=True))",
        ),
        "logs/python_dependencies.stdout.log",
        "logs/python_dependencies.stderr.log",
    )
    add(
        "rustc_version",
        "environment",
        (executables["rustc"], "--version"),
        "logs/rustc_version.stdout.log",
        "logs/rustc_version.stderr.log",
    )
    add(
        "cargo_version",
        "environment",
        (executables["cargo"], "--version"),
        "logs/cargo_version.stdout.log",
        "logs/cargo_version.stderr.log",
    )
    add(
        "authority_verification",
        "preflight",
        (executables["python"], VERIFIER_PATH),
        "authority_verification.json",
        "logs/authority_verification.stderr.log",
    )
    add(
        "python_authority_tests",
        "preflight",
        (executables["python"], "tools/test_audit2_real_client_authority.py", "-v"),
        "logs/python_authority_tests.stdout.log",
        "logs/python_authority_tests.stderr.log",
    )
    add(
        "python_local_receipt_tests",
        "preflight",
        (executables["python"], "tools/test_audit2_bateman_local_receipt.py", "-v"),
        "logs/python_local_receipt_tests.stdout.log",
        "logs/python_local_receipt_tests.stderr.log",
    )
    add(
        "rust_authority_contracts",
        "preflight",
        (
            executables["cargo"], "test", "--locked", "-p", "rodas5p-integrators",
            "--features", "audit2-bateman-authority", "--test",
            "audit2_real_client_authority_contracts", "--", "--nocapture",
            "--test-threads=1",
        ),
        "logs/rust_authority_contracts.stdout.log",
        "logs/rust_authority_contracts.stderr.log",
    )
    add(
        "readiness",
        "preflight",
        (executables["bash"], READINESS_PATH),
        "logs/readiness.stdout.log",
        "logs/readiness.stderr.log",
        extra_environment={"AUDIT2_OUTPUT_DIR": str(package / "readiness-output")},
    )
    add(
        "clippy",
        "preflight",
        (
            executables["cargo"], "clippy", "--locked", "-p", "rodas5p-integrators",
            "-p", "rodas5p-fair-ab", "--all-targets", "--features",
            "rodas5p-integrators/audit2-bateman-authority", "--", "-D", "warnings",
        ),
        "logs/clippy.stdout.log",
        "logs/clippy.stderr.log",
    )
    add(
        "fmt",
        "preflight",
        (executables["cargo"], "fmt", "--all", "--", "--check"),
        "logs/fmt.stdout.log",
        "logs/fmt.stderr.log",
    )
    add(
        "diff_check",
        "preflight",
        (executables["git"], "diff", "--check"),
        "logs/diff_check.stdout.log",
        "logs/diff_check.stderr.log",
    )
    add(
        "cargo_build_candidate",
        "preflight",
        (
            executables["cargo"], "build", "--locked", "-p", "rodas5p-integrators",
            "--features", "audit2-bateman-authority", "--example",
            "audit2_bateman_local_six_case",
        ),
        "logs/cargo_build_candidate.stdout.log",
        "logs/cargo_build_candidate.stderr.log",
    )
    add_source("source_prelaunch")
    add(
        "candidate",
        "candidate",
        (executables["cargo"], *CANDIDATE_ARGV[1:]),
        "result_summary.json",
        "logs/candidate.stderr.log",
    )
    add(
        "validator",
        "validator",
        (
            executables["python"],
            str(source / VALIDATOR_PATH),
            str(package / "result_summary.json"),
        ),
        "local_receipt_validation.json",
        "logs/validator.stderr.log",
    )
    add_source("source_post")
    return rows


def parse_events(path: pathlib.Path) -> list[Any]:
    events: list[Any] = []
    for line_number, line in enumerate(path.read_bytes().splitlines(), 1):
        if not line:
            raise ValueError(f"blank event line: {line_number}")
        events.append(strict_json_bytes(line))
    return events


def expected_authority_verification(expected_hashes: Mapping[str, str]) -> dict[str, Any]:
    return {
        "candidate_executions": 0,
        "declared_reference_l2_uncertainty": 1e-15,
        "execution_scenarios": 6,
        "fast_exponent_exceeds_one": True,
        "holdout_access": "NOT_OPENED_OR_EXECUTED",
        "local_six_case_status": "NOT_RUN_DURING_AUTHORITY_CONSTRUCTION",
        "max_reference_l2_bound": 2.075243427511439e-17,
        "receipt_sha256": expected_hashes[PROOF_PATH],
        "status": "AUTHORITY_CONSTRUCTION_VERIFIED",
        "verified_operator_cases": 2,
    }


def source_snapshot_from_rows(
    rows: Sequence[Mapping[str, Any]],
    package: pathlib.Path,
    prefix: str,
) -> dict[str, Any] | None:
    suffixes = (
        "head",
        "tree",
        "parent",
        "implementation_tree",
        "implementation_parent",
        "base_tree",
        "execution_ancestry",
        "implementation_ancestry",
        "status",
    )
    if [row.get("name") for row in rows] != [
        f"{prefix}_{suffix}" for suffix in suffixes
    ]:
        return None
    result: dict[str, Any] = {}
    for row, suffix in zip(rows, suffixes, strict=True):
        stdout_path = row.get("stdout_path")
        if not isinstance(stdout_path, str):
            return None
        text = (package / stdout_path).read_text(errors="replace").strip()
        successful = (
            row.get("launch_status") == "LAUNCHED" and row.get("exit_code") == 0
        )
        if suffix.endswith("ancestry"):
            result[suffix] = successful
        elif suffix == "status":
            result[suffix] = text
            if not successful:
                result["status_query_failed"] = True
        else:
            result[suffix] = text if successful else None
    expected = expected_source_identity()
    result["valid"] = bool(
        result.get("head") == expected["head"]
        and result.get("tree") == expected["tree"]
        and result.get("parent") == expected["parent"]
        and result.get("implementation_tree") == expected["implementation_tree"]
        and result.get("implementation_parent") == expected["implementation_parent"]
        and result.get("base_tree") == expected["base_tree"]
        and result.get("execution_ancestry") is True
        and result.get("implementation_ancestry") is True
        and result.get("status") == ""
        and result.get("status_query_failed") is not True
    )
    return result


def validate_candidate_free_protocol(
    manifest: Any,
    package: pathlib.Path,
    source: pathlib.Path,
    validator_attempt: Any,
    events: Any,
) -> tuple[bool, str | None, list[str]]:
    """Validate a sealed pre-candidate stop without executing any subprocess."""

    errors: list[str] = []
    if (
        not isinstance(manifest, dict)
        or manifest.get("schema")
        != "vigilode-audit2-bateman-local-execution-manifest/v1"
    ):
        return False, None, ["execution manifest schema mismatch"]
    if manifest.get("protocol_version") != PROTOCOL_VERSION:
        errors.append("protocol version mismatch")
    if set(manifest) != FULL_MANIFEST_KEYS:
        errors.append("execution manifest key set mismatch")
    if (
        not isinstance(manifest.get("run_id"), str)
        or not isinstance(manifest.get("utc_start"), str)
        or not isinstance(manifest.get("utc_end"), str)
        or manifest["utc_start"] > manifest["utc_end"]
    ):
        errors.append("manifest run identity/timestamp mismatch")
    if manifest.get("package") != {"sha256sums_path": "SHA256SUMS"}:
        errors.append("manifest package descriptor mismatch")
    if manifest.get("math_blocker_coverage") != expected_math_blocker_coverage(False):
        errors.append("manifest math blocker coverage mismatch")

    commands = manifest.get("commands")
    if not isinstance(commands, list):
        return False, None, errors + ["commands ledger missing"]
    if any(not isinstance(row, dict) for row in commands):
        return False, None, errors + ["command row malformed"]
    if len(commands) not in {0, 9, *range(10, 22), 30}:
        errors.append("candidate-free command prefix length mismatch")
    observed = [(row.get("name"), row.get("phase")) for row in commands]
    if observed != list(EXPECTED_COMMANDS[: len(commands)]):
        errors.append("command sequence mismatch")
    if any(
        row.get("name") == "candidate" or row.get("name") == "validator"
        for row in commands
    ):
        errors.append("candidate-bearing row in candidate-free package")

    tool_versions = manifest.get("tool_versions")
    executable_rows = (
        tool_versions.get("executables") if isinstance(tool_versions, dict) else None
    )
    environment_policy = (
        tool_versions.get("environment_policy")
        if isinstance(tool_versions, dict)
        else None
    )
    controller = (
        tool_versions.get("controller") if isinstance(tool_versions, dict) else None
    )
    executables: dict[str, str] = {}
    if not isinstance(executable_rows, dict) or set(executable_rows) != {
        "bash",
        "cargo",
        "git",
        "python",
        "rustc",
    }:
        errors.append("executable identity set mismatch")
    else:
        for name, row in executable_rows.items():
            if not isinstance(row, dict):
                errors.append(f"executable identity malformed: {name}")
                continue
            if set(row) != EXECUTABLE_IDENTITY_KEYS:
                errors.append(f"executable identity key set mismatch: {name}")
            path = row.get("path")
            realpath = row.get("realpath")
            if (
                not isinstance(path, str)
                or not pathlib.Path(path).is_absolute()
                or not isinstance(realpath, str)
                or not pathlib.Path(realpath).is_absolute()
                or not isinstance(row.get("sha256"), str)
                or len(row["sha256"]) != 64
                or type(row.get("size_bytes")) is not int
                or row["size_bytes"] <= 0
            ):
                errors.append(f"executable byte identity malformed: {name}")
                continue
            executables[name] = path

    environment = (
        environment_policy.get("effective")
        if isinstance(environment_policy, dict)
        else None
    )
    forbidden = (
        environment_policy.get("forbidden_ambient_detected")
        if isinstance(environment_policy, dict)
        else None
    )
    cargo_configs = (
        environment_policy.get("cargo_config_files_present")
        if isinstance(environment_policy, dict)
        else None
    )
    if (
        not isinstance(environment_policy, dict)
        or set(environment_policy) != ENVIRONMENT_POLICY_KEYS
        or environment_policy.get("mode")
        != "SANITIZED_ALLOWLIST_PLUS_FIXED_VALUES"
        or not isinstance(forbidden, list)
        or not isinstance(cargo_configs, list)
        or not isinstance(environment, dict)
    ):
        errors.append("environment policy mismatch")
        environment = {}
        forbidden = []
        cargo_configs = []
    else:
        allowed = PRESERVED_ENVIRONMENT | set(FIXED_ENVIRONMENT) | {
            "HOME",
            "CARGO_HOME",
            "PATH",
            "CARGO_TARGET_DIR",
        }
        if not set(environment) <= allowed:
            errors.append("effective environment contains an unapproved key")
        for key, expected in FIXED_ENVIRONMENT.items():
            if environment.get(key) != expected:
                errors.append(f"effective environment mismatch: {key}")
        target = environment.get("CARGO_TARGET_DIR")
        expected_target = expected_cargo_target_dir(manifest.get("run_id"))
        target_path = pathlib.Path(target) if isinstance(target, str) else None
        if (
            not isinstance(target, str)
            or target_path is None
            or not target_path.is_absolute()
            or expected_target is None
            or target != str(expected_target)
            or inside(target_path, source)
            or inside(target_path, package)
        ):
            errors.append("fixed Cargo target directory mismatch")
        home = environment.get("HOME")
        if (
            not isinstance(environment.get("PATH"), str)
            or not isinstance(home, str)
            or not pathlib.Path(home).is_absolute()
            or environment.get("CARGO_HOME") != str(pathlib.Path(home) / ".cargo")
        ):
            errors.append("fixed account/Cargo home mismatch")
    if commands and (forbidden != [] or cargo_configs != []):
        errors.append("runner executed commands after forbidden environment state")
    if (
        not isinstance(controller, dict)
        or set(controller) != CONTROLLER_KEYS
        or controller.get("sha256_pre") != EXPECTED_RUNNER_SHA256
        or controller.get("sha256_post") != EXPECTED_RUNNER_SHA256
        or not isinstance(controller.get("path"), str)
        or not pathlib.Path(controller["path"]).is_absolute()
    ):
        errors.append("published runner byte identity mismatch")
    platform_row = tool_versions.get("platform") if isinstance(tool_versions, dict) else None
    if (
        not isinstance(platform_row, dict)
        or set(platform_row) != PLATFORM_KEYS
        or any(not isinstance(platform_row.get(key), str) for key in PLATFORM_KEYS)
    ):
        errors.append("platform identity shape mismatch")

    expected_rows = (
        expected_command_contract(source, package, executables, environment)
        if len(executables) == 5
        else []
    )
    for index, row in enumerate(commands, 1):
        if set(row) != FULL_COMMAND_ROW_KEYS:
            errors.append(f"command key set mismatch: {row.get('name')}")
        if row.get("ordinal") != index:
            errors.append("command ordinal mismatch")
        if index <= len(expected_rows):
            expected = expected_rows[index - 1]
            for field in (
                "name",
                "phase",
                "argv",
                "cwd",
                "environment_overrides",
                "stdout_path",
                "stderr_path",
            ):
                if row.get(field) != expected[field]:
                    errors.append(f"command {field} mismatch: {row.get('name')}")
        launch = row.get("launch_status")
        exit_code = row.get("exit_code")
        if not (
            (launch == "LAUNCHED" and type(exit_code) is int)
            or (launch == "SPAWN_FAILED" and exit_code == 127)
        ):
            errors.append(f"command launch/exit mismatch: {row.get('name')}")
        for stream in ("stdout", "stderr"):
            relative = row.get(f"{stream}_path")
            digest = row.get(f"{stream}_sha256")
            if not isinstance(relative, str):
                errors.append(f"command {stream} path missing: {row.get('name')}")
                continue
            pure = pathlib.PurePosixPath(relative)
            path = package / relative
            if (
                pure.is_absolute()
                or ".." in pure.parts
                or not path.is_file()
                or path.is_symlink()
                or digest != sha256_file(path)
            ):
                errors.append(f"command {stream} byte mismatch: {row.get('name')}")
        if not isinstance(row.get("started_at_utc"), str) or not isinstance(
            row.get("finished_at_utc"), str
        ):
            errors.append(f"command timestamp missing: {row.get('name')}")
        elif row["started_at_utc"] > row["finished_at_utc"]:
            errors.append(f"command timestamp order mismatch: {row.get('name')}")

    if errors:
        return False, None, errors

    failed_commands = [
        {
            "ordinal": row["ordinal"],
            "name": row["name"],
            "launch_status": row["launch_status"],
            "exit_code": row["exit_code"],
        }
        for row in commands
        if row.get("launch_status") != "LAUNCHED" or row.get("exit_code") != 0
    ]
    terminal = commands[-1] if commands else None
    expected_stop = {
        "runner_verdict": manifest.get("verdict"),
        "reached_ordinal": len(commands),
        "terminal_command": terminal.get("name") if terminal else None,
        "terminal_phase": terminal.get("phase") if terminal else None,
        "terminal_launch_status": terminal.get("launch_status") if terminal else None,
        "terminal_exit_code": terminal.get("exit_code") if terminal else None,
        "failed_commands": failed_commands,
        "kind": "COMMAND_FAILURE" if failed_commands else "SEMANTIC_OR_STAGE_FAILURE",
    }
    if manifest.get("pre_candidate_stop") != expected_stop:
        errors.append("pre-candidate stop record mismatch")

    expected_candidate = {
        "invocation_count": 0,
        "launch_status": "NOT_RUN",
        "exit_code": None,
        "argv": list(CANDIDATE_ARGV),
    }
    expected_validator = {
        "attempt_count": 0,
        "launch_status": "NOT_RUN",
        "exit_code": None,
    }
    expected_validator_attempt = {
        "schema": "vigilode-audit2-bateman-validator-attempt/v1",
        "attempted": False,
        "attempt_count": 0,
        "launch_status": "NOT_RUN",
        "exit_code": None,
        "argv": None,
        "stdout_path": None,
        "stderr_path": None,
        "stdout_sha256": None,
        "stderr_sha256": None,
    }
    if manifest.get("candidate") != expected_candidate:
        errors.append("candidate was not recorded as unattempted")
    if manifest.get("validator") != expected_validator:
        errors.append("validator was not recorded as unattempted")
    if validator_attempt != expected_validator_attempt:
        errors.append("validator-attempt sidecar mismatch for not-run package")
    if manifest.get("source_post") is not None:
        errors.append("manifest source_post must be null before candidate")
    if manifest.get("scientific_implementation_provenance") != SCIENTIFIC_IMPLEMENTATION_PROVENANCE:
        errors.append("manifest scientific implementation provenance mismatch")
    if manifest.get("declarations") != EXPECTED_DECLARATIONS:
        errors.append("manifest declarations mismatch")

    source_pre: dict[str, Any] | None = None
    if len(commands) >= 9:
        source_pre = source_snapshot_from_rows(commands[:9], package, "source_pre")
    if manifest.get("source_pre") != source_pre:
        errors.append("manifest source_pre snapshot mismatch")

    bundle_path = package / "authority_bundle_sha256.txt"
    source_hashes = manifest.get("source_hashes")
    hashes_all_match = False
    if source_pre is None or source_pre.get("valid") is not True:
        if source_hashes != {} or bundle_path.read_bytes() != b"":
            errors.append("source-failure package must have empty authority binding")
    elif not isinstance(source_hashes, dict) or set(source_hashes) != set(EXPECTED_SOURCE_HASHES):
        errors.append("manifest source_hashes set mismatch")
    else:
        hashes_all_match = True
        bundle_lines: list[str] = []
        for relative, expected in sorted(EXPECTED_SOURCE_HASHES.items()):
            row = source_hashes.get(relative)
            if not isinstance(row, dict) or set(row) != SOURCE_HASH_ROW_KEYS:
                errors.append(f"manifest source hash malformed: {relative}")
                hashes_all_match = False
                continue
            observed_hash = row.get("observed")
            match = observed_hash == expected
            if row.get("expected") != expected or row.get("match") is not match:
                errors.append(f"manifest source hash mismatch: {relative}")
            hashes_all_match &= match
            bundle_lines.append(f"{observed_hash or 'MISSING'}  {relative}\n")
        if bundle_path.read_text(errors="replace") != "".join(bundle_lines):
            errors.append("authority bundle digest listing mismatch")
    if len(commands) > 9 and not hashes_all_match:
        errors.append("runner advanced after unresolved source hash binding")

    rows_by_name = {row["name"]: row for row in commands}
    base_tool_keys = {"executables", "platform", "controller", "environment_policy"}
    expected_tool_keys = set(base_tool_keys)
    version_contracts: tuple[tuple[str, str, Any], ...] = (
        (
            "python_dependencies",
            "python",
            {
                "python": EXPECTED_PYTHON_VERSION,
                "numpy": EXPECTED_NUMPY_VERSION,
                "mpmath": EXPECTED_MPMATH_VERSION,
            },
        ),
        ("rustc_version", "rustc_version", f"rustc {EXPECTED_RUST_VERSION} "),
        ("cargo_version", "cargo_version", f"cargo {EXPECTED_RUST_VERSION} "),
    )
    for command_name, tool_key, frozen in version_contracts:
        row = rows_by_name.get(command_name)
        parsed: Any = None
        parsed_ok = False
        if row is not None and row.get("launch_status") == "LAUNCHED" and row.get("exit_code") == 0:
            stdout = (package / row["stdout_path"]).read_bytes()
            if command_name == "python_dependencies":
                try:
                    parsed = strict_json_bytes(stdout)
                    parsed_ok = True
                except (ValueError, json.JSONDecodeError):
                    parsed_ok = False
            else:
                parsed = stdout.decode("utf-8", "replace").strip()
                parsed_ok = True
        if parsed_ok:
            expected_tool_keys.add(tool_key)
            if not isinstance(tool_versions, dict) or tool_versions.get(tool_key) != parsed:
                errors.append(f"staged tool version mismatch: {tool_key}")
            is_frozen = parsed == frozen if isinstance(frozen, dict) else str(parsed).startswith(frozen)
            if row is not terminal and not is_frozen:
                errors.append(f"runner advanced after version mismatch: {tool_key}")
        elif isinstance(tool_versions, dict) and tool_key in tool_versions:
            errors.append(f"tool version recorded before successful command: {tool_key}")

    authority_row = rows_by_name.get("authority_verification")
    authority_path = package / "authority_verification.json"
    expected_authority = expected_authority_verification(EXPECTED_SOURCE_HASHES)
    if authority_row is None:
        if authority_path.exists() or manifest.get("authority_preflight") != {}:
            errors.append("authority artifact present before authority command")
    else:
        if not authority_path.is_file() or authority_path.is_symlink():
            errors.append("authority command stream missing")
            parsed_authority = None
        else:
            try:
                parsed_authority = strict_json_file(authority_path)
            except (OSError, ValueError, json.JSONDecodeError):
                parsed_authority = None
        authority_succeeded = (
            authority_row.get("launch_status") == "LAUNCHED"
            and authority_row.get("exit_code") == 0
        )
        if authority_succeeded and parsed_authority is not None:
            if manifest.get("authority_preflight") != parsed_authority:
                errors.append("authority verification manifest mismatch")
        elif manifest.get("authority_preflight") != {}:
            errors.append("failed authority command populated manifest authority")
        if authority_row is not terminal:
            if parsed_authority != expected_authority:
                errors.append("runner advanced after authority mismatch")
        elif authority_succeeded and parsed_authority == expected_authority:
            errors.append("runner stopped after successful canonical authority command")

    if len(commands) == 30:
        expected_tool_keys.add("candidate_binary")
        source_prelaunch = source_snapshot_from_rows(
            commands[21:30], package, "source_prelaunch"
        )
        if manifest.get("source_prelaunch") != source_prelaunch:
            errors.append("manifest source_prelaunch snapshot mismatch")
        binary = tool_versions.get("candidate_binary") if isinstance(tool_versions, dict) else None
        target = environment.get("CARGO_TARGET_DIR") if isinstance(environment, dict) else None
        expected_binary = (
            str(pathlib.Path(target) / "debug/examples/audit2_bateman_local_six_case")
            if isinstance(target, str)
            else None
        )
        if (
            not isinstance(binary, dict)
            or set(binary) != CANDIDATE_BINARY_KEYS
            or binary.get("path") != expected_binary
            or not isinstance(binary.get("sha256_prelaunch"), str)
            or len(binary["sha256_prelaunch"]) != 64
            or binary.get("sha256_post") is not None
            or type(binary.get("size_bytes")) is not int
            or binary["size_bytes"] <= 0
        ):
            errors.append("candidate binary prelaunch identity mismatch")
    else:
        if manifest.get("source_prelaunch") is not None:
            errors.append("manifest source_prelaunch must be null at this stage")
        if isinstance(tool_versions, dict) and "candidate_binary" in tool_versions:
            errors.append("candidate binary identity recorded before prelaunch stage")
    if isinstance(tool_versions, dict) and set(tool_versions) != expected_tool_keys:
        errors.append("stage-dependent tool-version key set mismatch")

    allowed_verdicts: set[str]
    if not commands:
        allowed_verdicts = {
            "INCONCLUSIVE_ENVIRONMENT_UNRESOLVED",
            "INCONCLUSIVE_RUNNER_INTERNAL_FAILURE",
        }
        if not forbidden and not cargo_configs:
            errors.append("empty prefix lacks a recorded environment blocker")
    elif source_pre is None or source_pre.get("valid") is not True:
        allowed_verdicts = {
            "INCONCLUSIVE_SOURCE_IDENTITY_UNRESOLVED",
            "INCONCLUSIVE_RUNNER_INTERNAL_FAILURE",
        }
    elif len(commands) == 9:
        allowed_verdicts = {
            "INCONCLUSIVE_AUTHORITY_BUNDLE_MISMATCH",
            "INCONCLUSIVE_RUNNER_INTERNAL_FAILURE",
        }
        if hashes_all_match:
            errors.append("runner stopped after complete source binding without a failure")
    elif len(commands) <= 12:
        allowed_verdicts = {
            "INCONCLUSIVE_ENVIRONMENT_UNRESOLVED",
            "INCONCLUSIVE_RUNNER_INTERNAL_FAILURE",
        }
    elif len(commands) <= 20:
        allowed_verdicts = {
            "INCONCLUSIVE_AUTHORITY_PREFLIGHT_FAILED",
            "INCONCLUSIVE_RUNNER_INTERNAL_FAILURE",
        }
    elif len(commands) == 21:
        allowed_verdicts = {
            "INCONCLUSIVE_AUTHORITY_PREFLIGHT_FAILED",
            "INCONCLUSIVE_CANDIDATE_BINARY_UNRESOLVED",
            "INCONCLUSIVE_RUNNER_INTERNAL_FAILURE",
        }
    else:
        allowed_verdicts = {
            "INCONCLUSIVE_SOURCE_CHANGED_BEFORE_CANDIDATE",
            "INCONCLUSIVE_ONE_SHOT_ALREADY_CONSUMED",
            "INCONCLUSIVE_RUNNER_INTERNAL_FAILURE",
        }
    recorded_verdict = manifest.get("verdict")
    if not isinstance(recorded_verdict, str) or recorded_verdict not in allowed_verdicts:
        errors.append("runner verdict disagrees with candidate-free stage")

    if not commands and forbidden:
        try:
            forbidden_artifact = strict_json_file(
                package / "forbidden_environment.json"
            )
        except (OSError, ValueError, json.JSONDecodeError):
            forbidden_artifact = None
        if forbidden_artifact != {"keys": forbidden}:
            errors.append("forbidden environment artifact mismatch")

    if recorded_verdict == "INCONCLUSIVE_RUNNER_INTERNAL_FAILURE":
        try:
            internal_failure = strict_json_file(
                package / "runner_internal_failure.json"
            )
        except (OSError, ValueError, json.JSONDecodeError):
            internal_failure = None
        if (
            not isinstance(internal_failure, dict)
            or set(internal_failure) != {"type", "message"}
            or not isinstance(internal_failure.get("type"), str)
            or not internal_failure["type"]
            or not isinstance(internal_failure.get("message"), str)
        ):
            errors.append("runner internal failure artifact mismatch")

    if recorded_verdict == "INCONCLUSIVE_ONE_SHOT_ALREADY_CONSUMED":
        try:
            contention_lock = strict_json_file(package / "attempt_lock.json")
        except (OSError, ValueError, json.JSONDecodeError):
            contention_lock = None
        expected_key_material = {
            "implementation_head": IMPLEMENTATION_HEAD,
            "authority_manifest_sha256": EXPECTED_SOURCE_HASHES[MANIFEST_PATH],
            "example_target": "audit2_bateman_local_six_case",
            "protocol_version": PROTOCOL_VERSION,
        }
        expected_key_sha = sha256_bytes(
            json.dumps(
                expected_key_material, sort_keys=True, separators=(",", ":")
            ).encode()
        )
        if (
            not isinstance(contention_lock, dict)
            or set(contention_lock) != ATTEMPT_LOCK_KEYS
            or contention_lock.get("schema")
            != "vigilode-audit2-bateman-one-shot-lock/v1"
            or contention_lock.get("protocol_version") != PROTOCOL_VERSION
            or contention_lock.get("key_material") != expected_key_material
            or contention_lock.get("key_sha256") != expected_key_sha
            or not isinstance(contention_lock.get("created_at_utc"), str)
            or not isinstance(contention_lock.get("run_id"), str)
            or contention_lock["run_id"] == manifest.get("run_id")
        ):
            errors.append("one-shot contention lock mismatch")

    if not isinstance(events, list) or len(events) != 2 * len(commands) + 2:
        errors.append("event count mismatch")
    else:
        expected_events: list[dict[str, Any]] = [
            {
                "schema": "vigilode-audit2-bateman-execution-event/v1",
                "ordinal": 1,
                "event": "run_created",
                "utc": manifest.get("utc_start"),
                "run_id": manifest.get("run_id"),
            }
        ]
        for row in commands:
            expected_events.extend(
                [
                    {
                        "schema": "vigilode-audit2-bateman-execution-event/v1",
                        "ordinal": len(expected_events) + 1,
                        "event": "command_started",
                        "name": row["name"],
                        "phase": row["phase"],
                        "utc": row["started_at_utc"],
                        "argv": row["argv"],
                    },
                    {
                        "schema": "vigilode-audit2-bateman-execution-event/v1",
                        "ordinal": len(expected_events) + 2,
                        "event": "command_finished",
                        "name": row["name"],
                        "phase": row["phase"],
                        "utc": row["finished_at_utc"],
                        "launch_status": row["launch_status"],
                        "exit_code": row["exit_code"],
                    },
                ]
            )
        expected_events.append(
            {
                "schema": "vigilode-audit2-bateman-execution-event/v1",
                "ordinal": len(expected_events) + 1,
                "event": "sealing_started",
                "utc": manifest.get("utc_end"),
                "verdict": manifest.get("verdict"),
            }
        )
        if events != expected_events:
            errors.append("event state mismatch")
    terminal_name = terminal.get("name") if terminal else "environment"
    return not errors, str(terminal_name), errors


def validate_manifest_protocol(
    manifest: Any,
    package: pathlib.Path,
    source: pathlib.Path,
    authority_verification: Any,
    events: Any,
) -> tuple[bool, str | None, list[str]]:
    errors: list[str] = []
    preflight_failure: str | None = None
    if (
        not isinstance(manifest, dict)
        or manifest.get("schema")
        != "vigilode-audit2-bateman-local-execution-manifest/v1"
    ):
        return False, None, ["execution manifest schema mismatch"]
    if set(manifest) != FULL_MANIFEST_KEYS:
        errors.append("execution manifest key set mismatch")
    if manifest.get("protocol_version") != PROTOCOL_VERSION:
        errors.append("protocol version mismatch")
    if manifest.get("pre_candidate_stop") is not None:
        errors.append("candidate-bearing package has a pre-candidate stop record")
    if (
        not isinstance(manifest.get("run_id"), str)
        or not isinstance(manifest.get("utc_start"), str)
        or not isinstance(manifest.get("utc_end"), str)
        or manifest["utc_start"] > manifest["utc_end"]
    ):
        errors.append("manifest run identity/timestamp mismatch")
    if manifest.get("package") != {"sha256sums_path": "SHA256SUMS"}:
        errors.append("manifest package descriptor mismatch")
    eligible_receipt = manifest.get("verdict") in {
        RUNNER_ACCEPT_VERDICT,
        REJECT_VERDICT,
    }
    if manifest.get("math_blocker_coverage") != expected_math_blocker_coverage(
        eligible_receipt
    ):
        errors.append("manifest math blocker coverage mismatch")

    tool_versions = manifest.get("tool_versions")
    if not isinstance(tool_versions, dict) or set(tool_versions) != TOOL_VERSION_KEYS:
        errors.append("tool version key set mismatch")
    executable_rows = (
        tool_versions.get("executables") if isinstance(tool_versions, dict) else None
    )
    environment_policy = (
        tool_versions.get("environment_policy")
        if isinstance(tool_versions, dict)
        else None
    )
    controller = (
        tool_versions.get("controller") if isinstance(tool_versions, dict) else None
    )
    if not isinstance(executable_rows, dict) or set(executable_rows) != {
        "bash",
        "cargo",
        "git",
        "python",
        "rustc",
    }:
        errors.append("executable identity set mismatch")
        executables: dict[str, str] = {}
    else:
        executables = {}
        for name, row in executable_rows.items():
            if not isinstance(row, dict):
                errors.append(f"executable identity malformed: {name}")
                continue
            if set(row) != EXECUTABLE_IDENTITY_KEYS:
                errors.append(f"executable identity key set mismatch: {name}")
            path = row.get("path")
            realpath = row.get("realpath")
            if (
                not isinstance(path, str)
                or not pathlib.Path(path).is_absolute()
                or not isinstance(realpath, str)
                or not pathlib.Path(realpath).is_absolute()
            ):
                errors.append(f"executable path malformed: {name}")
                continue
            executables[name] = path
            if (
                not isinstance(row.get("sha256"), str)
                or len(row["sha256"]) != 64
                or type(row.get("size_bytes")) is not int
                or row["size_bytes"] <= 0
            ):
                errors.append(f"executable byte identity malformed: {name}")

    platform_row = (
        tool_versions.get("platform") if isinstance(tool_versions, dict) else None
    )
    if (
        not isinstance(platform_row, dict)
        or set(platform_row) != PLATFORM_KEYS
        or any(not isinstance(platform_row.get(key), str) for key in PLATFORM_KEYS)
    ):
        errors.append("platform identity shape mismatch")

    environment = (
        environment_policy.get("effective")
        if isinstance(environment_policy, dict)
        else None
    )
    if (
        not isinstance(environment_policy, dict)
        or set(environment_policy) != ENVIRONMENT_POLICY_KEYS
        or environment_policy.get("mode")
        != "SANITIZED_ALLOWLIST_PLUS_FIXED_VALUES"
        or environment_policy.get("forbidden_ambient_detected") != []
        or environment_policy.get("cargo_config_files_present") != []
        or not isinstance(environment, dict)
    ):
        errors.append("environment policy mismatch")
        environment = {}
    else:
        allowed = PRESERVED_ENVIRONMENT | set(FIXED_ENVIRONMENT) | {
            "HOME",
            "CARGO_HOME",
            "PATH",
            "CARGO_TARGET_DIR",
        }
        if not set(environment) <= allowed:
            errors.append("effective environment contains an unapproved key")
        for key, expected in FIXED_ENVIRONMENT.items():
            if environment.get(key) != expected:
                errors.append(f"effective environment mismatch: {key}")
        target = environment.get("CARGO_TARGET_DIR")
        expected_target = expected_cargo_target_dir(manifest.get("run_id"))
        target_path = pathlib.Path(target) if isinstance(target, str) else None
        if (
            not isinstance(target, str)
            or target_path is None
            or not target_path.is_absolute()
            or expected_target is None
            or target != str(expected_target)
            or inside(target_path, source)
            or inside(target_path, package)
        ):
            errors.append("fixed Cargo target directory mismatch")
        if not isinstance(environment.get("PATH"), str):
            errors.append("effective PATH missing")
        home = environment.get("HOME")
        cargo_home = environment.get("CARGO_HOME")
        if (
            not isinstance(home, str)
            or not pathlib.Path(home).is_absolute()
            or cargo_home != str(pathlib.Path(home) / ".cargo")
        ):
            errors.append("fixed account/Cargo home mismatch")

    if (
        not isinstance(controller, dict)
        or set(controller) != CONTROLLER_KEYS
        or controller.get("sha256_pre") != EXPECTED_RUNNER_SHA256
        or controller.get("sha256_post") != EXPECTED_RUNNER_SHA256
        or not isinstance(controller.get("path"), str)
        or not pathlib.Path(controller["path"]).is_absolute()
    ):
        errors.append("published runner byte identity mismatch")

    if (
        isinstance(tool_versions, dict)
        and tool_versions.get("python")
        != {
            "python": EXPECTED_PYTHON_VERSION,
            "numpy": EXPECTED_NUMPY_VERSION,
            "mpmath": EXPECTED_MPMATH_VERSION,
        }
    ):
        errors.append("Python dependency version mismatch")
    for key, prefix in (
        ("rustc_version", f"rustc {EXPECTED_RUST_VERSION} "),
        ("cargo_version", f"cargo {EXPECTED_RUST_VERSION} "),
    ):
        value = tool_versions.get(key) if isinstance(tool_versions, dict) else None
        if not isinstance(value, str) or not value.startswith(prefix):
            errors.append(f"{key} mismatch")

    candidate_binary = (
        tool_versions.get("candidate_binary")
        if isinstance(tool_versions, dict)
        else None
    )
    if (
        not isinstance(candidate_binary, dict)
        or set(candidate_binary) != CANDIDATE_BINARY_KEYS
        or not isinstance(candidate_binary.get("path"), str)
        or not pathlib.Path(candidate_binary["path"]).is_absolute()
        or not isinstance(candidate_binary.get("sha256_prelaunch"), str)
        or len(candidate_binary["sha256_prelaunch"]) != 64
        or not isinstance(candidate_binary.get("sha256_post"), str)
        or len(candidate_binary["sha256_post"]) != 64
        or type(candidate_binary.get("size_bytes")) is not int
        or candidate_binary["size_bytes"] <= 0
    ):
        errors.append("candidate binary identity shape mismatch")

    candidate_summary = manifest.get("candidate")
    if (
        not isinstance(candidate_summary, dict)
        or set(candidate_summary) != CANDIDATE_SUMMARY_KEYS
    ):
        errors.append("candidate summary key set mismatch")
    validator_summary = manifest.get("validator")
    if (
        not isinstance(validator_summary, dict)
        or set(validator_summary) != VALIDATOR_SUMMARY_KEYS
    ):
        errors.append("validator summary key set mismatch")

    commands = manifest.get("commands")
    if not isinstance(commands, list):
        return False, None, errors + ["commands ledger missing"]
    if len(commands) > len(EXPECTED_COMMANDS) or len(executables) != 5:
        expected_rows: list[dict[str, Any]] = []
    else:
        expected_rows = expected_command_contract(
            source, package, executables, environment
        )[: len(commands)]
    observed = [
        (row.get("name"), row.get("phase"))
        if isinstance(row, dict)
        else (None, None)
        for row in commands
    ]
    if observed != list(EXPECTED_COMMANDS[: len(commands)]):
        errors.append("command sequence mismatch")
    for index, row in enumerate(commands, 1):
        if not isinstance(row, dict) or row.get("ordinal") != index:
            errors.append("command ordinal mismatch")
            continue
        if set(row) != FULL_COMMAND_ROW_KEYS:
            errors.append(f"command key set mismatch: {row.get('name')}")
        if index <= len(expected_rows):
            expected = expected_rows[index - 1]
            for field in (
                "name",
                "phase",
                "argv",
                "cwd",
                "environment_overrides",
                "stdout_path",
                "stderr_path",
            ):
                if row.get(field) != expected[field]:
                    errors.append(f"command {field} mismatch: {row.get('name')}")
        for stream in ("stdout", "stderr"):
            relative = row.get(f"{stream}_path")
            digest = row.get(f"{stream}_sha256")
            if not isinstance(relative, str):
                errors.append(f"command {stream} path missing: {row.get('name')}")
                continue
            pure = pathlib.PurePosixPath(relative)
            path = package / relative
            if (
                pure.is_absolute()
                or ".." in pure.parts
                or not path.is_file()
                or path.is_symlink()
                or digest != sha256_file(path)
            ):
                errors.append(f"command {stream} byte mismatch: {row.get('name')}")
        if not isinstance(row.get("started_at_utc"), str) or not isinstance(
            row.get("finished_at_utc"), str
        ):
            errors.append(f"command timestamp missing: {row.get('name')}")
        elif row["started_at_utc"] > row["finished_at_utc"]:
            errors.append(f"command timestamp order mismatch: {row.get('name')}")

    if any(not isinstance(row, dict) for row in commands):
        return False, None, errors

    candidates = [
        row
        for row in commands
        if isinstance(row, dict) and row.get("name") == "candidate"
    ]
    validators = [
        row
        for row in commands
        if isinstance(row, dict) and row.get("name") == "validator"
    ]
    prefix_stopped_before_candidate = len(candidates) == 0 and len(validators) == 0
    if not prefix_stopped_before_candidate and len(commands) != len(EXPECTED_COMMANDS):
        errors.append("complete candidate command ledger length mismatch")
    if prefix_stopped_before_candidate:
        if not commands:
            errors.append("empty executed command prefix")
        else:
            failed = commands[-1]
            if (
                failed.get("name") == "candidate"
                or (
                    failed.get("launch_status") == "LAUNCHED"
                    and failed.get("exit_code") == 0
                )
            ):
                errors.append("executed prefix has no terminal preflight failure")
            else:
                preflight_failure = str(failed.get("name"))
            for row in commands[:-1]:
                if row.get("exit_code") != 0 or row.get("launch_status") != "LAUNCHED":
                    errors.append(
                        f"executed prefix has an earlier failure: {row.get('name')}"
                    )
    elif len(candidates) != 1 or len(validators) != 1:
        errors.append("candidate/validator command count mismatch")
    else:
        candidate_index = commands.index(candidates[0])
        for row in commands[:candidate_index]:
            if row.get("exit_code") != 0 or row.get("launch_status") != "LAUNCHED":
                preflight_failure = str(row.get("name"))
                break
        if preflight_failure is not None:
            errors.append("candidate was recorded after a failed candidate-free command")
        for row in commands[candidate_index + 2 :]:
            if row.get("exit_code") != 0 or row.get("launch_status") != "LAUNCHED":
                errors.append(f"post-candidate source command failed: {row.get('name')}")
        if candidates[0].get("launch_status") != "LAUNCHED":
            errors.append("candidate was not recorded as launched")

    if errors:
        return False, preflight_failure, errors

    expected_source = expected_source_identity()
    source_snapshots = {
        "source_pre": source_snapshot_from_rows(commands[0:9], package, "source_pre"),
        "source_prelaunch": source_snapshot_from_rows(
            commands[21:30], package, "source_prelaunch"
        ),
        "source_post": source_snapshot_from_rows(
            commands[32:41], package, "source_post"
        ),
    }
    for key, reconstructed in source_snapshots.items():
        if reconstructed != expected_source:
            errors.append(f"reconstructed {key} identity mismatch")
        if manifest.get(key) != reconstructed:
            errors.append(f"manifest {key} snapshot mismatch")
    if (
        manifest.get("scientific_implementation_provenance")
        != SCIENTIFIC_IMPLEMENTATION_PROVENANCE
    ):
        errors.append("manifest scientific implementation provenance mismatch")
    if manifest.get("declarations") != EXPECTED_DECLARATIONS:
        errors.append("manifest declarations mismatch")
    source_hashes = manifest.get("source_hashes")
    if not isinstance(source_hashes, dict) or set(source_hashes) != set(
        EXPECTED_SOURCE_HASHES
    ):
        errors.append("manifest source_hashes set mismatch")
    else:
        for relative, expected in EXPECTED_SOURCE_HASHES.items():
            row = source_hashes.get(relative)
            if (
                not isinstance(row, dict)
                or set(row) != SOURCE_HASH_ROW_KEYS
                or row.get("expected") != expected
                or row.get("observed") != expected
                or row.get("match") is not True
            ):
                errors.append(f"manifest source hash mismatch: {relative}")

    expected_authority = expected_authority_verification(EXPECTED_SOURCE_HASHES)
    if authority_verification != expected_authority:
        errors.append("authority verification artifact mismatch")
    if manifest.get("authority_preflight") != authority_verification:
        errors.append("authority verification manifest mismatch")

    if not isinstance(events, list) or len(events) != 2 * len(commands) + 2:
        errors.append("event count mismatch")
    else:
        run_created = events[0]
        if run_created != {
            "schema": "vigilode-audit2-bateman-execution-event/v1",
            "ordinal": 1,
            "event": "run_created",
            "utc": manifest.get("utc_start"),
            "run_id": manifest.get("run_id"),
        }:
            errors.append("run-created event mismatch")
        for index, row in enumerate(commands):
            started = events[1 + 2 * index]
            finished = events[2 + 2 * index]
            expected_started = {
                "schema": "vigilode-audit2-bateman-execution-event/v1",
                "ordinal": 2 + 2 * index,
                "event": "command_started",
                "name": row.get("name"),
                "phase": row.get("phase"),
                "utc": row.get("started_at_utc"),
                "argv": row.get("argv"),
            }
            expected_finished = {
                "schema": "vigilode-audit2-bateman-execution-event/v1",
                "ordinal": 3 + 2 * index,
                "event": "command_finished",
                "name": row.get("name"),
                "phase": row.get("phase"),
                "utc": row.get("finished_at_utc"),
                "launch_status": row.get("launch_status"),
                "exit_code": row.get("exit_code"),
            }
            if started != expected_started or finished != expected_finished:
                errors.append(f"event state mismatch: {row.get('name')}")
        sealing = events[-1]
        if sealing != {
            "schema": "vigilode-audit2-bateman-execution-event/v1",
            "ordinal": len(events),
            "event": "sealing_started",
            "utc": manifest.get("utc_end"),
            "verdict": manifest.get("verdict"),
        }:
            errors.append("sealing-started event mismatch")
    return not errors, preflight_failure, errors


def report_has_frozen_shape(report: Any, expected_hashes: Mapping[str, str]) -> bool:
    if not isinstance(report, dict):
        return False
    if (
        report.get("schema") != REPORT_SCHEMA
        or report.get("claim_scope") != REPORT_CLAIM_SCOPE
        or report.get("client_id") != CLIENT_ID
        or report.get("authority_manifest_sha256") != expected_hashes[MANIFEST_PATH]
        or report.get("exact_verifier_sha256") != expected_hashes[VERIFIER_PATH]
        or report.get("authority_proof_sha256") != expected_hashes[PROOF_PATH]
    ):
        return False
    plan = report.get("scenario_plan")
    receipts = report.get("scenario_receipts")
    if not isinstance(plan, list) or not isinstance(receipts, list) or len(plan) != 6 or len(receipts) != 6:
        return False
    for index, (scenario, operator_case, kind, disposition) in enumerate(
        EXPECTED_SCENARIOS, 1
    ):
        planned = plan[index - 1]
        receipt = receipts[index - 1]
        if not isinstance(planned, dict) or not isinstance(receipt, dict):
            return False
        if (
            planned.get("ordinal") != index
            or planned.get("scenario_id") != scenario
            or planned.get("operator_case_id") != operator_case
            or planned.get("kind") != kind
            or receipt.get("ordinal") != index
            or receipt.get("scenario_id") != scenario
            or receipt.get("operator_case_id") != operator_case
            or receipt.get("kind") != kind
            or receipt.get("disposition") != disposition
            or type(receipt.get("contract_satisfied")) is not bool
        ):
            return False
    return True


def report_is_success(report: Any, expected_hashes: Mapping[str, str]) -> bool:
    return bool(
        report_has_frozen_shape(report, expected_hashes)
        and report.get("all_six_executed") is True
        and report.get("all_contracts_satisfied") is True
        and report.get("terminal_failure") is None
        and all(
            row.get("contract_satisfied") is True
            for row in report["scenario_receipts"]
        )
    )


def report_is_structured_reject(
    report: Any, expected_hashes: Mapping[str, str]
) -> bool:
    return bool(
        report_has_frozen_shape(report, expected_hashes)
        and report.get("all_six_executed") is True
        and (
            report.get("all_contracts_satisfied") is False
            or report.get("terminal_failure") is not None
            or any(
                row.get("contract_satisfied") is False
                for row in report["scenario_receipts"]
            )
        )
    )


def result_template(verdict: str) -> dict[str, Any]:
    eligible = verdict in {ACCEPT_VERDICT, REJECT_VERDICT}
    return {
        "schema": "vigilode-audit2-bateman-independent-adjudication/v1",
        "verdict": verdict,
        "evidence_eligibility": "ELIGIBLE" if eligible else "INELIGIBLE_OR_UNRESOLVED",
        "candidate_outcome": "NOT_ESTABLISHED",
        "claim_ceiling_after_review": GLOBAL_CLAIM_CEILING,
        "adjudicator": {
            "path": str(pathlib.Path(__file__).resolve()),
            "sha256": sha256_file(pathlib.Path(__file__).resolve()),
        },
        "math_blocker_coverage": {
            "X01": "EVALUATED_BY_SEALED_SIX_CASE_PACKAGE" if eligible else "NOT_ESTABLISHED",
            "M01": "PREEXISTING_MATHEMATICAL_AUTHORITY",
            "M02": "PARTIALLY_EVALUATED" if eligible else "NOT_EVALUATED",
            "M03": "PARTIALLY_EVALUABLE_IF_PER_SOLVE_TELEMETRY_PRESENT" if eligible else "NOT_EVALUATED",
            **{identifier: "NOT_EVALUATED" for identifier in ("M04", "M05", "M06", "M07", "M08", "M09", "M10", "M11", "M12", "X02")},
        },
        "limitations": [
            "The changed-W cache probe does not establish changed-W output accuracy.",
            "The compact receipt omits raw vectors needed to independently recompute embedded, original-target residual, and contraction scalars.",
            "M04-M12 and X02 remain NOT_EVALUATED because q_i, a_i, c_i, Theta, drift/FOV, Lipschitz, refinement, and timing evidence are absent.",
            "Unkeyed SHA-256 binds bytes but does not attest the executing host or all out-of-band behavior.",
        ],
        "errors": [],
    }


def adjudicate_package(
    package: pathlib.Path | str,
    source_worktree: pathlib.Path | str,
    out: pathlib.Path | str,
    *,
    command_runner: Callable[..., subprocess.CompletedProcess[bytes]] = default_command_runner,
) -> dict[str, Any]:
    package_path = pathlib.Path(package).resolve()
    source = pathlib.Path(source_worktree).resolve()
    out_path = pathlib.Path(out).resolve()
    if not package_path.is_dir() or not source.is_dir():
        raise AdjudicationInputError("package and source worktree must be directories")
    if inside(out_path, package_path) or inside(out_path, source):
        raise AdjudicationInputError("adjudication sidecar must be outside package and source")
    if out_path.exists():
        raise AdjudicationInputError("adjudication sidecar path already exists")
    out_path.mkdir(parents=True)

    seal_ok, seal_errors = verify_seal(package_path)
    if not seal_ok:
        result = result_template("INCONCLUSIVE_PACKAGE_INTEGRITY")
        result["errors"] = seal_errors
        write_json_new(out_path / "adjudication.json", result)
        return result

    package_files = regular_files(package_path)
    always_required = {
        "execution_manifest.json",
        "events.jsonl",
        "validator_attempt.json",
        "authority_bundle_sha256.txt",
        "SHA256SUMS",
    }
    missing = sorted(always_required - package_files)
    if missing:
        result = result_template("INCONCLUSIVE_PACKAGE_INTEGRITY")
        result["errors"] = [f"required artifacts missing: {', '.join(missing)}"]
        write_json_new(out_path / "adjudication.json", result)
        return result

    try:
        manifest = strict_json_file(package_path / "execution_manifest.json")
        validator_attempt = strict_json_file(package_path / "validator_attempt.json")
        events = parse_events(package_path / "events.jsonl")
    except (OSError, ValueError, json.JSONDecodeError) as error:
        result = result_template("INCONCLUSIVE_PROTOCOL_VIOLATION")
        result["errors"] = [str(error)]
        write_json_new(out_path / "adjudication.json", result)
        return result

    commands = manifest.get("commands") if isinstance(manifest, dict) else None
    candidate_free = bool(
        isinstance(commands, list)
        and not any(
            isinstance(row, dict)
            and (row.get("name") == "candidate" or row.get("name") == "validator")
            for row in commands
        )
    )
    if candidate_free:
        protocol_ok, stop_stage, protocol_errors = validate_candidate_free_protocol(
            manifest, package_path, source, validator_attempt, events
        )
        if not protocol_ok:
            result = result_template("INCONCLUSIVE_PROTOCOL_VIOLATION")
            result["errors"] = protocol_errors
            write_json_new(out_path / "adjudication.json", result)
            return result

        required_files, allowed_files = candidate_free_file_contract(
            commands, manifest
        )
        missing_files = sorted(required_files - package_files)
        unexpected_files = sorted(package_files - allowed_files)
        if missing_files or unexpected_files:
            result = result_template("INCONCLUSIVE_PACKAGE_INTEGRITY")
            result["errors"] = []
            if missing_files:
                result["errors"].append(
                    "stage-derived artifacts missing: " + ", ".join(missing_files)
                )
            if unexpected_files:
                result["errors"].append(
                    "stage-derived artifacts unexpected: "
                    + ", ".join(unexpected_files)
                )
            write_json_new(out_path / "adjudication.json", result)
            return result

        forbidden_candidate_files = {
            "candidate_launch.json",
            "result_summary.json",
            "local_receipt_validation.json",
            "logs/candidate.stderr.log",
            "logs/validator.stderr.log",
        }
        prefix_errors = [
            "candidate-only artifacts present in candidate-free package: "
            + ", ".join(sorted(forbidden_candidate_files & package_files))
        ] if forbidden_candidate_files & package_files else []
        one_shot = manifest.get("verdict") == "INCONCLUSIVE_ONE_SHOT_ALREADY_CONSUMED"
        attempt_path = package_path / "attempt_lock.json"
        if one_shot:
            try:
                attempt = strict_json_file(attempt_path)
            except (OSError, ValueError, json.JSONDecodeError):
                attempt = None
            key_material = attempt.get("key_material") if isinstance(attempt, dict) else None
            expected_key_material = {
                "implementation_head": IMPLEMENTATION_HEAD,
                "authority_manifest_sha256": EXPECTED_SOURCE_HASHES[MANIFEST_PATH],
                "example_target": "audit2_bateman_local_six_case",
                "protocol_version": PROTOCOL_VERSION,
            }
            if (
                not isinstance(attempt, dict)
                or attempt.get("schema")
                != "vigilode-audit2-bateman-one-shot-lock/v1"
                or key_material != expected_key_material
                or attempt.get("key_sha256")
                != sha256_bytes(
                    json.dumps(
                        expected_key_material,
                        sort_keys=True,
                        separators=(",", ":"),
                    ).encode()
                )
            ):
                prefix_errors.append("one-shot contention lock mismatch")
        elif attempt_path.exists():
            prefix_errors.append("attempt lock present before candidate boundary")
        if prefix_errors:
            result = result_template("INCONCLUSIVE_PROTOCOL_VIOLATION")
            result["errors"] = prefix_errors
        elif one_shot:
            result = result_template(
                "INCONCLUSIVE_NOT_RUN_ONE_SHOT_ALREADY_CONSUMED"
            )
            result["errors"] = ["candidate was not run because the one-shot key was already consumed"]
        elif manifest.get("verdict") == "INCONCLUSIVE_RUNNER_INTERNAL_FAILURE":
            result = result_template(
                "INCONCLUSIVE_NOT_RUN_RUNNER_INTERNAL_FAILURE"
            )
            result["errors"] = [f"runner stopped before candidate at: {stop_stage}"]
        else:
            result = result_template("INCONCLUSIVE_NOT_RUN_PREFLIGHT")
            result["errors"] = [f"runner stopped before candidate at: {stop_stage}"]
        write_json_new(out_path / "adjudication.json", result)
        return result

    if "authority_verification.json" not in package_files:
        result = result_template("INCONCLUSIVE_PACKAGE_INTEGRITY")
        result["errors"] = ["required artifacts missing: authority_verification.json"]
        write_json_new(out_path / "adjudication.json", result)
        return result
    try:
        authority_verification = strict_json_file(
            package_path / "authority_verification.json"
        )
    except (OSError, ValueError, json.JSONDecodeError) as error:
        result = result_template("INCONCLUSIVE_PROTOCOL_VIOLATION")
        result["errors"] = [str(error)]
        write_json_new(out_path / "adjudication.json", result)
        return result

    protocol_ok, preflight_failure, protocol_errors = validate_manifest_protocol(
        manifest, package_path, source, authority_verification, events
    )
    if not protocol_ok:
        result = result_template("INCONCLUSIVE_PROTOCOL_VIOLATION")
        result["errors"] = protocol_errors
        write_json_new(out_path / "adjudication.json", result)
        return result

    expected_package_files = full_candidate_file_allowlist(manifest["commands"])
    if package_files != expected_package_files:
        missing_files = sorted(expected_package_files - package_files)
        unexpected_files = sorted(package_files - expected_package_files)
        errors = []
        if missing_files:
            errors.append(
                "stage-derived artifacts missing: " + ", ".join(missing_files)
            )
        if unexpected_files:
            errors.append(
                "stage-derived artifacts unexpected: "
                + ", ".join(unexpected_files)
            )
        result = result_template("INCONCLUSIVE_PACKAGE_INTEGRITY")
        result["errors"] = errors
        write_json_new(out_path / "adjudication.json", result)
        return result

    recorded_tools = manifest["tool_versions"]
    try:
        trusted_git = trusted_host_executable("git", package_path, source)
    except AdjudicationInputError as error:
        result = result_template("INCONCLUSIVE_SOURCE_IDENTITY")
        result["errors"] = [str(error)]
        write_json_new(out_path / "adjudication.json", result)
        return result
    source_ok, source_observed, source_errors = source_identity(
        source,
        command_runner,
        git_executable=trusted_git,
        environment=None,
    )
    observed_hashes: dict[str, str | None] = {}
    for relative, expected in EXPECTED_SOURCE_HASHES.items():
        path = source / relative
        observed_hashes[relative] = sha256_file(path) if path.is_file() else None
        if observed_hashes[relative] != expected:
            source_errors.append(f"source hash mismatch: {relative}")
    source_ok = source_ok and not source_errors
    if not source_ok:
        result = result_template("INCONCLUSIVE_SOURCE_IDENTITY")
        result["errors"] = source_errors
        result["source_observed"] = source_observed
        write_json_new(out_path / "adjudication.json", result)
        return result

    if preflight_failure is not None:
        candidate_only = {
            "attempt_lock.json",
            "candidate_launch.json",
            "result_summary.json",
            "local_receipt_validation.json",
            "logs/candidate.stderr.log",
            "logs/validator.stderr.log",
        }
        expected_candidate = {
            "invocation_count": 0,
            "launch_status": "NOT_RUN",
            "exit_code": None,
            "argv": list(CANDIDATE_ARGV),
        }
        expected_validator = {
            "attempt_count": 0,
            "launch_status": "NOT_RUN",
            "exit_code": None,
        }
        expected_validator_attempt = {
            "schema": "vigilode-audit2-bateman-validator-attempt/v1",
            "attempted": False,
            "attempt_count": 0,
            "launch_status": "NOT_RUN",
            "exit_code": None,
            "argv": None,
            "stdout_path": None,
            "stderr_path": None,
            "stdout_sha256": None,
            "stderr_sha256": None,
        }
        prefix_errors: list[str] = []
        present_candidate_only = sorted(candidate_only & package_files)
        if present_candidate_only:
            prefix_errors.append(
                "candidate-only artifacts present after preflight failure: "
                + ", ".join(present_candidate_only)
            )
        if manifest.get("candidate") != expected_candidate:
            prefix_errors.append("candidate was not recorded as unattempted")
        if manifest.get("validator") != expected_validator:
            prefix_errors.append("validator was not recorded as unattempted")
        if validator_attempt != expected_validator_attempt:
            prefix_errors.append("validator-attempt sidecar mismatch for not-run prefix")
        if prefix_errors:
            result = result_template("INCONCLUSIVE_PROTOCOL_VIOLATION")
            result["errors"] = prefix_errors
        else:
            result = result_template("INCONCLUSIVE_NOT_RUN_PREFLIGHT")
            result["errors"] = [
                "candidate-free command failed before candidate: "
                f"{preflight_failure}"
            ]
        write_json_new(out_path / "adjudication.json", result)
        return result

    candidate_required = {
        "attempt_lock.json",
        "candidate_launch.json",
        "result_summary.json",
        "local_receipt_validation.json",
        "logs/candidate.stderr.log",
        "logs/validator.stderr.log",
    }
    missing = sorted(candidate_required - package_files)
    if missing:
        result = result_template("INCONCLUSIVE_PACKAGE_INTEGRITY")
        result["errors"] = [
            f"candidate artifacts missing: {', '.join(missing)}"
        ]
        write_json_new(out_path / "adjudication.json", result)
        return result
    try:
        attempt = strict_json_file(package_path / "attempt_lock.json")
        candidate_launch = strict_json_file(package_path / "candidate_launch.json")
    except (OSError, ValueError, json.JSONDecodeError) as error:
        result = result_template("INCONCLUSIVE_PROTOCOL_VIOLATION")
        result["errors"] = [str(error)]
        write_json_new(out_path / "adjudication.json", result)
        return result

    commands = manifest["commands"]
    candidate_row = next(row for row in commands if row["name"] == "candidate")
    validator_row = next(row for row in commands if row["name"] == "validator")
    candidate_manifest = manifest.get("candidate", {})
    validator_manifest = manifest.get("validator", {})
    attempt_errors: list[str] = []
    if (
        not isinstance(attempt, dict)
        or set(attempt) != ATTEMPT_LOCK_KEYS
        or attempt.get("schema")
        != "vigilode-audit2-bateman-one-shot-lock/v1"
        or attempt.get("protocol_version") != PROTOCOL_VERSION
        or attempt.get("run_id") != manifest.get("run_id")
        or not isinstance(attempt.get("created_at_utc"), str)
    ):
        attempt_errors.append("one-shot attempt lock schema mismatch")
    key_material = attempt.get("key_material") if isinstance(attempt, dict) else None
    if isinstance(key_material, dict):
        if set(key_material) != ATTEMPT_KEY_MATERIAL_KEYS:
            attempt_errors.append("one-shot attempt key material key set mismatch")
        recomputed_guard = sha256_bytes(
            json.dumps(key_material, sort_keys=True, separators=(",", ":")).encode()
        )
        if attempt.get("key_sha256") != recomputed_guard:
            attempt_errors.append("one-shot attempt lock digest mismatch")
        expected_key_material = {
            "implementation_head": IMPLEMENTATION_HEAD,
            "authority_manifest_sha256": EXPECTED_SOURCE_HASHES[MANIFEST_PATH],
            "example_target": "audit2_bateman_local_six_case",
            "protocol_version": PROTOCOL_VERSION,
        }
        if key_material != expected_key_material:
            attempt_errors.append("one-shot attempt key material mismatch")
    else:
        attempt_errors.append("one-shot attempt key material missing")
    expected_launch = (
        {**attempt, "state": "CANDIDATE_LAUNCH_COMMITTED"}
        if isinstance(attempt, dict)
        else None
    )
    if (
        not isinstance(candidate_launch, dict)
        or set(candidate_launch) != ATTEMPT_LOCK_KEYS | {"state"}
        or candidate_launch != expected_launch
    ):
        attempt_errors.append("candidate launch commitment mismatch")
    if (
        isinstance(attempt, dict)
        and isinstance(attempt.get("created_at_utc"), str)
        and isinstance(candidate_row.get("started_at_utc"), str)
        and attempt["created_at_utc"] > candidate_row["started_at_utc"]
    ):
        attempt_errors.append("candidate launch predates one-shot reservation")
    if (
        not isinstance(candidate_manifest, dict)
        or candidate_manifest.get("invocation_count") != 1
        or candidate_manifest.get("guard_key_sha256")
        != (attempt.get("key_sha256") if isinstance(attempt, dict) else None)
    ):
        attempt_errors.append("candidate invocation/guard mismatch")
    if (
        candidate_manifest.get("exit_code") != candidate_row.get("exit_code")
        or candidate_manifest.get("launch_status")
        != candidate_row.get("launch_status")
        or candidate_manifest.get("argv") != candidate_row.get("argv")
        or candidate_manifest.get("result_summary_path")
        != candidate_row.get("stdout_path")
        or candidate_manifest.get("stderr_path") != candidate_row.get("stderr_path")
    ):
        attempt_errors.append("candidate exit mismatch")
    if (
        not isinstance(validator_manifest, dict)
        or validator_manifest.get("attempt_count") != 1
        or validator_manifest.get("exit_code") != validator_row.get("exit_code")
        or validator_manifest.get("launch_status")
        != validator_row.get("launch_status")
        or validator_manifest.get("argv") != validator_row.get("argv")
        or validator_manifest.get("stdout_path") != validator_row.get("stdout_path")
        or validator_manifest.get("stderr_path") != validator_row.get("stderr_path")
    ):
        attempt_errors.append("validator attempt/exit mismatch")
    result_path = package_path / "result_summary.json"
    result_hash = sha256_file(result_path)
    if (
        candidate_manifest.get("result_summary_sha256") != result_hash
        or candidate_row.get("stdout_sha256") != result_hash
    ):
        attempt_errors.append("candidate report hash mismatch")
    runner_validator_stdout = package_path / "local_receipt_validation.json"
    runner_validator_stderr = package_path / "logs/validator.stderr.log"
    runner_validator_stdout_hash = sha256_file(runner_validator_stdout)
    runner_validator_stderr_hash = sha256_file(runner_validator_stderr)
    if (
        validator_manifest.get("stdout_sha256") != runner_validator_stdout_hash
        or validator_manifest.get("stderr_sha256") != runner_validator_stderr_hash
        or validator_row.get("stdout_sha256") != runner_validator_stdout_hash
        or validator_row.get("stderr_sha256") != runner_validator_stderr_hash
    ):
        attempt_errors.append("runner validator stream hash mismatch")
    expected_validator_attempt = {
        "schema": "vigilode-audit2-bateman-validator-attempt/v1",
        "attempted": True,
        "attempt_count": 1,
        "launch_status": validator_manifest.get("launch_status"),
        "exit_code": validator_manifest.get("exit_code"),
        "argv": validator_manifest.get("argv"),
        "stdout_path": validator_manifest.get("stdout_path"),
        "stderr_path": validator_manifest.get("stderr_path"),
        "stdout_sha256": validator_manifest.get("stdout_sha256"),
        "stderr_sha256": validator_manifest.get("stderr_sha256"),
    }
    if validator_attempt != expected_validator_attempt:
        attempt_errors.append("validator_attempt artifact mismatch")

    expected_bundle = "".join(
        f"{digest}  {relative}\n"
        for relative, digest in sorted(EXPECTED_SOURCE_HASHES.items())
    ).encode()
    if (package_path / "authority_bundle_sha256.txt").read_bytes() != expected_bundle:
        attempt_errors.append("authority bundle digest listing mismatch")

    tool_versions = manifest.get("tool_versions", {})
    executable_rows = tool_versions.get("executables", {})
    for name, row in (
        executable_rows.items() if isinstance(executable_rows, dict) else ()
    ):
        path = pathlib.Path(row.get("path", "")) if isinstance(row, dict) else pathlib.Path("")
        realpath = path.resolve()
        if (
            not realpath.is_file()
            or str(realpath) != row.get("realpath")
            or sha256_file(realpath) != row.get("sha256")
            or realpath.stat().st_size != row.get("size_bytes")
        ):
            attempt_errors.append(f"executable bytes changed or missing: {name}")
    controller = tool_versions.get("controller", {})
    controller_path = pathlib.Path(controller.get("path", "")) if isinstance(controller, dict) else pathlib.Path("")
    if (
        not controller_path.is_file()
        or controller_path.is_symlink()
        or sha256_file(controller_path) != EXPECTED_RUNNER_SHA256
    ):
        attempt_errors.append("published runner bytes unavailable or changed")
    candidate_binary = tool_versions.get("candidate_binary", {})
    target_dir = tool_versions.get("environment_policy", {}).get("effective", {}).get(
        "CARGO_TARGET_DIR"
    )
    expected_binary_path = (
        pathlib.Path(target_dir) / "debug/examples/audit2_bateman_local_six_case"
        if isinstance(target_dir, str)
        else None
    )
    binary_path = (
        pathlib.Path(candidate_binary.get("path", ""))
        if isinstance(candidate_binary, dict)
        else pathlib.Path("")
    )
    if (
        expected_binary_path is None
        or binary_path != expected_binary_path
        or not binary_path.is_file()
        or binary_path.is_symlink()
        or candidate_binary.get("sha256_prelaunch")
        != candidate_binary.get("sha256_post")
        or sha256_file(binary_path) != candidate_binary.get("sha256_post")
        or binary_path.stat().st_size != candidate_binary.get("size_bytes")
        or candidate_manifest.get("binary_sha256_prelaunch")
        != candidate_binary.get("sha256_prelaunch")
        or candidate_manifest.get("binary_sha256_post")
        != candidate_binary.get("sha256_post")
    ):
        attempt_errors.append("candidate binary byte identity mismatch")

    try:
        recorded_validator = strict_json_file(runner_validator_stdout)
    except (OSError, ValueError, json.JSONDecodeError):
        recorded_validator = None
    if attempt_errors:
        result = result_template("INCONCLUSIVE_PROTOCOL_VIOLATION")
        result["errors"] = attempt_errors
        write_json_new(out_path / "adjudication.json", result)
        return result

    try:
        report = strict_json_file(result_path)
        report_parse_error = None
    except (OSError, ValueError, json.JSONDecodeError) as error:
        report = None
        report_parse_error = str(error)
    structured_reject = report_is_structured_reject(report, EXPECTED_SOURCE_HASHES)

    runner_validator_exit = validator_row.get("exit_code")
    runner_validator_stderr_bytes = runner_validator_stderr.read_bytes()
    if (
        validator_row.get("launch_status") != "LAUNCHED"
        or runner_validator_exit not in {0, 1}
        or (
            runner_validator_exit == 0
            and (
                runner_validator_stderr_bytes != b""
                or not isinstance(recorded_validator, dict)
            )
        )
    ):
        result = result_template("INCONCLUSIVE_VALIDATOR_INFRASTRUCTURE")
        result["errors"] = ["runner validator infrastructure was not cleanly established"]
        result["package_sha256sums_sha256"] = sha256_file(package_path / "SHA256SUMS")
        result["result_summary_sha256"] = result_hash
        result["source_observed"] = source_observed
        write_json_new(out_path / "adjudication.json", result)
        return result

    try:
        validator_python = trusted_adjudicator_python(package_path, source)
    except AdjudicationInputError as error:
        result = result_template("INCONCLUSIVE_VALIDATOR_INFRASTRUCTURE")
        result["errors"] = [str(error)]
        result["package_sha256sums_sha256"] = sha256_file(
            package_path / "SHA256SUMS"
        )
        result["result_summary_sha256"] = result_hash
        result["source_observed"] = source_observed
        write_json_new(out_path / "adjudication.json", result)
        return result
    validator_argv = [
        validator_python,
        str(source / VALIDATOR_PATH),
        str(result_path),
    ]
    try:
        completed = command_runner(
            validator_argv,
            cwd=source,
            env=independent_validator_environment(validator_python),
        )
    except OSError as error:
        write_bytes_new(out_path / "validator.stdout.log", b"")
        write_bytes_new(
            out_path / "validator.stderr.log",
            (f"spawn failure: {error}\n").encode("utf-8", "backslashreplace"),
        )
        result = result_template("INCONCLUSIVE_VALIDATOR_INFRASTRUCTURE")
        result["errors"] = [f"independent validator spawn failure: {error}"]
        result["package_sha256sums_sha256"] = sha256_file(package_path / "SHA256SUMS")
        result["result_summary_sha256"] = result_hash
        result["source_observed"] = source_observed
        write_json_new(out_path / "adjudication.json", result)
        return result
    write_bytes_new(out_path / "validator.stdout.log", completed.stdout)
    write_bytes_new(out_path / "validator.stderr.log", completed.stderr)

    try:
        independent_validator = strict_json_bytes(completed.stdout) if completed.stdout else None
    except (ValueError, json.JSONDecodeError) as error:
        independent_validator = None
        report_parse_error = report_parse_error or f"validator output invalid: {error}"

    candidate_exit = candidate_row.get("exit_code")
    if (
        completed.returncode not in {0, 1}
        or (completed.returncode == 0 and completed.stderr != b"")
    ):
        result = result_template("INCONCLUSIVE_VALIDATOR_INFRASTRUCTURE")
        result["errors"] = ["independent validator infrastructure was not cleanly established"]
        result["package_sha256sums_sha256"] = sha256_file(package_path / "SHA256SUMS")
        result["result_summary_sha256"] = result_hash
        result["independent_validator_exit"] = completed.returncode
        result["source_observed"] = source_observed
        write_json_new(out_path / "adjudication.json", result)
        return result
    independent_ok = bool(
        completed.returncode == 0
        and completed.stderr == b""
        and isinstance(independent_validator, dict)
        and independent_validator.get("status") == "LOCAL_SIX_CASE_RECEIPT_VERIFIED"
        and independent_validator.get("scenario_count") == 6
        and independent_validator.get("report_sha256") == result_hash
        and independent_validator.get("claim_scope") == REPORT_CLAIM_SCOPE
    )
    runner_validator_ok = bool(
        runner_validator_exit == 0
        and runner_validator_stderr.read_bytes() == b""
        and isinstance(recorded_validator, dict)
        and recorded_validator.get("status") == "LOCAL_SIX_CASE_RECEIPT_VERIFIED"
        and recorded_validator.get("scenario_count") == 6
        and recorded_validator.get("report_sha256") == result_hash
        and recorded_validator.get("claim_scope") == REPORT_CLAIM_SCOPE
    )
    if (
        candidate_exit == 0
        and manifest.get("verdict") == RUNNER_ACCEPT_VERDICT
        and runner_validator_ok
        and independent_ok
        and report_is_success(report, EXPECTED_SOURCE_HASHES)
    ):
        result = result_template(ACCEPT_VERDICT)
        result["candidate_outcome"] = "SIX_SCENARIO_RECEIPT_VERIFIED"
    elif (
        structured_reject
        and manifest.get("verdict") == REJECT_VERDICT
        and runner_validator_exit == 1
        and completed.returncode == 1
    ):
        result = result_template(REJECT_VERDICT)
        result["candidate_outcome"] = "RECORDED_SCIENTIFIC_OR_STRUCTURAL_FAILURE"
    else:
        result = result_template(
            "INCONCLUSIVE_LAUNCH_OR_REPORT"
            if candidate_exit != 0
            else "INCONCLUSIVE_PROTOCOL_VIOLATION"
        )
        if report_parse_error:
            result["errors"] = [report_parse_error]

    result["package_sha256sums_sha256"] = sha256_file(package_path / "SHA256SUMS")
    result["result_summary_sha256"] = result_hash
    result["independent_validator_exit"] = completed.returncode
    result["source_observed"] = source_observed
    write_json_new(out_path / "adjudication.json", result)
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", required=True, type=pathlib.Path)
    parser.add_argument("--source-worktree", required=True, type=pathlib.Path)
    parser.add_argument("--out", required=True, type=pathlib.Path)
    args = parser.parse_args()
    result = adjudicate_package(args.package, args.source_worktree, args.out)
    print(json.dumps(result, sort_keys=True))
    return 0 if result["verdict"] == ACCEPT_VERDICT else (1 if result["verdict"] == REJECT_VERDICT else 2)


if __name__ == "__main__":
    raise SystemExit(main())
