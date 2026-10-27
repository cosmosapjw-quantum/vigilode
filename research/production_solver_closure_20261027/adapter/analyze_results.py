#!/usr/bin/env python3
"""Independent high-precision exact states and native-counter reduction."""
import hashlib,json,pathlib
from decimal import Decimal as D, getcontext, localcontext
from fractions import Fraction as F
getcontext().prec=90

def atan_series(x):
    value=D(0); power=x; n=0
    while True:
        term=power/(2*n+1)
        value+=term if n%2==0 else -term
        if abs(term)<D('1e-108'):return value
        power*=x*x;n+=1
with localcontext() as ctx:
    ctx.prec=115
    PI=16*atan_series(D(1)/5)-4*atan_series(D(1)/239)

def sin_cos(x):
    with localcontext() as ctx:
        ctx.prec=105
        x=x%(2*PI)
        if x>PI:x-=2*PI
        s=st=x;c=ct=D(1);n=0
        while True:
            n+=1;st*=-x*x/((2*n)*(2*n+1));ct*=-x*x/((2*n-1)*(2*n))
            s+=st;c+=ct
            if max(abs(st),abs(ct))<D('1e-100'):return s,c
class mp:
    mpf=D
    exp=staticmethod(lambda x:x.exp())
    log=staticmethod(lambda x:x.ln())
    sqrt=staticmethod(lambda x:x.sqrt())
    cos=staticmethod(lambda x:sin_cos(x)[1])
    sin=staticmethod(lambda x:sin_cos(x)[0])
    cosh=staticmethod(lambda x:(D(x).exp()+(-D(x)).exp())/2)
    tanh=staticmethod(lambda x:((2*x).exp()-1)/((2*x).exp()+1))
    nstr=staticmethod(lambda x,n:format(x,f'.{n}g'))
root=pathlib.Path(__file__).parent
files=[root/'raw'/'pass0_results.json',root/'raw'/'pass1_results.json']
out={'schema_version':1,'precision_decimal_digits':90,'oracle':'standard-library Decimal, Machin pi and convergent Taylor sin/cos; numerical high precision, not an interval certificate','rows':[],'input_hashes':{},'seed_checks':[]}

def exact_carrier(case,t):
    if case.endswith('cubic'):return mp.mpf(0)
    s=(1+mp.tanh(200*(t-mp.mpf('0.5'))))/2 if case=='relaxing-nonlinear' else mp.mpf(1)
    return mp.cos(t)+mp.mpf(0.01)*s*mp.sin(150*t)
def integrated_rate(case,t):
    if case=='easy-cubic':return t
    if case=='stiff-cubic':return 1000*t
    lc=lambda x:mp.log(mp.cosh(x))
    if case=='relaxing-nonlinear':return mp.mpf('5000.5')*t-mp.mpf('4999.5')/200*(lc(200*(t-mp.mpf('0.5')))-lc(-100))
    return 10000*t+mp.mpf('4999.5')/200*(-lc(200*(t-mp.mpf('0.25')))+lc(-50)+lc(200*(t-mp.mpf('0.75')))-lc(-150))

for file in files:
    out['input_hashes'][str(file)]=hashlib.sha256(file.read_bytes()).hexdigest()
    doc=json.loads(file.read_bytes())
    for row in doc['rows']:
        case=row['case']; err=mp.mpf(0); points=0
        initial=row['states'][0]
        for time,state in zip(row['times'],row['states']):
            t=mp.mpf(time); lam=integrated_rate(case,t); decay=mp.exp(-lam); carrier=exact_carrier(case,t)
            for i,value in enumerate(state):
                if i==0: exact=t
                else:
                    e0=mp.mpf(initial[i])-exact_carrier(case,mp.mpf(0))
                    exact=carrier+e0*decay/mp.sqrt(1+e0*e0*(1-decay*decay)/2)
                err=max(err,abs(mp.mpf(value)-exact));points+=1
        assert err<=mp.mpf(row['accuracy_cap']) and row['accounting'] and row['success']
        assert sum(s['rhs_calls'] for s in row['segments'])==row['rhs']
        assert sum(s['jvp_vectors'] for s in row['segments'])==row['jvp']
        summary={k:row[k] for k in ['case','policy','pass','work','rhs','jvp','attempts','switches','max_error','accuracy_cap','accounting']}
        summary.update({'independent_error_decimal90':mp.nstr(err,40),'checked_state_components':points,'independent_accuracy_pass':True})
        out['rows'].append(summary)
        if row['config'].get('reuse_observed_entry_step',False):
            observed=None
            for index,s in enumerate(row['segments']):
                if s['method']=='dopri5' and s['is_probe'] and s['committed']:
                    observed=None
                if s['method']!='rodas5p' or not s['decision'].startswith('same-checkpoint implicit pilot'):continue
                explicit=row['segments'][index+1]
                assert explicit['method']=='dopri5' and explicit['start_time']==s['start_time']
                expected=min(max(doc['options']['h0'],explicit['initial_step'],observed or doc['options']['h0']),s['target_time']-s['start_time'])
                assert expected==s['initial_step'],(case,row['policy'],expected,s['initial_step'])
                out['seed_checks'].append({'case':case,'policy':row['policy'],'time':s['start_time'],'expected':expected,'actual':s['initial_step'],'pass':True})
                if s['accepted_steps']:
                    observed=(s['reached_time']-s['start_time'])/s['accepted_steps']

# Sparse Laurent polynomials in e,L,Lp,gp,gpp,v0,vj, with exact coefficients.
zero=(0,)*7
def monomial(variable,exponent=1):
    powers=list(zero);powers[variable]=exponent;return {tuple(powers):F(1)}
def add(*terms):
    result={}
    for term in terms:
        for powers,coefficient in term.items():result[powers]=result.get(powers,F(0))+coefficient
    return {p:c for p,c in result.items() if c}
def scale(term,c):return {p:v*c for p,v in term.items() if v*c}
def mul(a,b):
    result={}
    for p,c in a.items():
        for q,d in b.items():
            r=tuple(x+y for x,y in zip(p,q));result[r]=result.get(r,F(0))+c*d
    return {p:c for p,c in result.items() if c}
def derivative(term,var):
    result={}
    for p,c in term.items():
        if p[var]:q=list(p);q[var]-=1;result[tuple(q)]=c*p[var]
    return result
e,L,Lp,gp,gpp,v0,vj=[monomial(i) for i in range(7)]
one={zero:F(1)};e2=mul(e,e);e3=mul(e2,e)
phi=add(e,scale(e3,F(1,2)));slope=add(one,scale(e2,F(3,2)))
flow=scale(mul(L,phi),F(-1))
inverse=add(mul(scale(monomial(0,-3),F(-2)),flow),scale(mul(L,monomial(0,-2)),F(-2)),scale(L,F(-1)))
f=add(flow,gp)
time_part=add(mul(derivative(f,0),scale(gp,F(-1))),mul(derivative(f,1),Lp),mul(derivative(f,3),gpp))
automatic=add(mul(time_part,v0),mul(derivative(f,0),vj))
manual=add(mul(add(scale(mul(Lp,phi),F(-1)),mul(mul(L,slope),gp),gpp),v0),scale(mul(mul(L,slope),vj),F(-1)))
out['symbolic_inverse_square_identity']='0' if not inverse else str(inverse)
out['symbolic_jvp_identity']='0' if not add(automatic,scale(manual,F(-1))) else 'failed'
out['symbolic_method']='independent exact Fraction Laurent polynomial arithmetic and chain rule'
assert out['symbolic_inverse_square_identity']==out['symbolic_jvp_identity']=='0'
out['all_arms_pass']=len(out['rows'])==36 and all(r['independent_accuracy_pass'] for r in out['rows'])
out['total_checked_state_components']=sum(r['checked_state_components'] for r in out['rows'])
out['seed_checks_pass']=len(out['seed_checks'])
(root/'RESULTS.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps({'arms':len(out['rows']),'all_pass':out['all_arms_pass'],'state_components':out['total_checked_state_components'],'seed_checks':out['seed_checks_pass']},indent=2))
