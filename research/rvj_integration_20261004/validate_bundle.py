#!/usr/bin/env python3
"""Artifact consistency only; never executes a historical scientific suite."""
from pathlib import Path
import json

root=Path(__file__).resolve().parent
files=list(root.glob('*.json'))
for path in files:
    json.loads(path.read_text())
dag=json.loads((root/'NEXT_DEVELOPMENT_DAG.json').read_text())
nodes={n['id']:n for n in dag['nodes']}
assert len(nodes)==len(dag['nodes']), 'duplicate node'
seen=set();active=set()
def visit(key):
    assert key in nodes, ('missing dependency',key)
    assert key not in active, ('cycle',key)
    if key in seen:return
    active.add(key)
    for dep in nodes[key]['depends_on']:visit(dep)
    active.remove(key);seen.add(key)
for key,n in nodes.items():
    visit(key)
    for dep in n.get('optional_dependencies',[]):assert dep in nodes
    assert n['priority'] in ['P1','P2','P3']
    assert n['status'] in dag['status_enum']
    assert n['acceptance'] and n['kill_or_abstain'] and n['code_seams']
    if n['status']=='COMPLETE_RESEARCH_SCOPE':
        for p in n['evidence']:assert (root/p).is_file(),p
r=json.loads((root/'RESULTS.json').read_text())
o=json.loads((root/'evidence/exact_oracle.json').read_text())
assert r['counts']['rhs_target_rows']==sum(x['rhs_target_rows'] for x in r['rows'])==len(o['rows'])
assert r['counts']['certified_rows']+r['counts']['rejected_rows']==r['counts']['rhs_target_rows']
assert r['target_counts']['candidates']==r['target_counts']['certified']+r['target_counts']['rejected']
assert r['counts']['enclosure_failures']==0 and o['pass']
a=json.loads((root/'ACCEPTANCE_MATRIX.json').read_text())
for criterion in a['criteria']:
    if criterion['status']=='PASS':
        for p in criterion['evidence']:assert (root/p).is_file(),p
try:
    import jsonschema
    jsonschema.validate(dag,json.loads((root/'DAG.schema.json').read_text()))
    schema_status='PASS_JSONSCHEMA'
except ImportError:
    schema_status='NOT_RUN_PACKAGE_UNAVAILABLE; structural checks performed'
print(json.dumps({'scope':'artifact consistency only','json_files':len(files),'dag_nodes':len(nodes),'dependency_cycle':False,'numeric_rows':len(o['rows']),'schema_validation':schema_status,'status':'PASS'}))
