#!/usr/bin/env python3
"""Strict source-free committed evidence checks. Not a full source or global CI claim."""
import csv,hashlib,json,pathlib
from fractions import Fraction
ROOT=pathlib.Path(__file__).resolve().parents[1]
SRC='6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8'
PARENT='d33ff58f3f9fc8dec0e9cc3b055108f005f309af3209a688a2251ef7e3649a16'

def check():
 m=json.loads((ROOT/'GIT_MANIFEST.json').read_text())
 for p,h in m['files_sha256'].items():
  actual=hashlib.sha256((ROOT/p).read_bytes()).hexdigest()
  assert actual==h,('GIT_MIRROR_BYTE_MISMATCH',p)
 s=json.loads((ROOT/'results/SCIENCE_SUMMARY.json').read_text())
 a=json.loads((ROOT/'results/EXACT_GENERATOR.json').read_text())
 assert s['source_sha256']==SRC and s['canonical_parent_sha256']==PARENT
 assert s['original_cases']==50 and s['saved_epochs']==193
 assert s['new_local_generator_values']==50*193*12==115800
 assert sum(s['new_high_modes'].values())==50*193*5==48250
 assert len(s['source_case_hashes_50'])==50
 assert s['new_ivps']==0 and s['dag']['official_delta']==0 and s['production']=='HOLD'
 assert a['A_J'][0][1]=='-2' and a['A_J'][1][0]=='-8/5'
 assert a['A_J'][5][6]=='1188/161' and a['A_c'][5][6]=='880/87'
 assert a['A_c'][0][0]==a['A_c'][0][1]=='10/7'
 for key in ('A_J','A_c'):
  mat=[[Fraction(q) for q in row] for row in a[key]]
  assert all(len(row)==len(mat) for row in mat)
  assert all(mat[i][j]==0 for i in range(len(mat)) for j in range(len(mat)) if abs(i-j)>1)
 with open(ROOT/'results/ENDPOINT_50_NEW_JC.csv') as f: rows=list(csv.DictReader(f))
 with open(ROOT/'results/PAIRED_25_NEW_JC.csv') as f: pair=list(csv.DictReader(f))
 with open(ROOT/'results/AUDIT_50_JC.csv') as f: audit=list(csv.DictReader(f))
 assert len(rows)==len(audit)==50 and len(pair)==25
 assert len({x['source_sha256'] for x in rows})==50
 assert {x['case'] for x in rows}==set(s['source_case_hashes_50'])
 assert all(s['source_case_hashes_50'][x['case']]==x['source_sha256'] for x in rows)
 pair_mean=sum(float(x['delta_c12']) for x in pair)/25
 assert abs(pair_mean-s['final_paired_mean_delta_c12'])<1e-19
 assert max(abs(float(x['spin2_c12'])) for x in rows)<=s['max_c12']+1e-20
 assert s['max_angular_GL64_96_c_gap']<1e-16
 assert s['max_parent_deltaJ_replay_gap']<1e-16
 assert s['max_parent_c_replay_gap']<1e-16
 print('SCOPED_GIT_MANIFEST_%s/%s; MATRIX_EXACT_RATIONAL; ORIGINAL_CASES_50; PAIRED_25; DAG_7/23; PASS'%(len(m['files_sha256']),len(m['files_sha256'])))

if __name__=='__main__': check()
