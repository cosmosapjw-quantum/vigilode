import glob, json, os, random, struct, sys
import numpy as np

C = sys.argv[1]
S = sys.argv[2]
ref = np.load(S + '/ref_mp.npy')
req = json.load(open(C + '/requests/HEAT_BASELINE_REQUEST.json'))['case']
T = req['output_times']
rtol = 1e-6
atol = np.array(req['atol'])
assert np.all(atol == 1e-9) and req['rtol'] == rtol
bud = json.load(open(C + '/results/APPLICATION_BUDGET.json'))
assert bud['rtol'] == rtol and bud['atol'] == req['atol']
den = atol[None, :] + rtol * np.abs(ref)


def ev(states):
    y = np.array(states, dtype=float)
    assert y.shape == ref.shape, y.shape
    assert np.all(np.isfinite(y))
    z = np.abs(y - ref) / den
    i = np.unravel_index(np.argmax(z), z.shape)
    return float(z.max()), float(z[-1].max()), float(np.abs(y - ref).max()), tuple(int(a) for a in i)


out = {}
# smoke
for arm in ('BASE', 'STEN'):
    d = json.load(open(f'{C}/raw/smoke/{arm}.stdout.json'))
    assert d['times'] == T
    out[f'smoke_{arm}'] = ev(d['states'])
print('smoke', out)

# A0 and A: all files, bitwise identity, times
y0 = req['initial_state']
allfiles = sorted(glob.glob(C + '/raw/A0/*-BASE.json') + glob.glob(C + '/raw/A0/*-STEN.json')
                  + glob.glob(C + '/raw/A/*-BASE.json') + glob.glob(C + '/raw/A/*-STEN.json'))
print('A0+A run files', len(allfiles))
first = None
nbad_times = nbad_states = 0
counts = {}
cnt_sets = {}
dw_sets = {}
for f in allfiles:
    d = json.load(open(f))
    arm = 'BASE' if f.endswith('-BASE.json') else 'STEN'
    ph = os.path.basename(os.path.dirname(f))
    counts[(ph, arm)] = counts.get((ph, arm), 0) + 1
    if d['times'] != T or any(struct.pack('<d', a) != struct.pack('<d', b) for a, b in zip(d['times'], T)):
        nbad_times += 1
    bits = np.array(d['states'], dtype=float).view(np.uint64)
    if first is None:
        first = bits
        firstf = f
    elif not np.array_equal(bits, first):
        nbad_states += 1
    cnt_sets.setdefault(arm, set()).add(json.dumps(d['counters'], sort_keys=True))
    dw_sets.setdefault(arm, set()).add(json.dumps(d['derivative_work'], sort_keys=True))
    if d['status'] != 'OK':
        print('status', f, d['status'])
print('counts', counts)
print('times mismatches', nbad_times, 'state bit mismatches vs', os.path.basename(firstf), nbad_states)
st0 = np.array(json.load(open(firstf))['states'])
print('t0 state equals y0 bitwise:', np.array_equal(st0[0].view(np.uint64), np.array(y0).view(np.uint64)))
for arm in cnt_sets:
    print(arm, 'distinct counters', len(cnt_sets[arm]), 'distinct derivative_work', len(dw_sets[arm]), list(dw_sets[arm]))
c = json.loads(next(iter(cnt_sets['BASE'])))
print('BASE accepted', c['accepted_steps'], 'rhs', c['rhs_calls'], 'rhs_eval', c['rhs_evaluations'],
      'jac builds', c['jacobian_builds'], 'lu', c['direct_factorizations'], 'solves', c['linear_solves'])
print('counters BASE==STEN', cnt_sets['BASE'] == cnt_sets['STEN'])

# random 20+ of each arm in A
random.seed(20261021)
for arm in ('BASE', 'STEN'):
    fl = sorted(glob.glob(f'{C}/raw/A/*-{arm}.json'))
    pick = random.sample(fl, 25)
    res = [ev(json.load(open(f))['states']) for f in pick]
    print('A', arm, 'sampled', len(pick), 'max scaled', max(r[0] for r in res), 'endpoint', max(r[1] for r in res),
          'maxabs', max(r[2] for r in res), 'argmax', set(r[3] for r in res))
# also full A eval since identical states -> just one
print('full first-file eval', ev(st0))

# comparators: all measured reps (and warmups if present) in X files
for arm in ('VODE_MF20', 'VODE_MF21', 'VODE_MF22', 'JULIA_RODAS5P', 'DIFFSOL_BDF'):
    files = sorted(glob.glob(f'{C}/raw/B_{arm}/*-X.json'))
    if arm == 'JULIA_RODAS5P':
        files = [f'{C}/raw/B_{arm}/preflight.json'] + files
    vals = []
    tbad = 0
    cset = set()
    t0inj = set()
    for f in files:
        d = json.load(open(f))
        for m in d['measurements']:
            tm = m.get('times')
            if tm is not None and tm != T:
                tbad += 1
            if tm is None and arm != 'DIFFSOL_BDF':
                tbad += 1
            vals.append(ev(m['states']))
            cset.add(json.dumps(m.get('counters'), sort_keys=True))
            t0inj.add(np.array_equal(np.array(m['states'][0]), np.array(y0)))
    print(arm, 'files', len(files), 'reps', len(vals), 'time mismatches', tbad,
          'max scaled range', min(v[0] for v in vals), max(v[0] for v in vals),
          'endpoint', max(v[1] for v in vals), 'maxabs', max(v[2] for v in vals),
          'argmax', set(v[3] for v in vals), 'pass', sum(v[0] <= 1 for v in vals),
          'distinct counters', len(cset), 't0==y0', t0inj)
    print('   counters', list(cset)[:2])
