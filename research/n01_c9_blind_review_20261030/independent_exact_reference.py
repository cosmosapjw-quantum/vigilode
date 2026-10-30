#!/usr/bin/env python3
"""Blind-style independent exact arithmetic cross-check of PR128's source owner assumption.

NOT a distinct third-party author and NOT an implementation of the native Rust API.
"""
from fractions import Fraction as Q
from pathlib import Path
from hashlib import sha256
import json, random

OUT=Path(__file__).resolve().parent

def rhs(k,y):
    k1,k2,k3=k;a,b,c=y
    return (-k1*a+k2*b*c, k1*a-k2*b*c-k3*b*b,k3*b*b)

def residual_for_zero_stage(k,y,tau):
    return tuple(-tau*x for x in rhs(k,y))

seed=20261030+128
rng=random.Random(seed)
fixed_y=(Q(3,4),Q(1,8),Q(1,8))
true=(Q(1,2),Q(10),Q(200))
false=(Q(0),Q(0),Q(0))
tau=Q(1,64)
rf=residual_for_zero_stage(false,fixed_y,tau)
rt=residual_for_zero_stage(true,fixed_y,tau)
assert rf==(0,0,0)
assert rt==(Q(7,2048),Q(93,2048),-Q(25,512))
assert sum(rt)==0
N=1024; flagged=0
for _ in range(N):
    y=(Q(rng.randint(1,96),128),Q(rng.randint(1,64),128),Q(rng.randint(1,96),128))
    k=(Q(rng.randint(1,32),16),Q(rng.randint(1,32),16),Q(rng.randint(1,64),16))
    step=Q(1,2**rng.randint(4,10))
    r_fake=residual_for_zero_stage((Q(0),)*3,y,step)
    r_true=residual_for_zero_stage(k,y,step)
    assert r_fake==(0,0,0)
    assert sum(r_true)==0
    assert any(z!=0 for z in r_true)
    flagged+=1
report={
  'schema':'vigilode.c9.blind_style.owner_attestation_exact.v1',
  'reviewer_identity':'same ChatGPT assistant, independent exact arithmetic not separate author',
  'seed':seed,
  'cases':N,
  'fake_owner_certifies_zero_for_zero_model':N,
  'true_source_raw_U0_nonzero':flagged,
  'fixed':{'y':list(map(str,fixed_y)),'actual_rates':list(map(str,true)),
           'fake_rates':list(map(str,false)),'tau':str(tau),
           'fake_raw_u0_residual':list(map(str,rf)),
           'actual_raw_u0_residual':list(map(str,rt)),
           'max_actual_residual':str(max(map(abs,rt)))} ,
  'conclusion':'the mathematical certificate for an owner selected by caller cannot authenticate an unrelated native OdeProblem; this is a source ownership boundary not a matrix/Neumann counterexample',
  'distinct_author_C9':'NOT_RUN'
}
(OUT/'independent_exact_results.json').write_text(json.dumps(report,indent=2,ensure_ascii=False)+'\n')
print('PYTHON_PASS exact forged-owner witness and independent randomized 1024/1024 cases PASS')
print(json.dumps(report['fixed'],indent=2))
