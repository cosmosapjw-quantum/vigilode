"""V1 preparation only: exact physical-error oracle and receipt-schema checks.

This is neither a native RODAS hook nor an actual BASS nonlinear-client test.
The receipt checker validates necessary supplied claims, not their authenticity;
its caller must bind every claim to immutable source and execution evidence.
"""
import math
import re
from fractions import Fraction
from collections.abc import Mapping

def norm_squared(error, scale):
    """Exact squared full33 WRMS of finite binary64 input values.

    Dependent species rows are 23, 25, 28 in the PR79 fixed compact ordering.
    Scale is supplied in physical coordinates. No offset belongs to an error.
    This allocating Fraction oracle is not the production hot-path algorithm.
    """
    if len(error) != 30 or len(scale) != 33:
        raise ValueError('expected error30 and full physical scale33')
    e = [float(x) for x in error]
    s = [float(x) for x in scale]
    if not all(math.isfinite(x) for x in e):
        raise ValueError('nonfinite error')
    if not all(math.isfinite(x) and x > 0 for x in s):
        raise ValueError('scales must be finite and strictly positive')
    e = [Fraction.from_float(x) for x in e]
    s = [Fraction.from_float(x) for x in s]
    total = Fraction(0)
    j = 0
    for i in range(33):
        if i in (23, 25, 28):
            length = {23: 1, 25: 2, 28: 3}[i]
            lifted = -sum(e[j:j+length], Fraction(0))
        else:
            lifted = e[j]
            j += 1
        total += (lifted / s[i]) ** 2
    assert j == 30
    return total / 33

def native_gate(receipt, observed_source):
    """Return unsatisfied necessary receipt fields, never infer missing PASS.

    An empty list is schema sufficiency only, not native execution verification.
    Synthetic fixtures in tests are NOT scientific receipts.
    """
    if not isinstance(receipt, Mapping):
        return ['receipt']
    expected = {
        'driver': 'native-rodas5p',
        'reduced_dimension': 30, 'physical_dimension': 33, 'denominator': 33,
        'scale_owner': 'client-full-physical', 'error_lift': 'L*error30-no-offset',
        'default_norm_digest_equal': True, 'correlated_oracle_pass': True,
        'mixed_exponent_contract_pass': True, 'typed_refusal_no_commit': True,
        'hot_path_allocations': 0, 'owner_clearance_current': True,
        'local_changes_preserved': True, 'source_bound_native_execution': True,
    }
    failed = [key for key, value in expected.items()
              if type(receipt.get(key)) is not type(value) or receipt[key] != value]
    source = receipt.get('source')
    if (not isinstance(observed_source, str)
        or re.fullmatch(r'[0-9a-f]{40}', observed_source) is None
        or source != observed_source):
        failed.append('source')
    return sorted(failed)
