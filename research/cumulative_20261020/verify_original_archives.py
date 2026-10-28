#!/usr/bin/env python3
"""Validate all four ZIPs and 72 extracted source objects, no dependencies.

Run at repository root on research/cumulative-evidence-20261020.
All bytes and SHA256 values are frozen in ORIGINALS_MANIFEST.json.
"""
import hashlib
import json
import zipfile
from pathlib import Path

root = Path(__file__).resolve().parents[2]
base = root / "research/cumulative_20261020"
manifest = json.loads((base / "ORIGINALS_MANIFEST.json").read_text())
files_by_archive = {}
for member in manifest["members"]:
    files_by_archive.setdefault(member["archive_name"], []).append(member)

sha256 = lambda b: hashlib.sha256(b).hexdigest()
checked_archives = 0
checked_members = 0
for item in manifest["archives"]:
    archive = base / "original_archives" / item["name"]
    data = archive.read_bytes()
    assert len(data) == item["bytes"], archive
    assert sha256(data) == item["sha256"], archive
    with zipfile.ZipFile(archive) as z:
        assert z.testzip() is None, archive
        actual_files = [n for n in z.namelist() if not n.endswith("/")]
        members = files_by_archive[item["name"]]
        assert len(actual_files) == len(members) == item["member_count"], archive
        assert set(actual_files) == {m["member"] for m in members}, archive
        for member in members:
            original = z.read(member["member"])
            checkout = (base / "original_sources" / member["path"]).read_bytes()
            assert original == checkout, member["path"]
            assert len(original) == member["bytes"], member["path"]
            assert sha256(original) == member["sha256"], member["path"]
            checked_members += 1
    checked_archives += 1
assert checked_archives == 4 and checked_members == 72
print(json.dumps({
    "status": "PASS_EXACT_ARCHIVE_MEMBERS_AND_SHA256",
    "zip_files": checked_archives,
    "source_members": checked_members,
}, sort_keys=True))
