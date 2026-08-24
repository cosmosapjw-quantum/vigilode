#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CONFIG = ROOT / ".cargo" / "config.toml"
OFFLINE_CONFIG = ROOT / ".cargo" / "config.offline.toml"
WRAPPER = ROOT / "tools" / "cargo-offline.sh"
VALIDATOR = ROOT / "tools" / "validate-cargo-vendor.py"
COMPARE = ROOT / "tools" / "compare-cargo-metadata.py"
WORKFLOW = ROOT / ".github" / "workflows" / "e4-fresh-clone-build.yml"
README = ROOT / "README.md"
GITIGNORE = ROOT / ".gitignore"


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise AssertionError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def make_package(root: Path, directory: str, name: str, version: str) -> None:
    package = root / directory
    package.mkdir()
    (package / "Cargo.toml").write_text(
        f"[package]\nname = {name!r}\nversion = {version!r}\n",
        encoding="utf-8",
    )
    (package / ".cargo-checksum.json").write_text(
        json.dumps({"files": {}, "package": "0"}, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def make_metadata(root: Path, suffix: str = "") -> dict:
    package_id = "path+file:///repo/crates/alpha#0.1.0"
    dependency_id = "registry+https://github.com/rust-lang/crates.io-index#serde@1.0.0"
    return {
        "packages": [
            {
                "name": "alpha",
                "version": "0.1.0",
                "id": package_id,
                "license": None,
                "license_file": None,
                "description": None,
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
                        "registry": None,
                    }
                ],
                "targets": [],
                "features": {},
                "manifest_path": str(root / suffix / "Cargo.toml"),
                "metadata": None,
                "publish": None,
                "authors": [],
                "categories": [],
                "keywords": [],
                "readme": None,
                "repository": None,
                "homepage": None,
                "documentation": None,
                "edition": "2024",
                "links": None,
                "default_run": None,
                "rust_version": "1.94.1",
            },
            {
                "name": "serde",
                "version": "1.0.0",
                "id": dependency_id,
                "license": None,
                "license_file": None,
                "description": None,
                "source": "registry+https://github.com/rust-lang/crates.io-index",
                "dependencies": [],
                "targets": [],
                "features": {},
                "manifest_path": str(root / suffix / "serde" / "Cargo.toml"),
                "metadata": None,
                "publish": None,
                "authors": [],
                "categories": [],
                "keywords": [],
                "readme": None,
                "repository": None,
                "homepage": None,
                "documentation": None,
                "edition": "2021",
                "links": None,
                "default_run": None,
                "rust_version": None,
            },
        ],
        "workspace_members": [package_id],
        "workspace_default_members": [package_id],
        "resolve": {
            "nodes": [
                {
                    "id": package_id,
                    "dependencies": [dependency_id],
                    "deps": [
                        {
                            "name": "serde",
                            "pkg": dependency_id,
                            "dep_kinds": [{"kind": None, "target": None}],
                        }
                    ],
                    "features": [],
                },
                {
                    "id": dependency_id,
                    "dependencies": [],
                    "deps": [],
                    "features": [],
                },
            ],
            "root": package_id,
        },
        "target_directory": str(root / suffix / "target"),
        "version": 1,
        "workspace_root": str(root / suffix),
        "metadata": None,
    }


class E4RepositoryContractTests(unittest.TestCase):
    def test_default_cargo_config_does_not_force_source_replacement_or_offline_mode(self) -> None:
        if not DEFAULT_CONFIG.exists():
            return
        text = DEFAULT_CONFIG.read_text(encoding="utf-8")
        for forbidden in (
            "replace-with",
            "vendored-sources",
            "offline = true",
            "rust-offline-rodas5p",
        ):
            self.assertNotIn(forbidden, text)

    def test_offline_config_is_explicit_and_not_auto_discovered(self) -> None:
        self.assertTrue(OFFLINE_CONFIG.is_file())
        text = OFFLINE_CONFIG.read_text(encoding="utf-8")
        self.assertIn('replace-with = "vendored-sources"', text)
        self.assertIn('directory = "vendor"', text)
        self.assertIn("offline = true", text)

    def test_public_offline_wrapper_and_helpers_exist(self) -> None:
        self.assertTrue(WRAPPER.is_file())
        self.assertTrue(VALIDATOR.is_file())
        self.assertTrue(COMPARE.is_file())

    def test_readme_and_gitignore_publish_both_build_modes(self) -> None:
        text = README.read_text(encoding="utf-8")
        self.assertIn("Build reproducibility", text)
        self.assertIn("cargo metadata --locked --format-version 1", text)
        self.assertIn("tools/cargo-offline.sh", text)
        self.assertIn("VIGILODE_CARGO_VENDOR_DIR", text)
        self.assertIn("temporary Cargo home", text)
        self.assertIn("/vendor/", GITIGNORE.read_text(encoding="utf-8"))

    def test_ci_exercises_default_and_explicit_offline_modes(self) -> None:
        self.assertTrue(WORKFLOW.is_file())
        text = WORKFLOW.read_text(encoding="utf-8")
        self.assertIn("cargo metadata --locked --format-version 1", text)
        self.assertIn("cargo test --workspace --all-targets --no-run --locked", text)
        self.assertIn("cargo vendor --locked", text)
        self.assertIn("tools/cargo-offline.sh", text)
        self.assertIn("metadata --frozen --format-version 1", text)
        self.assertIn(
            "cargo clippy -p rodas5p-integrators --all-targets --locked -- -D warnings",
            text,
        )
        self.assertNotIn("cargo clippy --workspace --all-targets", text)
        self.assertIn("v38d_performance_probe_contracts", text)
        self.assertIn("compare-cargo-metadata.py", text)


class VendorValidationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        if not VALIDATOR.is_file():
            raise AssertionError(f"missing vendor validator: {VALIDATOR}")
        cls.module = load_module(VALIDATOR, "e4_vendor_validator")

    def test_valid_sources_with_different_package_counts_pass(self) -> None:
        for count in (3, 5):
            with self.subTest(count=count), tempfile.TemporaryDirectory() as tmp:
                vendor = Path(tmp) / "vendor"
                vendor.mkdir()
                make_package(vendor, "faer-0.24.4", "faer", "0.24.4")
                for index in range(count - 1):
                    make_package(
                        vendor,
                        f"crate-{index}-1.0.0",
                        f"crate-{index}",
                        "1.0.0",
                    )
                result = self.module.validate_vendor_source(vendor)
                self.assertEqual(result["package_directory_count"], count)
                self.assertFalse(result["exact_package_count_enforced"])

    def test_hidden_and_manifestless_directories_are_ignored(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            vendor = Path(tmp) / "vendor"
            vendor.mkdir()
            make_package(vendor, "faer-0.24.4", "faer", "0.24.4")
            (vendor / ".cache").mkdir()
            (vendor / "notes").mkdir()
            result = self.module.validate_vendor_source(vendor)
            self.assertEqual(result["package_directory_count"], 1)
            self.assertEqual(result["ignored_hidden_directory_count"], 1)
            self.assertEqual(result["ignored_manifestless_directory_count"], 1)

    def test_manifest_bearing_directory_without_checksum_fails(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            vendor = Path(tmp) / "vendor"
            vendor.mkdir()
            make_package(vendor, "faer-0.24.4", "faer", "0.24.4")
            package = vendor / "broken-1.0.0"
            package.mkdir()
            (package / "Cargo.toml").write_text(
                "[package]\nname='broken'\nversion='1.0.0'\n",
                encoding="utf-8",
            )
            with self.assertRaises(self.module.VendorValidationError):
                self.module.validate_vendor_source(vendor)

    def test_missing_required_faer_version_fails(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            vendor = Path(tmp) / "vendor"
            vendor.mkdir()
            make_package(vendor, "faer-0.24.3", "faer", "0.24.3")
            with self.assertRaises(self.module.VendorValidationError):
                self.module.validate_vendor_source(vendor)


class WrapperContractTests(unittest.TestCase):
    def test_wrapper_rejects_missing_vendor(self) -> None:
        result = subprocess.run(
            ["bash", str(WRAPPER), "--vendor-dir", "/definitely/missing", "metadata"],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("does not exist", result.stderr)

    def test_wrapper_accepts_environment_vendor_and_uses_absolute_override(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            vendor = root / "vendor"
            vendor.mkdir()
            make_package(vendor, "faer-0.24.4", "faer", "0.24.4")
            bin_dir = root / "bin"
            bin_dir.mkdir()
            fake_cargo = bin_dir / "cargo"
            capture = root / "capture.json"
            fake_cargo.write_text(
                "#!/usr/bin/env python3\n"
                "import json, os, pathlib, sys\n"
                "cargo_home = os.environ.get('CARGO_HOME')\n"
                "if not cargo_home:\n"
                "    raise SystemExit('CARGO_HOME missing')\n"
                "config = pathlib.Path(cargo_home) / 'config.toml'\n"
                "if not config.is_file():\n"
                "    raise SystemExit('temporary Cargo config missing')\n"
                "pathlib.Path(os.environ['CAPTURE']).write_text(json.dumps({\n"
                "  'cwd': os.getcwd(),\n"
                "  'args': sys.argv[1:],\n"
                "  'cargo_home': cargo_home,\n"
                "  'config': config.read_text(),\n"
                "}, sort_keys=True))\n",
                encoding="utf-8",
            )
            fake_cargo.chmod(0o755)
            env = os.environ.copy()
            env["PATH"] = str(bin_dir) + os.pathsep + env.get("PATH", "")
            env["VIGILODE_CARGO_VENDOR_DIR"] = str(vendor)
            env["CAPTURE"] = str(capture)
            result = subprocess.run(
                ["bash", str(WRAPPER), "metadata", "--frozen"],
                cwd=root,
                env=env,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            payload = json.loads(capture.read_text(encoding="utf-8"))
            self.assertEqual(payload["cwd"], str(ROOT))
            self.assertEqual(payload["args"], ["metadata", "--frozen"])
            self.assertIn(str(vendor.resolve()), payload["config"])
            self.assertIn("offline = true", payload["config"])
            self.assertFalse(Path(payload["cargo_home"]).exists())


class MetadataComparisonTests(unittest.TestCase):
    def test_metadata_comparison_ignores_checkout_paths(self) -> None:
        module = load_module(COMPARE, "e4_metadata_compare")
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            left = make_metadata(root, "left")
            right = make_metadata(root, "right")
            self.assertEqual(module.canonicalize(left), module.canonicalize(right))

    def test_metadata_comparison_detects_dependency_graph_drift(self) -> None:
        module = load_module(COMPARE, "e4_metadata_compare_drift")
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            left = make_metadata(root, "left")
            right = make_metadata(root, "right")
            right["packages"][1]["version"] = "1.0.1"
            self.assertNotEqual(module.canonicalize(left), module.canonicalize(right))


if __name__ == "__main__":
    unittest.main(verbosity=2)
