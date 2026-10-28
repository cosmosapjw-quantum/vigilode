#!/usr/bin/env python3
"""A04 frozen Fraction oracle. No binary64 arithmetic occurs after decoding bits.

The native export and frozen manifest are inputs, never adjusted by refinement.
Output comparison certificates record exact signs and hashes of rational squared
quantities; all operands can be reconstructed from the bound inputs and script.
"""
import argparse
from fractions import Fraction as Q
import hashlib
import json
import math
from pathlib import Path
import struct
import sys

if hasattr(sys, 'set_int_max_str_digits'):
    sys.set_int_max_str_digits(0)
ORDERS = (64, 128, 256, 512, 1024)


def q(bits):
    value = struct.unpack('>d', int(bits, 16).to_bytes(8, 'big'))[0]
    if not math.isfinite(value):
        raise ValueError('nonfinite binary64 has no rational image')
    return Q.from_float(value)


def digest_q(value):
    # Hex integer encoding avoids decimal size limits and is deterministic.
    raw = f'{value.numerator:x}/{value.denominator:x}'.encode()
    return {'sha256_hex_rational': hashlib.sha256(raw).hexdigest(),
            'numerator_bits': value.numerator.bit_length(),
            'denominator_bits': value.denominator.bit_length()}


def triage(center, bound, boxes):
    assert len(center) == len(boxes) and len(center) > 0
    if bound < 0:
        return {'status': 'INVALID_REPORT'}
    upper = sum((max(abs(lo-y), abs(hi-y))**2
                 for y, (lo, hi) in zip(center, boxes)), Q(0))
    lower = sum((max(Q(0), lo-y, y-hi)**2
                 for y, (lo, hi) in zip(center, boxes)), Q(0))
    square = bound**2
    status = 'PASS' if upper <= square else ('FALSIFIED' if lower > square else 'INCONCLUSIVE')
    return {'status': status, 'U_le_bound_squared': upper <= square,
            'L_gt_bound_squared': lower > square,
            'upper_squared_distance': digest_q(upper),
            'lower_squared_distance': digest_q(lower), 'bound_squared': digest_q(square)}


def coefficient_boxes(table, order):
    c, gamma = q(table['c_bits']), q(table['gamma_bits'])
    nodes = [c + gamma*q(x) for x in table['xi_bits']]
    assert len(table['mid_bits']) == len(nodes) == len(table['radius_bits'])
    assert all(len(a) == 5 for a in table['mid_bits']+table['radius_bits'])
    results = []
    for j in range(len(nodes)):
        R = max(map(abs, nodes[:j+1]))
        if not order+2 > R:
            return None
        # h_r(x0,...,xj), using generating product prod_i (1-x_i z)^-1.
        homogeneous = [Q(1)] + [Q(0)]*order
        for x in nodes[:j+1]:
            for r in range(1, order+1):
                homogeneous[r] += x*homogeneous[r-1]
        for k in range(5):
            mid = gamma**j * sum((homogeneous[r]/math.factorial(r+j+k)
                                  for r in range(order+1)), Q(0))
            tail = (abs(gamma)**j * math.comb(order+1+j, j) * R**(order+1)
                    /math.factorial(order+1+j+k)/(1-R/Q(order+2)))
            result = triage([q(table['mid_bits'][j][k])], q(table['radius_bits'][j][k]), [(mid-tail,mid+tail)])
            result.update({'j': j, 'k': k})
            results.append(result)
    return results


def action_boxes(row, shared, order):
    f = row['fixture']
    h = q(f['h_bits'])
    a = [[h*q(x) for x in r] for r in shared[f['operator_bits_ref']]]
    n = len(a)
    B = max(sum(map(abs, r), Q(0)) for r in a)
    inp = f['input']
    if inp['kind'] == 'SameVector':
        v = [q(x) for x in shared[inp['vector_bits_ref']]]
        vectors = [v]*5
        scales = [q(x) for x in shared[inp['scales_bits_ref']]]
    else:
        vectors = [[q(x) for x in w] for w in shared[inp['vectors_bits_ref']]]
        scales = [q(x) for x in inp['effective_scales_bits']]
    center = [Q(0)]*n
    tail = Q(0)
    active = []
    for k, (scale, w) in enumerate(zip(scales, vectors)):
        W = max(map(abs, w))
        if abs(scale)*W == 0:
            continue
        active.append(k)
        if not order+2+k > B:
            return None
        power = w[:]
        for degree in range(order+1):
            for i in range(n):
                center[i] += scale*power[i]/math.factorial(degree+k)
            if degree != order:
                power = [sum((a[i][j]*power[j] for j in range(n)), Q(0)) for i in range(n)]
        if B != 0:
            tail += abs(scale)*W*B**(order+1)/math.factorial(order+1+k)/(1-B/Q(order+2+k))
    result = triage([q(x) for x in row['fused_bits']], q(row['total_bound_bits']),
                    [(c-tail,c+tail) for c in center])
    result.update({'active_k': active, 'domain_asserted_each_active_k': True,
                   'B': digest_q(B), 'tail': digest_q(tail)})
    return result


def refine(operation):
    attempts=[]
    for order in ORDERS:
        try:
            result=operation(order)
        except ValueError as error:
            return {'status':'INVALID_REPORT', 'error':str(error), 'attempts':attempts}
        if result is None:
            status='ORACLE_DOMAIN_UNMET'
        elif isinstance(result,list):
            statuses={r['status'] for r in result}
            status=next((s for s in ['INVALID_REPORT','FALSIFIED','INCONCLUSIVE'] if s in statuses),'PASS')
        else:
            status=result['status']
        attempts.append({'order':order,'status':status,'comparisons':result})
        if status not in ('INCONCLUSIVE','ORACLE_DOMAIN_UNMET'):
            return {'status':status,'attempts':attempts}
    return {'status':'HOLD','attempts':attempts}


def main():
    p=argparse.ArgumentParser()
    p.add_argument('native',type=Path)
    p.add_argument('--manifest',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True)
    args=p.parse_args()
    native=json.loads(args.native.read_text())
    manifest=json.loads(args.manifest.read_text())
    contract=manifest['next_action']['frozen_oracle_contract']
    assert [contract['fraction_orders']['initial']]+contract['fraction_orders']['refinement_schedule']==list(ORDERS)
    assert [r['id'] for r in native['actions']]==[r['id'] for r in contract['action_fixtures']]
    assert [r['id'] for r in native['coefficients']]==[r['id'] for r in contract['coefficient_fixtures']]
    rows=[]
    # Stop native mismatches before attempting a numerical admission.
    if native['contract_failures']:
        result={'status':'CONTRACT_FAILURE','native_failures':native['contract_failures']}
    else:
        for row, frozen in zip(native['actions'],contract['action_fixtures']):
            assert row['fixture']==frozen
            if row['return']=='Err':
                rows.append({'id':row['id'],'status':'TYPED_REFUSAL_OBSERVED',
                             'error_family':row['error_family'],
                             'last_report_preserved':row['last_report_preserved'],
                             'f09_coverage_caveat':row['f09_coverage_caveat']})
                continue
            fused=refine(lambda m:action_boxes(row,contract['shared_input_bit_arrays'],m))
            coefficients=refine(lambda m:coefficient_boxes(row['derived_coefficient_table'],m))
            rows.append({'id':row['id'],'fused':fused,'coefficients':coefficients})
            if 'FALSIFIED' in (fused['status'],coefficients['status']):
                break
        if not any('FALSIFIED' in (r.get('fused',{}).get('status'),r.get('coefficients',{}).get('status')) for r in rows):
            for row,frozen in zip(native['coefficients'],contract['coefficient_fixtures']):
                assert row['fixture']==frozen
                assert all(row['table'][k] == frozen[k] for k in ('c_bits','gamma_bits','xi_bits'))
                r=refine(lambda m:coefficient_boxes(row['table'],m))
                rows.append({'id':row['id'],'coefficients':r})
                if r['status']=='FALSIFIED':
                    break
        statuses=[s for r in rows for key in ('fused','coefficients') if (s:=r.get(key,{}).get('status'))]
        status=next((s for s in ['INVALID_REPORT','FALSIFIED','HOLD'] if s in statuses),'PASS_SCOPED_FIXTURES')
        result={'status':status,'rows':rows,'numeric_comparisons':sum(len(a['comparisons']) if isinstance(a['comparisons'],list) else 1 for r in rows for key in ('fused','coefficients') for a in r.get(key,{}).get('attempts',[]) if a['comparisons'] is not None),
                'authority':'EstimateOnly','full_A04_decision':'HOLD',
                'allocation_positive_control':native['allocation_positive_control'],
                'allocation_verdict': ('ALLOCATION_UNAVAILABLE' if native['allocation_positive_control'] < 3 else ('ZERO_ALLOCATION_PASS_SCOPED' if all(r.get('allocation_status') == 'ZERO_ALLOCATION_PASS_SCOPED' for r in native['actions'] if r['id'] in contract['allocator']['warmed_fixture_ids']) else 'ALLOCATION_NONZERO')),
                'allocation_rows':[{'id':r['id'],'status':r['allocation_status'],'allocations':r['warmed_allocations']} for r in native['actions'] if 'allocation_status' in r]}
    result['inputs_sha256']={str(path):hashlib.sha256(path.read_bytes()).hexdigest() for path in (args.native,args.manifest,Path(__file__))}
    args.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({k:v for k,v in result.items() if k not in ('rows','inputs_sha256')},indent=2))
    return 0 if result['status']=='PASS_SCOPED_FIXTURES' else 2


if __name__=='__main__':
    raise SystemExit(main())
