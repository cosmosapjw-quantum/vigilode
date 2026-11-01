#!/usr/bin/env python3
"""R10: source-owned associative affine scan inside original R9 graph-Schur ROW.

New math compares exact-rational Hillis-Steele logarithmic-depth path forest
against the original exact-R9 serial topological solve. Native wall excluded.
"""
from __future__ import annotations
from fractions import Fraction as F
from pathlib import Path
import hashlib, importlib.util, json, math, sys
import numpy as np

ROOT=Path(__file__).resolve().parents[2]
HERE=Path(__file__).resolve().parent
R9=ROOT/"research/vigilode_r9_feedback_cycle_schur_20261101/experiment.py"
R9_DATA=R9.parent/"RESULTS.json"
R9_SHA="a39becdbc2c5855669c42fedfd69f740a3a159263c314927c221ec5fd80bd828"
ORIGINAL_R9_HEAD="9694303d453c55ebcaf1ec4d851d0eb37cc87d52"
PREREG_HEAD="dd076af6fd279ff478a6503652199d59d35ed003"


def load():
    assert hashlib.sha256(R9_DATA.read_bytes()).hexdigest()==R9_SHA
    spec=importlib.util.spec_from_file_location("vigilode_r10_frozen_r9",R9)
    m=importlib.util.module_from_spec(spec)
    sys.modules[spec.name]=m
    spec.loader.exec_module(m)
    m.load_original().check_locks()
    return m


def compose(outer,inner):
    """Affine composition: F_outer ∘ F_inner, non-commutative monoid."""
    a,b=outer;c,d=inner
    return (a*c,b+a*d)


def path_forest(T,A):
    S=set(A)
    parent={i:[j for j in A if T[i][j]!=0 and i!=j] for i in A}
    children={i:[] for i in A}
    for i,prev in parent.items():
        if len(prev)>1:raise ValueError("NOT_PATH_FOREST_MULTI_PARENT")
        for j in prev:children[j].append(i)
    if any(len(v)>1 for v in children.values()):
        raise ValueError("NOT_PATH_FOREST_BRANCH")
    roots=sorted(i for i in A if not parent[i])
    chains=[];seen=set()
    for root in roots:
        chain=[];v=root
        while True:
            if v in seen:raise ValueError("NOT_PATH_FOREST_CYCLE")
            seen.add(v);chain.append(v)
            if not children[v]:break
            v=children[v][0]
        chains.append(chain)
    if seen!=S:raise ValueError("NOT_PATH_FOREST_CYCLE")
    return chains


class ScanProvider:
    def __init__(self,r9,k,g,c,y,tau,B):
        self.P=r9.create_provider(k,g,c,y,tau,B)
        self.T=self.P["T"];self.W=self.P["W"]
        self.B=self.P["B"];self.A=self.P["A"]
        self.chains=path_forest(self.T,self.A)
        self.scan_depth=max((max(0,(len(c)-1).bit_length()) for c in self.chains),default=0)
        self.serial_depth=max((len(c)-1 for c in self.chains),default=0)
        self.scan_pairs_per_apply=sum(
            sum(max(0,len(chain)-2**j) for j in range(max(0,(len(chain)-1).bit_length())))
            for chain in self.chains
        )
        self.serial_dependencies_per_apply=sum(len(chain)-1 for chain in self.chains)
        self.cols=[]
        for j in self.B:
            z=[self.T[i][j] for i in range(len(self.T))]
            self.cols.append(self.scan(z))
        self.B_solved_with_exact_CH=True

    def scan(self,z):
        T=self.T
        x=[F(0)]*len(T)
        for chain in self.chains:
            if not chain:continue
            maps=[]
            for v in chain:
                if v==chain[0]:maps.append((F(0),z[v]))
                else:
                    prior=chain[len(maps)-1]
                    maps.append((T[v][prior],z[v]))
            span=1
            while span<len(chain):
                prev=maps[:]
                for i in range(span,len(chain)):
                    maps[i]=compose(prev[i],prev[i-span])
                span*=2
            for v,(alpha,beta) in zip(chain,maps):
                assert alpha==0
                x[v]=beta
        return x

    def solve(self,rhs):
        T=self.T;P=self.P
        assert len(rhs)==len(T)
        b=[rhs[i]/P["diag"][i] for i in range(len(T))]
        x=self.scan(b)
        rhsB=[b[i]+sum(T[i][a]*x[a] for a in self.A) for i in self.B]
        ub=[sum(P["core_inverse"][i][j]*rhsB[j] for j in range(len(self.B))) for i in range(len(self.B))]
        u=x[:]
        for ii,i in enumerate(self.B):u[i]=ub[ii]
        for i in self.A:
            u[i]=x[i]+sum(self.cols[jj][i]*ub[jj] for jj in range(len(self.B)))
        resid=[v-w for v,w in zip(
           [sum(self.W[i][j]*u[j] for j in range(len(u))) for i in range(len(u))],rhs)]
        assert resid==[0]*len(rhs)
        return u


def source_record(r9,name,k,g,c,y,B,h,mod,a,C,b,gamma,phys):
    tau=float(h*gamma)
    native=r9.create_provider(k,g,c,y,tau,B)
    obj=ScanProvider(r9,k,g,c,y,tau,B)
    assert obj.scan_depth<=obj.serial_depth or obj.serial_depth==0
    n=len(y)
    u=np.zeros((8,n))
    for stage in range(8):
        rhs=mod.rhs_float(stage,u,np.asarray(y),k,g,c,a,C,tau,gamma)
        rf=[F.from_float(float(v)) for v in rhs]
        exact_scan=obj.solve(rf)
        exact_serial=native["solver"](rf)
        assert exact_scan==exact_serial
        u[stage]=[float(v) for v in exact_scan]
    cert=mod.bound_by_lower_triangle(u,y,k,g,c,a,C,b,tau,gamma)
    assert cert["certificate"]=="EXACT_BINARY64_AS_RATIONAL_SOURCE_SCOPED"
    assert cert["endpoint_bound_upper"]<1e-11
    return {
        "name":name,"type":"ORIGINAL_SOURCE" if phys else "SYNTHETIC_ALGORITHMIC",
        "n":n,"B_size":len(B),"A_size":len(obj.A),
        "path_count":len(obj.chains),"path_lengths":[len(chain) for chain in obj.chains],
        "path_serial_depth":obj.serial_depth,"path_parallel_scan_layers":obj.scan_depth,
        "serial_edges_per_path_apply":obj.serial_dependencies_per_apply,
        "parallel_affine_pairs_per_apply":obj.scan_pairs_per_apply,
        "setup_parallel_scan_pairs":obj.scan_pairs_per_apply*len(B),
        "original_RAW_U_stages":8,
        "mathematically_exact_same_as_R9_serial":True,
        "max_first_step_original_endpoint_error_upper":cert["endpoint_bound_upper"],
        "native_CPU_wall_speed_ratio":"NOT_RUN",
    }


def extra_ring_path_control(r9,n,gamma):
    k,g,c,y,B=r9.synthetic("ring24",n)
    h=0.08
    tau=float(h*gamma)
    serial=r9.create_provider(k,g,c,y,tau,B)
    scan=ScanProvider(r9,k,g,c,y,tau,B)
    rhs=[F(1,i+1) for i in range(n)]
    out=scan.solve(rhs)
    assert out==serial["solver"](rhs)
    assert scan.scan_depth==math.ceil(math.log2(n-1))
    return {
       "name":f"ring{n}","n":n,
       "type":"SYNTHETIC_ONE_LINEAR_STAGE_ONLY",
       "B_size":1,"A_size":n-1,
       "path_count":1,"path_length":n-1,
       "serial_depth":scan.serial_depth,"scan_layers":scan.scan_depth,
       "sequential_edge_operations":scan.serial_dependencies_per_apply,
       "affine_scan_pairs":scan.scan_pairs_per_apply,
       "exact_original_W_residual_zero":True,
       "whole_eight_stage_comparison":"NOT_RUN",
       "native_wall_speedup":"NOT_RUN"
    }


def false_branching_control(r9):
    k,g,c,y,B=r9.synthetic("dag8",8)
    c[2][0]=7.0
    p=r9.create_provider(k,g,c,y,0.002,B)
    try:ScanProvider(r9,k,g,c,y,0.002,B)
    except ValueError as e:
        assert str(e) in ("NOT_PATH_FOREST_BRANCH","NOT_PATH_FOREST_MULTI_PARENT")
        return {"name":"dag8_two_children","category":"REJECTED_PATH_ONLY",
                "reason":str(e),"general_DAG_R9_serial_still_valid":True}
    raise AssertionError("FALSELY_ADMITTED_GENERAL_DAG")


def main():
    r9=load()
    r6=r9.load_original()
    cases=r6.load_cases();a,C,b,gamma=r6.tableau()
    rows=[]
    for name,steps in r6.ORIGINAL_STEPS.items():
        k,g,c,y=cases[name]
        B=[0] if len(y)==2 else [0,1]
        rows.append(source_record(r9,name,k,g,c,y,B,0.1/steps,r6,a,C,b,gamma,True))
    for name,n in (("ring12",12),("two_cycles14",14),("ring24",24),("dag8",8)):
        k,g,c,y,B=r9.synthetic(name,n)
        rows.append(source_record(r9,name,k,g,c,y,B,0.08,r6,a,C,b,gamma,False))
    extras=[extra_ring_path_control(r9,n,gamma) for n in (48,96)]
    rejected=[false_branching_control(r9)]
    assert len(rows)==8
    r9_results=json.loads(R9_DATA.read_text())
    assert r9_results["source_original_physical_independent"]==3
    for row in rows:
        prior=next(x for x in r9_results["rows"] if x["name"]==row["name"])
        assert row["max_first_step_original_endpoint_error_upper"]==prior["exact_rational_first_step_endpoint_bound"]
    out={
      "schema":"vigilode.r10.parallel_affine_scan_source_closure.v1",
      "prereg_head":"dd076af6fd279ff478a6503652199d59d35ed003",
      "previous_R9_exact_source_sha256":R9_SHA,
      "original_source_cases":4,"original_physical_independent":3,
      "synthetic_eight_stage_controls":4,"synthetic_one_stage_holdouts":2,
      "actual_original_raw_U_mathematical_stage_comparisons":64,
      "additional_synthetic_one_stage_exact_checks":2,
      "scan_recurrence":"AFFINE_PAIR_MONOID_HILLIS_STEELE",
      "scan_parallel_layers_are_theoretical_work_span":True,
      "scan_actual_native_parallel_speed":"NOT_RUN",
      "general_DAG_scan_extension":"REFUSED_PATH_FOREST_ONLY",
      "rejected_controls":rejected,
      "verified_rows":rows,"extended_ring_controls":extras,
      "HOL_associative_sublemmas":"SEPARATE_KERNEL_RECEIPT",
      "official_DAG":"7/23","successor_admitted":"0/12",
      "production":"HOLD","native_subagent":"NOT_RUN",
      "no_production_mutation":True
    }
    p=HERE/"RESULTS.json"
    p.write_text(json.dumps(out,indent=2,sort_keys=True,allow_nan=False)+"\n")
    print("R10_STAGE_COMPARISONS",64,"ADDITIONAL_EXACT_HOLDOUTS",len(extras),"REJECTIONS",len(rejected))
    for row in rows+extras:
        print(row["name"],"m",row.get("A_size",row.get("path_length")),
              "serial",row.get("path_serial_depth",row.get("serial_depth")),
              "scan",row.get("path_parallel_scan_layers",row.get("scan_layers")),
              "pairs",row.get("parallel_affine_pairs_per_apply",row.get("affine_scan_pairs")))
    print("RESULT_SHA256",hashlib.sha256(p.read_bytes()).hexdigest())
    return out

if __name__=="__main__":
    main()
