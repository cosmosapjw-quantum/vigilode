#!/usr/bin/env python3
"""Read-only source-owner F05 clock/forcing exporter; NEVER a Python IVP solver."""
from __future__ import annotations
import argparse, hashlib, importlib.util, json, sys, subprocess
from pathlib import Path
import numpy as np

OWNER_HEAD="fcc90939b6b7cc842fae65d5a0ffbb69a3ec3120"
SOURCE_SHA="a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b"
STATE_SHA="cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01"
KERNEL_SHA="579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644"
N=46080
GAMMA=1.0-1.0/np.sqrt(2.0)

def load(path,name):
    spec=importlib.util.spec_from_file_location(name,str(path))
    mod=importlib.util.module_from_spec(spec)
    sys.modules[name]=mod
    spec.loader.exec_module(mod)
    return mod

def mid(value):
    a=np.asarray(value.lo,dtype=np.float64)
    b=np.asarray(value.hi,dtype=np.float64)
    if not np.isfinite(a).all() or not np.isfinite(b).all() or np.any(a>b):
        raise ArithmeticError("SOURCE_FORCING_INTERVAL_INVALID")
    x=0.5*a+0.5*b
    if x.size!=1:raise ValueError("NON_SCALAR_CLOCK_FORCING")
    return float(x.item())

def run(repo:Path,out:Path):
    if subprocess.check_output(["git","-C",str(repo),"rev-parse","HEAD"],text=True).strip()!=OWNER_HEAD:
        raise ArithmeticError("ORIGINAL_REI_HEAD_DRIFT")
    edge=load(repo/"research/rei_igm_original26_temperature_edges_20261101/edge_probe.py","rei_f05_clock_edge")
    stage=repo/edge.REL_STAGE
    worker=load(stage/"analysis/attempt_worker.py","rei_f05_clock_worker")
    worker.jax_guard.install_if_missing()
    original=worker._kernel()
    if hashlib.sha256(worker.KERNEL_PATH.read_bytes()).hexdigest()!=KERNEL_SHA:
        raise ArithmeticError("ORIGINAL_KERNEL_DRIFT")
    decoded,source=edge.source_state(stage,"LOCAL_NEUTRAL_HAZARD_PRIMARY",worker.state_io)
    if source["archive_sha256"]!=SOURCE_SHA or source["accepted_array_sha256"]!=STATE_SHA:
        raise ArithmeticError("ORIGINAL_T160_STATE_OR_TAR_DRIFT")
    base,_,_,nh,nhe=worker._context(original,"LOCAL_NEUTRAL_HAZARD_PRIMARY")
    if len(nh)!=N:raise ValueError("NOT_46080_CELLS")
    model=original.rim.ReducedIntervalModel.from_repo(repo)
    duration=float(base.forcing.duration_seconds(0))
    if not np.isfinite(duration) or duration<=0:raise ValueError("INVALID_ORIGINAL_DURATION")
    tick0=duration*160/131072
    tick1=duration*161/131072
    half=(tick0+tick1)/2
    layout=[]
    for name,a,b in (("full",tick0,tick1),("half1",tick0,half),("half2",half,tick1)):
        tg=float(a+GAMMA*(b-a))
        sites=[]
        for label,time in (("t0",a),("tg",tg),("t1",b)):
            src=model.forcing_bounds(time,time)
            z=mid(src.z)
            J=[mid(q) for q in src.current]
            E=[mid(q) for q in src.external_subgrid]
            scale=float(np.longdouble(original.rim.NH0_CM3)*
                        (np.longdouble(1.0)+np.longdouble(z))**2*
                        np.longdouble(original.rim.MPC_CM))
            a_coeff=[]
            for g in range(4):
                a_coeff.append([])
                for s,(den,yhe) in enumerate(((model.n_h_total,1.0),(model.n_he_total,original.rim.YHE),(model.n_h_total,original.rim.YHE))):
                    sig=model.inputs.sigma_cm2[s,g] if model.inputs.owner_support[s+1,g] else 0.0
                    a_coeff[-1].append(float(np.longdouble(yhe)*np.longdouble(scale)*np.longdouble(sig)/np.longdouble(den)))
            # Original comoving cells have distinct proper volumes.
            # Derive their exact common expansion factor relative to t160.
            vlo=np.asarray(src.volume_cm3.lo,dtype=np.float64)
            vhi=np.asarray(src.volume_cm3.hi,dtype=np.float64)
            if vlo.shape!=(N,) or vhi.shape!=(N,) or np.any(vlo<=0) or np.any(vlo>vhi):
                raise ArithmeticError("ORIGINAL_VOLUME_CELL_SHAPE_OR_CONE_DRIFT")
            if name=="full" and label=="t0":
                base_z=z
                original_base_volume=0.5*vlo+0.5*vhi
            factor=((1.0+base_z)/(1.0+z))**3
            if not np.allclose(0.5*vlo+0.5*vhi,original_base_volume*factor,rtol=2e-14,atol=0):
                raise ArithmeticError("SOURCE_VOLUME_REDSHIFT_MISMATCH")
            hubble=mid(src.hubble_s_inv)
            if not (factor>0 and hubble>=0 and all(x>=0 for x in J+E) and all(x>=0 for row in a_coeff for x in row)):
                raise ArithmeticError("PHYSICAL_SOURCE_CLOCK_DOMAIN")
            sites.append(dict(label=label,time_s=float(time),current_J=J,external_E=E,alpha=a_coeff,z=z,volume_scale_from_t160=factor,hubble_s_inv=hubble))
        layout.append(dict(name=name,t0_s=float(a),tg_s=tg,t1_s=float(b),sites=sites))
    # The original source handles *point* forcing here; all original
    # interval uncertainty must be retained by original owner certification.
    data=dict(schema="REI_F05_ORIGINAL_NATIVE_RUST_CLOCK_SOURCE_ONLY_V1",
        original_owner_head=OWNER_HEAD,original_source_tar_sha256=SOURCE_SHA,
        original_state_sha256=STATE_SHA,original_kernel_sha256=KERNEL_SHA,
        accepted_tick="160/131072",source_cells=N,photon_groups=4,
        duration_seconds=duration,source_tick_width_seconds=tick1-tick0,
        gamma=float(GAMMA),excess_eV=np.asarray(model.excess_eV,float).tolist(),
        eV_to_erg=float(original.rim.EV_ERG),maps=layout,
        source_owning_rust_time_map="NOT_YET_CERTIFIED",
        official_tick161="NOT_ACCEPTED",production="HOLD")
    out.parent.mkdir(parents=True,exist_ok=True)
    out.write_text(json.dumps(data,sort_keys=True,indent=2,allow_nan=False)+"\n")
    print(json.dumps(dict(exported_maps=len(layout),sites=sum(len(x["sites"]) for x in layout),
           clocks=[[x["t0_s"],x["tg_s"],x["t1_s"]] for x in layout],
           source_sha256=hashlib.sha256(out.read_bytes()).hexdigest(),
           source_cells=N,official_tick161="NOT_ACCEPTED"),sort_keys=True))

if __name__=="__main__":
    p=argparse.ArgumentParser()
    p.add_argument("--owner",type=Path,required=True)
    p.add_argument("--out",type=Path,required=True)
    v=p.parse_args()
    run(v.owner.resolve(),v.out.resolve())
