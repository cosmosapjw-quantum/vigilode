"""Independent exact-rational audit of emitted native binary64 source data."""
from fractions import Fraction as F
from decimal import Decimal, localcontext
from itertools import product
from math import comb
from pathlib import Path
import json, struct, time

HERE=Path(__file__).resolve().parent

def fl(s): return struct.unpack(">d",bytes.fromhex(s))[0]
def f(s): return F.from_float(fl(s))
def iv(s): return tuple(map(f,s))
def dec(q): return Decimal(q.numerator)/Decimal(q.denominator)
def poly(a,x):
    y=F()
    for c in a[::-1]: y=y*x+c
    return y
def bern(a):
    n=len(a)-1
    return [sum((a[j]*F(comb(k,j),comb(n,j)) for j in range(k+1)),F()) for k in range(n+1)]

def main():
    started=time.perf_counter(); data=json.loads((HERE/"NATIVE_BITS.json").read_text())
    checks=queries=diagnostics=0; all_rows=[]; prior=None
    def check(ok,description):
        nonlocal checks
        checks+=1
        if not ok: raise AssertionError(description)
    for row in data["records"]:
        t0,t1=map(f,row["times"]);y0,y1=map(f,row["states"]);h=t1-t0
        H=[[f(v) for v in r] for r in row["dense_h"]];u=list(map(f,row["stages"]))
        d=[sum((v*w for v,w in zip(r,u)),F()) for r in H]
        a=[y0,y1-y0+d[0],d[1]-d[0],d[2]-d[1],-d[2]]
        lam=iv(row["lambda"]);b=iv(row["b"]);kappa=lam[0];e0=f(row["initial_error"])
        check(iv(row["clock_width"])[0]<=h<=iv(row["clock_width"])[1], "real clock width")
        for x,I in zip(a,map(iv,row["power"])): check(I[0]<=x<=I[1],"quartic coefficient enclosure")
        rbounds=list(map(iv,row["residual_bernstein"]));delta=f(row["defect_upper"])
        for l,bb in product(set(lam),set(b)):
            r=[l*a[j]+((j+1)*a[j+1]/h if j<4 else 0)-(bb if j==0 else 0) for j in range(5)]
            for x,I in zip(bern(r),rbounds):
                check(I[0]<=x<=I[1],"exact physical residual Bernstein coefficient")
                check(abs(x)<=delta,"whole-slab defect bound")
        pb=bern(a);P=iv(row["polynomial_range"])
        check(P[0]<=min(pb) and max(pb)<=P[1],"exact polynomial Bernstein hull")
        z=kappa*h
        expected=max((e0+h*delta)/(1+z),((1-z/2)*e0+h*delta)/(1+z/2))
        eh=f(row["endpoint_error_upper"]);whole=f(row["whole_slab_error_upper"])
        check(eh>=expected,"rational Dini endpoint upper")
        check(whole>=max(e0,eh),"whole-slab radius")
        tube=iv(row["state_tube"])
        check(tube[0]<=min(pb)-whole and max(pb)+whole<=tube[1],"state tube")
        flow=iv(row["flow_range"])
        for l,bb,y in product(set(lam),set(b),set(tube)):
            check(flow[0]<=-l*y+bb<=flow[1],"physical event derivative")
        for q in row["queries"]:
            t=f(q["time"]);x=f(q["actual_center"]);th=(t-t0)/h
            p=y0 if t==t0 else y1 if t==t1 else poly(a,th)
            I=iv(q["represented_polynomial"]);rho=f(q["evaluation_rounding_upper"]);E=f(q["error_upper"])
            check(I[0]<=p<=I[1],"exact physical query polynomial")
            check(abs(x-p)<=rho,"actual output minus exact target")
            state_error=e0 if t==t0 else eh if t==t1 else whole
            check(E>=state_error+rho,"state and rounding radii combined outward")
            Y=iv(q["state_enclosure"])
            check(Y[0]<=x-E and x+E<=Y[1],"returned state interval")
            threshold=f(row["threshold"])
            if q["symbol"]=="Positive": check(Y[0]>threshold,"strict positive")
            if q["symbol"]=="Negative": check(Y[1]<threshold,"strict negative")
            error_diagnostic=None
            if row["case"]=="native-accepted-affine":
                with localcontext() as ctx:
                    ctx.prec=90
                    exact=(-dec(t)).exp()
                    error=abs(dec(x)-exact)
                    check(dec(Y[0])<=exact<=dec(Y[1]),"Decimal90 native exact affine solution diagnostic")
                    diagnostics+=1;error_diagnostic=str(error)
            all_rows.append({"case":row["case"],"origin":row["origin"],"segment":row["segment_index"],"time":float(t),"center":float(x),"error_upper":float(E),"rounding_upper":float(rho),"analytic_error":error_diagnostic,"symbol":q["symbol"]})
            queries+=1
        if row["origin"]=="NativeAcceptedRodas5pFast":
            if prior:
                check(row["times"][0]==prior["times"][1] and row["states"][0]==prior["states"][1],"native carry adjacency")
                check(row["initial_error"]==prior["endpoint_error_upper"],"native carry preserves accumulated error bits")
                check(row["segment_index"]==prior["segment_index"]+1,"native segment index")
            prior=row
    result={"result":"PASS_SCOPED","method":"Independent fractions.Fraction from emitted binary64 hex; source expression reconstructed independently","published_code_commit":data["compile_source_commit"],"record_count":len(data["records"]),"native_steps":data["native_steps"],"queries":queries,"exact_or_diagnostic_checks":checks,"Decimal90_native_diagnostics":diagnostics,"native_coefficient_snapshot_sha256":data["native_coefficient_snapshot_sha256"],"wall_seconds":time.perf_counter()-started,"actual_BASS30_provider":False,"performance_claim":False}
    (HERE/"BIT_ORACLE_RESULTS.json").write_text(json.dumps(result,indent=2)+"\n")
    (HERE/"PLOT_DATA.json").write_text(json.dumps(all_rows,indent=2)+"\n")
    print(json.dumps(result))

if __name__=="__main__":main()
