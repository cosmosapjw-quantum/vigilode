#!/usr/bin/env python3
"""Append measured new D4 subprocesses; closed ledgers are never reopened."""
import argparse,datetime,hashlib,json,subprocess,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('name');p.add_argument('--timeout-seconds',type=float,default=45);p.add_argument('command',nargs=argparse.REMAINDER);a=p.parse_args()
root=Path(__file__).resolve().parent
path=root/'BUDGET.json';b=json.loads(path.read_text())
if b['closed']:raise SystemExit('closed ledger; refused')
cmd=a.command
if cmd and cmd[0]=='--':cmd=cmd[1:]
if not cmd:raise SystemExit('no command')
log=root/'evidence'/(a.name+'.log')
if log.exists():raise SystemExit('existing execution name; refused')
remaining=b['cap_seconds']-b['total_seconds']
if remaining<=0:raise SystemExit('budget exhausted')
timeout=min(a.timeout_seconds,remaining)
start=datetime.datetime.now(datetime.timezone.utc).isoformat();t=time.perf_counter()
try:
 r=subprocess.run(cmd,cwd=root,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=timeout)
 out=r.stdout;code=r.returncode;reason=None
except subprocess.TimeoutExpired as e:
 out=(e.stdout or b'')+b'\nTIMEOUT\n';code=124;reason='timeout'
elapsed=time.perf_counter()-t
log.write_bytes(out)
record={'name':a.name,'cmd':cmd,'started_utc':start,'seconds':elapsed,'exit':code,'timeout_seconds':timeout,'log':str(log.relative_to(root)),'sha256':hashlib.sha256(out).hexdigest(),'reason':reason}
b['entries'].append(record);b['total_seconds']+=elapsed
path.write_text(json.dumps(b,indent=2)+'\n')
print(json.dumps(record));print(out.decode(errors='replace'))
raise SystemExit(code)
