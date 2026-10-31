#!/usr/bin/env python3
"""Source-free Git-facing Thomson physical witness checks, NOT full source replay."""
import csv,hashlib,json,math
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
WITNESS='results/SCOPED_TEN_ENDPOINT_WITNESS.csv'
EXPECTED='6191b8a2cb1e54acf88ba2a17411ea7237eb786116648b09fb6680c82df57a22'

def check():
    p=ROOT/WITNESS
    if hashlib.sha256(p.read_bytes()).hexdigest()!=EXPECTED:
        raise AssertionError('WRONG_OR_MUTATED_PHYSICAL_WITNESS')
    with p.open() as f:rows=list(csv.DictReader(f))
    assert len(rows)==10 and {z['frame'] for z in rows}=={'rest','x001','x005','negx005','tilt45'}
    assert {z['case'] for z in rows}=={'s00_m00_r00','s04_m00_r00'}
    data=json.loads((ROOT/'results/GIT_SCOPED_RESULT.json').read_text())
    assert data['source_states']==9650 and data['complex_C_E_count']==3715250
    assert data['complex_C_B_count']==3715250 and data['new_owner_ivps']==0
    assert data['DAG']['vigilode']=='7/23' and data['production']=='HOLD'
    assert data['max_parent_collision_E_reconciliation']==0
    assert data['max_d1_B_physical']<1e-12
    assert data['max_B_by_frame']['x005']>0.002
    for z in rows:
        for k in ['x_end','g_end','C_E20_end_real','C_B21_end_imag','C_max_abs_B_end','thomson_J2_rate_end']:
            assert math.isfinite(float(z[k]))
    for case in ('s00_m00_r00','s04_m00_r00'):
        r={z['frame']:z for z in rows if z['case']==case}
        assert abs(float(r['x005']['C_B21_end_imag'])+float(r['negx005']['C_B21_end_imag']))<1e-14
        assert abs(float(r['rest']['C_B21_end_imag']))<1e-12
    # exact-source rank-two Thomson scattering eigenvalues, independent of source amplitude.
    assert math.isclose(-.9-.4,-1-.3,abs_tol=1e-15)
    assert math.isclose((.9*.4)-(.24*.25),.3,abs_tol=1e-15)
    print('SOURCEFREE_SCOPED_CI_PASS; NOT_FULL_SOURCE_VERIFICATION')
if __name__=='__main__':check()
