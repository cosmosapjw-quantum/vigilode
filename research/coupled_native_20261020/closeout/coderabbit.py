"""Real CodeRabbit prerequisite/review attempt. Never publish auth payloads."""
import json,os,re,subprocess,sys,tempfile
from pathlib import Path
out=Path(sys.argv[1]);out.mkdir(parents=True,exist_ok=True)
os.environ['PATH']=str(Path.home()/'.local/bin')+os.pathsep+os.environ['PATH']
def run(cmd,seconds):
 try:
  p=subprocess.run(cmd,capture_output=True,text=True,timeout=seconds);return p.returncode,p.stdout,p.stderr
 except subprocess.TimeoutExpired as e:return 124,(e.stdout or b'').decode(errors='replace')if isinstance(e.stdout,bytes)else(e.stdout or''),'timeout'
code,version,_=run(['bash','-lc','command -v coderabbit && coderabbit --version'],20)
installer=None
if code:
 with tempfile.TemporaryDirectory()as t:
  path=str(Path(t)/'install.sh');c,_,_=run(['curl','-fsSL','--max-time','30','https://cli.coderabbit.ai/install.sh','-o',path],35)
  if c==0:c,_,_=run(['bash',path],90)
  installer=c
 code,version,_=run(['bash','-c','coderabbit --version'],20)
status={'cli_exit':code,'installer_exit':installer,'authenticated':False,'login_attempted':False,'review_executed':False,'issues':None}
def detect(text):
 if re.search(r'not\s+(?:authenticated|logged\s+in)|unauthenticated',text,re.I):return False
 found=[]
 def walk(x):
  if isinstance(x,dict):
   for k,v in x.items():
    if k.lower().replace('_','')in ('authenticated','isauthenticated','loggedin','isloggedin')and isinstance(v,bool):found.append(v)
    if k.lower()=='status'and isinstance(v,str)and v.lower()in ('authenticated','logged-in','logged_in'):found.append(True)
    walk(v)
  elif isinstance(x,list):
   for a in x:walk(a)
 for l in text.splitlines():
  try:walk(json.loads(l))
  except ValueError:pass
 return any(found)and not any(v is False for v in found)
if code==0:
 a,auth,err=run(['coderabbit','auth','status','--agent'],20);status['auth_status_exit']=a
 status['authenticated']=detect(auth+'\n'+err)
 if not status['authenticated']:
  status['login_attempted']=True
  login,_,_=run(['coderabbit','auth','login','--agent'],30);status['login_exit']=login
  a,auth,err=run(['coderabbit','auth','status','--agent'],20);status['auth_recheck_exit']=a;status['authenticated']=detect(auth+'\n'+err)
 # Exit0 is not assumed to mean authenticated; the prior prerequisite used only
 # exit codes, and this semantic check explicitly corrects that interpretation.
 if status['authenticated']:
  status['review_executed']=True
  c,stdout,stderr=run(['coderabbit','review','--agent','-t','committed','--base-commit','c03ca90f19a9e64ba7751c069325a48b54b39c2e'],600)
  (out/'CODERABBIT_REVIEW.ndjson').write_text(stdout)
  (out/'CODERABBIT_REVIEW.stderr').write_text(stderr)
  events=[]
  for line in stdout.splitlines():
   try:events.append(json.loads(line))
   except ValueError:pass
  findings=[e for e in events if e.get('type')=='finding'];errors=[e for e in events if e.get('type')=='error']
  status.update(review_exit=c,events=len(events),error_events=len(errors),issues=len(findings)if c==0 and events and not errors else None,
   status='REVIEW_RETURNED_NEEDS_RESULT_READBACK'if c==0 and events and not errors else'REVIEW_FAILED_OR_TIMED_OUT')
 else:status['status']='BLOCKED_AUTHENTICATION_NOT_CONFIRMED'
else:status['status']='BLOCKED_CLI_INSTALLATION'
status['authentication_output_published']=False
(out/'CODERABBIT_STATUS.json').write_text(json.dumps(status,indent=2)+'\n')
print(json.dumps(status))
