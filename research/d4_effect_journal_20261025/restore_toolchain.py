#!/usr/bin/env python3
"""Fetch the exact official Rust archive when the supplied scratch copy is absent."""
import hashlib,json,os,shutil,tarfile,urllib.request
from pathlib import Path,PurePosixPath
root=Path(__file__).resolve().parent
archive=root.parent/'rust-1.94.1-x86_64-unknown-linux-gnu.tar.xz'
url='https://static.rust-lang.org/dist/rust-1.94.1-x86_64-unknown-linux-gnu.tar.xz'
expected='294b3d81fa72e62581276290c60c81eb8b58498d333d422ca1dfc432877d0c40'
if not archive.exists():
 with urllib.request.urlopen(url,timeout=30) as r, archive.open('xb') as f:shutil.copyfileobj(r,f)
h=hashlib.sha256(archive.read_bytes()).hexdigest()
if h!=expected:raise SystemExit('archive identity mismatch: '+h)
dest=root/'toolchain';dest.mkdir(exist_ok=True)
count=0
with tarfile.open(archive,'r|xz') as tf:
 for m in tf:
  parts=PurePosixPath(m.name).parts
  if len(parts)<3 or parts[1] not in ('rustc','rust-std-x86_64-unknown-linux-gnu') or not m.isfile():continue
  rel=Path(*parts[2:])
  if rel.is_absolute() or '..' in rel.parts:raise ValueError('unsafe archive path')
  target=dest/rel;target.parent.mkdir(parents=True,exist_ok=True)
  with tf.extractfile(m) as src,target.open('wb') as f:shutil.copyfileobj(src,f)
  os.chmod(target,m.mode & 0o777);count+=1
print(json.dumps({'source':url,'bytes':archive.stat().st_size,'sha256':h,'matches_historical_uploaded_archive':True,'files_extracted':count,'signature_verification':'NOT_RUN','reason':'Provided scratch attachment and installed compiler were absent after observed runtime file loss; no attachment was read through Library.'}))
