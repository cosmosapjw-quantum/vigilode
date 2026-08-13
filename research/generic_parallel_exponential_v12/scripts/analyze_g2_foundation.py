#!/usr/bin/env python3
import argparse, hashlib, json, math
from pathlib import Path


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('canonical', type=Path)
    ap.add_argument('independent', type=Path)
    ap.add_argument('output', type=Path)
    ns=ap.parse_args()
    d=json.loads(ns.canonical.read_text())
    hp=json.loads(ns.independent.read_text())
    by_method={}
    for r in d['order_screen']:
        by_method.setdefault(r['method'], []).append(r)
    method_summary={}
    for method, rows in by_method.items():
        rows=sorted(rows,key=lambda r:-r['h'])
        orders=[r['observed_order_to_next'] for r in rows if r['observed_order_to_next'] is not None]
        method_summary[method]={
            'minimum_observed_order': min(orders),
            'finest_observed_order': orders[-1],
            'finest_error': rows[-1]['endpoint_error'],
            'phi_actions_per_step_at_coarsest_h': rows[0]['phi_actions']/(0.25/rows[0]['h']),
            'jvp_vectors_per_step_at_coarsest_h': rows[0]['jvp_vectors']/(0.25/rows[0]['h']),
        }
    stiff={}
    for r in d['stiff_linear_screen']:
        stiff.setdefault(r['method'], []).append(r['absolute_error'])
    osc={}
    for r in d['oscillatory_screen']:
        osc.setdefault(r['method'], []).append(r)
    analysis={
        'schema':'generic-g2-exponential-foundation-analysis-v1',
        'canonical_sha256':sha(ns.canonical),
        'independent_sha256':sha(ns.independent),
        'summary':d['summary'],
        'method_summary':method_summary,
        'maximum_stiff_linear_error_by_method':{m:max(v) for m,v in stiff.items()},
        'maximum_oscillatory_phase_error_by_method':{m:max(r['phase_error_radians'] for r in rows) for m,rows in osc.items()},
        'maximum_oscillatory_amplitude_error_by_method':{m:max(r['amplitude_error'] for r in rows) for m,rows in osc.items()},
        'high_precision':hp,
        'classification':{
            'coefficient_and_order_authority':'PROMOTE',
            'dense_phi_oracle':'PROMOTE_REFERENCE_ONLY',
            'full_arnoldi_phi_action':'PROMOTE_RESEARCH_REFERENCE',
            'exprb2_exprb43':'PROMOTE_CORRECTNESS_FOUNDATION',
            'pexprb54s4':'PROMOTE_CORRECTNESS_FOUNDATION',
            'adaptive_krylov_kiops':'NOT_IMPLEMENTED',
            'nonautonomous_augmented_state':'NOT_IMPLEMENTED',
            'mass_matrix_support':'NOT_IMPLEMENTED',
            'performance_promotion':'HOLD',
        },
        'next_node':'generic-parallel-exp-adaptive-v1.3',
    }
    ns.output.parent.mkdir(parents=True,exist_ok=True)
    ns.output.write_text(json.dumps(analysis,indent=2,sort_keys=True)+'\n')

if __name__=='__main__': main()
