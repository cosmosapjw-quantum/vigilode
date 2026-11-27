from pathlib import Path
from fractions import Fraction as Q
import gzip,json,collections,math
P=Path(__file__).parent
PRIOR=P.parent/'m2_cost_aware_early_krylov_20261127'
def read(p,m):
 if not p.exists():p=Path(str(p)+'.gz')
 s=gzip.decompress(p.read_bytes()).decode()if p.suffix=='.gz'else p.read_text()
 return [json.loads(l.split(m+' ',1)[1])for l in s.splitlines()if l.startswith(m+' {')or l.startswith('test ')and ' ... '+m+' {'in l]
def dump(n,o):(P/n).write_text(json.dumps(o,indent=2,sort_keys=True,allow_nan=False)+'\n')
def req(a):return sum(a[k]for k in ('alloc','zeroed','realloc'))
def up(x):
 v=float(x);return math.nextafter(v,math.inf)if Q(v)<x else v
def pt(x):return (Q(x),Q(x))
def dot_box(box,v):
 lo=hi=Q(0)
 for (a,b),c in zip(box,v):lo+=min(a*c,b*c);hi+=max(a*c,b*c)
 return lo,hi
def error(v,box):return max(max(abs(Q(x)-a),abs(Q(x)-b))for x,(a,b)in zip(v,box))
def encl(pair,box,label):
 a,b=map(Q,pair);l,h=box
 assert a<=l<=h<=b,(label,float(a),float(l),float(h),float(b))
def log1p(a):
 assert 0<=a<1
 s=Q(0);v=a
 for k in range(1,81):
  s+=v/k;v*=-a
  if k==79:hi=s
 return s,hi
def state(case,n,t):
 if case=='quadratic':
  z=Q(1,2)+t/16;x=z*z-Q(1,4);y=Q(1,16)+t*t/32+t**3/768
  return list(map(pt,[x,y,z,y]))
 s=[]
 for j in range(n//2):
  a=Q(j+1);v=a*t*t/8;d=1+v
  l,h=log1p(v);s.extend([pt(Q(1,4)/d),(Q(1,16)+l/a,Q(1,16)+h/a)])
 return s+s
def exp_pos(t):
 term=s=Q(1)
 for k in range(1,151):term*=t/k;s+=term
 tail=term*t/151/(1-t/152)
 return s,s+tail
def exact_state(case,n,t):
 if case=='growth2':
  l,h=exp_pos(8*t);l/=1024;h/=1024;st=[(l,h),((l-Q(1,1024))/8,(h-Q(1,1024))/8)]
  return st+st
 if case=='coupled_clock':
  x=Q(1,4)/(1+t*t/8);l,h=log1p(t*t/8);y=(Q(1,16)+l,Q(1,16)+h);st=[(x+y[0],x+y[1]),y]
  return st+st
 return state(case,n,t)
rows=read(P/'goal.log','ESC_GOAL');out=[]
for r in rows:
 a=r['attempts'][-1];budget=Q(r['absolute'])
 if not a['physical_qualified']:status='PHYSICAL_CERTIFICATE_UNRESOLVED';lo=hi=None
 else:
  w=list(map(Q,r['weights']));j=dot_box(exact_state(r['case'],r['n'],Q(r['end'])),w)
  num=sum(Q(x)*v for x,v in zip(a['terminal_state'],w));lo,hi=j[0]-num,j[1]-num
  if r['qualified']:status='CERTIFIED_ACCEPT'
  elif lo>budget or hi<-budget:status='ACTUAL_GOAL_ERROR_EXCEEDS'
  elif max(abs(lo),abs(hi))<=budget:status='ENCLOSURE_TOO_WIDE'
  else:status='REFERENCE_BOUNDARY_UNRESOLVED'
 out.append(dict(case=r['case'],end=r['end'],budget=r['budget'],candidate=r['candidate'],status=status,actual_error=[str(lo),str(hi)]if lo is not None else None,actual_error_float=[float(lo),float(hi)]if lo is not None else None))
dump('GOAL_FAILURE_CLASSIFICATION.json',out)
print('GOAL CLASS',dict(collections.Counter(r['status']for r in out)))
for k in dict.fromkeys((r['case'],r['end'],r['budget'])for r in out):
 a=[r for r in out if (r['case'],r['end'],r['budget'])==k]
 print(k,[(v['candidate'],v['status'],v['actual_error_float'])for v in a])
