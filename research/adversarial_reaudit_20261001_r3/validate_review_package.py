#!/usr/bin/env python3
"""Read-only integrity/consistency check. Does not rerun numerical experiments."""
import hashlib
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parent
SOURCE='cc2cd041737e7ff543624d1b59893a3b4397369f'
def read(name):
    return json.loads((ROOT/name).read_text(),parse_constant=lambda x:(_ for _ in ()).throw(ValueError(x)))
def require(condition,message):
    if not condition:raise ValueError(message)

def main():
    entries={}
    for line in (ROOT/'MANIFEST.sha256').read_text().splitlines():
        digest,name=line.split('  ',1)
        p=Path(name)
        require(not p.is_absolute() and '..' not in p.parts,'Unsafe manifest path')
        require(name not in entries,'Duplicate manifest entry')
        entries[name]=digest
        require(hashlib.sha256((ROOT/name).read_bytes()).hexdigest()==digest,'Hash mismatch: '+name)
    actual={str(p.relative_to(ROOT)) for p in ROOT.rglob('*') if p.is_file() and p!=ROOT/'MANIFEST.sha256' and '__pycache__' not in p.parts}
    require(actual==set(entries),'Manifest does not exactly cover package')
    for name in entries:
        if name.endswith('.json'):read(name)
        if name.endswith('.jsonl'):
            for line in (ROOT/name).read_text().splitlines():
                if line.strip():json.loads(line,parse_constant=lambda x:(_ for _ in ()).throw(ValueError(x)))
    f=read('FINDINGS.json');s=read('REPORT_STATUS.json');d=read('NEXT_DEVELOPMENT_DAG.json');c=read('RESEARCH_CLAIMS.json')
    for x in [f,s,d,c]:require(x['source_commit']==SOURCE,'Source identity mismatch')
    require(len(f['findings'])==4 and len(f['prior_closures'])==7,'Finding/closure counts')
    require(sum(x['severity']=='P1' for x in f['findings'])==1,'P1 count')
    require(sum(x['new_counterexample'] for x in f['findings'])==3,'New counterexample count')
    require(len({x['id'] for x in f['findings']})==4,'Duplicate finding ID')
    require(all(not x['introduced_regression_demonstrated'] for x in f['findings']),'Unexpected regression claim')
    nodes={x['id']:x for x in d['nodes']}
    require(len(nodes)==d['node_count']==s['next_development_nodes'],'DAG count')
    seen=set()
    for id in d['topological_order']:
        require(id in nodes and id not in seen,'Invalid/duplicate DAG order')
        require(set(nodes[id]['depends_on'])<=seen,'DAG dependency order')
        require(bool(nodes[id]['acceptance']) and bool(nodes[id]['implementation']),'Empty development contract')
        seen.add(id)
    require(seen==set(nodes),'Incomplete DAG')
    wave={id:i for i,w in enumerate(d['execution_waves']) for id in w}
    require(len(wave)==len(nodes) and set(wave)==set(nodes),'Wave coverage')
    for id,n in nodes.items():
        require(all(wave[x]<wave[id] for x in n['depends_on']),'Wave dependency error: '+id)
    require(s['research_ledger_verdict']==c['research_process_verdict']=='INCONCLUSIVE','Process authority mismatch')
    require(not s['actual_speedup_measured'] and not s['production_modified'],'Unestablished promotion')
    decision=read('decision/INDEPENDENT_DECISION.json')
    require(decision['status']=='FINAL_INDEPENDENT_BOUNDED_ACCEPTANCE','Independent review incomplete')
    require('PENDING' not in decision['scientific_promotion'],'Stale decision header')
    for lane,file,count in [('time','native_raw.jsonl',29),('statistics','native_probe.jsonl',10),('arithmetic','native.jsonl',529),('arithmetic','legacy_closure.jsonl',104)]:
        require(sum(bool(x.strip()) for x in (ROOT/lane/file).read_text().splitlines())==count,'Native row count '+lane+'/'+file)
    require(all(read('polynomial/RESULTS.json')['checks'].values()),'Polynomial recorded gate')
    h=read('homotopy/results.json')
    require(h['target']=='strict-lower-projection' and len(h['rows'])==24 and h['all_exact_checks'],'Homotopy target/results')
    runtime=read(s['runtime_summary'])
    require(runtime.get('finalized') is True,'Runtime evidence not finalized')
    require(runtime.get('full_workspace_pass') is False,'Unexpected full-suite claim')
    # A report-only publication can advance HEAD; match audited code bytes, not HEAD.
    if (ROOT.parent.parent/'Cargo.toml').is_file():
        repo=ROOT.parent.parent
        for x in read('time/SOURCE_CONTRACT.json')['files']:
            require(hashlib.sha256((repo/x['path']).read_bytes()).hexdigest()==x['sha256'],'Production bytes changed: '+x['path'])
    print(json.dumps({'status':'PASS_PACKAGE_CONTRACTS','files':len(entries),'findings':4,'closures':7,'development_nodes':len(nodes),'claim_scope':'integrity and recorded evidence consistency only'},ensure_ascii=False))

if __name__=='__main__':main()
