"""Finite rational matrix certificates in the inherited fixed physical33 metric.

No model, native norm, lift, ODE, or numerical eigensolver is called. Supplied
matrix enclosures remain external premises even when their algebra is verified.
"""
from fractions import Fraction as F
from functools import lru_cache
from pathlib import Path
import hashlib
import importlib.util
import json

ROOT = Path(__file__).resolve().parents[1]
UNIT = 'V8_FIXED_METRIC_JACOBIAN_LOGNORM_20261028'
ROLE = 'WHOLE_LINKING_PATH_AND_R_PLUS_GAMMA'


def require(ok, code):
    if not ok:
        raise ValueError(code)


@lru_cache(maxsize=1)
def prior():
    base = ROOT / 'deps/prior_metric'
    manifest = json.loads((ROOT / 'deps/PRIOR_COPY_IDENTITY.json').read_text())
    for row in manifest['files']:
        require(hashlib.sha256((base / row['path']).read_bytes()).hexdigest() == row['sha256'],
                'PRIOR_BYTE_IDENTITY_MISMATCH:' + row['path'])
    spec = importlib.util.spec_from_file_location('matrix_reused_native_bounds', base / 'code/native_metric_bounds.py')
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


def exact(value):
    return prior().exact(value)


def resource_check(refinement, precision):
    require(isinstance(refinement, int) and not isinstance(refinement, bool)
            and 0 <= refinement <= 12, 'REFINEMENT_OUTSIDE_FROZEN_CAP')
    prior().precision_check(precision)


def matrix(value, *, dimension=30, nonnegative=False):
    require(isinstance(dimension, int) and not isinstance(dimension, bool)
            and 1 <= dimension <= 30, 'MATRIX_DIMENSION_CAP')
    require(isinstance(value, list) and len(value) == dimension
            and all(isinstance(r, list) and len(r) == dimension for r in value), 'MATRIX_SHAPE_MISMATCH')
    a = [[exact(v) for v in row] for row in value]
    require(not nonnegative or all(v >= 0 for r in a for v in r), 'NEGATIVE_MATRIX_RADIUS')
    return a


def zeros(n=30):
    return [[F(0) for _ in range(n)] for _ in range(n)]


def identity(n=30):
    a = zeros(n)
    for i in range(n):
        a[i][i] = F(1)
    return a


def transpose(a):
    return [list(r) for r in zip(*a)]


def multiply(a, b):
    n = len(a)
    out = zeros(n)
    sparse_b = [[(j, v) for j, v in enumerate(row) if v] for row in b]
    for i, row in enumerate(a):
        for k, v in enumerate(row):
            if v:
                for j, w in sparse_b[k]:
                    out[i][j] += v * w
    return out


def pencil(t, g, bound):
    return [[bound * gij - tij for gij, tij in zip(gr, tr)] for gr, tr in zip(g, t)]


def quadratic(a, v):
    return sum((x * y * aij for x, row in zip(v, a) if x
                for y, aij in zip(v, row) if y and aij), F(0))


@lru_cache(maxsize=16)
def _gram_cached(u):
    p = prior()
    C, E, _, _, _ = p.source().dependencies()
    g = zeros()
    for scale, terms in zip(u, p._row_terms()):
        for i, v in terms.items():
            for j, w in terms.items():
                g[i-1][j-1] += v * w / (33 * scale * scale)
    columns = []
    for i in range(30):
        e = [F(int(i == j)) for j in range(30)]
        supported = C.support_core(e, list(u), E, precision=0)
        columns.append([33 * exact(v) for v in supported['w']])
    inverse = transpose(columns)
    require(multiply(g, inverse) == identity(), 'INHERITED_GRAM_INVERSE_IDENTITY_FAILED')
    require(psd_decision(g)['status'] == 'PSD', 'INHERITED_GRAM_NOT_POSITIVE_DEFINITE')
    return g, inverse


def metric(u33):
    u, metric_id = prior()._metric(u33)
    g, inverse = _gram_cached(tuple(u))
    return {'upper_scales': u, 'fixed_norm_identity': metric_id, 'G': g,
            'Ginv': inverse, 'G_times_Ginv_equals_I': True,
            'inverse_source': '33*w from inherited support_core(e_i,u,E,precision=0)',
            'positive_definiteness_basis': 'Inherited full-column-rank physical lift and positive frozen scales'}


def psd_decision(value):
    """Exact unpivoted symmetric Schur elimination, including singular pivots."""
    require(isinstance(value, list) and 1 <= len(value) <= 30, 'PSD_DIMENSION_CAP')
    original = matrix(value, dimension=len(value))
    n = len(original)
    require(original == transpose(original), 'SYMMETRIC_MATRIX_REQUIRED')
    a = [r[:] for r in original]
    coeff = zeros(n)
    pivots = []

    def negative(v, reason, pivot):
        for k in range(len(pivots)-1, -1, -1):
            if pivots[k] > 0:
                v[k] = -sum((coeff[k][j] * v[j] for j in range(k+1, n)), F(0))
        q = quadratic(original, v)
        require(q < 0, 'NEGATIVE_WITNESS_INTERNAL_FAILURE')
        return {'status': 'NOT_PSD', 'reason': reason, 'pivot_index': pivot,
                'negative_witness': v, 'quadratic': q, 'pivots_completed': pivots[:]}

    for k in range(n):
        d = a[k][k]
        if d < 0:
            v = [F(0)] * n
            v[k] = F(1)
            return negative(v, 'NEGATIVE_PIVOT', k)
        if d == 0:
            j = next((j for j in range(k+1, n) if a[k][j]), None)
            if j is not None:
                v = [F(0)] * n
                v[j] = F(1)
                v[k] = -(a[j][j] + 1) / (2 * a[k][j])
                return negative(v, 'ZERO_PIVOT_NONZERO_ROW', k)
            pivots.append(d)
            continue
        active = [j for j in range(k+1, n) if a[k][j]]
        for j in active:
            coeff[k][j] = a[k][j] / d
        for i in active:
            for j in active:
                a[i][j] -= a[k][i] * coeff[k][j]
        pivots.append(d)
    return {'status': 'PSD', 'pivots': pivots, 'completed_square_coefficients': coeff,
            'zero_pivot_rows_verified': [i for i, p in enumerate(pivots) if p == 0],
            'factor_identity': 'A=U^T diag(pivots) U, U=I+completed_square_coefficients'}


def generalized_upper(t, g, inverse, *, refinement=8, proposal=None):
    """Finite exact generalized-largest-eigenvalue bracket; upper carries PSD."""
    resource_check(refinement, 40)
    lower = max(t[i][i] / g[i][i] for i in range(len(g)))
    product = multiply(inverse, t)
    upper = max(row[i] + sum((abs(v) for j, v in enumerate(row) if j != i), F(0))
                for i, row in enumerate(product))
    initial = [lower, upper]
    cert = psd_decision(pencil(t, g, upper))
    require(cert['status'] == 'PSD', 'INITIAL_GERSHGORIN_UPPER_PSD_FAILURE')
    proposal_evidence = None
    if proposal is not None:
        proposed = exact(proposal)
        proposal_evidence = {'value': proposed, 'decision': psd_decision(pencil(t, g, proposed))}
        if proposal_evidence['decision']['status'] == 'PSD' and proposed < upper:
            upper, cert = proposed, proposal_evidence['decision']
    last_negative, steps = None, 0
    for _ in range(refinement):
        if lower == upper:
            break
        mid = (lower + upper) / 2
        decision = psd_decision(pencil(t, g, mid))
        if decision['status'] == 'PSD':
            upper, cert = mid, decision
        else:
            lower, last_negative = mid, {'value': mid, 'decision': decision}
        steps += 1
    return {'lower': lower, 'upper': upper, 'initial_bracket': initial,
            'steps': steps, 'requested_steps': refinement, 'upper_psd_certificate': cert,
            'last_negative_lower_witness': last_negative, 'proposal': proposal_evidence}


def beta_squared(radius, g, inverse):
    # Exactly the prescribed finite Frobenius majorant, skipping only zeros.
    nonzero = [(i, j, v) for i, row in enumerate(radius) for j, v in enumerate(row) if v]
    return sum((abs(g[i][k]) * abs(inverse[j][l]) * rij * rkl
                for i, j, rij in nonzero for k, l, rkl in nonzero
                if g[i][k] and inverse[j][l]), F(0))


def validate_binding(binding, metric_id, *, capsule=None, left=None, right=None, gamma=None):
    c = prior().source()
    require(isinstance(binding, dict), 'FAMILY_BINDING_REQUIRED')
    for name, value in [('bass_commit', c.BASS), ('binding_id', c.BINDING),
                        ('clock', c.CLOCK), ('fixed_norm_identity', metric_id),
                        ('enclosure_role', ROLE), ('corridor', 'R_PLUS_GAMMA')]:
        require(binding.get(name) == value, 'FAMILY_BINDING_MISMATCH:' + name)
    for name in ('evidence', 'enclosure_identity', 'corridor_identity'):
        require(isinstance(binding.get(name), str) and bool(binding[name].strip()), 'FAMILY_IDENTITY_REQUIRED:' + name)
    l, r, ga = (exact(binding.get(k)) for k in ('left', 'right', 'gamma'))
    require(l < r, 'FAMILY_CELL_MUST_INCREASE')
    require(ga > 0, 'POSITIVE_OUTER_GAMMA_REQUIRED')
    for key, expected, actual in [('left', left, l), ('right', right, r), ('gamma', gamma, ga)]:
        if expected is not None:
            require(actual == exact(expected), 'FAMILY_CELL_CORRIDOR_MISMATCH:' + key)
    if capsule is not None:
        require(binding.get('capsule_identity') == prior()._capsule_identity(capsule), 'FAMILY_CAPSULE_IDENTITY_MISMATCH')
    return {**binding, 'left': l, 'right': r, 'gamma': ga}


def family_certificate(center, radius, u33, binding, *, refinement=8, precision=40,
                       s_proposal=None, m_proposal=None):
    resource_check(refinement, precision)
    # Validate all external identities and shape before matrix certificate construction.
    u, metric_id = prior()._metric(u33)
    b = validate_binding(binding, metric_id)
    c, r = matrix(center), matrix(radius, nonnegative=True)
    met = metric(u)
    g, inverse = met['G'], met['Ginv']
    gc = multiply(g, c)
    t = multiply(transpose(c), gc)
    s = [[(gc[i][j] + gc[j][i]) / 2 for j in range(30)] for i in range(30)]
    op = generalized_upper(t, g, inverse, refinement=refinement, proposal=s_proposal)
    log = generalized_upper(s, g, inverse, refinement=refinement, proposal=m_proposal)
    root = prior().scalar_sqrt_enclosure(op['upper'], precision=precision)
    b2 = beta_squared(r, g, inverse)
    beta = prior().scalar_sqrt_enclosure(b2, precision=precision)
    jop, mu = root['upper'] + beta['upper'], log['upper'] + beta['upper']
    ident = {**b, 'source_sha256': prior().source().SOURCE}
    records = {name: prior()._record(name, value, ident,
               'Exact generalized-eigenvalue PSD upper and center-plus-Frobenius-radius inequality',
               actual_model_enclosure_verified=False, enclosure_premise='EXTERNAL_NOT_VERIFIED')
               for name, value in [('Jop', jop), ('mu', mu)]}
    return {'schema': 'vigilode.fixed-metric-matrix-bounds.v1',
            'status': 'PASS_SCOPED_INTERVAL_MATRIX_BOUNDS', 'unit_id': UNIT,
            'verification_scope': 'EXACT_FINITE_ALGEBRA_ONLY', **ident,
            'center': c, 'radius': r, 'upper_scales': u,
            'operator_squared_center': op, 'lognorm_center': log,
            'operator_center_sqrt': root, 'beta_squared': b2, 'beta': beta,
            'Jop': records['Jop'], 'mu': records['mu'],
            'family_interpretation': 'Independent entry box enclosing superset; sufficient bound, no optimality claim',
            'actual_model_enclosure_verified': False, 'owner_provenance_verified': False,
            **prior()._ceilings()}


def pr90_bridge(cached, current, *, a0, a, refinement=8, precision=40):
    a0, a = exact(a0), exact(a)
    require(a0 > 0 and a > 0, 'POSITIVE_PR90_STEPS_REQUIRED')
    for key in ('bass_commit', 'binding_id', 'clock', 'fixed_norm_identity',
                'left', 'right', 'gamma', 'corridor', 'corridor_identity'):
        require(cached.get(key) == current.get(key), 'PR90_FAMILY_BINDING_MISMATCH:' + key)
    require(cached.get('status') == current.get('status') == 'PASS_SCOPED_INTERVAL_MATRIX_BOUNDS', 'PR90_GENERATED_CERTIFICATES_REQUIRED')
    mu0, mu1 = exact(cached['mu']['value']), exact(current['mu']['value'])
    require(mu0 <= 0, 'CACHED_DISSIPATIVITY_REQUIRED')
    difference = [[y - x for x, y in zip(row0, row1)] for row0, row1 in zip(cached['center'], current['center'])]
    radius = [[x + y for x, y in zip(row0, row1)] for row0, row1 in zip(cached['radius'], current['radius'])]
    binding = {key: current[key] for key in ('bass_commit', 'binding_id', 'clock', 'fixed_norm_identity',
               'left', 'right', 'gamma', 'corridor', 'corridor_identity', 'enclosure_role', 'evidence')}
    binding['enclosure_identity'] = 'DIFFERENCE:' + current['enclosure_identity'] + ':' + cached['enclosure_identity']
    diff = family_certificate(difference, radius, current['upper_scales'], binding,
                              refinement=refinement, precision=precision)
    alpha = -mu0
    omega, qstar = 2*a/(a+a0), abs(a-a0)/(a+a0)
    q = qstar + omega*a0*diff['Jop']['value']/(1+a0*alpha)
    alpha_prime = -mu1 if mu1 <= 0 else None
    return {'schema': 'vigilode.pr90-exact-reference-bridge.v1',
            'status': 'PASS_EXACT_REFERENCE_CONTRACTION' if q < 1 else 'INCONCLUSIVE_Q_NOT_BELOW_ONE',
            'a0': a0, 'a': a, 'alpha': alpha, 'alpha_prime': alpha_prime,
            'omega': omega, 'qstar': qstar, 'q': q, 'd_bound': diff['Jop']['value'],
            'difference_family': diff, 'difference_correlation': 'R0+R1 loses shared-parameter correlation',
            'scaled_residual_factor': a/(1+a*alpha_prime) if alpha_prime is not None else None,
            'normalized_residual_factor': 1/(1+a*alpha_prime) if alpha_prime is not None else None,
            'reference_scope': 'Exact inverse and exact relaxation only; no commutation assumption',
            'external_errors': ['inverse-action error', 'represented-relaxation error'],
            'native_iteration_certificate': False, 'actual_divergence_claimed': False,
            **prior()._ceilings()}


def generated_capsule_chain(capsule, partition, cells, families, u33, gamma,
                            E0_record, continuation, *, refinement=8, precision=40):
    resource_check(refinement, precision)
    require(isinstance(partition, list) and 2 <= len(partition) <= 17, 'CELL_CAP_OR_PARTITION_REQUIRED')
    t = [exact(v) for v in partition]
    require(all(l < r for l, r in zip(t, t[1:])), 'PARTITION_MUST_STRICTLY_INCREASE')
    require(isinstance(cells, list) and isinstance(families, list)
            and len(cells) == len(families) == len(t)-1, 'ONE_CELL_FAMILY_PER_INTERVAL_REQUIRED')
    p = prior()
    parsed = p.source().parse_capsule(capsule)
    require(t[0] == parsed['q']['x0'] and t[-1] == parsed['q']['x1'], 'PARTITION_ENDPOINT_COVERAGE_REQUIRED')
    _, metric_id = p._metric(u33)
    source = p.source()
    source.premise(E0_record, 'E0', metric_id)
    require(exact(E0_record.get('at')) == t[0], 'INITIAL_ENDPOINT_MISMATCH')
    source.premise(continuation, 'continuation', metric_id)
    p.piecewise().interval_record(continuation, t[0], t[-1], outer=True, gamma=exact(gamma))
    corridor_id = continuation.get('corridor_identity')
    require(isinstance(corridor_id, str) and bool(corridor_id.strip()), 'CONTINUATION_CORRIDOR_IDENTITY_REQUIRED')
    # Preflight every cell and all collisions before any matrix certificate.
    for left, right, cell, family in zip(t, t[1:], cells, families):
        require(isinstance(cell, dict) and isinstance(cell.get('bounds'), dict), 'CELL_BOUNDS_REQUIRED')
        require(exact(cell.get('left')) == left and exact(cell.get('right')) == right, 'CELL_PARTITION_MISMATCH')
        require(not set(cell['bounds']).intersection(('Jop', 'mu', 'B1', 'B2', 'r_query')), 'GENERATED_RECORD_COLLISION')
        require(isinstance(family, dict), 'MATRIX_FAMILY_REQUIRED')
        b = validate_binding(family.get('binding'), metric_id, capsule=capsule, left=left, right=right, gamma=gamma)
        require(b['corridor_identity'] == corridor_id, 'FAMILY_CONTINUATION_CORRIDOR_IDENTITY_MISMATCH')
        for name in ('alpha', 'epsilon_eval', 'source_error', 'Tx', 'regularity'):
            source.premise(cell['bounds'].get(name), name, metric_id)
        matrix(family.get('center'))
        matrix(family.get('radius'), nonnegative=True)
    certs, joined, mus = [], [], []
    for cell, family in zip(cells, families):
        cert = family_certificate(family['center'], family['radius'], u33, family['binding'],
                                  refinement=refinement, precision=precision)
        certs.append(cert)
        joined.append({**cell, 'bounds': {**cell['bounds'], 'Jop': cert['Jop']}})
        mus.append(cert['mu'])
    generated = p.generated_residual_transfer(capsule, t, joined, u33, depth=2, precision=precision)
    chain = p.piecewise().capsule_chain(capsule, t, generated['cells_with_generated_bounds'], mus,
                 E0_record, u33, gamma, continuation, max_subcells=4096, refinement=1,
                 sqrt_precision=precision, max_isolation_steps=128)
    return {'status': chain['status'], 'matrix_certificates': certs,
            'generated_residual_transfer': generated, 'piecewise_join': chain,
            'actual_model_enclosure_verified': False, 'owner_provenance_verified': False,
            'external_nonlinear_premises': ['C1 vector field', 'Jacobian enclosed on whole linking paths and R_PLUS_GAMMA corridor',
               'path inclusion', 'initial bound', 'source inputs', 'continuation'], **p._ceilings()}


def jsonable(value):
    if isinstance(value, F):
        return {'numerator': str(value.numerator), 'denominator': str(value.denominator)}
    if isinstance(value, dict):
        return {str(k): jsonable(v) for k, v in value.items()}
    if isinstance(value, (list, tuple)):
        return [jsonable(v) for v in value]
    return value
