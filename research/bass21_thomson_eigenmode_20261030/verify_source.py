#!/usr/bin/env python3
"""Clean source-full verifier: truly original BASS21 source, scientific NPZ and replay.

Distinct from the Git source-free scoped CI. No new initial-value problem executed.
"""
from __future__ import annotations
import hashlib,io,json,os,pathlib,subprocess,sys,tempfile,zipfile
ROOT=pathlib.Path(__file__).resolve().parent
OWNER_SHA='6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8'
PARENT_SHA='ce2b3507c30f5429fb93244bd7f3de7426f1f158bf52962e24734b6d1237dfe2'
PR138_SHA='ab7044f604fdd7badb7eb21bae60f7900dd47e1e81b0ed1cce3091283162309f'
OUTPUTS=('EIGENMODE_50x193_SOURCE_PHYSICS.npz','EIGENMODE_CASE50_ENDPOINT.csv',
         'EIGENMODE_PAIR25_ENDPOINT.csv','EIGENMODE_TIME193_MEANS.csv',
         'SCIENCE_SUMMARY.json','EXACT_ALGEBRA.json')

def sha(p):
    with open(p,'rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def recover_owner(parent,td):
    if sha(parent)!=PARENT_SHA:raise AssertionError('PARENT_PR141_RAW_SHA_MISMATCH')
    with zipfile.ZipFile(parent) as z:
        if z.testzip() is not None:raise AssertionError('PARENT_PR141_ZIP_CRC')
        pr138=z.read('source/PARENT_PR138_FULL_SOURCE.zip')
    assert hashlib.sha256(pr138).hexdigest()==PR138_SHA,'PR138_RAW_SHA_DRIFT'
    pr138_path=td/'PR138_original.zip';pr138_path.write_bytes(pr138)
    sys.path.insert(0,str(ROOT/'source'))
    from PR141_verify_source import original_from_nested
    owner_bytes=original_from_nested(pr138_path)
    assert hashlib.sha256(owner_bytes).hexdigest()==OWNER_SHA,'OWNER_RAW_BYTE_DRIFT'
    owner_path=td/'ORIGINAL_BASS21_SOURCE_FULL.zip';owner_path.write_bytes(owner_bytes)
    return owner_path

def main():
    manifest=json.loads((ROOT/'FILE_MANIFEST.json').read_text())
    for k,v in manifest['files_sha256'].items():
        p=ROOT/k
        if not p.is_file() or sha(p)!=v:raise AssertionError('SOURCE_MANIFEST_SHA_MISMATCH:'+k)
    env=os.environ.copy()
    env.update({'OPENBLAS_NUM_THREADS':'1','OMP_NUM_THREADS':'1','MKL_NUM_THREADS':'1','PYTHONDONTWRITEBYTECODE':'1'})
    with tempfile.TemporaryDirectory() as t:
        td=pathlib.Path(t)
        owner=recover_owner(ROOT/'source/PARENT_PR141_FULL_SOURCE.zip',td)
        sys.path.insert(0,str(ROOT/'source_code'))
        from coupled_thomson_volterra import load_owner_archive
        hist=load_owner_archive(owner)
        previous=json.loads((ROOT/'source/PR141_SCIENCE_SUMMARY.json').read_text())
        science=json.loads((ROOT/'results/SCIENCE_SUMMARY.json').read_text())
        if len(hist)!=50:raise AssertionError('OWNER_COUNT_NOT_50')
        for key in sorted(hist):
            if hist[key]['source_sha256'] != previous['source_case_SHA256'][key] or hist[key]['source_sha256']!=science['source_sha256_by_case'][key]:
                raise AssertionError('BASS_OWNER_CASE_SHA:'+key)
        subprocess.run([sys.executable,'-m','unittest','discover','-s','tests','-p','test_eigenmode.py','-v'],cwd=ROOT,env=env,check=True)
        subprocess.run([sys.executable,'code/check_git_scoped.py'],cwd=ROOT,env=env,check=True)
        result_dir=td/'regen';result_dir.mkdir()
        subprocess.run([sys.executable,'code/eigenmode_atlas.py','--owner',str(owner),'--parent',str(ROOT/'source/PARENT_PR141_FULL_SOURCE.zip'),'--out',str(result_dir),'--limit','50'],cwd=ROOT,env=env,check=True,stdout=subprocess.DEVNULL)
        for name in OUTPUTS:
            if sha(ROOT/'results'/name)!=sha(result_dir/name):raise AssertionError('FULL_SOURCE_NUMERIC_BYTE_REPLAY_FAIL:'+name)
        print('ORIGINAL_BASS_50/50_SHA_PASS; FULL_MANIFEST_%d/%d_PASS; UNIT_TESTS_10/10; GIT_SCOPED_PASS; SCIENCE_REPLAY_6/6_BYTE_EXACT; NEW_IVPS=0; PASS' %(len(manifest['files_sha256']),len(manifest['files_sha256'])))
if __name__=='__main__':main()
