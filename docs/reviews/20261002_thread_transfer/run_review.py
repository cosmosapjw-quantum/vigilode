#!/usr/bin/env python3
"""Reproduce the isolated review, without network, Rust, holdouts or mutations."""
from __future__ import annotations
from fractions import Fraction as F
from pathlib import Path
import hashlib, json, math, platform, subprocess, sys, time
import sympy
from probes import (quadratic_component, feasible_radius, peval,
                    matrix_at, path_sum_action, path_sum_matrix, decode_bits)
ROOT=Path(__file__).resolve().parent

def digest(path): return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def main():
    output=ROOT/'results'; output.mkdir(exist_ok=True)
    tests=subprocess.run([sys.executable,'-m','pytest','-q',str(ROOT/'test_probes.py')],cwd=ROOT,text=True,capture_output=True)
    (output/'TESTS.txt').write_text(tests.stdout+tests.stderr)
    if tests.returncode: raise SystemExit(tests.returncode)
    target=json.loads((ROOT/'inputs/target_bits.json').read_text())
    cases=[quadratic_component(target,u) for u in range(16)]
    weights=[abs(decode_bits(s)) for s in target['b']]
    rows=[]
    for n in [1,2,4,8,16]:
        ps=[p for c in cases[:n] for p in c['constraints']]
        answer=feasible_radius(ps)
        assert answer['status']=='feasible'
        lo,hi=[None if q is None else F(q) for q in answer['safe_interval']]
        grid=[F(.001)*4**j for j in range(6)]
        attempts=[{'radius':float(d),'closes':all(peval(p,d)<=d for p in ps)} for d in grid]
        b0=max(p[0] for p in ps)
        proposed=F(float(2*b0))
        assert all(peval(p,proposed)<=proposed for p in ps)
        worst=F(0); scaled=[]; full_work={'mul':0,'add':0}; action_work={'mul':0,'add':0}
        for c in cases[:n]:
            hh=matrix_at(c,proposed)
            e=path_sum_action(hh,c['a'],action_work)
            matrix_e=path_sum_matrix(hh,c['a'],full_work)
            assert e==matrix_e==[peval(p,proposed) for p in c['E_polynomials']]
            assert all(abs(r-k)<=ee for r,k,ee in zip(c['root'],c['candidate'],e))
            o=sum(w*v for w,v in zip(weights,e)); s=sum(w*v for w,v in zip(weights,c['serial']))
            worst=max(worst,o/s)
            scale=1e-8+1e-6*max(abs(float(c['y'])),abs(float(c['y']+c['candidate'][0])))
            scaled.append(float(o)/scale)
        rows.append({
            'dimension':n,'existing_six_point_schedule':attempts,
            'certified_safe_radius_interval_approx':[float(lo),None if hi is None else float(hi)],
            'certified_safe_radius_interval_exact':answer['safe_interval'],
            'zero_radius_majorant_B':float(b0),
            'two_B_binary64_radius':float(proposed),'two_B_bits':float(proposed).hex(),
            'two_B_exact_recheck':True,
            'max_state_bound_at_two_B':float(max(peval(p,proposed) for p in ps)),
            'max_projection_width_ratio_to_existing_serial_algebra':float(worst),
            'stage_projection_term_WRMS_diagnostic':math.sqrt(sum(v*v for v in scaled)/n),
            'WRMS_scope':'stage-bound term only; not a full native certificate, ODE LTE or acceptance',
            'sum_matrix_nonzero_work':full_work,'action_first_nonzero_work':action_work,
            'work_scope':'reference scalar arithmetic, common skip-zero policy; excludes H/residual/witness construction, pool, rounding and allocation cost',
            'native_Rust_rounding_validation':'not_run'
        })
    selected_clock=0.05527225150005569+0.03512646776226307
    source_counts=[{'n':n,'pivot_comparisons':n*(n-1)//2,'multiplier_zero_tests':n*(n-1)//2,'fast_J_plus_W_f64_slots':2*n*n,'diagonal_certificate_J_plus_dense_U_slots':2*n*n} for n in [8,100,400,10000]]
    result={
        'schema':'vigilode-thread-transfer-probes-v1',
        'source_commit':'d1e9ba3b0125ee478c28d0b2c280ca0869289c15',
        'preregistration_commit':'f2797d8891eb7201da31576080f2118bb23b7b1b',
        'environment':{'python':platform.python_version(),'sympy':sympy.__version__,'native_cargo':'not available in runtime PATH'},
        'scope':'isolated exact-real review probes; no native solver, timing, holdout or CI campaign',
        'test_summary':tests.stdout.strip().splitlines()[-1],
        'source_selection_sha256':digest(ROOT/'inputs/target_bits.json'),
        'rows':rows,
        'arithmetic_ratio_action_to_sum_matrix':(135+159)/(216+408),
        'nonmonotone_radius_example':{'p':'29/100+(4/5)D^2','feasible_interval':'[(1-sqrt(9/125))/(8/5), (1+sqrt(9/125))/(8/5)]','old_schedule_closes':False,'rational_witness':'1/2','witness_margin':'1/100'},
        'fast_driver_static_lower_bounds':source_counts,
        'Amdahl_instruction_only':{'selected_clock_share':selected_clock,'perfect_removal_ceiling':1/(1-selected_clock),'not_a_wall_time_prediction':True},
        'homotopy_equal_W_model':{'P_at_least_8_margin':'1+p1-8*pf','p1_0p7_pf_0p2_margin':0.1,'costs_excluded':['RHS','certificate','synchronization','fallback overhead','pool']},
        'forbidden_inferences':['no native radius admission claim','no ODE error certificate claim','no real-wall-speedup claim','no claim that current Q2 uses doubling','no general RVJ replacement','no claim of bitwise equivalence of reordered floating point operations']
    }
    # Cross-check the displayed analytic example, independent of the radius search.
    d=F(1,2)
    assert d-F(29,100)-F(4,5)*d*d==F(1,100)
    (output/'PROBE_RESULTS.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
    print(tests.stdout.strip())
    print('wrote',output/'PROBE_RESULTS.json')
if __name__=='__main__': main()
