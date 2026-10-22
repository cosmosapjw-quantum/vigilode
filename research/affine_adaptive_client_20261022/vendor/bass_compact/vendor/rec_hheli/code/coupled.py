"""Executable full40 + 21 isotope-ion states + optical-depth pilot.

Physical states evolve simultaneously with BDF and an analytic 62x62 Jacobian.
Each isotope is scaled by its own initial inventory, so tiny Li abundances do
not vanish under a hydrogen-sized absolute tolerance. No state is clipped or
renormalized. Outputs retain signed stresses and all original populations.
"""
from __future__ import annotations

import hashlib
import json
import math
import os
from pathlib import Path
import platform
import sys
import time
import traceback

import numpy as np
import scipy
from scipy.integrate import solve_ivp
from numpy.polynomial.legendre import legvander

from geometry import Geometry, ROOT, C, KB, KAPPA_T
from lrs_boltzmann import GAMMA_PER_NE
from atomic_network import AtomicNetwork

SCHEMA = 'rec.rei.hheli.coupled62.v1'
THOMSON = GAMMA_PER_NE/1e6
XEND = math.log(7.)


def canonical(obj):
    return json.dumps(obj,sort_keys=True,separators=(',',':'),allow_nan=False).encode()


def atomic_json(path, value):
    path=Path(path)
    if path.exists():raise FileExistsError(str(path))
    temporary=path.with_name(path.name+f'.{os.getpid()}.tmp')
    with temporary.open('xb') as stream:
        stream.write(json.dumps(value,indent=2,allow_nan=False).encode()+b'\n')
        stream.flush();os.fsync(stream.fileno())
    os.link(temporary,path)
    temporary.unlink()


def thermal_uv(x):
    """Frozen phenomenological inputs, not fitted reionization histories.

    Q(z)=.5[1+tanh((7-z)/.8)], T=100+11900Q K, UV scale=Q.
    dT/dx is analytic; x=ln(a/a_ref). The finite temperature range is 100..12000K.
    """
    z=21*math.exp(-x)-1
    t=math.tanh((7-z)/.8)
    q=.5*(1+t)
    dqdx=.5*(1-t*t)*(1+z)/.8
    return 100+11900*q,q,11900*dqdx


class CoupledSystem:
    size=62

    def __init__(self, config):
        self.config=config
        self.geometry=Geometry(float(config['Sigma_F']),float(config['matter_A']),
                               float(config['radiation_A']),config.get('backend','prepared'))
        self.atomic=AtomicNetwork(uv_cutoff_eV=config.get('uv_cutoff_eV',200.))
        self.scale=self.atomic.scales
        self.opacity=bool(config.get('opacity',True))
        self.started=time.perf_counter()
        self.deadline=float(config.get('_case_wall_seconds',180.))
        self.nrhs=0
        self.njac=0
        self.y0=np.r_[self.geometry.initial,self.atomic.initial_Y/self.scale,0.]

    def point(self,x,y):
        field=self.geometry.fields(x,y[:40])
        temperature,uv,tdx=thermal_uv(x)
        nb=self.geometry.baryon_number_density(x)
        network=self.atomic.evaluate(y[40:61]*self.scale,temperature,uv,nb)
        return field,network,temperature,uv,tdx,nb

    def rhs(self,x,y):
        self.nrhs+=1
        if time.perf_counter()-self.started>self.deadline:
            raise TimeoutError('cooperative case wall limit')
        field,network,*_=self.point(x,y)
        ne=network['ne']
        # Smooth signed Newton trials are allowed by the chemistry owner, but
        # no accepted solution may have negative electron density. Physical
        # residual electrons keep actual trials away from that boundary here.
        ne_collision=ne if self.opacity else 0.
        out=np.empty(62)
        out[:40]=self.geometry.rhs_from_fields(y[:40],field,ne_collision)
        out[40:61]=network['dY_dt']/self.scale/field['H']
        out[61]=THOMSON*ne/field['H']
        return out

    def jac(self,x,y):
        self.njac+=1
        field,network,*rest=self.point(x,y)
        nb=rest[-1]
        h=field['H'];ne=network['ne']
        jgeo,hgrad=self.geometry.jacobian_blocks(x,y[:40],field,ne if self.opacity else 0.)
        jac=np.zeros((62,62))
        jac[:40,:40]=jgeo
        negrad=nb*self.atomic.q*self.scale
        if self.opacity:
            for start in (4,13):
                weighted=self.geometry.model.thomson*y[start:start+9]
                jac[start:start+9,40:61]=-THOMSON/h*np.outer(weighted,negrad)
        chemistry=network['dY_dt']/self.scale/h
        jac[40:61,:40]=-np.outer(chemistry,hgrad)
        jac[40:61,40:61]=(network['jac']*self.scale[None,:]/self.scale[:,None])/h
        jac[61,:40]=-THOMSON*ne/h*hgrad
        jac[61,40:61]=THOMSON/h*negrad
        return jac

    def diagnostics(self,x,y):
        field,network,t,uv,tdx,nb=self.point(x,y)
        Y=y[40:61]*self.scale
        h=field['H'];ne=network['ne']
        ng=nb*float(np.sum(Y))+ne
        pressure=KB*t*ng
        uth=1.5*pressure
        uion=nb*float(self.atomic.ionization_energy_J@Y)
        # Net input required by the prescribed gas path. Neither this nor
        # photo absorption fixes the separate escape and bath terms.
        re=network['electron_source_m3_s']
        ion_source=nb*float(self.atomic.ionization_energy_J@network['dY_dt'])
        q_required=1.5*KB*ng*h*(tdx+2*t)+1.5*KB*t*re+ion_source
        rho_crit_dynamic=3*h*h/KAPPA_T
        norm=self.geometry.normalized(x,y[:40],field)
        # Reconstruct finite-L radiation intensity on a separate 129-angle
        # diagnostic mesh. This is a finite diagnostic, not positivity theorem.
        angular=legvander(np.linspace(-1,1,129),8)
        weights=2*np.arange(9)+1
        minima=[]
        for start in (4,13,22,31):
            moments=y[start:start+9]
            minima.append(float(np.min(angular@(weights*moments))/moments[0]))
        return dict(H=field['H'],b=field['b'],a=field['a'],
            elapsed_s=y[0]/self.geometry.source.href,nb_m3=nb,ne_m3=ne,
            T_K=t,uv_scale=uv,tau_T=y[61],
            P_b_J_m3=pressure,u_thermal_J_m3=uth,u_ionization_J_m3=uion,
            uv_energy_J_m3=network['uv_energy_density_J_m3'],
            trace_energy_fraction=(uth+uion+network['uv_energy_density_J_m3'])/rho_crit_dynamic,
            net_required_gas_input_W_m3=q_required,
            photo_absorbed_W_m3=network['photo_absorbed_power_W_m3'],
            bath_minus_escape_W_m3=q_required-network['photo_absorbed_power_W_m3'],
            primary_photoelectron_W_m3=network['primary_kinetic_power_W_m3'],
            unresolved_core_W_m3=network['unresolved_core_power_W_m3'],
            li1s_events_m3_s=network['li1s_photo_event_rate_m3_s'],
            gamma_T_per_H=THOMSON*ne/h,
            min_photon_angular_weight=minima[0],min_neutrino_angular_weight=minima[2],
            min_photon_number_angular_weight=minima[1],min_neutrino_number_angular_weight=minima[3],
            signed_observables=norm)


def run_case(config,outdir):
    """Run one immutable scientific case; never overwrite existing evidence."""
    outdir=Path(outdir);outdir.mkdir(parents=True,exist_ok=True)
    if (outdir/'RESULT.json').exists() or (outdir/'trajectory.npz').exists():
        raise FileExistsError('case evidence already exists')
    started=time.perf_counter()
    public={k:v for k,v in config.items() if not k.startswith('_')}
    result=dict(schema=SCHEMA,status='FAILED',config=public,
        scientific_identity_sha256=config.get('_identity','DIRECT_DEVELOPMENT_CALL'),
        outputs={},gates={},environment=dict(python=sys.version.split()[0],
        numpy=np.__version__,scipy=scipy.__version__,platform=platform.platform()))
    try:
        system=CoupledSystem(config)
        count=int(config.get('samples',193))
        if count<3:raise ValueError('at least 3 samples required')
        xend=float(config.get('x_end',XEND))
        if not 0<xend<=XEND:raise ValueError('invalid finite integration interval')
        grid=np.linspace(0,xend,count)
        atol=np.r_[np.full(40,float(config.get('geometry_atol',2e-11))),
                   np.full(21,float(config.get('atomic_atol',2e-10))),2e-11]
        sol=solve_ivp(system.rhs,(0,xend),system.y0,method='BDF',jac=system.jac,
            t_eval=grid,rtol=float(config.get('rtol',2e-8)),atol=atol,
            max_step=.025)
        if not sol.success or sol.y.shape!=(62,count):
            raise RuntimeError('integration failed: '+sol.message)
        Y=sol.y[40:61].T*system.scale
        histories=[system.diagnostics(float(x),sol.y[:,i]) for i,x in enumerate(grid)]
        observables=np.array([row.pop('signed_observables') for row in histories])
        history_keys=list(histories[0])
        diagnostics=np.array([[row[k] for k in history_keys] for row in histories])
        index={k:i for i,k in enumerate(history_keys)}
        isotope_errors={}
        for name,indices in system.atomic.isotope_groups.items():
            # n and T have zero inventory here; the exact decay maps are tested
            # separately. H1/He3 constant here follows zero decay flux.
            total=Y[:,indices].sum(axis=1)
            nominal=system.atomic.initial_Y[list(indices)].sum()
            scale=system.scale[indices[0]]
            isotope_errors[name]=float(np.max(abs(total-nominal))/scale)
        baryon_error=float(np.max(abs(Y@system.atomic.A-1)))
        min_scaled=float(np.min(sol.y[40:61]))
        target=np.array([config['Sigma_F'],config.get('P_m_initial_target',observables[0,1]),
                         config.get('P_r_initial_target',observables[0,2])])
        initial_axis_error=float(np.max(abs(observables[0,:3]-target)))
        number_error=float(max(np.max(abs(sol.y[13]-1)),np.max(abs(sol.y[31]-1))))
        constraint=[]
        for i,x in enumerate(grid):
            f=system.geometry.fields(x,sol.y[:40,i])
            lhs=3*f['H']**2
            rhs=3*system.geometry.source.h0**2*(f['dust_energy_norm']+
                f['massive_energy_norm']+f['rad_energy_norm']+system.geometry.source.omega_lambda)+3*f['s']**2
            constraint.append(abs(lhs-rhs)/lhs)
        gates=dict(full_interval_and_samples=bool(sol.t[-1]==xend and len(sol.t)==count),
            initial_physical_axes=initial_axis_error<2e-12,
            finite=bool(np.isfinite(sol.y).all() and np.isfinite(diagnostics).all()),
            population_positivity=min_scaled>=-2e-8,
            isotope_scaled_conservation=max(isotope_errors.values())<2e-7,
            baryon_conservation=baryon_error<2e-8,
            nonnegative_electrons=bool(np.min(diagnostics[:,index['ne_m3']])>=0),
            number_monopoles=number_error<2e-9,
            monotone_optical_depth=bool(np.min(np.diff(sol.y[61]))>=-2e-10),
            einstein_constraint=max(constraint)<2e-13,
            tensor_sum=bool(np.max(abs(observables[:,3]-observables[:,1]-observables[:,2]))<2e-16),
            finite_angular_positivity=bool(min(np.min(diagnostics[:,index['min_photon_angular_weight']]),
                np.min(diagnostics[:,index['min_neutrino_angular_weight']]),
                np.min(diagnostics[:,index['min_photon_number_angular_weight']]),
                np.min(diagnostics[:,index['min_neutrino_number_angular_weight']]))>-2e-7),
            trace_energy_small=bool(np.max(diagnostics[:,index['trace_energy_fraction']])<1e-5))
        temporary=outdir/'trajectory.tmp.npz'
        np.savez_compressed(temporary,x=grid,z=21*np.exp(-grid)-1,
            raw_state=sol.y.T,Y=Y,scaled_populations=sol.y[40:61].T,
            signed_observables=observables,observables_norm=np.abs(observables),
            diagnostics=diagnostics,diagnostic_names=np.array(history_keys),
            species_names=np.array(system.atomic.names),A=system.atomic.A,q=system.atomic.q,
            isotope_scales=system.scale,initial_Y=system.atomic.initial_Y)
        os.link(temporary,outdir/'trajectory.npz');temporary.unlink()
        payload=(outdir/'trajectory.npz').read_bytes()
        result.update(status='COMPLETE' if all(gates.values()) else 'FAILED',gates=gates,
            outputs={'trajectory.npz':dict(bytes=len(payload),sha256=hashlib.sha256(payload).hexdigest())},
            nfev=int(sol.nfev),njev=int(sol.njev),nlu=int(sol.nlu),rhs_calls=system.nrhs,
            jac_calls=system.njac,samples=count,x_end=xend,z_end=float(21*math.exp(-xend)-1),
            initial_signed_observables=observables[0].tolist(),final_signed_observables=observables[-1].tolist(),
            min_scaled_population=min_scaled,baryon_error=baryon_error,
            initial_axis_error=initial_axis_error,
            isotope_scaled_errors=isotope_errors,number_monopole_error=number_error,
            max_einstein_constraint=max(constraint),
            max_trace_energy_fraction=float(np.max(diagnostics[:,index['trace_energy_fraction']])),
            final_tau_T=float(sol.y[61,-1]),solver_message=sol.message,
            physical_scope='conditional prescribed T/UV, test-fluid baryons, single-photoelectron yield; Physical HOLD')
    except Exception as error:
        result.update(error=str(error),traceback=traceback.format_exc())
    result['wall_seconds']=time.perf_counter()-started
    atomic_json(outdir/'RESULT.json',result)
    return result
