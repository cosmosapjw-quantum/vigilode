"""Source-locked BASS21 original 125-grid saved endpoint F/J and tau atlas.

No IVP, no solver or cosmological transfer. Optical-depth tau is forward
integrated and stored by the ORIGINAL BASS pilot. All generated quantities are
conditional on prescribed T/UV and original 21-species/source closure.
"""
from __future__ import annotations
import argparse,csv,hashlib,importlib,io,json,math,sys,zipfile
from pathlib import Path
from fractions import Fraction
import numpy as np

ROOT=Path(__file__).resolve().parents[1]
SOURCE=ROOT/'source'
ORIGINAL_CSV=SOURCE/'ORIGINAL_BASS_127_ENDPOINTS.csv'
ORIGINAL_ZIP=SOURCE/'PR122_FIVE_PAIRS_SOURCE_FULL.zip'
ORIGINAL_BLOB='a49aa5276b4256d6bf0ad443b6b830effe2ab738'
ORIGINAL_SHA256='f8020fe09d4ac34573de0e452f46a324f7dca2097b857131eadfee5ed8d6c866'
PARENT_SHA256='93a305a9599cd7a87d5f574fc8105ea9096af24fc5763287ff1d4fbc190f5561'
OWNER_COMMIT='cc602e209e7deeb511ab2a1f3b2268d55206c43c'
SHEAR=np.array([0,.003,.03,.15,.6],dtype=float)
SPECIES=('n','H1I','H1II','DI','DII','TI','TII','He3I','He3II','He3III','He4I','He4II','He4III','Li6I','Li6II','Li6III','Li6IV','Li7I','Li7II','Li7III','Li7IV')
K=(2.99792458e10*6.6524587051e-25)/1e6 # original BASS c sigma_T, m^3 / s

def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def git_blob(raw):return hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\x00'+raw).hexdigest()

def locked_inputs():
    raw=ORIGINAL_CSV.read_bytes()
    if hashlib.sha256(raw).hexdigest()!=ORIGINAL_SHA256 or git_blob(raw)!=ORIGINAL_BLOB:
        raise ValueError('BASS_OWNER_ENDPOINT_SOURCE_IDENTITY_FAILURE')
    if digest(ORIGINAL_ZIP)!=PARENT_SHA256:
        raise ValueError('VIGILODE_PR122_PARENT_BYTES_FAILURE')
    with ORIGINAL_ZIP.open('rb') as f,zipfile.ZipFile(f) as z:
        assert z.testzip() is None
    return True

def get_network():
    locked_inputs()
    dest=ROOT/'_source_runtime'/'rei_pr269'
    # Sources only; do not materialize/modify original source package.
    parent_zip_bytes=zipfile.ZipFile(ORIGINAL_ZIP).read('source/BASS21_PR121_PARENT_FULL.zip')
    with zipfile.ZipFile(io.BytesIO(parent_zip_bytes)) as parent:
        for fn in parent.namelist():
            if fn.startswith('rei_pr269/code/') and fn.endswith('.py') or fn.startswith('rei_pr269/source/') and fn.endswith(('.json','.py')):
                out=ROOT/'_source_runtime'/fn
                out.parent.mkdir(parents=True,exist_ok=True)
                out.write_bytes(parent.read(fn))
    sys.path.insert(0,str(dest/'code'))
    import native_full_rate as model
    model.source_manifest_check()
    return model.new_native_network(),model

def read_grid():
    locked_inputs()
    with ORIGINAL_CSV.open(newline='') as f:rows=list(csv.DictReader(f))
    if len(rows)!=127 or len({r['case_id'] for r in rows})!=127:
        raise ValueError('EXPECTED_127_UNIQUE_ENDPOINTS')
    grid=[]
    for r in rows:
        cid=r['case_id']
        if cid.startswith('cancel_'):continue
        blocks=cid.split('_')
        if len(blocks)!=3 or blocks[0][0]!='s' or blocks[1][0]!='m' or blocks[2][0]!='r':raise ValueError('CASE_ID_LAYOUT')
        a,b,c=(int(s[1:]) for s in blocks)
        if any(i not in range(5) for i in (a,b,c)):raise ValueError('BAD_125_GRID_INDEX')
        grid.append((a,b,c,r))
    grid.sort(key=lambda x:(x[0],x[1],x[2]))
    if len(grid)!=125 or len({(s,m,r) for s,m,r,_ in grid})!=125:raise ValueError('INCOMPLETE_FIVE_CUBED_GRID')
    return grid

def compute():
    grid=read_grid(); net,_=get_network()
    raw=[];F=[];J=[];electron=[];y_arrays=[];gen_by_key={}
    max_ne=0.;max_sum=0.;max_massF=0.;max_charge=0.
    for s,m,r,row in grid:
        Y=np.array([float(row['Y_'+n]) for n in SPECIES],dtype=float)
        if not np.all(np.isfinite(Y)) or (Y<0).any():raise ValueError('NONPHYSICAL_SAVED_STATE')
        nb=float(row['nb_m3']);T=float(row['T_K']);uv=float(row['uv_scale']);n_e=float(row['ne_m3'])
        gamma=float(row['gamma_T_per_H']);tau=float(row['tau_T'])
        values=net.evaluate(np.ascontiguousarray(Y),T,uv,nb)
        f=np.asarray(values['dY_dt']);j=np.asarray(values['jac']);e=float(values['ne']);
        ph=nb*float(net.q@values['photo_rhs']);rr=nb*float(net.q@values['rr_rhs']);dr=nb*float(net.q@values['dr_rhs']);weak=nb*float(net.q@values['decay_rhs'])
        F.append(f);J.append(j);y_arrays.append(Y);electron.append(e)
        x_e=float(net.q@Y)
        rec={'case_id':row['case_id'],'shear_initial':float(SHEAR[s]),'s':s,'m':m,'r':r,'x':float(row['x']),'z':float(row['z']),
         'tau_T_saved':tau,'gamma_T_per_H_saved':gamma,'n_e_m3_saved':n_e,'n_e_m3_native':e,'n_b_m3':nb,'x_e_native':x_e,
         'mean_H_reconstructed_s_inv':float(K*e/gamma),'proper_T_K':T,'original_uv_scale':uv,
         'electron_source_native_m3_s':float(nb*net.q@f),'electron_photo_m3_s':ph,'electron_RR_m3_s':rr,'electron_DR_m3_s':dr,
         'electron_decay_m3_s':weak,'photo_absorbed_power_W_m3':float(values['photo_absorbed_power_W_m3']),
         'primary_kinetic_power_W_m3':float(values['primary_kinetic_power_W_m3']),
         'unresolved_core_power_W_m3':float(values['unresolved_core_power_W_m3']),
         'LiI_1s_event_rate_m3_s':float(values['li1s_photo_event_rate_m3_s']),
         'mass_sum':float(net.A@Y),'A_dot_F_s_inv':float(net.A@f),'charge_xe_ne_discrepancy':float(e/nb-x_e),
         'source_ne_saved_residual':e-n_e,'source_charge_balance_residual':float(nb*(net.q@f)-ph-rr-dr-weak),
         'trace_energy_fraction':float(row['trace_energy_fraction'])}
        max_ne=max(max_ne,abs(rec['source_ne_saved_residual']))
        max_sum=max(max_sum,abs(rec['mass_sum']-1));max_massF=max(max_massF,abs(rec['A_dot_F_s_inv']));max_charge=max(max_charge,abs(rec['source_charge_balance_residual']))
        raw.append(rec);gen_by_key[(s,m,r)]=rec
    pairs=[];curvs=[]
    for m in range(5):
      for r in range(5):
        a=gen_by_key[0,m,r];b=gen_by_key[4,m,r]
        na,nb=a['n_e_m3_saved'],b['n_e_m3_saved']
        ga,gb=a['gamma_T_per_H_saved'],b['gamma_T_per_H_saved']
        Ra,Rb=ga/na,gb/nb
        # Means times differences, exact identity (no perturbation approximation).
        ion=(Rb+Ra)/2*(nb-na)
        geom=(nb+na)/2*(Rb-Ra)
        density=(a['n_b_m3']-b['n_b_m3'])
        direct=gb-ga
        dy=b['tau_T_saved']-a['tau_T_saved']
        denom=a['tau_T_saved']
        initial_tau=np.array([gen_by_key[s,m,r]['tau_T_saved'] for s in range(5)])
        slopes=np.diff(initial_tau)/np.diff(SHEAR)
        divided_curvature=np.diff(slopes)/((SHEAR[2:]-SHEAR[:-2])/2)
        exact_a=Fraction(str(ga))/Fraction(str(na));exact_b=Fraction(str(gb))/Fraction(str(nb));X=Fraction(str(na));Y=Fraction(str(nb))
        exact_res=(Y*exact_b-X*exact_a)-((Y-X)*(exact_a+exact_b)/2+(Y+X)*(exact_b-exact_a)/2)
        if exact_res!=0:raise ValueError('EXACT_BINARY_INPUT_RATIONAL_IDENTITY_FAILED')
        pairs.append({'pair':f'm{m:02d}_r{r:02d}','m':m,'r':r,'tau_A':a['tau_T_saved'],'tau_B':b['tau_T_saved'],
          'stored_delta_tau':dy,'relative_delta_tau':dy/denom,'rate_delta':direct,'endpoint_rate_geometry':geom,
          'endpoint_rate_electron':ion,'endpoint_rate_baryon_density':0.,'endpoint_rate_geometry_fraction':geom/direct,
          'endpoint_ne_delta_m3':nb-na,'endpoint_xe_delta':b['x_e_native']-a['x_e_native'],
          'initial_0_to_0p003_delta_tau':initial_tau[1]-initial_tau[0],
          'positive_small_shear_step':bool(slopes[0]>0),
          'all_secant_slopes_decreasing':bool(np.all(np.diff(slopes)<0)),
          'max_abs_endpoint_rate_split_residual':abs(direct-ion-geom),
          'source_comp':{'A':a['case_id'],'B':b['case_id']}})
        curvs.append({'pair':f'm{m:02d}_r{r:02d}', 'tau_at_shear':initial_tau.tolist(),
          'secant_slopes':[float(v) for v in slopes],
          'divided_second_derivative':[float(v) for v in divided_curvature],
          'has_positive_0_to_0p003_slope':bool(slopes[0]>0)})
    # Nonseparability of the 25-pair shear-response grid (not a physical independence theorem).
    matrix=np.array([[next(p['stored_delta_tau'] for p in pairs if p['m']==m and p['r']==r) for r in range(5)] for m in range(5)])
    cross=matrix-matrix[:,0,None]-matrix[None,0,:]+matrix[0,0]
    photo=np.array([r['electron_photo_m3_s'] for r in raw]);rr=np.array([r['electron_RR_m3_s'] for r in raw]);e_src=np.array([r['electron_source_native_m3_s'] for r in raw])
    summary={'schema':'vigilode.bass21.125_endpoint_source.v1','date_kst':'2026-10-30',
     'identity':{'original_csv_git_blob':ORIGINAL_BLOB,'original_csv_sha256':ORIGINAL_SHA256,'PR122_parent_zip_sha256':PARENT_SHA256,'source_owner_commit':OWNER_COMMIT},
     'counts':{'original_endpoints':127,'five_cubed_positive_grid':125,'matched_shear_pairs':25,'native_FJ_evaluations':125,'new_IVPs':0,'preserved_full_history_pairs_from_PR122':5},
     'physical':{'shear_initial_values':SHEAR.tolist(),'z_end':float(raw[0]['z']),'x_end':float(raw[0]['x']),
       'mean_stored_delta_tau':float(np.mean(matrix)),'min_stored_delta_tau':float(np.min(matrix)),'max_stored_delta_tau':float(np.max(matrix)),
       'mean_relative_stored_delta_tau':float(np.mean([p['relative_delta_tau'] for p in pairs])),
       'negative_all_25':bool(np.all(matrix<0)),
       'decreasing_secant_slopes_25_of_25':int(sum(t['all_secant_slopes_decreasing'] for t in pairs)),
       'small_positive_initial_shear_step_7_of_25':int(sum(t['positive_small_shear_step'] for t in pairs)),
       'max_positive_initial_shear_step_delta_tau':float(max(p['initial_0_to_0p003_delta_tau'] for p in pairs)),
       'endpoint_rate_geometry_fraction_min':float(min(p['endpoint_rate_geometry_fraction'] for p in pairs)),
       'endpoint_rate_geometry_fraction_max':float(max(p['endpoint_rate_geometry_fraction'] for p in pairs)),
       'endpoint_ne_delta_range_m3':[float(min(p['endpoint_ne_delta_m3'] for p in pairs)),float(max(p['endpoint_ne_delta_m3'] for p in pairs))],
       'max_abs_m_r_mixed_interaction_tau':float(np.max(np.abs(cross))),
       'mean_native_photo_electron_m3_s':float(np.mean(photo)),
       'mean_native_RR_electron_m3_s':float(np.mean(rr)),
       'mean_net_electron_source_m3_s':float(np.mean(e_src))},
     'validation':{'original_csv_and_source_zip_sha256':'MATCH','native_ne_max_abs_vs_saved':max_ne,'baryon_mass_max_residual':max_sum,
         'max_A_F_s_inv':max_massF,'max_electron_source_balance_m3_s':max_charge,
         'max_25_rate_product_identity_error':float(max(p['max_abs_endpoint_rate_split_residual'] for p in pairs)),
         'exact_Fraction_25_of_25':'PASS'},
     'claims':{'125_original_source_FJ':'NUMERICALLY_CHECKED','finite_25_endpoint_rate_split':'DERIVED_NUMERICALLY_CHECKED',
       'five_point_secant_curvature':'NUMERICALLY_OBSERVED_NOT_SOLVER_ERROR_CERTIFIED',
       'near_zero_shear_7_sign_reversals':'STORED_DATA_ONLY_NOT_PHYSICALLY_ADMITTED',
       '25_full_time_history_decomposition':'NOT_RUN_ONLY_5_PRIOR','causal_independent_H_electron':'NOT_ESTABLISHED',
       'cosmological_CMB_inference':'NOT_RUN','independent_R6_C9':'NOT_RUN','production':'HOLD'},
     'DAG':{'original_VigilODE':'7/23','original_REI_central':'28/88','REI_working':'33/88',
       'old_auxiliary_PR122':'1/1 scoped','new_auxiliary_endpoint_atlas':'1/1 scoped created','original_node_delta':0}}
    return raw,pairs,curvs,np.array(F),np.array(J),np.array(y_arrays),summary

def dump_csv(path,rows):
    with path.open('w',newline='') as f:
        dw=csv.DictWriter(f,fieldnames=list(rows[0].keys()));dw.writeheader();dw.writerows(rows)

def write():
    root=ROOT/'results';root.mkdir(exist_ok=True)
    rows,pairs,curvs,F,J,Y,summary=compute()
    dump_csv(root/'BASS125_NATIVE_ENDPOINT_CHEMISTRY.csv',rows)
    # flattened pair table only scalar fields for interoperable CSV
    dump_csv(root/'BASS25_MATCHED_SHEAR_OPACITY.csv',[{k:v for k,v in p.items() if k!='source_comp'} for p in pairs])
    (root/'BASS25_SHEAR_SECANT_CURVATURE.json').write_text(json.dumps(curvs,ensure_ascii=False,indent=2)+'\n')
    np.savez_compressed(root/'BASS125_ACTUAL_NATIVE_FJ.npz',F=F,J=J,Y=Y,case_ids=np.array([r['case_id'] for r in rows],dtype='U32'))
    (root/'SCIENCE_RESULTS.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2,sort_keys=True)+'\n')
    print(json.dumps(summary,ensure_ascii=False,indent=2,sort_keys=True))

if __name__=='__main__':
    q=argparse.ArgumentParser();q.add_argument('--write',action='store_true');o=q.parse_args()
    if o.write:write()
    else:print(json.dumps(compute()[-1],indent=2,ensure_ascii=False))
