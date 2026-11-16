#!/usr/bin/env python3
"""Finite new supplier diagnostic allocation, separate from REI PR377 launches.
No retries, no source-owner state updates, one actual diagnostic process.
"""
import argparse,json,os,resource,signal,subprocess,time
from pathlib import Path
p=argparse.ArgumentParser()
p.add_argument('--kind',choices=['test_red','test_green','build','native'],required=True)
p.add_argument('--record',type=Path,required=True)
p.add_argument('command',nargs=argparse.REMAINDER)
a=p.parse_args();cmd=a.command[1:] if a.command[:1]==['--'] else a.command
if not cmd:raise SystemExit('EMPTY_COMMAND')
if a.record.exists():raise SystemExit('EXISTING_RECEIPT_NO_RETRY')
a.record.parent.mkdir(parents=True,exist_ok=True)
wall=30 if a.kind=='native' else 90
cpu=wall
memory=536870912 if a.kind=='native' else 3221225472
record={'kind':a.kind,'command':cmd,'pid':None,'status':'RESERVED','wall_cap_s':wall,'cpu_cap_s':cpu,'memory_cap_bytes':memory,'prior_campaign_cost':'UNKNOWN_PRESERVED','retry':False}
a.record.write_text(json.dumps(record,indent=2)+'\n')
def limits():
 os.setsid();os.sched_setaffinity(0,{min(os.sched_getaffinity(0))})
 resource.setrlimit(resource.RLIMIT_CPU,(cpu,cpu+1))
 resource.setrlimit(resource.RLIMIT_AS,(memory,memory))
 file_cap=8388608 if a.kind=='native' else 134217728
 resource.setrlimit(resource.RLIMIT_FSIZE,(file_cap,file_cap))
started=time.monotonic();usage0=resource.getrusage(resource.RUSAGE_CHILDREN)
with a.record.with_suffix('.stdout').open('wb') as out,a.record.with_suffix('.stderr').open('wb') as err:
 proc=subprocess.Popen(cmd,stdout=out,stderr=err,preexec_fn=limits,env=dict(os.environ,OMP_NUM_THREADS='1',OPENBLAS_NUM_THREADS='1',CARGO_BUILD_JOBS='1'))
 record['pid']=proc.pid;a.record.write_text(json.dumps(record,indent=2)+'\n')
 try:code=proc.wait(timeout=wall)
 except subprocess.TimeoutExpired:
  os.killpg(proc.pid,signal.SIGKILL);proc.wait();code=124
usage=resource.getrusage(resource.RUSAGE_CHILDREN)
actual_test_failure=(a.kind=='test_red' and code==101 and 'test result: FAILED' in a.record.with_suffix('.stdout').read_text(errors='replace') and 'panicked' in a.record.with_suffix('.stderr').read_text(errors='replace'))
record.update(status='EXIT0' if code==0 else 'EXPECTED_RED' if actual_test_failure else 'STOP_FAILURE',exit_code=code,wall_s=time.monotonic()-started,cpu_s=usage.ru_utime+usage.ru_stime-usage0.ru_utime-usage0.ru_stime,max_rss_kib=usage.ru_maxrss)
a.record.write_text(json.dumps(record,indent=2,sort_keys=True)+'\n')
print(json.dumps(record,sort_keys=True))
raise SystemExit(0 if record['status'] in ['EXIT0','EXPECTED_RED'] else code or 1)
