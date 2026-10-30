#!/usr/bin/env python3
"""Clean source-inclusive rehydration/replay for source-owned BASS21 J/c generator."""
from __future__ import annotations
import hashlib, json, os, pathlib, subprocess, sys, tempfile, zipfile
ROOT=pathlib.Path(__file__).resolve().parent
ORIGINAL_SHA='6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8'
PR134_SHA='d33ff58f3f9fc8dec0e9cc3b055108f005f309af3209a688a2251ef7e3649a16'
OUTPUTS=('EXACT_GENERATOR.json','BASS21_50x193_JC_GENERATOR.npz','SCIENCE_SUMMARY.json',
         'ENDPOINT_50_NEW_JC.csv','PAIRED_25_NEW_JC.csv','AUDIT_50_JC.csv')

def sha(p):
    with open(p,'rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def main():
    manifest=json.loads((ROOT/'FILE_MANIFEST.json').read_text())
    for name,expected in manifest['files_sha256'].items():
        p=ROOT/name
        if not p.is_file() or sha(p)!=expected:
            raise AssertionError('FILE_SHA_MISMATCH '+name)
    canonical=ROOT/'source/PR134_CANONICAL_FULL_SOURCE.zip'
    assert sha(canonical)==PR134_SHA,'PR134_ORIGINAL_RAW_SHA_DRIFT'
    with zipfile.ZipFile(canonical) as archive:
        assert archive.testzip() is None, 'PR134_ZIP_CRC_FAIL'
    env=os.environ.copy()
    env.update(OPENBLAS_NUM_THREADS='1',OMP_NUM_THREADS='1',MKL_NUM_THREADS='1',PYTHONDONTWRITEBYTECODE='1')
    with tempfile.TemporaryDirectory() as td:
        root=pathlib.Path(td)
        with zipfile.ZipFile(canonical) as z:z.extractall(root/'base')
        parentzip=root/'base/source/PARENT_PR133_COMPLETE.zip'
        with zipfile.ZipFile(parentzip) as z:z.extractall(root/'parent')
        sys.path.insert(0,str(root/'base/code'))
        from e2_spin2_atlas import restore_owner
        original=restore_owner(parentzip,root/'ORIGINAL_BASS21_SOURCE_FULL.zip')
        assert sha(original)==ORIGINAL_SHA,'ORIGINAL_OWNER_SHA_DRIFT'
        with zipfile.ZipFile(original) as z:assert z.testzip() is None, 'SOURCE_ZIP_CRC_FAIL'
        # Relocate only text/code/result payload for a clean isolated execution.
        # Canonical original bytes are read from the existing locked package, unchanged.
        import shutil
        wr=root/'work'
        shutil.copytree(ROOT,wr,ignore=shutil.ignore_patterns('__pycache__','*.pyc'))
        p=wr/'results';p.mkdir(exist_ok=True)
        cmd=(sys.executable,'-m','unittest','discover','-s','tests','-p','test_jc_generator.py','-v')
        subprocess.run(cmd,cwd=wr,env=env,check=True)
        rep=root/'replayed';rep.mkdir()
        cmd=(sys.executable,'code/jc_generator.py','--source',str(original),
             '--canonical-parent',str(wr/'source/PR134_CANONICAL_FULL_SOURCE.zip'),
             '--reference-j',str(root/'parent/results/STRESS_50x193x6_SAME_OWNER.npz'),
             '--reference-c',str(root/'base/results/BASS21_E2_SPIN2_50x193x5.npz'),
             '--out',str(rep),'--limit','50')
        subprocess.run(cmd,cwd=wr,env=env,check=True,stdout=subprocess.DEVNULL)
        for name in OUTPUTS:
            if sha(rep/name)!=sha(wr/'results'/name):
                raise AssertionError('OUTPUT_REPLAY_SHA_MISMATCH '+name)
        sys.path.insert(0,str(root/'base/source_code'))
        from coupled_thomson_volterra import load_owner_archive
        hist=load_owner_archive(original)
        expected=json.loads((wr/'results/SCIENCE_SUMMARY.json').read_text())['source_case_hashes_50']
        assert len(hist)==len(expected)==50
        assert all(hist[k]['source_sha256']==v for k,v in expected.items()),'SOURCE_CASE_DRIFT'
    print(f'SOURCE_50/50_SHA; FILE_MANIFEST_{len(manifest["files_sha256"])}/{len(manifest["files_sha256"])}_SHA; UNITTEST_13/13; FULL_REPLAY_6/6_SHA_EXACT; PASS')

if __name__=='__main__':main()
