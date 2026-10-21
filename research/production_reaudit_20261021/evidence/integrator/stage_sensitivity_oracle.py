#!/usr/bin/env python3
"""Exact Fraction polynomial recurrence; no NumPy/SymPy dependency.

Snapshot decimal convention and actual binary64-parsed convention are reported
separately. Rational complex squared magnitudes prove the domain counterexample.
Cayley coefficient sums supply exact-arithmetic dissipative Hilbert bounds.
"""
import json, math, hashlib
from fractions import Fraction as Q
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
SNAP = ROOT / 'fixtures/rodas5p_coefficients_snapshot.json'

def padd(a,b):
    return [(a[i] if i<len(a) else Q(0))+(b[i] if i<len(b) else Q(0)) for i in range(max(len(a),len(b)))]
def pmul(a,b):
    c=[Q(0)]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b):c[i+j]+=x*y
    return c

def add(a,b):return (a[0]+b[0],a[1]+b[1])
def mul(a,b):return (a[0]*b[0]-a[1]*b[1],a[0]*b[1]+a[1]*b[0])
def evaluate(p,w):
    result=(Q(0),Q(0))
    for c in reversed(p):result=add(mul(result,w),(c,Q(0)))
    return result

def cayley_sum(p):
    # p((1+z)/2): exact binomial expansion, then triangle inequality.
    transformed=[Q(0)]*len(p)
    for k,c in enumerate(p):
        for j in range(k+1):transformed[j]+=c*math.comb(k,j)/2**k
    return sum(map(abs,transformed)),transformed

def calc(convention):
    data=json.loads(SNAP.read_text());parse=(lambda x:Q(float(x))) if convention=='binary64_parsed' else Q
    g=parse(data['gamma']);A=[[parse(x) for x in r] for r in data['A']];C=[[parse(x) for x in r] for r in data['C']];b=list(map(parse,data['b_code']))
    p=[None]*8
    for i in reversed(range(8)):
        p[i]=[b[i]]
        for j in range(i+1,8):p[i]=padd(p[i],pmul(p[j],[-A[j][i],g*C[j][i]+A[j][i]]))
    m=[[Q(0)]+v for v in p]
    # z=i => w=(1+i gamma)/(1+gamma^2).
    w=(1/(1+g*g),g/(1+g*g))
    m1=evaluate(m[0],w);square=m1[0]**2+m1[1]**2
    limit=Q('0.58');assert square>limit*limit
    rows={}
    for name,polys in [('p_stage_error_to_output',p),('m_raw_residual_to_output',m)]:
        bounds=[cayley_sum(v)[0] for v in polys]
        raw=[sum(map(abs,v)) for v in polys]
        rows[name]={'cayley_bounds':[float(x) for x in bounds],'cayley_bounds_exact':[str(x) for x in bounds], 'sum':float(sum(bounds)),'raw_power_bounds':[float(x) for x in raw],'raw_power_sum':float(sum(raw)),'improvement_factor':float(sum(raw)/sum(bounds)), 'transformed_coefficients':[[str(x) for x in cayley_sum(v)[1]] for v in polys]}
    return {'convention':convention,'counterexample':{'z':'i','normal_matrix':[[0,-1],[1,0]],'m1_modulus_squared_exact':str(square),'m1_modulus':math.sqrt(float(square)),'stored_target':float(limit),'strict_exceedance_proved':True},'conditional_hilbert_bounds':rows}

if __name__=='__main__':
    result={'snapshot_sha256':hashlib.sha256(SNAP.read_bytes()).hexdigest(),'authority':'exact_fraction_identity_and_conditional_theorem; no floating enclosure or production promotion','results':[calc('decimal_snapshot'),calc('binary64_parsed')]}
    path=Path(__file__).with_name('stage_sensitivity_oracle.json');path.write_text(json.dumps(result,indent=2)+'\n')
    for row in result['results']:
        print(row['convention'], row['counterexample']['m1_modulus'])
        for name,b in row['conditional_hilbert_bounds'].items():print(name,b['cayley_bounds'],'sum',b['sum'],'raw_sum',b['raw_power_sum'],'improvement',b['improvement_factor'])
