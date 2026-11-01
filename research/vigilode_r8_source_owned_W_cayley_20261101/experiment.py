#!/usr/bin/env python3
"""R8 source-owned algebraic W inverse for the *original* 8-stage RODAS5P raw-U.
Protected source and coefficients read as frozen Git blobs. No core mutation.
Exact binary64-as-rational CH inverse, full eight stage source replay, finite
Neumann truncation plus exact residual certificate and signed H-matrix control.
"""
from __future__ import annotations
from fractions import Fraction as F
from pathlib import Path
import hashlib, importlib.util, json, math, sys
import numpy as np
import sympy as sp

ROOT=Path(__file__).resolve().parents[2]
HERE=Path(__file__).resolve().parent
R6=ROOT/"research/vigilode_r6_eightstage_causal_frontier_20261101/experiment.py"
R7=ROOT/"research/vigilode_r7_native_transactional_discriminant_20261101/RESULTS.json"
ORIGINAL_CASES=["asymmetric_2","asymmetric_2_refined","coupled_3_cubic","linear_exchange_control"]
SOURCE_LOCK={"R6": "d68239d53977e36f98b186b627aa0c4c5c7de7cb",
 "R7_result":"583e2fada6050a25dfcebd89b682edf45929a8da717b9a2385f628bed47094d4"}
PRE_REG="71109f9ab908442cf3ba50222bc0af34bfacf74c"


def source():
    m=importlib.util.spec_from_file_location("vigilode_r8_locked_r6",R6)
    mod=importlib.util.module_from_spec(m);sys.modules[m.name]=mod;m.loader.exec_module(mod)
    mod.check_locks()
    assert hashlib.sha256(R7.read_bytes()).hexdigest()==SOURCE_LOCK["R7_result"]
    return mod


def rr(x):
    return F.from_float(float(x))


def sy(x):
    return sp.Rational(x.numerator,x.denominator)


def py(x):
    x=sp.Rational(x)
    return F(int(x.p),int(x.q))


def mv(A,x):
    return [sum(A[i][j]*x[j] for j in range(len(x))) for i in range(len(A))]


def matmul(A,B):
    return [[sum(A[i][k]*B[k][j] for k in range(len(B))) for j in range(len(B[0]))] for i in range(len(A))]


def norminf(x):
    return max(map(abs,x))


def exact_inverse(k,g,c,y,tau):
    n=len(k)
    assert n in (2,3),"SOURCE_DIMENSION_NOT_SUPPORTED"
    assert len(g)==len(c)==len(y)==n
    assert all(x>0 for x in k) and all(x>=0 for x in g)
    assert all(len(row)==n and row[i]==0 and all(x>=0 for x in row)
               for i,row in enumerate(c)), "NOT_COOPERATIVE_SOURCE"
    tauq=rr(tau)
    assert tauq>0
    s=[sum(map(rr,row)) for row in c]
    d=[1+tauq*(rr(k[i])+s[i]+3*rr(g[i])*rr(y[i])**2) for i in range(n)]
    W=[[d[i] if i==j else -tauq*rr(c[i][j]) for j in range(n)] for i in range(n)]
    T=[[F(0) if i==j else tauq*rr(c[i][j])/d[i] for j in range(n)] for i in range(n)]
    q=max(sum(T[i]) for i in range(n))
    m=min(d[i]-tauq*s[i] for i in range(n))
    assert F(0)<=q<1 and m>=1+tauq*min(map(rr,k))
    S=sp.Matrix([[sy(x) for x in row] for row in W])
    det=py(S.det())
    assert det>0
    # Cayley-Hamilton inverse, not a Newton root or inverse by iterative GMRES.
    # p(z)=z^n+a[n-1]*z^(n-1)+...+a[1]*z+a[0].
    cof=[py(x) for x in S.charpoly().all_coeffs()]
    a0=cof[-1]
    assert a0!=0 and a0==(-1)**n*det
    identity=sp.eye(n)
    polynomial=S**(n-1)
    for power in range(n-2,-1,-1):
        polynomial+=sy(cof[n-1-power])*S**power
    invCH=(-1/sy(a0))*polynomial
    invAdj=S.adjugate()/sy(det)
    assert invCH==invAdj
    INV=[[py(invCH[i,j]) for j in range(n)] for i in range(n)]
    ident=matmul(W,INV)
    assert ident==[[F(int(i==j)) for j in range(n)] for i in range(n)]
    assert all(a>=0 for row in INV for a in row)
    return {"W":W,"T":T,"d":d,"inv":INV,"q":q,"m":m,
            "det":det,"charpoly":cof}


def neumann(A,r,K):
    z=[r[i]/A["d"][i] for i in range(len(r))]
    total=z[:]
    for _ in range(K):
        z=mv(A["T"],z)
        total=[u+v for u,v in zip(total,z)]
    res=[v-w for v,w in zip(mv(A["W"],total),r)]
    alpha=norminf([r[i]/A["d"][i] for i in range(len(r))])
    geometric=A["q"]**(K+1)*alpha/(1-A["q"])
    residual=norminf(res)/A["m"]
    exact=mv(A["inv"],r)
    error=norminf([a-b for a,b in zip(exact,total)])
    assert error<=residual and error<=geometric
    return {"K":K,"error":float(error),"residual_bound":float(residual),
            "geometric_bound":float(geometric),"residual_exact_zero":res==[0]*len(r),
            "oracle_exact_CH":True}


def signed_row_dominant_control():
    tau=F(1,8)
    J=[[-F(4),-F(3)],[F(2),-F(5)]]
    W=[[F(i==j)-tau*J[i][j] for j in range(2)] for i in range(2)]
    m=min(abs(W[i][i])-sum(abs(W[i][j]) for j in range(2) if j!=i) for i in range(2))
    assert m>0 and W[0][1]>0 # NOT a Z or M-matrix
    S=sp.Matrix([[sy(v) for v in row] for row in W])
    inv=[[py(S.inv()[i,j]) for j in range(2)] for i in range(2)]
    n=max(sum(abs(t) for t in row) for row in inv)
    assert n<=1/m
    return {"W":[list(map(str,row)) for row in W],"lower_dominance_margin":str(m),
            "actual_inverse_infty":str(n),"bound":str(1/m),
            "nonnegative_inverse_claim":False,"is_M_matrix":False,
            "source_scope":"signed_H_matrix_inequality_not_original_cooperative_rhs"}


def main():
    prev=source();cases=prev.load_cases();a,C,b,gamma=prev.tableau()
    observed=json.loads(R7.read_text())
    assert observed["distinct_physics_models"]==3
    assert observed["no_native_q2_default_winner"]
    rows=[]
    for case in ORIGINAL_CASES:
        k,g,c,y=cases[case];steps=prev.ORIGINAL_STEPS[case]
        h,tau,W_float=prev.build_case(k,g,c,y,steps,gamma)
        A=exact_inverse(k,g,c,y,tau)
        n=len(y)
        assert max(abs(float(A["W"][i][j])-W_float[i,j]) for i in range(n) for j in range(n))<1e-15
        # Entire first RODAS step: recompute *every original RHS stage* with
        # Cayley-Hamilton W^-1 at each stage, never alter ROW tableau.
        U=np.zeros((8,n),dtype=float)
        stage_rows=[]
        for i in range(8):
            rhs=prev.rhs_float(i,U,np.asarray(y),k,g,c,a,C,tau,gamma)
            r=list(map(rr,rhs))
            exact=mv(A["inv"],r)
            represented=[float(q) for q in exact]
            residual=norminf([x-y for x,y in zip(mv(A["W"],list(map(rr,represented))),r)])
            assert residual/A["m"]<F(1,10**12)
            U[i]=represented
            stage_rows.append({"stage":i+1,"original_rhs_norm":float(norminf(r)),
                "CH_exact_stage_residual":0.0,
                "binary64_residual_bound":prev.ceil_f(residual/A["m"])})
        sequential,endpoint_s=prev.protected_sequential(y,k,g,c,a,C,b,tau,gamma,W_float)
        u_ref,endpoint_ref=prev.high_precision_reference(y,k,g,c,a,C,b,tau,gamma)
        certificate=prev.bound_by_lower_triangle(U,y,k,g,c,a,C,b,tau,gamma)
        assert certificate["certificate"]=="EXACT_BINARY64_AS_RATIONAL_SOURCE_SCOPED"
        endpoint=np.asarray(y)+b@U
        actual=float(np.max(np.abs(endpoint-endpoint_ref)))
        assert actual<=certificate["endpoint_bound_upper"]+5e-14,(case,actual,certificate)
        assert actual<1e-12,(case,actual)
        # Neumann can be pruned by a residual/cost gate, not by optimistic
        # closeness of a single stage/idealized geometric ratio.
        first=prev.rhs_float(0,U,np.asarray(y),k,g,c,a,C,tau,gamma)
        rhsq=list(map(rr,first))
        ns=[neumann(A,rhsq,K) for K in (0,1,2,3,4,5,6,8,10,12)]
        certified=[v for v in ns if v["residual_bound"]<=1e-12]
        rows.append({"source":case,
          "physical_family":"asymmetric_2" if case=="asymmetric_2_refined" else case,
          "is_resolution_control":case=="asymmetric_2_refined",
          "n":n,"h":h,"tau":tau,
          "original_stage_count":8,"exact_CH_inverse_valid":True,
          "Cayley_Hamilton_characteristic_coefficients":[str(x) for x in A["charpoly"]],
          "determinant_positive":A["det"]>0,
          "q_infty":float(A["q"]),"M_inverse_inf_bound":float(1/A["m"]),
          "full_first_step_CH_endpoint":endpoint.tolist(),
          "protected_source_endpoint":endpoint_s.tolist(),
          "reference_90digit_endpoint":endpoint_ref.tolist(),
          "CH_against_reference_endpoint_linf":actual,
          "CH_vs_protected_endpoint_linf":float(np.max(np.abs(endpoint-endpoint_s))),
          "whole_eight_stage_rational_residual_bound":certificate["endpoint_bound_upper"],
          "CH_stage_details":stage_rows,
          "first_Raw_U_neumann":ns,
          "first_K_meeting_1e_minus_12":min((z["K"] for z in certified),default=None),
          "native_source_driver_code_changed":False,
          "actual_native_wall_ns":"NOT_RUN_FOR_CH",
          "theoretical_callback_JVP_count_for_exact_CH":0})
    assert len({r["physical_family"] for r in rows})==3
    r8={"schema":"vigilode.r8.source_owned_cayley_hamilton_raw_U.v1",
       "pre_simulation_registration_commit":"71109f9ab908442cf3ba50222bc0af34bfacf74c",
       "source":"original_freezebit_cooperative_cubic_RODAS5P",
       "original_R7_results_sha256":SOURCE_LOCK["R7_result"],
       "original_R7_protected_over_native_q2_ratio":observed["median_matched_error_protected_over_transactional"],
       "original_R7_transactional_decision":"HOLD_DEFAULT_NATIVE_Q2_FOR_THESE_SOURCE_CLASSES",
       "new_candidate":"CH_exact_W_inverse_for_source_owned_n2_or_n3_original_raw_U",
       "source_physics_family_count":3,"total_full_original_first_step_rows":4,
       "total_full_original_RAW_U_stage_solves":32,
       "first_stage_Neumann_probes":40,
       "actual_native_full_IVP_CH":"NOT_RUN",
       "strong_no_go":"NOT_ESTABLISHED",
       "source_math_derived":True,
       "code_scope":"standalone mathematics/source-owned numerical check only",
       "total_source_family_rows":rows,
       "non_Metzler_signed_H_matrix_branch":signed_row_dominant_control(),
       "frozen_original_DAG":"7/23","successor_admitted":"0/12",
       "production":"HOLD","default_or_tolerance_changed":False}
    p=HERE/"RESULTS.json"
    p.write_text(json.dumps(r8,sort_keys=True,indent=2,allow_nan=False)+"\n")
    print("R8_CH_RAW_U_STAGE_COMPUTATIONS",32,"SOURCE_PHYSICS_FAMILIES",3,
          "FIRST_STAGE_NEUMANN_PROBES",40)
    for row in rows:
        print("SOURCE",row["source"],"n",row["n"],"q",row["q_infty"],
              "K_1e-12",row["first_K_meeting_1e_minus_12"],
              "whole_CH_error",row["CH_against_reference_endpoint_linf"],
              "cert_upper",row["whole_eight_stage_rational_residual_bound"])
    print("RESULT_SHA256",hashlib.sha256(p.read_bytes()).hexdigest())
    return r8


if __name__=="__main__":
    main()
