"""Independent rational oracle for represented binary64 stage; no Rust library imports."""
from fractions import Fraction as Q
from pathlib import Path
import struct,json
base=Path(__file__).resolve().parent

def val(v):return struct.unpack('>d',int(v,16).to_bytes(8,'big'))[0]
def qs(txt):return [Q.from_float(val(t)) for t in txt.split(',')]
passed=0
fail=[]
for lineno,line in enumerate((base/'oracle.txt').read_text().splitlines(),1):
    nstr,st,jt,bt,wt,xt,rt,rs=line.split('|')
    n=int(nstr);sigma=Q.from_float(val(st));J=qs(jt);b=qs(bt);w=qs(wt);x=qs(xt)
    assert len(J)==n*n and len(b)==n and len(w)==n and len(x)==n
    exact_rho=max(abs(b[i]-x[i]+sigma*sum((J[i*n+k]*x[k] for k in range(n)),Q(0)))/w[i] for i in range(n))
    exact_mu=max(J[i*n+i]+sum((abs(J[i*n+k])*w[k]/w[i] for k in range(n) if k!=i),Q(0)) for i in range(n))
    denom=1-sigma*exact_mu
    assert denom>0
    exact_k=1/denom
    exact_stage=exact_k*exact_rho
    for mode,raw in [('dense',rt),('sparse',rs)]:
        rho,mu,K,stage=(Q.from_float(val(z)) for z in raw.split(','))
        for key,ok in [('rho_upper',rho>=exact_rho),('mu_upper',mu>=exact_mu),
                       ('K_upper',K>=exact_k),('stage_error_upper',stage>=exact_stage)]:
            passed+=1
            if not ok:fail.append((lineno,mode,key,str(Q.from_float(val(raw.split(',')[0]))),str(exact_rho)))
result={'schema':'vigilode.sg01.fraction_oracle.v1','records':lineno,'comparisons':passed,
        'failures':fail[:20], 'pass':not fail,
        'authority':'Python Fraction.from_float over exact represented binary64 J,sigma,b,w,x; independent of Rust interval algorithm'}
(base/'fraction_oracle.json').write_text(json.dumps(result,indent=2)+'\n')
print(result)
if fail:raise SystemExit(1)
