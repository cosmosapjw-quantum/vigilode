"""Exact consumer of supplied scalar hypotheses in theory/DEFECT.md.

This module checks algebraic consistency of declared bounds. It does not verify
that an RHS, source, metric, clock, or physical domain satisfies those bounds.
It is deliberately independent of the native Rust dispatcher and accepts no
binary floating input. Domain inclusion and regularity remain provider premises.
"""

from dataclasses import dataclass
from fractions import Fraction as Q


class ContractError(ValueError):
    """Invalid or incompatible supplied data."""


class RadiusRejected(ContractError):
    """The declared radius does not satisfy the sufficient admission rule."""


def exact(value, name):
    if isinstance(value, bool) or not isinstance(value, (int, Q)):
        raise ContractError(f"{name} requires an exact int or Fraction")
    return Q(value)


def nonnegative(value, name):
    value = exact(value, name)
    if value < 0:
        raise ContractError(f"{name} must be nonnegative")
    return value


@dataclass(frozen=True)
class Binding:
    target: str
    source_epoch: str
    norm: str
    clock: str
    premise_locator: str

    def __post_init__(self):
        for name, value in vars(self).items():
            if not isinstance(value, str) or not value.strip():
                raise ContractError(f"binding {name} must be a nonempty string")


@dataclass(frozen=True)
class Slab:
    binding: Binding
    t0: Q
    t1: Q
    lambda_carrier: Q
    carrier_mismatch: Q
    gamma: Q
    residual: Q
    source_offset: Q
    radius: Q
    domain_margin: Q

    def __post_init__(self):
        if not isinstance(self.binding, Binding):
            raise ContractError("binding must be a Binding")
        for name in ("t0", "t1", "lambda_carrier"):
            object.__setattr__(self, name, exact(getattr(self, name), name))
        for name in (
            "carrier_mismatch", "gamma", "residual", "source_offset",
            "radius", "domain_margin",
        ):
            object.__setattr__(self, name, nonnegative(getattr(self, name), name))
        if self.t1 <= self.t0:
            raise ContractError("a slab must have positive physical duration")
        if self.domain_margin <= self.radius:
            raise ContractError("declared tube must lie strictly inside domain margin")


@dataclass(frozen=True)
class Bound:
    slab: Slab
    incoming: Q
    alpha: Q
    delta: Q
    barrier_margin: Q
    beta: Q
    decay_lower: Q | None
    decay_upper: Q | None
    endpoint: Q
    whole: Q
    scope: str = "conditional_scalar_contract_only"


def certify(slab: Slab, incoming) -> Bound:
    """D2/D3: admit one declared tube and enclose its comparison equation."""
    if not isinstance(slab, Slab):
        raise ContractError("slab must be a Slab")
    e = nonnegative(incoming, "incoming")
    r = slab.radius
    if e > r:
        raise RadiusRejected("incoming error exceeds declared radius")
    alpha = slab.lambda_carrier - slab.carrier_mismatch
    delta = slab.residual + slab.source_offset
    margin = alpha * r - slab.gamma * r * r - delta
    if margin < 0:
        raise RadiusRejected("quadratic radius inequality fails")
    beta = alpha - slab.gamma * r
    if r == 0:
        # Feasibility implies e=delta=0; beta need not be positive.
        return Bound(slab, e, alpha, delta, margin, beta, None, None, Q(0), Q(0))
    if beta == 0:
        # Feasibility at positive r implies delta=0.
        return Bound(slab, e, alpha, delta, margin, beta, Q(1), Q(1), e, e)
    if beta < 0:
        raise AssertionError("positive feasible radius implies nonnegative beta")
    z = beta * (slab.t1 - slab.t0)
    qlo = max(Q(0), (1 - z / 2) / (1 + z / 2))
    qhi = 1 / (1 + z + z * z / 2)
    equilibrium = delta / beta
    upper = max(
        qlo * e + (1 - qlo) * equilibrium,
        qhi * e + (1 - qhi) * equilibrium,
    )
    endpoint = min(r, upper)
    return Bound(
        slab, e, alpha, delta, margin, beta, qlo, qhi,
        endpoint, max(e, endpoint),
    )


@dataclass(frozen=True)
class Transfer:
    old: Binding
    new: Binding
    kappa: Q
    eta: Q
    premise_locator: str

    def __post_init__(self):
        if not isinstance(self.old, Binding) or not isinstance(self.new, Binding):
            raise ContractError("transfer endpoints must be Bindings")
        object.__setattr__(self, "kappa", nonnegative(self.kappa, "kappa"))
        object.__setattr__(self, "eta", nonnegative(self.eta, "eta"))
        if not isinstance(self.premise_locator, str) or not self.premise_locator.strip():
            raise ContractError("transfer requires its own premise locator")
        if self.old.target != self.new.target or self.old.clock != self.new.clock:
            raise ContractError("target or clock changes need an external transformation")

    def apply(self, error):
        return self.kappa * nonnegative(error, "error") + self.eta


def compose(slabs, transfers, incoming):
    """D4: chronological flow/transfer composition with explicit bindings.

    One transfer is required even for an identity route change. This prevents
    norm/source-epoch changes from silently inheriting an untransported error.
    No maximum of errors in different norms is returned.
    """
    slabs, transfers = tuple(slabs), tuple(transfers)
    if not slabs or len(transfers) != len(slabs) - 1:
        raise ContractError("need one explicit transfer per slab boundary")
    if any(not isinstance(slab, Slab) for slab in slabs):
        raise ContractError("all composition entries must be Slabs")
    records = []
    e = nonnegative(incoming, "incoming")
    for index, slab in enumerate(slabs):
        if index:
            previous = slabs[index - 1]
            transfer = transfers[index - 1]
            if not isinstance(transfer, Transfer):
                raise ContractError("boundary record must be a Transfer")
            if previous.t1 != slab.t0:
                raise ContractError("physical clocks do not meet exactly")
            if transfer.old != previous.binding or transfer.new != slab.binding:
                raise ContractError("transfer binding does not match adjacent slabs")
            e = transfer.apply(records[-1].endpoint)
        records.append(certify(slab, e))
    return tuple(records)


def norm_transport(old_weights, new_weights):
    """Identity map for norms max_i |x_i|/w_i, with positive exact weights."""
    old_weights, new_weights = tuple(old_weights), tuple(new_weights)
    if not old_weights or len(old_weights) != len(new_weights):
        raise ContractError("positive equal-dimensional weights required")
    old = [exact(v, "old_weight") for v in old_weights]
    new = [exact(v, "new_weight") for v in new_weights]
    if min(old + new) <= 0:
        raise ContractError("weights must be strictly positive")
    return max(a / b for a, b in zip(old, new))


def event_sign(candidate_lower, candidate_upper, error, lipschitz):
    """D5: strict separation only, using supplied continuous/event bounds."""
    lo = exact(candidate_lower, "candidate_lower")
    hi = exact(candidate_upper, "candidate_upper")
    if lo > hi:
        raise ContractError("reversed candidate event interval")
    padding = nonnegative(error, "error") * nonnegative(lipschitz, "lipschitz")
    if lo - padding > 0:
        return "POS"
    if hi + padding < 0:
        return "NEG"
    return "UNRESOLVED"


def crossing(left_sign, right_sign, lie_lower, lie_upper):
    """Certified opposite endpoint signs plus original-field transversality."""
    if left_sign not in {"POS", "NEG", "UNRESOLVED"} or right_sign not in {
        "POS", "NEG", "UNRESOLVED"
    }:
        raise ContractError("unknown endpoint symbol")
    lo, hi = exact(lie_lower, "lie_lower"), exact(lie_upper, "lie_upper")
    if lo > hi:
        raise ContractError("reversed Lie derivative interval")
    opposite = {left_sign, right_sign} == {"POS", "NEG"}
    if not opposite:
        return "UNRESOLVED"
    if (left_sign == "NEG" and hi < 0) or (left_sign == "POS" and lo > 0):
        raise ContractError("endpoint symbols contradict derivative direction")
    if lo > 0 or hi < 0:
        return "UNIQUE_CROSSING"
    return "AT_LEAST_ONE_CROSSING"
