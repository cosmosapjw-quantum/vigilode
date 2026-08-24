#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path
import tomllib
from typing import Any

SCHEMA = "vigilode-cargo-directory-source-validation-v1"
REQUIRED_NAME = "faer"
REQUIRED_VERSION = "0.24.4"


class VendorValidationError(RuntimeError):
    pass


def _load_toml(path: Path) -> dict[str, Any]:
    try:
        with path.open("rb") as handle:
            value = tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError) as exc:
        raise VendorValidationError(f"invalid Cargo manifest: {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise VendorValidationError(f"Cargo manifest must be a table: {path}")
    return value


def _load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise VendorValidationError(f"invalid Cargo checksum metadata: {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise VendorValidationError(f"checksum metadata must be an object: {path}")
    if "files" not in value or "package" not in value:
        raise VendorValidationError(
            f"checksum metadata must contain 'files' and 'package': {path}"
        )
    return value


def validate_vendor_source(vendor_dir: Path) -> dict[str, Any]:
    vendor = vendor_dir.expanduser().resolve()
    if not vendor.is_dir():
        raise VendorValidationError(f"vendor directory does not exist: {vendor}")

    immediate_directories = sorted(path for path in vendor.iterdir() if path.is_dir())
    hidden = [path for path in immediate_directories if path.name.startswith(".")]
    visible = [path for path in immediate_directories if not path.name.startswith(".")]
    package_candidates = [path for path in visible if (path / "Cargo.toml").is_file()]
    manifestless = [path for path in visible if not (path / "Cargo.toml").is_file()]

    packages: list[dict[str, str]] = []
    missing_checksum_packages: list[str] = []
    required_present = False

    for package_dir in package_candidates:
        manifest = _load_toml(package_dir / "Cargo.toml")
        package_table = manifest.get("package")
        if not isinstance(package_table, dict):
            raise VendorValidationError(
                f"Cargo manifest has no [package] table: {package_dir / 'Cargo.toml'}"
            )
        name = package_table.get("name")
        version = package_table.get("version")
        if not isinstance(name, str) or not isinstance(version, str):
            raise VendorValidationError(
                f"Cargo package name/version must be strings: {package_dir / 'Cargo.toml'}"
            )

        checksum_path = package_dir / ".cargo-checksum.json"
        if not checksum_path.is_file():
            missing_checksum_packages.append(package_dir.name)
            continue
        _load_json(checksum_path)

        packages.append(
            {
                "directory": package_dir.name,
                "name": name,
                "version": version,
            }
        )
        if name == REQUIRED_NAME and version == REQUIRED_VERSION:
            required_present = True

    if missing_checksum_packages:
        raise VendorValidationError(
            "package directories missing .cargo-checksum.json: "
            + ", ".join(sorted(missing_checksum_packages))
        )
    if not required_present:
        raise VendorValidationError(
            f"required package missing: {REQUIRED_NAME} {REQUIRED_VERSION}"
        )

    packages.sort(key=lambda item: (item["name"], item["version"], item["directory"]))
    return {
        "schema": SCHEMA,
        "vendor_dir": str(vendor),
        "immediate_directory_count": len(immediate_directories),
        "package_directory_count": len(package_candidates),
        "checksum_record_count": len(packages),
        "ignored_hidden_directory_count": len(hidden),
        "ignored_manifestless_directory_count": len(manifestless),
        "required_packages_present": [f"{REQUIRED_NAME}-{REQUIRED_VERSION}"],
        "missing_checksum_packages": [],
        "exact_package_count_enforced": False,
        "packages": packages,
    }


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate a Cargo directory source without enforcing an inventory count."
    )
    parser.add_argument("vendor_dir", type=Path)
    parser.add_argument("--json-out", type=Path)
    args = parser.parse_args()

    try:
        result = validate_vendor_source(args.vendor_dir)
    except VendorValidationError as exc:
        parser.exit(1, f"ERROR: {exc}\n")

    encoded = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.json_out:
        args.json_out.parent.mkdir(parents=True, exist_ok=True)
        args.json_out.write_text(encoded, encoding="utf-8")
    else:
        print(encoded, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
