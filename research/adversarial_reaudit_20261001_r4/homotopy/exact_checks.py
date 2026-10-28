#!/usr/bin/env python3
"""Independent Fraction oracles; run only after preregistration commit."""
import argparse, json, math
from fractions import Fraction as F
from pathlib import Path

def mm(a,b):
    return [[sum((x*y for x,y in zip(row,col)),F(0)) for col in zip(*b)] for row in a]
def add(a,b):return [[x+y for x,y in zip(r,s)] for r,s in zip(a,b)]
def eye(n):return [[F(i==j) for j in range(n)] for i in range(n)]
def adaptive_doubling(h):
    n=len(h)
    assert n>0 and all(len(r)==n for r in h)
    assert all(h[i][j]==0 for i in range(n) for j in range(i,n))
    power=h; total=eye(n); levels=(n-1).bit_length()
    for level in range(levels):
        total=add(total,mm(power,total))
        if level+1<levels:power=mm(power,power)
    return total,levels

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--native',required=True);ap.add_argument('--output',required=True);args=ap.parse_args()
    rows=[json.loads(x) for x in Path(args.native).read_text().splitlines() if x]
    checks=[]
    for r in rows:
        if r['case']=='witness-integrity':
            if r['status']=='rejected':checks.append({'test':'witness:'+r['mode'],'rejected':True,'sound':True});continue
            exact=F(1,16)/(1+F(1,16)*F(r['gamma']))
            bound=F(r['certificate']['stage_bound'][0][0])
            checks.append({'test':'witness:'+r['mode'],'exact_error_fraction':str(exact),'bound':float(bound),'sound':bound>=exact})
        else:
            s=r['s']; c=r['doubling']['certificate']
            checks.append({'test':'depth:'+str(s),'s':s,'serial_sound':all(F(x[0])>=i+1 for i,x in enumerate(r['serial']['stage_bound'])),'doubling_sound':c is None or all(F(x[0])>=i+1 for i,x in enumerate(c['stage_bound'])),'doubling_output_bound':None if c is None else c['output_bound'][0],'exact_output_distance':s})
    candidate=[]
    for n in [1,2,4,8,9,16]:
        h=[[F(i==j+1) for j in range(n)] for i in range(n)]
        total,levels=adaptive_doubling(h)
        e=[sum(r,F(0)) for r in total]
        inverse_check=mm([[F(i==j)-h[i][j] for j in range(n)] for i in range(n)],total)==eye(n)
        candidate.append({'s':n,'levels':levels,'last_bound':int(e[-1]),'exact':all(e[i]==i+1 for i in range(n)) and inverse_check})
    component_rows=[]
    for n in [1,2,4]:
        st=8; m=st*n
        h=[[F((i%n)+1,16*(i//n-j//n+1)) if i%n==j%n and j//n<i//n else F(0) for j in range(m)] for i in range(m)]
        a=[F(i//n+i%n+1,128) for i in range(m)]
        full,_=adaptive_doubling(h)
        full_e=[sum((v*w for v,w in zip(row,a)),F(0)) for row in full]
        block_e=[F(0)]*m
        for u in range(n):
            hu=[[h[i*n+u][j*n+u] for j in range(st)] for i in range(st)]
            su,_=adaptive_doubling(hu)
            for i in range(st):block_e[i*n+u]=sum((su[i][j]*a[j*n+u] for j in range(st)),F(0))
        serial=[]
        for i in range(m):serial.append(a[i]+sum((h[i][j]*serial[j] for j in range(i)),F(0)))
        component_rows.append({'s':st,'n':n,'full_equals_component_equals_serial':full_e==block_e==serial,'full_stored_matrix_values':m*m,'component_stored_matrix_values':n*st*st,'dense_matmul_multiply_count_full':m**3,'dense_matmul_multiply_count_component':n*st**3,'claim':'algebraic storage and dense-loop work counts only'})
    result={'schema_version':1,'source_commit':'1c54194123ee6abc6daa512e8574922f510b4e2c','native_rows':len(rows),'native_checks':checks,'adaptive_doubling_candidate':candidate,'candidate_all_exact':all(x['exact'] for x in candidate),'component_factorization':component_rows,'component_all_exact':all(x['full_equals_component_equals_serial'] for x in component_rows),'claim_ceiling':'exact arithmetic structural candidate; no production integration, no speed measurement'}
    Path(args.output).write_text(json.dumps(result,indent=2,allow_nan=False)+'\n')
    print(json.dumps({'native_rows':len(rows),'witness_unsound':sum(x.get('sound') is False for x in checks),'doubling_unsound':sum(x.get('doubling_sound') is False for x in checks),'candidate_all_exact':result['candidate_all_exact']}))
if __name__=='__main__':main()
