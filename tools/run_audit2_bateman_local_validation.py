#!/usr/bin/env python3
"""Run and seal the frozen Audit-2 Bateman six-case protocol once.

This stdlib-only controller exposes no scientific knobs.  It validates the
published source and authority bytes, runs only candidate-free preflight work,
commits a durable one-shot marker, invokes the exact candidate command once,
attempts the frozen receipt validator once, and seals every retained byte.
"""

from __future__ import annotations

import argparse
import dataclasses
import datetime as dt
import hashlib
import json
import os
import pathlib
import platform
import pwd
import secrets
import shutil
import stat
import subprocess
import sys
from typing import Any, Mapping

PROTOCOL_VERSION = "vigilode-audit2-bateman-local-execution-orchestrator/v1"
EXECUTION_SOURCE_HEAD = "6b00a886c4eb38d3fe199e3d77852cc1eb35eb39"
EXECUTION_SOURCE_TREE = "4a9ede5c442514f1ae86d018419a2afeee5b6d01"
IMPLEMENTATION_HEAD = "cac7d1b7337a6dff25a60072009658f6ddf155d9"
IMPLEMENTATION_TREE = "c23abbee0d47e2dbe002e01516bf34e2481bc333"
IMPLEMENTATION_PARENT = "f954e39130e5141256731d0745666a872c0267ea"
IMPLEMENTATION_PARENT_TREE = "4314da2f9e1533737d4169526ebd2d84515ab19d"
BASE_HEAD = IMPLEMENTATION_PARENT
BASE_TREE = IMPLEMENTATION_PARENT_TREE

MANIFEST_PATH = "research/audit2_real_client_authority_construction_20260830/authority_manifest.json"
VERIFIER_PATH = "research/audit2_real_client_authority_construction_20260830/verify_authority_manifest.py"
PROOF_PATH = "research/audit2_real_client_authority_construction_20260830/evidence/AUTHORITY_VERIFICATION_RECEIPT.json"
VALIDATOR_PATH = "research/audit2_real_client_authority_construction_20260830/verify_local_six_case_receipt.py"
EXAMPLE_PATH = "crates/rodas5p-integrators/examples/audit2_bateman_local_six_case.rs"
READINESS_PATH = "tools/check-audit2-readiness.sh"
PROMPT_PATH = "research/audit2_real_client_authority_construction_20260830/CODEX_START_HERE.md"
HANDOFF_PATH = "research/audit2_real_client_authority_construction_20260830/handoff.json"

MANIFEST_SHA256 = "673045bf6b9e723fceb6a3b8df8e9e9e9075c942cf1c438f0ebd03574dbac360"
VERIFIER_SHA256 = "542715ca749efbf2060d608f2089ee8457e32f9c61fd0d35f613d5ecec26487d"
PROOF_SHA256 = "057cceba92fed0d707db1d586b53adebee5aed00583b224811d091f1d453ab12"

EXPECTED_FILE_HASHES: dict[str, str] = {
    MANIFEST_PATH: MANIFEST_SHA256,
    VERIFIER_PATH: VERIFIER_SHA256,
    PROOF_PATH: PROOF_SHA256,
    VALIDATOR_PATH: "8391e03e6f94f305f2675799c923d787547cad662cef8d8f8384a8c1bbe94e67",
    EXAMPLE_PATH: "0873ed8189a7e0f77ebd4eef05ce6067f84958e7f118aa3e686654e7dc3c48f9",
    READINESS_PATH: "74dc27607ff1fc764e3ea89912b333418c86a1dfb5e3c14764481b94821b7521",
    "Cargo.lock": "d9255cd442dfbca2890152549ae7edc60e890aa062a1046d8f0b8e44678d678a",
    "Cargo.toml": "86e27546665f923265a8addd3c464ac6017fe35558ab95fe0af7248cd99fb73b",
    "rust-toolchain.toml": "f53198ae4fdecfd87da36fe431c771b54c51e975d01c0e99f653bc14d5d48211",
    PROMPT_PATH: "ce96761b5cd067fe21e8d01e52a74767a65a8d9eaaa8c2c18ed1db8ca47de776",
    HANDOFF_PATH: "391861375a01a772e918aad28cfee887600b929cb7ed6b00b555a8bbc2aadb91",
}

EXPECTED_PYTHON_VERSION = "3.12.13"
EXPECTED_NUMPY_VERSION = "2.3.5"
EXPECTED_MPMATH_VERSION = "1.3.0"
EXPECTED_RUST_VERSION = "1.94.1"

EXPECTED_AUTHORITY_VERIFICATION = {
    "candidate_executions": 0,
    "declared_reference_l2_uncertainty": 1e-15,
    "execution_scenarios": 6,
    "fast_exponent_exceeds_one": True,
    "holdout_access": "NOT_OPENED_OR_EXECUTED",
    "local_six_case_status": "NOT_RUN_DURING_AUTHORITY_CONSTRUCTION",
    "max_reference_l2_bound": 2.075243427511439e-17,
    "receipt_sha256": PROOF_SHA256,
    "status": "AUTHORITY_CONSTRUCTION_VERIFIED",
    "verified_operator_cases": 2,
}

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
ACCEPT_VERDICT = (
    "ACCEPT_BOUNDED_EXACT_BATEMAN_CLIENT_MANIFEST_TWO_OPERATOR_CASES_SIX_SCENARIOS_ONLY"
)
REJECT_VERDICT = "REJECT_BOUNDED_EXACT_BATEMAN_SIX_SCENARIO_CLAIM"

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

FORBIDDEN_BUILD_ENVIRONMENT = frozenset(
    {
        "AR",
        "CC",
        "CFLAGS",
        "CXX",
        "CXXFLAGS",
        "RANLIB",
        "RUSTC",
        "RUSTC_WRAPPER",
        "RUSTFLAGS",
        "CARGO_BUILD_RUSTC",
        "CARGO_BUILD_RUSTC_WRAPPER",
        "CARGO_BUILD_TARGET",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_TARGET_DIR",
    }
)
PRESERVED_ENVIRONMENT = ("USER", "LOGNAME", "TMPDIR")


@dataclasses.dataclass(frozen=True)
class CommandTemplate:
    name: str
    phase: str
    argv: tuple[str, ...]
    stdout_path: str
    stderr_path: str
    environment: tuple[tuple[str, str], ...] = ()


PREFLIGHT_COMMANDS = (
    CommandTemplate("python_dependencies", "environment", ("{python}", "-c", "import json,platform,numpy,mpmath; print(json.dumps({'python':platform.python_version(),'numpy':numpy.__version__,'mpmath':mpmath.__version__},sort_keys=True))"), "logs/python_dependencies.stdout.log", "logs/python_dependencies.stderr.log"),
    CommandTemplate("rustc_version", "environment", ("{rustc}", "--version"), "logs/rustc_version.stdout.log", "logs/rustc_version.stderr.log"),
    CommandTemplate("cargo_version", "environment", ("{cargo}", "--version"), "logs/cargo_version.stdout.log", "logs/cargo_version.stderr.log"),
    CommandTemplate("authority_verification", "preflight", ("{python}", VERIFIER_PATH), "authority_verification.json", "logs/authority_verification.stderr.log"),
    CommandTemplate("python_authority_tests", "preflight", ("{python}", "tools/test_audit2_real_client_authority.py", "-v"), "logs/python_authority_tests.stdout.log", "logs/python_authority_tests.stderr.log"),
    CommandTemplate("python_local_receipt_tests", "preflight", ("{python}", "tools/test_audit2_bateman_local_receipt.py", "-v"), "logs/python_local_receipt_tests.stdout.log", "logs/python_local_receipt_tests.stderr.log"),
    CommandTemplate("rust_authority_contracts", "preflight", ("{cargo}", "test", "--locked", "-p", "rodas5p-integrators", "--features", "audit2-bateman-authority", "--test", "audit2_real_client_authority_contracts", "--", "--nocapture", "--test-threads=1"), "logs/rust_authority_contracts.stdout.log", "logs/rust_authority_contracts.stderr.log"),
    CommandTemplate("readiness", "preflight", ("{bash}", READINESS_PATH), "logs/readiness.stdout.log", "logs/readiness.stderr.log", (("AUDIT2_OUTPUT_DIR", "{readiness_output}"),)),
    CommandTemplate("clippy", "preflight", ("{cargo}", "clippy", "--locked", "-p", "rodas5p-integrators", "-p", "rodas5p-fair-ab", "--all-targets", "--features", "rodas5p-integrators/audit2-bateman-authority", "--", "-D", "warnings"), "logs/clippy.stdout.log", "logs/clippy.stderr.log"),
    CommandTemplate("fmt", "preflight", ("{cargo}", "fmt", "--all", "--", "--check"), "logs/fmt.stdout.log", "logs/fmt.stderr.log"),
    CommandTemplate("diff_check", "preflight", ("{git}", "diff", "--check"), "logs/diff_check.stdout.log", "logs/diff_check.stderr.log"),
    CommandTemplate("cargo_build_candidate", "preflight", ("{cargo}", "build", "--locked", "-p", "rodas5p-integrators", "--features", "audit2-bateman-authority", "--example", "audit2_bateman_local_six_case"), "logs/cargo_build_candidate.stdout.log", "logs/cargo_build_candidate.stderr.log"),
)


@dataclasses.dataclass
class Command:
    name: str
    phase: str
    argv: list[str]
    cwd: pathlib.Path
    package_dir: pathlib.Path
    stdout_path: pathlib.Path | None
    stderr_path: pathlib.Path | None
    environment: dict[str, str]


@dataclasses.dataclass(frozen=True)
class RunOutcome:
    verdict: str
    package_dir: pathlib.Path
    exit_code: int


class SubprocessExecutor:
    """Literal-argv subprocess boundary used by the production CLI."""

    def execute(self, command: Command) -> int:
        if command.stdout_path is None or command.stderr_path is None:
            raise ValueError("command streams must be retained")
        command.stdout_path.parent.mkdir(parents=True, exist_ok=True)
        command.stderr_path.parent.mkdir(parents=True, exist_ok=True)
        with command.stdout_path.open("xb") as stdout, command.stderr_path.open("xb") as stderr:
            try:
                completed = subprocess.run(
                    command.argv,
                    cwd=command.cwd,
                    env=command.environment,
                    stdout=stdout,
                    stderr=stderr,
                    check=False,
                )
            except OSError as error:
                stderr.write(f"spawn failure: {error}\n".encode("utf-8", "backslashreplace"))
                stderr.flush()
                os.fsync(stderr.fileno())
                raise
            stdout.flush()
            stderr.flush()
            os.fsync(stdout.fileno())
            os.fsync(stderr.fileno())
            return completed.returncode


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def write_new(path: pathlib.Path, value: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("xb") as stream:
        stream.write(value)
        stream.flush()
        os.fsync(stream.fileno())


def write_json_new(path: pathlib.Path, value: Any) -> None:
    write_new(path, (json.dumps(value, indent=2, sort_keys=True) + "\n").encode())


def ensure_real_directory(path: pathlib.Path) -> None:
    path.mkdir(parents=True, exist_ok=True)
    absolute = path.absolute()
    current = pathlib.Path(absolute.anchor)
    for part in absolute.parts[1:]:
        current = current / part
        if current.is_symlink() or not current.is_dir():
            raise ValueError(f"unsafe state directory component: {current}")


def append_event(path: pathlib.Path, event: Mapping[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("ab") as stream:
        stream.write((json.dumps(dict(event), sort_keys=True) + "\n").encode())
        stream.flush()
        os.fsync(stream.fileno())


def strict_json(path: pathlib.Path) -> Any:
    def pairs(items: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in items:
            if key in result:
                raise ValueError(f"duplicate JSON key: {key}")
            result[key] = value
        return result

    def reject_constant(value: str) -> None:
        raise ValueError(f"non-finite JSON constant: {value}")

    return json.loads(
        path.read_bytes(), object_pairs_hook=pairs, parse_constant=reject_constant
    )


def resolve_executables(overrides: Mapping[str, str] | None) -> dict[str, str]:
    if overrides is not None:
        return dict(overrides)
    resolved = {
        "git": shutil.which("git"),
        "python": sys.executable,
        "cargo": shutil.which("cargo"),
        "rustc": shutil.which("rustc"),
        "bash": shutil.which("bash"),
    }
    missing = [name for name, value in resolved.items() if value is None]
    if missing:
        raise RuntimeError(f"required executables unavailable: {', '.join(missing)}")
    return {
        name: str(pathlib.Path(str(value)).expanduser().absolute())
        for name, value in resolved.items()
    }


def build_environment(
    executables: Mapping[str, str], cargo_target_dir: pathlib.Path
) -> tuple[dict[str, str], list[str]]:
    forbidden = sorted(
        key
        for key in os.environ
        if key in FORBIDDEN_BUILD_ENVIRONMENT
        or key.startswith("CARGO_TARGET_")
        or key.startswith(("CC_", "CFLAGS_", "CXX_", "CXXFLAGS_", "AR_", "RANLIB_"))
        or key.endswith(("_CC", "_CFLAGS", "_CXX", "_CXXFLAGS", "_AR", "_RANLIB"))
    )
    environment = {
        key: os.environ[key] for key in PRESERVED_ENVIRONMENT if key in os.environ
    }
    account_home = pwd.getpwuid(os.getuid()).pw_dir
    executable_dirs = [
        str(pathlib.Path(executables[name]).parent)
        for name in ("python", "cargo", "rustc", "git", "bash")
    ]
    environment.update(
        {
            "HOME": account_home,
            "PATH": os.pathsep.join(dict.fromkeys([*executable_dirs, "/usr/bin", "/bin"])),
            "LANG": "C",
            "LC_ALL": "C",
            "TZ": "UTC",
            "CARGO_NET_OFFLINE": "true",
            "CARGO_TERM_COLOR": "never",
            "CARGO_HOME": str(pathlib.Path(account_home) / ".cargo"),
            "CARGO_TARGET_DIR": str(cargo_target_dir),
            "PYTHONDONTWRITEBYTECODE": "1",
            "RUST_BACKTRACE": "0",
        }
    )
    return environment, forbidden


def executable_identities(
    executables: Mapping[str, str], *, require_files: bool
) -> dict[str, dict[str, Any]]:
    identities: dict[str, dict[str, Any]] = {}
    for name, raw_path in sorted(executables.items()):
        path = pathlib.Path(raw_path)
        realpath = path.resolve()
        if require_files and not realpath.is_file():
            raise RuntimeError(f"unsafe or missing executable: {name}: {path}")
        identities[name] = {
            "path": str(path),
            "realpath": str(realpath),
            "sha256": sha256_file(realpath) if realpath.is_file() else None,
            "size_bytes": realpath.stat().st_size if realpath.is_file() else None,
        }
    return identities


def command_from_template(
    template: CommandTemplate,
    source: pathlib.Path,
    package: pathlib.Path,
    executables: Mapping[str, str],
    base_environment: Mapping[str, str],
) -> Command:
    substitutions = {
        **executables,
        "readiness_output": str(package / "readiness-output"),
    }
    def substitute(value: str) -> str:
        for key, replacement in substitutions.items():
            value = value.replace("{" + key + "}", replacement)
        return value

    argv = [substitute(part) for part in template.argv]
    environment = dict(base_environment)
    for key, value in template.environment:
        environment[key] = substitute(value)
    return Command(
        template.name,
        template.phase,
        argv,
        source,
        package,
        package / template.stdout_path,
        package / template.stderr_path,
        environment,
    )


def source_templates(prefix: str, executables: Mapping[str, str]) -> tuple[CommandTemplate, ...]:
    git = executables["git"]
    return (
        CommandTemplate(f"{prefix}_head", "source", (git, "rev-parse", "HEAD"), f"logs/{prefix}_head.stdout.log", f"logs/{prefix}_head.stderr.log"),
        CommandTemplate(f"{prefix}_tree", "source", (git, "rev-parse", "HEAD^{tree}"), f"logs/{prefix}_tree.stdout.log", f"logs/{prefix}_tree.stderr.log"),
        CommandTemplate(f"{prefix}_parent", "source", (git, "rev-parse", "HEAD^"), f"logs/{prefix}_parent.stdout.log", f"logs/{prefix}_parent.stderr.log"),
        CommandTemplate(f"{prefix}_implementation_tree", "source", (git, "rev-parse", f"{IMPLEMENTATION_HEAD}^{{tree}}"), f"logs/{prefix}_implementation_tree.stdout.log", f"logs/{prefix}_implementation_tree.stderr.log"),
        CommandTemplate(f"{prefix}_implementation_parent", "source", (git, "rev-parse", f"{IMPLEMENTATION_HEAD}^"), f"logs/{prefix}_implementation_parent.stdout.log", f"logs/{prefix}_implementation_parent.stderr.log"),
        CommandTemplate(f"{prefix}_base_tree", "source", (git, "rev-parse", f"{IMPLEMENTATION_PARENT}^{{tree}}"), f"logs/{prefix}_base_tree.stdout.log", f"logs/{prefix}_base_tree.stderr.log"),
        CommandTemplate(f"{prefix}_execution_ancestry", "source", (git, "merge-base", "--is-ancestor", IMPLEMENTATION_HEAD, EXECUTION_SOURCE_HEAD), f"logs/{prefix}_execution_ancestry.stdout.log", f"logs/{prefix}_execution_ancestry.stderr.log"),
        CommandTemplate(f"{prefix}_implementation_ancestry", "source", (git, "merge-base", "--is-ancestor", IMPLEMENTATION_PARENT, IMPLEMENTATION_HEAD), f"logs/{prefix}_implementation_ancestry.stdout.log", f"logs/{prefix}_implementation_ancestry.stderr.log"),
        CommandTemplate(f"{prefix}_status", "source", (git, "status", "--porcelain=v1", "--untracked-files=all"), f"logs/{prefix}_status.stdout.log", f"logs/{prefix}_status.stderr.log"),
    )


def validate_source_values(values: Mapping[str, Any]) -> bool:
    return bool(
        values.get("head") == EXECUTION_SOURCE_HEAD
        and values.get("tree") == EXECUTION_SOURCE_TREE
        and values.get("parent") == IMPLEMENTATION_HEAD
        and values.get("implementation_tree") == IMPLEMENTATION_TREE
        and values.get("implementation_parent") == IMPLEMENTATION_PARENT
        and values.get("base_tree") == IMPLEMENTATION_PARENT_TREE
        and values.get("status") == ""
        and values.get("status_query_failed") is not True
        and values.get("execution_ancestry") is True
        and values.get("implementation_ancestry") is True
    )


def validate_report_shape(report: Any) -> bool:
    if not isinstance(report, dict):
        return False
    if (
        report.get("schema") != REPORT_SCHEMA
        or report.get("claim_scope") != REPORT_CLAIM_SCOPE
        or report.get("client_id") != CLIENT_ID
        or report.get("authority_manifest_sha256") != MANIFEST_SHA256
        or report.get("exact_verifier_sha256") != VERIFIER_SHA256
        or report.get("authority_proof_sha256") != PROOF_SHA256
    ):
        return False
    plan = report.get("scenario_plan")
    receipts = report.get("scenario_receipts")
    if not isinstance(plan, list) or not isinstance(receipts, list) or len(plan) != 6 or len(receipts) != 6:
        return False
    for index, expected in enumerate(EXPECTED_SCENARIOS, 1):
        scenario, operator, kind, disposition = expected
        planned = plan[index - 1]
        receipt = receipts[index - 1]
        if not isinstance(planned, dict) or not isinstance(receipt, dict):
            return False
        if (
            planned.get("ordinal") != index
            or planned.get("scenario_id") != scenario
            or planned.get("operator_case_id") != operator
            or planned.get("kind") != kind
            or receipt.get("ordinal") != index
            or receipt.get("scenario_id") != scenario
            or receipt.get("operator_case_id") != operator
            or receipt.get("kind") != kind
            or receipt.get("disposition") != disposition
            or receipt.get("contract_satisfied") is not True
        ):
            return False
    return True


def seal_package(package: pathlib.Path) -> None:
    sums = package / "SHA256SUMS"
    if sums.exists():
        raise FileExistsError(sums)
    regular: list[str] = []
    for path in package.rglob("*"):
        if path.is_symlink():
            raise ValueError(f"symlink forbidden in package: {path}")
        if path.is_file():
            regular.append(path.relative_to(package).as_posix())
    regular.sort()
    payload = "".join(
        f"{sha256_file(package / relative)}  {relative}\n" for relative in regular
    ).encode("ascii")
    write_new(sums, payload)


def run_protocol(
    source_worktree: pathlib.Path | str,
    *,
    executor: Any | None = None,
    executables: Mapping[str, str] | None = None,
) -> RunOutcome:
    source = pathlib.Path(source_worktree).resolve()
    state = CANONICAL_STATE_ROOT.absolute()
    if not source.is_dir():
        raise ValueError(f"source worktree is not a directory: {source}")
    if source == state or source in state.parents or state in source.parents:
        raise ValueError("state root and source worktree must be disjoint")
    ensure_real_directory(state)
    runs = state / "runs"
    ensure_real_directory(runs)
    run_id = f"{dt.datetime.now(dt.timezone.utc).strftime('%Y%m%dT%H%M%SZ')}-{secrets.token_hex(4)}"
    package = runs / run_id
    package.mkdir()
    (package / "logs").mkdir()
    (package / "readiness-output").mkdir()
    events_path = package / "events.jsonl"
    started = utc_now()
    append_event(events_path, {"schema": "vigilode-audit2-bateman-execution-event/v1", "ordinal": 1, "event": "run_created", "utc": started, "run_id": run_id})

    resolved = resolve_executables(executables)
    controller = executor or SubprocessExecutor()
    cargo_target_dir = state / "build-cache" / EXECUTION_SOURCE_HEAD / run_id
    ensure_real_directory(cargo_target_dir)
    environment, forbidden_environment = build_environment(resolved, cargo_target_dir)
    cargo_home = pathlib.Path(environment["CARGO_HOME"]).expanduser().absolute()
    cargo_config_files_present = [
        str(path)
        for path in (cargo_home / "config", cargo_home / "config.toml")
        if path.exists()
    ]
    if cargo_config_files_present:
        forbidden_environment.append("CARGO_HOME_CONFIG_PRESENT")
    recorded_environment = dict(sorted(environment.items()))
    controller_path = pathlib.Path(__file__).resolve()
    controller_sha256_pre = sha256_file(controller_path)

    records: list[dict[str, Any]] = []
    event_ordinal = 1

    def execute(command: Command) -> int:
        nonlocal event_ordinal
        ordinal = len(records) + 1
        began = utc_now()
        event_ordinal += 1
        append_event(events_path, {"schema": "vigilode-audit2-bateman-execution-event/v1", "ordinal": event_ordinal, "event": "command_started", "name": command.name, "phase": command.phase, "utc": began, "argv": command.argv})
        launch_status = "LAUNCHED"
        try:
            exit_code = int(controller.execute(command))
        except Exception as error:  # retain an operational spawn/controller failure
            launch_status = "SPAWN_FAILED"
            exit_code = 127
            if command.stdout_path is not None and not command.stdout_path.exists():
                write_new(command.stdout_path, b"")
            if command.stderr_path is not None and not command.stderr_path.exists():
                write_new(command.stderr_path, (f"executor failure: {error}\n").encode())
        finished = utc_now()
        record = {
            "ordinal": ordinal,
            "name": command.name,
            "phase": command.phase,
            "argv": command.argv,
            "cwd": str(command.cwd),
            "environment_overrides": {
                **recorded_environment,
                **(
                    {"AUDIT2_OUTPUT_DIR": command.environment["AUDIT2_OUTPUT_DIR"]}
                    if "AUDIT2_OUTPUT_DIR" in command.environment
                    else {}
                ),
            },
            "stdout_path": None if command.stdout_path is None else command.stdout_path.relative_to(package).as_posix(),
            "stderr_path": None if command.stderr_path is None else command.stderr_path.relative_to(package).as_posix(),
            "started_at_utc": began,
            "finished_at_utc": finished,
            "launch_status": launch_status,
            "exit_code": exit_code,
        }
        for label, path in (("stdout", command.stdout_path), ("stderr", command.stderr_path)):
            if path is not None and path.is_file():
                record[f"{label}_sha256"] = sha256_file(path)
        records.append(record)
        event_ordinal += 1
        append_event(events_path, {"schema": "vigilode-audit2-bateman-execution-event/v1", "ordinal": event_ordinal, "event": "command_finished", "name": command.name, "phase": command.phase, "utc": finished, "launch_status": launch_status, "exit_code": exit_code})
        return exit_code

    def run_source(prefix: str) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for template in source_templates(prefix, resolved):
            command = command_from_template(template, source, package, resolved, environment)
            code = execute(command)
            stdout = command.stdout_path.read_text(errors="replace").strip() if command.stdout_path else ""
            key = template.name.removeprefix(prefix + "_")
            if key.endswith("ancestry"):
                result[key] = code == 0
            elif key == "status":
                result[key] = stdout
                if code != 0:
                    result["status_query_failed"] = True
            else:
                result[key] = stdout if code == 0 else None
        result["valid"] = validate_source_values(result)
        return result

    verdict = "INCONCLUSIVE_RUNNER_INTERNAL_FAILURE"
    source_pre: dict[str, Any] | None = None
    source_prelaunch: dict[str, Any] | None = None
    source_post: dict[str, Any] | None = None
    source_hashes: dict[str, dict[str, Any]] = {}
    tool_versions: dict[str, Any] = {
        "executables": executable_identities(
            resolved, require_files=executor is None
        ),
        "platform": {
            "system": platform.system(),
            "release": platform.release(),
            "machine": platform.machine(),
            "python_runtime": platform.python_version(),
        },
        "controller": {
            "path": str(controller_path),
            "sha256_pre": controller_sha256_pre,
            "sha256_post": None,
        },
        "environment_policy": {
            "mode": "SANITIZED_ALLOWLIST_PLUS_FIXED_VALUES",
            "forbidden_ambient_detected": forbidden_environment,
            "cargo_config_files_present": cargo_config_files_present,
            "effective": recorded_environment,
        },
    }
    authority_preflight: dict[str, Any] = {}
    candidate: dict[str, Any] = {"invocation_count": 0, "launch_status": "NOT_RUN", "exit_code": None, "argv": list(CANDIDATE_ARGV)}
    validator: dict[str, Any] = {"attempt_count": 0, "launch_status": "NOT_RUN", "exit_code": None}
    marker_payload: dict[str, Any] | None = None

    try:
        if forbidden_environment:
            verdict = "INCONCLUSIVE_ENVIRONMENT_UNRESOLVED"
            write_json_new(
                package / "forbidden_environment.json",
                {"keys": forbidden_environment},
            )
        else:
            source_pre = run_source("source_pre")
        if source_pre is None:
            pass
        elif not source_pre["valid"]:
            verdict = "INCONCLUSIVE_SOURCE_IDENTITY_UNRESOLVED"
        else:
            all_hashes_match = True
            lines: list[str] = []
            for relative in sorted(EXPECTED_FILE_HASHES):
                expected = EXPECTED_FILE_HASHES[relative]
                path = source / relative
                observed = sha256_file(path) if path.is_file() else None
                match = observed == expected
                all_hashes_match &= match
                source_hashes[relative] = {"expected": expected, "observed": observed, "match": match}
                lines.append(f"{observed or 'MISSING'}  {relative}\n")
            write_new(package / "authority_bundle_sha256.txt", "".join(lines).encode())
            if not all_hashes_match:
                verdict = "INCONCLUSIVE_AUTHORITY_BUNDLE_MISMATCH"
            else:
                preflight_ok = True
                executable = (
                    cargo_target_dir
                    / "debug/examples/audit2_bateman_local_six_case"
                )
                for template in PREFLIGHT_COMMANDS:
                    if template.name == "cargo_build_candidate":
                        try:
                            ensure_real_directory(executable.parent)
                            try:
                                existing_mode = executable.lstat().st_mode
                            except FileNotFoundError:
                                existing_mode = None
                            if existing_mode is not None:
                                if stat.S_ISREG(existing_mode) or stat.S_ISLNK(
                                    existing_mode
                                ):
                                    executable.unlink()
                                else:
                                    preflight_ok = False
                                    verdict = (
                                        "INCONCLUSIVE_CANDIDATE_BINARY_UNRESOLVED"
                                    )
                                    break
                        except (OSError, ValueError):
                            preflight_ok = False
                            verdict = "INCONCLUSIVE_CANDIDATE_BINARY_UNRESOLVED"
                            break
                    command = command_from_template(template, source, package, resolved, environment)
                    code = execute(command)
                    if code != 0:
                        preflight_ok = False
                        if template.phase == "environment":
                            verdict = "INCONCLUSIVE_ENVIRONMENT_UNRESOLVED"
                        else:
                            verdict = "INCONCLUSIVE_AUTHORITY_PREFLIGHT_FAILED"
                        break
                    if template.name == "python_dependencies":
                        tool_versions["python"] = strict_json(command.stdout_path)
                        expected_versions = {
                            "python": EXPECTED_PYTHON_VERSION,
                            "numpy": EXPECTED_NUMPY_VERSION,
                            "mpmath": EXPECTED_MPMATH_VERSION,
                        }
                        if tool_versions["python"] != expected_versions:
                            preflight_ok = False
                            verdict = "INCONCLUSIVE_ENVIRONMENT_UNRESOLVED"
                            break
                    elif template.name in {"rustc_version", "cargo_version"}:
                        text = command.stdout_path.read_text(errors="replace").strip()
                        tool_versions[template.name] = text
                        if not text.startswith(("rustc " if template.name == "rustc_version" else "cargo ") + EXPECTED_RUST_VERSION + " "):
                            preflight_ok = False
                            verdict = "INCONCLUSIVE_ENVIRONMENT_UNRESOLVED"
                            break
                    elif template.name == "authority_verification":
                        authority_preflight = strict_json(command.stdout_path)
                        if authority_preflight != EXPECTED_AUTHORITY_VERIFICATION:
                            preflight_ok = False
                            verdict = "INCONCLUSIVE_AUTHORITY_PREFLIGHT_FAILED"
                            break
                if preflight_ok:
                    try:
                        executable_mode = executable.lstat().st_mode
                        executable_resolved = executable.resolve(strict=True)
                    except (FileNotFoundError, OSError):
                        executable_mode = None
                        executable_resolved = None
                    if not (
                        executable_mode is not None
                        and stat.S_ISREG(executable_mode)
                        and not executable.is_symlink()
                        and executable_resolved == executable.absolute()
                    ):
                        verdict = "INCONCLUSIVE_CANDIDATE_BINARY_UNRESOLVED"
                        preflight_ok = False
                    else:
                        tool_versions["candidate_binary"] = {
                            "path": str(executable),
                            "sha256_prelaunch": sha256_file(executable),
                            "sha256_post": None,
                            "size_bytes": executable.stat().st_size,
                        }
                if preflight_ok:
                    source_prelaunch = run_source("source_prelaunch")
                    prelaunch_hashes_match = all(
                        (source / relative).is_file()
                        and sha256_file(source / relative) == expected
                        for relative, expected in EXPECTED_FILE_HASHES.items()
                    )
                    if not source_prelaunch["valid"] or not prelaunch_hashes_match:
                        verdict = "INCONCLUSIVE_SOURCE_CHANGED_BEFORE_CANDIDATE"
                        preflight_ok = False
                if preflight_ok:
                    one_shot = state / "one-shot"
                    ensure_real_directory(one_shot)
                    key_material = {
                        "implementation_head": IMPLEMENTATION_HEAD,
                        "authority_manifest_sha256": MANIFEST_SHA256,
                        "example_target": "audit2_bateman_local_six_case",
                        "protocol_version": PROTOCOL_VERSION,
                    }
                    key_sha = sha256_bytes(json.dumps(key_material, sort_keys=True, separators=(",", ":")).encode())
                    marker = one_shot / f"BATEMAN_CANDIDATE_ATTEMPT.{key_sha}.json"
                    marker_payload = {
                        "schema": "vigilode-audit2-bateman-one-shot-lock/v1",
                        "protocol_version": PROTOCOL_VERSION,
                        "key_sha256": key_sha,
                        "key_material": key_material,
                        "created_at_utc": utc_now(),
                        "run_id": run_id,
                    }
                    marker_bytes = (json.dumps(marker_payload, indent=2, sort_keys=True) + "\n").encode()
                    try:
                        write_new(marker, marker_bytes)
                        directory_fd = os.open(one_shot, os.O_RDONLY)
                        try:
                            os.fsync(directory_fd)
                        finally:
                            os.close(directory_fd)
                    except FileExistsError:
                        write_new(package / "attempt_lock.json", marker.read_bytes())
                        verdict = "INCONCLUSIVE_ONE_SHOT_ALREADY_CONSUMED"
                        preflight_ok = False
                    if preflight_ok:
                        write_new(package / "attempt_lock.json", marker_bytes)
                        write_json_new(package / "candidate_launch.json", {**marker_payload, "state": "CANDIDATE_LAUNCH_COMMITTED"})
                        candidate_command = Command(
                            "candidate", "candidate", [resolved.get(part, part) if part == "cargo" else part for part in CANDIDATE_ARGV], source, package,
                            package / "result_summary.json", package / "logs/candidate.stderr.log", dict(environment),
                        )
                        candidate_exit = execute(candidate_command)
                        binary_identity = tool_versions.get("candidate_binary")
                        if isinstance(binary_identity, dict) and executable.is_file():
                            binary_identity["sha256_post"] = sha256_file(executable)
                        binary_unchanged = bool(
                            isinstance(binary_identity, dict)
                            and binary_identity.get("sha256_prelaunch")
                            == binary_identity.get("sha256_post")
                        )
                        candidate = {
                            "invocation_count": 1,
                            "launch_status": records[-1]["launch_status"],
                            "exit_code": candidate_exit,
                            "argv": candidate_command.argv,
                            "result_summary_path": "result_summary.json",
                            "result_summary_sha256": sha256_file(candidate_command.stdout_path),
                            "stderr_path": "logs/candidate.stderr.log",
                            "guard_key_sha256": key_sha,
                            "binary_sha256_prelaunch": (
                                binary_identity.get("sha256_prelaunch")
                                if isinstance(binary_identity, dict)
                                else None
                            ),
                            "binary_sha256_post": (
                                binary_identity.get("sha256_post")
                                if isinstance(binary_identity, dict)
                                else None
                            ),
                        }
                        validator_command = Command(
                            "validator", "validator",
                            [resolved["python"], str(source / VALIDATOR_PATH), str(candidate_command.stdout_path)],
                            source, package, package / "local_receipt_validation.json", package / "logs/validator.stderr.log", dict(environment),
                        )
                        validator_exit = execute(validator_command)
                        validator = {
                            "attempt_count": 1,
                            "launch_status": records[-1]["launch_status"],
                            "exit_code": validator_exit,
                            "argv": validator_command.argv,
                            "stdout_path": "local_receipt_validation.json",
                            "stdout_sha256": sha256_file(validator_command.stdout_path),
                            "stderr_path": "logs/validator.stderr.log",
                            "stderr_sha256": sha256_file(validator_command.stderr_path),
                            "parsed_status": None,
                        }
                        try:
                            parsed_validator = strict_json(validator_command.stdout_path)
                            validator["parsed_status"] = parsed_validator.get("status") if isinstance(parsed_validator, dict) else None
                        except Exception:
                            parsed_validator = None
                        try:
                            parsed_report = strict_json(candidate_command.stdout_path)
                        except Exception:
                            parsed_report = None
                        report_ok = validate_report_shape(parsed_report)
                        validator_stderr_empty = (
                            validator_command.stderr_path.read_bytes() == b""
                        )
                        validator_infrastructure_ok = bool(
                            validator["launch_status"] == "LAUNCHED"
                            and validator_exit in {0, 1}
                            and (
                                validator_exit == 1
                                or (
                                    validator_stderr_empty
                                    and isinstance(parsed_validator, dict)
                                )
                            )
                        )
                        receipt_ok = bool(
                            validator_exit == 0
                            and validator_stderr_empty
                            and isinstance(parsed_validator, dict)
                            and parsed_validator.get("status") == "LOCAL_SIX_CASE_RECEIPT_VERIFIED"
                            and parsed_validator.get("scenario_count") == 6
                            and parsed_validator.get("report_sha256") == candidate["result_summary_sha256"]
                            and parsed_validator.get("claim_scope") == REPORT_CLAIM_SCOPE
                        )
                        structured_candidate_reject = bool(
                            report_ok
                            and parsed_report.get("all_six_executed") is True
                            and (
                                parsed_report.get("all_contracts_satisfied") is False
                                or parsed_report.get("terminal_failure") is not None
                            )
                        )
                        if candidate["launch_status"] != "LAUNCHED":
                            verdict = "INCONCLUSIVE_LAUNCH_OR_REPORT"
                        elif not binary_unchanged and executor is None:
                            verdict = "INCONCLUSIVE_CANDIDATE_BINARY_CHANGED"
                        elif not validator_infrastructure_ok:
                            verdict = "INCONCLUSIVE_VALIDATOR_INFRASTRUCTURE"
                        elif candidate_exit == 0 and report_ok and receipt_ok and parsed_report.get("all_six_executed") is True and parsed_report.get("all_contracts_satisfied") is True and parsed_report.get("terminal_failure") is None:
                            verdict = ACCEPT_VERDICT
                        elif structured_candidate_reject:
                            verdict = REJECT_VERDICT
                        else:
                            verdict = "INCONCLUSIVE_LAUNCH_OR_REPORT"
                        source_post = run_source("source_post")
                        post_hashes_match = all(
                            (source / relative).is_file() and sha256_file(source / relative) == expected
                            for relative, expected in EXPECTED_FILE_HASHES.items()
                        )
                        if not source_post["valid"] or not post_hashes_match:
                            verdict = "INCONCLUSIVE_SOURCE_CHANGED_DURING_EXECUTION"
    except Exception as error:
        verdict = "INCONCLUSIVE_RUNNER_INTERNAL_FAILURE"
        write_json_new(package / "runner_internal_failure.json", {"type": type(error).__name__, "message": str(error)})

    if not (package / "authority_bundle_sha256.txt").exists():
        write_new(package / "authority_bundle_sha256.txt", b"")
    if not (package / "validator_attempt.json").exists():
        write_json_new(package / "validator_attempt.json", {
            "schema": "vigilode-audit2-bateman-validator-attempt/v1",
            "attempted": validator["attempt_count"] == 1,
            "attempt_count": validator["attempt_count"],
            "launch_status": validator.get("launch_status"),
            "exit_code": validator.get("exit_code"),
            "argv": validator.get("argv"),
            "stdout_path": validator.get("stdout_path"),
            "stderr_path": validator.get("stderr_path"),
            "stdout_sha256": validator.get("stdout_sha256"),
            "stderr_sha256": validator.get("stderr_sha256"),
        })
    controller_sha256_post = sha256_file(controller_path)
    tool_versions["controller"]["sha256_post"] = controller_sha256_post
    if controller_sha256_post != controller_sha256_pre:
        verdict = "INCONCLUSIVE_CONTROLLER_CHANGED_DURING_EXECUTION"
    pre_candidate_stop: dict[str, Any] | None = None
    if candidate["invocation_count"] == 0:
        failed_commands = [
            {
                "ordinal": row["ordinal"],
                "name": row["name"],
                "launch_status": row["launch_status"],
                "exit_code": row["exit_code"],
            }
            for row in records
            if row["launch_status"] != "LAUNCHED" or row["exit_code"] != 0
        ]
        terminal = records[-1] if records else None
        pre_candidate_stop = {
            "runner_verdict": verdict,
            "reached_ordinal": len(records),
            "terminal_command": terminal["name"] if terminal else None,
            "terminal_phase": terminal["phase"] if terminal else None,
            "terminal_launch_status": (
                terminal["launch_status"] if terminal else None
            ),
            "terminal_exit_code": terminal["exit_code"] if terminal else None,
            "failed_commands": failed_commands,
            "kind": (
                "COMMAND_FAILURE"
                if failed_commands
                else "SEMANTIC_OR_STAGE_FAILURE"
            ),
        }
    finished = utc_now()
    event_ordinal += 1
    append_event(events_path, {"schema": "vigilode-audit2-bateman-execution-event/v1", "ordinal": event_ordinal, "event": "sealing_started", "utc": finished, "verdict": verdict})
    eligible_receipt = verdict in {ACCEPT_VERDICT, REJECT_VERDICT}
    manifest = {
        "schema": "vigilode-audit2-bateman-local-execution-manifest/v1",
        "verdict": verdict,
        "protocol_version": PROTOCOL_VERSION,
        "run_id": run_id,
        "utc_start": started,
        "utc_end": finished,
        "source_pre": source_pre,
        "source_prelaunch": source_prelaunch,
        "source_post": source_post,
        "source_hashes": source_hashes,
        "scientific_implementation_provenance": {
            "execution_source_head": EXECUTION_SOURCE_HEAD,
            "execution_source_tree": EXECUTION_SOURCE_TREE,
            "implementation_head": IMPLEMENTATION_HEAD,
            "implementation_tree": IMPLEMENTATION_TREE,
            "implementation_parent": IMPLEMENTATION_PARENT,
            "implementation_parent_tree": IMPLEMENTATION_PARENT_TREE,
        },
        "tool_versions": tool_versions,
        "authority_preflight": authority_preflight,
        "candidate": candidate,
        "validator": validator,
        "pre_candidate_stop": pre_candidate_stop,
        "commands": records,
        "declarations": {
            "execution_policy": "HOST_CODEX_ONLY",
            "local_llm_used": False,
            "local_llm_used_attestation": "SELF_DECLARED_NOT_HOST_ATTESTED",
            "holdout_access": "NOT_OPENED_OR_EXECUTED",
            "holdout_access_attestation": "SELF_DECLARED_AND_ABSENT_FROM_FIXED_COMMAND_PLAN",
            "remote_write": "NOT_PERFORMED_BY_RUNNER_COMMAND_PLAN",
            "remote_write_attestation": "FIXED_COMMAND_PLAN_ONLY_NOT_HOST_WIDE_ATTESTATION",
            "claim_ceiling": GLOBAL_CLAIM_CEILING,
        },
        "math_blocker_coverage": {
            "X01": "EVALUATED_BY_ELIGIBLE_SEALED_RUN" if eligible_receipt else "NOT_ESTABLISHED",
            "M01": "PREEXISTING_MATHEMATICAL_AUTHORITY",
            "M02": "PARTIALLY_EVALUABLE_FROM_COMPACT_RECEIPT" if eligible_receipt else "NOT_EVALUATED",
            "M03": "PARTIALLY_EVALUABLE_IF_PER_SOLVE_TELEMETRY_PRESENT" if eligible_receipt else "NOT_EVALUATED",
            **{identifier: "NOT_EVALUATED" for identifier in ("M04", "M05", "M06", "M07", "M08", "M09", "M10", "M11", "M12", "X02")},
        },
        "package": {"sha256sums_path": "SHA256SUMS"},
    }
    write_json_new(package / "execution_manifest.json", manifest)
    seal_package(package)
    exit_code = 0 if verdict == ACCEPT_VERDICT else (1 if verdict == REJECT_VERDICT else 2)
    return RunOutcome(verdict, package, exit_code)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-worktree", required=True, type=pathlib.Path)
    args = parser.parse_args()
    outcome = run_protocol(args.source_worktree)
    print(json.dumps({"verdict": outcome.verdict, "package_dir": str(outcome.package_dir)}, sort_keys=True))
    return outcome.exit_code


if __name__ == "__main__":
    raise SystemExit(main())
