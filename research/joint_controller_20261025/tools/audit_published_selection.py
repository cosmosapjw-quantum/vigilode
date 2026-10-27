#!/usr/bin/env python3
"""Recompute the published, bounded selection packet without executing a solver.

Checks saved values and manifest links. The large host-only raw data are not
available here; this program never upgrades their verification status.
"""
from __future__ import annotations

import argparse
from collections import defaultdict
from fractions import Fraction
import hashlib
import json
import math
from pathlib import Path
import statistics


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def analyze(root):
    packet_dir = root / 'external_reports/adapter_local_followup_20261025'
    names = ['SELECTION_WINDOWS.json', 'SELECTION_ANALYSIS.json',
             'NARROW_GROUP_SUMMARY.json', 'TRACE_SUMMARY.json', 'DECISIONS.json',
             'IDENTITY.json', 'AUDIT_RECEIPT.json', 'REPORT_KO.md',
             'RAW_MANIFEST_FINAL.json', 'ANALYTIC_REFERENCE_CROSSCHECK.json',
             'ENDPOINT_ANALYTIC_CROSSCHECK.json']
    data = {name: json.loads((packet_dir / name).read_text())
            for name in names if name.endswith('.json')}
    rows = data['SELECTION_WINDOWS.json']['rows']
    published = {(w['window_id'], w['paired_block_id']): w
                 for w in data['SELECTION_ANALYSIS.json']['windows']}
    narrow = {(w['cohort'], w['candidate']): w
              for w in data['NARROW_GROUP_SUMMARY.json']['cohorts']}
    manifest = {Path(f['path']).name: f['sha256']
                for f in data['RAW_MANIFEST_FINAL.json']['files']
                if f['type'] == 'regular'}
    groups = defaultdict(list)
    errors = []
    for row in rows:
        groups[(row['window_id'], row['paired_block_id'])].append(row)
    results = []
    receipt_links = 0
    for key, group in sorted(groups.items()):
        if len(group) != 2 or sum(r['selected'] for r in group) != 1:
            errors.append({'block': key, 'error': 'invalid_pair_shape'})
            continue
        chosen = next(r for r in group if r['selected'])
        challenger = next(r for r in group if not r['selected'])
        if chosen['identity'] != challenger['identity']:
            errors.append({'block': key, 'error': 'identity_mismatch'})
        for row in group:
            cap = Fraction(float.fromhex(row['identity']['error_ceiling_hex']))
            if (row['status'] != 'completed' or
                row['final_time_hex'] != row['identity']['t_end_hex'] or
                Fraction(row['physical_error']) + Fraction(row['reference_uncertainty']) > cap):
                errors.append({'block': key, 'error': 'ineligible_run'})
            cost = row['cost']
            parts = [cost[k] for k in ['solve_seconds', 'setup_seconds', 'probe_seconds', 'transition_seconds']]
            if (not cost['disjoint_components'] or
                any(not math.isfinite(x) or x < 0 for x in parts) or
                not math.isclose(math.fsum(parts), cost['total_seconds'], rel_tol=2e-12, abs_tol=2e-15)):
                errors.append({'block': key, 'error': 'invalid_cost'})
            summary = narrow[(row['window_id'], row['candidate_id'])]
            receipt = next(r for r in summary['receipts'] if str(r['block']) == key[1])
            if receipt['receipt_sha256'] != row['receipt_sha256'] or receipt['binary_sha256'] != row['binary_sha256']:
                errors.append({'block': key, 'error': 'receipt_link_mismatch'})
            if manifest.get(receipt['receipt']) != receipt['receipt_sha256'] or manifest.get(receipt['stdout']) != receipt['stdout_sha256']:
                errors.append({'block': key, 'error': 'manifest_link_mismatch'})
            else:
                receipt_links += 1
        c = chosen['cost']['total_seconds']
        a = challenger['cost']['total_seconds']
        ratio = c / min(c, a)
        old = published[key]
        if old['status'] != 'COMPARABLE' or not math.isclose(ratio, old['observed_cost_ratio'], rel_tol=1e-14):
            errors.append({'block': key, 'error': 'published_ratio_mismatch'})
        results.append({'cohort': key[0], 'block': key[1],
                        'selected': chosen['candidate_id'], 'challenger': challenger['candidate_id'],
                        'selected_seconds': c, 'challenger_seconds': a,
                        'selected_over_best': ratio, 'selected_over_challenger': c / a,
                        'selected_more_expensive': c > a})
    cohorts = []
    for name in sorted({r['cohort'] for r in results}):
        rr = [r for r in results if r['cohort'] == name]
        selected_name, alternative_name = rr[0]['selected'], rr[0]['challenger']
        chosen = narrow[(name, selected_name)]
        alternative = narrow[(name, alternative_name)]
        med_selected = statistics.median(r['selected_seconds'] for r in rr)
        med_alternative = statistics.median(r['challenger_seconds'] for r in rr)
        for candidate, median in [(chosen, med_selected), (alternative, med_alternative)]:
            if candidate['solver_timing_seconds']['median'] != median:
                errors.append({'cohort': name, 'error': 'summary_median_mismatch'})
        cohorts.append({'cohort': name,
                        'selection_kind': 'declared_specialization' if name == 'chirp_grid' else 'automatic_wrapper',
                        'blocks': len(rr),
                        'selected_more_expensive_blocks': sum(r['selected_more_expensive'] for r in rr),
                        'selected': selected_name, 'challenger': alternative_name,
                        'selected_median_seconds': med_selected,
                        'challenger_median_seconds': med_alternative,
                        'ratio_of_medians': med_selected / med_alternative,
                        'median_paired_ratio_to_best': statistics.median(r['selected_over_best'] for r in rr),
                        'min_paired_ratio_to_best': min(r['selected_over_best'] for r in rr),
                        'max_paired_ratio_to_best': max(r['selected_over_best'] for r in rr),
                        'selected_rhs_per_run': chosen['rhs_calls_per_run']['median'],
                        'challenger_rhs_per_run': alternative['rhs_calls_per_run']['median'],
                        'selected_attempts_per_run': chosen['attempts_per_run']['median'],
                        'challenger_attempts_per_run': alternative['attempts_per_run']['median'],
                        'selected_max_observed_error': chosen['max_observed_physical_error'],
                        'challenger_max_observed_error': alternative['max_observed_physical_error'],
                        'physical_error_cap': chosen['physical_error_cap'],
                        'reference_uncertainty': chosen['reference_uncertainty_bound']})
    receipt = data['AUDIT_RECEIPT.json']
    if receipt['reviewed_report_sha256'] != sha(packet_dir / 'REPORT_KO.md'):
        errors.append({'error': 'independent_review_report_hash_mismatch'})
    automatic = [r for r in results if r['cohort'] != 'chirp_grid']
    return {
        'schema': 'vigilode.joint-controller.published-evidence-audit.v1',
        'publication_commit': '4fe67602502440d76aa9dd245c6b1d67558c14bf',
        'publication_tree': '8bab954cffcf7abd9cdd0ca528567ece47e43bc7',
        'measurement_source_commit': data['IDENTITY.json']['source_commit'],
        'measurement_source_tree': data['IDENTITY.json']['source_tree'],
        'audit_scope': 'published values, exact reported-error admission, aggregate arithmetic and manifest link consistency',
        'host_raw_bytes_reverified': False,
        'raw_verification_status': 'reuse prior source-locked local attestation; raw remains on user host',
        'new_ode_runs': 0,
        'inputs': {str((packet_dir / n).relative_to(root)): sha(packet_dir / n) for n in names},
        'checks': {'rows': len(rows), 'paired_blocks': len(results), 'receipt_manifest_links': receipt_links,
                   'error_count': len(errors), 'errors': errors,
                   'result': 'PASS_SCOPED' if not errors else 'FAIL'},
        'denominators': {
            'automatic_wrapper_comparisons': len(automatic),
            'automatic_wrapper_more_expensive': sum(r['selected_more_expensive'] for r in automatic),
            'distinct_automatic_problems': len({r['cohort'] for r in automatic}),
            'declared_chart_comparisons': len(results) - len(automatic),
            'independent_problem_draws': False,
            'per_decision_false_positive_rate': None,
            'reason_rate_not_identified': 'whole-trajectory wrapper costs do not label each method decision; repeated timing blocks share the same problems and no calibrated stiffness truth labels are supplied',
            'future_evaluation_unit': 'same accepted checkpoint, matched physical horizon and error cap; actual selected route plus one admitted challenger; separate probe trigger and actual route choice'},
        'timing_authority': 'diagnostic_only',
        'cost_semantics': data['NARROW_GROUP_SUMMARY.json']['cost_component_policy'],
        'cohorts': cohorts,
        'blocks': results,
        'trace_summary_reused': data['TRACE_SUMMARY.json'],
        'production_decision': 'HOLD',
        'production_remaining': [
            'reduce repeated pilot and wrapper overhead while retaining late-transition detection',
            'preserve or account for implicit-session restart costs',
            'bind general structural providers to full physical error and current model/chart/operator epochs',
            'qualify paired per-decision costs and accuracy on unseen transitions',
            'connect large-grid callbacks to the integrated solver before claiming PDE parallel speed',
            'complete broader solver validation on a source-locked release candidate']}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--repo', type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = analyze(args.repo)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('x') as stream:
        json.dump(result, stream, ensure_ascii=False, indent=2, allow_nan=False)
        stream.write('\n')
    print(json.dumps(result['checks'], ensure_ascii=False))
    return int(bool(result['checks']['error_count']))


if __name__ == '__main__':
    raise SystemExit(main())
