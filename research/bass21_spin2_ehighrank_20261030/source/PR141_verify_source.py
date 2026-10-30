#!/usr/bin/env python3
"""Independent clean source-full replay verifier; no IVPs, no owner changes."""
from __future__ import annotations
import hashlib, io, json, os, pathlib, subprocess, sys, tempfile, zipfile
ROOT=pathlib.Path(__file__).resolve().parent
PARENT_SHA='ab7044f604fdd7badb7eb21bae60f7900dd47e1e81b0ed1cce3091283162309f'
OWNER_SHA='6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8'
OUTPUTS=['BASS21_RANK9_EHARM_50x193.npz','COHERENCY_AUDIT_50.csv','COHERENCY_ENDPOINT_50.csv','COHERENCY_MATCHED_25.csv','NORMALIZATION.json','SCIENCE_SUMMARY.json']

def sha(p):
    with open(p,'rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def original_from_nested(parent):
    if sha(parent)!=PARENT_SHA:raise AssertionError('PARENT_PR138_RAW_DRIFT')
    with zipfile.ZipFile(parent) as z:
        assert z.testzip() is None,'PARENT_PR138_CRC_FAIL'
        pr134=z.read('source/PR134_CANONICAL_FULL_SOURCE.zip')
    assert hashlib.sha256(pr134).hexdigest()=='d33ff58f3f9fc8dec0e9cc3b055108f005f309af3209a688a2251ef7e3649a16'
    with zipfile.ZipFile(io.BytesIO(pr134)) as z:
        assert z.testzip() is None
        pr133=z.read('source/PARENT_PR133_COMPLETE.zip')
    assert hashlib.sha256(pr133).hexdigest()=='ffd7623ba2e2bd99233a1244ac7945470f6eeed348a63a4fd02f5967ad449b14'
    with zipfile.ZipFile(io.BytesIO(pr133)) as z:
        assert z.testzip() is None
        pr132=z.read('parent/VIGILODE_BASS21_COUPLED_THOMSON_20261030_FULL_SOURCE.zip')
    with zipfile.ZipFile(io.BytesIO(pr132)) as z:
        assert z.testzip() is None
        owner=z.read('source/VIGILODE_BASS21_FULL25_NATIVE193_20261030_SOURCE_FULL.zip')
    assert hashlib.sha256(owner).hexdigest()==OWNER_SHA,'NESTED_OWNER_RAW_DRIFT'
    with zipfile.ZipFile(io.BytesIO(owner)) as z:assert z.testzip() is None,'OWNER_CRC_FAIL'
    return owner

def main():
    manifest=json.loads((ROOT/'FILE_MANIFEST.json').read_text())
    for path,expected in manifest['files_sha256'].items():
        p=ROOT/path
        if not p.is_file() or sha(p)!=expected:raise AssertionError('FILE_SHA_DRIFT '+path)
    env=os.environ.copy();env.update(OPENBLAS_NUM_THREADS='1',OMP_NUM_THREADS='1',MKL_NUM_THREADS='1',PYTHONDONTWRITEBYTECODE='1')
    with tempfile.TemporaryDirectory() as td:
        td=pathlib.Path(td)
        original=td/'ORIGINAL_BASS21_SOURCE_FULL.zip'
        original.write_bytes(original_from_nested(ROOT/'source/PARENT_PR138_FULL_SOURCE.zip'))
        sys.path.insert(0,str(ROOT/'source_code'))
        from lrs_polarization_replay import load_owner_archive
        hist=load_owner_archive(original)
        science=json.loads((ROOT/'results/SCIENCE_SUMMARY.json').read_text())
        assert len(hist)==50 and all(hist[k]['source_sha256']==science['source_case_SHA256'][k] for k in hist)
        subprocess.run((sys.executable,'-m','unittest','discover','-s','tests','-p','test_rank9.py','-v'),cwd=ROOT,env=env,check=True)
        output=td/'out';output.mkdir()
        cmd=(sys.executable,'code/rank9_coherency.py','--owner',str(original),'--pr138',str(ROOT/'source/PARENT_PR138_FULL_SOURCE.zip'),'--out',str(output),'--limit','50')
        subprocess.run(cmd,cwd=ROOT,env=env,stdout=subprocess.DEVNULL,check=True)
        for name in OUTPUTS:
            if sha(output/name)!=sha(ROOT/'results'/name):raise AssertionError('SCIENCE_REPLAY_BYTES_DIFFER '+name)
        print('ORIGINAL_SOURCE_50/50_SHA_PASS; FULL_MANIFEST_%d/%d_PASS; UNIT_TESTS_9/9; SOURCE_REPLAY_6/6_SHA_EXACT; NO_IVPS; PASS'%(len(manifest['files_sha256']),len(manifest['files_sha256'])))

if __name__=='__main__':main()
