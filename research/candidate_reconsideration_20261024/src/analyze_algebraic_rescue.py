#!/usr/bin/env python3
"""Parse current refinement probe, retaining failed attempts and timing scope."""
import json
import pathlib
import statistics
import sys

base = pathlib.Path(__file__).resolve().parents[1]
source = (pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else base / 'evidence/runs/algebraic-benchmark-corrected.stdout').resolve()
rows = [json.loads(line) for line in source.read_text().splitlines() if line.strip()]
assert len(rows) == 24
kernels = [row for row in rows if row['kind'] == 'linear_system_kernel']
trajectories = [row for row in rows if row['kind'] == 'implicit_euler_trajectory']
assert len(kernels) == 21 and len(trajectories) == 3
for row in rows:
    result = row['result']
    if row['method'] == 'legacy_single_quotient':
        assert result['accepted'] is False
        assert result.get('polynomial_operator_actions', result.get('operator_actions')) == 1
    else:
        assert result['accepted'] is True
    expected = 21 if row['kind'] == 'linear_system_kernel' else 11
    assert len(row['samples_seconds']) == expected
    assert all(sample > 0 for sample in row['samples_seconds'])

summaries = []
for n in [2, 32, 128, 256]:
    matching = {row['method']: row for row in kernels if row['dimension'] == n}
    medians = {name: statistics.median(row['samples_seconds']) for name, row in matching.items()}
    best = min((name for name in medians if name.startswith('dense_lu') or name.startswith('specialized_direct')), key=medians.get)
    refined = matching['refined_workspace_reused']['result']
    assert refined['absolute_error'] <= 1e-12 and refined['residual'] <= 1e-10
    summaries.append({
        'dimension': n, 'median_seconds': medians,
        'cold_lu_over_cold_refined': medians['dense_lu_with_setup'] / medians['refined_with_setup'],
        'reused_lu_over_reused_refined': medians['dense_lu_factor_reused'] / medians['refined_workspace_reused'],
        'best_baseline': best,
        'best_baseline_over_reused_refined': medians[best] / medians['refined_workspace_reused'],
        'refined_result': refined,
        'legacy_refused_residual': matching['legacy_single_quotient']['result']['residual'],
    })
trajectory = {row['method']: row for row in trajectories}
refined = trajectory['refined_workspace_reused']
direct = trajectory['dense_lu_factor_reused']
assert refined['result']['completed_steps'] == 50
assert refined['result']['same_discrete_method_error'] <= 1e-11
result = {
    'schema_version': 1, 'source': str(source.relative_to(base)),
    'row_count': len(rows), 'accepted_rows': sum(row['result']['accepted'] for row in rows),
    'legacy_refusals_preserved': 5, 'kernel_summary': summaries,
    'trajectory_summary': {
        'same_method_error': refined['result']['same_discrete_method_error'],
        'continuous_error_refined': refined['result']['continuous_exact_error'],
        'continuous_error_direct': direct['result']['continuous_exact_error'],
        'refined_steps': refined['result']['completed_steps'],
        'refined_polynomial_operator_actions': refined['result']['operator_actions'],
        'refined_max_residual': refined['result']['max_original_residual'],
        'reused_lu_over_refined_median': statistics.median(direct['samples_seconds']) / statistics.median(refined['samples_seconds']),
        'timing_scope': '11 serial samples per method; illustrative only, not pooled with 21 cyclic kernel groups',
    },
    'decision': 'Specialized opt-in kernel revived for approximate relations. Fresh-factorization crossover exists in this probe; reused LU/direct blocks remain stronger controls. No general automatic routing or RODAS trajectory speed promotion.',
    'limitations': [
        'One process on one host; grouped timings are not independent-process confidence intervals.',
        'JSON construction and validation diagnostics included as stated in raw rows.',
        'Counter covers polynomial solver only; LU arithmetic and extra diagnostic actions are included in elapsed time but not that counter.',
        'Manufactured dense near-scalar family uses an explicitly known full-space relation bound; arbitrary nonlinear RHS is outside this contract.',
        'Fixed-step implicit Euler trajectory is an integration client, not adaptive Rosenbrock validation.',
    ],
}
(base / 'analysis/ALGEBRAIC_BENCHMARK.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
print(json.dumps({'rows': len(rows), 'accepted': result['accepted_rows'], 'legacy_refusals': 5, 'output': 'analysis/ALGEBRAIC_BENCHMARK.json'}))
