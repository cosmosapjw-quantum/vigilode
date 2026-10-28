#!/usr/bin/env python3
"""Targeted adapter checks against the frozen benchmark's independent scorer."""
import argparse, importlib.util, json, os, pathlib, subprocess, sys, time
sys.dont_write_bytecode = True
import numpy as np

HERE=pathlib.Path(__file__).resolve().parent
SOURCE='45037fc24e8f8101605f309e1fdb3a9a4e3df56d'

def main():
    p=argparse.ArgumentParser();p.add_argument('--frozen-protocol',required=True);p.add_argument('--out',required=True);p.add_argument('--baseline');args=p.parse_args()
    out=pathlib.Path(args.out);out.mkdir(parents=True,exist_ok=False)
    spec=importlib.util.spec_from_file_location('frozen_analyzer',pathlib.Path(args.frozen_protocol)/'analyze_bench.py');scorer=importlib.util.module_from_spec(spec);spec.loader.exec_module(scorer)
    results=[]
    env=os.environ.copy();env['JULIA_DEPOT_PATH']=env.get('VIGILODE_JULIA_DEPOT','/home/cosmosapjw/vigilode-nonstiff-adapters-run-tTYnLc/julia-development/julia-depot');env['JULIA_NUM_THREADS']='1';env['OPENBLAS_NUM_THREADS']='1'
    executable=env.get('VIGILODE_JULIA_EXECUTABLE','/home/cosmosapjw/.julia/juliaup/julia-1.12.7+0.x64.linux.gnu/bin/julia')
    probe=subprocess.run([executable,'--startup-file=no',f'--project={HERE}',str(HERE/'model_probe.jl')],capture_output=True,text=True,env=env,timeout=45)
    (out/'model-probe.stdout').write_text(probe.stdout);(out/'model-probe.stderr').write_text(probe.stderr)
    assert probe.returncode==0,probe.stderr
    for row in json.loads(probe.stdout):
        name=row['case'];n=row['dimension'];i=np.arange(n);t=row['time'];y,ref=scorer.exact(name,n,t);u=np.array(row['u'])
        assert np.max(np.abs(np.array(row['initial'])-y)/np.maximum(1.,np.abs(y)))<=5e-14
        assert np.max(np.abs(np.array(row['reference'])-ref)/np.maximum(1.,np.maximum(np.abs(y),np.abs(ref))))<=5e-14
        a=np.array([.25,1.,3.,.5])[i%4]
        if name=='logistic':a=.25+.1*(i%8)
        if name=='large-vector':a=.5+.05*(i%8)
        if name=='moderate-stiff':a=np.array([1.,30.,300.])[i%3]
        if name=='stiff-control':a=np.array([1.,1000.,100000.])[i%3]
        expected=-a*u
        if name=='logistic':expected=a*u*(1-u)
        if name=='driven':expected=-a*u+np.cos(t)+a*np.sin(t)
        if name=='nonnormal':expected[0::2]=-u[0::2]+30*u[1::2];expected[1::2]=-2*u[1::2]
        if name in ('harmonic','rapid','damped'):
            j=np.arange(n//2)%4;w=(100.+5*j) if name=='rapid' else (10.+j) if name=='damped' else (1.+.25*j);d=3. if name=='damped' else 0.
            expected[0::2]=-d*u[0::2]-w*u[1::2];expected[1::2]=w*u[0::2]-d*u[1::2]
        assert np.max(np.abs(np.array(row['rhs'])-expected)/np.maximum(1.,np.abs(expected)))<=5e-14,(name,n)

    cases=[('decay',8),('driven',8),('logistic',8),('harmonic',8),('rapid',8),('damped',8),('mixed-scales',12),('nonnormal',8),('large-vector',4096),('large-vector',16384),('moderate-stiff',12),('stiff-control',12)]
    cases += [('stiff-control',12),('decay',8),('rapid',8)]
    for i,(case,n) in enumerate(cases):
        req=dict(case=case,dimension=n,rtol=1e-12 if i==14 else 1e-6,atol=1e-15 if i==14 else 1e-9,source_commit=SOURCE,max_steps=1 if i>=12 else 100000,mode='julia')
        started=time.monotonic();(out/f'{i:02}.request.json').write_text(json.dumps(req))
        try:result=subprocess.run([str(HERE/'run.sh')],input=json.dumps(req),text=True,capture_output=True,timeout=45)
        except subprocess.TimeoutExpired as error:
            (out/f'{i:02}.stdout').write_bytes(error.stdout or b'');(out/f'{i:02}.stderr').write_bytes(error.stderr or b'');(out/f'{i:02}.timeout.json').write_text(json.dumps({'status':'TIMEOUT','limit_seconds':45,'process_seconds':time.monotonic()-started}));raise
        (out/f'{i:02}.stdout').write_text(result.stdout);(out/f'{i:02}.stderr').write_text(result.stderr)
        assert result.returncode==0,(i,result.returncode,result.stderr[-2000:])
        record=json.loads(result.stdout);replay=scorer.checked_record(record)
        assert replay['valid'],(i,replay)
        assert record['success']==(i<12),(i,record['message'])
        assert len(record['last_accepted_state'])==n
        if i<12:
            assert record['last_accepted_time']==record['end']
            assert np.array_equal(np.array(record['last_accepted_state']).view(np.uint64),np.array(record['states'][-1]).view(np.uint64))
        if i>=12:
            assert record['work']['rhs_completed']>0 and record['last_accepted_time']<record['end']
            stats=record['work']['solver_stats']
            if i in (12,14):
                assert stats['nreject']>=1 and stats['naccept']==0
                assert record['last_accepted_time']==0.
                assert np.array_equal(record['last_accepted_state'],scorer.exact(case,n,0.)[0])
            else:
                assert stats['naccept']==1 and record['last_accepted_time']>0
                assert np.max(np.abs(np.array(record['last_accepted_state'])-scorer.exact(case,n,record['last_accepted_time'])[1]))<5e-14
        if args.baseline and i<12:
            baseline=json.loads((pathlib.Path(args.baseline)/f'{i:02}.stdout').read_text())
            assert np.array_equal(np.array(record['states']).view(np.uint64),np.array(baseline['states']).view(np.uint64)),(case,'state bits changed')
            assert record['times']==baseline['times'] and record['work']==baseline['work'],(case,'time/counters changed')
        results.append(dict(case=case,dimension=n,process_seconds=time.monotonic()-started,success=record['success'],replay=replay))
    for i,updates in enumerate([{'case':'unknown'},{'case':'harmonic','dimension':3},{'rtol':0.},{'max_steps':0},{'source_commit':'wrong'},{'mode':'diffsol'},{'dimension':7},{'case':'large-vector','dimension':1024},{'adapter_source_commit':'wrong'}]):
        req=dict(case='decay',dimension=8,rtol=1e-6,atol=1e-9,source_commit=SOURCE,max_steps=100000);req.update(updates)
        result=subprocess.run([str(HERE/'run.sh')],input=json.dumps(req),text=True,capture_output=True,timeout=45)
        (out/f'negative-{i}.stdout').write_text(result.stdout);(out/f'negative-{i}.stderr').write_text(result.stderr)
        assert result.returncode!=0 and not result.stdout,(updates,result.stdout)
    (out/'RESULTS.json').write_text(json.dumps(dict(results=results,negative_controls=9),indent=2));print(json.dumps(dict(records=len(results),negative_controls=9)))

if __name__=='__main__':main()
