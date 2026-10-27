#!/usr/bin/env python3
"""One final all-target compile for the new public variants/configuration."""
import datetime,hashlib,json,os,pathlib,subprocess,time
here=pathlib.Path(__file__).resolve().parent
repo=here.parents[2]; scratch=repo.parent
env=os.environ.copy()
env.update(PATH=str(scratch/'tmp/rust1941/bin')+os.pathsep+env['PATH'],
 CARGO_HOME=str(scratch/'tmp/cargo_home'),CARGO_TARGET_DIR=str(scratch/'tmp/vigil_target'),CARGO_BUILD_JOBS='2')
cmd=['cargo','check','--profile','measurement','--locked','--offline','-p','vigilode','-p','vigilode-coupled','--all-targets']
start=time.monotonic()
source={str(p.relative_to(repo)):hashlib.sha256(p.read_bytes()).hexdigest()
 for prefix in ['crates/vigilode','crates/vigilode-coupled']
 for p in (repo/prefix).rglob('*.rs')}
stamp=datetime.datetime.now(datetime.timezone.utc).isoformat()
p=subprocess.run(cmd,cwd=repo,env=env,capture_output=True,text=True,timeout=240)
(here/'final_all_targets.stdout.log').write_text(p.stdout)
(here/'final_all_targets.stderr.log').write_text(p.stderr)
receipt={'command':cmd,'started_utc':stamp,'finished_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),
 'elapsed_seconds':time.monotonic()-start,'exit_code':p.returncode,'source_sha256':source,
 'scope':'cross-target compile after new enums/configuration; not a repeated benchmark'}
(here/'FINAL_ALL_TARGETS_CHECK.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps({k:v for k,v in receipt.items() if k!='source_sha256'}),flush=True)
if p.returncode: print(p.stderr[-6000:],flush=True)
raise SystemExit(p.returncode)
