from pathlib import Path
import subprocess,json,datetime,hashlib,importlib,platform,os
root=Path(__file__).resolve().parents[3];rt=root/'runtime_r4';ev=Path(__file__).resolve().parent;p=rt/'probes';p.mkdir(exist_ok=True);(p/'hello.rs').write_text('fn main() { println!("RUST_EXECUTION_PASS {}", 6 * 7); }\n')
rows=[]
for args in [['rustc','--version','--verbose'],['cargo','--version'],['rustfmt','--version'],['rustc',str(p/'hello.rs'),'-o',str(p/'hello')],[str(p/'hello')],['flock',str(rt/'cargo-build.lock'),'cargo','metadata','--locked','--offline','--format-version','1','--manifest-path',str(root/'vigilode/Cargo.toml')]]:
 r=subprocess.run(args,cwd=root/'vigilode',capture_output=True,text=True);row={'argv':args,'exit_code':r.returncode,'stdout':r.stdout,'stderr':r.stderr}
 if 'metadata' in args:
  (ev/'cargo_metadata.json').write_text(r.stdout);row['stdout']='cargo_metadata.json';row['packages']=len(json.loads(r.stdout)['packages']) if r.returncode==0 else None
 rows.append(row)
 if r.returncode:break
py={'version':platform.python_version(),'packages':{}}
for name in ['numpy','scipy','matplotlib','mpmath','sympy']:
 try:mod=importlib.import_module(name);py['packages'][name]={'status':'IMPORT_PASS','version':mod.__version__}
 except ImportError as e:py['packages'][name]={'status':'MISSING','error':str(e)}
(ev/'python_host_preflight.json').write_text(json.dumps(py,indent=2)+'\n')
r={'schema':'vigilode.r4.runtime_setup.v1','utc':datetime.datetime.now(datetime.UTC).isoformat(),'status':'PASS' if len(rows)==6 and all(x['exit_code']==0 for x in rows) else 'FAIL','commands':rows,'memory_max':Path('/sys/fs/cgroup/memory.max').read_text().strip(),'cpu_max':Path('/sys/fs/cgroup/cpu.max').read_text().strip(),'source_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=root/'vigilode',text=True).strip(),'prior_pass_inherited':False,'source_mutation':False,'installation_incidents':[{'classification':'RUNTIME_ENVIRONMENT','stage':'vendor extraction','observed':'tar ownership uid/gid 1000 invalid argument','first_failure_log':'vendor_extract.initial_ownership_error.stderr','repair':'--no-same-owner','repaired_exit':0}],'dependency_receipt':'dependency_match.json'}
(ev/'setup_evidence.json').write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(r,indent=2));raise SystemExit(0 if r['status']=='PASS' else 1)
