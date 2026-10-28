"""Constant invariant-subspace restriction of the unchanged REC full62 owner.

No angular matrices, rates, closure or full62 equations are replaced. The
coordinate order is 23 even geometry entries, 9 element fractions, then tau.
"""
from __future__ import annotations
from functools import lru_cache
import hashlib
import json
from pathlib import Path
import sys
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
DONOR = ROOT / 'vendor/rec_hheli'
sys.path.insert(0, str(DONOR / 'code'))
from coupled import CoupledSystem

GEOMETRY = np.array([0, 1, 2] + [s+k for s in (4,13,22,31) for k in (0,2,4,6,8)])
ATOMIC = np.array([1,2,10,11,12,17,18,19,20])
SELECT = np.r_[GEOMETRY,40+ATOMIC,61]
R = np.eye(62)[SELECT]
P = R.T.copy()
for target, representative in ((3,1),(4,2),(7,10),(8,11),(9,12),
                               (13,17),(14,18),(15,19),(16,20)):
    P[40+target] = P[40+representative]
EXCLUDED_GEOMETRY = np.setdiff1d(np.arange(40), GEOMETRY)
INITIAL_EQ_ATOL = 8*np.finfo(float).eps
for array in (GEOMETRY,ATOMIC,SELECT,R,P,EXCLUDED_GEOMETRY):
    array.setflags(write=False)

@lru_cache(maxsize=1)
def verify_donor():
    manifest = json.loads((ROOT/'provenance/DONOR_IDENTITY.json').read_text())
    for row in manifest['files']:
        relative = Path(row['path']).relative_to('research/rec_rei_hheli_sandbox_20261021')
        data = (DONOR/relative).read_bytes()
        actual = hashlib.sha1(f'blob {len(data)}\0'.encode()+data).hexdigest()
        if actual != row['git_blob']:
            raise ValueError('DONOR_SOURCE_CHANGED:'+str(relative))
    return manifest

def restrict(y):
    y = np.asarray(y)
    if y.shape[0] != 62: raise ValueError('expected leading full62 axis')
    return y[SELECT].copy()

def lift(v):
    v = np.asarray(v)
    if v.shape[0] != 33: raise ValueError('expected leading compact33 axis')
    # Sparse constant lift handles both a vector and column-wise trajectories.
    y = np.zeros((62,)+v.shape[1:],dtype=v.dtype)
    y[SELECT] = v
    for target, representative in ((3,1),(4,2),(7,10),(8,11),(9,12),
                                   (13,17),(14,18),(15,19),(16,20)):
        y[40+target] = y[40+representative]
    return y

def require_eligible(owner, initial=None):
    """Reject models/states outside the frozen equality model, explicitly."""
    y = owner.y0 if initial is None else np.asarray(initial,dtype=float)
    if y.shape != (62,) or not np.isfinite(y).all():
        raise ValueError('NEED_FULL62: initial vector must be finite length 62')
    if np.any(y[EXCLUDED_GEOMETRY] != 0):
        raise ValueError('NEED_FULL62: nonzero initial odd geometry or placeholder')
    a=owner.atomic; scaled=y[40:61]
    if np.any(scaled[[0,5,6]] != 0) or np.any(a.initial_Y[[0,5,6]] != 0):
        raise ValueError('NEED_FULL62: nonzero initial neutron/tritium inventory')
    residual=0.
    for left,right in (([1,2],[3,4]),([10,11,12],[7,8,9]),([17,18,19,20],[13,14,15,16])):
        error=float(np.max(abs(scaled[left]-scaled[right])));residual=max(residual,error)
        if error > INITIAL_EQ_ATOL:
            raise ValueError('NEED_FULL62: unequal initial isotope charge fractions')
        # Owner electronic rates depend only on Z,q; check prepared rate inputs
        # and spectrum arrays, rather than introducing isotope approximations.
        if not np.array_equal(a.Z[left],a.Z[right]) or not np.array_equal(a.q[left],a.q[right]):
            raise ValueError('NEED_FULL62: changed electronic equality model')
        for name in ('_photo_s','_li1s_photo_s','_absorbed_W','_primary_W','_core_W'):
            values=getattr(a,name)
            if not np.array_equal(values[left],values[right]):
                raise ValueError('NEED_FULL62: changed isotope electronic input '+name)
    # Matrices are constructed from the same coefficient dictionaries by Z:q.
    # Check complete corresponding blocks at all prescribed thermal anchors.
    for temperature in (100.,6050.,12000.):
        rates=a.rates(temperature,1.)
        for name in ('G0','Gne'):
            for left,right in (([1,2],[3,4]),([10,11,12],[7,8,9]),([17,18,19,20],[13,14,15,16])):
                m=rates[name]
                if not np.array_equal(m[np.ix_(left,left)],m[np.ix_(right,right)]):
                    raise ValueError('NEED_FULL62: changed isotope electronic rates '+name)
    return {'initial_isotope_scaled_residual':residual,
            'initial_equivalence_atol':INITIAL_EQ_ATOL,
            'initial_lift_max_abs':float(np.max(abs(lift(restrict(y))-y)))}

class CompactSystem:
    size=33
    def __init__(self,config):
        verify_donor()
        self.owner=CoupledSystem(config)
        self.eligibility=require_eligible(self.owner)
        self.original_initial62=self.owner.y0.copy()
        self.y0=restrict(self.original_initial62)
    def __getattr__(self,name):
        return getattr(self.owner,name)
    def rhs(self,x,v):
        return restrict(self.owner.rhs(x,lift(v)))
    def jac(self,x,v):
        return self.owner.jac(x,lift(v))[SELECT] @ P
    def diagnostics(self,x,y):
        # Run adapter passes reconstructed full62 vectors to inherited diagnostics.
        return self.owner.diagnostics(x,y)
