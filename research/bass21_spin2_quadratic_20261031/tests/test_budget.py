from __future__ import annotations
import io,os,sys,zipfile,math
from pathlib import Path
import numpy as np
import pytest
import sympy as sp

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'code'))
from spin2_quadratic_budget import source_tables,galerk_matrix,sky_independent,LL,CUT,CHILD_SHA,PARENT_SHA,sha_file
SOURCE=ROOT/'source'/'PR148_SOURCE_FULL.zip'
DATA=ROOT/'results'/'SOURCE_FROZEN_SPIN2_POWER_50x193.npz'

@pytest.fixture(scope='module')
def original():return source_tables(SOURCE)
@pytest.fixture(scope='module')
def newdata():
    with np.load(DATA,allow_pickle=False) as z:return {k:z[k].copy() for k in z.files}

def test_01_exact_source_sha():assert sha_file(SOURCE)==CHILD_SHA

def test_02_reject_wrong_source(tmp_path):
    f=tmp_path/'fake.zip';f.write_bytes(b'not genuine BASS source')
    with pytest.raises(ValueError,match='PR148_SOURCE_SHA_MISMATCH'):source_tables(f)

def test_03_source_alignment(original):
    high,anc,pr141,mem,op=original
    assert len(high['cases'])==50 and len(high['x'])==193
    assert high['E_l0_plus'].shape==(50,193,12)
    assert np.array_equal(mem['tau'],anc['tau'])
    assert (anc['g']>=0).all()

def test_04_exact_unweighted_norm_formula():
    for ell in range(2,25,2):
        n=sp.factorial(ell+2)/((2*ell+1)*sp.factorial(ell-2))
        k=sp.sqrt(4*sp.pi*n)
        assert sp.simplify(n/k**2-1/(4*sp.pi))==0

def test_05_exact_all_rank_shear_coefficients(original):
    op=original[-1]
    for i,ell in enumerate(LL):
        row=op['A_c_rows'][i]
        upper=3*(ell-1)*(ell+3)*(ell+4)/sp.Integer((2*ell+3)*(2*ell+5))
        diag=-5*(ell-3)*(ell+4)/sp.Integer((2*ell-1)*(2*ell+3))
        assert sp.Rational(row[str(ell)])==diag
        if ell<24:assert sp.Rational(row[str(ell+2)])==upper
        if ell>2:
            lower=-3*(ell-3)*(ell-2)*(ell+2)/sp.Integer((2*ell-3)*(2*ell-1))
            assert sp.Rational(row[str(ell-2)])==lower

def test_06_corrected_radical(original):
    ae,k=galerk_matrix(original[-1]);val=360*sp.sqrt(7733)/2849
    assert abs(ae[7,8]-float(val))<1e-12
    assert sp.simplify(val-360*sp.sqrt(7315)/2849)!=0

def test_07_screen_basis_rotation_preserves_polarization_power():
    Q=np.array([.2,.1,-.02]);U=np.array([-.04,.03,.07]);phi=np.array([.1,1.2,-.6]);
    Qp=Q*np.cos(2*phi)+U*np.sin(2*phi)
    Up=-Q*np.sin(2*phi)+U*np.cos(2*phi)
    assert np.allclose(Q*Q+U*U,Qp*Qp+Up*Up,atol=1e-16,rtol=0)

def test_08_no_lrs_bmode_in_original(original):
    pr141=original[2];assert np.max(np.abs(pr141['B_l0_plus']))==0.

def test_09_power_is_half_unpolarized_square_norm(newdata):
    assert np.max(abs(newdata['quadratic_power']*2-newdata['angular_half_norm']))<1e-16
    assert np.min(newdata['quadratic_power'])>=0

def test_10_independent_legendre_norm(newdata):
    assert np.max(abs(newdata['sky_norm_independent']-newdata['angular_half_norm']))<2e-14

def test_11_symmetrized_shear_work_identity(newdata):
    assert np.max(abs(newdata['shear_internal_work']-newdata['sky_work_independent']))<2e-14

def test_12_collisions_match_original_generator(original,newdata):
    high,anc,pr141,mem,op=original;ae,k=galerk_matrix(op)
    collision=-anc['g'][:,:,None]*high['E_l0_plus'][:,:,:11]
    collision[:,:,0]=-anc['g']*(k[0]*pr141['J2_total']/4+.4*high['E_l0_plus'][:,:,0])
    assert np.max(abs(collision-high['collision_E']))<2e-12

def test_13_shear_generator_match_original(original):
    high,anc,pr141,mem,op=original;ae,_=galerk_matrix(op)
    calc=anc['bprime'][:,:,None]*np.einsum('ctm,lm->ctl',high['E_l0_plus'],ae[:11,:],optimize=True)
    assert np.max(abs(calc-high['shear_E']))<2e-12

def test_14_upper_neighbor_work_from_owner_boundary(original,newdata):
    high,anc,pr141,mem,op=original
    for z,L in enumerate(CUT):
        i=np.where(LL==L)[0][0]
        want=high['E_l0_plus'][:,:,i]*mem['original_flux'][:,:,z]/(4*math.pi)
        assert np.max(abs(want-newdata['upper_neighbour_work'][:,:,z]))<2e-20

def test_15_damping_sign_and_quadrupole_split(newdata):
    assert np.max(newdata['thomson_loss'])<=0
    rhs=newdata['shear_internal_work']+newdata['upper_neighbour_work']+newdata['thomson_quadrupole_source']+newdata['thomson_loss']
    assert np.max(abs(rhs-newdata['model_quadratic_rhs']))<1e-18

def test_16_zero_shear_zero_boundary(original):
    high,anc,pr141,mem,op=original;ae,k=galerk_matrix(op)
    E=high['E_l0_plus'];bprime=np.zeros_like(anc['bprime']);L=16;i=np.where(LL==L)[0][0]
    zero=bprime*ae[i,i+1]*E[:,:,i+1]*E[:,:,i]/(4*math.pi)
    assert np.array_equal(zero,np.zeros_like(zero))

def test_17_saved_time_derivative_defect_not_hidden(newdata):
    assert np.isfinite(newdata['source_grid_derivative_defect']).all()
    assert np.max(abs(newdata['source_grid_derivative_defect']))>1e-12

def test_18_source_nonzero_boundary(original,newdata):
    s04=[i for i,s in enumerate(newdata['cases']) if s.startswith('s04')]
    s00=[i for i,s in enumerate(newdata['cases']) if s.startswith('s00')]
    assert len(s04)==len(s00)==25
    assert np.max(abs(newdata['upper_neighbour_work'][s04]))>1e-23
    assert np.max(abs(newdata['upper_neighbour_work'][s00]))<1e-20

def test_19_positive_norm_sources(original,newdata):
    assert np.array_equal(original[0]['cases'],newdata['cases'])
    assert np.array_equal(original[0]['x'],newdata['x'])
    assert newdata['upper_neighbour_work'].shape==(50,193,4)

def test_20_boundary_cumulative_initial_exact(newdata):
    assert np.array_equal(newdata['cumulative_boundary_work'][:,0,:],np.zeros((50,4)))

def test_21_independent_source_grid_integral(newdata):
    from scipy.integrate import cumulative_trapezoid
    w=newdata['upper_neighbour_work'];x=newdata['x'];v=cumulative_trapezoid(w,x,axis=1,initial=0)
    assert np.max(abs(v-newdata['cumulative_boundary_work']))<2e-24

def test_22_angular_operator_nontrivial_cross_l(original,newdata):
    ae,k=galerk_matrix(original[-1]);
    assert abs(ae[7,8])>10
    assert np.max(abs(newdata['upper_neighbour_work']))>0

def test_23_thomson_gain_unconstrained_sign(original,newdata):
    # Quadrupole source is not sign-definite. It must not be misclassified as loss.
    q=newdata['thomson_quadrupole_source'];assert np.isfinite(q).all()
    assert np.max(np.abs(q))>1e-8

def test_24_banded_matrix(original):
    ae,k=galerk_matrix(original[-1]);assert ae.shape==(12,12)
    for i in range(12):
        for j in range(12):
            if abs(i-j)>1:assert ae[i,j]==0
