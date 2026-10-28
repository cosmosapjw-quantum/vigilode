"""Exact quartic signs on declared real envelopes. No native model evaluation."""
from fractions import Fraction as F
from pathlib import Path
from math import comb
import hashlib
import importlib.util
import json

ROOT = Path(__file__).resolve().parents[1]
BINDING = '8cb268e8de741e284cf494013f1e0748d27f03c5e77ba0c64413ae92af082d16'

class Refusal(ValueError):
    def __init__(self, code):
        self.code = code
        super().__init__(code)

def _decimal_integer(value):
    """Exact internal decimal formatting independent of Python's digit guard."""
    if value == 0:
        return '0'
    sign = '-' if value < 0 else ''
    value = abs(value)
    chunks = []
    while value:
        value, remainder = divmod(value, 10**9)
        chunks.append(remainder)
    return sign + str(chunks[-1]) + ''.join(f'{part:09d}' for part in reversed(chunks[:-1]))

def _parse_decimal_integer(value):
    """Decode our trusted internal decimal records without an external bit cap."""
    if isinstance(value, int) and not isinstance(value, bool):
        return value
    if not isinstance(value, str) or not value or value.strip() != value:
        raise Refusal('INTERNAL_MALFORMED_RATIONAL')
    sign = -1 if value.startswith('-') else 1
    digits = value[1:] if value.startswith(('-', '+')) else value
    if not digits or not digits.isascii() or not digits.isdigit():
        raise Refusal('INTERNAL_MALFORMED_RATIONAL')
    result = 0
    for start in range(0, len(digits), 9):
        part = digits[start:start+9]
        result = result * 10**len(part) + int(part)
    return sign * result

def rat(x):
    return {'numerator': _decimal_integer(x.numerator), 'denominator': _decimal_integer(x.denominator)}

def _trusted_exact(value):
    """Private decoder for generated proof records, not external input validation."""
    if isinstance(value, F):
        return value
    if not isinstance(value, dict) or set(value) != {'numerator', 'denominator'}:
        raise Refusal('INTERNAL_MALFORMED_RATIONAL')
    numerator = _parse_decimal_integer(value['numerator'])
    denominator = _parse_decimal_integer(value['denominator'])
    if denominator <= 0:
        raise Refusal('INTERNAL_INVALID_RATIONAL_DENOMINATOR')
    return F(numerator, denominator)

def integer(v):
    if isinstance(v, bool) or not isinstance(v, (int, str)):
        raise Refusal('INVALID_EXACT_INTEGER')
    if isinstance(v, str):
        if len(v) > 2468 or not v or v.strip() != v or not v.lstrip('+-').isdigit():
            raise Refusal('INVALID_EXACT_INTEGER')
        try:
            v = int(v)
        except ValueError:
            raise Refusal('INVALID_EXACT_INTEGER')
    if abs(v).bit_length() > 8192:
        raise Refusal('INPUT_RATIONAL_BIT_LIMIT')
    return v

def exact(v):
    if isinstance(v, bool):
        raise Refusal('INVALID_EXACT_COEFFICIENT')
    if isinstance(v, F):
        n, d = v.numerator, v.denominator
    elif isinstance(v, int):
        n, d = v, 1
    elif isinstance(v, dict) and set(v) == {'numerator', 'denominator'}:
        n, d = integer(v['numerator']), integer(v['denominator'])
    else:
        raise Refusal('INVALID_EXACT_COEFFICIENT')
    if abs(n).bit_length() > 8192 or d.bit_length() > 8192:
        raise Refusal('INPUT_RATIONAL_BIT_LIMIT')
    if d <= 0:
        raise Refusal('INVALID_RATIONAL_DENOMINATOR')
    return F(n, d)

def trim(p):
    p = list(p)
    while len(p) > 1 and not p[-1]:
        p.pop()
    return p or [F(0)]

def zero(p):
    return trim(p) == [0]

def degree(p):
    return len(trim(p)) - 1

def evaluate(p, t):
    y = F(0)
    for a in reversed(p):
        y = y*t + a
    return y

def mul(p, q):
    r = [F(0)] * (len(p) + len(q) - 1)
    for i, a in enumerate(p):
        for j, b in enumerate(q):
            r[i+j] += a*b
    return trim(r)

def derivative(p):
    return trim([i*p[i] for i in range(1, len(p))])

def divrem(p, q):
    p, q = trim(p), trim(q)
    if zero(q):
        raise Refusal('INTERNAL_ZERO_DIVISOR')
    r = list(p)
    ans = [F(0)] * max(1, len(p)-len(q)+1)
    while not zero(r) and len(r) >= len(q):
        k = len(r)-len(q)
        a = r[-1]/q[-1]
        ans[k] += a
        for j, b in enumerate(q):
            r[k+j] -= a*b
        r = trim(r)
    return trim(ans), trim(r)

def quotient(p, q):
    result, rem = divrem(p, q)
    if not zero(rem):
        raise Refusal('INTERNAL_INEXACT_DIVISION')
    return result

def monic(p):
    p = trim(p)
    return [a/p[-1] for a in p] if not zero(p) else p

def gcd(p, q):
    while not zero(q):
        p, q = q, divrem(p, q)[1]
    return monic(p)

def square_free(p):
    """Yun decomposition over Q, keeping the original signed multiplier."""
    p = trim(p)
    c = p[-1]
    f = monic(p)
    repeated = gcd(f, derivative(f))
    w = quotient(f, repeated)
    factors = []
    j = 1
    while degree(w) > 0:
        y = gcd(w, repeated)
        z = quotient(w, y)
        if degree(z) > 0:
            factors.append((monic(z), j))
        w = y
        repeated = quotient(repeated, y)
        j += 1
    reconstructed = [c]
    for f, multiplicity in factors:
        for _ in range(multiplicity):
            reconstructed = mul(reconstructed, f)
    return c, factors, reconstructed == p

def endpoint_remove(p):
    p = trim(p)
    counts = []
    for t in (F(0), F(1)):
        n = 0
        while degree(p) > 0 and evaluate(p, t) == 0:
            p = quotient(p, [-t, F(1)])
            n += 1
        counts.append(n)
    return p, counts

def sturm(p):
    p = trim(p)
    if degree(p) == 0:
        return [p]
    chain = [p, derivative(p)]
    while not zero(chain[-1]):
        rem = divrem(chain[-2], chain[-1])[1]
        if zero(rem):
            break
        neg = [-a for a in rem]
        # Positive scalar normalization only; signs are never flipped.
        scale = abs(neg[-1])
        chain.append([a/scale for a in neg])
    return chain

def variation(chain, t):
    values = [evaluate(p, t) for p in chain]
    signs = [(v > 0)-(v < 0) for v in values if v]
    return sum(a != b for a, b in zip(signs, signs[1:])), values

def root_record(p, a=F(0), b=F(1)):
    if evaluate(p, a) == 0 or evaluate(p, b) == 0:
        raise Refusal('INTERNAL_STURM_ENDPOINT_ROOT')
    chain = sturm(p)
    va, ea = variation(chain, a)
    vb, eb = variation(chain, b)
    return {'polynomial': [rat(x) for x in p], 'interval': [rat(a), rat(b)],
            'chain': [[rat(x) for x in row] for row in chain],
            'values_at_a': [rat(x) for x in ea], 'values_at_b': [rat(x) for x in eb],
            'variations': [va, vb], 'root_count': va-vb,
            'endpoints_nonroots': True, 'normalization': 'STRICTLY_POSITIVE_SCALARS_ONLY'}

def analyze(p):
    p = trim(p)
    if zero(p):
        return {'nonnegative': True, 'positive': False, 'zero_polynomial': True,
                'right_hand_sign': 0, 'endpoint_values': [rat(F(0)), rat(F(0))],
                'factorization': None}
    sign = next((1 if a > 0 else -1) for a in p if a)
    c, factors, reconstructed = square_free(p)
    odd, distinct = [F(1)], [F(1)]
    for f, j in factors:
        distinct = mul(distinct, f)
        if j % 2:
            odd = mul(odd, f)
    odd_interior, odd_end = endpoint_remove(odd)
    all_interior, all_end = endpoint_remove(distinct)
    odd_roots = root_record(odd_interior)
    all_roots = root_record(all_interior)
    ends = [evaluate(p, F(0)), evaluate(p, F(1))]
    _, endpoint_multiplicities = endpoint_remove(p)
    return {'nonnegative': sign > 0 and odd_roots['root_count'] == 0,
            'positive': all(v > 0 for v in ends) and all_roots['root_count'] == 0,
            'zero_polynomial': False, 'right_hand_sign': sign,
            'endpoint_values': [rat(x) for x in ends],
            'endpoint_multiplicities': endpoint_multiplicities,
            'factorization': {'multiplier': rat(c),
                'factors': [{'power': [rat(x) for x in f], 'multiplicity': j} for f, j in factors],
                'reconstruction_equals_input': reconstructed},
            'odd_interior': odd_roots, 'distinct_interior': all_roots,
            'odd_endpoint_factors_removed': odd_end,
            'distinct_endpoint_factors_removed': all_end}

def substitute(p, a, b):
    """p(a+(b-a)u), exact ascending-power coefficients."""
    result = [F(0)] * len(p)
    for i, c in enumerate(p):
        for k in range(i+1):
            result[k] += c*comb(i, k)*a**(i-k)*(b-a)**k
    return trim(result)

def negative_witness(p, limit):
    a, b = F(0), F(1)
    trace = []
    for step in range(limit+1):
        m = (a+b)/2
        for t in (a, m, b):
            v = evaluate(p, t)
            if v < 0:
                return {'kind': 'RATIONAL_NEGATIVE', 'theta': rat(t), 'value': rat(v)}, trace, step
        if step == limit:
            return None, trace, step
        left = analyze(substitute(p, a, m))
        keep_left = not left['nonnegative']
        trace.append({'interval': [rat(a), rat(b)], 'midpoint': rat(m),
                      'retained_half': 'LEFT' if keep_left else 'RIGHT',
                      'left_half_nonnegative': left['nonnegative']})
        if keep_left:
            b = m
        else:
            a = m

def zero_witness(p, analysis, limit):
    for t in (F(0), F(1, 2), F(1)):
        if evaluate(p, t) == 0:
            return {'kind': 'RATIONAL_ZERO', 'theta': rat(t), 'value': rat(F(0))}, [], 0
    q, _ = endpoint_remove(mul_factors(analysis))
    a, b = F(0), F(1)
    trace = []
    for step in range(limit+1):
        rec = root_record(q, a, b)
        if rec['root_count'] == 1:
            quo, rem = divrem(p, q)
            return {'kind': 'ALGEBRAIC_ZERO', 'root_polynomial': [rat(x) for x in q],
                    'isolating_interval': [rat(a), rat(b)], 'root_certificate': rec,
                    'divisibility_quotient': [rat(x) for x in quo],
                    'divisibility_remainder': [rat(x) for x in rem],
                    'p_mod_q_zero': zero(rem), 'exact_real_root': True}, trace, step
        if step == limit:
            return None, trace, step
        m = (a+b)/2
        if evaluate(q, m) == 0:
            return {'kind': 'RATIONAL_ZERO', 'theta': rat(m), 'value': rat(F(0))}, trace, step+1
        left = root_record(q, a, m)
        trace.append({'interval': [rat(a), rat(b)], 'left_count': left['root_count']})
        if left['root_count']:
            b = m
        else:
            a = m

def mul_factors(analysis):
    q = [F(1)]
    for f in analysis['factorization']['factors']:
        q = mul(q, [_trusted_exact(x) for x in f['power']])
    return q

def _scalar_certificate_core(p, *, strict, max_isolation_steps):
    """Trusted Fraction coefficients; external input magnitude guard is upstream."""
    p = trim(p)
    proof = analyze(p)
    passed = proof['positive'] if strict else proof['nonnegative']
    witness, trace, steps = None, [], 0
    if not passed:
        if not proof['nonnegative']:
            witness, trace, steps = negative_witness(p, max_isolation_steps)
        else:
            witness, trace, steps = zero_witness(p, proof, max_isolation_steps)
    status = 'PASS' if passed else 'VIOLATION_WITNESS' if witness is not None else 'UNRESOLVED_WITNESS_AT_LIMIT'
    return {'schema': 'vigilode.exact-scalar-sign.v1', 'status': status,
            'strict': strict, 'power': [rat(x) for x in p], 'degree': degree(p),
            'sign_decision_proven': True, 'sign_condition_holds': passed,
            'certificate_admissible': passed or witness is not None,
            'proof': proof, 'witness': witness, 'witness_search_trace': trace,
            'work': {'isolation_steps': steps}, 'max_isolation_steps': max_isolation_steps,
            'actual_query_failure': False, 'exact_extremum_claim': False,
            'quantified_positive_margin': None}

def scalar_certificate(power, *, strict=False, max_isolation_steps=128):
    try:
        if not isinstance(strict, bool):
            raise Refusal('INVALID_STRICTNESS')
        if isinstance(max_isolation_steps, bool) or not isinstance(max_isolation_steps, int) or not 0 <= max_isolation_steps <= 256:
            raise Refusal('INVALID_MAX_ISOLATION_STEPS')
        if not isinstance(power, (list, tuple)) or not power or len(power) > 1024:
            raise Refusal('INVALID_POLYNOMIAL_SHAPE')
        p = trim([exact(x) for x in power])
        if degree(p) > 4:
            raise Refusal('DEGREE_EXCEEDS_FOUR')
        return _scalar_certificate_core(p, strict=strict, max_isolation_steps=max_isolation_steps)
    except Refusal as exc:
        return {'schema': 'vigilode.exact-scalar-sign-refusal.v1', 'status': 'REFUSED',
                'refusal': exc.code, 'certificate_admissible': False,
                'actual_query_failure': False}

def load_dependencies():
    manifest = json.loads((ROOT/'inputs/REUSED_ROWWISE_IDENTITY.json').read_text())
    for r in manifest['files']:
        if hashlib.sha256((ROOT/r['path']).read_bytes()).hexdigest() != r['sha256']:
            raise Refusal('REUSED_SOURCE_BYTE_IDENTITY_MISMATCH:'+r['path'])
    spec = importlib.util.spec_from_file_location('preserved_rowwise', ROOT/'inputs/reused_rowwise/code/rowwise_bernstein.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    dense, domain, _ = module.load_dependencies()
    return module, dense, domain, manifest

def flags():
    return {'query_rounding_bound': None, 'actual_owner_segment': None,
            'actual_BASS_segment_supplied': False, 'whole_domain_certificate': False,
            'VerifiedModelProvider': None, 'actual_ode_solution_enclosure': False,
            'production': 'HOLD', 'globalphysicalcompletion': None, 'L4': False,
            'canonical': '28/88_UNCHANGED', 'completion_delta': 0,
            'native_unknown_budgets': 'UNKNOWN_PRESERVED', 'actual_query_failure': False,
            'physical_calls': {'rhs': 0, 'jacobian': 0, 'ode': 0, 'ncp': 0, 'native_norm': 0}}

def input_sha(inp):
    try:
        return hashlib.sha256(json.dumps(inp, sort_keys=True, separators=(',', ':'), allow_nan=False).encode()).hexdigest()
    except (ValueError, TypeError):
        return None

def _certificate(inp):
    if not isinstance(inp, dict):
        raise Refusal('MALFORMED_INPUT')
    canonical_input_sha = input_sha(inp)
    if canonical_input_sha is None:
        raise Refusal('INPUT_NOT_JSON_SERIALIZABLE')
    if inp.get('binding_id') != BINDING:
        raise Refusal('BINDING_MISMATCH')
    if inp.get('synthetic_fixture') is not True or 'actual_owner_segment' not in inp or inp['actual_owner_segment'] is not None:
        raise Refusal('SYNTHETIC_PROVENANCE_REQUIRED')
    limit = inp.get('max_isolation_steps', 128)
    if isinstance(limit, bool) or not isinstance(limit, int) or not 0 <= limit <= 256:
        raise Refusal('INVALID_MAX_ISOLATION_STEPS')
    old, dense, domain, manifest = load_dependencies()
    _, q, n = dense.parse(inp)
    if n != 30:
        raise Refusal('STATE_DIMENSION_MUST_BE_30')
    binding = domain.source_binding()
    inherited, guards = [], []
    for i, values in enumerate(zip(*(q[k] for k in ('y0', 'y1', 'd0', 'd1', 'd2')))):
        bound = dense.coordinate_bound(*values, q['delta'])
        if any(v > dense.FMAX for _, v in bound['guards']):
            raise Refusal('INHERITED_SUFFICIENT_RANGE_BOUND_UNAVAILABLE:'+str(i))
        inherited.append(bound['rho'])
        guards.append({'coordinate': i, 'rho': rat(bound['rho']),
                       'guards': [{'name': name, 'magnitude': rat(v), 'passed': v <= dense.FMAX} for name, v in bound['guards']]})
    if 'rho_allowance' in inp:
        v = inp['rho_allowance']
        if not isinstance(v, list) or len(v) != 30:
            raise Refusal('INVALID_RHO_ALLOWANCE_SHAPE')
        state_rho = [exact(x) for x in v]
        if any(x < 0 for x in state_rho):
            raise Refusal('NEGATIVE_RHO_ALLOWANCE')
        if any(a < b for a, b in zip(state_rho, inherited)):
            raise Refusal('RHO_ALLOWANCE_BELOW_INHERITED_BOUND')
    else:
        state_rho = inherited
    rho = [F(0)] + state_rho
    powers = [[q['x0'], q['delta'], F(0), F(0), F(0)]] + old.state_power(q)
    def row(name, c, terms, strict, category):
        H, p, sigma, *_ = old.row_data(c, terms, q, rho)
        sc = _scalar_certificate_core(p, strict=strict, max_isolation_steps=limit)
        result = {'name': name, 'category': category, 'strict': strict,
                  'constant': rat(c), 'terms': {str(i): rat(F(v)) for i, v in terms.items()},
                  'power_H': [rat(x) for x in H], 'power_H_minus_sigma': [rat(x) for x in p],
                  'sigma': rat(sigma), 'certificate': sc}
        w = sc.get('witness')
        if w:
            offset = [-((1 if terms.get(i, 0) > 0 else -1) if terms.get(i, 0) else 0)*rho[i] for i in range(31)]
            corner_ok = all(abs(e) <= r for e, r in zip(offset, rho))
            point_power = [list(v) for v in powers]
            for v, e in zip(point_power, offset):
                v[0] += e
            composed = [sum((F(coef)*point_power[i][k] for i, coef in terms.items()), F(0)) for k in range(5)]
            composed[0] += c
            dw = {'kind': w['kind'], 'error_offset': [rat(x) for x in offset],
                  'error_box_membership_checked': corner_ok,
                  'row_equals_polynomial_minus_support': trim(composed) == trim(p),
                  'scope': 'DECLARED_REAL_POLYNOMIAL_PLUS_INDEPENDENT_ERROR_BOX_ONLY',
                  'actual_query_failure': False}
            if 'theta' in w:
                t = _trusted_exact(w['theta'])
                basepoint = [evaluate(v, t) for v in powers]
                point = [a+b for a, b in zip(basepoint, offset)]
                value = c + sum((F(coef)*point[i] for i, coef in terms.items()), F(0))
                dw.update(theta=rat(t), base_point=[rat(x) for x in basepoint],
                          point=[rat(x) for x in point], exact_row_value=rat(value),
                          row_equals_polynomial_minus_support=value == evaluate(p, t))
            else:
                divisor = [_trusted_exact(x) for x in w['root_polynomial']]
                dw.update(basepoint_power=[[rat(x) for x in v] for v in powers],
                          point_power=[[rat(x) for x in v] for v in point_power],
                          root=w, row_polynomial_remainder=[rat(x) for x in divrem(composed, divisor)[1]],
                          row_mod_q_zero=zero(divrem(composed, divisor)[1]),
                          binary64_point=None)
            result['declared_envelope_witness'] = dw
        return result
    physical = [row(name, c, terms, strict, 'PHYSICAL_AFFINE') for name, c, terms, strict in domain.rows(binding)]
    finite = [row('finite_'+kind+'_w'+str(i), dense.FMAX, {i: s}, False, 'FINITE_RANGE')
              for i in range(31) for kind, s in (('plus', 1), ('minus', -1))]
    def group(rows):
        states = [r['certificate']['status'] for r in rows]
        if 'VIOLATION_WITNESS' in states:
            return 'DECLARED_ENVELOPE_COUNTEREXAMPLE'
        return 'PASS' if all(s == 'PASS' for s in states) else 'UNRESOLVED'
    ps, fs = group(physical), group(finite)
    status = 'DECLARED_ENVELOPE_COUNTEREXAMPLE' if 'DECLARED_ENVELOPE_COUNTEREXAMPLE' in (ps, fs) else 'PASS' if ps == fs == 'PASS' else 'UNRESOLVED'
    return {'schema': 'vigilode.exact-row-sign.v1', 'unit': 'V8_EXACT_ROW_SIGN_20261027',
            'status': status, 'input_sha256': canonical_input_sha, 'source_dependencies': manifest,
            'source_binding': binding, 'synthetic_fixture': True,
            'rho_inherited': [rat(x) for x in inherited], 'rho_used': [rat(x) for x in rho],
            'rho_clock': rat(F(0)), 'rho_allowance_domination': all(a >= b for a, b in zip(state_rho, inherited)),
            'inherited_source_range_guards': guards,
            'physical19': {'count': len(physical), 'status': ps, 'rows': physical},
            'finite62': {'count': len(finite), 'status': fs, 'rows': finite},
            'source_family_uniform_declared_envelope_domain_inclusion': status == 'PASS',
            'exact_extremum_claim': False, 'quantified_positive_margin': None,
            'remaining_model_obligations': ['actual accepted native capsule', 'existing physical norm/lift',
                'residual upper bound eta', 'corridor lognorm mu including clock derivative',
                'first-exit/self-consistent radius', 'inherited/initial/source error', 'continuous-angle positivity'],
            **flags()}

def certificate(inp):
    try:
        return _certificate(inp)
    except ValueError as exc:
        if not hasattr(exc, 'code'):
            raise
        return {'schema': 'vigilode.exact-row-sign-refusal.v1', 'status': 'REFUSED',
                'refusal': exc.code, 'input_sha256': input_sha(inp), **flags()}
