#!/usr/bin/env python3
"""Export the R3 homotopy certificate fixtures (candidate bits, exact-root
brackets and exact candidate-to-root distances) from certified_majorant.py
on the strict-lower-projection target. Merge into
fixtures/r3_homotopy_certificate_fixtures.json with the recorded decisions of
research/adversarial_reaudit_20261001_r3/homotopy/results.json."""
import importlib.util,json,struct,math,sys
from fractions import Fraction as F
import numpy as np
import pathlib
base=str(pathlib.Path(__file__).resolve().parents[1])+'/research/adversarial_reaudit_20261001_r3/homotopy/'
spec=importlib.util.spec_from_file_location('cm',base+'certified_majorant.py');cm=importlib.util.module_from_spec(spec);spec.loader.exec_module(cm)
raw=json.load(open(base+'native_tableau_bits.json'))
dec=lambda x:[dec(v) for v in x] if isinstance(x,list) else struct.unpack('>d',bytes.fromhex(x))[0]
c={k:dec(raw[k]) for k in ('gamma','alpha','l','b','btilde')}
for k in ('alpha','l'): c[k]=np.tril(c[k],-1).tolist()
hx=lambda v: struct.pack('>d',float(v)).hex()
def enc(x):
    lo=float(x); lo = lo if F(lo)<=x else math.nextafter(lo,-math.inf)
    hi=float(x); hi = hi if F(hi)>=x else math.nextafter(hi,math.inf)
    return [hx(lo),hx(hi)]
rec=json.load(open(base+'results.json'))['rows']
out=[];idx=0
for fam,J,y,q in [('affine',[[-1.]],[1.],[0.]),('nonnormal',[[-1.,100.],[0.,-1.]],[0.,1.],[0.,0.]),('quadratic',[[-1.]],[1.],[.25]),('nonnormal_quadratic',[[-1.,10.],[0.,-1.]],[.01,.01],[.05,.05])]:
  for h in [.1,1.,10.]:
    p=cm.Problem(c,J,y,h,q);ref=p.exact_root()
    for m,k,d in p.candidates():
      cert=p.certify(k,1e-8,1e-6); r=rec[idx]; idx+=1
      assert r['family']==fam and r['method']==m and r['certificate']['E']==cert['E']
      kf=[[F(float(v)) for v in row] for row in k]
      b=[F(float(v)) for v in p.b]; bt=[F(float(v)) for v in p.bt]
      exactout=[p.yf[a]+cm.dot(b,[x[a] for x in ref]) for a in range(p.n)]
      exactemb=[cm.dot(bt,[x[a] for x in ref]) for a in range(p.n)]
      out.append(dict(family=fam,h=h,method=m,candidate=[[hx(v) for v in row] for row in k],exact_root=[[enc(v) for v in row] for row in ref],
        stage_distance=[[enc(abs(kf[i][a]-ref[i][a])) for a in range(p.n)] for i in range(p.s)],
        output_distance=[enc(abs(F(float(cert['yhat'][a]))-exactout[a])) for a in range(p.n)],
        embedded_distance=[enc(abs(F(float(cert['ehat'][a]))-exactemb[a])) for a in range(p.n)],
        yhat=[hx(v) for v in cert['yhat']],ehat=[hx(v) for v in cert['ehat']]))
json.dump(out,open(sys.argv[1],'w'))
print(len(out),'fixtures ok')
