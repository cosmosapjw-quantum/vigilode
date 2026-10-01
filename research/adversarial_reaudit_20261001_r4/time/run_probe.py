#!/usr/bin/env python3
"""Relocatable build runner; source is never edited to select absolute dependencies."""
import argparse, fcntl, json, os, pathlib, shutil, subprocess, tempfile, time
p=argparse.ArgumentParser();p.add_argument('--repo',type=pathlib.Path,required=True);p.add_argument('--output',type=pathlib.Path,required=True);a=p.parse_args()
repo=a.repo.resolve();out=a.output.resolve();out.mkdir(parents=True,exist_ok=True);src=pathlib.Path(__file__).resolve().parent/'probe'
workspace=repo.parent;lock=workspace/'runtime_r4/cargo-build.lock';lock.parent.mkdir(parents=True,exist_ok=True)
record={'audited_source':'1c54194123ee6abc6daa512e8574922f510b4e2c','head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip(),'stages':[]}
def run(stage,cmd,timeout):
 start=time.time()
 with (out/(stage+'.stdout')).open('w') as stdout,(out/(stage+'.stderr')).open('w') as stderr:
  try: code=subprocess.run(cmd,stdout=stdout,stderr=stderr,timeout=timeout).returncode
  except subprocess.TimeoutExpired: code=124
 record['stages'].append({'stage':stage,'command':cmd,'returncode':code,'seconds':time.time()-start});(out/'EXECUTION_RECEIPT.json').write_text(json.dumps(record,indent=2)+'\n');return code
with tempfile.TemporaryDirectory(prefix='r4-time-') as td:
 temp=pathlib.Path(td);shutil.copytree(src,temp/'probe');manifest=temp/'probe/Cargo.toml';txt=manifest.read_text()
 for name in ['rodas5p-core','rodas5p-integrators']:txt=txt.replace('../../../../crates/'+name,str(repo/'crates'/name))
 manifest.write_text(txt)
 with lock.open('w') as handle:
  fcntl.flock(handle,fcntl.LOCK_EX)
  code=run('native_build',['cargo','build','--offline','--manifest-path',str(manifest)],600)
 if code: raise SystemExit(code)
 target=pathlib.Path(os.environ.get('CARGO_TARGET_DIR',str(temp/'probe/target')))
 code=run('native_run',[str(target/'debug/vigilode-r4-time-probe')],120)
 shutil.copyfile(out/'native_run.stdout',out/'native_raw.jsonl')
 raise SystemExit(code)
