"""Exact source-bound inclusion of a declared REAL tube. No model evaluation."""
from fractions import Fraction as F
from pathlib import Path
import argparse, hashlib, json, math, sys
ROOT=Path(__file__).resolve().parents[1]
BINDING='8cb268e8de741e284cf494013f1e0748d27f03c5e77ba0c64413ae92af082d16'
CLIENT_SHA='327514dc3a036690ad584f9461375ec1cb034f5cf5b282294e3afcb7e6df7a13'
class Refusal(ValueError):
    def __init__(self,code): self.code=code; super().__init__(code)
def rational(x): return {'numerator':str(x.numerator),'denominator':str(x.denominator)}
def source_binding():
    b=json.loads((ROOT/'inputs/source_binding.json').read_text())
    if b['binding_id']!=BINDING or b['source_commit']!='cc602e209e7deeb511ab2a1f3b2268d55206c43c' or b['reference_sha256']['client_binding.py']!=CLIENT_SHA: raise Refusal('SOURCE_IDENTITY_MISMATCH')
    pinned={'client_binding.py':CLIENT_SHA,'affine_system.py':'e4427768a3b5030036eb10dd83c876a23317afa963ce549d333e88c1e11f07c7','CLIENT_IDENTITY.json':'ec5355ac4fdce2f5faa4280aea98120efe0f627c0f16690dbf70d2965940d97d'}
    if b['reference_sha256']!=pinned: raise Refusal('SOURCE_IDENTITY_MISMATCH')
    for n,h in pinned.items():
        if hashlib.sha256((ROOT/'inputs/reference'/n).read_bytes()).hexdigest()!=h: raise Refusal('SOURCE_HASH_MISMATCH')
    ident=json.loads((ROOT/'inputs/reference/CLIENT_IDENTITY.json').read_text())
    expected=['0x1.fffffffffffffp-1','0x1.0000000000000p+0','0x1.0000000000000p+0']
    if b['totals_hex']!=expected or ident['conserved_totals_hex']!=expected or float(ident['config']['x_end']).hex()!=b['x_end_hex'] or b['x_end_hex']!=float(1.9459101490553132).hex() or ident['binding_id']!=BINDING: raise Refusal('SOURCE_IDENTITY_MISMATCH')
    return b
def rows(b):
    out=[]
    def add(name,c,terms,strict=False): out.append((name,c,terms,strict))
    add('time_lower',F(0),{0:1}); add('time_upper',F.from_float(float.fromhex(b['x_end_hex'])),{0:-1})
    add('chart_lower',F(1),{2:1});add('chart_upper',F(1),{2:-1})
    for i in (0,29): add('nonnegative_z'+str(i),F(0),{i+1:1})
    for i in (3,8,13,18): add('strict_monopole_z'+str(i),F(0),{i+1:1},True)
    for i in range(23,29): add('charged_z'+str(i),F(0),{i+1:1})
    for name,t,block in zip(('H','He','Li'),b['totals_hex'],((23,),(24,25),(26,27,28))): add('neutral_'+name,F.from_float(float.fromhex(t)),{i+1:-1 for i in block})
    return out
def parse(inp):
    if not isinstance(inp,dict): raise Refusal('MALFORMED_INPUT')
    if inp.get('binding_id')!=BINDING: raise Refusal('SOURCE_IDENTITY_MISMATCH')
    arrays=[]
    for key in ('u','v','r'):
        a=inp.get(key)
        if not isinstance(a,list) or len(a)!=31: raise Refusal('INVALID_STATE_SHAPE')
        vals=[]
        for x in a:
            if isinstance(x,bool) or not isinstance(x,(str,float,int)): raise Refusal('MALFORMED_BINARY64')
            try:
                if isinstance(x,str):
                    if not x.lower().startswith(('0x','-0x','+0x')): raise ValueError()
                    v=float.fromhex(x)
                else: v=float(x)
            except (ValueError,OverflowError): raise Refusal('MALFORMED_BINARY64')
            if not math.isfinite(v): raise Refusal('NONFINITE_INPUT')
            vals.append(F.from_float(v))
        arrays.append(vals)
    u,v,r=arrays
    if any(x<0 for x in r): raise Refusal('NEGATIVE_RADIUS')
    maximum=F.from_float(sys.float_info.max)
    if any(max(abs(a),abs(b))+rr>maximum for a,b,rr in zip(u,v,r)): raise Refusal('FINITE_RANGE_ENVELOPE_EXCEEDED')
    return u,v,r
def certify(inp):
    b=source_binding(); u,v,r=parse(inp); result=[]
    for name,c,a,strict in rows(b):
        au=sum((coef*u[i] for i,coef in a.items()),F()); av=sum((coef*v[i] for i,coef in a.items()),F()); radius=sum((abs(coef)*r[i] for i,coef in a.items()),F())
        lo=c+min(au,av)-radius; hi=c+max(au,av)+radius; ok=lo>0 if strict else lo>=0
        endpoint=0 if au<=av else 1; w=list(u if endpoint==0 else v)
        for i,coef in a.items(): w[i]-=(1 if coef>0 else -1)*r[i]
        result.append({'row':name,'strict':strict,'minimum':rational(lo),'maximum':rational(hi),'included':ok,'minimizing_witness':{'endpoint':endpoint,'w':[rational(q) for q in w]}})
    included=all(x['included'] for x in result)
    return {'schema':'vigilode.v8.affine-tube-domain-certificate.v1','binding_id':BINDING,'input_sha256':hashlib.sha256(json.dumps(inp,sort_keys=True,separators=(',',':')).encode()).hexdigest(),'source_binding':b,'domain_included':included,'real_declared_tube':True,'rows':result,'represented_lift_claim':{'conditional_nonnegative_finite':included,'assumptions':['query binary64 shape (30,) lies in specified REAL tube','fixed pinned AffineSystem.lift source and totals','IEEE binary64 round-to-nearest ties-to-even, gradual underflow','numpy sum of one/two/three nonnegative entries uses binary additions of the fixed dtype'],'proof_scope':'all possible first pairs for three charged entries; exact simplex required'},'query_rounding_bound':None,'certifies_rounded_tube_construction':False,'certifies_ode_tube_enclosure':False,'residual_bound':None,'jacobian_bound':None,'lognorm_bound':None,'error_bound':None,'source_bound':None,'initial_bound':None,'whole_domain_certificate':False,'provider_constructed':False,'physical_evaluations':0}
def obligation(cert):
    return {'schema':'vigilode.v8.domain-inclusion-obligation.v1','node':'V8','binding_id':BINDING,'domain_inclusion_satisfied':cert['domain_included'],'certificate_input_sha256':cert['input_sha256'],'authority':'DECLARED_REAL_TUBE_DOMAIN_INCLUSION_ONLY','whole_domain_certificate':False,'verified_model_provider':None,'query_rounding_bound':None,'parent_status':'OPEN','global_completion':'27/88_UNCHANGED','remaining_obligations':['query rounding/membership bound','ODE trajectory enclosure','residual bound','Jacobian/lognorm bound','error/source/initial bounds','verified model provider admission']}
def main():
    ap=argparse.ArgumentParser();ap.add_argument('input');ap.add_argument('--output');ap.add_argument('--obligation');a=ap.parse_args()
    try: out=certify(json.loads(Path(a.input).read_text())); status=0
    except Refusal as e: out={'schema':'vigilode.v8.typed-refusal.v1','refusal':e.code};status=2
    except (json.JSONDecodeError,OSError) as e: out={'schema':'vigilode.v8.typed-refusal.v1','refusal':'INPUT_READ_ERROR','detail':str(e)};status=2
    text=json.dumps(out,indent=2)+'\n'
    if a.output: Path(a.output).write_text(text)
    else: print(text,end='')
    if a.obligation and status==0: Path(a.obligation).write_text(json.dumps(obligation(out),indent=2)+'\n')
    return status
if __name__=='__main__':sys.exit(main())
