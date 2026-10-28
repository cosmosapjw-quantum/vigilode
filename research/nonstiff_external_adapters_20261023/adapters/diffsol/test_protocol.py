#!/usr/bin/env python3
"""Bounded development protocol checks, independent frozen analyzer replay."""
import argparse, hashlib, importlib.util, json, pathlib, subprocess
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--analyzer',required=True);p.add_argument('--out',required=True);args=p.parse_args()
out=pathlib.Path(args.out);out.mkdir(exist_ok=False,parents=True)
spec=importlib.util.spec_from_file_location('frozen_analyzer',args.analyzer);analyzer=importlib.util.module_from_spec(spec);spec.loader.exec_module(analyzer)
source='45037fc24e8f8101605f309e1fdb3a9a4e3df56d'
base=dict(case='decay',dimension=8,rtol=1e-7,atol=1e-10,source_commit=source,max_steps=100000)
checks=[]
def invoke(label,changes):
    request={**base,**changes};(out/(label+'.request.json')).write_text(json.dumps(request)+'\n')
    run=subprocess.run([args.binary],input=json.dumps(request),text=True,capture_output=True,timeout=45)
    (out/(label+'.stdout.json')).write_text(run.stdout);(out/(label+'.stderr.log')).write_text(run.stderr)
    return run
for case in ['decay','driven','logistic','harmonic','rapid','damped','mixed-scales','nonnormal','large-vector','moderate-stiff','stiff-control']:
    n=16384 if case=='large-vector' else 12 if case in ['mixed-scales','moderate-stiff','stiff-control'] else 8
    run=invoke(case,dict(case=case,dimension=n));assert run.returncode==0,run.stderr
    row=json.loads(run.stdout);replay=analyzer.checked_record(row);assert replay['valid'],replay
    assert row['success'] and row['times']==[row['end']*i/4 for i in range(5)]
    assert row['states'][0]==analyzer.exact(case,n,0)[0].tolist()
    assert row['work']['rhs_attempted']==row['work']['rhs_completed']>0
    assert row['adapter']['method']==('BDF' if case in ['moderate-stiff','stiff-control'] else 'Tsit45')
    if row['adapter']['method']=='Tsit45':assert row['work']['jvp_calls']==0
    checks.append(dict(case=case,valid=True,replay=replay,work=row['work']))
for label,change in [('unknown',dict(case='unknown')),('odd',dict(case='harmonic',dimension=7)),('rtolzero',dict(rtol=0)),('atolnegative',dict(atol=-1)),('source',dict(source_commit='wrong')),('adapter-source',dict(adapter_source_commit='wrong')),('mode',dict(mode='wrong'))]:
    run=invoke(label,change);assert run.returncode==2 and not run.stdout;checks.append(dict(negative=label,valid=True))
run=invoke('max_steps',dict(max_steps=1));assert run.returncode==0
row=json.loads(run.stdout);assert not row['success'] and len(row['times'])<5 and row['work']['public_step_calls']==1
assert row['last_accepted_time']>0 and row['work']['rhs_attempted']>0
assert analyzer.checked_record(row)['valid'];checks.append(dict(negative='max_steps',valid=True,partial_outputs=len(row['times'])))
(out/'SUMMARY.json').write_text(json.dumps(dict(status='PASS',binary_sha256=hashlib.sha256(pathlib.Path(args.binary).read_bytes()).hexdigest(),checks=checks),indent=2)+'\n')
print(json.dumps(dict(status='PASS',checks=len(checks))))
