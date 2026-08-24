#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github" / "workflows" / "e4-fresh-clone-build.yml"
README = ROOT / "README.md"
WRAPPER = ROOT / "tools" / "cargo-offline.sh"
COMPARATOR = ROOT / "tools" / "compare-cargo-metadata.py"
PLAN_ROOT = ROOT / "docs" / "superpowers" / "plans"
SPEC_ROOT = ROOT / "docs" / "superpowers" / "specs"
DIRECT_WRAPPER_CALL = re.compile(
    r"(?m)^[ \t]*\./tools/cargo-offline\.sh(?:[ \t\\]|$)"
)
REGISTRY_SERDE_ID = (
    "registry+https://github.com/rust-lang/crates.io-index#serde@1.0.0"
)


def load_comparator():
    spec = importlib.util.spec_from_file_location(
        "e4_final_metadata_comparator", COMPARATOR
    )
    if spec is None or spec.loader is None:
        raise AssertionError(f"cannot load metadata comparator: {COMPARATOR}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def make_metadata(checkout_root: Path, relative_package_path: str) -> dict:
    package_dir = checkout_root / relative_package_path
    package_id = f"path+{package_dir.as_uri()}#0.1.0"
    return {
        "packages": [
            {
                "name": "alpha",
                "version": "0.1.0",
                "id": package_id,
                "source": None,
                "dependencies": [
                    {
                        "name": "serde",
                        "source": "registry+https://github.com/rust-lang/crates.io-index",
                        "req": "^1",
                        "kind": None,
                        "rename": None,
                        "optional": False,
                        "uses_default_features": True,
                        "features": [],
                        "target": None,
                    }
                ],
                "features": {},
                "manifest_path": str(package_dir / "Cargo.toml"),
            },
            {
                "name": "serde",
                "version": "1.0.0",
                "id": REGISTRY_SERDE_ID,
                "source": "registry+https://github.com/rust-lang/crates.io-index",
                "dependencies": [],
                "features": {},
                "manifest_path": str(
                    checkout_root / "registry" / "serde" / "Cargo.toml"
                ),
            },
        ],
        "workspace_members": [package_id],
        "workspace_default_members": [package_id],
        "resolve": {
            "nodes": [
                {
                    "id": package_id,
                    "dependencies": [REGISTRY_SERDE_ID],
                    "deps": [
                        {
                            "name": "serde",
                            "pkg": REGISTRY_SERDE_ID,
                            "dep_kinds": [{"kind": None, "target": None}],
                        }
                    ],
                    "features": [],
                },
                {
                    "id": REGISTRY_SERDE_ID,
                    "dependencies": [],
                    "deps": [],
                    "features": [],
                },
            ],
            "root": package_id,
        },
        "target_directory": str(checkout_root / "target"),
        "version": 1,
        "workspace_root": str(checkout_root),
    }


def make_vendor_package(vendor: Path) -> None:
    package = vendor / "faer-0.24.4"
    package.mkdir(parents=True)
    (package / "Cargo.toml").write_text(
        "[package]\nname = 'faer'\nversion = '0.24.4'\n",
        encoding="utf-8",
    )
    (package / ".cargo-checksum.json").write_text(
        json.dumps({"files": {}, "package": "0"}, sort_keys=True) + "\n",
        encoding="utf-8",
    )


class LoadBearingInvocationTests(unittest.TestCase):
    def test_load_bearing_corpus_never_directly_executes_wrapper(self) -> None:
        files = [README, WORKFLOW, WRAPPER]
        files.extend(sorted(PLAN_ROOT.rglob("*.md")))
        files.extend(sorted(SPEC_ROOT.rglob("*.md")))
        violations: list[str] = []
        for path in files:
            text = path.read_text(encoding="utf-8")
            for match in DIRECT_WRAPPER_CALL.finditer(text):
                line = text.count("\n", 0, match.start()) + 1
                violations.append(f"{path.relative_to(ROOT)}:{line}")
        self.assertEqual(violations, [])


class CrossCheckoutMetadataTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.comparator = load_comparator()

    def test_same_workspace_layout_ignores_checkout_root_in_package_ids(self) -> None:
        left = make_metadata(Path("/tmp/checkout-a"), "crates/alpha")
        right = make_metadata(Path("/opt/checkout-b"), "crates/alpha")
        self.assertEqual(
            self.comparator.canonicalize(left),
            self.comparator.canonicalize(right),
        )

    def test_different_relative_workspace_member_path_remains_distinct(self) -> None:
        left = make_metadata(Path("/tmp/checkout-a"), "crates/alpha")
        right = make_metadata(Path("/opt/checkout-b"), "packages/alpha")
        self.assertNotEqual(
            self.comparator.canonicalize(left),
            self.comparator.canonicalize(right),
        )

    def test_registry_package_identity_remains_fully_qualified(self) -> None:
        value = self.comparator.canonicalize(
            make_metadata(Path("/tmp/checkout-a"), "crates/alpha")
        )
        registry = next(
            package for package in value["packages"] if package["name"] == "serde"
        )
        self.assertEqual(registry["id"], REGISTRY_SERDE_ID)


class WrapperTomlEscapingTests(unittest.TestCase):
    def test_valid_vendor_path_with_quote_and_backslash_round_trips(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            vendor = root / 'vendor "quoted" \\ source'
            vendor.mkdir()
            make_vendor_package(vendor)

            bin_dir = root / "bin"
            bin_dir.mkdir()
            fake_cargo = bin_dir / "cargo"
            capture = root / "capture.json"
            fake_cargo.write_text(
                "#!/usr/bin/env python3\n"
                "import json, os, pathlib, tomllib\n"
                "config = pathlib.Path(os.environ['CARGO_HOME']) / 'config.toml'\n"
                "value = tomllib.loads(config.read_text(encoding='utf-8'))\n"
                "directory = value['source']['vigilode-vendored-sources']['directory']\n"
                "pathlib.Path(os.environ['CAPTURE']).write_text(\n"
                "    json.dumps({'directory': directory}), encoding='utf-8'\n"
                ")\n",
                encoding="utf-8",
            )
            fake_cargo.chmod(0o755)

            env = os.environ.copy()
            env["PATH"] = str(bin_dir) + os.pathsep + env.get("PATH", "")
            env["CAPTURE"] = str(capture)
            result = subprocess.run(
                [
                    "bash",
                    str(WRAPPER),
                    "--vendor-dir",
                    str(vendor),
                    "metadata",
                    "--frozen",
                ],
                cwd=root,
                env=env,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            payload = json.loads(capture.read_text(encoding="utf-8"))
            self.assertEqual(payload["directory"], str(vendor.resolve()))


if __name__ == "__main__":
    unittest.main(verbosity=2)
