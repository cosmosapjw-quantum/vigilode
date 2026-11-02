#!/usr/bin/env python3
"""Independent stdlib-only physical audit of Rust native F05 time-map RAW output.

No invocation/import of Rust implementation, Python owner integrator or SciPy.
Outputs are point-valued shadows, NOT original interval/full-owner certificates.
"""
from __future__ import annotations
import argparse, hashlib,json, math,struct
from pathlib import Path
N=46080
KB=1.380649e-16
RAW_MAGIC=b"REIF05M1"
PARENT_SHA="eeb4aa50c56ed5437c26b1c7087d0f68884db3db331d4ef435657b0bceb22064"
CLOCK_SHA="0bd888ce706ee110cea4ba23a1e871817cf1f054b85ebe5837934fd19db0492e"
def sha(data):return hashlib.sha256(data).hexdigest()
def particles(p):return sum(p)+p[1]+p[3]+2.0*p[4]
def electron(p):return p[1]+p[3]+2.0*p[4]
def u(p,z):return 1.5*KB*particles(p)*math.exp(z)
def pairs(raw):
    assert raw[:8]==RAW_MAGIC,"NATIVE_TIME_MAP_MAGIC_FAIL"
    assert struct.unpack_from('<Q',raw,8)[0]==N,"NATIVE_TIME_MAP_CELL_COUNT_FAIL"
    assert len(raw)==16+3*N*6*8,"NATIVE_TIME_MAP_RAW_SIZE_FAIL"
    block={}
    off=16
    for name in ("full","half1","half2"):
        rows=[]
        for i in range(N):
            vals=struct.unpack_from('<6d',raw,off)
            off+=48
            if any(not math.isfinite(x) for x in vals) or min(vals[:5])<=0:
                raise AssertionError("NONPOSITIVE_OR_NONFINITE_NATIVE_STATE")
            rows.append(vals)
        block[name]=rows
    assert off==len(raw)
    return block

def check(rawfile,parentfile,clockfile,native_receipt,outfile):
    raw=rawfile.read_bytes()
    parent=parentfile.read_bytes()
    clock=clockfile.read_bytes()
    report=json.loads(native_receipt.read_text())
    assert sha(parent)==PARENT_SHA,"ORIGINAL_PARENT_BYTES_CHANGED"
    assert sha(clock)==CLOCK_SHA,"ORIGINAL_SOURCE_CLOCK_DRIFT"
    assert parent[:8]==b"REIF05P2" and len(parent)==24+12*N*8,"PARENT_LAYOUT_CHANGED"
    assert sha(raw)==report["native_point_states_raw_sha256"],"RAW_SHA_REPORT_MISMATCH"
    assert report["official_tick161"]=="NOT_ACCEPTED"
    x=json.loads(clock)
    assert [m["name"] for m in x["maps"]]==["full","half1","half2"]
    duration=x["duration_seconds"]
    a,b=duration*160/131072,duration*161/131072
    mid=(a+b)/2
    for m,(l,r) in zip(x["maps"],((a,b),(a,mid),(mid,b))):
        assert abs(m["t0_s"]-l)<0.001 and abs(m["t1_s"]-r)<0.001
        assert abs(m["tg_s"]-(l+x["gamma"]*(r-l)))<0.001,"SOURCE_SITE_ORDER_CHANGED"
    data=pairs(raw)
    vals=struct.unpack_from("<"+str(12*N)+"d",parent,24)
    plo,phi=vals[:5*N],vals[5*N:10*N]
    zlo,zhi=vals[10*N:11*N],vals[11*N:12*N]
    max_H=max_He=0.
    delta_Ne={k:0. for k in data}
    delta_U={k:0. for k in data}
    abs_energies={k:0. for k in data}
    abs_energy_deltas={k:0. for k in data}
    err={k:0. for k in ("x_HII_max","x_HeII_max","x_HeIII_max","log_T_max")}
    for i in range(N):
        init=tuple(0.5*(plo[5*i+j]+phi[5*i+j]) for j in range(5))
        z0=0.5*(zlo[i]+zhi[i])
        if min(init)<=0:raise AssertionError("PARENT_POP_NOT_POSITIVE")
        total_h=init[0]+init[1]
        total_he=sum(init[2:5])
        for k in data:
            pop=data[k][i][:5]
            z=data[k][i][5]
            max_H=max(max_H,abs(pop[0]+pop[1]-total_h)/total_h)
            max_He=max(max_He,abs(sum(pop[2:])-total_he)/total_he)
            # half2's input is the actual half1 output, not frozen tick160!
            parent_pop,parent_log=(init,z0) if k!="half2" else (
                data["half1"][i][:5],data["half1"][i][5])
            delta_Ne[k]+=electron(pop)-electron(parent_pop)
            ui=u(parent_pop,parent_log);uf=u(pop,z)
            delta=uf-ui
            delta_U[k]+=delta
            abs_energies[k]+=abs(ui)+abs(uf)
            abs_energy_deltas[k]+=abs(delta)
        pf=data["full"][i]
        ph=data["half2"][i]
        err["x_HII_max"]=max(err["x_HII_max"],abs((pf[1]-ph[1])/total_h))
        err["x_HeII_max"]=max(err["x_HeII_max"],abs((pf[3]-ph[3])/total_he))
        err["x_HeIII_max"]=max(err["x_HeIII_max"],abs((pf[4]-ph[4])/total_he))
        err["log_T_max"]=max(err["log_T_max"],abs(pf[5]-ph[5]))
    assert max_H<2e-12 and max_He<2e-12,"NUCLEUS_MATERIAL_LEDGER_FAIL"
    for k in data:
        def close(x,y):return abs(x-y)<=max(1e-13*max(abs(x),abs(y)),1e-300)
        assert close(delta_Ne[k],report[k]["native_point_delta_N_e"]),(k,"electron",delta_Ne[k])
        # Two independent ordinary f64 loops, with exp/sum reordering.
        # Condition on large U_out-U_in cancellation explicitly, without
        # changing ANY original owner scientific/local-error threshold.
        eps=2.220446049250313e-16
        gamma_n=N*eps/(1-N*eps)
        rounding_bound=gamma_n*abs_energy_deltas[k]+64*eps*abs_energies[k]
        energy_discrepancy=abs(delta_U[k]-report[k]["native_point_delta_U_erg"])
        assert energy_discrepancy<=rounding_bound,(k,"energy_roundoff_exceeds_bound",energy_discrepancy,rounding_bound)
        assert report[k]["point_14_shape_closure_all_checks"],(k,"POINT_CLOSURE_FAIL")
        assert report[k]["table_event_candidate_cells"]==0,(k,"TABLE_KNOT_EVENT")
    for k,y in err.items():
        assert abs(y-report["native_point_doublemap_errors"][k])<1e-14,(k,y,report["native_point_doublemap_errors"][k])
    audit=dict(schema="INDEPENDENT_STDLIB_F05_NATIVE_46080_FULL_HALFMAP_AUDIT_V1",
       state_raw_sha256=sha(raw),clock_sha256=sha(clock),
       source_cells=N,maps=3,nuclei_max_H_rel=max_H,nuclei_max_He_rel=max_He,
       source_clock_site_count=9,map_error=err,
       delta_ne=delta_Ne,delta_u_erg=delta_U,
       energy_ordinary_f64_reduction_roundoff_bound_erg={
           k:(N*2.220446049250313e-16/(1-N*2.220446049250313e-16))*abs_energy_deltas[k]+
                64*2.220446049250313e-16*abs_energies[k] for k in data},
       physical_scalar_checks="PASS",
       owner_interval_14_ledgers="NOT_RUN",
       original_26event_uncertainty_IVP="NOT_ESTABLISHED",
       canonical_delta=0,official_tick161="NOT_ACCEPTED",production="HOLD")
    outfile.write_text(json.dumps(audit,indent=2,sort_keys=True,allow_nan=False)+"\n")
    print(json.dumps({k:audit[k] for k in ("physical_scalar_checks","source_cells","map_error","nuclei_max_H_rel","nuclei_max_He_rel")},sort_keys=True))
    return audit

if __name__=="__main__":
    p=argparse.ArgumentParser()
    for name in ("raw","parent","clock","receipt","out"):p.add_argument("--"+name,required=True,type=Path)
    a=p.parse_args()
    check(a.raw,a.parent,a.clock,a.receipt,a.out)
