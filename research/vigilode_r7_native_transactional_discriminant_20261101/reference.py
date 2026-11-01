#!/usr/bin/env python3
"""Independent SciPy Radau + DOP853 reference, frozen Rust WP3 original source.

This is a numerical consensus reference, not outward interval certified.
"""
from pathlib import Path
import hashlib
import importlib.util
import json
import math
import struct
import sys

import numpy as np
from scipy.integrate import solve_ivp

ROOT=Path(__file__).resolve().parents[2]
HERE=Path(__file__).resolve().parent
WP3=ROOT/"crates/rodas5p-integrators/examples/vigilode_wp3_cubic_rodas5p.rs"
R5=ROOT/"docs/vigilode_r5_newtonfree_algebraic_homotopy_20261101/r5_experiment.py"
PRE_REGISTERED_ORIGINAL_WP3_GIT_BLOB="5dc8df34d314c0d8562c6f548a574625cef442ff"
METHODS=("Radau","DOP853")
RTOL=2e-13
ATOL=2e-15
REFERENCE_CROSS_MAX=2e-10
STEPS={"asymmetric_2":64,"asymmetric_2_refined":128,
       "coupled_3_cubic":128,"linear_exchange_control":128}


def ieee_bits(x: float)->int:
    return int.from_bytes(struct.pack(">d",float(x)),byteorder="big")


def original_cases():
    spec=importlib.util.spec_from_file_location("locked_r5_reference",R5)
    m=importlib.util.module_from_spec(spec)
    sys.modules[spec.name]=m
    spec.loader.exec_module(m)
    m.locked_cases()
    a=m.locked_cases()
    return {key:m.params(value) for key,value in a.items()}


def function(original):
    k,g,c,_=original
    n=len(k)
    def f(t,y):
        out=[0.0]*n
        for i in range(n):
            val=-k[i]*y[i]-g[i]*y[i]*y[i]*y[i]
            for j in range(n):
                if i!=j:
                    val+=c[i][j]*(y[j]-y[i])
            out[i]=val
        return out
    return f


def main():
    import subprocess
    blob=subprocess.check_output(["git","hash-object",str(WP3)],text=True).strip()
    assert blob==PRE_REGISTERED_ORIGINAL_WP3_GIT_BLOB
    source=original_cases()
    assert source["asymmetric_2"]==source["asymmetric_2_refined"]
    rows=[]
    for name,steps in STEPS.items():
        k,g,c,y0=source[name]
        h=float(0.1/steps)
        grid=np.array([h*i for i in range(steps+1)],dtype=float)
        assert grid[-1]==0.1 and grid[0]==0.0
        values={}
        for method in METHODS:
            sol=solve_ivp(function(source[name]),(0.0,0.1),y0,
                          method=method,rtol=RTOL,atol=ATOL,
                          t_eval=grid)
            assert sol.success and sol.y.shape==(len(k),len(grid)),(name,method,sol.message)
            arr=sol.y.T.tolist()
            values[method]={
                "output":arr,"nfev":int(sol.nfev),
                "njev":int(sol.njev),"nlu":int(sol.nlu),
                "success":bool(sol.success)
            }
        consensus=float(np.max(np.abs(
            np.array(values["Radau"]["output"])-
            np.array(values["DOP853"]["output"]))))
        row={
            "case":name,"model_family":("asymmetric_2" if name=="asymmetric_2_refined" else name),
            "source_equivalent_resolution_control":bool(name=="asymmetric_2_refined"),
            "source_state":list(map(float,y0)),
            "original_step_h_bits":ieee_bits(h),
            "original_steps":steps,"grid_time_bits":[ieee_bits(x) for x in grid],
            "grid_times":grid.tolist(),
            "reference_methods":list(METHODS),
            "reference_rtol":RTOL,"reference_atol":ATOL,
            "cross_solver_state_linf":consensus,
            "reference_consensus_pass":bool(consensus<=REFERENCE_CROSS_MAX),
            "radau":values["Radau"],"dop853":values["DOP853"]
        }
        rows.append(row)
        print("REFERENCE",name,"GRID",len(grid),"CROSS",consensus,"PASS",row["reference_consensus_pass"],
              "NFEV_RADAU",values["Radau"]["nfev"],"NFEV_DOP853",values["DOP853"]["nfev"])
    result={
        "schema":"vigilode.r7.original_wp3.radau_dop853_consensus.v1",
        "frozen_source_git_blob":blob,
        "source_families_distinct":3,
        "all_reference_consensus_pass":all(r["reference_consensus_pass"] for r in rows),
        "consensus_absolute_limit":REFERENCE_CROSS_MAX,
        "formally_interval_certified":False,
        "numerically_checked":True,
        "official_dag":"7/23","production":"HOLD",
        "rows":rows,
    }
    path=HERE/"REFERENCE.json"
    path.write_text(json.dumps(result,indent=2,sort_keys=True,allow_nan=False)+"\n")
    print("REFERENCE_RESULT_SHA256",hashlib.sha256(path.read_bytes()).hexdigest())
    if not result["all_reference_consensus_pass"]:
        raise RuntimeError("NUMERIC_REFERENCE_NOT_AGREED")


if __name__=="__main__":
    main()
