#!/usr/bin/env python3
"""Independent raw-output reaggregation; never imports or runs a native solver."""
from __future__ import annotations
import argparse
import hashlib
import json
import math
from pathlib import Path
import re
import statistics
import struct
import sys
import time

CASES = ('rotation', 'rotation-large', 'damped-rotation', 'time-jvp', 'badcoeff')
DIMENSIONS = dict(zip(CASES, (16, 4096, 16, 16, 16)))
ARMS = ('gmres', 'proposal')
GRID = [i / 64 for i in range(17)]
TOLERANCES = dict(rtol=1e-7, atol=1e-10, linear_rtol=1e-10, linear_atol=1e-14, h0=1e-3)
FIXED = ('work', 'callback_counts', 'proposal_statistics', 'strict_work_units', 'attempts',
         'accepted_steps', 'rejected_steps', 'gmres_cycles')
TEST_RE = re.compile(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;')


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def unique_pairs(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f'duplicate JSON key {key}')
        result[key] = value
    return result


def reject_constant(value):
    raise ValueError(f'nonfinite JSON token {value}')


def write(path, data):
    Path(path).write_text(json.dumps(data, ensure_ascii=False, indent=2, allow_nan=False) + '\n')


def finite(value):
    return type(value) in (int, float) and math.isfinite(value)


def nonnegative_int(value):
    return type(value) is int and value >= 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bench', type=Path, required=True)
    parser.add_argument('--checks', type=Path, required=True)
    parser.add_argument('--earlier-checks', type=Path, action='append', default=[])
    parser.add_argument('--expected-source', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.bench = args.bench.resolve()
    args.checks = args.checks.resolve()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    started, cpu = time.perf_counter(), time.process_time()
    inputs = {}
    assertions = 0

    def check(condition, message):
        nonlocal assertions
        assertions += 1
        if not condition:
            raise AssertionError(message)

    def read(path):
        path = Path(path)
        raw = path.read_bytes()
        inputs[str(path.resolve())] = {'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()}
        return json.loads(raw, parse_constant=reject_constant, object_pairs_hook=unique_pairs)

    def text(path):
        path = Path(path)
        raw = path.read_bytes()
        inputs[str(path.resolve())] = {'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()}
        return raw.decode('utf-8')

    try:
        run = read(args.bench / 'RUN.json')
        check(run['source'] == args.expected_source, 'wrong benchmark source')
        check(run['execution'] == 'completed', 'benchmark is not completed; no partial result accepted')
        check(run['benchmark_result'] == 'pass_scoped_all_registered_cases', 'benchmark not qualified')
        check(run['source_hashes_unchanged'] is True, 'source mutation')
        check(read(args.bench / 'SOURCE_HASHES_BEFORE.json') == read(args.bench / 'SOURCE_HASHES_AFTER.json'), 'source hashes differ')
        check(run['aggregate_campaign_wall_seconds'] == run['used_wall_seconds_before_this_run'] + run['wall_seconds'], 'run wall accounting mismatch')
        check(run['aggregate_campaign_wall_seconds'] <= 900, 'scientific cap exceeded')
        commands = {row['label']: row for row in run['commands']}
        check(len(commands) == len(run['commands']), 'duplicate command label')
        for label, row in commands.items():
            receipt = read(args.bench / label / 'RECEIPT.json')
            check(receipt == row, f'command receipt mismatch: {label}')
            check(row['exit_code'] == 0 and row['timed_out'] is False, f'failed command: {label}')
            for stream in ('stdout', 'stderr'):
                path = args.bench / label / f'{stream}.txt'
                check(digest(path) == row[f'{stream}_sha256'], f'{stream} receipt digest: {label}')
        manifest = read(args.bench / 'MANIFEST.json')
        for relative, item in manifest.items():
            path = args.bench / relative
            check(path.is_file() and path.stat().st_size == item['bytes'] and digest(path) == item['sha256'], f'raw manifest mismatch: {relative}')
        checkrun = read(args.checks / 'RUN.json')
        check(checkrun['source'] == args.expected_source, 'checks source differs')
        check(checkrun['checks_result'] == 'pass_scoped', 'checks not passed')
        counts = dict(passed=0, failed=0, ignored=0, measured=0, filtered=0)
        test_rows = []
        for label in ('mf-structural-unit-contracts', 'kernel-and-mf-contracts'):
            stdout = text(args.checks / label / 'stdout.txt')
            receipt = read(args.checks / label / 'RECEIPT.json')
            check(receipt['exit_code'] == 0, f'test command failed: {label}')
            check(receipt['stdout_sha256'] == digest(args.checks / label / 'stdout.txt'), 'test stdout digest')
            matches = TEST_RE.findall(stdout)
            check(bool(matches), f'no actual test result: {label}')
            for status, *values in matches:
                row = dict(zip(counts, map(int, values)))
                check(status == 'ok' and row['passed'] > 0 and row['failed'] == 0, 'nonpassing/zero native tests')
                test_rows.append(dict(command=label, **row))
                for key in counts:
                    counts[key] += row[key]
        check(counts['passed'] == 19 and counts['ignored'] == 2 and counts['failed'] == 0, 'registered actual native test inventory changed')
        oracle = read(args.checks / 'NATIVE_BIT_ORACLE.json')
        check(oracle['source_commit'] == args.expected_source and oracle['result'] == 'PASS_SCOPED' and oracle['exit_code'] == 0, 'oracle identity/verdict')
        check((oracle['exact_checks'], oracle['accepted_records'], oracle['refused_records']) == (88, 6, 2), 'oracle record/check inventory')
        check(oracle['input_sha256'] == digest(args.checks / 'kernel-and-mf-contracts/stdout.txt'), 'oracle input digest')
        retained_checks = []
        for directory in args.earlier_checks:
            prior = read(directory / 'RUN.json')
            retained_checks.append({key: prior.get(key) for key in ('source', 'tree', 'checks_result', 'benchmark_result', 'execution', 'wall_seconds', 'failure')})
            retained_checks[-1]['commands'] = [{key: row.get(key) for key in ('label', 'exit_code', 'wall_seconds', 'timed_out')} for row in prior['commands']]
            retained_checks[-1]['path'] = str(directory.resolve())
        inherited = run['inherited_phase1_wall_seconds']
        independently_charged = inherited + sum(row['wall_seconds'] for row in retained_checks) + run['wall_seconds']
        check(abs(independently_charged - run['aggregate_campaign_wall_seconds']) <= 1e-9, 'earlier structural charge not independently reproduced')
        exported_trials = read(args.bench / 'TRIALS.json')
        trials_by_label = {row['label']: row for row in exported_trials}
        check(len(trials_by_label) == len(exported_trials) == 170, 'trial/command inventory')
        exported_pairs = read(args.bench / 'PAIRED_TIMINGS.json')
        pairs_by_key = {(row['case'], row['block']): row for row in exported_pairs}
        check(len(pairs_by_key) == len(exported_pairs) == 35, 'pair inventory')
        exported_cases = read(args.bench / 'CASE_SUMMARY.json')
        check(set(exported_cases) == set(CASES), 'case inventory')
        sample_counts = dict(correctness=0, warmup=0, timed=0)
        total_error = 0.0
        profiles, results, derived_pairs = {}, {}, []
        visited = set()

        def payload(label, case, arm, start, mode, repeats, population):
            nonlocal total_error
            check(label not in visited, 'duplicate raw trial read')
            visited.add(label)
            check(label in commands, f'missing receipt: {label}')
            data = read(args.bench / label / 'stdout.txt')
            expected = dict(schema='directional-quadratic-full-ivp-v1', case=case, dimension=DIMENSIONS[case], arm=arm,
                            start=start, mode=mode, repeats=repeats, span=0.25, output_grid=GRID,
                            tolerances=TOLERANCES, physical_error_cap=1e-5, solve_log_enabled=(mode == 'probe'))
            for key, value in expected.items():
                check(data[key] == value, f'fixture metadata {label}.{key}')
            samples = data['samples']
            check(len(samples) == repeats, f'sample count {label}')
            for row in samples:
                check(row['success'] is True and row['finite'] is True and row['gridpoint_count'] == 17, f'failed sample {label}')
                check(finite(row['max_abs_error']) and 0 <= row['max_abs_error'] <= 1e-5, f'accuracy gate {label}')
                check(finite(row['scaled_rms_error']) and row['scaled_rms_error'] >= 0, f'scaled error {label}')
                check(all(nonnegative_int(row[k]) for k in ('elapsed_ns', 'drop_ns', 'solve_plus_drop_ns', 'attempts', 'accepted_steps', 'rejected_steps', 'strict_work_units', 'gmres_cycles')), f'integer metrics {label}')
                check(row['solve_plus_drop_ns'] == row['elapsed_ns'] + row['drop_ns'], f'timer sum {label}')
                check(row['attempts'] == row['accepted_steps'] + row['rejected_steps'], f'outer-attempt accounting {label}')
                check(all(nonnegative_int(x) for field in ('work', 'callback_counts', 'proposal_statistics') for x in row[field].values()), f'counter values {label}')
                ps = row['proposal_statistics']
                check(ps['attempts'] == ps['accepted'] + ps['refused'] and ps['callback_failures'] == 0, f'proposal accounting {label}')
                check(row['callback_counts']['jvp'] == row['work']['jvp_calls'], f'actual/counted JVP mismatch {label}')
                w = row['work']
                strict = w['rhs_calls'] + w['jvp_vectors'] + w['ft_calls'] + w['linear_solves'] + w['jacobian_builds'] * DIMENSIONS[case]
                check(strict == row['strict_work_units'], f'strict work units {label}')
                if mode == 'probe':
                    check(row['t'] == GRID and len(row['y']) == 17, f'probe grid {label}')
                    check(len(row['t_bits']) == len(row['y_bits']) == 17, f'bit grid {label}')
                    for t, bits in zip(row['t'], row['t_bits']):
                        check(struct.pack('>d', t).hex() == bits, f'time bit encoding {label}')
                    for values, bits in zip(row['y'], row['y_bits']):
                        check(len(values) == len(bits) == DIMENSIONS[case], f'output dimensions {label}')
                        check(all(finite(x) for x in values), f'nonfinite output {label}')
                        check(all(struct.pack('>d', x).hex() == bit for x, bit in zip(values, bits)), f'value bit encoding {label}')
                    check(sum(entry['iterations'] for entry in row['solve_log']) == w['linear_iterations'], f'Arnoldi logged/counted mismatch {label}')
                else:
                    check(row['max_logged_relative_residual'] is None, f'timed logging overhead {label}')
                total_error = max(total_error, row['max_abs_error'])
            sample_counts[population] += repeats
            expected_trial = dict(label=label, case=case, arm=arm, start=start, mode=mode, repeats=repeats,
                max_grid_error=max(row['max_abs_error'] for row in samples),
                strict_work_units=[row['strict_work_units'] for row in samples],
                callback_counts=[row['callback_counts'] for row in samples],
                proposal_statistics=[row['proposal_statistics'] for row in samples],
                gmres_cycles=[row['gmres_cycles'] for row in samples],
                accepted_steps=[row['accepted_steps'] for row in samples],
                rejected_steps=[row['rejected_steps'] for row in samples],
                solve_plus_drop_ns=[row['elapsed_ns'] + row['drop_ns'] for row in samples])
            check(trials_by_label[label] == expected_trial, f'exported TRIALS differs: {label}')
            return data

        for case in CASES:
            profiles[case] = {}
            initial = {}
            for start in ('zero', 'previous'):
                for arm in ARMS:
                    data = payload(f'{case}-{start}-probe-{arm}', case, arm, start, 'probe', 3, 'correctness')
                    rows = data['samples']
                    for row in rows[1:]:
                        check(all(row[key] == rows[0][key] for key in FIXED), f'probe repeat counters differ: {case}/{arm}/{start}')
                        check(row['y_bits'] == rows[0]['y_bits'] and row['max_abs_error'] == rows[0]['max_abs_error'], f'probe repeat results differ: {case}/{arm}/{start}')
                    initial[arm] = rows[0]['t_bits'][0], rows[0]['y_bits'][0]
                    if start == 'previous':
                        profile = {key: rows[0][key] for key in FIXED}
                        profile.update(dimension=DIMENSIONS[case], max_abs_error=rows[0]['max_abs_error'],
                            max_logged_relative_residual=rows[0]['max_logged_relative_residual'],
                            proposal_workspace_bytes=rows[0]['proposal_workspace_bytes'], krylov_workspace_bytes=rows[0]['krylov_workspace_bytes'],
                            qr_least_squares_solves_derived_from_gmres_cycles=rows[0]['gmres_cycles'],
                            qr_derivation='ordinary GMRES-into returns least_squares_solves=cycles; no independent hardware/function QR counter',
                            fixed_counters_identical_in_three_repeats=True)
                        profiles[case][arm] = profile
                check(initial['gmres'] == initial['proposal'], f'initial bits differ: {case}/{start}')
            pool = {arm: [] for arm in ARMS}
            for arm in ARMS:
                data = payload(f'{case}-warmup-{arm}', case, arm, 'previous', 'timing', 16, 'warmup')
                for row in data['samples']:
                    check(all(row[key] == profiles[case][arm][key] for key in FIXED), f'warmup work differs from probe {case}/{arm}')
            ratios = []
            for block in range(7):
                order = ['gmres', 'proposal', 'proposal', 'gmres'] if block % 2 == 0 else ['proposal', 'gmres', 'gmres', 'proposal']
                medians = {arm: [] for arm in ARMS}
                for position, arm in enumerate(order):
                    data = payload(f'{case}-b{block}-{position}-{arm}', case, arm, 'previous', 'timing', 16, 'timed')
                    for row in data['samples']:
                        check(all(row[key] == profiles[case][arm][key] for key in FIXED), f'timed work differs from probe {case}/{arm}')
                    values = sorted(row['elapsed_ns'] + row['drop_ns'] for row in data['samples'])
                    median = (values[7] + values[8]) / 2
                    medians[arm].append(median)
                    pool[arm].append(median)
                check(sum(medians['proposal']) > 0, 'nonpositive duration')
                ratio = sum(medians['gmres']) / sum(medians['proposal'])
                ratios.append(ratio)
                pair = dict(case=case, block=block, order=order, child_medians_ns=medians, gmres_over_proposal_ratio=ratio)
                check(pair == pairs_by_key[(case, block)], f'paired timing export mismatch {case}/{block}')
                derived_pairs.append(pair)
            summary = exported_cases[case]
            check(summary['completed_blocks'] == 7 and summary['ratios'] == ratios, f'case block export {case}')
            for key, value in (('median', statistics.median(ratios)), ('minimum', min(ratios)), ('maximum', max(ratios))):
                check(summary[key] == value, f'case aggregate export {case}/{key}')
            check(summary['range_is_confidence_interval'] is False and summary['timing_authority'] == 'diagnostic_only', 'inference scope changed')
            results[case] = dict(dimension=DIMENSIONS[case], ratios=ratios, ratio_median=statistics.median(ratios), ratio_min=min(ratios), ratio_max=max(ratios),
                faster_blocks=sum(value > 1 for value in ratios), slower_blocks=sum(value < 1 for value in ratios),
                child_medians_ns=pool, median_of_14_child_medians_ns={arm: statistics.median(pool[arm]) for arm in ARMS},
                gmres_over_proposal_jvp_ratio=profiles[case]['gmres']['callback_counts']['jvp'] / max(1, profiles[case]['proposal']['callback_counts']['jvp']))
        check(visited == set(trials_by_label), 'unexpected or missing trial output')
        check(sample_counts == dict(correctness=60, warmup=160, timed=2240), 'actual solve inventory')
        check(profiles['badcoeff']['proposal']['proposal_statistics']['refused'] > 0, 'adverse wrong-coefficient control has no paid refusal')
        result = dict(schema='vigilode.independent-structural-analysis.v1', result='PASS_SCOPED_RAW_REAGGREGATION', source=args.expected_source,
            benchmark_tree=run['tree'], bench_directory=str(args.bench), checks_directory=str(args.checks),
            binary_sha256=run['binary_sha256'], source_hashes_unchanged=True, assertions=assertions,
            native_tests=dict(total=counts, target_summaries=test_rows, unique_vs_rerun='current-source execution counts; earlier source runs not added'),
            fraction_oracle={key: oracle[key] for key in ('exact_checks', 'accepted_records', 'refused_records', 'source_commit', 'input_sha256', 'physical_ivp_certificate')},
            solve_counts=dict(**sample_counts, total=sum(sample_counts.values())), max_all_grid_error=total_error,
            cases=results, previous_start_profiles=profiles,
            cost=dict(inherited_phase1_seconds=inherited, retained_earlier_structural_seconds=sum(row['wall_seconds'] for row in retained_checks),
                current_structural_seconds=run['wall_seconds'], aggregate_campaign_seconds=independently_charged, cap_seconds=900,
                cumulative_child_maxrss_kib=max(row['cumulative_child_maxrss_kib'] for row in commands.values()),
                child_cpu_seconds=sum(row['child_cpu_seconds'] for row in commands.values()),
                note='maxRSS is cumulative child high-water mark; not a per-solver incremental allocation measurement'),
            retained_earlier_checks=retained_checks,
            scope=dict(accuracy='same binary64 analytic17-grid-point1e-5 target; not whole-slab enclosure', timing='full integration+workspace construction+owned drop; audit/JSON/model setup excluded',
                counters='MGS counters are existing inner-products/vector-updates, not total FLOPs; QR/LS inferred from ordinary GMRES cycles', production='HOLD', intervals='min/max are all7 block ranges, not confidence intervals'),
            exported_trials_and_pairs_exactly_reproduced=True)
        write(out / 'ANALYSIS.json', result)
        write(out / 'RECONSTRUCTED_PAIRS.json', derived_pairs)
        lines = ['# Structural quadratic raw reaggregation', '', f"Source `{args.expected_source}`. {sum(sample_counts.values())} IVP solves:60 correctness,160 warmup,2240 timed;19 native tests PASS,2 ignored;88 exact Fraction checks over6 accepted/2 refused kernel records.", '',
            '| Case | n | GMRES median µs | Proposal median µs | GMRES/proposal median [min,max] | Faster blocks |',
            '|---|---:|---:|---:|---:|---:|']
        for case, item in results.items():
            durations = item['median_of_14_child_medians_ns']
            lines.append(f"| {case} | {item['dimension']} | {durations['gmres']/1000:.3f} | {durations['proposal']/1000:.3f} | {item['ratio_median']:.6f} [{item['ratio_min']:.6f},{item['ratio_max']:.6f}] | {item['faster_blocks']}/7 |")
        lines += ['', 'Durations are medians of14 child medians per arm; paired ratios use the two child-median sums within each ABBA/BAAB block. Ranges are not confidence intervals.', '',
            '| Case | JVP old→new | MGS inner products old→new | MGS updates old→new | Arnoldi columns old→new | QR/LS cycles old→new | Proposals accept/refuse | Outer attempts old→new | Grid max error old→new |',
            '|---|---:|---:|---:|---:|---:|---:|---:|---:|']
        for case, arms in profiles.items():
            old, new = arms['gmres'], arms['proposal']; a, b = old['work'], new['work']; ps = new['proposal_statistics']
            lines.append(f"| {case} | {old['callback_counts']['jvp']}→{new['callback_counts']['jvp']} | {a['orthogonalization_inner_products']}→{b['orthogonalization_inner_products']} | {a['orthogonalization_vector_updates']}→{b['orthogonalization_vector_updates']} | {a['linear_iterations']}→{b['linear_iterations']} | {old['gmres_cycles']}→{new['gmres_cycles']} | {ps['accepted']}/{ps['refused']} | {old['attempts']}→{new['attempts']} | {old['max_abs_error']:.6g}→{new['max_abs_error']:.6g} |")
        lines += ['', 'Work values are one complete previous-start probe solve; all three repeats agree, and every warm/timed sample has the same counters. QR/LS count is derived from reported GMRES cycles for this ordinary GMRES-into path; there is no separate hardware/function QR counter. New O(n) loops are included in wall time but not counted as zero FLOPs.', '',
            f"All-grid diagnostic maximum: {total_error:.12g}; campaign scientific wall: {independently_charged:.9f}/900s including the retained earlier failed checks. Production HOLD.", '']
        (out / 'TABLES.md').write_text('\n'.join(lines))
        exit_code = 0
    except BaseException as exc:
        write(out / 'ANALYSIS_FAILURE.json', dict(result='FAIL', error_type=type(exc).__name__, error=str(exc), assertions=assertions))
        exit_code = 1
    finally:
        write(out / 'INPUTS.json', inputs)
        write(out / 'ANALYSIS_RECEIPT.json', dict(argv=sys.argv, exit_code=exit_code if 'exit_code' in locals() else 1,
            wall_seconds=time.perf_counter()-started, cpu_seconds=time.process_time()-cpu,
            script_sha256=digest(__file__), source=args.expected_source, native_executions=0, assertions=assertions))
    if exit_code == 0:
        print((out / 'TABLES.md').read_text())
    else:
        print((out / 'ANALYSIS_FAILURE.json').read_text(), file=sys.stderr)
    return exit_code

if __name__ == '__main__':
    raise SystemExit(main())
