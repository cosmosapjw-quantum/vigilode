#!/usr/bin/env python3
"""Independent-trajectory throughput supplement, local only and bounded."""
import argparse,hashlib,json,os,pathlib,random,subprocess,time,statistics,gzip,tarfile,math

def replay(d):
    try:
        n=d['dimension'];members=d['members'];max_error=0.;max_ref=0.
        assert len(d['states'])==len(d['times'])==len(d['reference'])==members
        for member in range(members):
            assert len(d['states'][member])==len(d['times'][member])==len(d['reference'][member])
            if d.get('success'):assert d['times'][member]==[0.,4.]
            for t,state,ref in zip(d['times'][member],d['states'][member],d['reference'][member]):
                assert len(state)==len(ref)==n
                for i,(y,r) in enumerate(zip(state,ref)):
                    expected=(.5+.5*(i%17)/16.)*math.exp(-(.5+.35*(i%13)/12.+member*.01)*t)
                    assert math.isfinite(t) and math.isfinite(y) and math.isfinite(r)
                    max_error=max(max_error,abs(y-expected));max_ref=max(max_ref,abs(r-expected))
        assert math.isfinite(d['elapsed_seconds']) and d['elapsed_seconds']>0
        assert max_ref<5e-14
        if d.get('success'):assert abs(max_error-d['max_normalized_error'])<1e-13
        return {'valid':True,'max_error':max_error,'reference_difference':max_ref,
                'states_sha256':hashlib.sha256(json.dumps(d['states'],separators=(',',':')).encode()).hexdigest()}
    except Exception as e:return {'valid':False,'reason':repr(e)}

def save(path,obj):path.write_text(json.dumps(obj,indent=2)+'\n')
def main():
    ap=argparse.ArgumentParser();ap.add_argument('--binary',required=True);ap.add_argument('--out',required=True)
    ap.add_argument('--source-commit',required=True);ap.add_argument('--budget-seconds',type=float,default=180.)
    a=ap.parse_args();binary=pathlib.Path(a.binary).resolve();out=pathlib.Path(a.out);out.mkdir(parents=True,exist_ok=False)
    if not 0<a.budget_seconds<=7200:ap.error('positive budget <=7200 required')
    configs=[{'id':i,'threads':w,'rtol':tol} for i,(w,tol) in enumerate((w,tol) for tol in (1e-6,1e-8) for w in (1,2))]
    plan={'schema':'vigilode.nonstiff-ensemble-campaign.v1','source_commit':a.source_commit,'binary':str(binary),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),
          'ncp_used':False,'members':16,'dimension_per_member':4096,'configurations':configs,'warmups':1,'repeats':3,
          'affinity':sorted(os.sched_getaffinity(0)) if hasattr(os,'sched_getaffinity') else None,'budget_seconds':a.budget_seconds,
          'timing':'reused Rayon pool constructed before timer; all member problem/y0/workspace/integration inside timer'}
    save(out/'PLAN.json',plan);receipts=[];start=time.monotonic();rng=random.Random(20261023)
    for repeat in range(-1,3):
        order=configs.copy();rng.shuffle(order)
        for cfg in order:
            phase='warmup' if repeat<0 else 'timed';stem=f"{cfg['id']:03d}_{phase}_{max(0,repeat)}"
            stdout=out/(stem+'.stdout.json');stderr=out/(stem+'.stderr.log');remaining=a.budget_seconds-(time.monotonic()-start)
            rec={**cfg,'phase':phase,'repeat':repeat,'stdout':stdout.name,'stderr':stderr.name}
            cmd=[str(binary),'--threads',str(cfg['threads']),'--dimension','4096','--members','16','--rtol',str(cfg['rtol']),'--source-commit',a.source_commit]
            rec['command']=cmd
            if remaining<=0:rec.update(status='NOT_RUN_BUDGET',exit_code=None)
            else:
                t=time.monotonic()
                try:
                    with stdout.open('w') as f,stderr.open('w') as e:r=subprocess.run(cmd,stdout=f,stderr=e,timeout=min(45.,remaining),check=False)
                    rec.update(status='COMPLETED' if r.returncode==0 else 'PROCESS_FAILED',exit_code=r.returncode)
                except subprocess.TimeoutExpired:rec.update(status='TIMEOUT',exit_code=None)
                except OSError as error:
                    stderr.write_text(str(error)+'\n');rec.update(status='LAUNCH_FAILED',exit_code=None)
                rec['elapsed_process_seconds']=time.monotonic()-t
            for key,path in [('stdout',stdout),('stderr',stderr)]:
                if path.exists():rec[key+'_sha256']=hashlib.sha256(path.read_bytes()).hexdigest()
            receipts.append(rec);save(out/'RECEIPTS.json',receipts);print(json.dumps(rec),flush=True)
    rows=[];replays=[];unparsed=[]
    for cfg in configs:
        records=[]
        for r in receipts:
            if r['id']!=cfg['id']:continue
            try:
                d=json.loads((out/r['stdout']).read_text());check=replay(d)
                if d.get('source_commit')!=a.source_commit or any(d.get(k)!=v for k,v in {'case':'ensemble-decay','mode':'dopri5-ensemble','dimension':4096,'members':16,'threads':cfg['threads'],'rtol':cfg['rtol']}.items()):
                    check={'valid':False,'reason':'source/configuration identity mismatch'}
                if hashlib.sha256((out/r['stdout']).read_bytes()).hexdigest()!=r.get('stdout_sha256'):
                    check={'valid':False,'reason':'stdout hash mismatch'}
                replays.append({'stdout':r['stdout'],**check})
                d['replay']=check;records.append((r,d))
            except Exception as error:unparsed.append({'receipt':r,'reason':repr(error)})
        timed=[d for r,d in records if r['phase']=='timed']
        seconds=[d['elapsed_seconds'] for d in timed]
        rows.append({**cfg,'all_runs_successful':len(records)==4 and all(r['status']=='COMPLETED' and r['exit_code']==0 and d.get('success',False) and d['replay']['valid'] for r,d in records),
                     'median_seconds':statistics.median(seconds) if seconds else None,'seconds':seconds,
                     'max_normalized_error':max((d['max_normalized_error'] for d in timed if d.get('max_normalized_error') is not None),default=None),
                     'state_digests':list(dict.fromkeys(d['replay'].get('states_sha256') for d in timed)),
                     'work':timed[-1].get('work') if timed else None})
    paired=[]
    for tol in (1e-6,1e-8):
        one=next(r for r in rows if r['rtol']==tol and r['threads']==1);two=next(r for r in rows if r['rtol']==tol and r['threads']==2)
        paired.append({'rtol':tol,'one_worker_over_two_worker_time':one['median_seconds']/two['median_seconds'] if one['all_runs_successful'] and two['all_runs_successful'] else None,
                       'identical_states_across_workers_and_repeats':len(one['state_digests'])==1 and one['state_digests']==two['state_digests']})
    save(out/'SUMMARY.json',{'schema':'vigilode.nonstiff-ensemble-summary.v1','unparsed':unparsed,'source_commit':a.source_commit,'ncp_used':False,'elapsed_campaign_seconds':time.monotonic()-start,
                           'rows':rows,'comparisons':paired,'interpretation':'Independent member throughput only; no RK-stage or single-trajectory parallel speed claim.'})
    save(out/'REPLAY.json',replays)
    paths=sorted([*out.glob('*.stdout.json'),*out.glob('*.stderr.log')]);entries=[];archive=out/'RAW_OUTPUTS.tar.gz'
    with archive.open('wb') as raw:
        with gzip.GzipFile(filename='',mode='wb',fileobj=raw,mtime=0) as gz:
            with tarfile.open(mode='w',fileobj=gz,format=tarfile.USTAR_FORMAT) as tar:
                for path in paths:
                    content=path.read_bytes();entries.append({'path':path.name,'bytes':len(content),'sha256':hashlib.sha256(content).hexdigest()})
                    info=tarfile.TarInfo(path.name);info.size=len(content);info.mode=0o644;info.mtime=0
                    with path.open('rb') as f:tar.addfile(info,f)
    with tarfile.open(archive,'r:gz') as tar:
        for e in entries:assert hashlib.sha256(tar.extractfile(e['path']).read()).hexdigest()==e['sha256']
    save(out/'RAW_MANIFEST.json',{'archive':archive.name,'sha256':hashlib.sha256(archive.read_bytes()).hexdigest(),'files':entries,'verified':True})
    (out/'.gitignore').write_text('# Original bytes are archived, not discarded.\n*.stdout.json\n*.stderr.log\n')

if __name__=='__main__':main()
