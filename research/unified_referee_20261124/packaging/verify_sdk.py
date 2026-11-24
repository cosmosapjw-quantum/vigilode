"""Check first-party blobs, locked registry file checksums and bundled license material."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import tomllib


def main(archive):
    prefix = "vigilode-sdk/"
    files = {}
    checksums = {}
    small = {}
    with tarfile.open(archive, "r|gz") as tar:
        for member in tar:
            if not member.isfile():
                continue
            if not member.name.startswith(prefix):
                raise ValueError(member.name)
            path = member.name[len(prefix):]
            if path in files:
                raise ValueError("duplicate archive path " + path)
            data = tar.extractfile(member).read()
            files[path] = hashlib.sha256(data).hexdigest()
            if path.endswith("/.cargo-checksum.json"):
                checksums[path.rsplit("/", 1)[0]] = json.loads(data)
            if path in ("LICENSE", "Cargo.lock", "SDK_SOURCE.json") or (
                path.startswith("vendor/") and path.endswith("/Cargo.toml") and len(path.split("/")) == 3
            ):
                small[path] = data
    source = json.loads(small["SDK_SOURCE.json"])
    packages = [p for p in tomllib.loads(small["Cargo.lock"].decode())["package"] if "source" in p]
    vendor_files = 0
    license_files = 0
    manifest_license_only = []
    for p in packages:
        name = "vendor/" + p["name"] + "-" + p["version"]
        chk = checksums[name]
        assert chk["package"] == p["checksum"]
        for path, checksum in chk["files"].items():
            if files.get(name + "/" + path) != checksum:
                raise ValueError("vendor file mismatch " + name + "/" + path)
            vendor_files += 1
        material = [p for p in chk["files"] if any(t in Path(p).name.lower() for t in ("license", "licence", "copying", "notice"))]
        license_files += len(material)
        manifest = tomllib.loads(small[name + "/Cargo.toml"].decode())["package"]
        if "license-file" in manifest:
            assert name + "/" + manifest["license-file"] in files
        if not material:
            if not manifest.get("license") and not manifest.get("license-file"):
                raise ValueError("package lacks license metadata " + name)
            manifest_license_only.append(name)
    if len(checksums) != len(packages):
        raise ValueError("SDK includes packages outside the lock")
    checked = 0
    paths = source["archives"]
    out = subprocess.check_output(["git", "ls-tree", "-rz", source["commit"], "--", *paths])
    # A persistent cat-file process avoids spawning a process for each tracked source blob.
    with subprocess.Popen(["git", "cat-file", "--batch"], stdin=subprocess.PIPE, stdout=subprocess.PIPE) as cat:
        for row in out.split(b"\0"):
            if not row:
                continue
            meta, name = row.decode().split("\t", 1)
            mode, kind, obj = meta.split()
            if kind != "blob" or mode == "120000" or name == ".cargo/config.toml":
                continue
            cat.stdin.write(obj.encode()+b"\n")
            cat.stdin.flush()
            length = int(cat.stdout.readline().split()[-1])
            data = cat.stdout.read(length)
            assert cat.stdout.read(1) == b"\n"
            if files.get(name) != hashlib.sha256(data).hexdigest():
                raise ValueError("first-party archive mismatch " + name)
            checked += 1
        cat.stdin.close()
    fixtures = sorted(n for n in files if n.startswith("fixtures/"))
    assert "fixtures/rodas5p_coefficients_snapshot.json" in fixtures and small["LICENSE"]
    print(json.dumps({"first_party_verified_blobs": checked, "config_replaced_by_vendor_source": ".cargo/config.toml",
                      "fixtures": len(fixtures), "locked_registry_packages": len(packages),
                      "verified_registry_file_checksums": vendor_files,
                      "registry_named_license_notice_files": license_files,
                      "registry_packages_with_only_manifest_license": manifest_license_only,
                      "first_party_license_sha256": files["LICENSE"]}))


if __name__ == "__main__":
    main(sys.argv[1])
