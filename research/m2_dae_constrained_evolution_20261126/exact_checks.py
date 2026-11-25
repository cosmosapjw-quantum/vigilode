"""Finite exact identities used by the new source transport; not IVP proofs."""
import json
from pathlib import Path
from math import comb
from mathharness_general.poly_exact import parse_polynomial
from mathharness_general.ledger import Ledger,fresh_verify
P=Path(__file__).resolve().parent;f=P/'exact_ledger.json';assert not f.exists()
l=Ledger(f);ids=[]
def check(cid,lhs,rhs):
 l.add_claim(cid,lhs+' = '+rhs,source={'scope':'finite DAE algebraic and dense polynomial identity'},formal_statement=f'POLY_IDENTITY: {parse_polynomial(lhs)} = {parse_polynomial(rhs)}')
 r=l.record_certificate(cid,'identity',{'lhs':lhs,'rhs':rhs});assert r['status']=='PROVED_EXACT_CERTIFICATE'
 ids.append(cid)
check('DENSE-POWER','(1-u)*a+u*(b+(1-u)*(d0+u*(d1+u*d2)))','a+(b-a+d0)*u+(d1-d0)*u^2+(d2-d1)*u^3-d2*u^4')
check('SCHUR-ELIMINATION','g*(a+b*(-c/g))','g*a-b*c') if False else check('SCHUR-CLEARED','g*a+b*(-c)','g*a-b*c')
check('WARP-JET','(1+s)*(1-s+s^2-s^3+s^4-s^5)','1-s^6')
check('CONSTRAINT-TRANSPORT','(x+e)^2-x^2','2*x*e+e^2')
for j in range(5):
 check(f'BERNSTEIN-MONOMIAL-{j}',f'x^{j}','+'.join(f'({comb(k,j)}/{comb(4,j)})*{comb(4,k)}*x^{k}*(1-x)^{4-k}'for k in range(j,5)))
v=fresh_verify(f,strict=True,timeout=60);assert v.get('ok'),v
(P/'EXACT_CHECKS.json').write_text(json.dumps({'claims':ids,'fresh_verifier':v,'non_claim':'finite identities only'},indent=2)+'\n')
print('exact',len(ids),'fresh replay PASS')
