"""Source-bound real-arithmetic six-variable chemical polynomial certificates.

No owner module is imported or executed. Effective rates and inventory weights
are frozen supplied coefficients; preparation/clock/geometry/rounding are open.
"""
from fractions import Fraction as F
from functools import lru_cache
from pathlib import Path
import ast
import hashlib
import importlib.util
import itertools
import json
import math

ROOT = Path(__file__).resolve().parents[1]
UNIT = 'V8_SOURCE_CHEMICAL_JACOBIAN_REMAINDER_20261028'
BASS = 'cc602e209e7deeb511ab2a1f3b2268d55206c43c'
BINDING = '8cb268e8de741e284cf494013f1e0748d27f03c5e77ba0c64413ae92af082d16'
TOTALS = (F(2**53-1, 2**53), F(1), F(1))
BLOCKS = ((0,), (1, 2), (3, 4, 5))
COORDINATES = (23, 24, 25, 26, 27, 28)
RATE_NAMES = ('photo_H0', 'photo_He0', 'photo_He1', 'photo_Li0', 'photo_Li1', 'photo_Li2',
              'recomb_H1', 'recomb_He1', 'recomb_He2', 'recomb_Li1', 'recomb_Li2', 'recomb_Li3')


def require(ok, message):
    if not ok:
        raise ValueError(message)


@lru_cache(maxsize=1)
def prior():
    base = ROOT/'deps/prior_matrix'
    manifest = json.loads((ROOT/'deps/PRIOR_MATRIX_IDENTITY.json').read_text())
    for item in manifest['files']:
        require(hashlib.sha256((base/item['path']).read_bytes()).hexdigest() == item['sha256'],
                'PRIOR_MATRIX_BYTE_IDENTITY_MISMATCH:' + item['path'])
    spec = importlib.util.spec_from_file_location('chemical_reused_matrix', base/'code/matrix_bounds.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def exact(value):
    return prior().exact(value)


def vector(value, n, name):
    require(isinstance(value, (list, tuple)) and len(value) == n, 'INVALID_' + name + '_SHAPE')
    return [exact(x) for x in value]


def fixed_totals(totals=None):
    t = list(TOTALS) if totals is None else vector(totals, 3, 'TOTALS')
    require(tuple(t) == TOTALS, 'FIXED_SOURCE_TOTALS_MISMATCH_H_NOT_ONE')
    return t


def binding_check(binding):
    require(isinstance(binding, dict), 'SOURCE_BINDING_REQUIRED')
    require(binding.get('bass_commit') == BASS and binding.get('binding_id') == BINDING,
            'FIXED_SOURCE_BINDING_MISMATCH')
    require(binding.get('clock') == prior().prior().source().CLOCK, 'FIXED_CLOCK_MISMATCH')
    require(isinstance(binding.get('evidence'), str) and bool(binding['evidence'].strip()), 'INPUT_EVIDENCE_REQUIRED')
    return dict(binding)


def weights_check(weights, provenance):
    w = vector(weights, 3, 'ELEMENT_WEIGHTS')
    require(all(x > 0 for x in w), 'POSITIVE_FROZEN_ELEMENT_WEIGHTS_REQUIRED')
    require(isinstance(provenance, dict) and provenance.get('kind') in
            ('DIAGNOSTIC_SYNTHETIC_WEIGHTS', 'DECLARED_FROZEN_INPUT', 'SOURCE_RECONSTRUCTED_UNDER_RN'),
            'WEIGHT_PROVENANCE_REQUIRED')
    require(isinstance(provenance.get('evidence'), str) and bool(provenance['evidence'].strip()), 'WEIGHT_EVIDENCE_REQUIRED')
    return w, [w[0], w[1], 2*w[1], w[2], 2*w[2], 3*w[2]]


def ceilings():
    return {**prior().prior()._ceilings(), 'actual_model_enclosure_verified': False,
            'owner_prepared_scale_array_observed': False, 'full30_model_provider': False,
            'native_iteration_certificate': False}


@lru_cache(maxsize=1)
def source_bytes():
    manifest = json.loads((ROOT/'deps/SOURCE_BYTE_IDENTITY.json').read_text())
    result = {}
    for row in manifest['files']:
        path = ROOT/'deps/source_bytes'/row['name']
        data = path.read_bytes()
        require(hashlib.sha256(data).hexdigest() == row['sha256'], 'MODEL_SOURCE_BYTE_IDENTITY_MISMATCH')
        result[row['name']] = data
    return result, manifest


def coverage():
    _, manifest = source_bytes()
    return {'source': manifest, 'rows_reduced30': list(COORDINATES), 'columns_reduced30': list(COORDINATES),
            'scope': 'CHEMICAL_PARTIAL_6_BY_6_ONLY',
            'full_Jacobian_missing_blocks': 'OPEN_NOT_ZERO',
            'open_not_zero': ['chemical geometry columns -Fchem*(grad log H)^T', 'geometry rows',
                             'Thomson/tau rows', 'time variation', 'rate preparation bounds', 'callback rounding'],
            'source_formulation': 'Real arithmetic on frozen represented coefficients; effective rates supplied externally',
            'actual_effective_rate_enclosure_verified': False}


def rates_check(rates):
    r = vector(rates, 12, 'RATES')
    require(all(x >= 0 for x in r), 'NONNEGATIVE_EFFECTIVE_RATES_REQUIRED')
    return r


def polynomial(rates, weights, provenance, binding, *, totals=None):
    bind = binding_check(binding)
    r, t = rates_check(rates), fixed_totals(totals)
    w, e = weights_check(weights, provenance)
    p, q, b = prior().zeros(6), prior().zeros(6), [F(0)]*6
    # These coefficients are the six-variable restriction of adjacent source fluxes.
    for block, total, offset in zip(BLOCKS, t, (0,1,3)):
        first = block[0]
        b[first] = r[offset]*total
        for j in block:
            p[first][j] -= r[offset]
        for local, index in enumerate(block[:-1]):
            rate = r[offset+local+1]
            p[index][index] -= rate
            p[index+1][index] += rate
        for local, index in enumerate(block):
            rate = r[6+offset+local]
            q[index][index] -= rate
            if local:
                q[index-1][index] += rate
    return {'schema': 'vigilode.source-chemical-polynomial.v1', 'unit_id': UNIT,
            'binding': bind, 'rates': r, 'rate_order': list(RATE_NAMES), 'totals': t,
            'weights': w, 'electron_coefficients': e, 'weight_provenance': dict(provenance),
            'b': b, 'P': p, 'Q': q, 'coverage': coverage(), **ceilings()}


def matvec(a, x):
    return [sum((aij*xj for aij, xj in zip(row,x) if aij and xj), F(0)) for row in a]


def dot(a, b):
    return sum((x*y for x,y in zip(a,b)), F(0))


def state_membership(z, *, totals=None):
    z, t = vector(z, 6, 'CHEMICAL_STATE'), fixed_totals(totals)
    populations = []
    valid = True
    for block, total in zip(BLOCKS,t):
        charged = [z[i] for i in block]
        neutral = total-sum(charged,F(0))
        populations.extend([neutral]+charged)
        valid &= neutral >= 0 and all(x >= 0 for x in charged)
    return {'in_product_simplex': bool(valid), 'nine_state_populations': populations, 'totals': t}


def evaluate(model, z):
    z = vector(z, 6, 'CHEMICAL_STATE')
    pz, qz = matvec(model['P'],z), matvec(model['Q'],z)
    ne = dot(model['electron_coefficients'],z)
    return [b+p+ne*q for b,p,q in zip(model['b'],pz,qz)]


def jacobian(model, z):
    z = vector(z, 6, 'CHEMICAL_STATE')
    e = model['electron_coefficients']
    ne, qz = dot(e,z), matvec(model['Q'],z)
    return [[model['P'][i][j]+ne*model['Q'][i][j]+qz[i]*e[j]
             for j in range(6)] for i in range(6)]


def hessian_action(model, d, h):
    d,h = vector(d,6,'INCREMENT'),vector(h,6,'DIRECTION')
    e = model['electron_coefficients']
    ed,eh = dot(e,d),dot(e,h)
    return [ed*qh+eh*qd for qh,qd in zip(matvec(model['Q'],h),matvec(model['Q'],d))]


def quadratic_remainder(model, d):
    d = vector(d,6,'INCREMENT')
    ed = dot(model['electron_coefficients'],d)
    return [ed*x for x in matvec(model['Q'],d)]


def state_vertices(*, totals=None):
    t = fixed_totals(totals)
    vertices = []
    for choices in itertools.product(range(2),range(3),range(4)):
        z = [F(0)]*6
        for choice, block, total in zip(choices,BLOCKS,t):
            if choice:
                z[block[choice-1]] = total
        vertices.append(z)
    return vertices


def rate_box_check(intervals):
    require(isinstance(intervals, (list,tuple)) and len(intervals) == 12, 'TWELVE_RATE_INTERVALS_REQUIRED')
    box = [vector(pair,2,'RATE_INTERVAL') for pair in intervals]
    require(all(0 <= lo <= hi for lo,hi in box), 'NONNEGATIVE_INCREASING_RATE_INTERVALS_REQUIRED')
    return box


def jacobian_extrema(intervals, weights, provenance, binding, *, totals=None, requested_scope='CHEMICAL_PARTIAL_6_BY_6_ONLY'):
    require(requested_scope == 'CHEMICAL_PARTIAL_6_BY_6_ONLY', 'PARTIAL_BLOCK_CANNOT_PROMOTE_FULL_MODEL_PROVIDER')
    bind = binding_check(binding)
    t, box = fixed_totals(totals), rate_box_check(intervals)
    weights_check(weights,provenance)
    unit_models = [polynomial([F(int(j==k)) for j in range(12)], weights,provenance,bind,totals=t) for k in range(12)]
    entries = [[{'row':i,'column':j,'lower':None,'upper':None} for j in range(6)] for i in range(6)]
    vertices = state_vertices(totals=t)
    for vertex_index,z in enumerate(vertices):
        coefficients = [jacobian(model,z) for model in unit_models]
        for i in range(6):
            for j in range(6):
                cs = [a[i][j] for a in coefficients]
                for side in ('min','max'):
                    rates = [hi if (v < 0 if side=='min' else v > 0) else lo for v,(lo,hi) in zip(cs,box)]
                    value = dot(cs,rates)
                    name = 'lower' if side=='min' else 'upper'
                    old = entries[i][j][name]
                    if old is None or (value < old if side=='min' else value > old):
                        entries[i][j][name] = value
                        entries[i][j][side+'_witness'] = {'state_vertex_index':vertex_index,'state':z[:],
                                                          'rates':rates,'rate_coefficients':cs,'value':value}
    center = [[(x['lower']+x['upper'])/2 for x in row] for row in entries]
    radius = [[(x['upper']-x['lower'])/2 for x in row] for row in entries]
    return {'schema':'vigilode.exact-chemical-jacobian-entry-extrema.v1','unit_id':UNIT,
            'status':'PASS_EXACT_DECLARED_PRODUCT_FAMILY_ENTRY_EXTREMA','binding':bind,
            'rate_intervals':box,'weights':vector(weights,3,'ELEMENT_WEIGHTS'),
            'weight_provenance':dict(provenance),'totals':t,'state_vertices':vertices,
            'entries':entries,'center6':center,'radius6':radius,
            'entry_count':36,'attaining_witness_count':72,'state_vertex_count':24,
            'rate_coefficient_count_per_entry':12,'proof':'Affine in full state at fixed rates and affine in rates at each state vertex; endpoint signs choose each rate independently',
            'family_correlation':'Declared independent state-simplex product and rate box; shared H/T/UV correlation may make this wider than the actual family',
            'coverage':coverage(),**ceilings()}


def remainder_bound(model, u33, *, refinement=8, precision=40):
    """Named coefficient operator only: this is not a completed model Jacobian."""
    m = prior()
    m.resource_check(refinement,precision)
    met = m.metric(u33)
    qbar, ebar = m.zeros(), [F(0)]*30
    for i,a in enumerate(COORDINATES):
        ebar[a] = model['electron_coefficients'][i]
        for j,b in enumerate(COORDINATES):
            qbar[a][b] = model['Q'][i][j]
    t = m.multiply(m.transpose(qbar),m.multiply(met['G'],qbar))
    s = m.generalized_upper(t,met['G'],met['Ginv'],refinement=refinement)
    sigma2 = m.quadratic(met['Ginv'],ebar)
    product = sigma2*s['upper']
    root = m.prior().scalar_sqrt_enclosure(product,precision=precision)
    return {'schema':'vigilode.fixed-coefficient-chemical-remainder-bound.v1','unit_id':UNIT,
            'status':'PASS_FIXED_COEFFICIENT_CHEMICAL_POLYNOMIAL_REMAINDER_BOUND',
            'scope':'CHEMICAL_POLYNOMIAL_REMAINDER_ONLY','Qbar_role':'NAMED_POLYNOMIAL_COEFFICIENT_OPERATOR_NOT_FULL_JACOBIAN',
            'Qbar30':qbar,'ebar30':ebar,'frozen_rates':model['rates'],
            'weights':model['weights'],'weight_provenance':model['weight_provenance'],
            'fixed_norm_identity':met['fixed_norm_identity'],'upper_scales':met['upper_scales'],
            'operator_squared_Qbar':s,'dual_e_sigma_squared':sigma2,'Gamma_squared_sufficient':product,
            'Gamma_sqrt':root,'Gamma':root['upper'],'Hessian_bilinear_factor':2*root['upper'],
            'inequalities':['||R(d)||_G <= Gamma ||d||_G^2', '||D2F[d,h]||_G <= 2 Gamma ||d||_G ||h||_G'],
            'rates_scope':'FIXED_SUPPLIED_RATES_ONLY; interval-rate extension not implemented',
            'coverage':coverage(),**ceilings()}


def reconstruct_inventory_RN():
    """Finite source scalar inventory witness; never imports/initializes owner code."""
    data,manifest = source_bytes()
    model = json.loads(data['atomic_model.json'],parse_float=str,parse_int=str)
    tree = ast.parse(data['atomic_network.py'])
    mass_values, groups = None,None
    for node in ast.walk(tree):
        if isinstance(node,ast.Assign):
            for target in node.targets:
                if isinstance(target,ast.Attribute) and isinstance(target.value,ast.Name) and target.value.id=='self':
                    if target.attr=='A' and isinstance(node.value,ast.Call):
                        mass_values = ast.literal_eval(node.value.args[0])
                    if target.attr=='isotope_groups':
                        groups = ast.literal_eval(node.value)
    require(mass_values is not None and groups is not None, 'STATIC_INVENTORY_SOURCE_LITERALS_REQUIRED')
    def rn(q):
        v = float(q)
        require(math.isfinite(v), 'RN_INVENTORY_NONFINITE')
        return F.from_float(v), v.hex()
    ratios, ratio_hex, products, product_hex, mass_hex = {},{},{},{},{}
    for name,indices in groups.items():
        ratios[name],ratio_hex[name] = rn(F(model['inventory_number_ratios_to_H1'][name]))
        mass,mass_hex[name] = rn(F(mass_values[indices[0]]))
        products[name],product_hex[name] = rn(mass*ratios[name])
    normalization,normalization_hex = rn(sum(products.values(),F(0)))
    require(normalization>0,'POSITIVE_RN_INVENTORY_NORMALIZATION_REQUIRED')
    inventory,inventory_hex = {},{}
    for name in groups:
        inventory[name],inventory_hex[name] = rn(ratios[name]/normalization)
    zero_scale,zero_scale_hex = rn(F(model['zero_inventory_scale']))
    assigned_scale_hex = {name:inventory_hex[name] if value>0 else zero_scale_hex for name,value in inventory.items()}
    weights = [inventory['H1']+inventory['D'],inventory['He3']+inventory['He4'],inventory['Li6']+inventory['Li7']]
    return {'schema':'vigilode.source-reconstructed-inventory-RN.v1','status':'SOURCE_RECONSTRUCTED_UNDER_RN',
            'provenance':{'kind':'SOURCE_RECONSTRUCTED_UNDER_RN','evidence':'Static pinned source scalar operations; ratios rounded to binary64, mass-times-ratio rounded, exact fsum rounded, divisions rounded; exact real element sums'},
            'input_ratio_decimal_literals':model['inventory_number_ratios_to_H1'],
            'source_mass_array_literals':mass_values,'representative_mass_hex':mass_hex,
            'ratio_hex':ratio_hex,'rounded_mass_times_ratio_hex':product_hex,
            'zero_inventory_scale_hex':zero_scale_hex,'assigned_isotope_scale_hex':assigned_scale_hex,
            'normalization_hex':normalization_hex,'inventory_hex':inventory_hex,
            'represented_inventory_exact':inventory,'weights_exact_real_sums':weights,
            'electron_coefficients':[weights[0],weights[1],2*weights[1],weights[2],2*weights[2],3*weights[2]],
            'source_identity':manifest,'original_owner_scale_array_observed':False,
            'original_source_manifest_body_recovered':False,'source_initialization_calls':0,
            'native_model_calls':0,'rounding_assumption':'IEEE binary64 RN ties-to-even, gradual underflow; finite scalar reconstruction only',
            'physical_interpretation':'Not an observed owner-prepared scale array or actual rate enclosure',**ceilings()}


def jsonable(value):
    return prior().jsonable(value)
