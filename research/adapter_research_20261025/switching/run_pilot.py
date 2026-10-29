#!/usr/bin/env python3
"""Serial bounded benchmark; outputs new files and never controls other jobs."""
import argparse, gzip, hashlib, json, pathlib, statistics, subprocess, time
p=argparse.ArgumentParser()
p.add_argument('--binary', required=True, type=pathlib.Path)
p.add_argument('--output', required=True, type=pathlib.Path)
p.add_argument('--repeats', type=int, default=3)
a=p.parse_args()
if not 1 <= a.repeats <= 10: p.error('repeats must be 1..10')
a.output.mkdir(parents=True, exist_ok=False)
raw=[]; groups=[]
for case in ('decay','rotation','stiff-forced'):
    for h0 in ('1e-7','1e-4'):
        for mode in ('evidence','screened-reset','pressure-only','dopri','rodas'):
            rows=[]
            for rep in range(-1,a.repeats):
                cmd=[str(a.binary.resolve()),case,mode,h0]
                started=time.time()
                try:
                    result=subprocess.run(cmd,text=True,capture_output=True,timeout=10)
                except subprocess.TimeoutExpired as exc:
                    result=subprocess.CompletedProcess(cmd,124,exc.stdout or '',exc.stderr or 'timeout')
                    if isinstance(result.stdout,bytes): result.stdout=result.stdout.decode(errors='replace')
                    if isinstance(result.stderr,bytes): result.stderr=result.stderr.decode(errors='replace')
                record={'command':cmd,'rep':rep,'warmup':rep==-1,'exit':result.returncode,
                        'process_seconds':time.time()-started,'stderr':result.stderr}
                try: record['data']=json.loads(result.stdout)
                except json.JSONDecodeError: record['stdout']=result.stdout
                raw.append(record)
                with gzip.open(a.output/'raw.jsonl.gz','at') as f:
                    f.write(json.dumps(record,allow_nan=False)+'\n')
                if result.returncode or 'data' not in record:
                    break
                if rep>=0: rows.append(record['data'])
            groups.append({'case':case,'h0':float(h0),'mode':mode,'samples':len(rows),
                'all_accuracy_pass':bool(rows) and all(x['accuracy_pass'] for x in rows),
                'median_seconds':statistics.median(x['seconds'] for x in rows) if rows else None,
                'max_endpoint_error':max((x['endpoint_max_abs_error'] for x in rows),default=None),
                'rhs_calls': [x['rhs_calls'] for x in rows],
                'switches':[x['record'].get('stats',{}).get('switches') for x in rows]})
summary={'schema':'vigilode.switcher-pilot-summary.v1','binary_sha256':hashlib.sha256(a.binary.read_bytes()).hexdigest(),
    'groups':groups,'timing_kind':'short same-host shared-runtime exploratory measurements',
    'accuracy_kind':'common endpoint cap1e-5, not matched-achieved-error ranking',
    'warmups_per_group':1,'timed_repeats':a.repeats}
(a.output/'SUMMARY.json').write_text(json.dumps(summary,indent=2,allow_nan=False)+'\n')
print(json.dumps(summary,indent=2,allow_nan=False))
