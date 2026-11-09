#!/usr/bin/env python3
"""Compact CPU-only reproduction for VigilODE Peer-W PW07; research/mock only."""
import json, math
from pathlib import Path
import numpy as np
import sympy as sp
r3=math.sqrt(3.0)
def c2c(s):
 A=np.array([[(8-9*s)/28,9*(s-4)/28],[-5*(8+5*s)/252,5*(36+5*s)/252]])
 G=np.array([[24-54*s,54*(s-2)],[24+30*s,-108-30*s]])/175
 return A,G,np.array([-1,5/9]),np.array([1/28,27/28]),np.zeros(2),12/25
def c2b(s):
 g=(3-r3)/2
 A=np.array([[3*(5-3*r3)*s,24-16*r3+3*(-5+3*r3)*s],[-(1+r3)*s,8+(1+r3)*s]])/8
 G=np.array([[3*(r3-2)*s,-6+2*r3+6*s-3*r3*s],[r3*s,-6+2*r3-r3*s]])/4
 return A,G,np.array([3-2*r3,1]),np.array([0,g]),(r3-1)/8*np.array([s,4-s]),g
def scalar(m,N,var=False):
 shift=2 if m=="PTSW2C" else 2*r3-2; fn=c2c if m=="PTSW2C" else c2b
 if var:
  raw=[1.]
  for k in range(1,N): raw.append(raw[-1]*(.8 if k%2 else 1.25))
  h0=1/(shift+sum(raw)); hs=np.array(raw)*h0
 else: h0=1/(N+shift); hs=np.full(N,h0)
 t=shift*h0; u=math.exp(-t)
 kp=np.array([-1,-math.exp(-14*h0/9)]) if m=="PTSW2C" else np.array([-1,-math.exp(-shift*h0)])
 hp=h0
 for h in hs:
  s=h/hp; A,G,c,b,v,g=fn(s); W=1-h*g*(-1); Y=u+h*A@kp
  kn=(-Y-h*(G@kp))/W; u=float(u+h*(b@kn+v@kp)); kp=kn; hp=h; t+=h
 return abs(u-math.exp(-1))
def exact():
 s=sp.symbols("s",positive=True); rt=sp.sqrt(3)
 def sh(c,l): return [sp.Integer(1)]*len(c) if l==1 else [(x-1)**(l-1) for x in c]
 def pc(c,l): return [sp.Integer(1)]*len(c) if l==1 else [x**(l-1) for x in c]
 def chk(c,b,v,g,A,G,q,p):
  C=[];H=[];B=[]
  for l in range(1,q+1):
   C += [sp.factor(sum(A[i][j]*sh(c,l)[j] for j in range(2))-s**(l-1)*c[i]**l/l) for i in range(2)]
   H += [sp.factor(sum(G[i][j]*sh(c,l)[j] for j in range(2))+g*s**(l-1)*pc(c,l)[i]) for i in range(2)]
  for l in range(1,p+1): B.append(sp.factor(sum(b[j]*s**(l-1)*pc(c,l)[j]+v[j]*sh(c,l)[j] for j in range(2))-s**(l-1)/l))
  return list(map(str,C)),list(map(str,H)),list(map(str,B))
 c=[-1,sp.Rational(5,9)]; b=[sp.Rational(1,28),sp.Rational(27,28)]; v=[0,0]; g=sp.Rational(12,25)
 A=[[(8-9*s)/28,9*(s-4)/28],[-5*(8+5*s)/252,5*(36+5*s)/252]]
 G=[[(24-54*s)/175,54*(s-2)/175],[(24+30*s)/175,(-108-30*s)/175]]
 c2=chk(c,b,v,g,A,G,2,3)
 g=(3-rt)/2;c=[3-2*rt,sp.Integer(1)];b=[0,g];v=[(rt-1)*s/8,(rt-1)*(4-s)/8]
 A=[[3*(5-3*rt)*s/8,(24-16*rt+3*(-5+3*rt)*s)/8],[-(1+rt)*s/8,(8+(1+rt)*s)/8]]
 G=[[3*(rt-2)*s/4,(-6+2*rt+6*s-3*rt*s)/4],[rt*s/4,(-6+2*rt-rt*s)/4]]
 c2b_=chk(c,b,v,g,A,G,2,2)
 M=sp.Matrix([[rt/4,3*rt/4,-1-rt/3],[-rt/12,-rt/4,-1-rt/3],[0,0,0]])
 return {"PTSW2C_C2_Gamma2_B3":c2,"PTSW2B_C2_Gamma2_B2":c2b_,"PTSW2B_charpoly":str(sp.factor(M.charpoly(sp.Symbol("z")).as_expr())),"M_inf2_nonzero":bool(M**2!=sp.zeros(3)),"M_inf3_zero":bool(M**3==sp.zeros(3))}
def main():
 fixed={m:[scalar(m,n) for n in (20,40,80,160,320,640)] for m in ("PTSW2C","PTSW2B")}
 variable={m:[scalar(m,n,True) for n in (40,80,160,320,640)] for m in ("PTSW2C","PTSW2B")}
 slope=lambda a:[math.log(a[i]/a[i+1],2) for i in range(len(a)-1)]
 out={"exact_symbolic":exact(),"fixed_observed_orders":{k:slope(v) for k,v in fixed.items()},"variable_observed_orders":{k:slope(v) for k,v in variable.items()},"fixed_abs_errors":fixed,"variable_abs_errors":variable,"limitations":["oracle startup; startup error/cost excluded","smooth scalar smoke test only","no Rust/full-IVP/production qualification"]}
 p=Path(__file__).with_name("peerw_pw07_compact_results.json"); p.write_text(json.dumps(out,indent=2,sort_keys=True)+"\n"); print(p.read_text())
if __name__=="__main__": main()
