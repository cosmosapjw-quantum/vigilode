#!/usr/bin/env python3
"""Independent exact-input oracle; native output is input, no SciPy/native phi reuse."""
from pathlib import Path
from fractions import Fraction as F
from decimal import Decimal as D, localcontext
import json, struct, math
HERE=Path(__file__).resolve().parent

def fbits(s): return struct.unpack('>d',bytes.fromhex(s))[0]
def exact(s): return F.from_float(fbits(s))
def dec(x): return D(x.numerator)/D(x.denominator)
def upper(v): return F.from_float(v['mantissa'])*F(2)**v['exponent']
def phi(z,k):
    if z==0: return D(1)/D(math.factorial(k))
    x=z.exp()
    for j in range(1,k+1): x=(x-D(1)/D(math.factorial(j-1)))/z
    return x

def main():
    rows=[json.loads(line) for line in (HERE/'native.jsonl').read_text().splitlines()]
    output={'source_commit':'1c54194123ee6abc6daa512e8574922f510b4e2c','oracle':'Fraction exact binary64 inputs; Decimal 140-digit exp and algebraic spectral projectors','transform':[],'polynomial':[],'norm_candidate':[],'factorial_candidate':[],'claim_ceiling':'Bounded toy corpus; no speed or production promotion'}
    byid={r.get('id'):r for r in rows}
    for r in rows:
        if r['kind']=='transform' and 'result' in r and r['result']['status']=='bounded':
            h=exact(r['h_bits'][0]); b=[[exact(x) for x in v] for v in r['b_bits']];s=[[exact(x) for x in v] for v in r['stored_bits']];n=len(b[0]);a=[exact(x) for x in r['a_bits']]
            error=[F(0)]*n
            for k,(v,w) in enumerate(zip(b,s)):
                delta=[h**k*x-y for x,y in zip(v,w)]
                for i in range(n): error[i]+=delta[i]/math.factorial(k)+h*sum(a[i*n+j]*delta[j] for j in range(n))/math.factorial(k+1)
            u=upper(r['result']['upper']);sq=sum(x*x for x in error)
            with localcontext() as ctx:
                ctx.prec=140
                e=dec(sq).sqrt(); ud=dec(u)
                output['transform'].append({'id':r['id'],'error_norm':str(e),'bound':str(ud),'encloses_exact_error':u*u>=sq,'admits_zero':r['admits_zero'],'admits_negative':r['admits_negative'],'exact_error_fraction':[str(x) for x in error]})
        elif r['kind']=='polynomial':
            if 'error' in r: output['polynomial'].append({'id':r['id'],'status':'SAFE_REJECTION','error':r['error']});continue
            a=[[exact(x) for x in v] for v in r['matrix_bits']];h=exact(r['h_bits'][0]);w=[[exact(x) for x in v] for v in r['w_bits']];report=r['report']; cert=report['total_error'];
            with localcontext() as ctx:
                ctx.prec=140
                cols=[]
                for k in range(5):
                    if a[0][1]==0: col=[phi(dec(h*a[i][i]),k)*dec(w[k][i]) for i in range(2)]
                    else:
                        # A=[[-2,1],[1,-2]], exact projectors P+=(I+swap)/2 and P-=(I-swap)/2.
                        low=phi(dec(-h),k)*(dec(w[k][0])+dec(w[k][1]))/2
                        high=phi(dec(-3*h),k)*(dec(w[k][0])-dec(w[k][1]))/2
                        col=[low+high,low-high]
                    cols.append(col)
                true=[sum(col[i] for col in cols) for i in range(2)]
                err=sum((D.from_float(report['fused'][i])-true[i])**2 for i in range(2)).sqrt()
                bound=D.from_float(cert.get('bound',cert.get('bounded_components',0.)))
                output['polynomial'].append({'id':r['id'],'error_norm':str(err),'status':cert['status'],'bound':str(bound),'bound_encloses':err<=bound if cert['status']=='certified' else None,'within_1e_10':err<=D('1e-10'),'degree':report['degree'],'truncation_bound':report['truncation_bound_exact_arithmetic']})
        elif r['kind']=='norm_candidate':
            sq=sum(exact(x)**2 for x in r['input_bits']);u=upper(r['bound']);output['norm_candidate'].append({'input_bits':r['input_bits'],'bound':r['bound'],'encloses_exact_norm':u*u>=sq})
        elif r['kind']=='factorial_candidate':
            p=r['order'];source=byid['order_'+str(p)];h=exact(source['h_bits'][0]);b=exact(source['b_bits'][p][0]);stored=exact(source['stored_bits'][p][0]);error=abs(h**p*b-stored)/math.factorial(p);u=upper(r['error_bound']);c=upper(r['coefficient']);output['factorial_candidate'].append({'order':p,'coefficient_encloses':c>=F(1,math.factorial(p)),'error_encloses':u>=error,'bound':r['error_bound']})
    output['summary']={'native_rows':len(rows),'transform_false_bounds':[r['id'] for r in output['transform'] if not r['encloses_exact_error']],'polynomial_certified_false_bounds':[r['id'] for r in output['polynomial'] if r.get('bound_encloses') is False],'polynomial_accuracy_failed':[r['id'] for r in output['polynomial'] if r.get('within_1e_10') is False],'norm_candidate_all_enclose':all(r['encloses_exact_norm'] for r in output['norm_candidate']),'factorial_candidate_all_enclose':all(r['coefficient_encloses'] and r['error_encloses'] for r in output['factorial_candidate'])}
    (HERE/'ORACLE_RESULTS.json').write_text(json.dumps(output,indent=2)+'\n');print(json.dumps(output['summary'],indent=2))
if __name__=='__main__': main()
