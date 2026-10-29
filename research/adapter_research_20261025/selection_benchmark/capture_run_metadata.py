#!/usr/bin/env python3
"""Capture current source/binary/host bytes before a new measurement, read-only.

Capture is not proof that the binary was built from the current source. The build
receipt must bind its before-build manifest to the resulting binary separately.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys


def sha(data): return hashlib.sha256(data).hexdigest()
def encoded(obj): return json.dumps(obj, sort_keys=True, separators=(',', ':'), allow_nan=False).encode()
def command(args, cwd):
    p = subprocess.run(args,cwd=cwd,capture_output=True,text=True,check=False)
    return {'argv':args,'exit_code':p.returncode,'stdout':p.stdout,'stderr':p.stderr}

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--repo',required=True,type=Path)
    p.add_argument('--binary',type=Path)
    p.add_argument('--build-command',required=True,help='exact recorded build command, descriptive only; not executed')
    p.add_argument('--build-receipt',type=Path,help='actual saved build log/receipt; hash only, no claim of successful validation')
    p.add_argument('--output',required=True,type=Path)
    a=p.parse_args(); root=a.repo.resolve()
    listing=subprocess.run(['git','ls-files','-z','--cached','--others','--exclude-standard','--',
                            'crates','Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo/config.toml'],
                           cwd=root,capture_output=True,check=True).stdout
    files={}
    for name in sorted(set(x.decode() for x in listing.split(b'\0') if x)):
        path=root/name
        if path.is_file():
            if path.is_symlink(): raise ValueError('source symlinks require explicit resolved provenance: '+name)
            content=path.read_bytes();files[name]={'sha256':sha(content),'bytes':len(content)}
    host={'uname':dict(platform.uname()._asdict()),'logical_cpu_count':os.cpu_count(),
          'affinity':sorted(os.sched_getaffinity(0)) if hasattr(os,'sched_getaffinity') else None,
          'cpuinfo_sha256':sha(Path('/proc/cpuinfo').read_bytes()) if Path('/proc/cpuinfo').exists() else None}
    # /proc/cpuinfo contains variable MHz; stable host comparison uses uname/cpu count/affinity.
    host_key={k:v for k,v in host.items() if k!='cpuinfo_sha256'}
    config={'declared_build_command':a.build_command,
            'environment':{k:v for k,v in os.environ.items() if k in {'RUSTFLAGS','CARGO_BUILD_JOBS','OMP_NUM_THREADS','OPENBLAS_NUM_THREADS','MKL_NUM_THREADS'} or k.startswith('CARGO_PROFILE_')},
            'rustc':command(['rustc','-Vv'],root),'cargo':command(['cargo','-V'],root)}
    result={'schema_version':1,'kind':'pre_measurement_snapshot',
            'source_manifest':files,'source_tree_sha256':sha(encoded(files)),
            'source_identity_semantics':'SHA256 of canonical tracked+untracked crates/Cargo/toolchain source-file manifest; not Git tree object ID',
            'host':host,'host_sha256':sha(encoded(host_key)),
            'build_config':config,'build_config_sha256':sha(encoded(config)),
            'git_head':command(['git','rev-parse','HEAD'],root),
            'binary':None,'build_receipt':None,
            'binding_status':'SNAPSHOT_ONLY_BINARY_SOURCE_RELATION_REQUIRES_BUILD_RECEIPT_REVIEW'}
    if a.binary:
        b=a.binary.resolve();data=b.read_bytes()
        result['binary']={'path':str(b),'bytes':len(data),'sha256':sha(data)}
    if a.build_receipt:
        b=a.build_receipt.resolve();data=b.read_bytes()
        result['build_receipt']={'path':str(b),'bytes':len(data),'sha256':sha(data)}
    with a.output.open('x',encoding='utf8') as out:
        json.dump(result,out,indent=2,ensure_ascii=False,allow_nan=False);out.write('\n')
    return 0
if __name__=='__main__': raise SystemExit(main())
