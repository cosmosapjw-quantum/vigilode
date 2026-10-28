#!/usr/bin/env python3
"""Independent reviewer fixture/root; candidate certificate evaluator imported."""
from pathlib import Path
from fractions import Fraction as F
import importlib.util,json,struct
import numpy as np
out=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('cert',out.parent/'homotopy/certified_majorant.py');mod=importlib.util.module_from_spec(spec);spec.loader.exec_module(mod)
def dec(v):
    if isinstance(v,list):return [dec(x) for x in v]
    if isinstance(v,str):return struct.unpack('>d',bytes.fromhex(v))[0]
    return v
raw=json.loads((out.parent/'homotopy/native_tableau_bits.json').read_text()); c={k:dec(raw[k]) for k in ['gamma','alpha','l','b','btilde']}
for key in ['alpha','l']:c[key]=[[v if j<i else 0. for j,v in enumerate(row)] for i,row in enumerate(c[key])]
J=[[-2.,3.],[0.,-.5]];y=[.125,-.0625];h=.25;q=[.25,.125]
p=mod.Problem(c,J,y,h,q)
# Root recurrence independently authored here; no exact_root, matvec, dot or
# inverse_small from the prototype is used by this oracle.
gamma=F(c['gamma']); hf=F(h);jf=[[F(v) for v in row] for row in J];yf=list(map(F,y));qf=list(map(F,q))
a=1-hf*gamma*jf[0][0];b=-hf*gamma*jf[0][1];d=1-hf*gamma*jf[1][1]
w=[[a,b],[F(0),d]]; wi=[[1/a,-b/(a*d)],[F(0),1/d]]
g=[hf*(sum(jf[v][k]*yf[k] for k in range(2))-qf[v]*yf[v]**2) for v in range(2)]
root=[]
for i in range(8):
    delta=[sum(F(c['alpha'][i][j])*root[j][v] for j in range(i)) for v in range(2)]
    lc=[sum(F(c['l'][i][j])*root[j][v] for j in range(i)) for v in range(2)]
    rhs=[g[v]+hf*(sum(jf[v][k]*lc[k] for k in range(2))+qf[v]*delta[v]**2) for v in range(2)]
    root.append([sum(wi[v][k]*rhs[k] for k in range(2)) for v in range(2)])
rows=[]
for label,k,depth in p.candidates():
    cert=p.certify(k,1e-8,1e-6);dc=p.doubling_certificate(k)
    stage=all(abs(F(float(k[i,v]))-root[i][v])<=F(cert['E'][i][v]) for i in range(8) for v in range(2))
    output=[yf[v]+sum(F(c['b'][j])*root[j][v] for j in range(8)) for v in range(2)]
    emb=[sum(F(c['btilde'][j])*root[j][v] for j in range(8)) for v in range(2)]
    outok=all(abs(F(float(cert['yhat'][v]))-output[v])<=F(cert['output_bound'][v]) for v in range(2))
    embok=all(abs(F(float(cert['ehat'][v]))-emb[v])<=F(cert['embedded_difference_bound'][v]) for v in range(2))
    dcok=all(abs(F(float(k[i,v]))-root[i][v])<=F(dc['E'][i][v]) for i in range(8) for v in range(2)) if dc['closes'] else None
    assert stage and outok and embok and (not dc['closes'] or dcok)
    rows.append({'candidate':label,'stage_bound':stage,'output_bound':outok,'embedded_bound':embok,'doubling_radius_closes':dc['closes'],'doubling_stage_bound':dcok,'output_wrms_upper':cert['output_wrms']})
result={'status':'PASS_WITHIN_PROJECTED_TARGET_SCOPE','reference':'Reviewer-selected 2D quadratic dyadic fixture; separately authored exact rational triangular-root recurrence and inverse','source_coefficients':'native binary64 bits; alpha/l explicitly projected strict lower, not exact native full-block target','J':J,'y':y,'h':h,'q':q,'rows':rows,'claim_ceiling':['Strict-lower projected quadratic target only','Exact native full-block and sequential arithmetic parity not established','ODE truncation/global error not certified','Python serial certificate is no wall-speed measurement']}
(out/'homotopy_independent_check.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
