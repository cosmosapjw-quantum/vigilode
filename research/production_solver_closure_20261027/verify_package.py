#!/usr/bin/env python3
"""Verify publication/source/archive identity. This packaging check is not proof."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import tarfile


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def confined(root, name):
    """Accept canonical relative paths without following local symlink parents."""
    if not isinstance(name, str) or not name or "\\" in name:
        raise SystemExit(f"unsafe package path {name!r}")
    relative = PurePosixPath(name)
    if relative.is_absolute() or ".." in relative.parts or relative.as_posix() != name:
        raise SystemExit(f"unsafe package path {name!r}")
    destination = root
    for part in relative.parts:
        destination = destination / part
        if destination.is_symlink():
            raise SystemExit(f"symlink traversal refused: {name}")
    if not destination.resolve().is_relative_to(root.resolve()):
        raise SystemExit(f"package path escapes root: {name}")
    return destination


def read_json(path):
    if not path.is_file():
        raise SystemExit(f"missing package file: {path.name}")
    return json.loads(path.read_text())


def check_digest(path, expected, label):
    if not path.is_file() or digest(path) != expected:
        raise SystemExit(f"{label} identity mismatch")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--extract-evidence", action="store_true")
    parser.add_argument("--source-only", action="store_true", help="omit raw archive contents, retaining manifest/source/report checks")
    args = parser.parse_args()
    if args.source_only and args.extract_evidence:
        parser.error("--source-only and --extract-evidence are mutually exclusive")
    node = Path(__file__).resolve().parent
    root = node.parents[1]
    node_relative = node.relative_to(root).as_posix() + "/"
    manifest = read_json(node / "MANIFEST.json")
    source_path = confined(root, node_relative + "SOURCE_MANIFEST.json")
    raw_path = confined(root, node_relative + "evidence/RAW_MANIFEST.json")
    check_digest(source_path, manifest["source_manifest_sha256"], "source manifest")
    check_digest(raw_path, manifest["raw_manifest_sha256"], "raw manifest")
    report = confined(node, manifest["report_pdf"])
    check_digest(report, manifest["report_pdf_sha256"], "report PDF")
    inventory_path = confined(root, node_relative + "evidence/GIT_FILE_INVENTORY.json")
    check_digest(inventory_path, manifest["git_inventory_sha256"], "Git inventory")
    inventory = read_json(inventory_path)
    if len(inventory["files"]) != len(set(inventory["files"])):
        raise SystemExit("duplicate Git inventory path")
    if node_relative + "evidence/GIT_FILE_INVENTORY.json" not in inventory["files"]:
        raise SystemExit("Git inventory omits its own entry")
    for name in inventory["files"]:
        confined(root, name)

    plan = read_json(node / "DEVELOPMENT_PLAN.json")
    nodes = {x["id"]: x for x in plan["nodes"]}
    if len(nodes) != len(plan["nodes"]):
        raise SystemExit("duplicate DAG id")
    active, done = set(), set()

    def visit(key):
        if key not in nodes:
            raise SystemExit(f"missing DAG dependency {key}")
        if key in active:
            raise SystemExit(f"DAG cycle through {key}")
        if key in done:
            return
        active.add(key)
        for predecessor in nodes[key]["dependencies"]:
            visit(predecessor)
        active.remove(key)
        done.add(key)

    for key, value in nodes.items():
        visit(key)
        if "correction_loops_used" in value and not 0 <= value["correction_loops_used"] <= value["correction_loops_max"] <= 3:
            raise SystemExit(f"correction budget violated: {key}")
    source = read_json(source_path)
    for name, expected in source["sha256"].items():
        check_digest(confined(root, name), expected, f"source {name}")
    raw = read_json(raw_path)
    for name, expected in raw["members"].items():
        confined(root, name)
        if not name.startswith(node_relative):
            raise SystemExit(f"archive member outside research node: {name}")
        if name in source["sha256"] and expected["sha256"] != source["sha256"][name]:
            raise SystemExit(f"archive/source overlap mismatch: {name}")

    archive_count = 0
    if not args.source_only:
        archive = confined(root, raw["archive"])
        check_digest(archive, raw["archive_sha256"], "archive")
        if archive.stat().st_size != raw["archive_bytes"]:
            raise SystemExit("archive byte count mismatch")
        with tarfile.open(archive, "r:xz") as tar:
            entries = tar.getmembers()
            members = {m.name: m for m in entries}
            if len(members) != len(entries):
                raise SystemExit("duplicate archive member")
            if set(members) != set(raw["members"]):
                raise SystemExit("archive membership mismatch")
            # Validate every member/destination before creating any file.
            for name, expected in raw["members"].items():
                member = members[name]
                destination = confined(root, name)
                if not member.isfile():
                    raise SystemExit(f"non-regular archive member: {name}")
                data = tar.extractfile(member).read()
                if member.size != expected["bytes"] or len(data) != expected["bytes"]:
                    raise SystemExit(f"raw byte count mismatch: {name}")
                if hashlib.sha256(data).hexdigest() != expected["sha256"]:
                    raise SystemExit(f"raw hash mismatch: {name}")
                if args.extract_evidence and destination.exists() and (not destination.is_file() or destination.read_bytes() != data):
                    raise SystemExit(f"refuse to overwrite changed local evidence: {name}")
                archive_count += 1
            if args.extract_evidence:
                for name, member in members.items():
                    destination = confined(root, name)
                    if not destination.exists():
                        destination.parent.mkdir(parents=True, exist_ok=True)
                        destination = confined(root, name)
                        with destination.open("xb") as stream:
                            stream.write(tar.extractfile(member).read())
    print(json.dumps({"status": "pass_scoped", "scope": "manifest/DAG/source/report identity" + ("; raw archive omitted" if args.source_only else "/archive identity"),
                      "dag_nodes": len(nodes), "source_files": len(source["sha256"]), "raw_archive_members": archive_count}))


if __name__ == "__main__":
    main()
