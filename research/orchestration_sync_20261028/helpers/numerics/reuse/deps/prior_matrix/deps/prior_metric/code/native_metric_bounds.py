"""Exact polynomial-family and nonnegative-box certificates in a fixed metric.

This module has no point-norm/lift evaluator. All physical row polynomial and
box support algebra is inherited. Generated proof status refers to this finite
algebra only; owner provenance and external model premises remain unresolved.
"""
from fractions import Fraction as F
from functools import lru_cache
from pathlib import Path
import hashlib
import importlib.util
import json
from math import isqrt

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_DEPTH, MAX_DEPTH = 2, 6
DEFAULT_PRECISION, MAX_PRECISION = 40, 128
MAX_CELLS, MAX_COEFFICIENTS = 16, 21120


def require(ok, code):
    if not ok:
        raise ValueError(code)


@lru_cache(maxsize=1)
def piecewise():
    path = ROOT / 'code/reused_piecewise_barrier.py'
    spec = importlib.util.spec_from_file_location('native_bounds_reused_piecewise', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def source():
    # This inherited loader checks every byte-identical dependency before import.
    return piecewise().source()


def exact(value):
    return source().rational(value)


def precision_check(precision):
    require(isinstance(precision, int) and not isinstance(precision, bool)
            and 1 <= precision <= MAX_PRECISION, 'PRECISION_OUTSIDE_FROZEN_CAP')


def resource_plan(depth, cells=1):
    require(isinstance(depth, int) and not isinstance(depth, bool)
            and 0 <= depth <= MAX_DEPTH, 'DEPTH_OUTSIDE_FROZEN_CAP')
    require(isinstance(cells, int) and not isinstance(cells, bool)
            and 1 <= cells <= MAX_CELLS, 'PARTITION_CELL_CAP_EXCEEDED')
    count = 2 * 33 * 5 * (1 << depth)
    require(count <= MAX_COEFFICIENTS, 'BERNSTEIN_COEFFICIENT_CAP_EXCEEDED')
    return {'cells': cells, 'depth': depth, 'leaves_per_cell': 1 << depth,
            'row_bernstein_coefficients_per_cell': count,
            'row_bernstein_coefficients_total': cells * count,
            'preallocation_checked': True}


def _power2(exponent):
    return F(1 << exponent) if exponent >= 0 else F(1, 1 << -exponent)


def scalar_sqrt_enclosure(value, *, precision=DEFAULT_PRECISION):
    """Scale-aware scalar rational sqrt enclosure, never a vector metric API."""
    precision_check(precision)
    q = exact(value)
    require(q >= 0, 'NONNEGATIVE_SCALAR_REQUIRED')
    if not q:
        return {'Q': q, 'lower': F(0), 'upper': F(0), 'exact': True,
                'precision': precision, 'scale_exponent': None}
    rn, rd = isqrt(q.numerator), isqrt(q.denominator)
    if rn * rn == q.numerator and rd * rd == q.denominator:
        root = F(rn, rd)
        return {'Q': q, 'lower': root, 'upper': root, 'exact': True,
                'precision': precision, 'scale_exponent': None}
    e = (q.numerator.bit_length() - q.denominator.bit_length()) // 2
    while q < _power2(2 * e):
        e -= 1
    while q >= _power2(2 * (e + 1)):
        e += 1
    z = q / _power2(2 * e)
    scaled = z * (1 << (2 * precision))
    floor_root = isqrt(scaled.numerator // scaled.denominator)
    step = _power2(e - precision)
    lower = floor_root * step
    upper = (floor_root + 1) * step
    require(lower * lower <= q <= upper * upper, 'SCALAR_ROOT_ENCLOSURE_FAILED')
    return {'Q': q, 'lower': lower, 'upper': upper, 'exact': False,
            'precision': precision, 'scale_exponent': e,
            'relative_error_strictly_below': F(1, 1 << precision)}


@lru_cache(maxsize=1)
def _row_terms():
    c = source()
    C, E, _, _, _ = c.dependencies()
    _, _, binding = C.dependencies()
    matrix = binding['row_major_lift']
    require(len(matrix) == 33 * 30, 'FIXED_L_DIMENSION_MISMATCH')
    rows = []
    for j in range(33):
        terms = {i + 1: F.from_float(v) if isinstance(v, float) else F(v)
                 for i, v in enumerate(matrix[j * 30:(j + 1) * 30]) if v}
        signs = {1 if v > 0 else -1 for v in terms.values()}
        require(len(signs) == 1, 'FIXED_L_ROW_SAME_SIGN_REQUIRED')
        rows.append(terms)
    return tuple(rows)


def _metric(u33):
    C, E, _, _, _ = source().dependencies()
    u = C.vector(u33, 33, E, 'SCALES')
    require(all(v > 0 for v in u), 'PHYSICAL_UPPER_SCALES_MUST_BE_POSITIVE')
    return u, C.fixed_norm_id(u, E)


def _capsule_identity(capsule):
    return hashlib.sha256(json.dumps(capsule, sort_keys=True,
                          separators=(',', ':'), allow_nan=False).encode()).hexdigest()


def _identity(capsule, parsed, metric, left, right):
    c = source()
    return {'bass_commit': c.BASS, 'binding_id': c.BINDING,
            'source_sha256': c.SOURCE, 'clock': dict(c.CLOCK),
            'fixed_norm_identity': metric, 'capsule_identity': _capsule_identity(capsule),
            'left': left, 'right': right,
            'provenance_review': parsed['provenance_review']}


def _ceilings():
    return {**source().ceiling(), 'query_rounding_bound': None, 'V7fullrestore': 'NOT_RUN'}


def _record(name, value, identity, evidence, **extra):
    return {**identity, 'name': name, 'scope': source().SCOPES[name],
            'value': value, 'proof_status': 'VERIFIED', 'evidence': evidence,
            'verification_scope': 'EXACT_FINITE_ALGEBRA_ONLY',
            'owner_provenance_verified': False, **extra}


def cell_derivative_bounds(capsule, physical_left, physical_right, u33,
                           *, depth=DEFAULT_DEPTH, precision=DEFAULT_PRECISION):
    plan = resource_plan(depth)
    precision_check(precision)
    c = source()
    C, E, row, _, _ = c.dependencies()
    parsed = c.parse_capsule(capsule)
    q = parsed['q']
    left, right = exact(physical_left), exact(physical_right)
    require(q['x0'] <= left < right <= q['x1'], 'PHYSICAL_CELL_OUTSIDE_CAPSULE')
    u, metric = _metric(u33)
    identity = _identity(capsule, parsed, metric, left, right)
    theta_a, theta_b = (left - q['x0']) / q['delta'], (right - q['x0']) / q['delta']
    roots = {1: [], 2: []}
    powers = {1: [], 2: []}
    for terms in _row_terms():
        H = row.row_data(0, terms, q, [F(0)] * 31)[0]
        derivative = H
        for order in (1, 2):
            derivative = E.derivative(derivative)
            physical = [v / q['delta'] ** order for v in derivative]
            restricted = E.substitute(physical, theta_a, theta_b)
            powers[order].append(restricted)
            roots[order].append(row.power_to_bernstein(restricted + [F(0)] * (5 - len(restricted))))
    # All rows and orders split together. The leaf is the aggregation unit.
    leaves = [(F(0), F(1), roots)]
    for _ in range(depth):
        children = []
        for lo, hi, data in leaves:
            halves = {k: [row.split_midpoint(b) for b in data[k]] for k in (1, 2)}
            mid = (lo + hi) / 2
            children.extend((a, b, {k: [v[side] for v in halves[k]] for k in (1, 2)})
                            for side, (a, b) in enumerate(((lo, mid), (mid, hi))))
        leaves = children
    evidence = []
    for lo, hi, data in leaves:
        entry = {'local_interval': [lo, hi],
                 'physical_interval': [left + (right-left)*lo, left + (right-left)*hi],
                 'orders': {}}
        for order in (1, 2):
            maxima = [max(abs(v) for v in b) for b in data[order]]
            Q = sum(((v / scale) ** 2 for v, scale in zip(maxima, u)), F(0)) / 33
            entry['orders'][str(order)] = {'bernstein33': data[order], 'M33': maxima, 'Q': Q}
        evidence.append(entry)
    result = {'schema': 'vigilode.native-metric-derivative-bounds.v1',
              'status': 'PASS_EXACT_POLYNOMIAL_FAMILY_BOUND', **identity,
              'Delta': q['delta'], 'native_h': q['h'], 'upper_scales': u,
              'theta_interval': [theta_a, theta_b], 'depth': depth, 'resource_plan': plan,
              'restricted_physical_derivative_power33': powers,
              'common_leaf_evidence': evidence, **_ceilings()}
    for order in (1, 2):
        key = 'B' + str(order)
        Q = max(leaf['orders'][str(order)]['Q'] for leaf in evidence)
        root = scalar_sqrt_enclosure(Q, precision=precision)
        result[key] = _record(key, root['upper'], identity,
                      'common dyadic leaf Bernstein convex-hull proof; max of per-leaf weighted square sums',
                      Q=Q, sqrt=root, depth=depth)
    return result


def box_envelope_certificate(nonnegative30rho, u33, *, precision=DEFAULT_PRECISION):
    precision_check(precision)
    c = source()
    C, E, row, _, _ = c.dependencies()
    rho = C.vector(nonnegative30rho, 30, E, 'NONNEGATIVE_BOX_RADII')
    require(all(v >= 0 for v in rho), 'NONNEGATIVE_BOX_RADII_REQUIRED')
    u, metric = _metric(u33)
    # q is only row_data's unused zero polynomial context; rho_x is exactly zero.
    zero = [F(0)] * 30
    q = {'x0': F(0), 'delta': F(1), **{k: zero for k in ('y0','y1','d0','d1','d2')}}
    sigma = [row.row_data(0, terms, q, [F(0)] + rho)[2] for terms in _row_terms()]
    Q = sum(((v / scale) ** 2 for v, scale in zip(sigma, u)), F(0)) / 33
    root = scalar_sqrt_enclosure(Q, precision=precision)
    return {'schema': 'vigilode.nonnegative-box-envelope.v1',
            'status': 'PASS_EXACT_INDEPENDENT_BOX_SUPREMUM',
            'bass_commit': c.BASS, 'binding_id': c.BINDING,
            'fixed_norm_identity': metric, 'upper_scales': u, 'rho30': rho,
            'rho_clock': F(0), 'row_support33': sigma, 'row_same_sign_checked': True,
            'Q': Q, 'sqrt': root, 'upper': root['upper'],
            'sharpness': 'all-positive-radius corner simultaneously attains every absolute row support',
            'actual_floating_query_witness': False, **_ceilings()}


def query_metric_bounds(capsule, clock_bits, state_bits, u33,
                        *, precision=DEFAULT_PRECISION, physical_cell=None):
    precision_check(precision)
    c = source()
    parsed = c.parse_capsule(capsule)
    membership = c.query_membership(capsule, clock_bits, state_bits)
    require(membership['status'] == 'PASS_SOURCE_QUERY_MEMBERSHIP', 'QUERY_MEMBERSHIP_REQUIRED')
    q = parsed['q']
    left, right = q['x0'], q['x1']
    if physical_cell is not None:
        require(isinstance(physical_cell, (list, tuple)) and len(physical_cell) == 2,
                'PHYSICAL_CELL_PAIR_REQUIRED')
        left, right = map(exact, physical_cell)
    _, knot = c.bitvalue(clock_bits)
    require(q['x0'] <= left < right <= q['x1'] and left <= knot <= right,
            'QUERY_OUTSIDE_PHYSICAL_CELL')
    _, metric = _metric(u33)
    identity = _identity(capsule, parsed, metric, left, right)
    uniform = box_envelope_certificate(parsed['rho'], u33, precision=precision)
    specific = box_envelope_certificate(membership['component_errors'], u33, precision=precision)
    record = _record('r_query', specific['upper'], identity,
                     'inherited query membership and absolute-component-error independent-box envelope',
                     knot=knot, knot_bits=membership['clock_bits'], state_bits=[v.lower() for v in state_bits],
                     Q=specific['Q'], actual_floating_query_witness=False)
    return {'schema': 'vigilode.query-metric-box-bounds.v1',
            'status': 'PASS_QUERY_MEMBERSHIP_BOX_BOUND', **identity,
            'membership': membership, 'uniform_source_box': uniform,
            'knot_component_error_box': specific, 'r_query': record, **_ceilings()}


def generated_residual_transfer(capsule, partition, cells_with_external_bounds_only, u33,
                                *, depth=DEFAULT_DEPTH, precision=DEFAULT_PRECISION):
    require(isinstance(partition, list) and len(partition) >= 2, 'POSITIVE_PARTITION_REQUIRED')
    plan = resource_plan(depth, len(partition) - 1)
    precision_check(precision)
    c = source()
    parsed = c.parse_capsule(capsule)
    q = parsed['q']
    t = [exact(v) for v in partition]
    require(t[0] == q['x0'] and t[-1] == q['x1'], 'PARTITION_ENDPOINT_COVERAGE_REQUIRED')
    require(all(a < b for a,b in zip(t,t[1:])), 'PARTITION_MUST_STRICTLY_INCREASE')
    cells = cells_with_external_bounds_only
    require(isinstance(cells, list) and len(cells) == len(t)-1, 'ONE_CELL_PER_INTERVAL_REQUIRED')
    _, metric = _metric(u33)
    external_names = ('alpha','epsilon_eval','source_error','Jop','Tx','regularity')
    # Refuse all collisions/mismatches before constructing any certificates.
    for left, right, cell in zip(t,t[1:],cells):
        require(isinstance(cell, dict), 'CELL_OBJECT_REQUIRED')
        require(exact(cell.get('left')) == left and exact(cell.get('right')) == right,
                'CELL_PARTITION_MISMATCH')
        records = cell.get('bounds')
        require(isinstance(records, dict), 'CELL_BOUNDS_REQUIRED')
        require(not set(records).intersection(('B1','B2','r_query')), 'GENERATED_RECORD_COLLISION')
        for name in external_names:
            c.premise(records.get(name), name, metric)
    joined, certificates = [], []
    for left, right, cell in zip(t,t[1:],cells):
        derivative = cell_derivative_bounds(capsule,left,right,u33,depth=depth,precision=precision)
        query = query_metric_bounds(capsule,cell.get('knot_bits'),cell.get('state_bits'),u33,
                                    precision=precision,physical_cell=(left,right))
        records = dict(cell['bounds'])
        records.update(B1=derivative['B1'], B2=derivative['B2'], r_query=query['r_query'])
        joined.append({**cell, 'bounds': records})
        certificates.append({'derivative': derivative, 'query': query})
    transfer = c.residual_transfer(capsule, t, joined, metric)
    return {'schema': 'vigilode.generated-native-metric-residual-transfer.v1',
            'status': transfer['status'], 'fixed_norm_identity': metric,
            'resource_plan': plan, 'generated_certificates': certificates,
            'cells_with_generated_bounds': joined, 'transfer': transfer,
            'owner_provenance_verified': False, **_ceilings()}
