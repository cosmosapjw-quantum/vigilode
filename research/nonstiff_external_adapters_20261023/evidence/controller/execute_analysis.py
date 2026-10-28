from pathlib import Path
import subprocess,json,time,hashlib
r=Path(__file__).resolve().parent;s=Path('/home/cosmosapjw/vigilode-nonstiff-adapters-source-7htbCC');campaign=json.loads((r/'external-comparison/CAMPAIGN.json').read_text());assert campaign['receipt_count']==384
jobs=[('analysis',['python3',str(s/'research/nonstiff_external_adapters_20261023/analyze_external.py'),'--baseline','/home/cosmosapjw/vigilode-nonstiff-run-Ioh017/full-comparison','--external',str(r/'external-comparison'),'--merged-out',str(r/'merged-comparison')],'ANALYSIS_COMMAND.json'),('final-checks',['python3',str(r/'capture_final_checks.py')],'FINAL_CHECKS_COMMAND.json'),('report-generation',['python3',str(r/'write_completion_report.py')],'REPORT_GENERATION_COMMAND.json')]
for label,command,name in jobs:
 meta={'command':command,'cwd':str(s),'started_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'stage':'postprocessing; no solver executions'};t=time.monotonic()
 with (r/(label+'.stdout.log')).open('w') as out,(r/(label+'.stderr.log')).open('w') as err:p=subprocess.run(command,cwd=s,stdout=out,stderr=err)
 meta.update(exit_code=p.returncode,elapsed_seconds=time.monotonic()-t)
 for k in ['stdout','stderr']:
  path=r/(label+'.'+k+'.log');meta[k]=path.name;meta[k+'_sha256']=hashlib.sha256(path.read_bytes()).hexdigest()
 (r/name).write_text(json.dumps(meta,indent=2)+'\n');print(json.dumps(meta),flush=True)
 if p.returncode:raise SystemExit(p.returncode)
