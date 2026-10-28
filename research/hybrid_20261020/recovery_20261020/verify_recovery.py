#!/usr/bin/env python3
"""Bounded recovery: verify the original CSV and directly derived algebra.

Uses only the Python standard library. Does not run a solver, replay the entire
continuous certificate, read credentials, access the network, or push Git.
The full archived Fraction audit remains a separately identified prior result.
"""
from __future__ import annotations
import argparse
import csv
import hashlib
import io
import json
import math
import random
import sys
import unittest
from fractions import Fraction as Q
from pathlib import Path
from typing import Any

EXPECTED_SHA256 = 'dbea376ee7c526ba85cfdaf4b59f8883de9ec0f5b1665dcbe76f15ae1b70e8ad'
EXPECTED_BLOB = 'dce680be180ff551d1a00f1f572c29ece2cd19ef'
MEASURED = '49ca3150c68d527a275c855ecf814122ff9c533a'
PUBLISHED = 'f738fb481bad3f086b568c8493105576851093f2'
NATIVE = ('auto','structured','eager-full','lazy-full','lazy-local','sequential','cubic-auto')
EXTERNAL = ('fast','diffsol','dvode')
PAIRS = (('eager-full','lazy-local'),('lazy-full','lazy-local'),
         ('structured','auto'),('cubic-auto','auto'),('sequential','structured'),
         ('diffsol','auto'),('dvode','auto'),('fast','auto'))
CSV_FOR_TEST: Path | None = None

class VerificationError(ValueError):
    pass

def require(condition: bool, message: str) -> None:
    if not condition:
        raise VerificationError(message)

def exact_blob(data: bytes) -> str:
    return hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()

def positive(row: dict[str,str], key: str) -> float:
    value = float(row[key])
    require(math.isfinite(value) and value > 0.0, f'Invalid {key}')
    return value

def inspect_csv(data: bytes) -> dict[str,Any]:
    require(hashlib.sha256(data).hexdigest() == EXPECTED_SHA256, 'CSV_SHA256_MISMATCH')
    require(exact_blob(data) == EXPECTED_BLOB, 'CSV_GIT_BLOB_MISMATCH')
    rows = list(csv.DictReader(io.StringIO(data.decode('utf-8'), newline='')))
    require(len(rows) == 40, 'ROW_COUNT')
    keyed = {(int(r['n']), int(r['epsilon']), r['arm']): r for r in rows}
    expected = {(n,eps,arm) for n in (16,64) for eps in (0,1) for arm in NATIVE+EXTERNAL}
    require(len(keyed) == 40 and set(keyed) == expected, 'CELL_IDENTITIES')
    native_count = 0
    for key,r in keyed.items():
        require(r['valid'] == 'True' and int(r['complete_blocks']) == 4, f'ROW_STATUS:{key}')
        positive(r,'median_ms'); positive(r,'tol')
        error = positive(r,'absolute_error_upper')
        require(error <= 1e-6, f'OBSERVED_OUTPUT_TARGET:{key}')
        require(positive(r,'empirical_reference_radius') <= error, f'REFERENCE_RADIUS:{key}')
        if r['arm'] in NATIVE:
            native_count += 1
            bound = positive(r,'certified_radius')
            # This is a summary sanity check, not a replay of a continuous proof.
            require(error <= bound <= 1e-6, f'RECORDED_RADIUS_SANITY:{key}')
            require(int(r['accepted_windows']) > 0 and int(r['rejected_windows']) >= 0, 'WINDOW_COUNTS')
        else:
            require(r['certified_radius'] == '', f'EXTERNAL_CERTIFICATE_NOT_CLAIMED:{key}')
    comparisons=[]
    for n in (16,64):
        for eps in (0,1):
            for baseline,candidate in PAIRS:
                a,b=keyed[n,eps,baseline],keyed[n,eps,candidate]
                ea,eb=positive(a,'absolute_error_upper'),positive(b,'absolute_error_upper')
                comparisons.append(dict(n=n,epsilon=eps,baseline=baseline,candidate=candidate,
                    error_ratio=max(ea,eb)/min(ea,eb),
                    matched_empirical_error=max(ea,eb)/min(ea,eb)<=2.0,
                    certificate_contracts_same=baseline in NATIVE,
                    ratio_of_reported_medians=positive(a,'median_ms')/positive(b,'median_ms'),
                    statistic_scope='ratio of archived per-arm medians; NOT paired-block median'))
    require(sum(p['matched_empirical_error'] for p in comparisons)==22,'ERROR_PAIR_COUNTS')
    for n in (16,64):
        # Exact packet admission is epsilon=0 only; algebraic Jf is still counted.
        p=keyed[n,0,'auto']
        require(all(int(p[k])==0 for k in ('local_dual_cells','full_dual_cells','partial_ad_cells')), 'PACKET_AD_COUNTS')
        require(int(p['jvp_vectors'])>0, 'DO_NOT_RELABEL_QUINTIC_PACKET_AS_NO_JF')
        # epsilon=1 auto is the structured route; no epsilon averaging.
        a,b=keyed[n,1,'auto'],keyed[n,1,'structured']
        for k in ('absolute_error_upper','certified_radius','accepted_windows','rhs_vectors','jvp_vectors','partial_ad_cells'):
            require(a[k]==b[k], 'NONLINEAR_STRUCTURAL_ROUTE_BINDING:'+k)
    return dict(status='PASS_SCOPED_RECOVERED_CSV', measured_source=MEASURED,published_source=PUBLISHED,
        measured_run=39378576576,verified_csv_bytes=len(data),verified_rows=len(rows),
        csv_sha256=EXPECTED_SHA256,csv_git_blob=EXPECTED_BLOB,native_rows=native_count,
        external_rows=len(rows)-native_count,checked_error_pairs=len(comparisons),
        matched_empirical_error_pairs=22,
        matched_error_and_same_certificate_pairs=sum(p['matched_empirical_error'] and p['certificate_contracts_same'] for p in comparisons),
        comparisons=comparisons,new_solver_executions=0,new_timing_samples=0,
        full_rational_certificate_replayed_in_this_check=False,
        external_certificates_verified=False)

# Dual numbers are an independent arithmetic check of the model's local AD rule,
# not a rerun of the Rust AD implementation. Fractions make these checks exact.
class Dual:
    def __init__(self, val: Q, tangent: Q=Q(0)):
        self.val, self.tangent = Q(val), Q(tangent)
    @staticmethod
    def cast(x: Any) -> 'Dual':
        return x if isinstance(x,Dual) else Dual(Q(x))
    def __add__(self,x: Any) -> 'Dual':
        b=self.cast(x);return Dual(self.val+b.val,self.tangent+b.tangent)
    __radd__=__add__
    def __neg__(self)->'Dual':return Dual(-self.val,-self.tangent)
    def __sub__(self,x:Any)->'Dual':return self+-self.cast(x)
    def __rsub__(self,x:Any)->'Dual':return self.cast(x)+-self
    def __mul__(self,x:Any)->'Dual':
        b=self.cast(x);return Dual(self.val*b.val,self.tangent*b.val+self.val*b.tangent)
    __rmul__=__mul__

def matvec(a: list[list[Q]], v: list[Q])->list[Q]:
    return [sum((x*y for x,y in zip(row,v)),Q(0)) for row in a]

def gaussian(a: list[list[Q]], b: list[Q])->list[Q]:
    n=len(b);m=[row[:]+[v] for row,v in zip(a,b)]
    for k in range(n):
        pivot=next((i for i in range(k,n) if m[i][k]),None)
        if pivot is None: raise VerificationError('SINGULAR_EXACT_FIXTURE')
        m[k],m[pivot]=m[pivot],m[k]
        value=m[k][k];m[k]=[z/value for z in m[k]]
        for i in range(n):
            if i!=k:
                q=m[i][k];m[i]=[x-q*y for x,y in zip(m[i],m[k])]
    return [row[-1] for row in m]

def algebra_checks()->dict[str,Any]:
    rng=random.Random(20261020)
    cases=0;shift_solves=0;dissipativity_checks=0
    for n in (2,3,5,7):
        for _ in range(8):
            q=[Q(rng.randint(0,16),16) for i in range(n)]
            v=[Q(rng.randint(-7,7),8) for i in range(n)]
            d=Q(5,4);la=Q(2);al=Q(2,5);ep=Q(rng.randint(0,3),2)
            mean=sum(q,Q(0))/n;mv=sum(v,Q(0))/n
            lap=[[Q(0) for _ in range(n)] for _ in range(n)]
            for i in range(n-1):
                lap[i][i]-=1;lap[i][i+1]+=1;lap[i+1][i]+=1;lap[i+1][i+1]-=1
            lq=matvec(lap,q)
            f=[d*lq[i]-la*q[i]+al*mean*(1-q[i])-ep*q[i]**2 for i in range(n)]
            variance=sum(((x-mean)**2 for x in q),Q(0))/n
            require(sum(f,Q(0))/n==(al-la)*mean-(al+ep)*mean**2-ep*variance,'MEAN_VARIANCE_IDENTITY')
            if ep==0:
                transverse=[x-mean for x in q]
                require([x-sum(f,Q(0))/n for x in f]==[d*x-(la+al*mean)*z for x,z in zip(matvec(lap,transverse),transverse)],'TRANSVERSE_CLOSURE')
            local=[[d*lap[i][j]-(la+al*mean+2*ep*q[i] if i==j else 0) for j in range(n)] for i in range(n)]
            u=[al*(1-x) for x in q]
            jac=[[local[i][j]+u[i]/n for j in range(n)] for i in range(n)]
            localad=[]
            for i in range(n):
                tangent=(al*Dual(mean,mv)*(1-Dual(q[i],v[i]))-ep*Dual(q[i],v[i])*Dual(q[i],v[i])).tangent
                localad.append(d*matvec(lap,v)[i]-la*v[i]+tangent)
            require(localad==matvec(jac,v),'LOCAL_AD_AND_ALGEBRA_MATCH')
            for i in range(n):
                require(sum(jac[i],Q(0))==(al-la)-al*(q[i]+mean)-2*ep*q[i],'ROW_SUM_IDENTITY')
                require(all(jac[i][j]>=0 for j in range(n) if i!=j),'METZLER_DOMAIN')
                require(sum(jac[i],Q(0))<=al-la,'INFINITY_GAIN')
                dissipativity_checks+=1
            shift=Q(1,10)
            t=[[Q(int(i==j))-shift*local[i][j] for j in range(n)] for i in range(n)]
            a=[shift*x for x in u];z=gaussian(t,a);den=1-sum(z,Q(0))/n
            require(den>0,'POSITIVE_RANK_DENOMINATOR')
            b=v;z0=gaussian(t,b);sol=[x+zz*(sum(z0,Q(0))/n)/den for x,zz in zip(z0,z)]
            w=[[Q(int(i==j))-shift*jac[i][j] for j in range(n)] for i in range(n)]
            require(matvec(w,sol)==b,'SHIFT_ORIGINAL_SYSTEM_RESIDUAL')
            require(sol==gaussian(w,b),'SHIFT_DENSE_CONTROL')
            cases+=1;shift_solves+=1
    return dict(status='PASS_EXACT_RATIONAL_MODEL_IDENTITIES',sampled_rational_models=cases,
        exact_shift_systems=shift_solves,verified_gain_rows=dissipativity_checks,
        scope='exact rational test instances; general identities derived in report and Wolfram check',
        is_rust_execution=False)

class Tests(unittest.TestCase):
    def test_original_csv_identity(self):self.assertEqual(inspect_csv(CSV_FOR_TEST.read_bytes())['verified_rows'],40)
    def test_tamper_is_rejected(self):
        data=CSV_FOR_TEST.read_bytes()
        with self.assertRaises(VerificationError):inspect_csv(data.replace(b'0.7986125',b'0.7986124',1))
    def test_lf_conversion_not_byte_identical(self):
        with self.assertRaises(VerificationError):inspect_csv(CSV_FOR_TEST.read_bytes().replace(b'\r\n',b'\n'))
    def test_achieved_and_certificate_contracts_are_distinct(self):
        r=inspect_csv(CSV_FOR_TEST.read_bytes())
        self.assertEqual(r['matched_empirical_error_pairs'],22)
        self.assertLess(r['matched_error_and_same_certificate_pairs'],22)
    def test_exact_model_algebra(self):self.assertEqual(algebra_checks()['sampled_rational_models'],32)
    def test_nonlinear_mean_needs_variance(self):
        a=[Q(0),Q(1)];b=[Q(1,2),Q(1,2)]
        self.assertEqual(sum(a),sum(b));self.assertNotEqual(sum(x*x for x in a),sum(x*x for x in b))
    def test_dense_hidden_error_not_caught_by_endpoints(self):
        # p(t)=4t(1-t) solves neither y'=0 nor its target, although p(0)=p(1)=0.
        self.assertEqual(4*Q(0)*(1-Q(0)),0)
        self.assertEqual(4*Q(1)*(1-Q(1)),0)
        self.assertEqual(4*Q(1,2)*(1-Q(1,2)),1)
    def test_dual_product_is_not_merely_primal(self):
        x=Dual(Q(2,3),Q(5,7));y=x*x
        self.assertEqual(y.tangent,2*x.val*x.tangent)
    def test_nonfinite_metric_rejected(self):
        for bad in ('nan','inf','-1','0'):
            with self.assertRaises(VerificationError):positive({'x':bad},'x')

if __name__=='__main__':
    p=argparse.ArgumentParser()
    p.add_argument('--csv',type=Path,default=Path(__file__).resolve().parents[1]/'runs/39378576576/RESULTS.csv')
    p.add_argument('--output',type=Path,required=True)
    p.add_argument('--self-test',action='store_true')
    a=p.parse_args();CSV_FOR_TEST=a.csv
    try:
        result=inspect_csv(a.csv.read_bytes());result['new_exact_algebra_checks']=algebra_checks()
        if a.self_test:
            suite=unittest.defaultTestLoader.loadTestsFromTestCase(Tests)
            outcome=unittest.TextTestRunner(stream=sys.stdout,verbosity=2).run(suite)
            require(outcome.wasSuccessful(),'RECOVERY_TESTS_FAILED')
            result['new_recovery_tests']={'run':outcome.testsRun,'failures':len(outcome.failures),'errors':len(outcome.errors)}
        a.output.parent.mkdir(parents=True,exist_ok=True)
        a.output.write_text(json.dumps(result,indent=2,sort_keys=True,allow_nan=False)+'\n')
        print(json.dumps({k:v for k,v in result.items() if k!='comparisons'},indent=2))
    except (OSError,ValueError) as e:
        print(f'RECOVERY_VERIFICATION_FAILED: {e}',file=sys.stderr);sys.exit(1)
