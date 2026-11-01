#!/usr/bin/env python3
"""R9 original RODAS5P raw-U: cycle-core Schur + finite DAG path inverse.

Source-locked original R6/R8 equations, independent high precision original
first-step reference; synthetic sparse graph algorithm controls are labeled.
No native production integrator, no new full IVP performance promotion.
"""
from __future__ import annotations
from fractions import Fraction as F
from pathlib import Path
import hashlib, importlib.util, json, math, sys, time
import numpy as np
import sympy as sp

ROOT=Path(__file__).resolve().parents[2]
HERE=Path(__file__).resolve().parent
R8=ROOT/"research/vigilode_r8_source_owned_W_cayley_20261101/experiment.py"
R6=ROOT/"research/vigilode_r6_eightstage_causal_frontier_20261101/experiment.py"
R7=ROOT/"research/vigilode_r7_native_transactional_discriminant_20261101/RESULTS.json"
PREREG="4985060e598e8c039d759b9dd43e61a783cd2808"
SOURCE_R8_SHA="61adc2c39a191b14f7e705dc25fc56914ccaf147fe26331e210ec98c6babe833"
SOURCE_R7_SHA="583e2fada6050a25dfcebd89b682edf45929a8da717b9a2385f628bed47094d4"


def import_module(name,path):
    spec=importlib.util.spec_from_file_location(name,path)
    module=importlib.util.module_from_spec(spec)
    sys.modules[spec.name]=module
    spec.loader.exec_module(module)
    return module


def load_original():
    r6=import_module("vigilode_r9_original_r6",R6)
    r6.check_locks()
    assert hashlib.sha256((R8.parent/"RESULTS.json").read_bytes()).hexdigest()==SOURCE_R8_SHA
    assert hashlib.sha256(R7.read_bytes()).hexdigest()==SOURCE_R7_SHA
    return r6


def rr(x):return F.from_float(float(x))
def sym(x):return sp.Rational(x.numerator,x.denominator)
def ff(x):
    z=sp.Rational(x)
    return F(int(z.p),int(z.q))


def qmatrix(k,g,c,y,tau):
    n=len(k)
    assert n==len(g)==len(c)==len(y) and n>0
    assert float(tau)>0 and all(a>0 for a in k)
    assert all(z>=0 for z in g)
    assert all(len(row)==n and row[i]==0 and all(x>=0 for x in row)
               for i,row in enumerate(c)),"NON_COOPERATIVE_SOURCE"
    t=rr(tau)
    sums=[sum(map(rr,row)) for row in c]
    diag=[1+t*(rr(k[i])+sums[i]+3*rr(g[i])*rr(y[i])**2) for i in range(n)]
    T=[[F(0) if i==j else t*rr(c[i][j])/diag[i] for j in range(n)]
       for i in range(n)]
    W=[[diag[i] if i==j else -t*rr(c[i][j]) for j in range(n)] for i in range(n)]
    q=max(sum(row) for row in T)
    m=min(diag[i]-t*sums[i] for i in range(n))
    assert 0<=q<1 and m>1
    return dict(T=T,W=W,diag=diag,q=q,m=m,n=n)


def topological_order(T,A):
    """Source declared feedback set accepted iff induced A dependency DAG."""
    order=[]
    aset=set(A)
    pending={i:sum(T[i][j]!=0 for j in A if j!=i) for i in A}
    outgoing={i:[] for i in A}
    for i in A:
        assert T[i][i]==0
        for j in A:
            if i!=j and T[i][j]!=0:
                outgoing[j].append(i)
    queue=sorted(i for i in A if pending[i]==0)
    while queue:
        i=queue.pop(0);order.append(i)
        for j in outgoing[i]:
            pending[j]-=1
            if pending[j]==0:
                queue.append(j)
        queue.sort()
    if len(order)!=len(A):
        raise ValueError("A_GRAPH_HAS_CYCLE_SOURCE_FVS_INCORRECT")
    level={i:0 for i in order}
    for i in order:
        dep=[level[j]+1 for j in A if T[i][j]!=0]
        if dep:level[i]=max(dep)
    return order,max(level.values(),default=0)


def matvec(M,x):
    return [sum(M[i][j]*x[j] for j in range(len(x))) for i in range(len(M))]


def create_provider(k,g,c,y,tau,B):
    data=qmatrix(k,g,c,y,tau)
    T=data["T"];n=data["n"]
    if len(set(B))!=len(B) or not all(0<=i<n for i in B):
        raise ValueError("INVALID_FVS_INDICES")
    if len(B)>3:
        raise ValueError("CORE_TOO_LARGE_PROTECTED_FALLBACK")
    A=[i for i in range(n) if i not in B]
    order,depth=topological_order(T,A)
    def dag_solve(z):
        x=[F(0)]*n
        for i in order:
            x[i]=z[i]+sum(T[i][j]*x[j] for j in A if T[i][j])
        return x

    # Precompute independent A path matrix columns, no path enumeration.
    cols=[]
    for j in B:
        z=[T[i][j] for i in range(n)]
        cols.append(dag_solve(z))
    Z=[[T[i][j]+sum(T[i][a]*cols[jj][a] for a in A)
         for jj,j in enumerate(B)] for i in B]
    S=[[F(int(i==j))-Z[i][j] for j in range(len(B))] for i in range(len(B))]
    if B:
        core=sp.Matrix([[sym(x) for x in row] for row in S])
        assert ff(core.det())>0
        ch=[ff(x) for x in core.charpoly().all_coeffs()]
        degree=len(B)
        polynomial=core**(degree-1)
        for power in range(degree-2,-1,-1):
            polynomial+=sym(ch[degree-1-power])*core**power
        inverse=(-1/sym(ch[-1]))*polynomial
        assert inverse==core.inv()
        Sinv=[[ff(inverse[i,j]) for j in range(degree)] for i in range(degree)]
        assert all(x>=0 for row in Sinv for x in row)
        assert all(S[i][j]<=0 for i in range(degree) for j in range(degree) if i!=j)
        assert all(sum(Z[i])<1 for i in range(degree))
    else:
        ch=[];Sinv=[]

    def solve(rhs):
        b=[rhs[i]/data["diag"][i] for i in range(n)]
        x=dag_solve(b)
        rhsB=[b[i]+sum(T[i][a]*x[a] for a in A) for i in B]
        ub=[sum(Sinv[i][j]*rhsB[j] for j in range(len(B))) for i in range(len(B))]
        u=x[:]
        for ii,i in enumerate(B):
            u[i]=ub[ii]
        for i in A:
            u[i]=x[i]+sum(cols[jj][i]*ub[jj] for jj in range(len(B)))
        residual=[w-r for w,r in zip(matvec(data["W"],u),rhs)]
        assert not any(residual),("EXACT_CORE_SCHUR_RESIDUAL_NONZERO",residual)
        return u

    data.update(A=A,B=B,core_charpoly=ch,core_matrix=S,core_inverse=Sinv,
                topological_order=order,depth=depth,solver=solve,
                base_edges=sum(T[i][j]!=0 for i in range(n) for j in range(n)),
                A_edges=sum(T[i][j]!=0 for i in A for j in A),
                core_edges=sum(z!=0 for row in Z for z in row),
                precompute_dag_solve_count=len(B),
                setup_path_propagations=len(B),
                per_stage_path_propagations=1,
                forbidden_independent_source=False)
    return data


def synthetic(name,n):
    k=[4.+float(i%4) for i in range(n)]
    g=[2. if i%5 else 0. for i in range(n)]
    y=[0.01+0.005*(i%4) for i in range(n)]
    c=[[0.]*n for _ in range(n)]
    if name in ("ring12","ring24"):
        for i in range(1,n):c[i][i-1]=24.
        c[0][n-1]=24.
        B=[0]
    elif name=="two_cycles14":
        c[0][1]=24.;c[1][0]=18.
        c[2][3]=20.;c[3][2]=15.
        for i in range(4,n):c[i][i-1]=10.
        c[0][n-1]=8.;c[2][n-1]=7.
        B=[0,2]
    elif name=="dag8":
        for i in range(1,n):c[i][i-1]=18.
        B=[]
    elif name=="dense8_refuse":
        for i in range(n):
            for j in range(n):
                if i!=j:c[i][j]=8.
        B=[0,1,2] # Not a valid feedback vertex set.
    elif name=="four_cycles_refuse":
        for i in range(0,n,2):
            c[i][i+1]=16.;c[i+1][i]=14.
        B=[0,2,4,6]
    else:raise ValueError(name)
    return k,g,c,y,B


def run_case(name,k,g,c,y,h,gamma,a,C,b,B,orig,mod):
    tau=float(h*gamma)
    P=create_provider(k,g,c,y,tau,B)
    n=len(y);U=np.zeros((8,n),float)
    for stage in range(8):
        r=mod.rhs_float(stage,U,np.asarray(y),k,g,c,a,C,tau,gamma)
        exact=P["solver"]([rr(x) for x in r])
        U[stage]=[float(v) for v in exact]
    W_float=np.array([[float(t) for t in row] for row in P["W"]])
    protected,protected_end=mod.protected_sequential(y,k,g,c,a,C,b,tau,gamma,W_float)
    output=np.asarray(y)+b@U
    difference=float(np.max(np.abs(output-protected_end)))
    assert difference<1e-11,(name,difference)
    bound=mod.bound_by_lower_triangle(U,y,k,g,c,a,C,b,tau,gamma)
    assert bound["certificate"]=="EXACT_BINARY64_AS_RATIONAL_SOURCE_SCOPED"
    assert difference<=bound["endpoint_bound_upper"]+2e-13
    if orig:
        ref,refend=mod.high_precision_reference(y,k,g,c,a,C,b,tau,gamma)
        error=float(np.max(np.abs(output-refend)))
        assert error<=bound["endpoint_bound_upper"]+5e-14
    else:error=None
    return {
        "name":name,"kind":"ORIGINAL_FROZEN" if orig else "SYNTHETIC_ALGORITHM_CONTROL",
        "physical_independent":orig and name!="asymmetric_2_refined",
        "n":n,"B":B,"B_size":len(B),"A_size":len(P["A"]),"tau":tau,
        "directed_edges":P["base_edges"],"acyclic_A_edges":P["A_edges"],
        "dag_longest_path_depth":P["depth"],
        "Schur_nonzero_entries":P["core_edges"],
        "M_q":float(P["q"]),"M_residual_margin":float(P["m"]),
        "exact_Cayley_Schur":True,"original_RAW_U_stages":8,
        "full_first_step_endpoint_linf_vs_protected":difference,
        "full_first_step_endpoint_linf_vs_90digit":error,
        "exact_rational_first_step_endpoint_bound":bound["endpoint_bound_upper"],
        "cost_proxy_not_wall_time":"A_path_edges_and_core_size_only",
        "native_wall_time":"NOT_RUN","newton_calls":0,"jvp_callbacks":0,
        "full_IVP_global_error":"NOT_RUN"
    }


def main():
    mod=load_original();a,C,b,gamma=mod.tableau()
    cases=mod.load_cases()
    names=mod.ORIGINAL_STEPS
    rows=[]
    for name in names:
        k,g,c,y=cases[name];steps=names[name]
        B=[0] if len(y)==2 else [0,1]
        h=float(0.1/steps)
        rows.append(run_case(name,k,g,c,y,h,gamma,a,C,b,B,True,mod))
    for name,n in (("ring12",12),("two_cycles14",14),("ring24",24),("dag8",8)):
        k,g,c,y,B=synthetic(name,n)
        rows.append(run_case(name,k,g,c,y,0.08,gamma,a,C,b,B,False,mod))
    negative=[]
    for name,n in (("dense8_refuse",8),("four_cycles_refuse",8)):
        k,g,c,y,B=synthetic(name,n)
        try:create_provider(k,g,c,y,0.02,B)
        except ValueError as e:negative.append({"name":name,"status":"REFUSED","type":str(e)})
        else:raise AssertionError("UNSUPPORTED_GRAPH_FALSE_ACCEPTED")
    k,g,c,y=cases["asymmetric_2"]
    c=[list(row) for row in c];c[0][1]=-1.
    try:create_provider(k,g,c,y,0.001,[0])
    except AssertionError:
        negative.append({"name":"negative_coupling","status":"REFUSED","type":"NON_COOPERATIVE_SOURCE"})
    else:raise AssertionError("NEGATIVE_COUPLING_FALSE_ACCEPTED")
    assert len(rows)==8 and len(negative)==3
    assert sum(x["physical_independent"] for x in rows)==3
    out={
       "schema":"vigilode.r9.feedback_cycle_schur_raw_U.v1",
       "preregistration_commit":PREREG,
       "original_R8_sha256":SOURCE_R8_SHA,"original_R7_sha256":SOURCE_R7_SHA,
       "mathematical_contract":"ORIGINAL_SOURCE_OWNED_CYCLE_CORE_SCHUR_WITH_FINITE_ACYCLIC_PATHS",
       "exact_nonnegative_inverse":"M_MATRIX_SCHUR_SOURCE_CONDITIONAL",
       "source_original_cases":4,"source_original_physical_independent":3,
       "synthetic_algorithmic_controls":4,"first_step_total_8stage_raw_U_solves":len(rows)*8,
       "new_FVS_small_core":True,"FVS_optimality":"NOT_CLAIMED",
       "negative_controls":negative,"rows":rows,
       "full_native_production_speed":"NOT_RUN",
       "entire_ODE_global_achieved_error":"NOT_RUN",
       "HOL_full_source_graph_theorem":"NOT_PROVED",
       "official_DAG":"7/23","successor_admitted":"0/12",
       "production":"HOLD","no_existing_solver_mutation":True
    }
    path=HERE/"RESULTS.json"
    path.write_text(json.dumps(out,indent=2,sort_keys=True,allow_nan=False)+"\n")
    print("R9_RAW_U",out["first_step_total_8stage_raw_U_solves"],
          "ORIGINAL",out["source_original_cases"],"SYNTH",out["synthetic_algorithmic_controls"],
          "REFUSED",len(negative))
    for r in rows:
        print("CASE",r["name"],"n",r["n"],"core",r["B_size"],
             "Adepth",r["dag_longest_path_depth"],"q",r["M_q"],
             "err",r["full_first_step_endpoint_linf_vs_protected"],
             "upper",r["exact_rational_first_step_endpoint_bound"])
    print("SHA256",hashlib.sha256(path.read_bytes()).hexdigest())
    return out

if __name__=="__main__":main()
