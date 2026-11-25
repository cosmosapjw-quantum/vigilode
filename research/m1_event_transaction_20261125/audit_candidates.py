"""Independent result reduction; exact hidden-root witness and harness evidence binding."""
from pathlib import Path
from fractions import Fraction as F
import json,math,hashlib
from mathharness_general.poly_exact import parse_polynomial
from mathharness_general.sturm import count_distinct_real_roots
P=Path(__file__).resolve().parent
def read(name,tag):return [json.loads(l.split(tag,1)[1]) for l in (P/name).read_text().splitlines() if tag+'{' in l]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
s=json.loads((P/'SELECTION.json').read_text());c=s['selected']['id']
digest,name=(P/'SELECTION_PREHOLDOUT.sha256').read_text().split(maxsplit=1)
assert sha(P/'SELECTION.json')==digest
tr=read('training.log','CELL ');ho=read('holdout.log','HOLDOUT ')
assert len(tr)==156 and len(ho)==20 and len({(r['candidate'],r['case'],r['rtol']) for r in ho})==20
assert all(r['transaction_ok'] for r in tr+ho if r['candidate']!='legacy')
for r in tr+ho:
 expected=r['expected'];roots=r['roots']
 same=len(expected)==len(roots) and all(a[0]==b[0] for a,b in zip(expected,roots))
 if same:assert abs(max(abs(a[1]-b[1]) for a,b in zip(expected,roots))-r['error'])<1e-14
 else:assert r['error'] is None and not r['pass']
 if r['pass']:assert r['error']<=2e-5
 assert all(a[1]<=b[1] for a,b in zip(roots,roots[1:]))
 if r['candidate']!='legacy':assert r['numerical_end']>=r['end']
 if r['candidate']!='legacy' and r['pass']:assert all(e[1]<=r['end'] for e in roots)
# Exact rational witness, not inference from no sampled sign change.
a,b=F(431,1000),F(432,1000);l,h=F(17,40),F(9,20)
g=lambda t:(t-a)*(t-b)
assert l<a<b<h and g(l)>0 and g(h)>0
root=count_distinct_real_roots(parse_polynomial('(t-431/1000)*(t-432/1000)'),'t',str(l),str(h))
assert root['status']=='CERTIFIED' and root['count']==2
selected=[r for r in ho if r['candidate']==c];base=[r for r in ho if r['candidate']=='legacy']
assert sum(r['pass'] for r in selected)==8
assert all(not r['pass'] for r in selected if r['case']=='hidden-pair')
assert all(r['pass'] for r in selected if r['case']!='hidden-pair')
assert all(r['pass'] for r in tr if r['candidate']==c)
counts={r['id']:r['pass_count'] for r in s['candidates']}
results={'schema':'vigilode.m1.event.transaction.v1','selected':c,'selection_sha256':digest,'training_rows':len(tr),'training_candidate_rows':144,'training_legacy_rows':12,'training_candidate_pass':sum(r['pass'] for r in tr if r['candidate']!='legacy'),'training_legacy_pass':sum(r['pass'] for r in tr if r['candidate']=='legacy'),'qualified_candidates':sum(v==12 for v in counts.values()),'holdout_selected_pass':sum(r['pass'] for r in selected),'holdout_selected_total':len(selected),'holdout_legacy_pass':sum(r['pass'] for r in base),'holdout_legacy_total':len(base),'positive_holdout_max_time_error':max(r['error'] for r in selected if r['pass']),'exact_hidden_pair':{'interval':[str(l),str(h)],'roots':[str(a),str(b)],'endpoint_values':[str(g(l)),str(g(h))],'sturm':root},'bo_guidance':s['bo_guidance'],'per_case_rows':tr+ho,'scope':'accepted-step transaction for sampled roots; no arbitrary black-box completeness','next':'M1-POLYNOMIAL-ROOT-ISOLATION-02'}
(P/'RESULTS.json').write_text(json.dumps(results,indent=2,sort_keys=True,allow_nan=False)+'\n')
print('training',results['training_candidate_pass'],'/144; legacy',results['training_legacy_pass'],'/12; held-out selected',results['holdout_selected_pass'],'/10; legacy',results['holdout_legacy_pass'],'/10')
print('exact hidden pair roots',root['count'],'endpoints',g(l),g(h))
