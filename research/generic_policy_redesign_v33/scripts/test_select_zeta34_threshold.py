import importlib.util
from pathlib import Path

MODULE = Path(__file__).with_name('select_zeta34_threshold.py')
spec = importlib.util.spec_from_file_location('selector', MODULE)
selector = importlib.util.module_from_spec(spec)
spec.loader.exec_module(selector)


def row(z, safe, family):
    return {
        'budget_admitted': True,
        'prefix_succeeded': True,
        'audit_full_e_completed': True,
        'audit_full_e_locally_admissible': safe,
        'quadratic_drift_zeta34': z,
        'family': family,
    }


def test_selects_max_zero_unsafe_then_applies_one_rank_margin():
    rows = [
        row(1, True, 'a'), row(2, True, 'b'), row(3, True, 'c'),
        row(4, True, 'a'), row(5, True, 'b'), row(6, True, 'c'),
        row(7, True, 'a'), row(8, True, 'b'), row(10, False, 'c'),
    ]
    result = selector.select(rows)
    assert result['status'] == 'frozen-threshold'
    assert result['tau_selected'] == 8
    assert result['tau_final'] == 7
    assert result['final']['recommended'] == 7
    assert result['final']['unsafe'] == 0
    assert result['final']['distinct_families'] == 3


def test_no_observed_unsafe_event_fails_closed():
    rows = [row(i, True, ['a','b','c'][i % 3]) for i in range(1, 10)]
    result = selector.select(rows)
    assert result['status'] == 'all-abstain-no-unsafe-boundary'
    assert result['tau_final'] is None


def test_family_concentration_fails_closed():
    rows = [row(i, True, 'a') for i in range(1, 9)] + [row(10, False, 'b')]
    result = selector.select(rows)
    assert result['status'] == 'all-abstain'
    assert result['tau_final'] is None
