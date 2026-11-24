"""Compile and run an external default and structured consumer against an extracted SDK."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib


def main():
    args = argparse.ArgumentParser()
    args.add_argument("sdk", type=Path)
    sdk = args.parse_args().sdk.resolve()
    ident = json.loads((sdk / "SDK_SOURCE.json").read_text())
    packaging = sdk / "research/unified_referee_20261124/packaging"
    with tempfile.TemporaryDirectory() as temp:
        base = Path(temp)
        app = base / "consumer"
        (app / ".cargo").mkdir(parents=True)
        (app / "src").mkdir()
        cargo_home = base / "empty-cargo-home"
        cargo_home.mkdir()
        # A path dependency does not hand its own .cargo configuration to its consumer.
        config = '[source.crates-io]\nreplace-with="local-sdk"\n[source.local-sdk]\ndirectory='
        (app / ".cargo/config.toml").write_text(config + json.dumps(str(sdk / "vendor")) + "\n")
        manifest = ('[package]\nname="external-sdk-consumer"\nversion="0.0.0"\nedition="2024"\n'
                    '[dependencies]\nvigilode={path=' + json.dumps(str(sdk / "crates/vigilode")) +
                    '}\n[features]\nstructured=["vigilode/structured"]\n[workspace]\n')
        (app / "Cargo.toml").write_text(manifest)
        shutil.copyfile(packaging / "consumer.rs", app / "src/main.rs")
        # Cargo prunes unrelated workspace members and adds the new external root.
        shutil.copyfile(sdk / "Cargo.lock", app / "Cargo.lock")
        channel = tomllib.loads((sdk / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
        env = dict(os.environ, CARGO_HOME=str(cargo_home), CARGO_TARGET_DIR=str(base / "clean-target"),
                   RUSTUP_TOOLCHAIN=channel, CARGO_NET_OFFLINE="true", CARGO_INCREMENTAL="0",
                   CARGO_PROFILE_DEV_DEBUG="0")
        cmd = ["cargo", "metadata", "--offline", "--format-version=1", "--features", "structured"]
        metadata = json.loads(subprocess.check_output(cmd, cwd=app, env=env))
        def registry(lock):
            entries = tomllib.loads(lock.read_text())["package"]
            return {(p["name"], p["version"], p["checksum"]) for p in entries if "source" in p}
        if not registry(app / "Cargo.lock") <= registry(sdk / "Cargo.lock"):
            raise ValueError("consumer dependency graph diverged from SDK lock")
        app_id = next(p for p in metadata["packages"] if Path(p["manifest_path"]) == app / "Cargo.toml")
        if [p["name"] for p in app_id["dependencies"]] != ["vigilode"]:
            raise ValueError("consumer must use only the facade")
        if any(not Path(p["manifest_path"]).resolve().is_relative_to(sdk)
               for p in metadata["packages"] if p["id"] != app_id["id"]):
            raise ValueError("dependency resolved outside extracted SDK")
        outputs = {}
        for name, features in (("default", []), ("structured", ["--features", "structured"])):
            outputs[name] = subprocess.check_output(
                ["cargo", "run", "--locked", "--offline", "--quiet", *features],
                cwd=app, env=env, text=True,
            ).strip().splitlines()
        print(json.dumps({"source_commit": ident["commit"], "consumer_dependencies": ["vigilode"],
                          "offline": True, "initially_empty_cargo_home_and_target": True,
                          "consumer_registry_crates": len(registry(app / "Cargo.lock")),
                          "rustc": subprocess.check_output(["rustc","--version"],cwd=app,env=env,text=True).strip(),
                          "solves": outputs}))


if __name__ == "__main__":
    main()
