#!/usr/bin/env python3
"""Source-free Git mirror verifier. NOT full original-owner source/IVP replay."""
from __future__ import annotations
import csv,hashlib,json,math,pathlib
ROOT=pathlib.Path(__file__).resolve().parents[1]

def sha(path):
    with open(path,'rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def load(path):
    with open(ROOT/path,newline='',encoding='utf8') as f:return list(csv.DictReader(f))

def run():
    man=json.loads((ROOT/'GIT_MANIFEST.json').read_text())
    for path,expected in man['files_sha256'].items():
        p=ROOT/path
        if not p.is_file() or sha(p)!=expected:raise AssertionError(('GIT_FILE_BYTES_DRIFT',path))
    rows=load('results/EIGENMODE_CASE50_ENDPOINT.csv')
    pairs=load('results/EIGENMODE_PAIR25_ENDPOINT.csv')
    times=load('results/EIGENMODE_TIME193_MEANS.csv')
    assert len(rows)==50 and len(pairs)==25 and len(times)==193
    assert len(set(r['case'] for r in rows))==50
    maxep={'U':0,'V':0,'M':0,'J':0}
    for row in rows:
        g=float(row['g_final']);u=float(row['U_final']);v=float(row['V_final'])
        j=float(row['J2_final']);b=float(row['BQ_final']);m=float(row['M_anisotropy_final'])
        cu=float(row['C_U_final']);cv=float(row['C_V_final'])
        assert all(map(math.isfinite,[g,u,v,j,b,m,cu,cv])) and g>=0
        maxep['U']=max(maxep['U'],abs(cu+g*u))
        maxep['V']=max(maxep['V'],abs(cv+.3*g*v))
        maxep['M']=max(maxep['M'],abs(m+math.pi*v/2))
        maxep['J']=max(maxep['J'],abs(u-(8*j+b)),abs(v-(4*j-3*b)))
    if max(maxep.values())>=1e-10:raise AssertionError(('ORIGINAL_SCIENCE_ENDPOINT_FAILED',maxep))
    for row in pairs:
        u=float(row['delta_U_final']);v=float(row['delta_V_final'])
        pred_u=sum(float(row[k]) for k in ['delta_P_U_final','delta_H_U_final','delta_model_U_defect_final'])
        pred_v=sum(float(row[k]) for k in ['delta_P_V_final','delta_H_V_final','delta_model_V_defect_final'])
        if max(abs(u-pred_u),abs(v-pred_v))>1e-11:raise AssertionError(('PAIR_INITIAL_SOURCE_MISSING',row['pair']))
    s=json.loads((ROOT/'results/SCIENCE_SUMMARY.json').read_text())
    assert s['source_cases']==50 and s['saved_epochs']==193 and s['new_ivps']==0
    assert s['dag']['vigilode']=='7/23' and s['dag']['official_delta']==0
    assert s['production']=='HOLD' and s['dynamical_native_E_l']=='NOT_RUN'
    assert math.isclose(sum(float(row['U_final']) for row in rows)/50,s['mean_U_final'],rel_tol=0,abs_tol=1e-14)
    assert math.isclose(sum(float(row['V_final']) for row in rows)/50,s['mean_V_final'],rel_tol=0,abs_tol=1e-14)
    stages=json.loads((ROOT/'checkpoints/STAGE_STATUS.json').read_text())
    assert len(stages['stages'])==20 and stages['scope_18_of_20']=='18/20'
    assert stages['distinct_author_R6_C9']=='NOT_RUN'
    print(f'SCOPED_GIT_SOURCE_FREE_PASS: {len(man["files_sha256"])} file SHA, 50 original-owner endpoint rows, 25 paired source decompositions, 193 saved times; original ZIP full replay NOT_RUN_IN_GIT_CI')
    print('max endpoint eigen/tensor residual',maxep)

if __name__=='__main__':run()
