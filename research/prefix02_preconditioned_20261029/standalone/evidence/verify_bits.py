import csv,struct,json,sys
from fractions import Fraction as F
from pathlib import Path
p=Path(__file__).with_name('bit_probe.csv')
def frombits(s):return F.from_float(struct.unpack('>d',int(s,16).to_bytes(8,'big'))[0])
def arr(s):return [frombits(x) for x in s.split(';')]
counts={'rows':0,'mu':0,'rho_components':0,'rho_max':0,'inverse':0,'forward_product':0,'true_stage_error':0}
max_excess=F(0)
for row in csv.DictReader(p.open()):
 sigma,mu,rho,k,e=[frombits(row[x]) for x in ['sigma','mu','rho','k','e']]
 jd,rhs,x,w=[arr(row[t]) for t in ['jdiag','rhs','x','weights']]
 assert len(jd)==len(rhs)==len(x)==len(w)==4
 mu_exact=max(jd)
 assert mu>=mu_exact;counts['mu']+=1
 rs=[];es=[]
 for j,b,z,weight in zip(jd,rhs,x,w):
  assert weight>0
  ri=abs(b-(1-sigma*j)*z)/weight
  assert rho>=ri
  counts['rho_components']+=1
  rs.append(ri)
  exact=b/(1-sigma*j)
  es.append(abs(z-exact)/weight)
 assert rho>=max(rs);counts['rho_max']+=1
 assert 1-sigma*mu_exact>0
 assert k>=1/(1-sigma*mu_exact);counts['inverse']+=1
 assert e>=k*rho;counts['forward_product']+=1
 assert e>=max(es);counts['true_stage_error']+=1
 counts['rows']+=1
print(json.dumps({'status':'PASS','counts':counts,'total_inequalities':sum(v for k,v in counts.items() if k!='rows')},sort_keys=True))
