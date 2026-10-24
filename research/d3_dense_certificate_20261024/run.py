from pathlib import Path
import subprocess,time,json,hashlib,sys,datetime
R=Path(__file__).resolve().parent
p=R/'BUDGET.json'
if p.exists(): b=json.loads(p.read_text())
else:
 b={'unit':'D3-dense-certificate-resume','cap_seconds':120,'entries':[],'closed':False,'prior_known_seconds_at_S0':108.40006170199984,'prior_D2_closed_seconds':25.104395919000012,'prior_pre_D2_known_seconds':83.29566578299983,'separate_native_compile_boundary_closed_seconds':14.951735806986107,'older_remaining':None,'reset':False,'scope':'Only new local subprocess commands through this runner; retrieval/drafting/storage/external CAS separate.'}
assert not b['closed'], 'closed budget must not be reset'
args=sys.argv[1:]
step_timeout=45.0
if args[:1]==['--timeout-seconds']:
 step_timeout=float(args[1]);args=args[2:]
 assert 0 < step_timeout <= 120, 'timeout must remain inside the fixed aggregate cap'
name=args[0];cmd=args[1:]
assert name and not any(e['name']==name for e in b['entries']), 'unique log name required'
remaining=b['cap_seconds']-sum(x['seconds'] for x in b['entries'])
if remaining<=0: raise SystemExit('BUDGET_EXHAUSTED')
t=time.monotonic();started=datetime.datetime.now(datetime.timezone.utc).isoformat()
try:
 r=subprocess.run(cmd,cwd=R,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=min(step_timeout,remaining),text=True)
 out=r.stdout;rc=r.returncode
except subprocess.TimeoutExpired as e:
 out=(e.stdout or b'').decode() if isinstance(e.stdout,bytes) else(e.stdout or '');out+='\nTIMEOUT\n';rc=124
elapsed=time.monotonic()-t;log=R/'evidence'/f'{name}.log';log.write_text(out)
b['entries'].append({'name':name,'cmd':cmd,'started_utc':started,'seconds':elapsed,'timeout_seconds':min(step_timeout,remaining),'exit':rc,'log':str(log.relative_to(R)),'sha256':hashlib.sha256(log.read_bytes()).hexdigest()})
b['total_seconds']=sum(x['seconds'] for x in b['entries']);p.write_text(json.dumps(b,indent=2)+'\n')
print(out[-7000:]);print('RECORDED',name,'exit',rc,'seconds',elapsed,'total',b['total_seconds'])
sys.exit(rc)
