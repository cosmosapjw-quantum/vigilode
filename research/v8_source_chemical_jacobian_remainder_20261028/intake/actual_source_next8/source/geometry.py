"""Pinned LRS full40 equations with one prepared momentum evaluation per call.

No new quadrature or closure is introduced here. Dynamic electron density is
supplied in proper m^-3 by the atomic owner. CMB photons scatter; neutrinos do
not. UV and thermal baryon energies are diagnostics under the explicit
test-fluid approximation. All stress outputs are signed before taking norms.
"""
from __future__ import annotations

from functools import lru_cache
import hashlib
import json
import math
from pathlib import Path
import sys

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'vendor/baseline'))
sys.path.insert(0, str(ROOT / 'vendor/he31'))
sys.path.insert(0, str(ROOT / 'vendor/tangent'))
from canonical_fv import CanonicalFVSpecies
from lrs_boltzmann import SourceLock, GAMMA_PER_NE
from prepared_moments import PreparedCanonicalMoments
from tangent_core import TangentOperator

C = 299792458.0
G = 6.67430e-11
KB = 1.380649e-23
MU = 1.66053906892e-27
KAPPA_T = 8 * math.pi * G / C**2
SQRT6 = math.sqrt(6.0)


@lru_cache(maxsize=1)
def verify_sources():
    manifest = json.loads((ROOT/'provenance/BASELINE_SOURCES.json').read_text())
    for entry in manifest['files']:
        if hashlib.sha256((ROOT/entry['path']).read_bytes()).hexdigest() != entry['sha256']:
            raise ValueError('SOURCE_HASH_CHANGED:' + entry['path'])
    return manifest


@lru_cache(maxsize=1)
def cosmology():
    verify_sources()
    lock = json.loads((ROOT/'vendor/baseline/REI_BACKGROUND_FAMILY_LOCK.json').read_text())
    c = lock['cosmology']
    src = SourceLock(c['H0_per_s'], c['Omega_m'], c['Omega_r'],
                     c['Omega_lambda'], 1/21, 1/3)
    return src, c


class Geometry:
    size = 40

    def __init__(self, sigma_f: float, matter_A: float, radiation_A: float,
                 backend: str = 'prepared'):
        src, c = cosmology()
        if not all(math.isfinite(v) for v in (sigma_f, matter_A, radiation_A)):
            raise ValueError('finite seed required')
        if not (0 <= sigma_f <= .6 and -.9 <= matter_A <= 1.8
                and abs(radiation_A) <= .45):
            raise ValueError('seed outside the frozen preliminary campaign')
        if backend not in ('reference', 'prepared'):
            raise ValueError('unknown backend')
        self.model = CanonicalFVSpecies(src, c['Tcmb0_K'], 8,
            Sigma_ref=sigma_f/math.sqrt(2), orientation=int(sigma_f > 0),
            kappa_gamma=radiation_A, kappa_nu=radiation_A,
            ne_source=None, mass_fraction=.1, massive_A=matter_A,
            mass_over_Tref=30., radial_cells=24, angular_cells=12,
            subcell_gauss_order=3)
        self.backend = backend
        # Exact analytic Jacobian actions from PR118; BDF requests the small
        # 40x40 block only at Jacobian refreshes, not at every RHS call.
        self.tangent = TangentOperator(self.model)
        if backend == 'prepared':
            self.model.momentum = PreparedCanonicalMoments(self.model.momentum)
        self.source = src
        self.omega_b = c['Omega_b']
        self.nb0 = self.omega_b*src.rhocrit_j_m3/(MU*C*C)
        self.initial = self.model.initial_state()
        self.seed = dict(Sigma_F=sigma_f, matter_A=matter_A, radiation_A=radiation_A)

    def fields(self, x: float, y: np.ndarray):
        return self.model.fields(x, y)

    def rhs_from_fields(self, y, field, ne_m3=0.):
        if not math.isfinite(ne_m3) or ne_m3 < 0:
            raise ValueError('proper electron density must be finite and nonnegative')
        h, s = field['H'], field['s']
        v = s/h
        gamma = ne_m3*(GAMMA_PER_NE/1e6)/h
        out = np.zeros(40)
        out[0] = self.source.href/h
        out[1] = v
        out[2] = -3*y[2]+3*self.source.h0**2/(self.source.href*h)*(
            field['Pi_radiation_norm']+field['Pi_matter_norm'])
        for block, matrix in enumerate((self.model.energy_matrix,
                self.model.number_matrix, self.model.energy_matrix,
                self.model.number_matrix)):
            start = 4+9*block
            state = y[start:start+9]
            out[start:start+9] = v*(matrix@state)
            if block < 2:
                out[start:start+9] -= gamma*self.model.thomson*state
        return out

    def rhs(self, x, y, ne_m3=0.):
        return self.rhs_from_fields(y, self.fields(x, y), ne_m3)

    def normalized(self, x, y, field=None):
        q = self.fields(x, y) if field is None else field
        factor = SQRT6*KAPPA_T/q['H']**2
        pm = factor*q['Pi_matter_J_m3']
        pg = factor*q['Pi_gamma_J_m3']
        pn = factor*q['Pi_nu_J_m3']
        pr = pg+pn
        return np.array([SQRT6*q['s']/q['H'], pm, pr, pm+pr, pg, pn])

    def baryon_number_density(self, x):
        return self.nb0/(self.source.aref*math.exp(x))**3

    def jacobian_blocks(self, x, y, field, ne_m3):
        prepared = self.tangent.prepare(x, y)
        jac = prepared.jvp(np.eye(40))
        h, a = field['H'], field['a']
        hgrad = np.zeros(40)
        hgrad[1] = -3*self.source.h0**2*field['Pi_matter_norm']/h**2
        hgrad[2] = self.source.href**2*y[2]/h**2
        hgrad[4] = self.source.h0**2*self.model.omega_gamma/(2*h*h*a**4)
        hgrad[22] = self.source.h0**2*self.model.omega_nu/(2*h*h*a**4)
        gamma = ne_m3*(GAMMA_PER_NE/1e6)/h
        for start in (4,13):
            weighted = self.model.thomson*y[start:start+9]
            jac[start:start+9] += gamma*np.outer(weighted,hgrad)
            jac[start:start+9,start:start+9] -= gamma*np.diag(self.model.thomson)
        return jac, hgrad
