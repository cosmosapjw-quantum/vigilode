#!/usr/bin/env python3
"""Complete only exact runtime payloads; preserve partial restore and duplicate metadata."""
import hashlib,json,os,shutil,tarfile
from pathlib import Path,PurePosixPath
root=Path(__file__).resolve().parent
archive=root.parent.parent/'project_sources/06-rust-1.94.1-x86_64-unknown-linux-gnu.tar.xz'
expected='294b3d81fa72e62581276290c60c81eb8b58498d333d422ca1dfc432877d0c40'
assert hashlib.sha256(archive.read_bytes()).hexdigest()==expected
dest=root/'toolchain';dest.mkdir(exist_ok=True)
reused=created=skipped=0
with tarfile.open(archive,'r|xz') as tf:
 for m in tf:
  parts=PurePosixPath(m.name).parts
  if len(parts)<3 or parts[1] not in ('rustc','rust-std-x86_64-unknown-linux-gnu') or not m.isfile():continue
  rel=Path(*parts[2:])
  if rel.is_absolute() or '..' in rel.parts:raise ValueError('unsafe path')
  if rel.parts[0] not in ('bin','lib'):skipped+=1;continue
  target=dest/rel;target.parent.mkdir(parents=True,exist_ok=True)
  with tf.extractfile(m) as src:
   if target.exists():
    original=hashlib.sha256()
    for chunk in iter(lambda:src.read(1024*1024),b''):original.update(chunk)
    if original.hexdigest()!=hashlib.sha256(target.read_bytes()).hexdigest():raise RuntimeError('partial runtime byte mismatch '+str(rel))
    reused+=1
   else:
    with target.open('xb') as out:shutil.copyfileobj(src,out)
    os.chmod(target,m.mode & 0o777);created+=1
print(json.dumps({'archive_sha256':expected,'bytes':archive.stat().st_size,'runtime_files_reused_verified':reused,'runtime_files_added':created,'component_metadata_skipped':skipped,'prior_partial_and_failure_preserved':True,'old_science_replayed':0}))
