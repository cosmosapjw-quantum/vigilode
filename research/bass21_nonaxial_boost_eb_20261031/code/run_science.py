"""Produce original-BASS21 observer-frame spin-2 E/B modes, no IVPs."""
from __future__ import annotations
import csv,hashlib,json,os,pathlib,sys
import numpy as np
from nonaxial_boost import PARENT_SHA,load_original,kernel_for_frame,OUTPUT_MODES,FRAMES,ELL_INPUT,k_ell,spin_y,spatial_grid,observer_inverse_geometry

ROOT=pathlib.Path(__file__).resolve().parents[1]
INPUT=ROOT/'source'/'PR153_FULL_SOURCE.zip'

def wcsv(path,rows):
  with path.open('w',newline='',encoding='utf8') as fh:
    w=csv.DictWriter(fh,fieldnames=list(rows[0]),lineterminator='\n');w.writeheader();w.writerows(rows)

def run():
  data=load_original(INPUT)
  cases=data['cases'].astype(str);x=data['x'];src=data['E_l0_plus']
  assert len(cases)==50 and src.shape==(50,193,12)
  target=OUTPUT_MODES
  raw=src.reshape((-1,12))
  names=list(FRAMES)
  E=np.empty((len(names),50,193,len(target)),dtype=np.complex128)
  B=np.empty_like(E)
  grids={};kernels={}
  for k,name in enumerate(names):
    b=FRAMES[name]
    ke,kb=kernel_for_frame(b,72,96,4)
    E[k]=(raw@ke).reshape((50,193,len(target)))
    B[k]=(raw@kb).reshape((50,193,len(target)))
    kernels[name]=(ke,kb)
    if name in ('rest','x005','tilt45'):
      he,hb=kernel_for_frame(b,96,128,4)
      delta=max(float(np.max(np.abs(ke-he))),float(np.max(np.abs(kb-hb))))
      grids[name]={'kernel_GL72_96_vs_GL96_128_max':delta,'physical_E_diff_max':float(np.max(abs(raw@(ke-he)))),'physical_B_diff_max':float(np.max(abs(raw@(kb-hb))))}
    print('FRAME',name,'MAX_B',float(np.max(abs(B[k]))),'check',grids.get(name),flush=True)
  E1,B1=kernel_for_frame(FRAMES['x005'],72,96,1)
  weights=src.reshape((-1,12))
  maxd1=float(np.max(abs(weights@B1)))
  maxd1kernel=float(np.max(abs(B1)))
  m0=np.array([j for j,(l,m) in enumerate(target) if m==0 and l in ELL_INPUT])
  sidx=np.array([np.where(ELL_INPUT==target[j][0])[0][0] for j in m0])
  rest_gap=float(np.max(abs(E[0][:,:,m0]-src[:,:,sidx])))
  rest_b=float(np.max(abs(B[0])))
  rest_nonzero=np.array([j for j in range(len(target)) if j not in m0])
  rest_other=float(np.max(abs(E[0][:,:,rest_nonzero])))
  # Reality E/B_{l,-m}=(-1)^m conj(E/B_{l,m})
  reality_e=reality_b=0.
  for j,(l,m) in enumerate(target):
    j2=target.index((l,-m))
    reality_e=max(reality_e,float(np.max(abs(E[:,:,:,j2]-(-1)**m*np.conj(E[:,:,:,j])))))
    reality_b=max(reality_b,float(np.max(abs(B[:,:,:,j2]-(-1)**m*np.conj(B[:,:,:,j])))))
  # Reflection under x -> -x rotation about z pi: a_lm(-beta_x) = (-1)^m a_lm(+beta_x)
  a=names.index('x005'); n=names.index('negx005'); refl_e=refl_b=0.
  for j,(l,m) in enumerate(target):
    refl_e=max(refl_e,float(np.max(abs(E[n,:,:,j]-(-1)**m*E[a,:,:,j]))))
    refl_b=max(refl_b,float(np.max(abs(B[n,:,:,j]-(-1)**m*B[a,:,:,j]))))
  pure2_E=kernels['x005'][0][0,target.index((2,0))]
  pure2_B21=kernels['x005'][1][0,target.index((2,1))]
  # Additional small-beta derivative against closed form, exact first-order coefficient i(d-1)/sqrt6
  kb_small=kernel_for_frame((1e-4,0.,0.),72,96,4)[1]
  b21deriv=kb_small[0,target.index((2,1))]/1e-4
  b21analytic=1j*3/np.sqrt(6)
  # Local reciprocal geometry, four-vectors and Doppler factors
  mu,phi,w=spatial_grid(24,32)
  maxrec_dir=maxrec_dop=maxrec_phase=0.
  for b in [FRAMES['x005'],FRAMES['tilt45']]:
    m0n,ph0n,ph,dd=observer_inverse_geometry(mu,phi,np.asarray(b))
    mb,phb,invphase,invdd=observer_inverse_geometry(m0n,ph0n,-np.asarray(b))
    maxrec_dir=max(maxrec_dir,float(np.max(abs(mb-mu))))
    maxrec_dop=max(maxrec_dop,float(np.max(abs(dd*invdd-1))))
    maxrec_phase=max(maxrec_phase,float(np.max(abs(ph*invphase-1))))
  def samples(name):return {'E':E[names.index(name)],'B':B[names.index(name)]}
  ss=np.array([str(t).startswith('s04') for t in cases]);sw=np.array([str(t).startswith('s00') for t in cases]);assert ss.sum()==sw.sum()==25
  primary_j=target.index((2,1)); mE2=target.index((2,0)); mB31=target.index((3,1))
  endpoint=[];pairs=[];epoch=[]
  ix={c:j for j,c in enumerate(cases)}
  for name in names:
    k=names.index(name)
    for j,case in enumerate(cases):
      endpoint.append({'frame':name,'case':case,'x_end':float(x[-1]),'source_E20_final':float(src[j,-1,0]),'E_20_final_real':float(E[k,j,-1,mE2].real),'E_21_final_real':float(E[k,j,-1,primary_j].real),'E_21_final_imag':float(E[k,j,-1,primary_j].imag),'B_21_final_real':float(B[k,j,-1,primary_j].real),'B_21_final_imag':float(B[k,j,-1,primary_j].imag),'B_31_final_imag':float(B[k,j,-1,mB31].imag),'max_abs_B_final_l2_8':float(np.max(abs(B[k,j,-1,:])))})
    for m in range(5):
      for t in range(5):
        a0=f's00_m{m:02d}_r{t:02d}';a4=f's04_m{m:02d}_r{t:02d}'
        pairs.append({'frame':name,'matched':f'm{m:02d}_r{t:02d}','delta_B_21_final_imag':float(B[k,ix[a4],-1,primary_j].imag-B[k,ix[a0],-1,primary_j].imag),'delta_E_20_final_real':float(E[k,ix[a4],-1,mE2].real-E[k,ix[a0],-1,mE2].real)})
    for j,t in enumerate(x):
      epoch.append({'frame':name,'x':float(t),'s04_mean_B21_imag':float(np.mean(B[k,ss,j,primary_j].imag)),'s00_mean_B21_imag':float(np.mean(B[k,sw,j,primary_j].imag)),'s04_mean_E20_real':float(np.mean(E[k,ss,j,mE2].real)),'s04_mean_abs_B_l2_8':float(np.mean(np.max(abs(B[k,ss,j,:]),axis=-1)))})
  output=ROOT/'results';output.mkdir(exist_ok=True)
  np.savez_compressed(output/'OBSERVER_NONAXIAL_EB_50x193x5.npz',cases=cases,x=x,frame_names=np.array(names),frame_beta_vectors=np.array(list(FRAMES.values())),ell=np.array([z[0] for z in target]),m=np.array([z[1] for z in target]),E_lm=E,B_lm=B,source_E_l0=src)
  wcsv(output/'ENDPOINT_50x5_NONAXIAL_EB.csv',endpoint)
  wcsv(output/'MATCHED25x5_NONAXIAL_EB.csv',pairs)
  wcsv(output/'EPOCH193x5_NONAXIAL_EB.csv',epoch)
  s04final={name:{'mean_B21_imag':float(np.mean(samples(name)['B'][ss,-1,primary_j].imag)),'mean_abs_B_l2_8':float(np.mean(np.max(abs(samples(name)['B'][ss,-1,:]),axis=-1)))} for name in names}
  summary={
    'schema':'vigilode.bass21.nonaxial_spin2_bolometric_eb_v1',
    'prereg_sha':'e1a359101b52de720e024b6e57b4615cee79acbd',
    'parent_pr153_full_zip_sha256':PARENT_SHA,'owner_E_NPZ_SHA256':'b087a54bcd3bdbfaef5d91165634bf5c7590e6c43417112e4de0168327b3366b',
    'physical_source_cases':50,'physical_source_epochs':193,'frames':{k:list(v) for k,v in FRAMES.items()},'output_mode_count':len(target),'count_complex_E':int(E.size),'count_complex_B':int(B.size),'owner_ivps':0,
    'max_abs_B_by_frame':{name:float(np.max(abs(samples(name)['B']))) for name in names},
    'max_abs_E_by_frame':{name:float(np.max(abs(samples(name)['E']))) for name in names},
    's04_endpoint':s04final,
    'max_bolometric_B_x005':float(np.max(abs(samples('x005')['B']))),
    'max_weight1_control_B_kernel':maxd1kernel,'max_weight1_control_B_physical':maxd1,
    'max_rest_E_match':rest_gap,'max_rest_B':rest_b,'max_rest_other_E':rest_other,
    'max_E_spin_reality':reality_e,'max_B_spin_reality':reality_b,'reflection_E_error':refl_e,'reflection_B_error':refl_b,
    'pure_E20_to_B21_beta_0_05':{'real':float(pure2_B21.real),'imag':float(pure2_B21.imag)},'first_order_B21_over_beta':{'real':float(b21deriv.real),'imag':float(b21deriv.imag),'analytic_imag':float(b21analytic.imag),'absolute_error':float(abs(b21deriv-b21analytic))},
    'reciprocal_aberration_mu_error':maxrec_dir,'reciprocal_doppler_error':maxrec_dop,'reciprocal_screen_phase_error':maxrec_phase,
    'angular_quadrature_checks':grids,'physical_scope':'bandlimited original BASS21 source frozen Q24 bolometric observer boost, NOT full sky, original owner dynamics, new IVP or observational CMB',
    'DAG':{'vigilode':'7/23','REI_central':'28/88','REI_working':'33/88','BASS_CE1':'9/15','successor_admitted':'0/12','delta':0},
    'independent_author':'NOT_RUN','native_subagent':'NOT_RUN','production':'HOLD'}
  (output/'SCIENCE_SUMMARY.json').write_text(json.dumps(summary,indent=2,sort_keys=True)+'\n')
  from matplotlib import pyplot as plt
  figdir=ROOT/'figures';figdir.mkdir(exist_ok=True)
  plt.rcParams['svg.hashsalt']='vigilode-nonaxial-20261031'
  plt.rcParams['font.family']='DejaVu Sans'
  fig,ax=plt.subplots(figsize=(8,4.5));
  for name in ('rest','x001','x005','negx005','tilt45'):
    y=np.mean(samples(name)['B'][ss,:,primary_j].imag,axis=0)
    ax.plot(x,y,label=name)
  ax.set(xlabel='Original saved x=ln(a/a_ref)',ylabel='Im B_{2,1} (bolometric source E convention)',title='Source-owned BASS21 LRS Q24: nonaxial boost induces B mode');ax.legend(fontsize=8);fig.tight_layout();fig.savefig(figdir/'ORIGINAL_S04_B21_EVOLUTION.png',dpi=155);fig.savefig(figdir/'ORIGINAL_S04_B21_EVOLUTION.svg',metadata={'Date':'2026-10-31','Creator':'BASS21 nonaxial observer scientific source'});plt.close(fig)
  fig,ax=plt.subplots(figsize=(8,4.5));
  for name in ('x001','x005','tilt45'):
    z=np.mean(np.abs(samples(name)['B'][ss,-1,:]),axis=0)
    vals=np.array([float(np.sqrt(np.sum(z[[j for j,t in enumerate(target) if t[0]==l]]**2))) for l in range(2,9)])
    ax.semilogy(np.arange(2,9),vals,marker='o',label=name)
  ax.set(xlabel='l (summed |B_lm| over m)',ylabel='S04 final mean mode-magnitude proxy',title='Induced observer B multipole support (source Q24)');ax.legend(fontsize=8);fig.tight_layout();fig.savefig(figdir/'ORIGINAL_S04_B_SPECTRUM.png',dpi=155);fig.savefig(figdir/'ORIGINAL_S04_B_SPECTRUM.svg',metadata={'Date':'2026-10-31','Creator':'BASS21 nonaxial observer scientific source'});plt.close(fig)
  print('SUMMARY:',json.dumps({k:summary[k] for k in ('count_complex_E','count_complex_B','max_abs_B_by_frame','max_weight1_control_B_physical','max_rest_E_match','max_E_spin_reality','max_B_spin_reality','reflection_B_error','reciprocal_screen_phase_error','first_order_B21_over_beta')},indent=2),flush=True)
  return summary
if __name__=='__main__':run()
