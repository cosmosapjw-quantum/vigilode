#!/usr/bin/env python3
"""Targeted structural contracts and fixed, pre-registered complete trajectories."""
import datetime, hashlib, json, os, subprocess, time
from pathlib import Path
here=Path(__file__).resolve().parent
repo=here.parents[2]
scratch=repo.parent
env=os.environ.copy()
env.update(PATH=str(scratch/"tmp/rust1941/bin")+os.pathsep+env["PATH"],
 CARGO_HOME=str(scratch/"tmp/cargo_home"), CARGO_TARGET_DIR=str(scratch/"tmp/vigil_target"),
 CARGO_BUILD_JOBS="2", RAYON_NUM_THREADS="1", OMP_NUM_THREADS="1", OPENBLAS_NUM_THREADS="1")
binary=scratch/"tmp/vigil_target/measurement/examples/structural_mean_field_bench_20261027"
commands=[
 ("structural_contracts",["cargo","test","--profile","measurement","--locked","--offline","-p","vigilode-coupled","--test","structural_mean_field_contracts","--test","switching_contracts","--","--test-threads=1","--nocapture"]),
 ("structural_build",["cargo","build","--profile","measurement","--locked","--offline","-p","vigilode-coupled","--example","structural_mean_field_bench_20261027"]),
 ("structural_n64_t1",[str(binary),"64","64","5","1"]),
 ("structural_n2048_t1",[str(binary),"2048","64","5","1"]),
 ("structural_n2048_t2",[str(binary),"2048","64","5","2"])]
receipt={"started_utc":datetime.datetime.now(datetime.timezone.utc).isoformat(),"source":{},"commands":[]}
for p in ["crates/vigilode-coupled/src/structural_mean_field.rs","crates/vigilode-coupled/src/switching.rs","crates/vigilode-coupled/tests/structural_mean_field_contracts.rs","crates/vigilode-coupled/tests/switching_contracts.rs","crates/vigilode-coupled/examples/structural_mean_field_bench_20261027.rs"]:
 receipt["source"][p]=hashlib.sha256((repo/p).read_bytes()).hexdigest()
for name,cmd in commands:
 start=time.monotonic()
 p=subprocess.run(cmd,cwd=repo,env=env,capture_output=True,text=True,timeout=240)
 (here/(name+".stdout.log")).write_text(p.stdout)
 (here/(name+".stderr.log")).write_text(p.stderr)
 item={"name":name,"command":cmd,"exit_code":p.returncode,"elapsed_seconds":time.monotonic()-start}
 receipt["commands"].append(item)
 (here/"STRUCTURAL_EXECUTION.json").write_text(json.dumps(receipt,indent=2)+"\n")
 print(json.dumps(item),flush=True)
 if p.returncode:
  print(p.stdout[-5000:]+p.stderr[-5000:],flush=True)
  raise SystemExit(p.returncode)
 if name.startswith("structural_n"):
  data=json.loads(p.stdout)
  (here/(name+".json")).write_text(json.dumps(data,indent=2)+"\n")
receipt["binary_sha256"]=hashlib.sha256(binary.read_bytes()).hexdigest()
receipt["finished_utc"]=datetime.datetime.now(datetime.timezone.utc).isoformat()
(here/"STRUCTURAL_EXECUTION.json").write_text(json.dumps(receipt,indent=2)+"\n")
