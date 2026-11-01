#!/usr/bin/env python3
"""Source-conditioned R5 research: exact-source nonlinear M-matrix homotopy,
strictly Newton-free candidate and original first RODAS5P linear U-stage witness.

No modification to protected integrator, original source, defaults or tolerances.
The standalone stage experiment is not a full-IVP or production result.
"""
from __future__ import annotations
import hashlib
import importlib.util
import json
import math
import struct
from dataclasses import dataclass
from fractions import Fraction as F
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
HERE=Path(__file__).resolve().parent
FIXTURE=ROOT/"crates/vigilode-coupled/examples/coupled_achieved_wp.rs"
RHS=ROOT/"crates/vigilode-coupled/src/coupled_inverse_be.rs"
RODAS_SRC=ROOT/"crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs"
HOMO_SRC=ROOT/"crates/vigilode-coupled/src/rhs_homotopy.rs"
TABLEAU=ROOT/"fixtures/rodas5p_coefficients_snapshot.json"
PRIOR=ROOT/"research/vigilode_wp9b_measured_search_correction_20261101/experiment.py"
LOCKS={
    str(FIXTURE.relative_to(ROOT)):"1bfc17ec2ed4b902cc06073807388f6e332e31d6",
    str(RHS.relative_to(ROOT)):"d78c5c5f6f476f2a5b1ea88628ed25daa6cfc482",
    str(RODAS_SRC.relative_to(ROOT)):"d68239d53977e36f98b186b627aa0c4c5c7de7cb",
    str(HOMO_SRC.relative_to(ROOT)):"990103d9ab5d59b7c40f3ccd323f6ad52e504324",
}
LOCK_TABLEAU_SHA256="211255f6125b648c6955833c3da537d9c031ec78168b2918563c29fa88c8bce6"
LAMBDAS=(0.0,0.25,0.50,0.75,1.0)
INNER_BISECTIONS=55
MAX_SWEEPS=250
ITER_TOL=1e-16
EXACT_RESIDUAL_THRESHOLD=F(1,10**12)
PREREG_SHA="87e618b4848cbaa41942d8d92efdf9c9cbdd83c7"


def blob(payload: bytes)->str:
    return hashlib.sha1(b"blob "+str(len(payload)).encode()+b"\0"+payload).hexdigest()


def locked_cases():
    for relative,expected in LOCKS.items():
        assert blob((ROOT/relative).read_bytes())==expected,("SOURCE_IDENTITY",relative)
    assert hashlib.sha256(TABLEAU.read_bytes()).hexdigest()==LOCK_TABLEAU_SHA256
    module=importlib.util.spec_from_file_location("frozen_wp9b_cases",PRIOR)
    assert module is not None and module.loader is not None
    m=importlib.util.module_from_spec(module)
    module.loader.exec_module(m)
    m.verify_locks()
    return m.source_cases()


def hex_bits(value:float):
    return struct.pack(">d",float(value)).hex()


def exact(value:float)->F:
    return F.from_float(float(value))


@dataclass
class Meter:
    scalar_cubic_evals:int=0
    jacobi_sweeps:int=0
    coupling_products:int=0
    rejected:int=0
    stations:int=0

    def record(self):
        return vars(self).copy()


def params(model):
    k=[float(v) for v in model["k"]]
    g=[float(v) for v in model["g"]]
    c=[[float(v) for v in row] for row in model["c"]]
    b=[float(v) for v in model["y0"]]
    assert all(v>0 for v in k)
    assert all(v>=0 for v in g+b)
    assert len(c)==len(k)
    assert all(len(row)==len(k) and all(v>=0 for v in row) for row in c)
    assert all(c[i][i]==0 for i in range(len(k)))
    return k,g,c,b


def scalar_inverse(rhs,d,a,meter):
    """Positive cubic inverse by bisection, no Newton/derivative callback."""
    assert rhs>=0 and d>0 and a>=0
    if rhs==0:
        return 0.0
    if a==0.0:
        return rhs/d
    lo,hi=0.0,rhs/d
    for _ in range(INNER_BISECTIONS):
        mid=(lo+hi)/2
        meter.scalar_cubic_evals+=1
        f=(d*mid+a*mid**3)-rhs
        if f>0:
            hi=mid
        else:
            lo=mid
        if hi==lo:
            break
    return (lo+hi)/2


def q_contraction(k,c,tau,lam):
    return max(lam*tau*sum(row)/(1+tau*(k[i]+sum(row))) for i,row in enumerate(c))


def nonlinear_map(z,k,g,c,b,tau,lam,meter):
    out=[]
    for i in range(len(k)):
        s=sum(c[i][j]*z[j] for j in range(len(k)) if i!=j)
        meter.coupling_products+=len(k)-1
        d=1+tau*(k[i]+sum(c[i]))
        out.append(scalar_inverse(b[i]+lam*tau*s,d,tau*g[i],meter))
    return out


def nonlinear_residual_exact(z,k,g,c,b,tau,lam):
    T=F(tau); L=F(lam)
    ans=[]
    for i in range(len(k)):
        val=(1+T*(exact(k[i])+sum(map(exact,c[i]))))*exact(z[i])
        val+=T*exact(g[i])*exact(z[i])**3
        val-=L*T*sum(exact(c[i][j])*exact(z[j]) for j in range(len(k)) if j!=i)
        val-=exact(b[i])
        ans.append(val)
    return ans


def exact_upper(frac:F):
    # round TOWARDS +inf: the binary64 endpoint is conservative.
    f=float(frac)
    if exact(f)<frac:
        return math.nextafter(f,math.inf)
    return f


def m_analytic(k,c,tau,lam):
    return min(1+tau*(k[i]+(1-lam)*sum(row)) for i,row in enumerate(c))


def solve_nonlinear(k,g,c,b,tau,lams,zero_start):
    assert lams and lams[-1]==1.0 and all(0<=x<=1 for x in lams)
    meter=Meter()
    z=[0.0]*len(k) if zero_start else list(b)
    path=[]
    for lam in lams:
        assert q_contraction(k,c,tau,lam)<1
        done=False
        for sweep in range(MAX_SWEEPS):
            nxt=nonlinear_map(z,k,g,c,b,tau,lam,meter)
            meter.jacobi_sweeps+=1
            if max(abs(x-y) for x,y in zip(nxt,z))<ITER_TOL:
                z=nxt
                done=True
                break
            z=nxt
        meter.stations+=1
        rr=nonlinear_residual_exact(z,k,g,c,b,tau,lam)
        # Compute exact-real diagonal dominance margin, not rounded-down
        # denominator from a binary64 intermediate (which could overclaim).
        exact_margin=min(
            1+exact(tau)*(exact(k[i])+(1-exact(lam))*sum(map(exact,c[i])))
            for i in range(len(k))
        )
        bound=max(abs(x) for x in rr)/exact_margin
        # Even if roundoff defeats iteration stopping, the *independent
        # exact original-equation residual* is the scientific accept gate.
        if not done and bound>EXACT_RESIDUAL_THRESHOLD:
            meter.rejected+=1
        path.append({"lambda":lam, "state":z[:], "sweeps":sweep+1,
                     "q_infinity":q_contraction(k,c,tau,lam),
                     "residual_inf_upper":exact_upper(max(abs(x) for x in rr)),
                     "error_bound_inf_upper":exact_upper(bound),
                     "residual_gate":bound<=EXACT_RESIDUAL_THRESHOLD,
                     "converged_by_step":done})
    return path,meter.record()


def source_linear_first_rodas(k,g,c,b,steps,gamma):
    """Exact first U-stage structure. Prescribed RHS τ f(y0), τ=hγ.
    No assertion of bit-for-bit identity to native Rust evaluation ordering.
    """
    tau=(0.1/steps)*gamma
    n=len(k)
    rhs=[]
    diag=[]
    for i in range(n):
        fx=-k[i]*b[i]-g[i]*b[i]**3
        fx+=sum(c[i][j]*(b[j]-b[i]) for j in range(n) if i!=j)
        rhs.append(tau*fx)
        diag.append(1+tau*(k[i]+sum(c[i])+3*g[i]*b[i]**2))
    return tau,rhs,diag


def solve_linear_rodas(k,g,c,b,steps,gamma):
    tau,rhs,diag=source_linear_first_rodas(k,g,c,b,steps,gamma)
    n=len(k); z=[0.0]*n
    attempted=0
    for step in range(MAX_SWEEPS):
        zz=[]
        for i in range(n):
            zz.append((rhs[i]+tau*sum(c[i][j]*z[j] for j in range(n) if j!=i))/diag[i])
            attempted+=1
        if max(abs(x-y) for x,y in zip(zz,z))<ITER_TOL:
            z=zz
            break
        z=zz
    else:
        raise RuntimeError("linear original RODAS first U-stage approximate Jacobi failed")
    # Exact Fraction values from frozen represented k,g,c,y0, gamma,h.
    T=exact(tau)
    residuals=[]
    for i in range(n):
        f=-exact(k[i])*exact(b[i])-exact(g[i])*exact(b[i])**3
        f+=sum(exact(c[i][j])*(exact(b[j])-exact(b[i]))
               for j in range(n) if i!=j)
        w=(1+T*(exact(k[i])+sum(map(exact,c[i]))+
                 3*exact(g[i])*exact(b[i])**2))*exact(z[i])
        w-=T*sum(exact(c[i][j])*exact(z[j]) for j in range(n) if j!=i)
        residuals.append(w-T*f)
    resid=max(abs(x) for x in residuals)
    lower=1+exact(tau)*min(map(exact,k))
    return {"h_gamma":tau,"gamma":gamma,"first_stage_U":z,
            "reference_rhs_source":"tau*f(y0) autonomous first raw-U stage",
            "max_original_raw_U_residual_exact_upper":exact_upper(resid),
            "max_stage_error_bound_exact_upper":exact_upper(resid/lower),
            "stage_system_rank":n,"Jacobi_component_operations":attempted,
            "iterations":step+1,"requires_callback_JVP":False,
            "runtime_derivative_analytical_source":True,
            "not_full_8stage_trajectory":True}


def assert_path_and_margins(path,k,g,c,b,tau):
    inf=max(b)
    kmin=min(k)
    smax=max(sum(row) for row in c)
    for old,new in zip(path,path[1:]):
        dt=new["lambda"]-old["lambda"]
        lipschitz=dt*tau*smax*inf/(1+tau*kmin)**2
        assert all(new_i>=old_i-3e-15 for new_i,old_i in zip(new["state"],old["state"])),(old,new)
        assert max(abs(new_i-old_i) for new_i,old_i in zip(new["state"],old["state"]))<=lipschitz+5e-15,(old,new,lipschitz)
    for row in path:
        z=row["state"];lam=row["lambda"]
        assert all(v>=0 for v in z)
        assert max(z)<=max(b)/(1+tau*kmin)+1e-14
        assert all(1+tau*(k[i]+sum(c[i]))+3*tau*g[i]*z[i]**2-
                   lam*tau*sum(c[i])>=1+tau*kmin-1e-15
                   for i in range(len(k)))
        assert row["residual_gate"]
    return True


def source_identity(models):
    return {name:{
        "k_bits":[hex_bits(x) for x in params(model)[0]],
        "g_bits":[hex_bits(x) for x in params(model)[1]],
        "c_bits":[[hex_bits(x) for x in row] for row in params(model)[2]],
        "y0_bits":[hex_bits(x) for x in params(model)[3]],
    } for name,model in models.items()}


def run():
    models=locked_cases()
    t=json.loads(TABLEAU.read_text())
    gamma=float(t["gamma"])
    assert 0<gamma<1
    configs={"asymmetric_2":64,"asymmetric_2_refined":128,
             "coupled_3_cubic":128,"linear_exchange_control":128}
    records=[]
    for name,steps in configs.items():
        k,g,c,b=params(models[name]);tau=1.0/steps
        base,meter_base=solve_nonlinear(k,g,c,b,tau,(1.0,),True)
        hom,meter_hom=solve_nonlinear(k,g,c,b,tau,LAMBDAS,True)
        assert_path_and_margins(hom,k,g,c,b,tau)
        assert base[-1]["residual_gate"] and hom[-1]["residual_gate"]
        same_phys=("asymmetric_2" if name.endswith("_refined") else name)
        records.append({
            "source":name,"physical_family":same_phys,
            "source_equivalence_control":name.endswith("_refined"),
            "tau_stage_only":tau,"original_fixture_steps":steps,
            "baseline_value_only_picard":{"path":base,"counters":meter_base},
            "newton_free_homotopy":{"path":hom,"counters":meter_hom},
            "homotopy_cubic_eval_over_baseline":meter_hom["scalar_cubic_evals"] / max(meter_base["scalar_cubic_evals"],1),
            "strong_fallback_if_no_gain":"protected_original_RODAS5P",
            "rodas_original_first_U_stage":solve_linear_rodas(k,g,c,b,steps,gamma),
        })
    # Overarching controlled negative witness: F(z)=2z-z^3 with
    # k=1,tau=1,g=-1,c=0,b=0 has 3 real roots.
    negative={"polynomial":"2*z-z^3","roots_exact":["-sqrt(2)","0","+sqrt(2)"],
              "reason":"g<0 violates monotonicity / M-matrix",
              "source_admissible":False}
    for value in (0.0,math.sqrt(2),-math.sqrt(2)):
        assert abs(2*value-value**3)<1e-14
    assert all(not x["source_equivalence_control"] for x in records if x["source"]!="asymmetric_2_refined")
    assert len({x["physical_family"] for x in records})==3
    for source,blobid in LOCKS.items():
        assert blob((ROOT/source).read_bytes())==blobid
    return {
        "schema":"vigilode.r5.monotone_newton_free_algebraic_homotopy.v1",
        "preregistration_commit":PREREG_SHA,
        "frozen_source_blobs":LOCKS,
        "frozen_rodas_tableau_sha256":LOCK_TABLEAU_SHA256,
        "source_families_distinct":3,"cohorts_with_resolution_control":4,
        "nonlinear_station_cases":sum(len(x["newton_free_homotopy"]["path"]) for x in records),
        "rodas_first_stage_cases":len(records),
        "reference_tolerances_changed":False,
        "no_production_write":True,
        "scientific_verdict":"DERIVED_NUMERICALLY_CHECKED_SCOPED_STAGE_ONLY",
        "newton_iterations_runtime":0,
        "jacobian_callbacks_runtime":0,
        "cross_model_risk":"Scalar implicit/BE root is NOT the original RODAS U-stage",
        "records":records,"negative_control":negative,
        "original_dag":"7/23","successor_admitted":"0/12",
        "production":"HOLD",
        "full_native_8stage_and_same_error":"NOT_RUN",
        "external_referee":"NOT_RUN",
        "data_contract":"Binary64 input values preserved; exact Fraction source residual; no outward interval arithmetic on native callbacks",
    }


def main():
    out=run()
    HERE.mkdir(exist_ok=True,parents=True)
    path=HERE/"RESULTS.json"
    path.write_text(json.dumps(out,indent=2,sort_keys=True,allow_nan=False)+"\n")
    print("R5_NEWTON_FREE_SOURCE_STAGE",out["nonlinear_station_cases"],
          "homotopy stations",out["rodas_first_stage_cases"],"RODAS first-stage checks")
    for r in out["records"]:
        print(r["source"],"baseline_cubic_evals",r["baseline_value_only_picard"]["counters"]["scalar_cubic_evals"],
              "homotopy_cubic_evals",r["newton_free_homotopy"]["counters"]["scalar_cubic_evals"],
              "rho_hom",r["newton_free_homotopy"]["path"][-1]["residual_inf_upper"],
              "original_U_rho",r["rodas_original_first_U_stage"]["max_original_raw_U_residual_exact_upper"])
    print("RESULT_SHA256",hashlib.sha256(path.read_bytes()).hexdigest())


if __name__=="__main__":
    main()
