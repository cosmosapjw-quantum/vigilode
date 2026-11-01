#!/usr/bin/env python3
"""R6 / M2: immutable original 8-stage RODAS5P raw-U causal frontier study.

Not a native Rust driver or a full-IVP performance result. Original Rust source
and represented tableau are read-only. Independent mpmath protected sequential
and exact Fraction backward residuals with recursive a-posteriori stage bounds.
"""
from __future__ import annotations

from dataclasses import dataclass
from fractions import Fraction as F
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import struct
import sys

import mpmath as mp
import numpy as np

ROOT=Path(__file__).resolve().parents[2]
HERE=Path(__file__).resolve().parent
TABLEAU=ROOT/"fixtures/rodas5p_coefficients_snapshot.json"
NATIVE=ROOT/"crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs"
RHS=ROOT/"crates/vigilode-coupled/src/coupled_inverse_be.rs"
FIXTURE=ROOT/"crates/vigilode-coupled/examples/coupled_achieved_wp.rs"
R5=ROOT/"docs/vigilode_r5_newtonfree_algebraic_homotopy_20261101/r5_experiment.py"
BLOBS={
 str(NATIVE.relative_to(ROOT)):"d68239d53977e36f98b186b627aa0c4c5c7de7cb",
 str(RHS.relative_to(ROOT)):"d78c5c5f6f476f2a5b1ea88628ed25daa6cfc482",
 str(FIXTURE.relative_to(ROOT)):"1bfc17ec2ed4b902cc06073807388f6e332e31d6",
}
TAB_SHA256="211255f6125b648c6955833c3da537d9c031ec78168b2918563c29fa88c8bce6"
PARENT="7af490762873387c992edfbbbd66d0f8e6198607"
ORIGINAL_STEPS={"asymmetric_2":64,"asymmetric_2_refined":128,"coupled_3_cubic":128,"linear_exchange_control":128}
RTOL_DIAGNOSTIC=1e-10


def git_blob(data:bytes):
    return hashlib.sha1(b"blob "+str(len(data)).encode()+b"\0"+data).hexdigest()


def check_locks():
    assert hashlib.sha256(TABLEAU.read_bytes()).hexdigest()==TAB_SHA256
    for file,sha in BLOBS.items():
        assert git_blob((ROOT/file).read_bytes())==sha, (file,sha)
    assert ROOT.is_dir()
    return True


def load_cases():
    spec=importlib.util.spec_from_file_location("locked_r5_source_for_r6",R5)
    mod=importlib.util.module_from_spec(spec)
    sys.modules[spec.name]=mod
    spec.loader.exec_module(mod)
    return {key:mod.params(v) for key,v in mod.locked_cases().items()}


def tableau():
    check_locks()
    obj=json.loads(TABLEAU.read_text())
    a=np.array([[float(v) for v in row] for row in obj["A"]],float)
    c=np.array([[float(v) for v in row] for row in obj["C"]],float)
    b=np.array([float(v) for v in obj["b_code"]],float)
    gamma=float(obj["gamma"])
    assert a.shape==(8,8) and c.shape==(8,8) and b.shape==(8,)
    assert np.all(np.triu(a,0)==0) and np.all(np.triu(c,0)==0)
    assert gamma>0 and np.all(np.isfinite(a)) and np.all(np.isfinite(c))
    return a,c,b,gamma


def fr(x):
    return F.from_float(float(x))


def ceil_f(x):
    """Smallest binary64 not smaller than an exact rational, or refuse."""
    assert isinstance(x,F) and x>=0
    try:f=float(x)
    except OverflowError:return None
    if not math.isfinite(f):return None
    if fr(f)<x:
        f=math.nextafter(f,math.inf)
    assert fr(f)>=x
    return f


def mpval(x):
    x=fr(x)
    return mp.mpf(x.numerator)/mp.mpf(x.denominator)


def f_float(u,k,g,c):
    u=np.asarray(u,float)
    c=np.asarray(c,float)
    return -np.asarray(k)*u-np.asarray(g)*u**3+c@u-c.sum(axis=1)*u


def j_float(y,k,g,c):
    c=np.asarray(c,float)
    j=c.copy()
    np.fill_diagonal(j,-np.asarray(k)-c.sum(axis=1)-3*np.asarray(g)*np.asarray(y)**2)
    return j


def f_fraction(u,k,g,c):
    n=len(k)
    out=[]
    for i in range(n):
        f=-fr(k[i])*u[i]-fr(g[i])*u[i]**3
        f+=sum(fr(c[i][j])*(u[j]-u[i]) for j in range(n) if j!=i)
        out.append(f)
    return out


def build_case(k,g,c,y,steps,gamma):
    y=np.asarray(y,float)
    assert steps>0 and gamma>0 and len(y)==len(k)
    h=float(0.1/steps)
    tau=float(h*gamma)
    W=np.eye(len(y))-tau*j_float(y,k,g,c)
    assert np.all(np.diag(W)>0) and np.all(W-np.diag(np.diag(W))<=0)
    assert np.min(W.sum(axis=1))>1
    return h,tau,W


def rhs_float(i,U,y,k,g,c,a,C,tau,gamma):
    # Original RODAS5P autonomous raw-U equation, not an auxiliary BE system.
    z=np.array(y,dtype=float)
    for j in range(i):
        z+=a[i,j]*U[j,:]
    value=tau*f_float(z,k,g,c)
    for j in range(i):
        value+=gamma*C[i,j]*U[j,:]
    return value


def protected_sequential(y,k,g,c,a,C,b,tau,gamma,W):
    out=np.zeros((8,len(y)))
    for i in range(8):
        out[i]=np.linalg.solve(W,rhs_float(i,out,y,k,g,c,a,C,tau,gamma))
    endpoint=np.asarray(y)+b@out
    return out,endpoint


def high_precision_reference(y,k,g,c,a,C,b,tau,gamma):
    mp.mp.dps=90
    n=len(y)
    yy=[mpval(x) for x in y]
    kk=[mpval(x) for x in k]
    gg=[mpval(x) for x in g]
    cc=[[mpval(x) for x in row] for row in c]
    aa=[[mpval(x) for x in row] for row in a]
    CC=[[mpval(x) for x in row] for row in C]
    bb=[mpval(x) for x in b]
    taup=mpval(tau); gp=mpval(gamma)
    def F_mp(z):
        return [-kk[i]*z[i]-gg[i]*z[i]**3+
                sum(cc[i][j]*(z[j]-z[i]) for j in range(n) if j!=i)
                for i in range(n)]
    W=mp.matrix(n)
    for i in range(n):
        for j in range(n):
            if i==j:
                W[i,j]=1+taup*(kk[i]+sum(cc[i])+3*gg[i]*yy[i]**2)
            else:
                W[i,j]=-taup*cc[i][j]
    U=[[mp.mpf(0) for j in range(n)] for _ in range(8)]
    for i in range(8):
        z=[yy[v]+sum(aa[i][j]*U[j][v] for j in range(i)) for v in range(n)]
        f=F_mp(z)
        rhs=[taup*f[v]+gp*sum(CC[i][j]*U[j][v] for j in range(i)) for v in range(n)]
        uu=mp.lu_solve(W,mp.matrix(rhs))
        U[i]=list(uu)
    out=[yy[v]+sum(bb[i]*U[i][v] for i in range(8)) for v in range(n)]
    return np.array([[float(x) for x in row] for row in U]),np.array([float(x) for x in out])


def frontier_sweeps(y,k,g,c,a,C,b,tau,gamma,W):
    U=np.zeros((8,len(y)))
    rows=[]
    total=0
    for sweep in range(1,9):
        old=U.copy()
        # True causal prefix of length sweep-1 is immutable and not recomputed.
        for i in range(sweep-1,8):
            U[i]=np.linalg.solve(W,rhs_float(i,old,y,k,g,c,a,C,tau,gamma))
        charges=9-sweep
        total+=charges
        rows.append((sweep,U.copy(),charges,total))
    assert total==36
    return rows


def exact_stage_residuals(U,y,k,g,c,a,C,tau,gamma):
    n=len(y)
    yy=list(map(fr,y))
    Uq=[[fr(x) for x in row] for row in U]
    kq=list(map(fr,k));gq=list(map(fr,g))
    cq=[[fr(x) for x in row] for row in c]
    aq=[[fr(x) for x in row] for row in a]
    Cq=[[fr(x) for x in row] for row in C]
    tq=fr(tau);G=fr(gamma)
    sums=[sum(row) for row in cq]
    residual=[]
    shifted_states=[]
    Wq=[]
    for v in range(n):
        Wq.append([(1+tq*(kq[v]+sums[v]+3*gq[v]*yy[v]**2))
                   if w==v else -tq*cq[v][w] for w in range(n)])
    for i in range(8):
        z=[yy[v]+sum(aq[i][j]*Uq[j][v] for j in range(i)) for v in range(n)]
        f=f_fraction(z,k,g,c)
        stage_r=[]
        for v in range(n):
            lhs=sum(Wq[v][w]*Uq[i][w] for w in range(n))
            rhs=tq*f[v]+G*sum(Cq[i][j]*Uq[j][v] for j in range(i))
            stage_r.append(lhs-rhs)
        residual.append(stage_r)
        shifted_states.append(z)
    return residual,shifted_states


def bound_by_lower_triangle(U,y,k,g,c,a,C,b,tau,gamma):
    """Exact-real source-conditioned a-posteriori stage error bound.

    Uses represented coefficients, positive M-matrix inverse row bound and
    a finitely propagated source-cubic Jacobian envelope over a proven tube.
    The reported output floats are upwards-rounded via exact Fraction.
    """
    rr,states=exact_stage_residuals(U,y,k,g,c,a,C,tau,gamma)
    n=len(y)
    tq=fr(tau)
    G=fr(gamma)
    kk=list(map(fr,k));gg=list(map(fr,g))
    cq=[[fr(x) for x in row] for row in c]
    aq=[[fr(x) for x in row] for row in a]
    Cq=[[fr(x) for x in row] for row in C]
    bq=list(map(fr,b))
    yq=list(map(fr,y))
    s=[sum(row) for row in cq]
    J_W_row_sum_min=min(1+tq*(kk[v]+3*gg[v]*yq[v]**2) for v in range(n))
    beta=1/J_W_row_sum_min
    errors=[]
    per=[]
    for i in range(8):
        residual=max(abs(x) for x in rr[i])
        state_shift=sum(abs(aq[i][j])*errors[j] for j in range(i))
        tube=max(abs(x) for x in states[i])+state_shift
        lipschitz=max(kk[v]+2*s[v]+3*gg[v]*tube*tube for v in range(n))
        accumulation=sum((tq*lipschitz*abs(aq[i][j])+
                          abs(G*Cq[i][j]))*errors[j] for j in range(i))
        e_rational=beta*(residual+accumulation)
        upper=ceil_f(e_rational)
        if upper is None:
            return {"certificate":"BLOCKED_OVERFLOW",
                    "max_residual_upper":None,"max_error_upper":None,
                    "endpoint_bound_upper":None}
        # Make every recursive stage bound a represented upward-safe float.
        errors.append(fr(upper))
        per.append({
          "stage":i+1,
          "exact_residual_upper":ceil_f(residual),
          "error_bound_up":upper,
          "tube_inf_up":ceil_f(tube),
          "local_lipschitz_up":ceil_f(lipschitz),
        })
    endpoint=sum(abs(bq[i])*errors[i] for i in range(8))
    bound=ceil_f(endpoint)
    residual_max=max(abs(x) for row in rr for x in row)
    return {
      "certificate":"EXACT_BINARY64_AS_RATIONAL_SOURCE_SCOPED",
      "max_residual_upper":ceil_f(residual_max),
      "max_error_upper":max(float(e) for e in errors),
      "endpoint_bound_upper":bound,
      "stage_bound":per,
      "no_native_callback_rounding_certificate":True,
    }


def source_owned_linear_newton_free(rhs,W,tau,c,max_it=200):
    """Jacobi W solve with source-owned c, no derivative/JVP callback."""
    n=len(rhs)
    diag=np.diag(W)
    z=np.zeros(n,dtype=float)
    q=max(tau*sum(row)/diag[i] for i,row in enumerate(c))
    assert 0<=q<1
    work=0
    for it in range(max_it):
        zz=np.empty(n)
        for i in range(n):
            zz[i]=(rhs[i]+tau*sum(c[i][j]*z[j] for j in range(n) if j!=i))/diag[i]
        work+=n*n
        if np.max(np.abs(zz-z))<5e-16:
            z=zz
            break
        z=zz
    return z,{"iterations":it+1,"component_arithmetic_work_proxy":work,"contraction_ratio_bound":q,
              "exact_original_W_residual_inf":float(np.max(np.abs(W@z-rhs))),
              "no_jacobian_callback":True,"no_newton":True}


def strictly_lower_counterexample():
    """Noncontractive yet finite causal propagation in 8 real scalar stages."""
    coeff=2.5
    true=np.ones(8,float)
    for i in range(1,8):true[i]=1+coeff*true[i-1]
    u=np.zeros(8,float)
    history=[]
    for sweep in range(1,9):
        old=u.copy()
        u[0]=1
        for i in range(1,8):u[i]=1+coeff*old[i-1]
        history.append({"sweep":sweep,"max_true_error":float(np.max(np.abs(u-true)))})
    assert history[-1]["max_true_error"]==0
    assert history[2]["max_true_error"]>1
    return {"strictly_lower_block_norm_inf":coeff,
            "global_contraction":False,
            "8_sweeps_exact_arithmetic_double_representable":True,
            "at_three_sweeps_genuine_error":history[2]["max_true_error"],
            "history":history}


def main():
    check_locks()
    models=load_cases()
    a,C,b,gamma=tableau()
    records=[]
    for name,steps in ORIGINAL_STEPS.items():
        k,g,c,y=models[name]
        h,tau,W=build_case(k,g,c,y,steps,gamma)
        u_ref,endpoint_ref=high_precision_reference(y,k,g,c,a,C,b,tau,gamma)
        u_seq,endpoint_seq=protected_sequential(y,k,g,c,a,C,b,tau,gamma,W)
        seqres=bound_by_lower_triangle(u_seq,y,k,g,c,a,C,b,tau,gamma)
        assert seqres["certificate"].startswith("EXACT_")
        eps=float(np.max(np.abs(u_seq-u_ref)))
        assert eps<2e-12,(name,eps)
        rows=[]
        for sweep,U,charge,total in frontier_sweeps(y,k,g,c,a,C,b,tau,gamma,W):
            cert=bound_by_lower_triangle(U,y,k,g,c,a,C,b,tau,gamma)
            endpoint=np.asarray(y)+b@U
            err=float(np.max(np.abs(endpoint-endpoint_ref)))
            linf=float(np.max(np.abs(U-u_ref)))
            assert cert["certificate"]=="EXACT_BINARY64_AS_RATIONAL_SOURCE_SCOPED"
            # High precision is only a numerical crosscheck; mathematical
            # validity of the bound follows from exact Fraction recursion.
            assert err<=cert["endpoint_bound_upper"]+5e-14,(name,sweep,err,cert)
            assert linf<=cert["max_error_upper"]+5e-14,(name,sweep,linf,cert)
            rows.append({
                "sweep":sweep,"q_name":f"frontier_q{sweep-1}",
                "serial_stage_depth":sweep,
                "last_sweep_original_W_solves":charge,
                "cumulative_original_W_solves":total,
                "all_original_stages_residual_max":cert["max_residual_upper"],
                "rigorous_source_scoped_stage_bound_inf":cert["max_error_upper"],
                "rigorous_source_scoped_endpoint_bound_inf":cert["endpoint_bound_upper"],
                "actual_high_precision_stage_linf_error":linf,
                "actual_high_precision_endpoint_linf_error":err,
                "original_target_error_certificate":"SOURCE_EXACT_REPRESENTED_COEFFICIENTS_ONLY",
                "native_full_step_admission":"NOT_ADMITTED_NO_ORIGINAL_EMBEDDED_ERROR_ALLOWANCE",
                "stage_certificates":cert["stage_bound"],
            })
        assert len(rows)==8
        # Original first U stage with the original nonlinear source RHS.
        first_rhs=rhs_float(0,u_seq,np.asarray(y),k,g,c,a,C,tau,gamma)
        U_linear,inner=source_owned_linear_newton_free(first_rhs,W,tau,c)
        assert np.max(np.abs(U_linear-u_seq[0]))<2e-13
        records.append({
            "case":name,"source_family":("asymmetric_2" if name=="asymmetric_2_refined" else name),
            "independent_physical_model":name!="asymmetric_2_refined",
            "initial_state":y,"h":h,"h_gamma":tau,"steps_original":steps,
            "protected_sequential":{"original_W_stage_solves":8,"serial_stage_depth":8,
                 "stage_linf_reference_error":eps,
                 "endpoint":endpoint_seq.tolist(),
                 "reference_endpoint":endpoint_ref.tolist(),
                 "source_scoped_exact_rational_residual_certificate":seqres},
            "source_owned_newton_free_first_W_Jacobi":inner,
            "frontier_q0_to_q7":rows,
        })
    assert len({x["source_family"] for x in records})==3
    assert records[0]["initial_state"]==records[1]["initial_state"]
    no_go=strictly_lower_counterexample()
    check_locks()
    out={
      "schema":"vigilode.r6.original_eight_stage_causal_frontier.v1",
      "base_head":PARENT,
      "scientific_status":"DERIVED_AND_NUMERICALLY_CHECKED_SOURCE_SCOPED_ONLY",
      "source_sha256_tableau":TAB_SHA256,"frozen_source_git_blobs":BLOBS,
      "all_inputs_original":True,"original_raw_U_stage_count":8,
      "tested_original_fixture_rows":len(records),"distinct_models":3,
      "sweep_depth_rows":sum(len(x["frontier_q0_to_q7"]) for x in records),
      "original_solver_changed":False,"existing_tolerances_changed":False,
      "global_contraction_needed_for_causal_finite_sweeps":False,
      "exact_arithmetic_finite_sweep_depth":8,
      "causal_frontier_full_depth_W_work":36,
      "protected_serial_full_depth_W_work":8,
      "q2_causal_depth":3,"q2_cumulative_W_solve_work":21,
      "frontier_protocol_id":"UNACCELERATED_SIMULTANEOUS_CAUSAL_FRONTIER_V1",
      "native_transactional_q1_q2_implemented_comparator_executed":False,
      "runtime_newton_iterations":0,"runtime_jvp_callbacks":0,
      "full_native_Rust_8_stage_parity":"NOT_RUN",
      "full_IVP_matched_achieved_global_error":"NOT_RUN",
      "full_real_performance_claim":"NOT_ADMITTED",
      "protected_fallback_required_if_error_gate_fails":True,
      "nonnormal_negative_control":no_go,
      "records":records,
      "official_dag":"7/23","successor_admitted":"0/12",
      "production":"HOLD","independent_author_C9":"NOT_RUN",
    }
    HERE.mkdir(parents=True,exist_ok=True)
    final=HERE/"RESULTS.json"
    final.write_text(json.dumps(out,indent=2,sort_keys=True,allow_nan=False)+"\n")
    print("R6_ORIGINAL_RAW_U_ROWS",len(records),"DEPTH_SAMPLES",out["sweep_depth_rows"],"PARENT",PARENT)
    for item in records:
        q2=item["frontier_q0_to_q7"][2];q7=item["frontier_q0_to_q7"][7]
        print(item["case"],"protected_8_work=8", "q2_work=21",
              "q2_error=",q2["actual_high_precision_endpoint_linf_error"],
              "q2_bound=",q2["rigorous_source_scoped_endpoint_bound_inf"],
              "q7_error=",q7["actual_high_precision_endpoint_linf_error"],
              "q7_bound=",q7["rigorous_source_scoped_endpoint_bound_inf"],
              "inner_W_jacobi_iter=",item["source_owned_newton_free_first_W_Jacobi"]["iterations"])
    print("EXPERIMENT_RESULT_SHA256",hashlib.sha256(final.read_bytes()).hexdigest())


if __name__=="__main__":
    main()
