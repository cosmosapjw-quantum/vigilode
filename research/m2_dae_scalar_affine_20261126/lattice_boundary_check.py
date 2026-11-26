# Supplemental exact arithmetic check (not holdout, no solver receipts).
from fractions import Fraction as Q
import random,json
from pathlib import Path
G=2**51
rng=random.Random(1729)
roots=[Q(-1),Q(0),Q(1,2),Q(1),Q(2),Q(1,3),Q(1,2**1074),Q(1)-Q(1,2**1074)]
for _ in range(128):
 k=rng.randrange(G+1)
 for d in [Q(0),Q(1,G*3),-Q(1,G*3),Q(1,G*2**64),-Q(1,G*2**64)]:
  roots.append(Q(k,G)+d)
tols=[2.**-50,2.**-48,1e-6,0.001]
parts=[[(Q(0),Q(1))],[(Q(0),Q(1,2)),(Q(1,2),Q(1))]]
def compare(r,parts,tol):
 n=0;nodes=0;out=[]
 def v(t):
  nonlocal n;n+=1;return int(t<r)
 for a,b in reversed(parts):
  if v(a)-v(b)==0:continue
  while True:
   nodes+=1
   if b==r or b-a<=Q(tol*.5):
    out.append((b if b==r else a,b));break
   m=(a+b)/2
   if v(a)-v(m):b=m
   else:a=m
 k=(r*G).__floor__();exact=(r*G==k)
 cn=0;nn=0;rr=[]
 def u(i):
  nonlocal cn;cn+=1;return int(i<k or i==k and not exact)
 for a,b in reversed(parts):
  a,b=int(a*G),int(b*G)
  if u(a)-u(b)==0:continue
  while True:
   nn+=1
   if exact and b==k or (b-a)/G<=tol*.5:
    rr.append((Q(b if exact and b==k else a,G),Q(b,G)));break
   assert (b-a)%2==0
   m=(a+b)//2
   if u(a)-u(m):b=m
   else:a=m
 assert (out,n,nodes)==(rr,cn,nn)
 return nodes
count=visits=0
for r in roots:
 for t in tols:
  for p in parts:
   visits+=compare(r,p,t);count+=1
out={'scope':'supplemental exact rational versus integer lattice traversal, constructed boundary cases, not independent physical holdout','root_positions':len(roots),'tolerances':tols,'partition_families':len(parts),'comparisons':count,'matching_node_visits':visits}
Path(__file__).with_name('LATTICE_BOUNDARY.json').write_text(json.dumps(out,indent=2)+'\n')
print(out)
