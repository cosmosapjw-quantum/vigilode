"""Independent rational audit of the binary64 residual and error enclosures.
The input numbers are round-tripped binary64, not intended decimal reals.
No Rust matrix, residual, or LU routine is invoked by this script.
"""
import json
from fractions import Fraction as F
from pathlib import Path
root = Path(__file__).resolve().parent
def frac(x):
    return F.from_float(float(x))
def solve(a, b):
    m = len(b)
    r = [list(row)+[v] for row,v in zip(a,b)]
    for j in range(m):
        i = next(i for i in range(j,m) if r[i][j])
        r[i],r[j]=r[j],r[i]
        t=r[j][j]
        r[j]=[v/t for v in r[j]]
        for i in range(m):
            if i!=j:
                t=r[i][j]
                if t:r[i]=[v-t*w for v,w in zip(r[i],r[j])]
    return [row[-1] for row in r]
def norm(v,n):
    return sum(max(map(abs,v[j*n:(j+1)*n])) for j in range(3))
def matvec(w,x):
    return [sum(v*t for v,t in zip(row,x)) for row in w]
def apply(w,d,p,x,n):
    xs=[x[j*n:(j+1)*n] for j in range(3)]
    result=[matvec(w,v) for v in xs]+[[F(0)]*n]
    for j in range(3):
        dx=matvec(d,xs[j])
        result[j+1]=[a+b for a,b in zip(result[j+1],dx)]
    # Long polynomial reduction, not the folded candidate recurrence.
    for j in range(3):
        result[j]=[a-p[j]*b for a,b in zip(result[j],result[3])]
    return sum(result[:3],[])
rows=[]
decoder=json.JSONDecoder()
for line in (root/"exact_binary.log").read_text().splitlines():
    if "EXACT_CASE " not in line: continue
    case,_=decoder.raw_decode(line.split("EXACT_CASE ",1)[1])
    n=case["n"]
    w=[list(map(frac,case["w"][i*n:(i+1)*n])) for i in range(n)]
    d=[list(map(frac,case["d"][i*n:(i+1)*n])) for i in range(n)]
    p=list(map(frac,case["p"]))
    b=list(map(frac,sum(case["b"],[])))
    x=list(map(frac,sum(case["x"],[])))
    m=3*n
    cols=[apply(w,d,p,[F(int(i==j)) for i in range(m)],n) for j in range(m)]
    A=list(map(list,zip(*cols)))
    r=[a-bb for a,bb in zip(b,apply(w,d,p,x,n))]
    true_r=norm(r,n)
    exact=solve(A,b)
    true_e=norm([a-bb for a,bb in zip(exact,x)],n)
    rb,eb=frac(case["residual_upper"]),frac(case["error_upper"])
    assert true_r<=rb, ("residual underbound",len(rows),true_r,rb)
    assert true_e<=eb, ("forward underbound",len(rows),true_e,eb)
    assert (case["status"]=="Certified") == (eb<=frac(1e-10))
    rows.append({"n":n,"status":case["status"],"exact_residual_approx":float(true_r),"exact_error_approx":float(true_e),
                 "residual_upper":float(rb),"error_upper":float(eb)})
assert len(rows)==19,len(rows)
assert sum(r["status"]=="Certified" for r in rows)==10
res={"schema":"warp.exact-binary-audit.v1","count":len(rows),"underbounds":0,"rows":rows}
(root/"EXACT_AUDIT.json").write_text(json.dumps(res,indent=2,sort_keys=True)+"\n")
print("EXACT_RATIONAL_AUDIT",len(rows),"cases; zero residual/forward underbounds")
