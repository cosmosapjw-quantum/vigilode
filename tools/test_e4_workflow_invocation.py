#!/usr/bin/env python3
from __future__ import annotations

import copy
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github" / "workflows" / "e4-fresh-clone-build.yml"
COMPARATOR = ROOT / "tools" / "compare-cargo-metadata.py"


def load_comparator():
    spec = importlib.util.spec_from_file_location("e4_metadata_comparator", COMPARATOR)
    if spec is None or spec.loader is None:
        raise AssertionError(f"cannot load metadata comparator: {COMPARATOR}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def minimal_metadata() -> dict:
    package_id = "path+file:///repo/crates/alpha#0.1.0"
    return {
        "packages": [
            {
                "id": package_id,
                "name": "alpha",
                "version": "0.1.0",
                "source": None,
                "dependencies": [],
                "features": {},
            }
        ],
        "workspace_members": [package_id],
        "workspace_default_members": [package_id],
        "resolve": {
            "nodes": [
                {
                    "id": package_id,
                    "dependencies": [],
                    "features": [],
                    "deps": [],
                }
            ],
            "root": package_id,
        },
        "version": 1,
    }


def dependency_record(*, source, kind, target, rename) -> dict:
    return {
        "name": "shared",
        "source": source,
        "req": "^1",
        "kind": kind,
        "optional": False,
        "uses_default_features": True,
        "features": [],
        "target": target,
        "rename": rename,
    }


class WorkflowWrapperInvocationTests(unittest.TestCase):
    def test_offline_wrapper_calls_use_explicit_bash_interpreter(self) -> None:
        text = WORKFLOW.read_text(encoding="utf-8")
        calls = [
            line.strip()
            for line in text.splitlines()
            if "tools/cargo-offline.sh" in line
        ]
        self.assertEqual(len(calls), 3)
        self.assertTrue(
            all(call.startswith("bash ./tools/cargo-offline.sh") for call in calls),
            calls,
        )


class NullableMetadataOrderingTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.comparator = load_comparator()

    def test_dependency_records_with_mixed_nullable_fields_have_total_order(self) -> None:
        left = minimal_metadata()
        left["packages"][0]["dependencies"] = [
            dependency_record(source=None, kind=None, target=None, rename=None),
            dependency_record(
                source="registry+https://github.com/rust-lang/crates.io-index",
                kind="dev",
                target="cfg(unix)",
                rename="shared_alias",
            ),
        ]
        right = copy.deepcopy(left)
        right["packages"][0]["dependencies"].reverse()

        self.assertEqual(
            self.comparator.canonicalize(left),
            self.comparator.canonicalize(right),
        )

    def test_dep_kinds_with_mixed_nullable_fields_have_total_order(self) -> None:
        left = minimal_metadata()
        left["resolve"]["nodes"][0]["deps"] = [
            {
                "name": "shared",
                "pkg": "registry+https://github.com/rust-lang/crates.io-index#shared@1.0.0",
                "dep_kinds": [
                    {"kind": None, "target": None},
                    {"kind": "dev", "target": "cfg(unix)"},
                ],
            }
        ]
        right = copy.deepcopy(left)
        right["resolve"]["nodes"][0]["deps"][0]["dep_kinds"].reverse()

        self.assertEqual(
            self.comparator.canonicalize(left),
            self.comparator.canonicalize(right),
        )

    def test_none_and_empty_string_remain_semantically_distinct(self) -> None:
        left = minimal_metadata()
        right = minimal_metadata()
        left["packages"][0]["dependencies"] = [
            dependency_record(source=None, kind=None, target=None, rename=None)
        ]
        right["packages"][0]["dependencies"] = [
            dependency_record(source="", kind=None, target=None, rename=None)
        ]

        self.assertNotEqual(
            self.comparator.canonicalize(left),
            self.comparator.canonicalize(right),
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
