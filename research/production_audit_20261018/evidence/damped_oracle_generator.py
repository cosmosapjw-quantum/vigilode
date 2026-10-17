"""Regenerate independent 240-digit fixture brackets for new damped-moment tests.

This is a high-precision diagnostic oracle, not the interval proof of T3.
Decimal.from_float preserves exact binary64 inputs. Terms use the integrated
exponential Taylor identity, independent of native backward recurrence.
"""
from decimal import Decimal, localcontext
import json
import math
import struct

FIXTURES = [(0.1,10.,0,0),(.1,.3,2,1),(1.000001,1.,3,2),(10.,1.,0,0),(25.,.4,4,3),(.8,.3,6,2),(0.,.5,2,0),(0.,.5,2,3)]


def fixtures():
    result = []
    for kappa, horizon, j, k in FIXTURES:
        with localcontext() as context:
            context.prec = 240
            z = Decimal.from_float(kappa) * Decimal.from_float(horizon)
            term = Decimal(math.factorial(j)) * Decimal(math.factorial(k)) / Decimal(math.factorial(j+k+1))
            value = term
            for n in range(1, 1000):
                term *= -z * Decimal(j+n) / (Decimal(n) * Decimal(j+k+n+1))
                value += term
                if abs(term) < Decimal('1e-210'):
                    break
            else:
                raise RuntimeError('series budget exhausted')
            nearest = float(value)
            lo = nearest if Decimal(nearest) <= value else math.nextafter(nearest, -math.inf)
            hi = nearest if Decimal(nearest) >= value else math.nextafter(nearest, math.inf)
            bits = lambda x: f'{struct.unpack(">Q", struct.pack(">d", x))[0]:016x}'
            result.append(dict(kappa=kappa, horizon=horizon, j=j, k=k, lo_bits=bits(lo), hi_bits=bits(hi), decimal_value=str(value), terms=n))
    return result

if __name__ == '__main__':
    print(json.dumps({'status':'HIGH_PRECISION_DIAGNOSTIC', 'precision_digits':240, 'fixtures':fixtures()}, indent=2))
