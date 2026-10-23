"""Independent direct full33 Fraction oracle; exact f64 bits in/out, no float scores."""
from fractions import Fraction as F
from pathlib import Path
import hashlib,itertools,json,math,random,struct,subprocess
R=Path(__file__).resolve().parent
rng=random.Random(20261024)
def bits(x):return struct.pack('>d',x).hex()
def val(x):return struct.unpack('>d',bytes.fromhex(x))[0]
def p2(k):return F(2)**k
def direct(e,s):
 e=list(map(F.from_float,e));s=list(map(F.from_float,s))
 full=e[:23]+[-e[23],e[23],-e[24]-e[25],e[24],e[25],-e[26]-e[27]-e[28],e[26],e[27],e[28],e[29]]
 return sum((a/b)**2 for a,b in zip(full,s))/33
MAX=float.fromhex('0x1.fffffffffffffp1023');TINY=float.fromhex('0x0.0000000000001p-1022')
values=[-MAX,-1.,-TINY,0.,TINY,1.,MAX]
cases=[]
for v in itertools.product(values,repeat=3):cases.append(('S',list(v),abs(sum(map(F.from_float,v)))))
for n in range(256):
 e=[];s=[]
 for _ in range(30):
  b=rng.getrandbits(64)
  if ((b>>52)&2047)==2047:b^=1<<52
  e.append(struct.unpack('>d',b.to_bytes(8,'big'))[0])
 for _ in range(33):
  b=rng.getrandbits(63)
  if ((b>>52)&2047)==2047:b^=1<<52
  s.append(struct.unpack('>d',max(b,1).to_bytes(8,'big'))[0])
 cases.append(('N',e+s,direct(e,s)))
# Near threshold, frozen offsets both below and above sqrt33.
a=math.sqrt(33.)
for steps in range(-64,65):
 x=a
 for _ in range(abs(steps)):x=math.nextafter(x,math.inf if steps>=0 else -math.inf)
 e=[0.]*30;e[0]=x;s=[1.]*33;cases.append(('N',e+s,direct(e,s)))
# Controlled common dyadic scale spans, with useful finite normalized norms.
for k in [-1074,-1022,-700,-1,0,400,900,1023]:
 scale=math.ldexp(1.,k);e=[scale]*30;s=[scale]*33;cases.append(('N',e+s,direct(e,s)))
# Overflow raw sum and correlated decisions, zero, exact mixed-exponent residual.
for idx,vals,scale in [(24,[1e308,1e308],1e308),(24,[2.5,2.5],1.),(24,[2.5,-2.5],1.),(26,[MAX,-MAX,TINY],MAX),(0,[0.],1.)]:
 e=[0.]*30;e[idx:idx+len(vals)]=vals;s=[scale]*33;cases.append(('N',e+s,direct(e,s)))
text=''.join(mode+' '+' '.join(map(bits,x))+'\n' for mode,x,_ in cases)
(R/'evidence/ORACLE_INPUT.txt').write_text(text)
r=subprocess.run([str(R/'probe')],input=text,capture_output=True,text=True,timeout=15)
(R/'evidence/ORACLE_OUTPUT.txt').write_text(r.stdout)
(R/'evidence/ORACLE_STDERR.txt').write_text(r.stderr)
assert r.returncode==0,r.stderr
lines=r.stdout.splitlines();assert len(lines)==len(cases)
counts={'Accept':0,'Reject':0,'Indeterminate':0,'S':0};records=[];max_relative_width=F(0)
for i,((mode,x,exact),line) in enumerate(zip(cases,lines)):
 a,b,k,status=line.split();lo=F.from_float(val(a))*p2(int(k));hi=F.from_float(val(b))*p2(int(k))
 assert lo<=exact<=hi,(i,mode,line,'NOT_ENCLOSED')
 if mode=='N':
  assert not(status=='Accept' and exact>1),(i,'FALSE_ACCEPT')
  assert not(status=='Reject' and exact<=1),(i,'FALSE_REJECT')
  if exact: max_relative_width=max(max_relative_width,(hi-lo)/exact)
 counts[status]+=1
 records.append({'case':i,'kind':mode,'exact_num':str(exact.numerator),'exact_den':str(exact.denominator),'enclosed':True,'decision':status})
(R/'evidence/ORACLE_EXACT.json').write_text(json.dumps(records,separators=(',',':'))+'\n')
summary={'status':'PASS_EXACT_FRACTION_CONTAINMENT','cases':len(cases),'counts':counts,'false_accept':0,'false_reject':0,
 'max_relative_width':float(max_relative_width),'seed':20261024,'input_sha256':hashlib.sha256(text.encode()).hexdigest(),
 'scope':'fixed PR79 +/-1 lift; finite corpus not exhaustive theorem; independent arithmetic path same author',
 'native_ODE_runs':0}
(R/'ORACLE_SUMMARY.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary))
