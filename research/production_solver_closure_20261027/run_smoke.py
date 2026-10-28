#!/usr/bin/env python3
"""Build and run both deployment profiles on fixed, independently checked IVPs.

Invoke from the repository root under the native measurement lock. Toolchain
and Cargo target/cache paths are supplied by the local environment. All output
directories are new; a rerun never overwrites prior evidence.
"""
import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--profile", choices=("measurement", "release"), default="measurement")
    parser.add_argument("--online", action="store_true", help="allow Cargo's locked dependency fetch")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    sources = [root / p for p in (
        "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "crates/vigilode/src/production.rs",
        "crates/vigilode/src/switching.rs", "crates/vigilode/examples/production_from_json.rs")]
    source_hashes = {str(p.relative_to(root)): sha(p) for p in sources}
    receipt = {"schema":"vigilode.production-smoke-batch.v1",
               "started_utc":dt.datetime.now(dt.timezone.utc).isoformat(),
               "source_sha256":source_hashes, "profile":args.profile, "commands":[]}

    def run(command, name):
        start = time.monotonic()
        with (output / f"{name}.stdout").open("wb") as stdout, (output / f"{name}.stderr").open("wb") as stderr:
            result = subprocess.run(command, cwd=root, stdout=stdout, stderr=stderr, check=False)
        entry = {"name":name,"argv":command,"exit_code":result.returncode,"wall_seconds":time.monotonic()-start}
        receipt["commands"].append(entry)
        (output / "RECEIPT.json").write_text(json.dumps(receipt,indent=2)+"\n")
        if result.returncode:
            raise SystemExit(result.returncode)
        return output / f"{name}.stdout"

    build = ["cargo","build","--profile",args.profile,"--locked","-p","vigilode","--example","production_from_json"]
    if not args.online:
        build.append("--offline")
    run(build,"build")
    target = Path(os.environ.get("CARGO_TARGET_DIR",str(root / "target")))
    if not target.is_absolute(): target = root / target
    binary = (target / args.profile / "examples" / "production_from_json").resolve()
    receipt["binary_sha256"] = sha(binary)
    summary = []
    for mode in ("rhs-only","selective-jvp"):
        template = run([str(binary),"--template",mode],f"template_{mode}")
        config = output / f"{mode}.json"
        config.write_bytes(template.read_bytes())
        for rate in (1,1000):
            path = run([str(binary),str(config),str(rate)],f"{mode}_{rate}")
            data = json.loads(path.read_text())
            if data["pass"] is not True: raise SystemExit("native smoke assertion failed")
            stats = data["solution"]["stats"]
            summary.append({"mode":mode,"rate":rate,"pass":data["pass"],
                            "maximum_returned_state_error":data["maximum_returned_state_error"],
                            "rhs":stats["rhs_calls"],"jvp":stats["jvp_vectors"],
                            "attempts":stats["attempts"],"accounting_complete":stats["accounting_complete"]})
    receipt["source_unchanged"] = all(sha(p)==source_hashes[str(p.relative_to(root))] for p in sources)
    receipt["status"] = "completed"
    receipt["summary"] = summary
    (output / "RECEIPT.json").write_text(json.dumps(receipt,indent=2)+"\n")
    (output / "SUMMARY.json").write_text(json.dumps(summary,indent=2)+"\n")
    print(json.dumps({"status":"completed","source_unchanged":receipt["source_unchanged"],"arms":summary}),flush=True)
    if not receipt["source_unchanged"]: raise SystemExit("source changed during smoke execution")


if __name__ == "__main__":
    main()
