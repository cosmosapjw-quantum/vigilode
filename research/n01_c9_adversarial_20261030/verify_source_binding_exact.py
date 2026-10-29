#!/usr/bin/env python3
"""C9 adversarial verifier. Exact rational science, separate from official C8 native source.

Tests conditional resolvent math, causal nilpotency and a wrong-source pass.
The wrong-source case is a source-authentication counterexample, not a rejection
of the mathematical Neumann/error bound for the supplied, different equation.
"""
from fractions import Fraction as F
from pathlib import Path
import json, random, hashlib

OUT = Path(__file__).resolve().parent
rng = random.Random(20261030 + 9)
Z = F(0)
I = F(1)

def f(k, y):
    k1, k2, k3 = k
    a, b, c = y
    return (-k1*a+k2*b*c, k1*a-k2*b*c-k3*b*b, k3*b*b)

def jac(k, y):
    k1, k2, k3 = k
    _, b, c = y
    return ((-k1, k2*c, k2*b),
            (k1, -k2*c-2*k3*b, -k2*b),
            (Z, 2*k3*b, Z))

def norm1(M):
    return max(sum(abs(M[i][j]) for i in range(len(M))) for j in range(len(M)))

def invert3(A):
    n=3
    m=[[A[i][j] for j in range(n)]+[F(int(i==j)) for j in range(n)] for i in range(n)]
    for j in range(n):
        pivot=next((i for i in range(j,n) if m[i][j]!=0),None)
        assert pivot is not None, 'singular matrix, not a valid resolvent case'
        m[j],m[pivot]=m[pivot],m[j]
        q=m[j][j]
        m[j]=[v/q for v in m[j]]
        for i in range(n):
            if i==j:continue
            q=m[i][j]
            m[i]=[m[i][x]-q*m[j][x] for x in range(2*n)]
    return [row[n:] for row in m]

# Identify the actual C8 source contract by its immutable Git blob.  In a
# GitHub checkout this is the native test file; in the local C8 ZIP it is
# under work/.../native.  This is source identity, not a proof of its claims.
source_candidates=[
    OUT.parent.parent / 'crates/vigilode-coupled/tests/n01_df_c8_work_precision.rs',
    OUT / 'work/VIGILODE_N01_DF_C8_20261030/native/n01_df_c8_work_precision.rs',
]
source_path=next((p for p in source_candidates if p.exists()),None)
assert source_path is not None, 'Pinned C8 verifier source unavailable'
source_bytes=source_path.read_bytes()
source_text=source_bytes.decode()
marker='\n/// C9 adversarial reproducer'
if marker in source_text:
    original_prefix=(source_text.split(marker,1)[0].rstrip('\n')+'\n').encode()
else:
    original_prefix=source_bytes
actual_blob=hashlib.sha1(b'blob '+str(len(original_prefix)).encode()+b'\0'+original_prefix).hexdigest()
assert actual_blob=='e6a33b66ba80055cae9439cf683a1715ca77c5a2', actual_blob
start=source_text.index('fn certified_raw_stage_bound(')
end=source_text.index(') -> Result<',start)
signature=source_text[start:end]
assert 'case: Case' in signature and 'stage_k:' in signature
assert 'N01MassActionOwner' not in signature and 'N01MassActionTrial' not in signature

norm_checks=0
resolvent_checks=0
for i in range(512):
    k=tuple(F(rng.randint(0,256),128) for _ in range(3))
    y=tuple(F(rng.randint(0,256),256) for _ in range(3))
    tau=F(1,2**rng.randint(4,11))
    B=2*max(k[0],(k[1]+2*k[2])*F(11,10)*sum(y))
    J=jac(k,y)
    assert norm1(J)<=B
    assert all(sum(J[row][col] for row in range(3))==0 for col in range(3))
    assert sum(f(k,y))==0
    norm_checks+=1
    if tau*B<I:
        A=[[F(int(i==j))-tau*J[i][j] for j in range(3)] for i in range(3)]
        n=norm1(invert3(A))
        assert n <= I/(I-tau*B)
        resolvent_checks+=1

# The C8 verifier accepts scalar `case.k` instead of an opaque source-owner trial.
# With a mismatched fake model the entire first-stage raw-U defect is zero,
# while the SAME stage proposal has a nonzero actual residual under true rates.
y=(F(3,4),F(1,8),F(1,8))
kt=(F(1,2),F(10),F(200))
kf=(Z,Z,Z)
tau=F(1,64)
U0=(Z,Z,Z)
r_fake=tuple(U0[i]-tau*f(kf,y)[i] for i in range(3))
r_true=tuple(U0[i]-tau*f(kt,y)[i] for i in range(3))
assert r_fake==(Z,Z,Z)
assert r_true==(F(7,2048),F(93,2048),-F(25,512))
assert sum(r_true)==0
assert max(map(abs,r_true))==F(25,512)
# For all eight zero stages the fake rate source claims zero residual,
# positive carrier tube and Neumann margin=1; actual true source differs.
m_tube=F(11,10)*sum(y)
assert min(y)>0 and sum(y)<m_tube
assert F(1)-tau*2*max(kf[0],(kf[1]+2*kf[2])*m_tube)==F(1)

# Validate finite strictly-lower transfer, with no contraction needed;
# nilpotency L^8 = 0 and 8-step forward substitution must agree exactly.
tri_cases=128
for _ in range(tri_cases):
    L=[[F(rng.randint(0,16),64) if j<i else Z for j in range(8)] for i in range(8)]
    d=[F(rng.randint(1,20),100) for _ in range(8)]
    e=[]
    for i in range(8):
        e.append(d[i]+sum(L[i][j]*e[j] for j in range(i)))
    total=list(d)
    v=list(d)
    for _power in range(1,8):
        v=[sum(L[i][j]*v[j] for j in range(i)) for i in range(8)]
        total=[total[i]+v[i] for i in range(8)]
    assert e==total
    v=[sum(L[i][j]*v[j] for j in range(i)) for i in range(8)]
    assert all(x==0 for x in v)

report={
 'schema':'vigilode.n01.c9.adversarial_source_binding_exact.v1',
 'parent_C8_source_blob':'e6a33b66ba80055cae9439cf683a1715ca77c5a2',
 'seed':20261039,'pinned_C8_source_git_blob':'e6a33b66ba80055cae9439cf683a1715ca77c5a2',
 'checks':{'exact_positive_state_j_norm':norm_checks,
           'conditional_Neumann_inverse_norm':resolvent_checks,
           'exact_causal_nilpotent_transfer':tri_cases},
 'wrong_source_forgery':{
  'test_only_verifier_uses_external_case_k':True,
  'source_signature':signature.strip(),
  'carrier_y':list(map(str,y)), 'tau':str(tau),
  'fake_rates':list(map(str,kf)),'actual_owner_rates':list(map(str,kt)),
  'fake_stage_u0':list(map(str,U0)),
  'fake_residual':list(map(str,r_fake)),
  'actual_residual':list(map(str,r_true)),
  'actual_residual_infinity':str(max(map(abs,r_true))),
  'claim':'The error bound can be correct for the FAKE equations while not authenticating the ACTUAL OdeProblem source. This is a promotion blocker, not a refutation of C8 source-matched runs.'
 },
 'result':'PASS for adversarial reproduction; C9 independent-author NOT_RUN',
 'source_binding_admission':'FAIL_FOR_GENERAL_PHYSICAL_SOURCE',
 'production':'HOLD'
}
(OUT/'exact_results.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
