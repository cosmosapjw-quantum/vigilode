#!/usr/bin/env python3
"""Independent 1600-digit Decimal exponential/complex-series oracle; stdlib only."""
from decimal import Decimal as D, localcontext
import json
from pathlib import Path
CASES = [
 ('zero',0.,0.,.125,[1.,-2.],[.5,3.]),
 ('nearzero',1e-16,-2e-16,1.,[1.,-2.],[.5,3.]),
 ('period',0.,6.283185307179586,1.,[0.,0.],[1.,2.]),
 ('damped',3.,-7.,.4,[2.,-3.],[4.,5.]),
 ('underflow',1e308,0.,1.,[0.,0.],[1e308,-1e308]),
 ('tiny_coeff',0.,1e308,1e-308,[0.,0.],[1e308,1e308]),
 ('large_product',1e308,1e308,1e308,[1e308,-1e308],[1e308,1e308]),
 ('small_generator',1e-308,-1e-308,1e308,[0.,0.],[1e-308,-1e-308]),
 ('cancellation',0.,0.,1e308,[1e308,-1e308],[-1.,1.]),
 ('big_state_decay',1000.,0.,1.,[1e308,-1e308],[0.,0.]),
]
def mul(a,b): return a[0]*b[0]-a[1]*b[1],a[0]*b[1]+a[1]*b[0]
def exp_complex(z):
 # Huge-decay fixtures have d >=1e308 and h >=1. Both the initial
 # transient and forcing-transient corrections are far below binary64.
 if z[0] < -D(10000): return D(0),D(0)
 damping=z[0].exp(); theta=z[1]; term=(D(1),D(0)); total=term
 for k in range(1,220):
  term=mul(term,(D(0),theta));term=(term[0]/k,term[1]/k)
  total=(total[0]+term[0],total[1]+term[1])
 return damping*total[0],damping*total[1]
def oracle(d,w,h,y,b):
 d,w,h=map(D.from_float,(d,w,h)); y=tuple(map(D.from_float,y));b=tuple(map(D.from_float,b))
 lam=(-d,w);e=exp_complex((h*lam[0],h*lam[1]));hom=mul(e,y)
 if lam==(0,0): forced=(h*b[0],h*b[1])
 else:
  numerator=mul((e[0]-1,e[1]),b);den=lam[0]**2+lam[1]**2
  forced=((numerator[0]*lam[0]+numerator[1]*lam[1])/den,(numerator[1]*lam[0]-numerator[0]*lam[1])/den)
 return [float(hom[0]+forced[0]),float(hom[1]+forced[1])]
if __name__=='__main__':
 with localcontext() as context:
  context.prec=1600
  rows=[dict(name=n,damping=d,frequency=w,h=h,y=y,b=b,expected=oracle(d,w,h,y,b)) for n,d,w,h,y,b in CASES]
 root=Path(__file__).resolve().parents[3]
 dest=root/'crates/rodas5p-integrators/tests/frontier_affine_oracles.json'
 dest.write_text(json.dumps(rows,indent=2)+'\n')
 print(dest)
