"""Finite 21-state H/D/T, He3/He4, Li6/Li7 atomic network in SI units.

Y is the number of each nucleus/ion per baryon, not a charge fraction. The
active processes are prescribed photoabsorption with one-step charge yield,
case-A radiative recombination, dielectronic recombination, and explicit n/T
decay maps. All collisional ionization, charge exchange, molecules, and
nuclear fusion are outside this finite model. See provenance/atomic_scope_KO.md.

The Li I inner-shell cross section is source based; its q -> q+1 yield is an
explicit coarse-graining assumption. Absorbed energy, primary-electron energy,
and unresolved core excitation are kept distinct. No heating yield is inferred.
"""
from __future__ import annotations

import hashlib
import json
import math
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
EV_J = 1.602176634e-19
C_M_S = 299792458.0
MB_M2 = 1e-22
CM3_M3 = 1e-6


def _readonly(a):
    a = np.asarray(a)
    a.setflags(write=False)
    return a


def _verified_inputs():
    manifest = json.loads((ROOT / 'provenance/atomic_sources.json').read_text())
    for entry in manifest['files']:
        path = ROOT / entry['path']
        if path.stat().st_size != entry['bytes'] or hashlib.sha256(
                path.read_bytes()).hexdigest() != entry['sha256']:
            raise ValueError('ATOMIC_SOURCE_CHANGED:' + entry['path'])
    return (json.loads((ROOT / 'data/atomic_coefficients.json').read_text()),
            json.loads((ROOT / 'data/atomic_model.json').read_text()))


def _ph2_mb(energy, parameters):
    e0, s0, ya, p, yw, y0, y1 = parameters
    x = np.asarray(energy, dtype=float) / e0 - y0
    y = np.hypot(x, y1)
    return s0 * ((x - 1)**2 + yw**2) * y**(p / 2 - 5.5) * (
        1 + np.sqrt(y / ya))**(-p)


def _ph1_mb(energy, parameters):
    _threshold, e0, s0, ya, p, yw = parameters
    y = np.asarray(energy, dtype=float) / e0
    return s0 * ((y - 1)**2 + yw**2) * y**(p / 2 - 5.5) * (
        1 + np.sqrt(y / ya))**(-p)


class AtomicNetwork:
    """Prepared immutable coefficients, spectrum, and analytic abundance Jacobian.

    ``rates`` and ``evaluate`` use physical seconds. The cosmological caller
    divides dY_dt by H when its independent variable is ln(a). The optional
    UV cutoff can only shorten the adopted <=200 eV source support.

    ``evaluate`` permits finite signed *trial* Y for an implicit solver's smooth
    Newton extension; it never clips a population or electron density. Accepted
    trajectories must separately satisfy positivity and conservation. The
    positive-generator interpretation applies to physical Y with n_e >= 0.
    """
    size = 21

    def __init__(self, uv_cutoff_eV=None):
        self.coefficients, self.model = _verified_inputs()
        self.names = tuple(self.model['species_order'])
        self.A = _readonly([1, 1, 1, 2, 2, 3, 3, 3, 3, 3, 4, 4, 4,
                            6, 6, 6, 6, 7, 7, 7, 7])
        self.Z = _readonly([0, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2,
                            3, 3, 3, 3, 3, 3, 3, 3])
        self.q = _readonly([0, 0, 1, 0, 1, 0, 1, 0, 1, 2, 0, 1, 2,
                            0, 1, 2, 3, 0, 1, 2, 3])
        self.isotope_groups = {
            'n': (0,), 'H1': (1, 2), 'D': (3, 4), 'T': (5, 6),
            'He3': (7, 8, 9), 'He4': (10, 11, 12),
            'Li6': (13, 14, 15, 16), 'Li7': (17, 18, 19, 20)}
        self.photo_indices = _readonly(np.flatnonzero((self.Z > 0) & (self.q < self.Z)))
        self.recombination_indices = _readonly(np.flatnonzero(self.q > 0))
        self._prepare_inventory()
        self._prepare_spectrum(uv_cutoff_eV)
        self._prepare_recombination()
        self._prepare_decay()

    def _prepare_inventory(self):
        ratios = self.model['inventory_number_ratios_to_H1']
        normalization = math.fsum(self.A[indices[0]] * ratios[name]
                                 for name, indices in self.isotope_groups.items())
        self.initial_Y = np.zeros(self.size)
        self.scales = np.zeros(self.size)
        self.isotope_inventory = {}
        for name, indices in self.isotope_groups.items():
            number = ratios[name] / normalization
            self.isotope_inventory[name] = number
            self.scales[list(indices)] = number if number > 0 else self.model['zero_inventory_scale']
            if name in ('H1', 'D'):
                f = self.model['initial_H1_D_ionized_fraction']
                self.initial_Y[indices[0]] = number * (1 - f)
                self.initial_Y[indices[1]] = number * f
            else:
                charge = (self.model['initial_lithium_charge'] if name.startswith('Li')
                          else self.model['initial_helium_charge'] if name.startswith('He')
                          else 0)
                self.initial_Y[indices[charge]] = number
        self.initial_Y = _readonly(self.initial_Y)
        self.scales = _readonly(self.scales)
        self.transition_energy_eV = np.zeros(self.size)
        self.ionization_energy_J = np.zeros(self.size)
        for _name, indices in self.isotope_groups.items():
            if self.Z[indices[0]] == 0:
                continue
            cumulative = 0.0
            for index in indices:
                self.ionization_energy_J[index] = cumulative * EV_J
                if self.q[index] < self.Z[index]:
                    row = self.coefficients['photo_VF1996'][f'{self.Z[index]}:{self.q[index]}']
                    self.transition_energy_eV[index] = row['Eth_eV']
                    cumulative += row['Eth_eV']
        self.transition_energy_eV = _readonly(self.transition_energy_eV)
        self.ionization_energy_J = _readonly(self.ionization_energy_J)

    def photo_cross_sections(self, energy_eV):
        """Return total and Li-I-1s partial cross sections, shape (21, n), m2.

        Source support here is the finite [0,200] eV contract. A zero below an
        atomic threshold is structural. Above 64.39 eV neutral Li includes both
        PH1 shells; no unsupported valence-only zero or extension is used.
        """
        if np.iscomplexobj(energy_eV):
            raise ValueError('photo energies must be real')
        e = np.atleast_1d(np.asarray(energy_eV, dtype=float))
        if e.ndim != 1 or not np.all(np.isfinite(e)) or np.any(e < 0) or np.any(e > 200):
            raise ValueError('photo energy outside finite 0..200 eV contract')
        sigma = np.zeros((self.size, e.size))
        inner = np.zeros_like(sigma)
        for index in self.photo_indices:
            z, charge = self.Z[index], self.q[index]
            row = self.coefficients['photo_VF1996'][f'{z}:{charge}']
            if z == 3 and charge == 0:
                partial = self.coefficients['LiI_partial']
                edge = partial['PH1_1s'][0]
                low = (e >= row['Eth_eV']) & (e < edge)
                high = e >= edge
                sigma[index, low] = _ph2_mb(e[low], partial['PH2_2s'])
                inner[index, high] = _ph1_mb(e[high], partial['PH1_1s'])
                sigma[index, high] = _ph1_mb(e[high], partial['PH1_2s']) + inner[index, high]
            else:
                active = e >= row['Eth_eV']
                if np.any(e[active] > row['Emax_eV']):
                    raise ValueError('photo row unsupported above Emax')
                pars = [row[k] for k in ('E0_eV', 'sigma0_Mb', 'ya', 'P', 'yw', 'y0', 'y1')]
                sigma[index, active] = _ph2_mb(e[active], pars)
        return sigma * MB_M2, inner * MB_M2

    def _prepare_spectrum(self, cutoff):
        field = dict(self.model['photon_field'])
        emax = field['energy_max_eV'] if cutoff is None else float(cutoff)
        if not math.isfinite(emax) or not (13.6 < emax <= field['energy_max_eV']):
            raise ValueError('UV cutoff must be >13.6 and <=200 eV')
        emin = field['energy_min_eV']
        thresholds = list(self.transition_energy_eV[self.photo_indices]) + [64.39]
        edges = np.unique(np.concatenate((np.geomspace(emin, emax,
            field['base_log_bins'] + 1), [t for t in thresholds if emin < t < emax])))
        energy = np.sqrt(edges[:-1] * edges[1:])
        power = field['number_power_index'] + 1
        weights = (edges[1:]**power - edges[:-1]**power) / power
        sigma, inner = self.photo_cross_sections(energy)
        normalization = field['late_Gamma_HI_s'] / float(sigma[1] @ weights)
        flux = weights * normalization
        self.uv_config = dict(field, actual_energy_max_eV=emax,
                              actual_bin_count=int(len(energy)))
        self.photon_bin_edges_eV = _readonly(edges)
        self.photon_energy_eV = _readonly(energy)
        self.photon_flux_bins_base = _readonly(flux)
        self.photo_sigma_m2 = _readonly(sigma)
        self.li1s_photo_sigma_m2 = _readonly(inner)
        self._photo_s = _readonly(sigma @ flux)
        self._li1s_photo_s = _readonly(inner @ flux)
        self._absorbed_W = _readonly((sigma * (energy * EV_J)) @ flux)
        self._inner_absorbed_W = _readonly((inner * (energy * EV_J)) @ flux)
        self._core_W = _readonly(self._li1s_photo_s * (64.39 - 5.392) * EV_J)
        primary = sigma * ((energy[None, :] - self.transition_energy_eV[:, None]) * EV_J)
        primary -= inner * ((64.39 - 5.392) * EV_J)
        self._primary_W = _readonly(primary @ flux)
        self._Gphoto = _readonly(self._transition_matrix(self._photo_s, upward=True))

    def _prepare_recombination(self):
        rr_rows = [self.coefficients['rr'][f'{self.Z[i]}:{self.q[i]}']
                   for i in self.recombination_indices]
        self._rr_parameters = _readonly([[r[k] for k in (
            'A_cm3_s', 'B', 'T0_K', 'T1_K', 'C', 'T2_K')] for r in rr_rows])
        dr_indices, c_rows, e_rows = [], [], []
        for index in self.recombination_indices:
            row = self.coefficients['dr'].get(f'{self.Z[index]}:{self.q[index]}')
            if row:
                dr_indices.append(index)
                c_rows.append(row['c_cm3_s_K32'])
                e_rows.append(row['E_K'])
        self._dr_indices = _readonly(np.asarray(dr_indices, dtype=int))
        self._dr_c = _readonly(c_rows)
        self._dr_E = _readonly(e_rows)

    def _prepare_decay(self):
        config = self.model['weak_decay']
        self.neutron_lambda_s = 1 / config['neutron_mean_lifetime_s']
        self.tritium_lambda_s = math.log(2) / (
            config['tritium_half_life_years'] * config['julian_year_s'])
        matrix = np.zeros((self.size, self.size))
        for initial, final, rate in ((0, 2, self.neutron_lambda_s),
                                    (5, 8, self.tritium_lambda_s),
                                    (6, 9, self.tritium_lambda_s)):
            matrix[initial, initial] -= rate
            matrix[final, initial] += rate
        self.decay_matrix = _readonly(matrix)

    def _transition_matrix(self, outgoing, upward):
        matrix = np.zeros((self.size, self.size))
        indices = self.photo_indices if upward else self.recombination_indices
        matrix[indices, indices] = -outgoing[indices]
        matrix[indices + (1 if upward else -1), indices] = outgoing[indices]
        return matrix

    def rates(self, temperature_K, uv_scale):
        """Matrices act on column Y; Gne is in m3/s, G0 and D in s^-1."""
        temperature = float(temperature_K)
        uv = float(uv_scale)
        tmin, tmax = self.coefficients['runtime_temperature_domain_K']
        if not math.isfinite(temperature) or not (tmin <= temperature <= tmax):
            raise ValueError(f'temperature outside supported {tmin}..{tmax} K')
        if not math.isfinite(uv) or uv < 0:
            raise ValueError('UV scale must be finite and nonnegative')
        a, b, t0, t1, c, t2 = self._rr_parameters.T
        beff = b + c * np.exp(-t2 / temperature)
        u, v = np.sqrt(temperature / t0), np.sqrt(temperature / t1)
        rr = np.zeros(self.size)
        rr[self.recombination_indices] = CM3_M3 * a / (
            u * (1 + u)**(1 - beff) * (1 + v)**(1 + beff))
        dr = np.zeros(self.size)
        dr[self._dr_indices] = CM3_M3 * temperature**(-1.5) * np.sum(
            self._dr_c * np.exp(-self._dr_E / temperature), axis=1)
        if np.any(rr < 0) or np.any(dr < 0) or not np.all(np.isfinite(rr + dr)):
            raise ValueError('nonpositive or nonfinite total recombination fit')
        grr = self._transition_matrix(rr, upward=False)
        gdr = self._transition_matrix(dr, upward=False)
        return dict(G0=uv * self._Gphoto, Gne=grr + gdr, Grr=grr, Gdr=gdr,
                    D=self.decay_matrix, photo_s=uv * self._photo_s,
                    rr_m3_s=rr, dr_m3_s=dr,
                    photon_flux_bins=uv * self.photon_flux_bins_base,
                    absorbed_power_per_ion_W=uv * self._absorbed_W,
                    primary_kinetic_power_per_ion_W=uv * self._primary_W,
                    unresolved_core_power_per_ion_W=uv * self._core_W,
                    li1s_photo_s=uv * self._li1s_photo_s,
                    li1s_absorbed_power_per_ion_W=uv * self._inner_absorbed_W)

    def evaluate(self, Y, temperature_K, uv_scale, nb):
        """Return exact finite-model RHS/Jacobian and physical rate diagnostics.

        Jacobian = G0 + D + ne Gne + nb outer(Gne Y, q). The last term is
        essential: the same evolving abundances supply the free electrons.
        Decay energy is not supplied by this atomic ledger; n/T are zero in the
        main pilot. The returned absorbed power is not a thermal heating rate.
        """
        if np.iscomplexobj(Y):
            raise ValueError('abundances must be real')
        population = np.asarray(Y, dtype=float)
        density = float(nb)
        if population.shape != (self.size,) or not np.all(np.isfinite(population)):
            raise ValueError('Y must be a finite length-21 vector')
        if not math.isfinite(density) or density < 0:
            raise ValueError('proper baryon density must be finite and nonnegative')
        rates = self.rates(temperature_K, uv_scale)
        ne = density * float(self.q @ population)
        photo = rates['G0'] @ population
        rr = ne * (rates['Grr'] @ population)
        dr = ne * (rates['Gdr'] @ population)
        decay = rates['D'] @ population
        recombination_action = rates['Gne'] @ population
        derivative = photo + rr + dr + decay
        jacobian = rates['G0'] + rates['D'] + ne * rates['Gne'] + density * np.outer(
            recombination_action, self.q)
        return dict(dY_dt=derivative, ne=ne, jac=jacobian,
                    photo_rhs=photo, rr_rhs=rr, dr_rhs=dr, decay_rhs=decay,
                    electron_source_m3_s=density * float(self.q @ derivative),
                    photo_absorbed_power_W_m3=density * float(
                        population @ rates['absorbed_power_per_ion_W']),
                    primary_kinetic_power_W_m3=density * float(
                        population @ rates['primary_kinetic_power_per_ion_W']),
                    unresolved_core_power_W_m3=density * float(
                        population @ rates['unresolved_core_power_per_ion_W']),
                    li1s_photo_event_rate_m3_s=density * float(
                        population @ rates['li1s_photo_s']),
                    li1s_absorbed_power_W_m3=density * float(
                        population @ rates['li1s_absorbed_power_per_ion_W']),
                    uv_energy_density_J_m3=float(rates['photon_flux_bins'] @ (
                        self.photon_energy_eV * EV_J)) / C_M_S)


Network = AtomicNetwork
