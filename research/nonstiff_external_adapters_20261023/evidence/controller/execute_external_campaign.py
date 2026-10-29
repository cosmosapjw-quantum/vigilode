from pathlib import Path
import subprocess,os,json,time,hashlib
r=Path(__file__).resolve().parent;s=Path('/home/cosmosapjw/vigilode-nonstiff-adapters-source-7htbCC');study=s/'research/nonstiff_external_adapters_20261023'
sha=(r/'ADAPTER_SOURCE_COMMIT.txt').read_text().strip();p=json.loads((r/'frozen-execution/preflight/SUMMARY.json').read_text());assert p['failed']==0
b=json.loads((r/'frozen-execution/BINARY_IDENTITY.json').read_text());binary=Path(b['diffsol_binary']);assert hashlib.sha256(binary.read_bytes()).hexdigest()==b['binary_sha256']
env={**os.environ,**b['environment']};cmd=['python3',str(study/'run_external.py'),'--diffsol-adapter',str(binary),'--julia-adapter',str(study/'adapters/julia/run.sh'),'--adapter-source-commit',sha,'--out',str(r/'external-comparison')]
d={'command':cmd,'cwd':str(s),'environment':b['environment'],'started_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'source_commit':'45037fc24e8f8101605f309e1fdb3a9a4e3df56d','adapter_source_commit':sha,'preflight_summary':p,'status':'RUNNING','timing_authority':'diagnostic_only'}
(r/'EXTERNAL_EXECUTION.json').write_text(json.dumps(d,indent=2)+'\n');t=time.monotonic()
with (r/'external-campaign.stdout.log').open('w') as out,(r/'external-campaign.stderr.log').open('w') as err:res=subprocess.run(cmd,cwd=s,env=env,stdout=out,stderr=err)
d.update(exit_code=res.returncode,status='COMPLETED' if not res.returncode else 'FAILED',elapsed_process_seconds=time.monotonic()-t,ended_utc=time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()))
for name in ['external-campaign.stdout.log','external-campaign.stderr.log']:d[name+'_sha256']=hashlib.sha256((r/name).read_bytes()).hexdigest()
(r/'EXTERNAL_EXECUTION.json').write_text(json.dumps(d,indent=2)+'\n');print(json.dumps(d),flush=True)
raise SystemExit(res.returncode)
