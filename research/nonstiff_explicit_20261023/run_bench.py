#!/usr/bin/env python3
"""Bounded local explicit/implicit comparison; preserves every failure and output.

No SSH, provisioning, installation or NCP calls. Optional external executable
adapters accept one JSON request on stdin and emit the documented run schema.
"""
import argparse, hashlib, importlib.util, json, os, pathlib, platform, random
import shutil, subprocess, sys, time

CASES = ["decay", "driven", "logistic", "harmonic", "rapid", "damped",
         "mixed-scales", "nonnormal", "large-vector", "stiff-control"]
DIMENSIONS = {"decay":8,"driven":8,"logistic":8,"harmonic":8,"rapid":8,
              "damped":8,"mixed-scales":12,"nonnormal":8,"large-vector":4096,
              "moderate-stiff":12,"stiff-control":12}

def write_json(path, value):
    temporary = path.with_suffix(path.suffix + '.tmp')
    temporary.write_text(json.dumps(value, indent=2) + '\n')
    temporary.replace(path)

def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--binary',required=True)
    ap.add_argument('--out',required=True)
    ap.add_argument('--source-commit',required=True)
    ap.add_argument('--full',action='store_true')
    ap.add_argument('--budget-seconds',type=float)
    ap.add_argument('--per-run-timeout',type=float,default=45.)
    ap.add_argument('--repeats',type=int,default=3)
    ap.add_argument('--seed',type=int,default=20261023)
    ap.add_argument('--cases',help='Comma-separated registered subset; default all')
    ap.add_argument('--external',action='store_true',help='Run available VODE and explicitly supplied diffsol/Julia adapters')
    ap.add_argument('--diffsol-adapter',help='Executable accepting one JSON request on stdin')
    ap.add_argument('--julia-adapter',help='Executable accepting one JSON request on stdin')
    ap.add_argument('--cpu-affinity',help='Optional comma-separated logical CPU IDs')
    args=ap.parse_args()
    if args.repeats<1 or args.per_run_timeout<=0:ap.error('positive repeats and timeout required')
    budget=args.budget_seconds if args.budget_seconds is not None else (7200. if args.full else 900.)
    if not 0<budget<=7200:ap.error('campaign budget must be in (0,7200] seconds')
    binary=pathlib.Path(args.binary).resolve()
    if not binary.is_file():ap.error('binary does not exist; build the pinned source first')
    if args.cpu_affinity:
        os.sched_setaffinity(0,{int(x) for x in args.cpu_affinity.split(',')})
    folder=pathlib.Path(args.out);folder.mkdir(parents=True,exist_ok=False)
    cases=args.cases.split(',') if args.cases else CASES + (['moderate-stiff'] if args.full else [])
    if any(c not in DIMENSIONS for c in cases):ap.error('unregistered case')
    tolerances=[1e-4,1e-6,1e-8] if not args.full else [1e-3,1e-5,1e-7,1e-9]
    configurations=[];groups=[]
    scipy_available=importlib.util.find_spec('scipy') is not None
    adapters={
        'vode':{'available':scipy_available,'command':[sys.executable,str(pathlib.Path(__file__).with_name('vode_adapter.py'))] if scipy_available else None,'family':'VODE/DVODE','language':'Python callback; Fortran VODE'},
        'diffsol':{'available':bool(args.diffsol_adapter and pathlib.Path(args.diffsol_adapter).is_file()),'command':[str(pathlib.Path(args.diffsol_adapter).resolve())] if args.diffsol_adapter else None,'family':'diffsol'},
        'julia':{'available':bool(args.julia_adapter and pathlib.Path(args.julia_adapter).is_file()),'command':[str(pathlib.Path(args.julia_adapter).resolve())] if args.julia_adapter else None,'family':'Julia','julia_executable_observed':shutil.which('julia')},
    }
    external_status={name:{**d,'status':'REGISTERED' if args.external and d['available'] else 'NOT_RUN',
                          'reason':None if args.external and d['available'] else 'not requested or adapter unavailable; no substitute method used'} for name,d in adapters.items()}
    for case in cases:
        dims=[DIMENSIONS[case]]+([16384] if args.full and case=='large-vector' else [])
        for n in dims:
            for rtol in tolerances:
                group=[];modes=['dopri5','rodas-mf' if n>=256 else 'rodas-dense']
                if args.external:modes += [name for name,d in adapters.items() if d['available']]
                for mode in modes:
                    cfg={'id':len(configurations),'case':case,'dimension':n,'rtol':rtol,'atol':rtol*1e-3,'mode':mode}
                    configurations.append(cfg);group.append(cfg['id'])
                groups.append(group)
    plan={'schema':'vigilode.nonstiff-campaign.v1','source_commit':args.source_commit,
          'binary':str(binary),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),
          'profile':'measurement','compiler_options':'opt-level=3,lto=false,codegen-units=16',
          'ncp_used':False,'platform':platform.platform(),'logical_cpus':os.cpu_count(),
          'affinity':sorted(os.sched_getaffinity(0)) if hasattr(os,'sched_getaffinity') else None,
          'cgroup_cpu_max':pathlib.Path('/sys/fs/cgroup/cpu.max').read_text().strip() if pathlib.Path('/sys/fs/cgroup/cpu.max').exists() else None,
          'budget_seconds':budget,'per_run_timeout':args.per_run_timeout,'full':args.full,
          'warmups_per_configuration':1,'timed_repeats':args.repeats,'seed':args.seed,
          'order':'randomized case/tolerance groups in every round; shuffled solver order within each group',
          'primary_metric':'max error normalized by max(1,initial magnitude,reference magnitude); 5 observed times',
          'external_comparators':external_status,'configurations':configurations}
    write_json(folder/'PLAN.json',plan)
    receipts=[];start=time.monotonic();rng=random.Random(args.seed)
    rounds=[('warmup',-1)]+[('timed',i) for i in range(args.repeats)]
    for phase,repeat in rounds:
        order=groups.copy();rng.shuffle(order)
        for group in order:
            group=group.copy();rng.shuffle(group)
            for idx in group:
                cfg=configurations[idx];stem=f'{idx:03d}_{phase}_{repeat if repeat>=0 else 0}'
                stdout=folder/(stem+'.stdout.json');stderr=folder/(stem+'.stderr.log')
                receipt={**cfg,'phase':phase,'repeat':repeat,'stdout':stdout.name,'stderr':stderr.name}
                remaining=budget-(time.monotonic()-start)
                if remaining<=0:
                    receipt.update(status='NOT_RUN_BUDGET',exit_code=None,elapsed_process_seconds=0.)
                    receipts.append(receipt);write_json(folder/'RECEIPTS.json',receipts);continue
                request={**cfg,'source_commit':args.source_commit,'max_steps':100_000}
                if cfg['mode'] in adapters:
                    command=adapters[cfg['mode']]['command'];stdin=json.dumps(request)
                else:
                    command=[str(binary),'--case',cfg['case'],'--dimension',str(cfg['dimension']),
                             '--rtol',str(cfg['rtol']),'--mode',cfg['mode'],'--source-commit',args.source_commit]
                    stdin=None
                receipt['command']=command
                before=time.monotonic()
                try:
                    with stdout.open('w') as out,stderr.open('w') as err:
                        run=subprocess.run(command,input=stdin,text=True,stdout=out,stderr=err,
                                           timeout=min(args.per_run_timeout,remaining),check=False)
                    receipt.update(status='COMPLETED' if run.returncode==0 else 'PROCESS_FAILED',exit_code=run.returncode)
                except subprocess.TimeoutExpired:
                    receipt.update(status='TIMEOUT',exit_code=None)
                except OSError as error:
                    stderr.write_text(str(error)+'\n');receipt.update(status='LAUNCH_FAILED',exit_code=None)
                receipt['elapsed_process_seconds']=time.monotonic()-before
                for key,path in [('stdout',stdout),('stderr',stderr)]:
                    if path.exists():receipt[key+'_sha256']=hashlib.sha256(path.read_bytes()).hexdigest()
                receipts.append(receipt);write_json(folder/'RECEIPTS.json',receipts)
                print(json.dumps({k:receipt[k] for k in ['id','case','mode','phase','repeat','status','elapsed_process_seconds']}),flush=True)
    completed=sum(r['status']=='COMPLETED' for r in receipts)
    write_json(folder/'CAMPAIGN.json',{'source_commit':args.source_commit,'ncp_used':False,
        'elapsed_seconds':time.monotonic()-start,'planned_configurations':len(configurations),
        'planned_runs':len(configurations)*(1+args.repeats),'completed_processes':completed,
        'other_process_outcomes':len(receipts)-completed,'receipt_count':len(receipts),
        'numerical_success_is_separate':True})
    return 0

if __name__=='__main__':raise SystemExit(main())
