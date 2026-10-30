"""Targeted source-owned prospective alignment checks, no original IVP."""
import io,json,pathlib,sys,csv,hashlib
import numpy as np
import pytest
from scipy.special import lpmv
from numpy.polynomial.legendre import Legendre
ROOT=pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'code'))
import compute_alignment as m
SRC=ROOT/'source/PR151_ORIGINAL_SOURCE_FULL.zip'

@pytest.fixture(scope='session')
def source():return m.extract_parent(SRC)
@pytest.fixture(scope='session')
def results():
    f=ROOT/'results/ANGULAR_ALIGNMENT_50x193.npz'
    with np.load(f,allow_pickle=False) as d:return {k:d[k].copy() for k in d.files}

def test_preregistration_is_prospective_not_prior():
    x=json.loads((ROOT/'results/ANGULAR_ALIGNMENT_SUMMARY.json').read_text())
    assert x['registered_before_this_experiment']==m.PREREG_COMMIT
    assert x['earlier_work_status']=='POSTHOC_NOT_PREREGISTERED'

def test_locked_original_source_sha():
    assert m.sha(SRC)==m.ORIGINAL_ARCHIVE_SHA

def test_tampered_wrong_zip_rejected(tmp_path):
    fake=tmp_path/'wrong.zip';fake.write_bytes(b'not this source')
    with pytest.raises(ValueError,match='SOURCE_ZIP_SHA_MISMATCH'):m.extract_parent(fake)

def test_no_new_owner_ivp_and_shape(results):
    assert results['alignment'].shape==(50,193,4)
    assert results['eta'].shape==(50,193,4)
    summary=json.loads((ROOT/'results/ANGULAR_ALIGNMENT_SUMMARY.json').read_text())
    assert summary['new_ivps']==0 and summary['official_DAG_vigilode']=='7/23'

def test_original_case_epoch_and_cutoff_identity(source,results):
    st,hi,k=source
    assert np.array_equal(st['cases'],results['cases'])
    assert np.array_equal(st['x'],results['x'])
    assert results['cutoffs'].tolist()==list(m.CUT)

def test_zero_norm_initial_epoch_null_not_zero(results):
    assert not results['defined'][:,0,:].any()
    assert np.isnan(results['eta'][:,0,:]).all()
    assert np.isnan(results['alignment'][:,0,:]).all()

def test_exact_null_count(results):
    assert [int(np.sum(~results['defined'][:,:,k])) for k in range(4)]==[50]*4

def test_eta_bounds_only_defined(results):
    mask=results['defined'];x=results['eta'][mask]
    assert x.size==38400 and np.isfinite(x).all() and x.min()>=-1e-12 and x.max()<=1+1e-12

def test_alignment_bounds_only_defined(results):
    mask=results['defined'];a=results['alignment'][mask]
    assert np.isfinite(a).all() and a.min()>=-2-1e-12 and a.max()<=1+1e-12

def test_alignment_eta_identity(results):
    msk=results['defined']
    assert np.max(abs(results['alignment'][msk]-(1-3*results['eta'][msk])))<1e-14

def test_quadrature_basis_identity(source,results):
    st,hi,k=source
    for j,L in enumerate(m.CUT):
        a0,a2,eta,align,mask=m.integrate_alignment(hi['E_l0_plus'],k,L,96,True)
        assert np.max(abs(a0-results['I0'][:,:,j]))<1e-10
        assert np.max(abs(a2-results['I2'][:,:,j]))<1e-10
        assert np.nanmax(abs(align-results['alignment'][:,:,j]))<1e-10

def test_angular_power_norm_source_identity(source,results):
    st,hi,k=source
    assert np.max(abs(results['I0']-2*st['quadratic_power']))<1e-10

def test_original_shear_work_identity(source,results):
    st,hi,k=source
    rhs=(5/2)*st['bprime'][:,:,None]*(results['I0']-3*results['I2'])
    assert np.max(abs(rhs-st['shear_internal_work']))<1e-10

def test_positive_moments_and_order(results):
    assert np.all(results['I0']>=-1e-14)
    assert np.all(results['I2']>=-1e-14)
    assert np.all(results['I2']<=results['I0']+1e-14)

def test_independent_bases_manufactured_polynomial():
    t=np.zeros((1,1,12));t[0,0,0]=0.1
    k=np.ones(12)
    a=m.integrate_alignment(t,k,16,64,False)
    b=m.integrate_alignment(t,k,16,96,True)
    assert np.max(abs(a[0]-b[0]))<1e-12
    assert np.max(abs(a[1]-b[1]))<1e-12

def test_matched_25_csv_rows():
    with (ROOT/'results/ANGULAR_ALIGNMENT_25_MATCHED.csv').open() as f:
        r=list(csv.DictReader(f))
    assert len(r)==25 and r[0]['matched_pair']=='m00_r00'

def test_endpoints_50_csv_rows():
    with (ROOT/'results/ANGULAR_ALIGNMENT_50_ENDPOINT.csv').open() as f:
        r=list(csv.DictReader(f))
    assert len(r)==50

def test_mean_193_csv_rows():
    with (ROOT/'results/ANGULAR_ALIGNMENT_193_MEAN.csv').open() as f:
        r=list(csv.DictReader(f))
    assert len(r)==193

def test_actual_finite_endpoints(results):
    assert results['defined'][:,-1,:].all()

def test_science_plot_exists():
    assert (ROOT/'figures/S04_ALIGNMENT_EVOLUTION.png').is_file()
    assert (ROOT/'figures/S04_ALIGNMENT_EVOLUTION.svg').is_file()

def test_no_finite_closure_or_cmb_promotion():
    report=ROOT/'reports/RESEARCH_SCOPE.md'
    s=report.read_text(encoding='utf8')
    assert 'NOT a global closure bound' in s
    assert 'NOT observed CMB' in s
    assert 'NOT source-native E_l' in s
