"""Source-pinned, opt-in actual BASS client for the unchanged D5 point gateway.

This module supplies client data and callbacks. It contains no error norm,
integrator, hidden finite differences, native restore or continuous authority.
"""
from __future__ import annotations
import copy
import ctypes
from dataclasses import dataclass
from fractions import Fraction
import hashlib
import json
import math
import os
from pathlib import Path
import select
import subprocess
import sys
import time
from types import MappingProxyType
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
BASS = ROOT / 'vendor/bass'
COMPACT = BASS / 'research/rei_hheli_compact_pilot_20261022'
AFFINE = BASS / 'research/sandbox_affine_chemistry_20261022'
SOURCE_MANIFEST = ROOT / 'CLIENT_SOURCE_MANIFEST.json'
POINT = ROOT / 'SELECTED_ACTUAL_CLIENT_POINT.json'
DONOR = COMPACT / 'vendor/rec_hheli'
PINNED_MODULE_PATHS = {
    'compact_system': COMPACT/'code/compact_system.py',
    'affine_system': AFFINE/'code/affine_system.py',
    **{name: DONOR/'code'/f'{name}.py' for name in ('coupled','geometry','atomic_network')},
    **{name: DONOR/'vendor/baseline'/f'{name}.py' for name in
       ('canonical_fv','lrs_boltzmann','species_hierarchy')},
    'prepared_moments': DONOR/'vendor/he31/prepared_moments.py',
    'tangent_core': DONOR/'vendor/tangent/tangent_core.py',
}
_OWN_LOADED_MODULES = {}


class BindingRefusal(RuntimeError):
    def __init__(self, code, detail=''):
        self.code = code
        super().__init__(code + (': ' + detail if detail else ''))


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False).encode()


def check_module_origins(require_loaded=False):
    """Never reuse an unrelated module hidden behind the donor's short names."""
    for name, path in PINNED_MODULE_PATHS.items():
        module = sys.modules.get(name)
        if module is None:
            if require_loaded:
                raise BindingRefusal('MODULE_ORIGIN_COLLISION', name)
            continue
        # Even a coincidentally equal __file__ is insufficient for an unknown
        # cached object. Only this loader's previously verified objects repeat.
        if _OWN_LOADED_MODULES.get(name) is not module:
            raise BindingRefusal('MODULE_ORIGIN_COLLISION', name)
        origin = getattr(getattr(module, '__spec__', None), 'origin', None)
        filename = getattr(module, '__file__', None)
        if not origin or not filename or Path(origin).resolve()!=path.resolve() or Path(filename).resolve()!=path.resolve():
            raise BindingRefusal('MODULE_ORIGIN_COLLISION', name)


def load_pinned_types():
    check_module_origins()
    sys.path.insert(0, str(COMPACT / 'code'))
    sys.path.insert(0, str(AFFINE / 'code'))
    from compact_system import CompactSystem
    from affine_system import AffineSystem, LIFT, KEEP
    for name, path in PINNED_MODULE_PATHS.items():
        module = sys.modules.get(name)
        origin = getattr(getattr(module, '__spec__', None), 'origin', None)
        filename = getattr(module, '__file__', None)
        if not origin or not filename or Path(origin).resolve()!=path.resolve() or Path(filename).resolve()!=path.resolve():
            raise BindingRefusal('MODULE_ORIGIN_COLLISION', name)
    _OWN_LOADED_MODULES.update({name:sys.modules[name] for name in PINNED_MODULE_PATHS})
    return CompactSystem, AffineSystem, LIFT, KEEP


def verify_sources():
    """Read exact source bytes; the historical campaign itself is not replayed."""
    manifest = json.loads(SOURCE_MANIFEST.read_text())
    for row in manifest['files']:
        data = (BASS / row['path']).read_bytes()
        blob = hashlib.sha1(f'blob {len(data)}\0'.encode()+data).hexdigest()
        if len(data) != row['bytes'] or hashlib.sha256(data).hexdigest() != row['sha256'] or blob != row['git_blob']:
            raise BindingRefusal('SOURCE_CHANGED', row['path'])
    return manifest


def provider_environment():
    """Observed FE_TONEAREST and gradual subnormal arithmetic in this process.

    The Rust process also uses the unchanged D5 fixture's MXCSR probe. Neither
    observation proves every linked library's compiler options or no-ABA.
    """
    try:
        probe = ctypes.CDLL(None).fegetround
        probe.restype = ctypes.c_int
        probe.argtypes = []
        smallest = float.fromhex('0x0.0000000000001p-1022')
        return probe() == 0 and smallest * 2.0 == float.fromhex('0x0.0000000000002p-1022')
    except (AttributeError, OSError):
        return False


@dataclass(frozen=True)
class PreparedPoint:
    x: float
    reduced: np.ndarray
    rhs: np.ndarray
    jacobian: np.ndarray
    binding_id: str
    epoch: int
    counts: tuple[int, int, int]
    authority: str = 'D5_POINT_PAIR_ONLY'


class FrozenClient:
    """One immutable scientific configuration; counters are operational state.

    The driver keeps the model private during a request. Source hashes/epochs
    are identity checks, not security attestation of arbitrary Python captures.
    The provider must not mutate private coefficients or monkeypatch loaded
    functions; these checks do not attest a hostile Python process.
    """
    epoch = 1
    derivative_policy = 'ProvidedOnly'

    def __init__(self):
        manifest = verify_sources()
        self._manifest_digest = hashlib.sha256(SOURCE_MANIFEST.read_bytes()).hexdigest()
        self._point_digest = hashlib.sha256(POINT.read_bytes()).hexdigest()
        self.point = json.loads(POINT.read_text())
        campaign = json.loads((COMPACT / 'data/CAMPAIGN.json').read_text())
        selected = [c for c in campaign['cases'] if c['case_id'] == self.point['case_id']]
        if len(selected) != 1 or dict(selected[0]['config'], case_id=self.point['case_id']) != self.point['config']:
            raise BindingRefusal('CONFIG_IDENTITY_MISMATCH')
        config = copy.deepcopy(self.point['config'])
        self.config = MappingProxyType(config)
        CompactSystem, AffineSystem, LIFT, KEEP = load_pinned_types()
        self._compact = CompactSystem(copy.deepcopy(config))
        self._affine = AffineSystem(self._compact)
        self._lift = LIFT
        self._keep = KEEP
        self._totals = tuple(float(x) for x in self._affine.totals)
        self._config_digest = hashlib.sha256(canonical(config)).hexdigest()
        self.identity = dict(schema='vigilode.bass.actual-client.v1',
            source_commit=manifest['source_commit'], source_manifest_sha256=self._manifest_digest,
            selected_point_sha256=self._point_digest, case_id=self.point['case_id'],
            config=config, coordinate_map='fixed full62 -> compact33 -> affine30',
            conserved_totals_hex=[x.hex() for x in self._totals],
            clock='x=ln(a/a_ref);a_ref=1/21;dx/dt=H',
            derivative_policy=self.derivative_policy, partial_x='UNSUPPLIED',
            domain_policy='strict_actual_fractions_v1', epoch=self.epoch)
        self.binding_id = hashlib.sha256(canonical(self.identity)).hexdigest()
        self.domain_refusals = 0
        self.last_refusal = None

    @property
    def callback_counts(self):
        return (self._compact.owner.nrhs, self._compact.owner.njac)

    def _fail(self, code):
        self.domain_refusals += 1
        self.last_refusal = code
        raise BindingRefusal(code)

    def preflight(self, x, reduced):
        """Actual signed chart + exact represented fraction simplex, before F/J.

        The strict physical trial policy is opt-in; the donor's smooth signed
        Newton extension and old accepted-trajectory tolerances are unchanged.
        This point domain supplies no continuous tube or angular-positivity proof.
        """
        if isinstance(x, (complex, np.complexfloating)) or np.iscomplexobj(reduced):
            self._fail('COMPLEX_STATE')
        try:
            x = float(x)
            z = np.asarray(reduced, dtype=float)
        except (TypeError, ValueError, OverflowError):
            self._fail('INVALID_STATE_SHAPE')
        if z.shape != (30,):
            self._fail('INVALID_STATE_SHAPE')
        if not math.isfinite(x) or not np.isfinite(z).all():
            self._fail('NONFINITE_STATE')
        if not (0.0 <= x <= self.config['x_end']):
            self._fail('TIME_OUTSIDE_SOURCE')
        if abs(z[1]) > 1.0:
            self._fail('GEOMETRY_CHART')
        if z[0] < 0.0 or z[29] < 0.0:
            self._fail('NEGATIVE_ELAPSED_OR_OPTICAL_DEPTH')
        if any(z[i] <= 0.0 for i in (3, 8, 13, 18)):
            self._fail('NONPOSITIVE_MONOPOLE')
        if np.any(z[23:29] < 0.0):
            self._fail('NEGATIVE_FRACTION')
        for block, total in zip(((23,), (24,25), (26,27,28)), self._totals):
            exact = sum((Fraction.from_float(float(z[i])) for i in block), Fraction())
            if exact > Fraction.from_float(total):
                self._fail('EXACT_SIMPLEX_OVERFLOW')
        # Check actual rounded provider lift too; do not clip a rounded neutral.
        full = self._affine.lift(z)
        if not np.isfinite(full).all() or np.any(full[23:32] < 0.0):
            self._fail('REPRESENTED_LIFT_OUTSIDE_DOMAIN')
        return x, z.copy(), full

    def witness(self):
        try:
            if hashlib.sha256(SOURCE_MANIFEST.read_bytes()).hexdigest() != self._manifest_digest or hashlib.sha256(POINT.read_bytes()).hexdigest() != self._point_digest:
                raise BindingRefusal('BINDING_INPUT_CHANGED')
            verify_sources()
            check_module_origins(require_loaded=True)
            if hashlib.sha256(canonical(dict(self.config))).hexdigest() != self._config_digest or canonical(self._compact.owner.config) != canonical(dict(self.config)):
                raise BindingRefusal('CONFIG_CHANGED')
            return self.epoch, provider_environment()
        except (OSError, ValueError, BindingRefusal):
            return self.epoch, False

    def rhs(self, x, z):
        x, z, _ = self.preflight(x, z)
        return self._affine.rhs(x, z)

    def jacobian(self, x, z):
        x, z, _ = self.preflight(x, z)
        return self._affine.jac(x, z)

    def partial_x(self, x, z):
        self.preflight(x, z)
        raise BindingRefusal('PARTIAL_X_UNSUPPLIED_PROVIDED_ONLY')

    def native_norm_arguments(self):
        offset = np.zeros(33)
        offset[[23,25,28]] = self._totals
        return dict(native_source='aaf1f834d4c589e734ca7851a56c11bb44f7047a',
            native_blob='95efe19477aba8efe378a1f5e083c09b2cdffb69',
            constructor='AffineErrorNorm::new', reduced_dimension=30,
            physical_dimension=33, offset=offset.tolist(),
            row_major_lift=self._lift.ravel().tolist(),
            atol=list(self.point['atol_full33']), rtol=self.point['rtol'],
            status='ARGUMENTS_BOUND_EXISTING_IMPLEMENTATION_REUSED',
            native_RODAS_integration='OPEN_OWNER_INPUT')

    def v7_binding(self):
        return dict(node='V7-restart-cut', binding_id=self.binding_id,
            identity=self.identity, callback_effects='pure point values; no D4 external journal',
            source_counter_fields=['CoupledSystem.nrhs','CoupledSystem.njac'],
            required_provider_operational_fields=['CoupledSystem.nrhs','CoupledSystem.njac',
                'consumed provider wall allowance and remaining CoupledSystem.deadline',
                'rebase CoupledSystem.started without restoring spent allowance'],
            provider_deadline_policy='preserve consumed allowance across reconstruction; never reset to a fresh 180 seconds',
            status='CLIENT_IDENTITY_BOUND_NATIVE_CONTINUATION_OPEN',
            native_checkpoint_scope='borrowed summary only; not a full restore payload',
            required_native_fields=['t','y','tf','proposed_h','attempts','accepted_steps',
                'rejected_steps','counters','controller','output_schedule_and_cursor',
                'workspace_and_cache_or_explicit_safe_invalidation','identity_epochs',
                'terminal_and_last_failure','internal_step_and_reuse_counters','accepted_output_prefix',
                'cap_and_cumulative_budget'],
            native_owner='UNRESOLVED_INPUT_DEPENDENCY', reserved_budget='UNKNOWN_PRESERVED',
            interruption_test='NOT_RUN_NO_NATIVE_RESTORE_API', reset=False)

    def v8_binding(self):
        return dict(node='V8-whole-domain-provider', binding_id=self.binding_id,
            identity=self.identity, status='ACTUAL_MODEL_BOUND_OBLIGATIONS_OPEN',
            native_result='UnsupportedUnboundModel',
            point_preparation_authority='D5_POINT_PAIR_ONLY',
            requirements=['time/state tube and its domain inclusion',
                'whole-tube original residual bound', 'whole-tube scaled Jacobian/logarithmic-norm bound',
                'inherited endpoint error', 'query rounding bound',
                'model/source discrepancy bound kept separate from numerical residual',
                'native owner integration into private VerifiedModelProvider factory'],
            angular_positivity='129-angle donor diagnostic remains finite-sample evidence',
            inherited_error=None, source_model_error=None, numerical_residual_bound=None,
            jacobian_bound=None, query_rounding_bound=None, certificate_issued=False)


class D5Bridge:
    """A persistent Gate<30>; accept a pair only after all three output lines."""
    def __init__(self, client, executable=None, timeout_seconds=5.0):
        self.client = client
        self.timeout = timeout_seconds
        self.buffer = b''
        self.counts = (0,0,0)
        self.witness_calls = 0
        self.last_error = None
        self._transport_fault = None
        self._closed = False
        self._header_received = False
        self.delivered_pairs = 0
        self.work_accounting_state = 'OBSERVED'
        executable = ROOT/'bin/d5_bridge' if executable is None else Path(executable)
        self.process = subprocess.Popen([str(executable)], stdin=subprocess.PIPE,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, bufsize=0)
        maximum = np.finfo(float).max
        low, high = np.full(30,-maximum), np.full(30,maximum)
        low[[0,3,8,13,18,23,24,25,26,27,28,29]]=0.0
        low[1], high[1] = -1.0, 1.0
        high[23]=client._totals[0]
        high[24:26]=client._totals[1]
        high[26:29]=client._totals[2]
        try:
            self._send(f'INIT {client.epoch} 0 {client.config["x_end"]:.17e}')
            self._send('LOW '+self._floats(low))
            self._send('HIGH '+self._floats(high))
            if self._read() != 'READY':
                raise BindingRefusal('BRIDGE_INIT')
        except BaseException:
            self.close()
            raise

    @staticmethod
    def _floats(values):
        return ' '.join(format(float(v),'.17e') for v in np.asarray(values).ravel())

    def _send(self, text):
        try:
            data=(text+'\n').encode('ascii')
            while data:
                written=self.process.stdin.write(data)
                if not written:raise BrokenPipeError('zero IPC write')
                data=data[written:]
            self.process.stdin.flush()
        except (OSError, ValueError) as error:
            raise BindingRefusal('BRIDGE_WRITE',str(error)) from error

    def _read(self):
        deadline=time.monotonic()+self.timeout
        while b'\n' not in self.buffer:
            if len(self.buffer)>256*1024:raise BindingRefusal('BRIDGE_OVERSIZE')
            remaining=deadline-time.monotonic()
            if remaining<=0 or not select.select([self.process.stdout],[],[],remaining)[0]:
                raise BindingRefusal('BRIDGE_TIMEOUT')
            chunk=os.read(self.process.stdout.fileno(),65536)
            if not chunk:raise BindingRefusal('BRIDGE_EOF')
            self.buffer+=chunk
        line,self.buffer=self.buffer.split(b'\n',1)
        if len(line)>256*1024:raise BindingRefusal('BRIDGE_OVERSIZE')
        return line.decode('ascii')

    def _values(self, prefix, size):
        words=self._read().split()
        if len(words)!=size+1 or words[0]!=prefix:
            raise BindingRefusal('BRIDGE_OUTPUT_SHAPE')
        values=np.array([float(v) for v in words[1:]])
        if not np.isfinite(values).all():raise BindingRefusal('BRIDGE_NONFINITE_OUTPUT')
        return values

    def prepare(self, x, reduced):
        if self._transport_fault is not None:
            raise BindingRefusal('BRIDGE_POISONED',self._transport_fault)
        if self._closed:
            raise BindingRefusal('BRIDGE_CLOSED')
        x,z,_=self.client.preflight(x,reduced)
        epoch,environment=self.client.witness()
        if epoch!=self.client.epoch or not environment:
            raise BindingRefusal('SOURCE_OR_ENVIRONMENT')
        self._header_received=False
        try:
            self._send(f'PAIR {x:.17e} '+self._floats(z))
            return self._exchange(x,z)
        except BaseException as error:
            # A delayed earlier response must never become a later point's
            # result. End this channel after any interrupted/failed exchange.
            self._transport_fault=str(error)
            if not self._header_received:
                self.work_accounting_state='UNACKNOWLEDGED_CALLBACK_WORK'
            self.close()
            if not isinstance(error,Exception):raise
            if isinstance(error,BindingRefusal):raise
            raise BindingRefusal('BRIDGE_PROTOCOL',str(error)) from error

    def _exchange(self,x,z):
        seen=[]
        witnesses=0
        prior_counts=self.counts
        while True:
            line=self._read(); words=line.split()
            if line=='WITNESS':
                if witnesses>=4:
                    raise BindingRefusal('BRIDGE_WITNESS_ORDER')
                witnesses+=1
                self.witness_calls+=1
                epoch,environment=self.client.witness()
                self._send(f'W {epoch} {int(environment)}')
            elif words and words[0] in ('RHS','JAC'):
                try:
                    expected='RHS' if not seen else 'JAC' if seen==['RHS'] else None
                    if words[0]!=expected or witnesses!=(1 if expected=='RHS' else 3):
                        raise BindingRefusal('BRIDGE_CALLBACK_ORDER')
                    if len(words)!=32:raise BindingRefusal('BRIDGE_REQUEST_SHAPE')
                    sx=float(words[1]);sz=np.array([float(v) for v in words[2:]])
                    if sx.hex()!=x.hex() or not np.array_equal(sz.view(np.uint64),z.view(np.uint64)):
                        raise BindingRefusal('BRIDGE_POINT_CHANGED')
                    values=self.client.rhs(sx,sz) if words[0]=='RHS' else self.client.jacobian(sx,sz)
                    if np.shape(values)!=((30,) if words[0]=='RHS' else (30,30)):
                        raise BindingRefusal('PROVIDER_OUTPUT_SHAPE')
                    self._send('OK '+self._floats(values))
                    seen.append(words[0])
                except Exception as error:
                    self.last_error=repr(error)
                    self._send('ERR FATAL')
            elif len(words)==5 and words[:2]==['RESULT','OK']:
                pending_counts=tuple(int(v) for v in words[2:])
                if pending_counts!=tuple(v+1 for v in prior_counts):
                    raise BindingRefusal('BRIDGE_COUNTER_MISMATCH')
                # Work is charged when observed, independently of delivery.
                self.counts=pending_counts
                self._header_received=True
                self.work_accounting_state='HEADER_OBSERVED_PAYLOAD_INCOMPLETE'
                if seen!=['RHS','JAC'] or witnesses!=4:
                    raise BindingRefusal('BRIDGE_MISSING_CURRENT_CALLBACKS')
                rhs=self._values('F',30)
                jac=self._values('J',900).reshape(30,30)
                self.delivered_pairs+=1
                self.work_accounting_state='OBSERVED'
                for array in (z,rhs,jac):array.setflags(write=False)
                return PreparedPoint(x,z,rhs,jac,self.client.binding_id,self.client.epoch,self.counts)
            elif len(words)==6 and words[:2]==['RESULT','ERR']:
                observed=tuple(int(v) for v in words[3:])
                if any(v<p or v>p+1 for v,p in zip(observed,prior_counts)):
                    raise BindingRefusal('BRIDGE_COUNTER_MISMATCH')
                self.counts=observed
                self._header_received=True
                self.work_accounting_state='OBSERVED'
                raise BindingRefusal('D5_'+words[2],self.last_error or '')
            else:
                raise BindingRefusal('BRIDGE_PROTOCOL',line[:120])

    def close(self):
        self._closed=True
        if self.process.poll() is None:
            try:self._send('QUIT')
            except BindingRefusal:pass
            try:self.process.wait(timeout=1.0)
            except subprocess.TimeoutExpired:
                self.process.kill();self.process.wait(timeout=1.0)
        for stream in (self.process.stdin,self.process.stdout,self.process.stderr):
            stream.close()

    def __enter__(self):return self
    def __exit__(self,*_):self.close()
