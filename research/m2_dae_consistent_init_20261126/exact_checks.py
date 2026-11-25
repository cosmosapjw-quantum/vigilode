"""Finite exact algebraic subcontracts, not a general DAE theorem."""
from pathlib import Path
import json
from mathharness_general.poly_exact import parse_polynomial
from mathharness_general.ledger import Ledger,fresh_verify
P=Path(__file__).resolve().parent;f=P/'exact_ledger.json';assert not f.exists()
l=Ledger(f);checks=[]
def check(cid,lhs,rhs):
    l.add_claim(cid,lhs+' = '+rhs,source={'scope':'finite algebraic DAE initialization identity'},formal_statement=f'POLY_IDENTITY: {parse_polynomial(lhs)} = {parse_polynomial(rhs)}')
    r=l.record_certificate(cid,'identity',{'lhs':lhs,'rhs':rhs});assert r['status']=='PROVED_EXACT_CERTIFICATE'
    checks.append({'id':cid,'status':r['status']})
check('SCHUR-CLEAR','d*(a*x+b*z)-b*(c*x+d*z)','(a*d-b*c)*x')
check('ADJUGATE-11','a*d+b*(-c)','a*d-b*c')
check('ADJUGATE-12','a*(-b)+b*a','0')
check('ADJUGATE-21','c*d+d*(-c)','0')
check('ADJUGATE-22','c*(-b)+d*a','a*d-b*c')
check('ANALYTIC-TANGENT','(1-2*x^2)-2*x*(-x)-1','0')
check('CUBIC-INJECTIVE','(z^3+z)-(w^3+w)','(z-w)*(z^2+z*w+w^2+1)')
check('QUADRATIC-AMBIGUITY','z^2-1','(z-1)*(z+1)')
v=fresh_verify(f,strict=True,timeout=60);assert v.get('ok'),v
(P/'EXACT_CHECKS.json').write_text(json.dumps({'claims':checks,'fresh_verifier':v,'non_claim':'not an interval enclosure or full DAE solution'},indent=2)+'\n')
print('Exact finite claims',len(checks),'fresh verifier PASS')
