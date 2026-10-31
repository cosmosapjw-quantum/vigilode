"""Tests explicitly scoped to source-frozen local observer boost, NOT cosmological IVPs."""
import io, json, pathlib, numpy as np, pytest
from scipy.special import lpmv
from numpy.polynomial.legendre import leggauss
from nonaxial_boost import (FRAMES,OUTPUT_MODES,ELL_INPUT,k_ell,spin_y,spatial_grid,
       observer_inverse_geometry,kernel_for_frame,load_original,sha, PARENT_SHA)
ROOT=pathlib.Path(__file__).resolve().parents[1]

def results():
  with np.load(ROOT/'results'/'OBSERVER_NONAXIAL_EB_50x193x5.npz',allow_pickle=False) as f:
    return {k:f[k] for k in f.files}

def summary():return json.loads((ROOT/'results'/'SCIENCE_SUMMARY.json').read_text())

def test_source_identity():
  p=ROOT/'source'/'PR153_FULL_SOURCE.zip';assert sha(p.read_bytes())==PARENT_SHA
  q=load_original(p);assert q['E_l0_plus'].shape==(50,193,12)

def test_negative_parent_mutation(tmp_path):
  p=ROOT/'source'/'PR153_FULL_SOURCE.zip'
  tiny=tmp_path/'tampered.zip';tiny.write_bytes(p.read_bytes()[:2048])
  with pytest.raises(ValueError,match='PARENT_PR153_SHA_MISMATCH'):load_original(tiny)

@pytest.mark.parametrize('l', [2,3,4,6,8])
def test_wigner_spin0_basis_lm0(l):
  mu=np.linspace(-.9,.9,31);phi=np.linspace(0,2.1,31)
  for s in (-2,2):assert np.max(abs(spin_y(l,0,s,mu,phi)-lpmv(2,l,mu)/k_ell(l)))<1e-12

@pytest.mark.parametrize('l,m',[(2,0),(2,1),(3,-1),(3,2),(6,4),(8,-3)])
def test_spin_harmonic_reality(l,m):
  u,p,_=spatial_grid(24,36)
  a=np.conj(spin_y(l,m,2,u,p));b=spin_y(l,-m,-2,u,p)
  assert np.max(abs(a-(-1)**(m+2)*b))<3e-12

def test_doppler_screen_norm_and_reciprocity():
  mu,ph,_=spatial_grid(24,36)
  for b in (FRAMES['x005'],FRAMES['tilt45']):
    mup,php,phase,D=observer_inverse_geometry(mu,ph,b)
    mback,pback,iphase,iD=observer_inverse_geometry(mup,php,-np.asarray(b))
    assert np.max(abs(mback-mu))<2e-12
    assert np.max(abs(D*iD-1))<2e-12
    assert np.max(abs(phase*iphase-1))<2e-12
    assert np.max(abs(abs(phase)-1))<2e-12

def test_d1_zero_EB_mixing_pure_Q24():
  ke,kb=kernel_for_frame(FRAMES['x005'],72,96,1)
  assert np.max(abs(kb))<2e-12

def test_d4_nonzero_EB_mixing_pure_quadrupole():
  ke,kb=kernel_for_frame(FRAMES['x005'],72,96,4)
  assert abs(kb[0,OUTPUT_MODES.index((2,1))].imag)>0.06
  assert abs(kb[0,OUTPUT_MODES.index((2,-1))].imag)>0.06

def test_d4_pure2_firstorder_coefficient():
  e,b=kernel_for_frame((1e-4,0,0),72,96,4)
  rate=b[0,OUTPUT_MODES.index((2,1))]/1e-4
  assert abs(rate-1j*3/np.sqrt(6))<2e-7

def test_axial_control_zero_B():
  _,b=kernel_for_frame((0,0,0.05),72,96,4)
  assert np.max(abs(b))<2e-12

def test_rest_full_source_returns_original_even_E0():
  f=results();E=f['E_lm'];B=f['B_lm'];src=f['source_E_l0']
  for l in (2,4,6,8):
    j=OUTPUT_MODES.index((l,0)); i=int(np.where(ELL_INPUT==l)[0][0]);assert np.max(abs(E[0,:,:,j]-src[:,:,i]))<2e-12
  assert np.max(abs(B[0]))<2e-12

def test_all_observer_arrays_shape_finite():
  f=results();assert f['E_lm'].shape==f['B_lm'].shape==(5,50,193,77)
  assert np.all(np.isfinite(f['E_lm'])) and np.all(np.isfinite(f['B_lm']))
  assert len(set(f['cases'].astype(str)))==50

def test_reality_source_all_modes():
  f=results();E=f['E_lm'];B=f['B_lm']
  for j,(l,m) in enumerate(OUTPUT_MODES):
    k=OUTPUT_MODES.index((l,-m));assert np.max(abs(E[:,:,:,k]-(-1)**m*np.conj(E[:,:,:,j])))<3e-12
    assert np.max(abs(B[:,:,:,k]-(-1)**m*np.conj(B[:,:,:,j])))<3e-12

def test_velocity_opposite_azimuth_parity():
  f=results()
  for j,(l,m) in enumerate(OUTPUT_MODES):
    assert np.max(abs(f['E_lm'][3,:,:,j]-(-1)**m*f['E_lm'][2,:,:,j]))<3e-12
    assert np.max(abs(f['B_lm'][3,:,:,j]-(-1)**m*f['B_lm'][2,:,:,j]))<3e-12

def test_exact_frozen_source_B_nonzero():
  f=results();B=f['B_lm']
  assert np.max(abs(B[2]))>1e-3 and np.max(abs(B[4]))>1e-3
  assert np.max(abs(B[0]))<1e-12

def test_quadrature_robustness_kernel():
  a,b=kernel_for_frame(FRAMES['x005'],72,96,4)
  c,d=kernel_for_frame(FRAMES['x005'],96,128,4)
  assert np.max(abs(a-c))<1e-10 and np.max(abs(b-d))<1e-10

def test_summary_count_no_ivp():
  s=summary();assert s['owner_ivps']==0
  assert s['count_complex_E']==s['count_complex_B']==3715250
  assert s['DAG']['vigilode']=='7/23' and s['DAG']['delta']==0 and s['production']=='HOLD'

def test_summary_independent_d1_against_d4():
  s=summary();assert s['max_weight1_control_B_physical']<1e-12
  assert s['max_bolometric_B_x005']>1e-3

def test_source_grid_order():
  f=results();assert np.all(np.diff(f['x'])>0)
  assert f['x'].size==193

def test_d6_bolometric_angular_norm_independent_variable_change():
  from nonaxial_boost import basis_from_mu_phi,n_from_mu_phi
  f=results();src=f['source_E_l0'];z=src[42,-1,:] # original strong-shear endpoint
  for b in [FRAMES['x005'],FRAMES['tilt45']]:
    mu_p,ph_p,w=spatial_grid(96,128)
    mu_s,ph_s,phase,D=observer_inverse_geometry(mu_p,ph_p,np.asarray(b))
    qs=np.zeros_like(mu_s)
    for e,l in zip(z,ELL_INPUT):qs+=(e/k_ell(int(l)))*lpmv(2,int(l),mu_s)
    boosted=np.sum(w*(qs*D**4)**2)
    mu,ph,w0=spatial_grid(96,128)
    source=np.zeros_like(mu)
    for e,l in zip(z,ELL_INPUT):source+=(e/k_ell(int(l)))*lpmv(2,int(l),mu)
    n=n_from_mu_phi(mu,ph);beta=np.linalg.norm(b)
    dop=(1-beta*(n@(np.asarray(b)/beta)))/np.sqrt(1-beta**2)
    source_integral=np.sum(w0*dop**6*source**2)
    assert abs(boosted-source_integral)<1e-11


def test_fullsky_source_l2_spin_harmonic_orthonormality():
  mu,phi,w=spatial_grid(60,80)
  for l,m in [(2,0),(2,1),(3,1),(5,-2),(8,3)]:
    y=spin_y(l,m,2,mu,phi)
    assert abs(np.sum(w*(y*np.conj(y)))-1)<1e-11


def test_no_fake_B_for_zero_polarization():
  K_E,K_B=kernel_for_frame(FRAMES['x005'],36,48,4)
  z=np.zeros((5,12))
  assert not np.any(abs(z@K_E)) and not np.any(abs(z@K_B))
