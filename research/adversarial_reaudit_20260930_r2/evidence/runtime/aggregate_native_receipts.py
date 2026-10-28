#!/usr/bin/env python3
"""Derive exact per-test statuses from original libtest stdout, without reruns."""
from pathlib import Path
import json,re,hashlib,sys
p=Path(sys.argv[1]);x=json.loads(p.read_text());checks=[]
for b in x['binaries']:
 if 'invocation' not in b:continue
 r=b['invocation'];f=Path(r['stdout']);txt=f.read_text(errors='replace')
 pairs=re.findall(r'^test (.*?) \.\.\. (ok|FAILED|ignored)\b',txt,re.MULTILINE);st=dict(pairs)
 listed=b.get('listed_tests',[])
 b['tests']=[{'name':n,'status':{'ok':'PASS','FAILED':'FAIL','ignored':'IGNORED_NOT_RUN'}.get(st.get(n),'UNRESOLVED_BOUNDED_TIMEOUT' if r['timed_out'] else 'UNRESOLVED_MISSING_RESULT')} for n in listed]
 b['stdout_sha256']=hashlib.sha256(f.read_bytes()).hexdigest()
 if b.get('rust_summary'):
  observed={s:sum(t['status']==s for t in b['tests']) for s in ['PASS','FAIL','IGNORED_NOT_RUN']}
  summary=b['rust_summary'];ok=observed=={'PASS':summary['passed'],'FAIL':summary['failed'],'IGNORED_NOT_RUN':summary['ignored']};checks.append({'target':b['target'],'counts_agree':ok,'observed':observed,'summary':summary})
alltests=[t for b in x['binaries'] for t in b['tests']]
x['test_counts']={s:sum(t['status']==s for t in alltests) for s in sorted(set(t['status'] for t in alltests))}
x['binary_counts']={s:sum(b['status']==s for b in x['binaries']) for s in sorted(set(b['status'] for b in x['binaries']))}
x['receipt_aggregation']={'method':'anchored per-line libtest result parsing; no test rerun','full_summary_counts_all_agree':all(t['counts_agree'] for t in checks),'complete_summary_checks':checks,'note':'Execution runner v1 had an overly greedy result-line regex. This parser replaces derived counts using unchanged raw stdout; execution outcomes are unchanged.'}
x['unfinished_tests']=[{'target':b['target'],**t} for b in x['binaries'] for t in b['tests'] if t['status'].startswith('UNRESOLVED')]
x['ignored_tests']=[{'target':b['target'],**t} for b in x['binaries'] for t in b['tests'] if t['status']=='IGNORED_NOT_RUN']
if not p.with_name('suite_summary_execution_original.json').exists():
 p.with_name('suite_summary_execution_original.json').write_text(p.read_text())
p.write_text(json.dumps(x,indent=2)+'\n')
print(json.dumps({k:x[k] for k in ['full_suite_claim','binary_counts','test_counts','source_manifests_and_lock_unchanged']}));print('libtest summary agreement',x['receipt_aggregation']['full_summary_counts_all_agree'])
