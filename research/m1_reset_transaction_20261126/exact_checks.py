"""Finite algebraic subcontracts; independent MathHarness kernel."""
import json
from pathlib import Path
from mathharness_general.poly_exact import parse_polynomial
from mathharness_general.ledger import Ledger,fresh_verify
P=Path(__file__).resolve().parent;f=P/'exact_ledger.json';assert not f.exists()
l=Ledger(f);checks=[]
def check(cid,lhs,rhs):
 l.add_claim(cid,lhs+' = '+rhs,source={'scope':'exact scalar affine synchronized reset identity'},formal_statement=f'POLY_IDENTITY: {parse_polynomial(lhs)} = {parse_polynomial(rhs)}')
 r=l.record_certificate(cid,'identity',{'lhs':lhs,'rhs':rhs});assert r['status']=='PROVED_EXACT_CERTIFICATE'
 checks.append({'id':cid,'status':r['status']})
check('FLOW-COMPOSITION','E2*(E1*y+(1-E1)*q)+(1-E2)*q','E1*E2*y+(1-E1*E2)*q')
check('SALTATION-SCALAR-CLEARED','m*f+v-m*f-d','v-d')
check('SYNC-DERIVATIVE-CLEARED','a^2*E*(m*q+c+d*t)+a*E*d-a*b*E','a*E*(d+a*(m*q+c+d*t)-b)')
check('RESET-CLOCK-DIFFERENCE','m*q+c+d*t-(m*q+c+d*s)','d*(t-s)')
check('RESET-ANCHOR-CANCEL','d+a*(m*q+c+d*s)-(d+a*(m*q+c+d*s))','0')
check('RESET-DERIVATIVE-DIFFERENCE','d+a*(m*q+c+d*t)-b-(d+a*(m*q+c+d*s)-b)','a*d*(t-s)')
v=fresh_verify(f,strict=True,timeout=60);assert v.get('ok'),v
(P/'EXACT_CHECKS.json').write_text(json.dumps({'checks':checks,'fresh_verifier':v,'non_claim':'algebraic identities do not prove complete hybrid IVP qualification'},indent=2)+'\n')
print('exact identities',len(checks),'fresh verifier PASS')
