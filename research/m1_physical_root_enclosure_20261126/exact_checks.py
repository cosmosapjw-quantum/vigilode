"""Exact finite Bernstein/clock identities with unmodified MathHarness."""
from pathlib import Path
from fractions import Fraction as F
from math import comb
import json
from mathharness_general.poly_exact import parse_polynomial
from mathharness_general.ledger import Ledger,fresh_verify
P=Path(__file__).resolve().parent
ledger=P/'exact_ledger.json'
assert not ledger.exists()
l=Ledger(ledger)
checks=[]
def claim(name,lhs,rhs):
    l.add_claim(name,lhs+' = '+rhs,source={'scope':'finite exact normalized residual / restriction identity, not IVP enclosure'},formal_statement=f'POLY_IDENTITY: {parse_polynomial(lhs)} = {parse_polynomial(rhs)}')
    r=l.record_certificate(name,'identity',{'lhs':lhs,'rhs':rhs})
    assert r['status']=='PROVED_EXACT_CERTIFICATE',r
    checks.append({'name':name,'status':r['status']})
def B(n,k,v):return f'{comb(n,k)}*({v})^{k}*(1-({v}))^{n-k}'
for n in range(5):
    claim('PARTITION-'+str(n),'+'.join(B(n,k,'x') for k in range(n+1)),'1')
for k in range(5):
    # B_{k,4}(theta*u) = sum_{i=k}^4 B_{k,i}(theta) B_{i,4}(u)
    claim('PREFIX-'+str(k),B(4,k,'theta*u'),'+'.join(f'({B(i,k,"theta")})*({B(4,i,"u")})' for i in range(k,5)))
claim('PRODUCT-RULE-2','d1*f2+f1*d2','f2*d1+f1*d2')
claim('REPEATED-FACTOR','(x-r)^2','x^2-2*x*r+r^2')
v=fresh_verify(ledger,strict=True,timeout=60);assert v.get('ok'),v
(P/'EXACT_CHECKS.json').write_text(json.dumps({'checks':checks,'fresh_verifier':v,'non_claim':'no physical IVP theorem proven by finite identity ledger'},indent=2)+'\n')
print('Exact identities',len(checks),'fresh verifier PASS')
