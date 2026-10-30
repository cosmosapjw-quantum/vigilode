#!/usr/bin/env python3
"""Actual clean source-full nested ZIP identity, 50-case replay and tests."""
import hashlib,io,json,os,pathlib,subprocess,sys,tempfile,zipfile
ROOT=pathlib.Path(__file__).resolve().parent
OWNER_SHA='6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8'
PARENT_SHA='a2fa5e7cbae98cb7a28df19314b1c6da8b90ad5277c08831384290171f4231ee'
PR141_SHA='ce2b3507c30f5429fb93244bd7f3de7426f1f158bf52962e24734b6d1237dfe2'
PR138_SHA='ab7044f604fdd7badb7eb21bae60f7900dd47e1e81b0ed1cce3091283162309f'
OUTPUTS=['HIGH_E_HIERARCHY_50x193.npz','ENDPOINT_50_HIGH_E.csv','MATCHED_25_HIGH_E.csv','TIME193_HIGH_E_MEANS.csv','SCIENCE_SUMMARY.json','EXACT_LRS_E_OPERATOR.json']
def sha(p):
    with open(p,'rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def original_from_parent(parent,td):
    assert sha(parent)==PARENT_SHA,'PR143_PARENT_SHA_DRIFT'
    with zipfile.ZipFile(parent) as z:
        assert z.testzip() is None,'PR143_ZIP_CRC_FAIL'
        pr141=z.read('source/PARENT_PR141_FULL_SOURCE.zip')
    assert hashlib.sha256(pr141).hexdigest()==PR141_SHA,'PR141_SHA_DRIFT'
    with zipfile.ZipFile(io.BytesIO(pr141)) as z:
        assert z.testzip() is None,'PR141_ZIP_CRC_FAIL'
        pr138=z.read('source/PARENT_PR138_FULL_SOURCE.zip')
    assert hashlib.sha256(pr138).hexdigest()==PR138_SHA,'PR138_SHA_DRIFT'
    tmp=td/'PR138_SOURCE.zip';tmp.write_bytes(pr138)
    sys.path.insert(0,str(ROOT/'source'))
    from PR141_verify_source import original_from_nested
    owner=original_from_nested(tmp)
    assert hashlib.sha256(owner).hexdigest()==OWNER_SHA,'BASS_OWNER_SHA_DRIFT'
    op=td/'ORIGINAL_BASS21_SOURCE_FULL.zip';op.write_bytes(owner)
    return op

def main():
    manifest=json.loads((ROOT/'FILE_MANIFEST.json').read_text())
    for path,digest in manifest['files_sha256'].items():
        f=ROOT/path
        if not f.is_file() or sha(f)!=digest:raise AssertionError(('FILE_MANIFEST_DRIFT',path))
    env=os.environ.copy();env.update(OPENBLAS_NUM_THREADS='1',OMP_NUM_THREADS='1',MKL_NUM_THREADS='1',PYTHONDONTWRITEBYTECODE='1')
    with tempfile.TemporaryDirectory() as d:
        td=pathlib.Path(d); owner=original_from_parent(ROOT/'source/PARENT_PR143_FULL_SOURCE.zip',td)
        sys.path.insert(0,str(ROOT/'source_code'))
        from coupled_thomson_volterra import load_owner_archive
        h=load_owner_archive(owner)
        s=json.loads((ROOT/'results/SCIENCE_SUMMARY.json').read_text())
        assert len(h)==50
        for name,z in h.items():
            assert z['source_sha256']==s['source_by_case_SHA256'][name],('OWNER_CASE_SHA_DRIFT',name)
        subprocess.run([sys.executable,'-m','unittest','discover','-s','tests','-p','test_highrank.py','-v'],cwd=ROOT,env=env,check=True)
        subprocess.run([sys.executable,'code/check_git_scoped.py'],cwd=ROOT,env=env,check=True)
        tmpout=td/'regenerated';tmpout.mkdir()
        subprocess.run([sys.executable,'code/highrank_atlas.py','--owner',str(owner),'--parent',str(ROOT/'source/PARENT_PR143_FULL_SOURCE.zip'),'--out',str(tmpout),'--limit','50'],cwd=ROOT,env=env,check=True,stdout=subprocess.DEVNULL)
        for name in OUTPUTS:
            if sha(ROOT/'results'/name)!=sha(tmpout/name):raise AssertionError(('NUMERIC_FULL_SOURCE_REPLAY_DRIFT',name))
        print(f'ORIGINAL_BASS_50/50_PASS; FULL_MANIFEST_{len(manifest["files_sha256"])}/{len(manifest["files_sha256"])}_PASS; UNIT_TESTS_11/11; GIT_SOURCE_FREE_PASS; SOURCE_REPLAY_6/6_BYTE_EXACT; NEW_IVPS=0; PASS')
if __name__=='__main__':main()
