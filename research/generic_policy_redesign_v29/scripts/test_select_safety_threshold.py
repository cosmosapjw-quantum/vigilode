from select_safety_threshold import select

def row(a, unsafe=False, family='f', dimension=96):
    return {
        'budget_admitted': True, 'prefix_succeeded': True, 'audit_full_e_completed': True,
        'normalized_stage_growth_a34': a,
        'audit_full_e_locally_admissible': not unsafe,
        'family': family, 'dimension': dimension,
    }

def test_selector_prefers_largest_zero_unsafe_coverage_then_shrinks():
    # 60 safe low-score events and one unsafe high-score event. The predeclared
    # multiplicity guard passes for the low-score set, and shrink stays safe.
    rows=[row(0.10 + i*0.01) for i in range(60)] + [row(1.2, unsafe=True)]
    result=select(rows)
    assert result['status']=='frozen-threshold'
    assert result['tau_selected'] < 1.2
    assert result['tau_final'] == 0.95*result['tau_selected']
    assert result['final']['unsafe']==0
    assert result['final']['passes']

def test_selector_fails_closed_when_support_is_too_small():
    rows=[row(0.8+i*0.001) for i in range(8)]
    result=select(rows)
    assert result['status']=='all-abstain'
    assert result['tau_final'] is None
