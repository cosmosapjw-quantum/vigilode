#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
from typing import Any


class MetadataMismatch(RuntimeError):
    pass


def load(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise MetadataMismatch(f"cannot read Cargo metadata {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise MetadataMismatch(f"Cargo metadata must be a JSON object: {path}")
    return value


def stable_sort_key(value: Any) -> str:
    """Return a deterministic total-order key without changing metadata values."""
    return json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
    )


def canonical_path_package_id(
    package: dict[str, Any], workspace_root: Any
) -> Any:
    """Normalize checkout-local package identity while preserving layout semantics."""
    raw_id = package.get("id")
    if package.get("source") is not None:
        return raw_id

    manifest_path = package.get("manifest_path")
    if not isinstance(manifest_path, str) or not isinstance(workspace_root, str):
        return raw_id

    manifest_dir = Path(manifest_path).parent
    relative_dir = Path(os.path.relpath(manifest_dir, Path(workspace_root))).as_posix()
    identity = {
        "kind": "workspace-path",
        "path": relative_dir,
        "name": package.get("name"),
        "version": package.get("version"),
    }
    return "workspace-path:" + stable_sort_key(identity)


def canonicalize(value: dict[str, Any]) -> dict[str, Any]:
    raw_packages = value.get("packages", [])
    workspace_root = value.get("workspace_root")
    id_map: dict[str, Any] = {}
    for package in raw_packages:
        raw_id = package.get("id")
        if not isinstance(raw_id, str):
            continue
        canonical_id = canonical_path_package_id(package, workspace_root)
        previous = id_map.get(raw_id)
        if previous is not None and previous != canonical_id:
            raise MetadataMismatch(f"ambiguous Cargo package identity: {raw_id}")
        id_map[raw_id] = canonical_id

    def normalize_id(raw_id: Any) -> Any:
        if isinstance(raw_id, str):
            return id_map.get(raw_id, raw_id)
        return raw_id

    packages: list[dict[str, Any]] = []
    for package in raw_packages:
        dependency_records = [
            (
                dependency.get("name"),
                dependency.get("source"),
                dependency.get("req"),
                dependency.get("kind"),
                dependency.get("optional"),
                dependency.get("uses_default_features"),
                tuple(sorted(dependency.get("features", []))),
                dependency.get("target"),
                dependency.get("rename"),
            )
            for dependency in package.get("dependencies", [])
        ]
        packages.append(
            {
                "id": normalize_id(package.get("id")),
                "name": package.get("name"),
                "version": package.get("version"),
                "source": package.get("source"),
                "dependencies": sorted(
                    dependency_records,
                    key=stable_sort_key,
                ),
                "features": {
                    key: tuple(sorted(items))
                    for key, items in sorted(package.get("features", {}).items())
                },
            }
        )
    packages.sort(key=stable_sort_key)

    resolve = value.get("resolve") or {}
    nodes: list[dict[str, Any]] = []
    for node in resolve.get("nodes", []):
        dependency_edges = []
        for dependency in node.get("deps", []):
            dep_kinds = [
                (
                    kind.get("kind"),
                    kind.get("target"),
                )
                for kind in dependency.get("dep_kinds", [])
            ]
            dependency_edges.append(
                (
                    dependency.get("name"),
                    normalize_id(dependency.get("pkg")),
                    tuple(sorted(dep_kinds, key=stable_sort_key)),
                )
            )
        nodes.append(
            {
                "id": normalize_id(node.get("id")),
                "dependencies": tuple(
                    sorted(
                        (normalize_id(item) for item in node.get("dependencies", [])),
                        key=stable_sort_key,
                    )
                ),
                "features": tuple(sorted(node.get("features", []))),
                "deps": tuple(sorted(dependency_edges, key=stable_sort_key)),
            }
        )
    nodes.sort(key=stable_sort_key)

    return {
        "packages": packages,
        "workspace_members": tuple(
            sorted(
                (normalize_id(item) for item in value.get("workspace_members", [])),
                key=stable_sort_key,
            )
        ),
        "workspace_default_members": tuple(
            sorted(
                (
                    normalize_id(item)
                    for item in value.get("workspace_default_members", [])
                ),
                key=stable_sort_key,
            )
        ),
        "resolve_root": normalize_id(resolve.get("root")),
        "resolve_nodes": nodes,
        "version": value.get("version"),
    }


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Compare Cargo dependency graphs while ignoring checkout paths."
    )
    parser.add_argument("default_metadata", type=Path)
    parser.add_argument("offline_metadata", type=Path)
    args = parser.parse_args()

    default = canonicalize(load(args.default_metadata))
    offline = canonicalize(load(args.offline_metadata))
    if default != offline:
        raise SystemExit("ERROR: default and offline Cargo dependency graphs differ")
    print("PASS: default and offline Cargo dependency graphs match")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
