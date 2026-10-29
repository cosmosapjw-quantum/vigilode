#!/usr/bin/env python3
"""Preserve all structural native receipts and raw trials, including first failure."""
import argparse
import gzip
import hashlib
import io
import json
from pathlib import Path
import tarfile

parser = argparse.ArgumentParser()
parser.add_argument('--root', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
roots = ['lane-structural-checks01', 'lane-structural02',
         'lane-structural-analysis01', 'lane-structural-audit01',
         'lane-structural-audit02', 'lane-structural-figures01',
         'runtime/build_structural01', 'runtime/build_structural02']
files = sorted(p for name in roots for p in (args.root / name).rglob('*') if p.is_file())
files += [args.root / ('runtime/structural'+index+'/BINDINGS.json') for index in ['01', '02']]
members = {}
for path in files:
    data = path.read_bytes()
    members[path.relative_to(args.root).as_posix()] = {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}
args.output.mkdir(parents=True, exist_ok=True)
archive = args.output / 'STRUCTURAL_RAW_EVIDENCE.tar.gz'
with archive.open('wb') as raw:
    with gzip.GzipFile(fileobj=raw, mode='wb', mtime=0, filename='') as compressed:
        with tarfile.open(fileobj=compressed, mode='w') as tar:
            for name in sorted(members):
                data = (args.root / name).read_bytes()
                assert hashlib.sha256(data).hexdigest() == members[name]['sha256']
                info = tarfile.TarInfo(name)
                info.size = len(data)
                info.mode = 0o644
                info.mtime = 0
                tar.addfile(info, io.BytesIO(data))
with tarfile.open(archive, 'r:gz') as tar:
    assert tar.getnames() == sorted(members)
    for member in tar:
        data = tar.extractfile(member).read()
        assert len(data) == members[member.name]['bytes']
        assert hashlib.sha256(data).hexdigest() == members[member.name]['sha256']
manifest = {'schema': 'vigilode.structural-raw-packet.v1',
            'native_source': '0a2a8eb9b9b0b775561f42f42cf36b1ef6e96f4b',
            'first_checks_source': 'cdbee50feb6b7b5d60bd36a278caaf93fa60f665',
            'archive': archive.name, 'bytes': archive.stat().st_size,
            'sha256': hashlib.sha256(archive.read_bytes()).hexdigest(),
            'member_count': len(members), 'members': members,
            'readback': 'every decompressed member byte count and SHA256 matched',
            'exclusions': 'executables and offline toolchain/vendor payloads are not duplicated; their actual hashes/build receipts are retained'}
(args.output / 'STRUCTURAL_RAW_MANIFEST.json').write_text(json.dumps(manifest, indent=2)+'\n')
print(json.dumps({k: manifest[k] for k in ['bytes', 'sha256', 'member_count', 'readback']}))
