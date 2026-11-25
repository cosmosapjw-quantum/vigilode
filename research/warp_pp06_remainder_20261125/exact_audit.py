"""Independent exact-binary rational complex polynomial convolution audit."""
import json,struct
from fractions import Fraction as F
from pathlib import Path
root=Path(__file__).resolve().parent
Z=(F(0),F(0))
def f(x):return F.from_float(struct.unpack(">d",bytes.fromhex(x))[0])
def add(a,b):return (a[0]+b[0],a[1]+b[1])
def mul(a,b):return (a[0]*b[0]-a[1]*b[1],a[0]*b[1]+a[1]*b[0])
def parse(p):return {(k,j):(f(re),f(im)) for k,j,re,im in p}
def plus(a,b):
    z=dict(a)
    for key,v in b.items():z[key]=add(z.get(key,Z),v)
    return z
def product(a,b):
    z={}
    for (k,j),x in a.items():
        for (l,n),y in b.items():
            key=(k+l,j+n)
            z[key]=add(z.get(key,Z),mul(x,y))
    return z
def scale(a,z):return {key:mul(v,z) for key,v in a.items()}
def conj(a):return {(-k,j):(re,-im) for (k,j),(re,im) in a.items()}
def l1(a):return abs(a[0])+abs(a[1])
def compare(a,b):
    dif=max((l1(add(a.get(k,Z),mul(b.get(k,Z),(-1,0)))) for k in a.keys()|b.keys()),default=F(0))
    ref=max(map(l1,b.values()),default=F(0))
    assert dif<=F(1,10**12)*ref if ref else dif==0
    return float(dif/ref) if ref else 0.
rows=[]
for line in (root/"regression.log").read_text().splitlines():
    if "EXACT_CASE " not in line:continue
    c=json.JSONDecoder().raw_decode(line.split("EXACT_CASE ",1)[1])[0]
    a,b,p=map(parse,(c["a"],c["b"],c["phase"]))
    pa=product(p,a)
    coeff=plus({(0,0):(f(c["g"]),F(0))},scale(plus(pa,conj(pa)),(f(c["eps"])/2,F(0))))
    hq=f(c["h"])*f(c["q"])
    ref=[scale(product(product(coeff,conj(a)),b),(F(0),hq)),scale(product(product(coeff,a),a),(F(0),hq/2))]
    e=[compare(parse(x),r) for x,r in zip(c["out"],ref)]
    rows.append({"mode":c["mode"],"input_sizes":[len(a),len(b),len(p)],"max_relative_l1_coefficient_error":max(e)})
assert len(rows)==10,len(rows)
res={"schema":"warp.pp06.exact.v1","count":len(rows),"max_relative_l1_coefficient_error":max(r["max_relative_l1_coefficient_error"] for r in rows),"rows":rows}
(root/"EXACT_AUDIT.json").write_text(json.dumps(res,indent=2,sort_keys=True)+"\n")
print("EXACT_AUDIT",len(rows),res["max_relative_l1_coefficient_error"])
