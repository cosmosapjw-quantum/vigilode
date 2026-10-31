#!/usr/bin/env python3
"""Verify user-supplied full-source ZIP and run original clean physics tests.
GitHub holds source-free evidence only; do not claim full-source CI here.
"""
import pathlib,hashlib,zipfile,tempfile,subprocess,sys,os
EXPECTED="e8faa022fb6e567a97f17f73cca4571f071ffa37de5ea8e9597b0c4d98b088bc"
if len(sys.argv)!=2: raise SystemExit("usage: python source_full_replay.py FULL_SOURCE.zip")
p=pathlib.Path(sys.argv[1]);h=hashlib.sha256(p.read_bytes()).hexdigest()
if h!=EXPECTED:raise SystemExit("SOURCE_FULL_ARCHIVE_SHA256_MISMATCH")
with tempfile.TemporaryDirectory(prefix="vigilode_axisboost_") as tmp:
 with zipfile.ZipFile(p) as z:
  if z.testzip() is not None: raise SystemExit("ZIP_CRC_FAIL")
  z.extractall(tmp)
 env=os.environ.copy();env.update(OPENBLAS_NUM_THREADS="1",OMP_NUM_THREADS="1",MKL_NUM_THREADS="1",PYTHONDONTWRITEBYTECODE="1")
 subprocess.run(["bash","VERIFY.sh"],cwd=tmp,env=env,check=True)
print("SOURCE_FULL_REPLAY_SCOPED_PASS")
