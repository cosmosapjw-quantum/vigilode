#!/usr/bin/env python3
"""Check package integrity and report contracts; this is not a numerical proof."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parent)
    parser.add_argument('--repo', type=Path, help='Optional check of all original audited file bytes')
    args = parser.parse_args()
    root = args.root.resolve()
    def load(path):
        return json.loads((root/path).read_text(), parse_constant=lambda x: (_ for _ in ()).throw(ValueError(x)))
    def require(condition, message):
        if not condition:
            raise ValueError(message)
    def evidence(path):
        p = (root/path).resolve()
        require(p.is_relative_to(root) and p.is_file(), 'Missing or unsafe package evidence: '+path)
    for p in root.rglob('*.json'):
        load(p.relative_to(root))
    findings=load('FINDINGS.json'); status=load('REPORT_STATUS.json'); dag=load('NEXT_DEVELOPMENT_DAG.json')
    claims=load('RESEARCH_CLAIMS.json'); decision=load('decision/INDEPENDENT_DECISION.json')
    runtime=load('evidence/runtime/NATIVE_RESULT.json'); suite=load('evidence/runtime/full_suite/suite_summary.json')
    source=status['source']['head']
    for doc in (findings,dag,claims,decision,runtime,suite):
        require(doc['source_commit']==source,'Source identity mismatch')
    ids={x['id'] for x in findings['findings']}
    require(len(ids)==len(findings['findings'])==7,'Finding IDs/count mismatch')
    require(Counter(x['severity'] for x in findings['findings'])=={'P1':1,'P2':6},'Severity mismatch')
    require(set(decision['finding_ids_confirmed'])==ids,'Independent decision finding mismatch')
    require(len(status['prior_closures'])==11,'Prior closure count mismatch')
    require(status['production_source_mutated'] is False,'Unexpected production mutation')
    require(status['production_readiness']=='HOLD_MAJOR_REVISION_REQUIRED','Unexpected production promotion')
    require(decision['main_report_review']['status']=='FINAL_REVIEW_COMPLETE','Independent final report not reviewed')
    require(decision['main_report_review']['sha256_at_review']==hashlib.sha256((root/'REPORT_KO.md').read_bytes()).hexdigest(),'Report changed after independent review')
    nodes={n['id']:n for n in dag['nodes']}
    require(len(nodes)==14,'Development node count mismatch')
    visited=set(); active=set()
    def visit(key):
        require(key in nodes,'Missing dependency '+key)
        require(key not in active,'DAG cycle at '+key)
        if key in visited: return
        active.add(key)
        n=nodes[key]
        for field in ['priority','track','objective_ko','target_files','implementation_steps_ko','acceptance_criteria','required_evidence','stop_on_failure','claim_ceiling']:
            require(bool(n.get(field)),key+' lacks '+field)
        for dep in n['depends_on']: visit(dep)
        active.remove(key); visited.add(key)
    for key in nodes: visit(key)
    for finding in findings['findings']:
        require(finding['remediation_node'] in nodes,'Missing remediation')
        require(bool(finding['claim_ceiling']) and bool(finding['source_locations']),'Missing finding scope')
        for p in finding['evidence']+[finding['lane_record']]: evidence(p)
    for claim in claims['claims']:
        require(bool(claim['premises']) and bool(claim['not_established']),'Missing research boundaries')
        for p in claim['evidence']: evidence(p)
        for key in claim['next']: require(key in nodes,'Missing research next node')
    for closure in status['prior_closures']:
        for p in closure['evidence']: evidence(p)
    require(runtime['test_counts']==suite['test_counts']==status['current_test_campaign']['test_counts'],'Runtime summary mismatch')
    require(runtime['test_counts']=={'PASS':632,'UNRESOLVED_BOUNDED_TIMEOUT':23,'IGNORED_NOT_RUN':3},'Runtime observations changed')
    require(runtime['full_suite_claim']=='NOT_COMPLETE_BOUNDED_TIMEOUT','False full-suite claim')
    require(runtime['summary_sha256']==hashlib.sha256((root/'evidence/runtime/full_suite/suite_summary.json').read_bytes()).hexdigest(),'Runtime receipt hash mismatch')
    require('최종 집계 중' not in (root/'REPORT_KO.md').read_text(),'Unresolved runtime placeholder')
    manifested={}
    for line in (root/'MANIFEST.sha256').read_text().splitlines():
        digest,path=line.split('  ',1)
        require(path not in manifested,'Duplicate manifest path')
        evidence(path)
        require(hashlib.sha256((root/path).read_bytes()).hexdigest()==digest,'Package hash mismatch '+path)
        manifested[path]=digest
    actual={str(p.relative_to(root)) for p in root.rglob('*') if p.is_file() and p != root/'MANIFEST.sha256' and '__pycache__' not in p.parts}
    require(set(manifested)==actual,'Manifest does not cover exactly the package files')
    checked=0
    if args.repo:
        repo=args.repo.resolve()
        for path,digest in load('evidence/source/SOURCE_FILE_HASHES.json')['files'].items():
            p=(repo/path).resolve()
            require(p.is_relative_to(repo) and p.is_file(),'Source file missing '+path)
            require(hashlib.sha256(p.read_bytes()).hexdigest()==digest,'Audited source changed '+path)
            checked+=1
    print(json.dumps(dict(status='PASS_PACKAGE_CONTRACTS',files=len(manifested),findings=len(ids),development_nodes=len(nodes),research_claims=len(claims['claims']),source_files_checked=checked,scientific_ceiling='Integrity/consistency only; production HOLD unchanged.'),ensure_ascii=False))

if __name__=='__main__': main()
