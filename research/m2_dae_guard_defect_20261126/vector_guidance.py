# Post-campaign selection replay for the already frozen manufactured vector corpus.
# All candidate scores come from actual Rust receipts, not synthetic surrogate outputs.
from pathlib import Path
import json,sys,random,hashlib,zipfile
P=Path(__file__).parent
H=Path('/home/cosmosapjw/vigilode-preserved/20261125-final-preflight/harness/mathharness-general-3.1.0')
Z=Path('/home/cosmosapjw/Dropbox/mathharness-general-3.1.0.zip')
assert hashlib.sha256(Z.read_bytes()).hexdigest()=='937ddfd13e860162e6bf454d0b3178c64d34a59de5c0d1df98dc513020f678ee'
with zipfile.ZipFile(Z) as z:
 m=json.loads(z.read('mathharness-general-3.1.0/SHA256SUMS.json'))
 for n,d in m['files'].items():
  p=(H/n).resolve();assert H in p.parents and hashlib.sha256(p.read_bytes()).hexdigest()==d,n
sys.path.insert(0,str(H))
from mathharness_general.search.primitives import UCT,ProposalBandit,MapElites
F=['case','guard','derivative','value']
rows={}
for line in (P/'vector.log').read_text().splitlines():
 if 'VECTOR {' in line:
  r=json.loads(line.split('VECTOR ',1)[1]);k=tuple(r[x]for x in F);assert k not in rows;rows[k]=r
assert len(rows)==72
axes=[sorted({k[i]for k in rows})for i in range(4)]
parents={'SchurRefine1':'Schur','SchurRefine2':'Schur','PreparedSchurRefine2':'PreparedSchur'}
def eligible(r):return r['evidence']['velocity'][0]>0
def wd(x):return x[1]-x[0]
def obj(r):return (sum(r['allocation'][k]for k in ['alloc','zeroed','realloc']),r['allocation']['requested_bytes'],r['audit'].get('refinement_sweeps',{}).get('started',0),wd(r['evidence']['velocity']),max(map(wd,r['values'])))
def dom(a,b):return all(x<=y for x,y in zip(a,b))and a!=b
def pareto(data,case):
 ks=[k for k in data if k[:2]==case and eligible(data[k])]
 return [k for k in ks if not any(dom(obj(data[j]),obj(data[k]))for j in ks)]
rng=random.Random(123407);tree=UCT(lambda prefix:axes[len(prefix)] if len(prefix)<4 else [])
bandit=ProposalBandit(['structure','mutation']);archive=MapElites();seen={};used={'structure':0,'mutation':0}
for _ in range(512):
 arm=bandit.pick(rng);used[arm]+=1
 if arm=='mutation' and seen:
  p=rng.choice(list(seen));root=parents.get(p[2],p[2])
  nbr=[r for r in [root,*[v for v,m in parents.items()if m==root]]if r!=p[2]]
  k=(p[0],p[1],rng.choice(nbr),p[3])if nbr else tuple(tree.propose(rng))
 else:k=tuple(tree.propose(rng))
 fresh=k not in seen;seen[k]=rows[k]
 good=eligible(rows[k]);front=k in pareto(seen,k[:2])
 reward=fresh and good and front
 bandit.update(arm,reward);tree.backup(k,float(reward))
 if good:archive.insert('/'.join(k[:3]),float(front),k)
strata=[]
for case in sorted({k[:2]for k in rows}):
 f=pareto(rows,case);obs=pareto(seen,case)
 if f:
  # No weighted aggregate: select minimum sweep and resource, and separately width-first.
  cheap=min(f,key=lambda k:(obj(rows[k])[2],obj(rows[k])[0],obj(rows[k])[1],obj(rows[k])[3],obj(rows[k])[4],k))
  tight=min(f,key=lambda k:(obj(rows[k])[3],obj(rows[k])[4],obj(rows[k])[2],obj(rows[k])[0],k))
  rec={'resource_first':dict(zip(F,cheap),objectives=obj(rows[cheap])),'width_first':dict(zip(F,tight),objectives=obj(rows[tight]))}
 else:rec={'resource_first':None,'width_first':None}
 strata.append({'case':case,'pareto_count':len(f),'covered':len(set(f)&seen.keys()),'recommendations':rec})
result={'method':'post-campaign measured-table replay; UCT+ProposalBandit+MAP-Elites, front per problem stratum; no unseen score used in reward; no online promotion','seed':123407,'rows':len(rows),'unique':len(seen),'proposal_counts':used,'bandit_posterior':bandit.ab,'pareto_coverage':sum(s['covered']for s in strata),'pareto_size':sum(s['pareto_count']for s in strata),'strata':strata}
(P/'VECTOR_GUIDANCE.json').write_text(json.dumps(result,indent=2,sort_keys=True)+'\n')
print(json.dumps(result,indent=2))
