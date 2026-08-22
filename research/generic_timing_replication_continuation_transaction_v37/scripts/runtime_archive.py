#!/usr/bin/env python3
"""Deterministically pack and fail-closed verify VigilODE v3.7 runtime shards."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
import shutil
import sys
import tarfile
import tempfile
from pathlib import Path
from typing import BinaryIO

ARCHIVE_PREFIX = "runtime/"
COPY_BLOCK = 1024 * 1024


class ArchiveError(RuntimeError):
    """Runtime evidence does not match the committed verification authority."""


def _sha256_bytes(handle: BinaryIO) -> str:
    digest = hashlib.sha256()
    for block in iter(lambda: handle.read(COPY_BLOCK), b""):
        digest.update(block)
    return digest.hexdigest()


def sha256(path: Path) -> str:
    try:
        with path.open("rb") as handle:
            return _sha256_bytes(handle)
    except OSError as error:
        raise ArchiveError(f"cannot hash {path}: {error}") from error


def load_expected_hashes(verification: Path) -> dict[str, str]:
    try:
        document = json.loads(verification.read_text(encoding="utf-8"))
        expected = document["input_sha256"]["runtime"]
    except (OSError, json.JSONDecodeError, KeyError, TypeError) as error:
        raise ArchiveError(f"invalid verification JSON {verification}: {error}") from error
    if not isinstance(expected, dict) or not expected:
        raise ArchiveError("verification JSON must contain nonempty input_sha256.runtime")
    normalized: dict[str, str] = {}
    for raw_path, raw_hash in expected.items():
        if not isinstance(raw_path, str) or not isinstance(raw_hash, str):
            raise ArchiveError("runtime hash map must contain string paths and hashes")
        path = Path(raw_path)
        if path.is_absolute() or ".." in path.parts or raw_path.startswith("./"):
            raise ArchiveError(f"unsafe runtime path in verification JSON: {raw_path}")
        if len(raw_hash) != 64 or any(ch not in "0123456789abcdef" for ch in raw_hash):
            raise ArchiveError(f"invalid SHA-256 for {raw_path}: {raw_hash}")
        normalized[path.as_posix()] = raw_hash
    if len(normalized) != len(expected):
        raise ArchiveError("duplicate normalized runtime paths in verification JSON")
    return normalized


def validate_runtime_tree(runtime_root: Path, expected: dict[str, str]) -> None:
    if runtime_root.name != "runtime" or not runtime_root.is_dir():
        raise ArchiveError(f"runtime root must be an existing directory named runtime: {runtime_root}")
    actual = {
        path.relative_to(runtime_root).as_posix()
        for path in runtime_root.rglob("*")
        if path.is_file()
    }
    if actual != set(expected):
        missing = sorted(set(expected) - actual)
        unexpected = sorted(actual - set(expected))
        raise ArchiveError(
            f"runtime file set mismatch: missing={missing}, unexpected={unexpected}"
        )
    for relative, expected_hash in sorted(expected.items()):
        actual_hash = sha256(runtime_root / relative)
        if actual_hash != expected_hash:
            raise ArchiveError(
                f"runtime hash mismatch for {relative}: {actual_hash} != {expected_hash}"
            )


def _canonical_tar_info(relative: str, size: int) -> tarfile.TarInfo:
    info = tarfile.TarInfo(f"{ARCHIVE_PREFIX}{relative}")
    info.size = size
    info.mode = 0o644
    info.uid = 0
    info.gid = 0
    info.uname = ""
    info.gname = ""
    info.mtime = 0
    info.type = tarfile.REGTYPE
    return info


def pack_runtime(
    runtime_root: Path,
    verification: Path,
    archive: Path,
    sha_output: Path,
) -> str:
    expected = load_expected_hashes(verification)
    validate_runtime_tree(runtime_root, expected)
    if archive.exists() or sha_output.exists():
        raise ArchiveError("refusing to overwrite runtime archive output")
    archive.parent.mkdir(parents=True, exist_ok=True)
    sha_output.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary_name = tempfile.mkstemp(
        prefix=f".{archive.name}.", suffix=".tmp", dir=archive.parent
    )
    os.close(fd)
    temporary = Path(temporary_name)
    try:
        with temporary.open("wb") as raw:
            with gzip.GzipFile(
                filename="", mode="wb", compresslevel=9, fileobj=raw, mtime=0
            ) as zipped:
                with tarfile.open(
                    fileobj=zipped, mode="w", format=tarfile.USTAR_FORMAT
                ) as bundle:
                    for relative in sorted(expected):
                        source = runtime_root / relative
                        info = _canonical_tar_info(relative, source.stat().st_size)
                        with source.open("rb") as handle:
                            bundle.addfile(info, handle)
        digest = sha256(temporary)
        os.replace(temporary, archive)
        temporary_sha = sha_output.with_name(f".{sha_output.name}.tmp")
        temporary_sha.write_text(f"{digest}\n", encoding="ascii")
        os.replace(temporary_sha, sha_output)
        return digest
    except Exception:
        temporary.unlink(missing_ok=True)
        raise


def _validated_members(
    bundle: tarfile.TarFile, expected: dict[str, str]
) -> dict[str, tarfile.TarInfo]:
    members = bundle.getmembers()
    actual_names = {member.name for member in members}
    expected_names = {f"{ARCHIVE_PREFIX}{relative}" for relative in expected}
    if actual_names != expected_names:
        missing = sorted(expected_names - actual_names)
        unexpected = sorted(actual_names - expected_names)
        raise ArchiveError(
            f"archive member set mismatch: missing={missing}, unexpected={unexpected}"
        )
    validated: dict[str, tarfile.TarInfo] = {}
    for member in members:
        if not member.isfile() or member.issym() or member.islnk():
            raise ArchiveError(f"non-regular archive member: {member.name}")
        relative = member.name.removeprefix(ARCHIVE_PREFIX)
        if (
            not relative
            or Path(relative).is_absolute()
            or ".." in Path(relative).parts
            or member.name != f"{ARCHIVE_PREFIX}{relative}"
        ):
            raise ArchiveError(f"unsafe archive member: {member.name}")
        validated[relative] = member
    return validated


def verify_archive(archive: Path, verification: Path) -> str:
    expected = load_expected_hashes(verification)
    try:
        with tarfile.open(archive, mode="r:gz") as bundle:
            members = _validated_members(bundle, expected)
            for relative, expected_hash in sorted(expected.items()):
                extracted = bundle.extractfile(members[relative])
                if extracted is None:
                    raise ArchiveError(f"cannot read archive member: {relative}")
                with extracted:
                    actual_hash = _sha256_bytes(extracted)
                if actual_hash != expected_hash:
                    raise ArchiveError(
                        f"archive hash mismatch for {relative}: "
                        f"{actual_hash} != {expected_hash}"
                    )
    except (OSError, tarfile.TarError) as error:
        raise ArchiveError(f"cannot read runtime archive {archive}: {error}") from error
    return sha256(archive)


def unpack_runtime(archive: Path, verification: Path, output_root: Path) -> str:
    expected = load_expected_hashes(verification)
    if output_root.exists():
        raise ArchiveError(f"refusing to overwrite runtime output: {output_root}")
    output_root.parent.mkdir(parents=True, exist_ok=True)
    temporary = Path(
        tempfile.mkdtemp(prefix=f".{output_root.name}.", dir=output_root.parent)
    )
    try:
        with tarfile.open(archive, mode="r:gz") as bundle:
            members = _validated_members(bundle, expected)
            for relative, expected_hash in sorted(expected.items()):
                destination = temporary / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                extracted = bundle.extractfile(members[relative])
                if extracted is None:
                    raise ArchiveError(f"cannot read archive member: {relative}")
                digest = hashlib.sha256()
                with extracted, destination.open("wb") as output:
                    for block in iter(lambda: extracted.read(COPY_BLOCK), b""):
                        digest.update(block)
                        output.write(block)
                actual_hash = digest.hexdigest()
                if actual_hash != expected_hash:
                    raise ArchiveError(
                        f"archive hash mismatch for {relative}: "
                        f"{actual_hash} != {expected_hash}"
                    )
                destination.chmod(0o644)
        os.replace(temporary, output_root)
        return sha256(archive)
    except (OSError, tarfile.TarError) as error:
        shutil.rmtree(temporary, ignore_errors=True)
        if isinstance(error, ArchiveError):
            raise
        raise ArchiveError(f"cannot unpack runtime archive {archive}: {error}") from error
    except Exception:
        shutil.rmtree(temporary, ignore_errors=True)
        raise


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    pack = subparsers.add_parser("pack", help="create a deterministic runtime archive")
    pack.add_argument("--runtime-root", type=Path, required=True)
    pack.add_argument("--verification", type=Path, required=True)
    pack.add_argument("--archive", type=Path, required=True)
    pack.add_argument("--sha-output", type=Path, required=True)

    verify = subparsers.add_parser("verify", help="verify an archive without extraction")
    verify.add_argument("--archive", type=Path, required=True)
    verify.add_argument("--verification", type=Path, required=True)

    unpack = subparsers.add_parser("unpack", help="extract and verify a runtime archive")
    unpack.add_argument("--archive", type=Path, required=True)
    unpack.add_argument("--verification", type=Path, required=True)
    unpack.add_argument("--output-root", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    arguments = parse_args()
    try:
        if arguments.command == "pack":
            digest = pack_runtime(
                arguments.runtime_root,
                arguments.verification,
                arguments.archive,
                arguments.sha_output,
            )
        elif arguments.command == "verify":
            digest = verify_archive(arguments.archive, arguments.verification)
        else:
            digest = unpack_runtime(
                arguments.archive,
                arguments.verification,
                arguments.output_root,
            )
    except ArchiveError as error:
        print(f"RUNTIME_ARCHIVE_FAIL: {error}", file=sys.stderr)
        return 1
    print(f"RUNTIME_ARCHIVE_PASS {digest}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
