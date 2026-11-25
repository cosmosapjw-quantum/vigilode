"""Independent exact polynomial/clock and multiplicity audit of Rust output."""
import json,sys,math,functools
from pathlib import Path
from fractions import Fraction as F
import sympy as s
from mathharness_general.poly_exact import parse_polynomial
from mathharness_general.sturm import count_distinct_real_roots
P=Path(__file__).resolve().parent;x=s.Symbol('x')
TOL=F(1,2**40)
def dump(f,x): (P/f).write_text(json.dumps(x,indent=2,sort_keys=True,allow_nan=False)+'\n')
def read(f,tag):return [json.loads(l.split(tag,1)[1]) for l in (P/f).read_text().splitlines() if tag+'{' in l]
def rat(v):return s.Rational(str(v))
def f64(v):return F.from_float(float(v))
def sf(x):return F(str(x))
@functools.lru_cache(maxsize=4096)
def poly(c):
    p=s.Poly.from_list([rat(v) for v in c[::-1]],x,domain=s.QQ)
    assert not p.is_zero
    return p,p.sqf_part(),p.sqf_list()[1]
def count(p,a,b):
    if a==b:return int(p.eval(a)==0)
    # SymPy count_roots is closed; convert to (a,b].
    return int(p.count_roots(a,b))-int(p.eval(a)==0)
def audit(c,clock,r):
    p,q,factors=poly(tuple(c));n=count(q,s.Rational(0),s.Rational(1))
    roots=r['roots'];assert len(roots)==n,(c,len(roots),n)
    previous=None
    for z in roots:
        a,b=map(rat,z['exact_theta']);assert 0<=a<=b<=1 and b>0
        assert previous is None or a>=previous
        previous=b
        assert count(q,a,b)==1
        mult=sum(m for v,m in factors if count(v,a,b)==1)
        assert z['multiplicity']==mult
        if mult%2==0:kind='Touch'
        else:
            v=p;value=v.eval(b)
            while value==0:v=v.diff();value=v.eval(b)
            kind='Up' if value>0 else 'Down'
        assert z['kind']==kind
        assert f64(z['theta'][0])<=sf(a)<=sf(b)<=f64(z['theta'][1])
        assert f64(z['theta'][1])-f64(z['theta'][0])<=TOL
        t,h=map(F,clock);assert h>0
        assert f64(z['time'][0])<=t+h*sf(a)<=t+h*sf(b)<=f64(z['time'][1])
    assert r['max_theta_width']<=float(TOL)
    w=r['work']
    return dict(roots=n,variation_calls=w['variation_calls'],screened=w['screened_intervals'],dictionary_uses=w['dictionary_uses'],native_bernstein_uses=w['native_bernstein_uses'],conditioned_horner_uses=w['conditioned_horner_uses'],companion_attempts=w['companion_attempts'],companion_hints=w['companion_hints'])
def audit_trace(trace):
    counts=[]
    for slab in trace['slabs']:
        assert slab['coefficients'] is not None
        assert F(slab['clock'][0])==f64(slab['times'][0])
        assert F(slab['clock'][1])==f64(slab['times'][1])-f64(slab['times'][0])
        counts.append(audit(slab['coefficients'],slab['clock'],slab['report']))
    assert len(counts)==trace['inspected_steps']
    return {k:sum(c[k] for c in counts) for k in ['roots','variation_calls','screened','dictionary_uses','native_bernstein_uses','conditioned_horner_uses','companion_attempts','companion_hints']}|{'polynomials':len(counts)}
def main(which):
    if which=='contracts':
        c=read('contracts.log','EXACT ');n=read('contracts.log','NATIVE ')
        stats=[audit(r['coefficients'],r['clock'],r['report']) for r in c]
        native=[audit_trace(r['trace']) for r in n]
        # MathHarness exact Sturm for hidden pair and repeated factor.
        # Its API intentionally refuses endpoint-root cases; SymPy covers these.
        checks=[]
        for r in c:
            if r['name'] not in ['hidden','touch','triple','positive']:continue
            expr='+'.join(f'({v})*x^{i}' for i,v in enumerate(r['coefficients']))
            a=count_distinct_real_roots(parse_polynomial(expr),'x','0','1')
            assert a['status']=='CERTIFIED' and a['count']==len(r['report']['roots']),a
            checks.append({'name':r['name'],'route':r['report']['route'],'count':a['count']})
        dump('CONTRACT_AUDIT.json',{'kernel_reports':len(stats),'native_polynomials':sum(v['polynomials'] for v in native),'checked_roots':sum(v['roots'] for v in stats+native),'mathharness_sturm_checks':checks,'sympy':s.__version__,'status':'exact finite replay'})
        print('Contract audit',len(stats),'kernel reports,',sum(v['polynomials'] for v in native),'native polynomials,',len(checks),'MathHarness Sturm crosschecks')
        return
    name='training' if which=='training' else 'holdout'
    rows=read(name+'.log','CELL ')
    assert len(rows)==(72 if name=='training' else 6),len(rows)
    assert len({(r['name'],r['candidate']) for r in rows})==len(rows)
    summaries=[]
    for r in rows:
        v={k:r[k] for k in ['kind','name','group','candidate','median_ns']}
        v['audit_pass']=False;v['reason']=r.get('reason')
        counts={'roots':0,'variation_calls':0,'screened':0,'dictionary_uses':0,'native_bernstein_uses':0,'conditioned_horner_uses':0,'companion_attempts':0,'companion_hints':0,'polynomials':0}
        try:
            if not r['ok']:raise ValueError(r.get('reason') or r.get('status'))
            if r['kind']=='kernel':
                counts.update(audit(r['coefficients'],r['clock'],r['report']));counts['polynomials']=1
            else:
                counts=audit_trace(r['trace'])
                assert len(r['events'])==len(r['expected'])
                e=max(abs(a['t']-b) for a,b in zip(r['events'],r['expected']))
                assert e<2e-7
                v.update(max_physical_time_error=e,rhs=r['rhs'],accepted=r['accepted'])
                assert r['trace']['inspected_steps']==r['accepted']
                assert r['trace']['committed_end']<=r['trace']['numerical_end']
                assert all(q['t']<=r['trace']['committed_end'] for q in r['events'])
            v['audit_pass']=True
        except (AssertionError,ValueError) as e:v['reason']=repr(e)
        v.update(counts);summaries.append(v)
    result={'rows':summaries,'cases':len(rows),'pass':sum(r['audit_pass'] for r in summaries),'polynomial_reports':sum(r['polynomials'] for r in summaries),'roots_checked':sum(r['roots'] for r in summaries),'sympy':s.__version__,'scope':'exact represented roots only; analytic physical time checks separate'}
    dump(name.upper()+'_AUDIT.json',result);print(name,result['pass'],'/',len(rows),'polynomial reports',result['polynomial_reports'],'roots',result['roots_checked'])
    for r in summaries:
        if not r['audit_pass']:print('FAIL',r['name'],r['candidate'],r['reason'])
if __name__=='__main__':main(sys.argv[1])
