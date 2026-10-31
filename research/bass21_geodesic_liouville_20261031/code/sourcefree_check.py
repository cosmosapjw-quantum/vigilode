#!/usr/bin/env python3
"""Check 12 truly original BASS21 published endpoint rows without confidential source binaries."""
import csv,hashlib,json,math
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
F=ROOT/'results/GIT_SOURCE_12_REAL_GEODESIC_ENDPOINTS.csv'
DIGEST='3c9f330fc3424499461c56dd77d90771643fffd49639513d46387b45e5c8c6e1'
def run():
    assert hashlib.sha256(F.read_bytes()).hexdigest()==DIGEST, 'ORIGINAL_WITNESS_HASH_DRIFT'
    with F.open() as f:rows=list(csv.DictReader(f))
    assert len(rows)==12, 'NOT_12_ORIGINAL_ROWS'
    for z in rows:
        u=float(z['mu_anchor']); db=float(z['b_delta'])
        r=math.sqrt((1-u*u)*math.exp(2*db)+u*u*math.exp(-4*db))
        mu=u*math.exp(-2*db)/r
        assert abs(r-float(z['R_terminal']))<1e-12, 'NULL_ENERGY_CONSERVATION_FAIL'
        assert abs(mu-float(z['mu_terminal']))<1e-12, 'NULL_DIRECTION_CONSERVATION_FAIL'
        D=float(z['local_D']); go=float(z['local_g_obs']); g=float(z['normal_g'])
        assert D>0 and g>=0 and abs(D*go-g)<2e-14, 'THOMSON_OPTICAL_SCALAR_FAIL'
        assert abs(float(z['Q_original_terminal'])-float(z['Q_survival_terminal'])-float(z['Q_source_minus_survival']))<1e-14
    assert {z['case'] for z in rows}=={'s00_m00_r00','s04_m00_r00'}
    s=json.loads((ROOT/'results/GIT_SCIENCE_SUMMARY.json').read_text())
    assert s['original_ray_rows']==164050 and s['local_observer_rows']==820250 and s['counterfactual_source_rows']==82450
    assert s['local_E_shear_values']==106150 and s['new_ivps']==0
    assert s['DAG']['vigilode']=='7/23' and s['production']=='HOLD'
    assert s['max_opacity_scalar_identity_gap']<2e-14 and s['max_projection_vs_parent_shear']<1e-10
    print('ORIGINAL_SOURCEFREE_WITNESS_PASS 12/12; FULL_OWNER_SOURCE_REPLAY_NOT_RUN_IN_GITHUB_CI')
if __name__=='__main__':run()
