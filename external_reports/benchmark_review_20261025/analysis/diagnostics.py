"""Read-only cost and endpoint diagnostics; no solver/control changes.

Phase is a principal endpoint angle, not an unwrapped trajectory phase.
All costs must share the same units and timing boundary.
"""
import math


def error_interval(difference, reference_uncertainty):
    if any(not math.isfinite(v) or v < 0 for v in (difference, reference_uncertainty)):
        raise ValueError("finite nonnegative error data required")
    lower = max(0.0, math.nextafter(difference-reference_uncertainty, -math.inf))
    upper = math.nextafter(difference+reference_uncertainty, math.inf)
    if not math.isfinite(upper):
        raise ValueError("error sum exceeds supported range")
    return lower, upper


def endpoint_phase(z, reference, radius_floor=1e-14):
    z, reference = complex(z), complex(reference)
    if not math.isfinite(radius_floor) or radius_floor < 0:
        raise ValueError("invalid radius floor")
    if not all(math.isfinite(v) for v in (z.real, z.imag, reference.real, reference.imag)):
        raise ValueError("nonfinite state")
    # A bounded endpoint diagnostic: square terms must remain representable.
    if max(abs(v) for v in (z.real, z.imag, reference.real, reference.imag)) > 1e150:
        raise ValueError("state exceeds supported diagnostic range (1e150)")
    r, R = abs(z), abs(reference)
    out = {"absolute_error": abs(z-reference), "radial_error": abs(r-R),
           "principal_phase_error": None, "phase_defined": False}
    if min(r, R) <= radius_floor:
        return out
    q = (z/r) * (reference/R).conjugate()
    delta = math.atan2(q.imag, q.real)
    out.update(principal_phase_error=delta, phase_defined=True,
               radial_squared=(r-R)**2,
               angular_squared=4*r*R*math.sin(delta/2)**2)
    return out


def reuse_threshold(setup, candidate, baseline):
    """Smallest positive integer m for strict S + m*C < m*B.

    Float inputs support descriptive measured costs, not rounded interval proofs.
    Candidate and baseline must both include required verification.
    """
    if any(not math.isfinite(v) or v < 0 for v in (setup, candidate, baseline)) or baseline == 0:
        raise ValueError("finite valid costs required")
    margin = baseline-candidate
    if margin <= 0:
        return None
    quotient = setup/margin
    if not math.isfinite(quotient):
        raise ValueError("reuse count exceeds supported float range")
    return max(1, math.floor(quotient)+1)


def end_to_end_speedup(fraction, component_speedup):
    if not 0 <= fraction <= 1 or not math.isfinite(component_speedup) or component_speedup <= 0:
        raise ValueError("invalid cost fractions")
    if fraction == 0:
        return 1.0
    return component_speedup/(component_speedup*(1-fraction)+fraction)
