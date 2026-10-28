#!/usr/bin/env python3
import datetime,hashlib,json,os,subprocess,time
from pathlib import Path
here=Path(__file__).resolve().parent; repo=here.parents[2]; scratch=repo.parent
env=os.environ.copy(); env.update(PATH=str(scratch/"tmp/rust1941/bin")+os.pathsep+env["PATH"],
 CARGO_HOME=str(scratch/"tmp/cargo_home"),CARGO_TARGET_DIR=str(scratch/"tmp/vigil_target"),CARGO_BUILD_JOBS="2",
 RAYON_NUM_THREADS="1",OMP_NUM_THREADS="1",OPENBLAS_NUM_THREADS="1")
binary=scratch/"tmp/vigil_target/measurement/examples/selective_jvp_20261027"
source=repo/"crates/vigilode/examples/selective_jvp_20261027.rs"
receipt={"started_utc":datetime.datetime.now(datetime.timezone.utc).isoformat(),"source_sha256":hashlib.sha256(source.read_bytes()).hexdigest(),"commands":[]}
for name,cmd in [("build",["cargo","build","--profile","measurement","--locked","--offline","-p","vigilode","--example","selective_jvp_20261027"]),("experiment",[str(binary)])]:
 start=time.monotonic(); p=subprocess.run(cmd,cwd=repo,env=env,capture_output=True,text=True,timeout=240)
 (here/(name+".stdout.log")).write_text(p.stdout); (here/(name+".stderr.log")).write_text(p.stderr)
 item={"name":name,"command":cmd,"exit_code":p.returncode,"elapsed_seconds":time.monotonic()-start}
 receipt["commands"].append(item); (here/"EXECUTION_RECEIPT.json").write_text(json.dumps(receipt,indent=2)+"\n")
 print(json.dumps(item),flush=True)
 if p.returncode: print(p.stdout[-2000:]+p.stderr[-3000:],flush=True);raise SystemExit(p.returncode)
 if name=="experiment": (here/"RESULTS.json").write_text(json.dumps(json.loads(p.stdout),indent=2)+"\n")
receipt["binary_sha256"]=hashlib.sha256(binary.read_bytes()).hexdigest()
(here/"EXECUTION_RECEIPT.json").write_text(json.dumps(receipt,indent=2)+"\n")
