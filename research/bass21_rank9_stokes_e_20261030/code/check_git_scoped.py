"""Minimal Python stdlib Git text/CSV scope receipt; NOT full source ZIP replay."""
import csv,hashlib,json,pathlib
from fractions import Fraction as F
ROOT=pathlib.Path(__file__).resolve().parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def check():
    payload=json.loads((ROOT/'GIT_MANIFEST.json').read_text())['files_sha256']
    for name,expected in payload.items():
        path=ROOT/name
        assert path.is_file() and sha(path)==expected,('GIT_BYTE_HASH_DIFFERENCE',name)
    a=json.loads((ROOT/'results/SCIENCE_SUMMARY.json').read_text())
    assert a['cases']==50 and a['epochs']==193 and a['new_ivps']==0
    assert a['dag']=={'vigilode':'7/23','rei_central':'28/88','rei_working':'33/88','bass_ce1':'9/15','successor_admitted':'0/12','official_delta':0}
    assert a['new_E_l0_values']==77200 and a['new_coherency_scalar_values']==28950
    assert a['new_stokes_gain_samples']==96500 and a['new_rank9_collisional_rate_values']==19300
    for name,n in [('COHERENCY_ENDPOINT_50.csv',50),('COHERENCY_MATCHED_25.csv',25),('COHERENCY_AUDIT_50.csv',50)]:
        with (ROOT/'results'/name).open(newline='') as f: assert len(list(csv.DictReader(f)))==n,name
    # Exact-rational frozen source tensor-to-Stokes algebra, not a CI global authority claim.
    j0,j2,b=F(7,5),F(2,13),F(1,11)
    mp=j0/F(3)+j2/F(6)-b/F(8)
    mz=(j0-j2)/F(3)+b/F(4)
    assert 2*mp+mz==j0
    for mu in (F(0),F(1,2),F(1)):
        si=F(3,2)*(mp*(1+mu*mu)+mz*(1-mu*mu))
        sq=F(3,2)*(mz-mp)*(1-mu*mu)
        assert si==j0+(j2/F(2)-F(3,8)*b)*(3*mu*mu-1)/2
        assert sq==(-F(3,4)*j2+F(9,16)*b)*(1-mu*mu)
    print('SOURCE_FREE_GIT_HASH_%d/%d; ORIGINAL_50x193_COUNTS; EXACT_RATIONAL_GAIN; SCIENCE_SCOPE; PASS'%(len(payload),len(payload)))
if __name__=='__main__':check()
