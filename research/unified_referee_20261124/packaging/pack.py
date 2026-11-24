"""Build the immutable source SDK with vendored registry crates from Cargo.lock."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib


def capture(*cmd, cwd=None):
    return subprocess.check_output(cmd, cwd=cwd, text=True).strip()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--revision", default="HEAD")
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    root = Path(capture("git", "rev-parse", "--show-toplevel"))
    commit = capture("git", "rev-parse", "--verify", args.revision + "^{commit}", cwd=root)
    tree = capture("git", "rev-parse", commit + "^{tree}", cwd=root)
    epoch = capture("git", "show", "-s", "--format=%ct", commit, cwd=root)
    manifest = tomllib.loads(capture("git", "show", commit + ":Cargo.toml", cwd=root))
    paths = ["Cargo.toml", "Cargo.lock", "LICENSE", "README.md", "rust-toolchain.toml", ".cargo",
             "fixtures", *manifest["workspace"]["members"]]
    packaging = "research/unified_referee_20261124/packaging"
    paths += [f"{packaging}/{f}" for f in ("consumer.rs", "smoke.py", "README.md")]
    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    # Commit-only sources avoid carrying untracked reports, caches or local credentials.
    with tempfile.TemporaryDirectory() as folder, tempfile.NamedTemporaryFile(dir=output.parent) as archive:
        work = Path(folder)
        sdk = work / "vigilode-sdk"
        sdk.mkdir()
        tarball = work / "first-party.tar"
        subprocess.run(["git", "archive", "-o", str(tarball), commit, "--", *paths], cwd=root, check=True)
        subprocess.run(["tar", "-xf", tarball, "-C", sdk], check=True)
        config = capture("cargo", "vendor", "--locked", "--offline", "--versioned-dirs", "vendor", cwd=sdk)
        (sdk / ".cargo/config.toml").write_text(config + "\n")
        lock = tomllib.loads((sdk / "Cargo.lock").read_text())
        registry = [p for p in lock["package"] if "source" in p]
        for pkg in registry:
            record = sdk / "vendor" / (pkg["name"] + "-" + pkg["version"]) / ".cargo-checksum.json"
            if json.loads(record.read_text())["package"] != pkg["checksum"]:
                raise ValueError("vendored checksum differs for " + pkg["name"])
        (sdk / "SDK_SOURCE.json").write_text(json.dumps({
            "schema": 1, "commit": commit, "tree": tree, "archives": paths,
            "registry_crates": len(registry),
            "cargo_lock_sha256": hashlib.sha256((sdk / "Cargo.lock").read_bytes()).hexdigest(),
            "verified_scope": ["external-public-vigilode-default", "external-public-vigilode-structured"],
        }, indent=2) + "\n")
        tar_cmd = ["tar", "--sort=name", "--format=gnu", "--owner=0", "--group=0",
                   "--numeric-owner", "--mtime=@" + epoch, "-cf", "-", "-C", folder, sdk.name]
        with subprocess.Popen(tar_cmd, stdout=subprocess.PIPE) as tar:
            subprocess.run(["gzip", "-n", "-6"], stdin=tar.stdout, stdout=archive, check=True)
            tar.stdout.close()
            if tar.wait() != 0:
                raise RuntimeError("tar failed")
        archive.flush()
        # Preserve a verified archive even when another run is interrupted.
        with open(archive.name, "rb") as src:
            digest = hashlib.file_digest(src, "sha256").hexdigest()
        staged = Path(archive.name + ".ready")
        os.link(archive.name, staged)
        os.replace(staged, output)
    print(json.dumps({"archive": str(output), "bytes": output.stat().st_size, "sha256": digest,
                      "source_commit": commit, "source_tree": tree, "registry_crates": len(registry)}))


if __name__ == "__main__":
    main()
