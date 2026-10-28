"""Independent exact algebra and high-precision analytic diagnostics; no IVP solver."""
from fractions import Fraction as F
from decimal import Decimal, localcontext
from itertools import product
from math import comb
from pathlib import Path
import json, time

def bernstein(a):
    n=len(a)-1
    return [sum((a[j]*F(comb(k,j),comb(n,j)) for j in range(k+1)),F()) for k in range(n+1)]

def horner(a,x):
    z=F()
    for v in a[::-1]: z=z*x+v
    return z

def dec(x): return Decimal(x.numerator)/Decimal(x.denominator)

def main():
    start=time.perf_counter(); algebra=diagnostics=0; max_ratio=Decimal(0)
    for h,lam,e0,b,y0,d2 in product([F(1,8),F(1,2),F(2)],[F(1,10),F(1),F(10)],[F(0),F(1,100),F(1)],[F(-1),F(0),F(1)],[F(-1),F(1)],[F(0),F(1,16)]):
        y1=y0*(1-h+h*h/2); d0=-y0*h*h/2; d1=F()
        a=[y0,y1-y0+d0,d1-d0,d2-d1,-d2]
        r=[lam*a[j]+((j+1)*a[j+1]/h if j<4 else 0)-(b if j==0 else 0) for j in range(5)]
        rb=bernstein(r); delta=max(map(abs,rb)); z=lam*h
        eh=max((e0+h*delta)/(1+z),((1-z/2)*e0+h*delta)/(1+z/2)); whole=max(e0,eh)
        for k in range(17):
            th=F(k,16); t=h*th; p=horner(a,th)
            factored=(1-th)*y0+th*(y1+(1-th)*(d0+th*(d1+th*d2)))
            assert p==factored
            reconstructed=sum((rb[j]*comb(4,j)*th**j*(1-th)**(4-j) for j in range(5)),F())
            assert reconstructed==horner(r,th)
            assert abs(horner(r,th))<=delta
            algebra+=3
            with localcontext() as ctx:
                ctx.prec=90
                for sign in [-1,1]:
                    exact=dec(b/lam)+(dec(y0+sign*e0-b/lam))*(-dec(lam*t)).exp()
                    error=abs(dec(p)-exact)
                    bound=dec(e0 if k==0 else eh if k==16 else whole)
                    assert error<=bound+Decimal("1e-85")
                    if bound: max_ratio=max(max_ratio,error/bound)
                    diagnostics+=1
    result={"scope":"324 represented quartic/constant-affine rational cases; Decimal90 analytic diagnostics","exact_algebra_assertions":algebra,"analytic_diagnostic_samples":diagnostics,"cases":324,"max_sample_error_to_bound":str(max_ratio),"wall_seconds":time.perf_counter()-start,"IVP_calls":0,"seed":None,"proof":"THEORY.md Dini comparison + Bernstein convex hull; finite diagnostics are separate","result":"PASS_SCOPED"}
    Path(__file__).with_name("MATH_RESULTS.json").write_text(json.dumps(result,indent=2)+"\n")
    print(json.dumps(result))

if __name__=="__main__": main()
