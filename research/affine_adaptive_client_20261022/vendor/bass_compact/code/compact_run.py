"""Compact BDF adapter; donor acceptance expressions retained verbatim."""
from __future__ import annotations
from compact_system import CompactSystem, lift, restrict, SELECT
from coupled import (atomic_json as _owner_atomic_json, THOMSON, XEND, np, scipy, solve_ivp, Path,
                     time, sys, platform, math, os, hashlib, traceback)
SCHEMA = 'rec.rei.hheli.compact33.v1'


def native_scalar_metadata(value):
    """Convert only NumPy scalar metadata; preserve arrays and gate values."""
    if isinstance(value, np.generic):
        return value.item()
    if isinstance(value, dict):
        return {key:native_scalar_metadata(item) for key,item in value.items()}
    if isinstance(value, list):
        return [native_scalar_metadata(item) for item in value]
    if isinstance(value, tuple):
        return tuple(native_scalar_metadata(item) for item in value)
    return value


def atomic_json(path, value):
    return _owner_atomic_json(path,native_scalar_metadata(value))

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
        system=CompactSystem(config)
        count=int(config.get('samples',193))
        if count<3:raise ValueError('at least 3 samples required')
        xend=float(config.get('x_end',XEND))
        if not 0<xend<=XEND:raise ValueError('invalid finite integration interval')
        grid=np.linspace(0,xend,count)
        atol=np.r_[np.full(40,float(config.get('geometry_atol',2e-11))),
                   np.full(21,float(config.get('atomic_atol',2e-10))),2e-11]
        sol=solve_ivp(system.rhs,(0,xend),system.y0,method='BDF',jac=system.jac,
            t_eval=grid,rtol=float(config.get('rtol',2e-8)),atol=restrict(atol),
            max_step=.025)
        if not sol.success or sol.y.shape!=(33,count):
            raise RuntimeError('integration failed: '+sol.message)
        compact_raw_state33=sol.y.T.copy()
        sol.y=lift(sol.y)
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
            raw_state=sol.y.T,raw_state62=sol.y.T,
            compact_raw_state33=compact_raw_state33,
            original_initial_state62=system.original_initial62,
            signed_observables_full_frobenius=observables,
            signed_observables_half_contraction=observables/math.sqrt(2.),
            signed_observable_names=np.array(["S_F","P_m_F","P_r_F","P_total_F","P_gamma_F","P_nu_F"]),
            half_contraction_observable_names=np.array(["S_half","P_m_half","P_r_half","P_total_half","P_gamma_half","P_nu_half"]),
            Y=Y,scaled_populations=sol.y[40:61].T,
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
            initial_compact_eligibility=system.eligibility,
            final_tau_T=float(sol.y[61,-1]),solver_message=sol.message,
            physical_scope='conditional prescribed T/UV, test-fluid baryons, single-photoelectron yield; Physical HOLD')
    except Exception as error:
        result.update(error=str(error),traceback=traceback.format_exc())
    result['wall_seconds']=time.perf_counter()-started
    atomic_json(outdir/'RESULT.json',result)
    return result
