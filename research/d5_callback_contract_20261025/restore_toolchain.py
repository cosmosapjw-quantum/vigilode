#!/usr/bin/env python3
"""Restore the supplied pinned compiler after observed scratch loss, without fetching."""
import hashlib,json,os,shutil,tarfile
from pathlib import Path,PurePosixPath
root=Path(__file__).resolve().parent
archive=root.parent.parent/'project_sources/06-rust-1.94.1-x86_64-unknown-linux-gnu.tar.xz'
expected='294b3d81fa72e62581276290c60c81eb8b58498d333d422ca1dfc432877d0c40'
sha=hashlib.sha256(archive.read_bytes()).hexdigest()
if sha!=expected:raise SystemExit('archive identity mismatch: '+sha)
dest=root/'toolchain'
if dest.exists():raise SystemExit('toolchain destination exists; do not duplicate restore')
dest.mkdir();count=0
with tarfile.open(archive,'r|xz') as tf:
 for m in tf:
  parts=PurePosixPath(m.name).parts
  if len(parts)<3 or parts[1] not in ('rustc','rust-std-x86_64-unknown-linux-gnu') or not m.isfile():continue
  rel=Path(*parts[2:])
  if rel.is_absolute() or '..' in rel.parts:raise ValueError('unsafe archive path')
  target=dest/rel;target.parent.mkdir(parents=True,exist_ok=True)
  with tf.extractfile(m) as src,target.open('xb') as f:shutil.copyfileobj(src,f)
  os.chmod(target,m.mode & 0o777);count+=1
print(json.dumps({'supplied_archive':str(archive),'bytes':archive.stat().st_size,'sha256':sha,'files_extracted':count,'signature_verification':'NOT_RUN','reason':'Old scratch toolchain absent after observed automated workspace pruning. User attachment read locally. No network fetch.'}))
