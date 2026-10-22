#!/usr/bin/env python3
"""Execute only the preregistered local configurations; keep every failure."""
import argparse, hashlib, json, os, pathlib, platform, subprocess, time


def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('--binary',required=True)
    ap.add_argument('--out',required=True)
    ap.add_argument('--source-commit',required=True)
    ap.add_argument('--budget-seconds',type=float,default=900)
    args=ap.parse_args()
    out=pathlib.Path(args.out); out.mkdir(parents=True,exist_ok=False)
    binary=pathlib.Path(args.binary).resolve()
    jobs=[]
    cases=['few-mode','riccati','stiff','nonnormal','oscillatory','broad','few-mode-large']
    for case in cases:
        for tol in ['1e-5','1e-7']:
            for mode,threads in [('sequential',1),('coupled',1),('coupled',2)]:
                jobs.append(['--case',case,'--rtol',tol,'--mode',mode,'--threads',str(threads)])
    for h in ['0.2','0.1','0.05','0.025']:
        for mode in ['sequential','coupled']:
            jobs.append(['--case','riccati','--rtol','0.1','--mode',mode,'--threads','1','--max-step',h,'--refinement'])
    jobs.append(['--case','riccati','--rtol','1e-7','--mode','forced-fallback','--threads','1'])
    env=os.environ.copy();env['VIGILODE_SOURCE_COMMIT']=args.source_commit
    metadata={'schema':'vigilode.local-adaptive-campaign.v1','host_kind':'local-sandbox-not-ncp',
              'platform':platform.platform(),'logical_cpus_visible':os.cpu_count(),
              'affinity_cpu_count':len(os.sched_getaffinity(0)) if hasattr(os,'sched_getaffinity') else None,
              'cgroup_cpu_max':pathlib.Path('/sys/fs/cgroup/cpu.max').read_text().strip() if pathlib.Path('/sys/fs/cgroup/cpu.max').exists() else None,
              'binary':str(binary),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),
              'source_commit':args.source_commit,'cargo_profile':'measurement','compiler_optimization':'opt-level=3,lto=false,codegen-units=16 (repository measurement profile)',
              'budget_seconds':args.budget_seconds,
              'warmup_per_configuration':1,'timed_repeats':3,'planned_configurations':len(jobs),
              'timing_interpretation':'single local host; no controlled CPU pinning or NCP extrapolation'}
    (out/'CAMPAIGN.json').write_text(json.dumps(metadata,indent=2)+'\n')
    receipts=[];start=time.monotonic()
    for i,job in enumerate(jobs):
        command=[str(binary)]+job+['--repeats','3']
        remaining=args.budget_seconds-(time.monotonic()-start)
        rec={'index':i,'command':command}
        if remaining<=0:
            rec.update(status='NOT_RUN',reason='registered campaign time budget exhausted')
        else:
            t=time.monotonic()
            try:
                result=subprocess.run(command,env=env,capture_output=True,text=True,timeout=min(remaining,120))
                rec.update(status='COMPLETED' if result.returncode==0 else 'FAILED',exit_code=result.returncode)
                stdout,stderr=result.stdout,result.stderr
            except subprocess.TimeoutExpired as error:
                rec.update(status='TIMEOUT',exit_code=None)
                stdout=error.stdout or '';stderr=error.stderr or ''
                if isinstance(stdout,bytes):stdout=stdout.decode(errors='replace')
                if isinstance(stderr,bytes):stderr=stderr.decode(errors='replace')
            rec['elapsed_seconds']=time.monotonic()-t
            (out/f'{i:03d}.stdout.json').write_text(stdout)
            (out/f'{i:03d}.stderr.log').write_text(stderr)
        receipts.append(rec)
        (out/'RECEIPTS.json').write_text(json.dumps(receipts,indent=2)+'\n')
        print(json.dumps({'index':i,'status':rec['status'],'elapsed_seconds':rec.get('elapsed_seconds')}),flush=True)
    metadata['elapsed_seconds']=time.monotonic()-start
    metadata['completed_configurations']=sum(r['status']=='COMPLETED' for r in receipts)
    metadata['failed_or_unrun_configurations']=sum(r['status']!='COMPLETED' for r in receipts)
    (out/'CAMPAIGN.json').write_text(json.dumps(metadata,indent=2)+'\n')

if __name__=='__main__':main()
