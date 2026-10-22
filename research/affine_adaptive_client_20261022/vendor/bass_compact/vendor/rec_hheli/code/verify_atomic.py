"""Small finite-model checks; no cosmological ODE solve or performance benchmark."""
from decimal import Decimal, localcontext
import hashlib
import json
import math
from pathlib import Path
import sys

import numpy as np
from scipy.linalg import expm

from atomic_network import AtomicNetwork, ROOT, EV_J


def relative_max(a, b, floor=1e-290):
    a, b = np.asarray(a), np.asarray(b)
    return float(np.max(np.abs(a - b)) / max(float(np.max(np.abs(a))),
                                            float(np.max(np.abs(b))), floor))


def decimal_li_shell(energy, shell, contract):
    """Independent log-form, 50-digit evaluation of the pinned provider fit."""
    if energy < (64.39 if shell == '1s' else 5.392):
        return 0.0
    with localcontext() as context:
        context.prec = 50
        d = Decimal
        e = d(str(energy))
        if shell == '1s' or energy >= 64.39:
            _, e0, sigma0, ya, p, yw = map(lambda v: d(str(v)), contract['PH1_' + shell])
            y = e / e0
            polynomial = (y - 1)**2 + yw**2
        else:
            e0, sigma0, ya, p, yw, y0, y1 = map(lambda v: d(str(v)), contract['PH2_2s'])
            x = e / e0 - y0
            y = (x*x + y1*y1).sqrt()
            polynomial = (x - 1)**2 + yw**2
        value = (sigma0.ln() + polynomial.ln() + (p / 2 - d('5.5')) * y.ln()
                 - p * (1 + (y / ya).sqrt()).ln()).exp()
        return float(value)


def main():
    net = AtomicNetwork()
    checks, details = {}, {}
    checks['source_hashes'] = True  # constructor verifies all 10 pinned data/source files
    checks['21_distinct_states'] = len(net.names) == len(set(net.names)) == 21
    checks['initial_baryon_normalization'] = abs(float(net.A @ net.initial_Y) - 1) < 5e-16
    checks['initial_n_T_zero'] = bool(np.all(net.initial_Y[[0, 5, 6]] == 0))
    checks['positive_scales'] = bool(np.all(net.scales > 0))

    generator_metrics = []
    for temperature in (100.0, 1e4, 1e6):
        rates = net.rates(temperature, 1.0)
        for name in ('G0', 'Gne', 'D'):
            matrix = rates[name]
            offdiag = matrix.copy()
            np.fill_diagonal(offdiag, 0.0)
            scale = max(float(np.max(np.abs(matrix))), 1e-290)
            col = float(np.max(np.abs(np.ones(21) @ matrix))) / scale
            baryon = float(np.max(np.abs(net.A @ matrix))) / scale
            row = dict(temperature_K=temperature, matrix=name,
                       min_offdiagonal=float(offdiag.min()),
                       relative_column_sum=col, relative_baryon_residual=baryon)
            generator_metrics.append(row)
    checks['positive_conservative_generators'] = all(
        r['min_offdiagonal'] >= 0 and r['relative_column_sum'] < 3e-15
        and r['relative_baryon_residual'] < 3e-15 for r in generator_metrics)
    details['generator_metrics'] = generator_metrics
    rates = net.rates(1e4, 1.)
    checks['HI_normalization'] = abs(rates['photo_s'][1] / 1e-12 - 1) < 3e-15
    checks['all_ionization_steps_positive_under_200eV'] = bool(
        np.all(rates['photo_s'][net.photo_indices] > 0))
    checks['bare_ion_DR_structural_zero'] = bool(
        np.all(rates['dr_m3_s'][(net.Z > 0) & (net.Z == net.q)] == 0))
    checks['elemental_isotope_equality'] = all(
        np.array_equal(rates[key][list(net.isotope_groups[a])],
                       rates[key][list(net.isotope_groups[b])])
        for key in ('photo_s', 'rr_m3_s', 'dr_m3_s')
        for a, b in (('H1', 'D'), ('H1', 'T'), ('He3', 'He4'), ('Li6', 'Li7')))

    contract = net.coefficients['LiI_partial']
    shell_rows = []
    for energy in (5.391, 5.392, 13.6, 40., np.nextafter(64.39, -math.inf),
                   64.39, 80., 91.83842857919419, 122.5, 200.):
        sigma, inner = net.photo_cross_sections([energy])
        native = {'1s': inner[13, 0] / 1e-22,
                  '2s': (sigma[13, 0] - inner[13, 0]) / 1e-22}
        for shell in ('1s', '2s'):
            reference = decimal_li_shell(energy, shell, contract)
            shell_rows.append(dict(energy_eV=float(energy), shell=shell,
                sigma_Mb=float(native[shell]), Decimal50_reference_Mb=reference,
                relative_error=relative_max(native[shell], reference)))
    checks['Li_shell_independent_Decimal_formula'] = all(
        r['relative_error'] < 5e-13 for r in shell_rows)
    details['Li_shell_formula_spots'] = shell_rows
    details['prior_validation_scope'] = 'T6 ended91.83842857919419eV; 122.5/200eV formula checks are new, not inherited physical-yield validation.'
    known = contract['cross_section_reference_Mb']
    checks['pinned_T6_reference_values'] = all(
        abs(row['sigma_Mb'] / known[key] - 1) < 5e-13
        for energy, shell, key in ((13.6, '2s', 'E13.6_2s'),
                                  (64.39, '1s', 'edge_right_1s'),
                                  (64.39, '2s', 'edge_right_2s'),
                                  (80., '1s', 'E80_1s'), (80., '2s', 'E80_2s'))
        for row in shell_rows if row['energy_eV'] == energy and row['shell'] == shell)
    accounted = rates['primary_kinetic_power_per_ion_W'] + rates[
        'unresolved_core_power_per_ion_W'] + net.transition_energy_eV * EV_J * rates['photo_s']
    energy_residual = relative_max(accounted, rates['absorbed_power_per_ion_W'])
    checks['absorption_primary_threshold_core_identity'] = energy_residual < 5e-15
    checks['primary_and_core_nonnegative'] = bool(
        np.all(rates['primary_kinetic_power_per_ion_W'] >= 0)
        and np.all(rates['unresolved_core_power_per_ion_W'] >= 0))
    details['energy_bookkeeping_relative_residual'] = energy_residual
    details['positive_LiI_inner_events_are_not_erased'] = {
        'photon_flux_above_64_39_m2_s': float(np.sum(rates['photon_flux_bins'][net.photon_energy_eV >= 64.39])),
        'LiI_inner_Gamma_s': float(rates['li1s_photo_s'][13]),
        'LiI_total_Gamma_s': float(rates['photo_s'][13]),
        'LiI_inner_event_fraction': float(rates['li1s_photo_s'][13] / rates['photo_s'][13])}
    checks['LiI_hard_photons_and_inner_rate_positive'] = (
        details['positive_LiI_inner_events_are_not_erased']['photon_flux_above_64_39_m2_s'] > 0
        and rates['li1s_photo_s'][13] > 0)

    # All charge fractions are positive for every nonzero isotope inventory.
    # Signed trial vectors below are the explicitly supported Newton extension.
    Y = np.zeros(21)
    for name, indices in net.isotope_groups.items():
        fractions = {1: [1.], 2: [.7, .3], 3: [.4, .4, .2], 4: [.1, .2, .3, .4]}[len(indices)]
        Y[list(indices)] = net.isotope_inventory[name] * np.asarray(fractions)
    temperature, uv, nb = 12000., .73, 1e8
    actual = net.evaluate(Y, temperature, uv, nb)
    finite = np.zeros((21, 21))
    step = 3e-5
    for j in range(21):
        direction = np.eye(21)[j]
        values = [net.evaluate(Y + s*step*direction, temperature, uv, nb)['dY_dt']
                  for s in (-2, -1, 1, 2)]
        finite[:, j] = (values[0] - 8*values[1] + 8*values[2] - values[3]) / (12*step)
    column_errors = [relative_max(actual['jac'][:, j], finite[:, j]) for j in range(21)]
    checks['all_21_Jacobian_columns_5point'] = max(column_errors) < 2e-7
    r = net.rates(temperature, uv)
    no_feedback = r['G0'] + r['D'] + actual['ne'] * r['Gne']
    atomic_columns = [i for i in range(21) if i not in (0, 5, 6)]
    feedback_defect = relative_max(no_feedback[:, atomic_columns], actual['jac'][:, atomic_columns])
    checks['electron_feedback_test_detects_missing_rank_one'] = feedback_defect > 1e-3
    details['Jacobian'] = dict(method='five-point finite differences of full nonlinear RHS, all21columns',
        absolute_Y_step=step, temperature_K=temperature, nb_m3=nb, uv_scale=uv,
        max_column_relative_error=max(column_errors), column_relative_errors=column_errors,
        missing_rank_one_relative_defect=feedback_defect,
        signed_trial_interpretation='Smooth Newton extension; accepted physical trajectories are checked separately.')

    # A finite frozen atomic generator exponential verifies semigroup wiring.
    # This is not a solve of the time-varying nonlinear cosmological equations.
    Q = r['G0'] + actual['ne'] * r['Gne']
    dt = 1 / float(np.max(-np.diag(Q)))
    transition = expm(dt * Q)
    after = transition @ Y
    checks['frozen_generator_exponential_positive_conservative'] = bool(
        transition.min() >= -2e-15 and after.min() >= -2e-15
        and abs(float(net.A @ after - net.A @ Y)) < 2e-15
        and np.max(np.abs(np.ones(21) @ transition - 1)) < 2e-15)
    details['frozen_generator'] = dict(dt_s=dt, minimum_transition=float(transition.min()),
        baryon_residual=float(net.A @ after - net.A @ Y),
        scope='Constant T, UV and ne; matrix exponential check only, no ODE trajectory.')

    decay_checks = []
    for initial, daughter, rate in ((0, 2, net.neutron_lambda_s),
                                     (5, 8, net.tritium_lambda_s),
                                     (6, 9, net.tritium_lambda_s)):
        pair = net.decay_matrix[np.ix_([initial, daughter], [initial, daughter])]
        t = math.log(2) / rate
        result = expm(pair * t) @ np.array([1., 0.])
        decay_checks.append(dict(parent=net.names[initial], daughter=net.names[daughter],
            half_time_s=t, residual=float(np.max(np.abs(result - .5))),
            charge_gain=int(net.q[daughter] - net.q[initial]),
            baryon_preserved=bool(net.A[daughter] == net.A[initial])))
    checks['weak_decay_analytic_half_population_maps'] = all(
        r['residual'] < 5e-16 and r['charge_gain'] == 1 and r['baryon_preserved'] for r in decay_checks)
    details['weak_decay_checks'] = decay_checks

    # A 60 eV control has complete source support, but cannot populate LiIII/IV
    # from the selected neutral/singly-ionized IC. This is a support diagnostic.
    soft = AtomicNetwork(uv_cutoff_eV=60.)
    sr = soft.rates(100., 1.)
    checks['soft_UV_control_support'] = bool(sr['photo_s'][13] > 0
        and np.all(sr['photo_s'][[14, 15, 18, 19]] == 0)
        and np.all(sr['li1s_photo_s'] == 0))
    rejected = 0
    for operation in (lambda: net.photo_cross_sections([201.]),
                      lambda: net.photo_cross_sections([10.+1j]),
                      lambda: net.rates(99., 1.), lambda: net.rates(100., -1.),
                      lambda: net.evaluate(np.ones(21, dtype=complex), 100., 1., 1.)):
        try:
            operation()
        except ValueError:
            rejected += 1
    checks['finite_domain_and_complex_guards'] = rejected == 5

    checks = {key: bool(value) for key, value in checks.items()}
    output = dict(schema='rec.atomic.verification.v1',
        status='PASS_FINITE_ATOMIC_CHECKS' if all(checks.values()) else 'FAIL',
        checks=checks, details=details,
        execution={'new_cosmological_ODE_solves': 0, 'benchmark_runs': 0,
                   'finite_matrix_exponentials': 4, 'nonlinear_Jacobian_FD_columns': 21},
        scope={'physical_yields': 'CONDITIONAL_NOT_VALIDATED',
               'global_convergence': 'NOT_RUN', 'production': 'NOT_APPROVED',
               'original_REC_U5_HOLD': 'UNCHANGED'},
        source_hashes={str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in [Path(__file__), ROOT/'code/atomic_network.py',
                      ROOT/'data/atomic_coefficients.json', ROOT/'data/atomic_model.json',
                      ROOT/'provenance/atomic_sources.json']})
    target = ROOT/'provenance/atomic_verification.json'
    target.write_text(json.dumps(output, indent=2, allow_nan=False) + '\n')
    print(json.dumps(dict(status=output['status'], checks_passed=sum(checks.values()),
        checks_total=len(checks), failed=[k for k, v in checks.items() if not v],
        jacobian_max_column_relative_error=max(column_errors),
        missing_rank_one_relative_defect=feedback_defect,
        energy_identity_relative_residual=energy_residual,
        output=str(target))))
    return 0 if all(checks.values()) else 1


if __name__ == '__main__':
    sys.exit(main())
