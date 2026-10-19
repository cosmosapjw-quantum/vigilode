import json,subprocess,sys
from pathlib import Path
parent,candidate,out=sys.argv[1:];out=Path(out);rows=[]
for case in ['brusselator-50','linear-diffusion-50']:
    ref=None
    for arm,binary,kernel in [('parent-slices',parent,'slices'),('patched-slices',candidate,'slices'),('fixed2',candidate,'fixed2'),('exact-cache',candidate,'exact-cache')]:
        p=subprocess.run([binary,case,kernel,'1e-6','endpoint','1.0','1','0'],capture_output=True,text=True,timeout=30)
        record={'case':case,'arm':arm,'returncode':p.returncode,'stderr':p.stderr}
        if p.returncode==0:record['row']=json.loads(p.stdout)
        rows.append(record);(out/'SMOKE.json').write_text(json.dumps(rows,indent=2,allow_nan=False)+'\n')
        assert p.returncode==0,record
        r=record['row'];assert r['status']=='ok' and r['deterministic'] is True
        pair=(r['times'],r['states'])
        if ref is None:ref=pair
        else:assert pair==ref,'smoke state/time mismatch'
print(json.dumps({'status':'PASS_ADAPTER_SMOKE','cases':2,'arm_runs':len(rows)}))
