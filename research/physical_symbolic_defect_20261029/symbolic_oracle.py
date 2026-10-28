"""Exact-rational, scoped scalar ODE/observable itinerary oracle.

Scope: y' = -lam*y + b, lam>0 constant, p(s) a power-basis polynomial on
s in [0,h], exact rational input, event observable g(y)=y-threshold.
No floating-point output is used to decide an admission status.
This is a research oracle, NOT a VigilODE production integration.
"""
from __future__ import annotations
from dataclasses import dataclass
from fractions import Fraction as F
from math import comb
from typing import Sequence


def _rational(x: F | int) -> F:
    # Prevent silent float -> represented-binary-fraction conversion.
    if isinstance(x, bool) or not isinstance(x, (F, int)):
        raise TypeError("Exact oracle accepts only int or fractions.Fraction; supply an explicit source conversion")
    return F(x)


def bernstein_on_slab(power: Sequence[F], h: F) -> tuple[F, ...]:
    """Bernstein coefficients on s=h*theta, 0<=theta<=1, exactly."""
    if h <= 0 or not power:
        raise ValueError("positive slab width and nonempty polynomial required")
    d = len(power)-1
    return tuple(sum((power[j] * h**j * F(comb(k,j),comb(d,j))
                      for j in range(k+1)), F(0)) for k in range(d+1))


def eval_poly(power: Sequence[F], s: F) -> F:
    acc = F(0)
    for a in reversed(power):
        acc = acc*s+a
    return acc


def exact_residual_coefficients(power: Sequence[F], lam: F, b: F) -> tuple[F,...]:
    # r = p' + lam*p - b, in the SAME time and source clock.
    return tuple((F(k+1)*power[k+1] if k+1<len(power) else F(0))+
                 lam*power[k]-(b if k==0 else F(0)) for k in range(len(power)))


def signed_symbol(value_minus_threshold: F, error: F) -> str:
    if error < 0: raise ValueError("error cannot be negative")
    if value_minus_threshold > error: return "POS"
    if value_minus_threshold < -error: return "NEG"
    return "UNRESOLVED"


@dataclass(frozen=True)
class Certificate:
    h: F
    lam: F
    b: F
    power: tuple[F,...]
    threshold: F
    initial_error: F
    defect_upper: F
    endpoint_error_upper: F
    whole_slab_error_upper: F
    endpoint_symbols: tuple[str,str]
    slab_symbol: str
    flow_direction: str
    event_status: str

    def symbol_at(self,s:F)->str:
        s=_rational(s)
        if not F(0)<=s<=self.h: raise ValueError("sample outside slab")
        e=self.initial_error if s==0 else self.endpoint_error_upper if s==self.h else self.whole_slab_error_upper
        return signed_symbol(eval_poly(self.power,s)-self.threshold,e)


def certify(power: Sequence[F], h: F, lam: F, b: F,
            threshold: F, initial_error: F = F(0)) -> Certificate:
    power=tuple(_rational(a) for a in power)
    h,lam,b,threshold,initial_error=map(_rational,(h,lam,b,threshold,initial_error))
    if lam<=0 or h<=0 or initial_error<0 or not power:
        raise ValueError("This certificate needs lam>0, h>0, nonnegative incoming error and polynomial")

    residual=exact_residual_coefficients(power,lam,b)
    residual_bern=bernstein_on_slab(residual,h)
    delta=max(map(abs,residual_bern)) # exact all-time upper via Bernstein convex hull
    z=lam*h
    forcing=h*delta
    u1=(initial_error+forcing)/(1+z)
    u2=((1-z/2)*initial_error+forcing)/(1+z/2)
    end=max(u1,u2)
    whole=max(initial_error,end) # exact majorant is monotone on the slab

    pbern=bernstein_on_slab(power,h)
    t0=signed_symbol(power[0]-threshold, initial_error)
    t1=signed_symbol(eval_poly(power,h)-threshold,end)
    if min(pbern)-threshold > whole:
        s="POS"
    elif max(pbern)-threshold < -whole:
        s="NEG"
    else:
        s="UNRESOLVED"
    lower=min(pbern)-whole
    upper=max(pbern)+whole
    equilibrium=b/lam
    if lower>equilibrium:
        direction="STRICTLY_DECREASING"
    elif upper<equilibrium:
        direction="STRICTLY_INCREASING"
    else:
        direction="NOT_CERTIFIED"
    if t0 != "UNRESOLVED" and t1 != "UNRESOLVED" and t0!=t1:
        status="UNIQUE_CROSSING" if direction!="NOT_CERTIFIED" else "AT_LEAST_ONE_CROSSING"
    elif s!="UNRESOLVED":
        status="NO_CROSSING"
    elif t0==t1 and t0!="UNRESOLVED" and direction!="NOT_CERTIFIED":
        status="NO_CROSSING"
    else:
        status="UNRESOLVED"
    return Certificate(h,lam,b,power,threshold,initial_error,delta,end,whole,(t0,t1),s,direction,status)


def fraction_json(x:F)->str:
    return str(x.numerator)+"/"+str(x.denominator)


def example():
    return certify((F(1),F(-1),F(1,2)),F(1,8),F(1),F(0),F(15,16))


if __name__=="__main__":
    import json
    c=example()
    obj={"scope":"SCALAR_AFFINE_ANALYTIC_RESEARCH_ONLY", "h":fraction_json(c.h),
         "defect_upper":fraction_json(c.defect_upper),
         "endpoint_error_upper":fraction_json(c.endpoint_error_upper),
         "whole_slab_error_upper":fraction_json(c.whole_slab_error_upper),
         "endpoint_symbols":c.endpoint_symbols,
         "sample_1_16":c.symbol_at(F(1,16)),
         "sample_1_15":c.symbol_at(F(1,15)),
         "sample_33_512":c.symbol_at(F(33,512)),
         "flow_direction":c.flow_direction,
         "event_status":c.event_status}
    print(json.dumps(obj,indent=2))
