#!/usr/bin/env python3
"""Pack the raw provocation reports of node V32 (provoke.sh) into one archive per condition.

For every directory logs/<batch>/<binary label>/<condition>/ that holds loose run_NNN.json files (and
run_NNN.json.stderr files), write reports.tar.xz with exactly those bytes, write reports.sha256 (one
line per member: sha256, size, name), read the archive back, compare every member with the loose file
and only then delete the loose files. Read by parse_results.py. Refuses to touch a directory that
already has an archive and also loose files (a batch must be packed once, after it is complete).

usage: python3 -I pack_logs.py <logs/batch directory>...
"""
import hashlib
import io
import os
import sys
import tarfile


def pack(directory):
    names = sorted(n for n in os.listdir(directory)
                   if n.startswith("run_") and (n.endswith(".json") or n.endswith(".json.stderr")))
    archive = os.path.join(directory, "reports.tar.xz")
    if not names:
        return 0
    if os.path.exists(archive):
        raise SystemExit(f"{directory}: archive exists and loose files remain; refusing")
    blobs = {}
    for name in names:
        with open(os.path.join(directory, name), "rb") as handle:
            blobs[name] = handle.read()
    with tarfile.open(archive, "w:xz", format=tarfile.PAX_FORMAT) as tar:
        for name in names:
            info = tarfile.TarInfo(name)
            info.size = len(blobs[name])
            info.mtime = int(os.path.getmtime(os.path.join(directory, name)))
            info.mode = 0o644
            tar.addfile(info, io.BytesIO(blobs[name]))
    with tarfile.open(archive, "r:xz") as tar:
        members = {m.name: tar.extractfile(m).read() for m in tar.getmembers()}
    if members != blobs:
        os.remove(archive)
        raise SystemExit(f"{directory}: archive read-back differs; loose files kept")
    with open(os.path.join(directory, "reports.sha256"), "w", encoding="utf-8") as handle:
        for name in names:
            handle.write(f"{hashlib.sha256(blobs[name]).hexdigest()}  {len(blobs[name])}  {name}\n")
    for name in names:
        os.remove(os.path.join(directory, name))
    return len(names)


def main():
    for batch in sys.argv[1:]:
        for label in sorted(os.listdir(batch)):
            label_dir = os.path.join(batch, label)
            if not os.path.isdir(label_dir):
                continue
            for condition in sorted(os.listdir(label_dir)):
                directory = os.path.join(label_dir, condition)
                if os.path.isdir(directory):
                    print(f"{directory}: packed {pack(directory)} files")


if __name__ == "__main__":
    main()
