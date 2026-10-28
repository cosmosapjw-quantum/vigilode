#!/usr/bin/env python3
"""Exact Fraction replay of frozen scalar native JSON. No float arithmetic in oracle."""
import argparse
import json
import math
import struct
from fractions import Fraction as F


def exact(bits):
    value = struct.unpack('>d', bytes.fromhex(bits))[0]
    return F(*value.as_integer_ratio())


def beta(j, k):
    return F(math.factorial(j) * math.factorial(k), math.factorial(j+k+1))


def endpoint(a, t, r, d, degree):
    if t == 0:
        return F(0), F(0)
    if a == 0:
        x = t**(r+d+1) * beta(d, r)
        return x, x
    if a*t > 1024:
        assert r == 0 and d == 0, 'inverse-positive branch is frozen to S05 Q00'
        # exp(-x) <= reciprocal of any positive exp(x) Taylor partial sum.
        term = total = F(1)
        for n in range(1, degree+1):
            term *= a*t/n
            total += term
        return (1-1/total)/a, 1/a
    total = F(0)
    term = t**(r+d+1) * beta(d, r)
    for n in range(degree):
        total += term
        # Exact ratio of successive integrated Taylor terms.
        term *= -a*t*F(d+n+1, (n+1)*(d+r+n+2))
    # Lagrange remainder for exp(-a*s), s>=0, integrated against
    # nonnegative polynomial weight: abs error <= omitted absolute term.
    rem = abs(term)
    return max(F(0), total-rem), total+rem


def run(native):
    outcomes = []
    for fixture in native['fixtures']:
        result = fixture['result']
        if result['status'] == 'REFUSED':
            outcomes.append({'id':fixture['id'], 'verdict':'TYPED_REFUSAL', 'refusal':result['refusal']})
            continue
        k = exact(fixture['kappa_bits'])
        h = exact(fixture['horizon_bits'])
        a = 2*(fixture['q']+1)*k*h
        tlo, thi = map(exact, fixture['time_bits'])
        lo, hi = exact(result['lo_bits']), exact(result['hi_bits'])
        assert hi-lo <= exact(fixture['max_width_bits']), 'native width exceeds fixed gate'
        rate_lo, rate_hi = map(exact, fixture['rate_bits'])
        assert rate_lo <= a <= rate_hi, 'exact original product excluded by native binding'
        verdict = 'INCONCLUSIVE'
        for degree in range(128, 2049, 64):
            lower = endpoint(a, tlo, fixture['r'], fixture['d'], degree)[0]
            upper = endpoint(a, thi, fixture['r'], fixture['d'], degree)[1]
            # Compare the full exact endpoint oracle hull, including horizon uncertainty.
            if lo <= lower and upper <= hi:
                verdict = 'PASS'
                break
            if hi < lower or upper < lo:
                verdict = 'FALSIFIED'
                break
        outcomes.append({'id':fixture['id'], 'verdict':verdict, 'degree':degree,
                         'rate_binding':'EXACT_INPUT_PRODUCT_INCLUDED'})
    return {'node':'A03-SCALAR-RATE-01', 'method':'EXACT_FRACTION',
            'outcomes':outcomes, 'numerical_verdict': 'PASS_SCOPED' if all(x['verdict'] in ('PASS','TYPED_REFUSAL') for x in outcomes) else 'HOLD',
            'claim_ceiling':'SCALAR_FORMULA_ONLY', 'packet':'HOLD', 'production':'HOLD'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('native')
    parser.add_argument('output')
    args = parser.parse_args()
    with open(args.native, encoding='utf-8') as source:
        result = run(json.load(source))
    with open(args.output, 'w', encoding='utf-8') as target:
        json.dump(result, target, indent=2)
        target.write('\n')
    print(json.dumps(result))
    raise SystemExit(0 if result['numerical_verdict']=='PASS_SCOPED' else 1)
