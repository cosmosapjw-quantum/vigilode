#!/usr/bin/env python3
"""Reproduce changed numerical contracts with locked dependencies; no network/NCP."""
import argparse, hashlib, json, os, pathlib, re, subprocess, time
p=argparse.ArgumentParser();p.add_argument('--output', required=True);a=p.parse_args()
out=pathlib.Path(a.output);out.mkdir(parents=True,exist_ok=False)
env=os.environ.copy();env.setdefault('CARGO_BUILD_JOBS','2');env.setdefault('CARGO_PROFILE_DEV_DEBUG','0');env.setdefault('CARGO_PROFILE_TEST_DEBUG','0')
commands=[['cargo','test','--locked','--offline','-p','vigilode-coupled'],['cargo','test','--locked','--offline','-p','rodas5p-integrators','--lib','adaptive_output_session']]
results=[]
for i,cmd in enumerate(commands):
 t=time.monotonic();log=out/f'{i:02d}.log'
 with log.open('w') as f:r=subprocess.run(cmd,env=env,stdout=f,stderr=subprocess.STDOUT)
 text=log.read_text();counts=[tuple(map(int,m)) for m in re.findall(r'test result: .*?\. (\d+) passed; (\d+) failed;',text)]
 results.append({'command':cmd,'exit_code':r.returncode,'seconds':time.monotonic()-t,'passed':sum(v[0] for v in counts),'failed':sum(v[1] for v in counts),'log':log.name,'log_sha256':hashlib.sha256(log.read_bytes()).hexdigest()})
 print(json.dumps(results[-1]),flush=True)
summary={'source_commit':subprocess.check_output(['git','rev-parse','HEAD']).decode().strip(),'rustc':subprocess.check_output(['rustc','--version'],env=env).decode().strip(),'cargo_lock_sha256':hashlib.sha256(pathlib.Path('Cargo.lock').read_bytes()).hexdigest(),'ncp_used':False,'results':results,'all_passed':all(r['exit_code']==0 and r['passed']>0 and r['failed']==0 for r in results)}
(out/'SUMMARY.json').write_text(json.dumps(summary,indent=2)+'\n')
raise SystemExit(0 if summary['all_passed'] else 1)
