"""Independent read-only aggregation of retained structural experiment outputs."""
from pathlib import Path
import hashlib
import json
import math
import re
import statistics
import struct
import subprocess
import time
from fractions import Fraction as F

ROOT = Path('/workspace/scratch/f9615df1ce94')
REPO = ROOT / 'vigilode'
RAW = ROOT / 'lane-structural02'
SHA = '0a2a8eb9b9b0b775561f42f42cf36b1ef6e96f4b'
NODE = Path('research/synchronized_production_20261029')
started = time.perf_counter()
checks = 0

def require(ok, why):
    global checks
    checks += 1
    if not ok:
        raise AssertionError(why)

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def load(path):
    return json.loads(path.read_text())

def fbits(x):
    return struct.unpack('>Q', struct.pack('>d', x))[0]

def qbits(x):
    return F.from_float(struct.unpack('>d', struct.pack('>Q', x))[0])

run = load(RAW / 'RUN.json')
require(run['source'] == SHA and run['execution'] == 'completed', 'actual execution source/result')
require(run['checks_result'] == 'pass_scoped' and run['benchmark_result'] == 'pass_scoped_all_registered_cases', 'actual gate results')
require(run['source_hashes_unchanged'], 'execution source stability')
before = load(RAW / 'SOURCE_HASHES_BEFORE.json')
after = load(RAW / 'SOURCE_HASHES_AFTER.json')
require(before == after, 'complete pre/post hashes')
for rel, sha in before.items():
    if rel.startswith('crates/') or rel in ('Cargo.toml', 'Cargo.lock'):
        require(digest(REPO / rel) == sha, f'current native source unchanged: {rel}')
binding = run['build_binding']
require(digest(Path(binding['binary'])) == run['binary_sha256'], 'actual same executable digest')
require(digest(Path(binding['build_receipt'])) == binding['build_receipt_sha256'], 'build receipt digest')
build = load(Path(binding['build_receipt']))
require(build['source'] == SHA and build['exit_code'] == 0 and build['source_file_hashes_unchanged'], 'successful immutable source build')
require(subprocess.check_output(['git', 'rev-parse', f'{SHA}^{{tree}}'], cwd=REPO, text=True).strip() == run['tree'], 'actual source tree')
manifest = load(RAW / 'MANIFEST.json')
for rel, meta in manifest.items():
    p = RAW / rel
    require(p.stat().st_size == meta['bytes'] and digest(p) == meta['sha256'], f'raw manifest {rel}')
for cmd in run['commands']:
    require(cmd['exit_code'] == 0 and not cmd['timed_out'], f'actual command result {cmd["label"]}')
    for stream in ('stdout', 'stderr'):
        require(digest(RAW / cmd['label'] / f'{stream}.txt') == cmd[f'{stream}_sha256'], f'command stream {cmd["label"]}')

test_counts = []
for label in ('mf-structural-unit-contracts', 'kernel-and-mf-contracts'):
    text = (RAW / label / 'stdout.txt').read_text()
    test_counts += [tuple(map(int, row)) for row in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)]
require(sum(x[0] for x in test_counts) == 19 and sum(x[1] for x in test_counts) == 0 and sum(x[2] for x in test_counts) == 2, '19 passed 2 ignored')
oracle = load(RAW / 'NATIVE_BIT_ORACLE.json')
require(oracle['result'] == 'PASS_SCOPED' and oracle['exact_checks'] == 88, 'retained independent oracle result')
prefix = 'STRUCTURAL_NATIVE_BITS '
line = next(s.split(prefix, 1)[1] for s in (RAW / 'kernel-and-mf-contracts/stdout.txt').read_text().splitlines() if prefix in s)
packet = json.loads(line)
require(packet['source_commit'] == SHA and len(packet['records']) == 8, 'actual native packet')
for row in packet['records']:
    if row['status'] == 'accepted':
        sigma = qbits(row['sigma_bits']); rhs = list(map(qbits, row['rhs_bits'])); x = list(map(qbits, row['candidate_bits'])); tau = qbits(row['threshold_bits'])
        residual, error = [], []
        for i in range(0, len(rhs), 2):
            u, v = x[i:i+2]; b, c = rhs[i:i+2]; D = 1 + 9*sigma*sigma
            residual += [b-u-3*sigma*v, c-v+3*sigma*u]
            error += [u-(b-3*sigma*c)/D, v-(c+3*sigma*b)/D]
        r2 = sum(t*t for t in residual); e2 = sum(t*t for t in error)
        require(r2 <= tau*tau and e2*(1+9*sigma*sigma) == r2, 'independently reconstructed dyadic residual/error')
    else:
        require(row['output_before_bits'] == row['output_after_bits'], 'actual refused output untouched')

protocol = json.loads(subprocess.check_output(['git', 'show', f'{SHA}:{NODE}/STRUCTURAL_PROTOCOL.json'], cwd=REPO))
trials = load(RAW / 'TRIALS.json'); by_label = {}
totals = {'probe': 0, 'warmup': 0, 'timed': 0}
cases = {}
max_error = 0.0
for trial in trials:
    label = trial['label']; payload = load(RAW / label / 'stdout.txt'); rows = payload['samples']; by_label[label] = rows
    case, arm, start = trial['case'], trial['arm'], trial['start']
    require(payload['tolerances'] == protocol['tolerances'] and payload['output_grid'] == protocol['output_grid'], 'fixed controls')
    kind = 'probe' if trial['mode'] == 'probe' else ('warmup' if '-warmup-' in label else 'timed')
    totals[kind] += len(rows)
    for row in rows:
        require(row['success'] and row['finite'] and row['gridpoint_count'] == 17 and row['max_abs_error'] <= 1e-5, 'every sample succeeds and meets fixed error cap')
        require(math.isfinite(row['max_abs_error']) and row['solve_plus_drop_ns'] == row['elapsed_ns'] + row['drop_ns'], 'finite diagnostic / timer sum')
        w, ps = row['work'], row['proposal_statistics']
        require(row['callback_counts']['rhs'] == w['rhs_calls'] and row['callback_counts']['jvp'] == w['jvp_vectors'] == w['jvp_calls'], 'actual callback/work agreement')
        require(w['linear_matvecs'] + w['diagnostic_matvecs'] == w['jvp_vectors'], 'all shifted actions accounted')
        require(row['strict_work_units'] == w['rhs_calls'] + w['jvp_vectors'] + w['ft_calls'] + w['linear_solves'] + w['jacobian_builds']*payload['dimension'], 'strict unit recomputation')
        require(w['linear_solves'] == row['attempts']*8 and row['attempts'] == row['accepted_steps']+row['rejected_steps'], 'eight stages per attempted step')
        if arm == 'proposal':
            require(ps['attempts'] == ps['accepted']+ps['refused'] == w['linear_solves'] and ps['callback_failures'] == 0 and ps['operator_actions'] <= w['jvp_vectors'], 'proposal attempt/fallback accounting')
        else:
            require(all(v == 0 for v in ps.values()), 'disabled arm has no proposal work')
        max_error = max(max_error, row['max_abs_error'])
        if kind != 'probe':
            reference = by_label[f'{case}-previous-probe-{arm}'][0]
            for key in ('work', 'proposal_statistics', 'attempts', 'accepted_steps', 'rejected_steps', 'callback_counts', 'strict_work_units', 'gmres_cycles', 'max_abs_error'):
                require(row[key] == reference[key], f'timed/warm full-solve numerical work unchanged from qualified probe: {case}/{arm}/{key}')
        if kind == 'probe':
            require(row['t'] == protocol['output_grid'] and len(row['solve_log']) == w['linear_solves'], 'actual requested grid and stage logs')
            independently_computed_error = 0.0
            for t, ts, values, bits in zip(row['t'], row['t_bits'], row['y'], row['y_bits'], strict=True):
                require(fbits(t) == int(ts, 16), 'time bit encoding')
                require(len(values) == len(bits) == payload['dimension'], 'state dimensions')
                for i, (y, bit) in enumerate(zip(values, bits, strict=True)):
                    if case == 'time-jvp':
                        target = math.exp(-t)*(1+i/16)
                    else:
                        amp = 1 + ((i//2)*2 % 16)/32
                        damp = 0.5 if case == 'damped-rotation' else 0
                        target = math.exp(-damp*t)*amp*(math.cos(3*t) if i%2 == 0 else math.sin(3*t))
                    require(math.isfinite(y) and fbits(y) == int(bit, 16), 'finite encoded observation')
                    independently_computed_error = max(independently_computed_error, abs(y-target))
            require(independently_computed_error <= 1e-5 and abs(independently_computed_error-row['max_abs_error']) <= 1e-15, 'independent analytic diagnostic recomputation')
    if kind == 'probe' and start == 'previous':
        row = rows[0]
        cases.setdefault(case, {})[arm] = {k:row[k] for k in ('max_abs_error','strict_work_units','callback_counts','proposal_statistics','gmres_cycles','attempts','accepted_steps','rejected_steps')}
        cases[case][arm]['mgs_dots'] = row['work']['orthogonalization_inner_products']
        cases[case][arm]['mgs_updates'] = row['work']['orthogonalization_vector_updates']
        cases[case][arm]['arnoldi_columns'] = row['work']['linear_iterations']
        cases[case][arm]['zero_iteration_stage_indices'] = [i for i,e in enumerate(row['solve_log']) if e['iterations'] == 0]

require(totals == {'probe':60, 'warmup':160, 'timed':2240}, '2460 registered solves')
pairs = load(RAW / 'PAIRED_TIMINGS.json'); registered_summaries = load(RAW / 'CASE_SUMMARY.json')
require(len(pairs) == 35, '35 paired blocks')
for case in cases:
    ratios = []
    for block in range(7):
        order = ['gmres','proposal','proposal','gmres'] if block%2 == 0 else ['proposal','gmres','gmres','proposal']
        medians = {'gmres':[], 'proposal':[]}
        for position, arm in enumerate(order):
            rows = by_label[f'{case}-b{block}-{position}-{arm}']
            medians[arm].append(statistics.median(row['solve_plus_drop_ns'] for row in rows))
        ratio = sum(medians['gmres']) / sum(medians['proposal']); ratios.append(ratio)
        stored = next(p for p in pairs if p['case'] == case and p['block'] == block)
        require(stored['order'] == order and stored['child_medians_ns'] == medians and stored['gmres_over_proposal_ratio'] == ratio, 'independent paired aggregation')
    require(registered_summaries[case]['ratios'] == ratios and registered_summaries[case]['median'] == statistics.median(ratios), 'published timing summary')
    cases[case]['timing'] = {'ratios':ratios,'median':statistics.median(ratios),'min':min(ratios),'max':max(ratios),'range_is_confidence_interval':False}
    cases[case]['strict_cost_ratio_proposal_over_gmres'] = cases[case]['proposal']['strict_work_units']/cases[case]['gmres']['strict_work_units']

require(cases['time-jvp']['proposal']['zero_iteration_stage_indices'] == list(range(7,144,8)), 'time-jvp zero-iteration log entries are eighth stage of all18steps')
for case in ('rotation','rotation-large','damped-rotation'):
    require(cases[case]['proposal']['proposal_statistics']['accepted'] == 144 and cases[case]['proposal']['mgs_dots'] == cases[case]['proposal']['arnoldi_columns'] == cases[case]['proposal']['gmres_cycles'] == 0, 'structural work elimination on admitted targets')
require(cases['badcoeff']['proposal']['proposal_statistics']['refused'] == 142, 'adverse refusal workload retained')
prior = load(ROOT/'lane-structural-checks01/RUN.json')
require(prior['checks_result'] == 'fail' and prior['benchmark_result'] == 'not_run', 'first Clippy failure not relabeled pass')
require(run['aggregate_campaign_wall_seconds'] == 491.53919015300016+prior['wall_seconds']+run['wall_seconds'] < 900, 'complete cumulative scientific budget')
result = {'review':'ACCEPT_SCOPED_RESULTS','source':SHA,'source_tree':run['tree'],'binary_sha256':run['binary_sha256'],'checks':checks,'native_tests':{'passed':19,'ignored':2,'new_methods':11},'recorded_fraction_checks':88,'independently_replayed_native_records':8,'registered_solves':totals,'total_solves':sum(totals.values()),'paired_blocks':35,'max_all_sample_grid_error':max_error,'cases':cases,'aggregate_campaign_wall_seconds':run['aggregate_campaign_wall_seconds'],'first_failed_check_run_retained':True,'whole_ivp_certificate':False,'production':'HOLD','scope':'independent retained-output/hash/aggregation review; no solver rerun','review_wall_seconds':time.perf_counter()-started,'input_manifest_sha256':digest(RAW/'MANIFEST.json'),'reviewer_script_sha256':digest(Path(__file__))}
(ROOT/'STRUCTURAL_RESULT_REVIEW.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({k:result[k] for k in ('review','source','checks','total_solves','paired_blocks','max_all_sample_grid_error','aggregate_campaign_wall_seconds','review_wall_seconds')},indent=2))
