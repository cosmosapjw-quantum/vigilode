#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
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


def canonicalize(value: dict[str, Any]) -> dict[str, Any]:
    packages: list[dict[str, Any]] = []
    for package in value.get("packages", []):
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
                "id": package.get("id"),
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
    packages.sort(key=lambda package: str(package["id"]))

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
                    dependency.get("pkg"),
                    tuple(sorted(dep_kinds, key=stable_sort_key)),
                )
            )
        nodes.append(
            {
                "id": node.get("id"),
                "dependencies": tuple(sorted(node.get("dependencies", []))),
                "features": tuple(sorted(node.get("features", []))),
                "deps": tuple(sorted(dependency_edges, key=stable_sort_key)),
            }
        )
    nodes.sort(key=lambda node: str(node["id"]))

    return {
        "packages": packages,
        "workspace_members": tuple(sorted(value.get("workspace_members", []))),
        "workspace_default_members": tuple(
            sorted(value.get("workspace_default_members", []))
        ),
        "resolve_root": resolve.get("root"),
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
