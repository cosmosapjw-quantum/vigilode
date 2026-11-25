"""Independent high precision root and reset-state oracle."""
from pathlib import Path
import hashlib,json,mpmath as mp
P=Path(__file__).parent;mp.mp.dps=85
def m(x):return mp.mpf(float(x))
def width(p):return m(p[1])-m(p[0])
def oracle_root(r):
    s,x0,th=map(m,(r['start'],r['x0'],r['threshold']))
    if r['kind']=='linear':return s+th-x0
    if r['kind']=='dissipative':return s-mp.log((1-th)/(1-x0))
    if r['kind']=='pencil':return mp.sqrt((1+s)**2+2*(th-x0))-1
    raise ValueError(r['kind'])
rows=[json.loads(l.split('CELL ',1)[1]) for l in (P/'training.log').read_text().splitlines() if 'CELL {' in l]
fields=['pre_route','root_route','sync_route','post_route']
keys={(v['kind'],*(v[f] for f in fields)) for v in rows}
assert len(rows)==len(keys)==270
cats=json.loads((P/'CANDIDATES.json').read_text())['routes']
assert keys=={(k,*(v[f] for f in fields)) for k in ['linear','dissipative','pencil'] for v in cats}
out=[];root_checks=0;samples=0;sync_checks=0;maxratio=mp.mpf(0);maxerr=0.;roots={}
for r in rows:
    v={k:r[k] for k in ['kind',*fields]};v['qualified']=False
    if 'pre_error' in r:v['refusal']=r['pre_error'];out.append(v);continue
    pr=r['pre'];root=pr['root']
    assert pr['work']['inspected']==pr['accepted_steps']
    prekey=(r['kind'],r['pre_route'],r['root_route'])
    if prekey in roots:assert roots[prekey]==pr
    roots[prekey]=pr
    if root is None:v['refusal']=pr['reason'];out.append(v);continue
    tau=oracle_root(r)
    assert m(root[0])<=tau<=m(root[1]),(r['kind'],root,tau)
    assert width(root)<=m(1e-6)
    assert pr['reason'] is None and m(pr['safe'])<=m(root[0]) and m(root[1])<=m(pr['numerical_end'])<2
    root_checks+=1;v['root_width']=float(width(root));v['numerical_end']=pr['numerical_end'];v['pre_rhs']=pr['stats']['rhs_calls'];v['guard_evals']=pr['work']['guard_evals'];v['hint_calls']=pr['work']['hint_calls'];v['dictionary_uses']=pr['work']['hint_dictionary_uses']
    if 'post_error' in r:v['refusal']=r['post_error'];out.append(v);continue
    po=r['post'];s=m(root[1]);assert m(po['epoch_start'])==s
    b=po['box'];ts=[m(root[0])+width(root)*i/32 for i in range(33)]+[tau]
    if m(root[0])<=mp.mpf('.5')<=s:ts.append(mp.mpf('.5'))
    X=lambda t,tau:(mp.mpf('1.5')-tau)*mp.exp(-(t-tau))
    for t in ts:assert m(b[0])<=X(s,t)<=m(b[1]),(v,b,t,X(s,t));sync_checks+=1
    v.update({'sync_width':float(width(b)),'incoming_radius':po['radius'],'bound':po['bound'],'post_rhs':po['stats']['rhs_calls'],'rhs_proxy':pr['stats']['rhs_calls']+pr['work']['projection']['rhs_count']+po['stats']['rhs_calls']+po['work']['rhs_count']})
    v['qualified']=bool(po['global'] and po['branch'] and po['pointwise'] and width(b)<=m(1e-6) and m(po['bound'])<=m(5e-6))
    for q in po['queries']:
        for tt in ts:
            t=m(q['t']);x=X(t,tt);z=3*x-2*t+1
            err=max(abs(m(q['state'][0])-x),abs(m(q['state'][1])-z))
            maxerr=max(maxerr,float(err));samples+=1
            if po['global']:
                bd=m(q['physical_error_upper'])
                assert err<=bd,(v,q['t'],tt,err,bd)
                if bd>0:maxratio=max(maxratio,err/bd)
    out.append(v)
summary={'cells':len(rows),'qualified':sum(v['qualified'] for v in out),'pre_calls_expected':45,'unique_pre_receipts_returned':len(roots),'unique_root_successes':sum(v['root'] is not None for v in roots.values()),'root_checks_logical_rows':root_checks,'sync_sample_checks':sync_checks,'post_sample_checks':samples,'max_error':maxerr,'max_ratio':float(maxratio),'training_sha256':hashlib.sha256((P/'training.log').read_bytes()).hexdigest(),'rows':out}
(P/'AUDIT.json').write_text(json.dumps(summary,indent=2,sort_keys=True,allow_nan=False)+'\n')
print({k:v for k,v in summary.items() if k!='rows'})
