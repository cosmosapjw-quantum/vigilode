#!/usr/bin/env python3
"""Capture actual native binaries; building is separate under the shared lock."""
from pathlib import Path
import argparse,hashlib,json,os,subprocess,time
p=argparse.ArgumentParser();p.add_argument('--target',type=Path,required=True);a=p.parse_args()
root=Path(__file__).resolve().parent
records=[]
for binary,name in [('vigilode-r2-phi-independent-probe','native'),('legacy_closure','legacy_closure'),('tableau_bits','tableau_bits')]:
    path=a.target.resolve()/'debug'/binary
    start=time.monotonic()
    with (root/(name+'.jsonl')).open('w') as out,(root/(name+'.stderr')).open('w') as err:
        try:
            r=subprocess.run([str(path)],stdout=out,stderr=err,timeout=60,check=False)
            status={'exit_code':r.returncode,'timed_out':False}
        except subprocess.TimeoutExpired:
            status={'exit_code':None,'timed_out':True}
    records.append({'binary':binary,'binary_sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'wall_seconds':time.monotonic()-start,**status,'stdout':name+'.jsonl','stderr':name+'.stderr'})
(root/'NATIVE_EXECUTION.json').write_text(json.dumps({'source_commit':'cc2cd041737e7ff543624d1b59893a3b4397369f','records':records},indent=2)+'\n')
print(json.dumps(records,indent=2))
raise SystemExit(0 if all(r['exit_code']==0 for r in records) else 1)
