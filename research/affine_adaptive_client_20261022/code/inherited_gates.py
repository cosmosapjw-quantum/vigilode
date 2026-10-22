"""Inherited compact_run gate expressions, unchanged; source manifest pins owner."""
import numpy as np
from types import SimpleNamespace
from compact_system import lift

def evaluate(system,grid,v33,config):
    count=len(grid);xend=grid[-1]
    sol=SimpleNamespace(y=lift(v33),t=grid)
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

    return dict(gates=gates,initial_signed_observables=observables[0].tolist(),final_signed_observables=observables[-1].tolist(),final_tau_T=float(sol.y[61,-1]),min_scaled_population=min_scaled,baryon_error=baryon_error,isotope_scaled_errors=isotope_errors,number_monopole_error=number_error,max_einstein_constraint=max(constraint),initial_axis_error=initial_axis_error,final21_scaled_populations=sol.y[40:61,-1].tolist(),species_names=list(system.atomic.names))
