#!/usr/bin/env python3
"""Paired actual-trajectory timing and instruction slopes; no solver tuning."""
import argparse, datetime, hashlib, json, os, pathlib, statistics, subprocess, time

p=argparse.ArgumentParser()
p.add_argument('--binary',required=True)
p.add_argument('--valgrind',required=True)
p.add_argument('--output',default=str(pathlib.Path(__file__).parent/'raw'/'performance'))
p.add_argument('--resume-timing',action='store_true',help='reuse timing JSON from this verified identical binary after an environment-only profiler failure')
a=p.parse_args(); root=pathlib.Path(a.output); root.mkdir(parents=True,exist_ok=True)
binary=pathlib.Path(a.binary).resolve()
meta={'schema_version':1,'started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),
      'binary':str(binary),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),
      'timing_scope':'same process repeated complete solves including model/result allocations; oracle+JSON after timer',
      'cases':[],'executions':[]}
pairs=[('stiff-cubic','pr91','observed-pr91',64),
       ('reentrant-nonlinear','rhs-only','observed-strict-sentinel',16)]

def execute(command, name):
    prior=root/(name+'.json')
    if a.resume_timing and '_time_' in name and prior.exists():
        row=json.loads(prior.read_bytes())['rows'][0]
        assert row['pass'] and row['case']==command[1] and row['policy']==command[2] and row['repeats']==int(command[3])
        meta['executions'].append({'command':command,'stdout':str(prior),'status':'reused',
            'sha256':hashlib.sha256(prior.read_bytes()).hexdigest(),
            'identity_receipt':'PROFILE_ENV_CORRECTION.json'})
        return row
    before=time.monotonic()
    r=subprocess.run(command,capture_output=True)
    (root/(name+'.json')).write_bytes(r.stdout)
    (root/(name+'.stderr')).write_bytes(r.stderr)
    meta['executions'].append({'command':command,'stdout':str(root/(name+'.json')),
        'stderr':str(root/(name+'.stderr')),'exit_code':r.returncode,
        'wall_seconds':time.monotonic()-before})
    if r.returncode: raise RuntimeError(f'{name} exit {r.returncode}')
    row=json.loads(r.stdout)['rows'][0]
    assert row['pass']
    return row

for case,baseline,candidate,repeats in pairs:
    blocks=[]
    for block in range(5):
        order=[baseline,candidate,candidate,baseline] if block%2==0 else [candidate,baseline,baseline,candidate]
        times={baseline:[],candidate:[]}
        for position,policy in enumerate(order):
            name=f'{case}_time_{block}_{position}_{policy}'
            row=execute([str(binary),case,policy,str(repeats)],name)
            times[policy].append(row['seconds']/repeats)
        ta=statistics.mean(times[baseline]); tb=statistics.mean(times[candidate])
        blocks.append({'block':block,'order':order,'baseline_seconds':ta,'candidate_seconds':tb,'speedup':ta/tb})
    slopes={}
    profiles=[]
    for policy in [baseline,candidate]:
        instructions=[]
        for count in [4,12]:
            name=f'{case}_profile_{policy}_{count}'
            profile=root/(name+'.callgrind')
            execute([a.valgrind,'--tool=callgrind','--cache-sim=no','--branch-sim=no',
                     '--callgrind-out-file='+str(profile),str(binary),case,policy,str(count)],name)
            lines=profile.read_text().splitlines()
            ir=int(next(x.split(':',1)[1].strip().split()[0] for x in lines if x.startswith('summary:')))
            instructions.append(ir)
            profiles.append({'policy':policy,'repeats':count,'instructions':ir,'file':str(profile)})
        slopes[policy]=(instructions[1]-instructions[0])/8
    ratios=[b['speedup'] for b in blocks]
    meta['cases'].append({'case':case,'baseline':baseline,'candidate':candidate,'repeats':repeats,
        'timing_blocks':blocks,'median_speedup':statistics.median(ratios),
        'observed_speedup_range':[min(ratios),max(ratios)],
        'instruction_slopes_per_trajectory':slopes,
        'instruction_change_percent':100*(slopes[candidate]/slopes[baseline]-1),
        'profiles':profiles})
    (root.parent.parent/'PERFORMANCE.json').write_text(json.dumps(meta,indent=2)+'\n')
meta['completed_utc']=datetime.datetime.now(datetime.timezone.utc).isoformat()
(root.parent.parent/'PERFORMANCE.json').write_text(json.dumps(meta,indent=2)+'\n')
print(json.dumps({'completed':True,'cases':[{
    k:c[k] for k in ['case','median_speedup','observed_speedup_range','instruction_slopes_per_trajectory','instruction_change_percent']
} for c in meta['cases']]},indent=2))
