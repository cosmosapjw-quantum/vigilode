"""Use the pinned MathHarness kernel/ledger for exact subcontracts, not solver qualification."""
import json,sys
from pathlib import Path
from fractions import Fraction as F
from mathharness_general.ledger import Ledger, fresh_verify
from mathharness_general.poly_exact import parse_polynomial
from mathharness_general.sturm import count_distinct_real_roots
from mathharness_general.matrix_exact import analyze_matrix
from mathharness_general.certificates import check_identity
repo=Path(sys.argv[1]).resolve()
out=Path(sys.argv[2]).resolve();out.mkdir(parents=True,exist_ok=True)
ledger=out/'exact_ledger.json'
assert not ledger.exists()
led=Ledger(ledger)
receipts=[]
def identity(cid,lhs,rhs,scope):
    spec=dict(lhs=lhs,rhs=rhs)
    formal=f"POLY_IDENTITY: {parse_polynomial(lhs)} = {parse_polynomial(rhs)}"
    led.add_claim(cid,f"{lhs} = {rhs}",source={"scope":scope},formal_statement=formal)
    r=led.record_certificate(cid,'identity',spec)
    assert r['status']=='PROVED_EXACT_CERTIFICATE',r
    receipts.append(dict(id=cid,scope=scope,status=r['status'],lhs=lhs,rhs=rhs))
# Scalar component of a Schur complement after clearing a nonzero denominator.
# Not a proof of nonlinear DAE existence or a claim about the solver.
identity('DAE-SCHUR-CLEARED','d*(a*x+b*z)-b*(c*x+d*z)','(a*d-b*c)*x','scalar elimination; recovering z requires d != 0')
# Affine composition associative scalar component; Rust noncommutation is
# tested separately, and the proof below uses all ordered matrix indices.
identity('AFFINE-SCALAR','a*(b*(c*x+u)+v)+w','a*b*c*x+a*b*u+a*v+w','scalar affine composition')
for i in range(2):
    # Explicitly expand matrix/vector product, not commuting symbolic matrices.
    lhs='+'.join(f'a{i}{j}*(b{j}0*(c00*x0+c01*x1+u0)+b{j}1*(c10*x0+c11*x1+u1)+v{j})' for j in range(2))+f'+w{i}'
    rhs='+'.join(f'a{i}{j}*b{j}{k}*c{k}{l}*x{l}' for j in range(2) for k in range(2) for l in range(2))+'+'+'+'.join(f'a{i}{j}*b{j}{k}*u{k}' for j in range(2) for k in range(2))+'+'+'+'.join(f'a{i}{j}*v{j}' for j in range(2))+f'+w{i}'
    identity(f'AFFINE-MAT2-{i}',lhs,rhs,'finite two-component symbolic ordered product')
data=repo/'crates/rodas5p-core/data/retro_dictionary_v1'
lag=json.loads((data/'LAGRANGE_TABLE.json').read_bytes())['records']
for n,r in sorted(lag.items()):
    expr=lambda c:'+'.join(f'({a})*t^{i}' for i,a in enumerate(c))
    identity('LAGRANGE-PARTITION-'+n,'+'.join('('+expr(c)+')' for c in r['coefficients_ascending']),'1','original authenticated rational basis partition of unity')
    for j,c in enumerate(r['coefficients_ascending']):
        for i,node in enumerate(r['nodes']):
            p='+'.join(f'({a})*({node})^{k}' for k,a in enumerate(c))
            identity(f'LAGRANGE-{n}-{j}-{i}',p,str(int(i==j)),'finite exact nodal basis value')
# Distinct roots incl repeated root (not a certificate of arbitrary callback).
roots=count_distinct_real_roots(parse_polynomial('(3*t-1)^2*(3*t-2)'),'t','0','1')
assert roots['status']=='CERTIFIED' and roots['count']==2,roots
# Independent exact matrix inverse for a prepared solve fixture.
m=[['2','1/8'],['-1/4','2']]
r=analyze_matrix(m)
assert r['rank']==2 and r['determinant']=='129/32',r
inv=[[F(v) for v in row] for row in r['inverse']]
assert all(sum(F(m[i][k])*inv[k][j] for k in range(2))==int(i==j) for i in range(2) for j in range(2))
neg=check_identity('t^2','t')
assert neg['status']=='REJECTED'
# Fresh verifier is separate process and checks the actual ledger entries.
v=fresh_verify(ledger,strict=True,timeout=60)
assert v.get('ok'),v
summary={'exact_claims':len(receipts),'claims':receipts,'sturm_finite':roots,'matrix_finite':r,
         'negative_identity':neg,'fresh_verifier':v,'trust_base':sorted(led.trust()),
         'non_claims':['No arbitrary DAE, event, IVP, or matched-error solver qualification.','All ledger statements are bounded exact algebraic subcontracts.']}
(out/'EXACT_CHECKS.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:v for k,v in summary.items() if k!='claims'},indent=2))
