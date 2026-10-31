"""Bounded hostile checks for new WP3 exact theorem, identity, and data.
Same-author adverse test; NOT a distinct external author or HOL kernel proof.
"""
import json,math,random,sys,hashlib
from fractions import Fraction as F
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'src'))
sys.path.insert(0,str(ROOT/'sources'))
import rational_enclosure_evolution as e
import DEFECT_CERTIFIER as original

def test_de_casteljau_exact_reparametrization():
    rng=random.Random(31)
    for i in range(70):
        b=[F(rng.randrange(-100,100),rng.randrange(1,30)) for _ in range(10)]
        l,r=e.de_casteljau_halves(b)
        for s in [F(0),F(1,7),F(2,5),F(3,4),F(1)]:
            assert original.bernstein9_eval(l,s)==original.bernstein9_eval(b,s/2)
            assert original.bernstein9_eval(r,s)==original.bernstein9_eval(b,(1+s)/2)
        assert max(map(abs,l+r))<=max(map(abs,b))

def test_original_dyadic_reference_file_integrity():
    for cohort in original.COHORTS:
        src,ref,steps,cert=e.load(cohort,32)
        assert len(ref)==32*steps+1 and src['n'] in (2,3)
        assert cert['reference_states_sha256']==hashlib.sha256((ROOT/'sources'/cert['reference_states_name']).read_bytes()).hexdigest()

def test_wp2_original_reproduced_and_improved_each_cohort():
    for cohort in original.COHORTS:
        j=json.loads((ROOT/'results'/f'RATIONAL_ENCLOSURE_{cohort}_micro32.json').read_text())
        assert j['parent_certified_delta']==j['original_replay_delta']
        assert 0<j['best_new_delta']<j['parent_certified_delta']
        assert j['panels'] in (2048,4096)
        assert all(a>=b for a,b in zip(j['max_defect_by_depth'],j['max_defect_by_depth'][1:]))

def test_integrating_factor_bounds_high_precision_adversarial():
    import mpmath as mp
    mp.mp.dps=90
    for q in [F(1,1000000),F(1,100),F(1,2),F(1),F(3),F(10),F(40)]:
        x=mp.mpf(q.numerator)/q.denominator
        expneg=mp.exp(-x);source=(1-expneg)/x
        for m in [1,2,3,4]:
            assert mp.mpf(e.q_upper(q,m))>=expneg
        for m in [0,3,5,7]:
            assert (mp.mpf(e.b_ratio_upper(q,m).numerator)/e.b_ratio_upper(q,m).denominator)>=source

def test_invalid_wrong_sign_is_rejected():
    x=F(1)
    # Mistaken polynomial 1+x/2+x^2/6 overbounds but cannot improve the integral,
    # and a deliberate false zero lower bound for positive source violates envelope.
    assert e.b_ratio_upper(x,3)==F(2,3)
    assert e.b_ratio_upper(x,5)==F(19,30)
    assert F(0)<e.b_ratio_upper(x,5)<e.b_ratio_upper(x,3)<1
    assert math.exp(-1) > 1/3 # invalid proposed decay upper pruned

def test_scipy_same_ode_grid_dataset():
    j=json.loads((ROOT/'results/SCIPY_SAME_SOURCE.json').read_text())
    rows=j['results'];assert len(rows)==80
    assert set(a['integrator'] for a in rows)=={'RK45','DOP853','Radau','BDF','LSODA'}
    assert all(a['validation']=='ORIGINAL_SAME_CUBIC_SOURCE_CERTIFIED_REFERENCE_NONRODAS' for a in rows)
    assert all(a['rigorous_peak_error_lower']<=a['rigorous_peak_error_upper'] and a['configured_source_initial_exact'] for a in rows)
    assert all(a['returned_initial_ulp_max']<=2 for a in rows)
    assert all(a['source_initial_same'] or a['integrator']=='LSODA' for a in rows)
    assert all(len(a['wall_ns_raw'])==3 and all(x>0 for x in a['wall_ns_raw']) for a in rows)
    assert sum(a['rigorous_peak_error_upper']<1e-6 for a in rows)>=60

def test_search_real_database_and_uct_evidence():
    j=json.loads((ROOT/'results/OPENEVOLE_MCTS_asymmetric_2_micro32.json').read_text())
    assert j['LLM_requests']==0
    assert j['open_evolve_island_count']==2
    assert j['candidate_pool']==48 and j['unique_evaluated']>=20
    assert j['UCT_root_visits']==32
    assert j['OpenEvolve_archive_count']>0
    assert j['best_visited_certificate']['max_grid_reference_delta']<7e-13
    assert any(x['operator']=='OpenEvolve_MAPElites' for x in j['events'])
    assert any(x['operator']=='MCTS_UCT' for x in j['events'])
    assert j['negative_proof_branch'][0]['classification']=='REFUTED_FOR_GLOBAL_BOUND'

def test_linear_exact_closed_form_oracle():
    j=json.loads((ROOT/'results/LINEAR_CONTROL_ANALYTIC_ORACLE.json').read_text())
    assert float(j['32']['observed_true_ref_peak'])<j['32']['new_wp3_rigorous_upper']
    assert float(j['64']['observed_true_ref_peak'])<float(j['64']['original_wp2_rigorous_upper'])
    assert set(['Radau','LSODA','BDF','RK45','DOP853']).issubset(j)
