#!/usr/bin/env python3
"""Full source-contained extraction verifier. Do not use Git text-only checkout."""
from __future__ import annotations
import hashlib,json,pathlib,os,subprocess,sys,tempfile,zipfile
ROOT=pathlib.Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT/'code'))
from e2_spin2_atlas import restore_owner, hash_file, load_owner_archive
OUTPUT_FILES=('BASS21_E2_SPIN2_50x193x5.npz','ENDPOINT_50_E2_SPIN2.csv','PAIRED_25_E2_SPIN2.csv',
              'NUMERIC_AUDIT_50.csv','SCIENCE_SUMMARY.json','PHYSICS_INTERPRETATION.json','ENDPOINT_50_SHEAR_COLLISION.csv')

def main():
    manifest=json.loads((ROOT/'FILE_MANIFEST.json').read_text())
    for name,sha in manifest['files_sha256'].items():
        actual=hash_file(ROOT/name)
        if actual!=sha:raise AssertionError('FILE_MANIFEST_MISMATCH '+name)
    with zipfile.ZipFile(ROOT/'source/PARENT_PR133_COMPLETE.zip') as z:
        if z.testzip() is not None:raise AssertionError('PARENT_ZIP_CRC_FAILURE')
    env=os.environ.copy();env.update(OPENBLAS_NUM_THREADS='1',OMP_NUM_THREADS='1',MKL_NUM_THREADS='1',PYTHONDONTWRITEBYTECODE='1')
    with tempfile.TemporaryDirectory() as td:
        td=pathlib.Path(td)
        owner=restore_owner(ROOT/'source/PARENT_PR133_COMPLETE.zip',td/'ORIGINAL_BASS21_SOURCE_FULL.zip')
        h=load_owner_archive(owner)
        if len(h)!=50:raise AssertionError('BAD_ORIGINAL_50')
        for args in ((sys.executable,'-m','unittest','discover','-s','tests','-p','test_e2_spin2.py','-v'),
                    (sys.executable,'-m','unittest','discover','-s','tests','-p','test_git_scoped_ci.py','-v')):
            subprocess.run(args,cwd=ROOT,env=env,check=True)
        out=td/'replay';out.mkdir()
        subprocess.run((sys.executable,'code/e2_spin2_atlas.py','--source',str(owner),'--out',str(out)),cwd=ROOT,env=env,check=True,stdout=subprocess.DEVNULL)
        subprocess.run((sys.executable,'code/postprocess_e2.py','--npz',str(out/'BASS21_E2_SPIN2_50x193x5.npz'),'--out',str(out)),cwd=ROOT,env=env,check=True,stdout=subprocess.DEVNULL)
        for name in OUTPUT_FILES:
            if hash_file(out/name)!=hash_file(ROOT/'results'/name):raise AssertionError('FULL_NUMERIC_REPLAY_MISMATCH '+name)
    print('SOURCE_COMPLETE_RESTORED_50/50; FILE_MANIFEST_%d/%d_PASS; UNIT_TESTS_9/9; GIT_TESTS_8/8; FULL_REPLAY_7/7_EXACT; PASS'%(len(manifest['files_sha256']),len(manifest['files_sha256'])))
if __name__=='__main__':main()
