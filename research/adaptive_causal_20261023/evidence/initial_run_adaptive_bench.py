#!/usr/bin/env python3
"""Frozen native single-thread endpoint-window campaign; preserves every failure."""
import argparse,gzip,hashlib,json,os,pathlib,platform,random,subprocess,time
HERE=pathlib.Path(__file__).resolve().parent
ROOT=HERE.parents[2]
MODELS=[('decay',1),('logistic',1),('prothero',1),('robertson',3),('rapid',2),('damped',2),('nonnormal',2),('transition',1),('vector',256)]
MODES=['dopri5','rodas-fd','rodas-jvp','switch-fd','switch-jvp','dopri-whole','dopri-whole-dense','rodas-whole-fd']
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def dump(p,x):p.write_text(json.dumps(x,indent=2,allow_nan=False)+'\n')
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True);p.add_argument('--out',required=True);p.add_argument('--source-commit',required=True);p.add_argument('--repeats',type=int,default=3);p.add_argument('--budget-seconds',type=float,default=900);p.add_argument('--timeout-seconds',type=float,default=30);p.add_argument('--cpu',type=int);p.add_argument('--cases');p.add_argument('--tolerances',default='1e-4,1e-6,1e-8');a=p.parse_args()
 if a.repeats<3 or not 0<a.budget_seconds<=900:p.error('repeats>=3 and0<budget<=900 required')
 binary=pathlib.Path(a.binary).resolve();out=pathlib.Path(a.out).resolve();out.mkdir(parents=True,exist_ok=False)
 sha=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
 if sha!=a.source_commit:raise SystemExit('source commit mismatch')
 # Native code must be frozen; report writing may continue outside crates.
 if subprocess.run(['git','diff','--quiet',sha,'--','Cargo.toml','Cargo.lock','crates'],cwd=ROOT).returncode:raise SystemExit('native source differs from commit')
 if hasattr(os,'sched_setaffinity'):
  allowed=sorted(os.sched_getaffinity(0));cpu=a.cpu if a.cpu is not None else allowed[0];os.sched_setaffinity(0,{cpu})
 env=os.environ.copy();env.update(RAYON_NUM_THREADS='1',OMP_NUM_THREADS='1',OPENBLAS_NUM_THREADS='1',MKL_NUM_THREADS='1')
 selected=set(a.cases.split(',')) if a.cases else {n for n,_ in MODELS};tols=[float(v)for v in a.tolerances.split(',')]
 configs=[];groups=[]
 for case,n in MODELS:
  if case not in selected:continue
  for tol in tols:
   group=[]
   for mode in MODES:
    cfg=dict(id=len(configs),case=case,dimension=n,mode=mode,rtol=tol,windows=16)
    configs.append(cfg);group.append(cfg['id'])
   groups.append(group)
 plan=dict(schema='vigilode.adaptive-causal-campaign.v1',source_commit=sha,source_tree=subprocess.check_output(['git','rev-parse','HEAD^{tree}'],cwd=ROOT,text=True).strip(),binary=str(binary),binary_sha256=digest(binary),benchmark_source_sha256=digest(ROOT/'crates/vigilode/examples/adaptive_causal_bench.rs'),orchestrator_sha256=digest(pathlib.Path(__file__)),analyzer_sha256=digest(HERE/'analyze_adaptive_bench.py'),profile='measurement',compiler_flags='opt-level=3,lto=false,codegen-units=16',host=platform.node(),platform=platform.platform(),affinity=sorted(os.sched_getaffinity(0)),thread_settings={k:env[k]for k in ['RAYON_NUM_THREADS','OMP_NUM_THREADS','OPENBLAS_NUM_THREADS','MKL_NUM_THREADS']},budget_seconds=a.budget_seconds,timeout_seconds=a.timeout_seconds,warmups=1,repeats=a.repeats,configurations=configs,ncp_used=False,output_contract='17 common observation times. Five window modes restart; dopri-whole uses retainedworkspace landing, dopri-whole-dense quartic observations, rodas-whole-fd retaineddenseoutput WFilteredEnforce. Switching can add partialaccepted endpoints',comparison='same binary, host, thread affinity, observation contract and common achieved-error caps; no old timing pooled',accuracy_caps=[1e-4,1e-6,1e-8])
 dump(out/'PLAN.json',plan);started=time.monotonic();rng=random.Random(20261023);receipts=[];raw=out/'RAW_RUNS.jsonl'
 with raw.open('w') as f:
  for phase,repeat in [('warmup',-1)]+[('timed',i)for i in range(a.repeats)]:
   gs=groups.copy();rng.shuffle(gs)
   for group in gs:
    idxs=group.copy();rng.shuffle(idxs)
    for idx in idxs:
     cfg=configs[idx];cmd=[str(binary),'--case',cfg['case'],'--mode',cfg['mode'],'--dimension',str(cfg['dimension']),'--rtol',str(cfg['rtol']),'--windows',str(cfg['windows']),'--source-commit',sha]
     receipt=dict(**cfg,phase=phase,repeat=repeat,command=cmd);remaining=a.budget_seconds-(time.monotonic()-started);before=time.monotonic()
     if remaining<=0:receipt.update(status='NOT_RUN_BUDGET',exit_code=None,stdout='',stderr='')
     else:
      try:
       r=subprocess.run(cmd,cwd=ROOT,env=env,text=True,capture_output=True,timeout=min(a.timeout_seconds,remaining));receipt.update(status='COMPLETED'if r.returncode==0 else'PROCESS_FAILED',exit_code=r.returncode,stdout=r.stdout,stderr=r.stderr)
      except subprocess.TimeoutExpired as e:receipt.update(status='TIMEOUT',exit_code=None,stdout=(e.stdout or b'').decode()if isinstance(e.stdout,bytes)else(e.stdout or''),stderr=(e.stderr or b'').decode()if isinstance(e.stderr,bytes)else(e.stderr or''))
     receipt['process_seconds']=time.monotonic()-before;receipt['stdout_sha256']=hashlib.sha256(receipt['stdout'].encode()).hexdigest();receipt['stderr_sha256']=hashlib.sha256(receipt['stderr'].encode()).hexdigest();f.write(json.dumps(receipt,allow_nan=False)+'\n');f.flush();receipts.append({k:v for k,v in receipt.items()if k not in ['stdout','stderr']})
   print(json.dumps(dict(phase=phase,repeat=repeat,records=len(receipts),elapsed=time.monotonic()-started)),flush=True)
 with (out/'RAW_RUNS.jsonl.gz').open('wb')as f:
  with gzip.GzipFile(fileobj=f,mode='wb',mtime=0)as g:g.write(raw.read_bytes())
 raw.unlink();dump(out/'RECEIPTS.json',receipts)
 dump(out/'EXECUTION.json',dict(status='completed',process_count=len(receipts),elapsed_seconds=time.monotonic()-started,binary_unchanged=digest(binary)==plan['binary_sha256'],raw_gzip_sha256=digest(out/'RAW_RUNS.jsonl.gz')))
if __name__=='__main__':main()
