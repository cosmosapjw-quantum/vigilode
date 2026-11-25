"""Independent physical guard, velocity and reduced-gradient oracle."""
from pathlib import Path
import json,hashlib,collections,mpmath as mp
P=Path(__file__).parent;mp.mp.dps=85
def m(x):return mp.mpf(float(x))
def width(v):return m(v[1])-m(v[0])
def inside(v,x):return m(v[0])<=x<=m(v[1])
def root(case):
    if case=='algebraic':return mp.mpf(1)/2
    if case=='coupled-cancellation':return mp.mpf(7)/16
    if case=='nonlinear':return mp.mpf(3)/8
    if case=='pencil':
        a=mp.mpf(17)/32;b=mp.mpf(75)/64;c=mp.mpf(5)/8
        return 2*c/(b+mp.sqrt(b*b+4*a*c))
    raise ValueError(case)
def exact(case,t):
    if case=='coupled-cancellation':return t-mp.mpf(7)/16,mp.mpf(1),[mp.mpf(0),mp.mpf(0)],mp.mpf(1)
    if case=='pencil':
        d=1+t/8;x=t+t*t/2;z=x/d
        g=1/d;clock=mp.mpf(1)/4-x/(8*d*d)
        return z+t/4-mp.mpf(5)/8,clock+g*(1+t),[g],clock
    x=t;z=x*x+t
    if case=='algebraic':return z-mp.mpf(3)/4,2*x+1,[2*x],mp.mpf(1)
    if case=='nonlinear':
        g=4*x*z+1;clock=2*z+mp.mpf(1)/4
        return z*z+x+t/4-mp.mpf(3009)/4096,clock+g,[g],clock
    raise ValueError(case)
rows=[json.loads(l.split('CELL ',1)[1]) for l in (P/'training.log').read_text().splitlines() if 'CELL {' in l]
fields=['solver','root_route','derivative','value'];cases=['algebraic','coupled-cancellation','pencil','nonlinear']
keys={(v['case'],*(v[f] for f in fields)) for v in rows}
cats=json.loads((P/'CANDIDATES.json').read_text())['routes']
assert len(rows)==len(keys)==240
assert keys=={(k,*(v[f] for f in fields))for k in cases for v in cats}
out=[];root_checks=0;velocity_checks=0;gradient_checks=0;guard_checks=0;slabs=0;max_width=mp.mpf(0);max_guard_width=mp.mpf(0);failures=collections.Counter()
for r in rows:
    case=r['case'];tau=root(case)
    v={k:r[k] for k in ['case',*fields]};v['qualified']=False
    if 'error' in r:
        v['refusal']=r['error'];failures[v['refusal']]+=1;out.append(v);continue
    assert r['work']['inspected']==r['accepted_steps'],r
    assert m(r['safe'])<=tau,(case,r['safe'],tau)
    prep=0;owner=0;gs=r['slabs']
    for s in gs:
        a,b=map(m,s['times'])
        assert 0<=a<b<=2
        slabs+=1;prep+=s['prepared_work']['rhs_count'];owner+=s['prepared_work']['prepared_owners']
        assert s['inverse_defect']<1.
        for i in range(17):
            t=a+(b-a)*i/16
            h,vel,grad,clock=exact(case,t)
            assert inside(s['velocity'],vel),(case,v,s['velocity'],str(vel),str(t))
            velocity_checks+=1
            assert len(grad)==len(s['reduced_gradient'])
            for g,bg in zip(grad,s['reduced_gradient']):
                assert inside(bg,g),(case,v,bg,str(g),str(t))
                gradient_checks+=1
            assert inside(s['reduced_clock'],clock),(case,v,s['reduced_clock'],str(clock),str(t))
            gradient_checks+=1
        for q in s['evaluations']:
            t=m(q['time']);assert a<=t<=b
            h,*_=exact(case,t)
            assert inside(q['value'],h),(case,v,q,str(h))
            guard_checks+=1;max_guard_width=max(max_guard_width,width(q['value']))
    v.update({'driver_rhs':r['driver_rhs'],'prepared_rhs':prep,'prepared_owners':owner,'guard_evals':r['work']['guard_evals'],'hint_calls':r['work']['hint_calls'],'dictionary_uses':r['work']['hint_dictionary_uses'],'rhs_proxy':r['driver_rhs']+r['work']['projection']['rhs_count']+prep,'slabs':len(gs),'numerical_end':r['numerical_end']})
    if r['root'] is None:
        assert r['reason'] is not None
        v['refusal']=r['reason'];failures[v['refusal']]+=1;out.append(v);continue
    b=r['root'];assert inside(b,tau),(case,b,str(tau));root_checks+=1
    assert r['reason'] is None and width(b)<=m(1e-6) and m(r['safe'])<=m(b[0]) and m(b[1])<=m(r['numerical_end'])<2
    assert gs and all(s['velocity'][0]>0. for s in gs)
    assert sum(len(s['evaluations'])for s in gs)==r['work']['guard_evals']
    v['qualified']=True;v['root_width']=float(width(b));max_width=max(max_width,width(b));out.append(v)
summary={'cells':len(rows),'qualified':sum(v['qualified']for v in out),'structural_or_api_errors':sum('error' in r for r in rows),'returned_unqualified':sum(r.get('root') is None and 'error' not in r for r in rows),'root_checks':root_checks,'certified_guard_slabs':slabs,'velocity_checks':velocity_checks,'gradient_clock_checks':gradient_checks,'guard_value_checks':guard_checks,'max_qualified_root_width':float(max_width),'max_observed_guard_value_width':float(max_guard_width),'refusals':dict(failures),'training_sha256':hashlib.sha256((P/'training.log').read_bytes()).hexdigest(),'rows':out}
(P/'AUDIT.json').write_text(json.dumps(summary,indent=2,sort_keys=True,allow_nan=False)+'\n')
print({k:v for k,v in summary.items() if k!='rows'})
