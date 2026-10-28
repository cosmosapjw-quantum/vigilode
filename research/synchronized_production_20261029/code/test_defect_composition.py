"""Independent controls for D1–D10; finite diagnostics are not proofs."""

import unittest
from dataclasses import replace
from decimal import Decimal, localcontext
from fractions import Fraction as Q

from defect_composition import (
    Binding, Slab, Transfer, ContractError, RadiusRejected,
    certify, compose, norm_transport, event_sign, crossing,
)


BINDING = Binding("manufactured-scalar", "exact-v1", "absolute", "t", "D1-test")


def slab(**changes):
    values = dict(
        binding=BINDING, t0=Q(0), t1=Q(1, 2),
        lambda_carrier=Q(2), carrier_mismatch=Q(0), gamma=Q(1),
        residual=Q(1, 8), source_offset=Q(0), radius=Q(1, 4),
        domain_margin=Q(1),
    )
    values.update(changes)
    return Slab(**values)


def decimal(q):
    return Decimal(q.numerator) / Decimal(q.denominator)


class DefectCompositionTests(unittest.TestCase):
    def test_exact_barrier_and_decay(self):
        b = certify(slab(), Q(1, 16))
        self.assertEqual(b.barrier_margin, Q(5, 16))
        self.assertEqual(b.beta, Q(7, 4))
        self.assertEqual(b.decay_lower, Q(9, 23))
        self.assertEqual(b.decay_upper, Q(128, 289))
        self.assertEqual(b.endpoint, Q(25, 368))
        self.assertEqual(b.whole, b.endpoint)

    def test_tangent_quadratic_exact_reference(self):
        # Original scalar ODE x'=(1-x)^2, x(0)=0; exact x=t/(1+t).
        # p=0, mu(A)=-2, N=x^2, residual=1, R=1, q(R)=0.
        for h in (Q(1, 16), Q(1, 2), Q(1), Q(2), Q(8)):
            s = slab(t1=h, residual=Q(1), radius=Q(1), domain_margin=Q(2))
            b = certify(s, Q(0))
            self.assertEqual(b.barrier_margin, 0)
            self.assertLessEqual(h / (1 + h), b.endpoint)
            self.assertLessEqual(b.whole, 1)
            for t in (Q(0), h / 3, 2 * h / 3, h):
                x = t / (1 + t)
                self.assertEqual(1 / (1 + t) ** 2, (1 - x) ** 2)
                self.assertLessEqual(x, b.whole)

    def test_incoming_maximum_cannot_be_dropped(self):
        s = slab(gamma=Q(0), residual=Q(0), radius=Q(1, 2))
        b = certify(s, Q(1, 4))
        self.assertLess(b.endpoint, b.incoming)
        self.assertEqual(b.whole, b.incoming)

    def test_carrier_and_source_terms_change_admission(self):
        with self.assertRaises(RadiusRejected):
            certify(slab(carrier_mismatch=Q(3), gamma=Q(0)), Q(0))
        certify(slab(carrier_mismatch=Q(0), gamma=Q(0)), Q(0))
        with self.assertRaises(RadiusRejected):
            certify(slab(residual=Q(0), source_offset=Q(1)), Q(0))
        certify(slab(residual=Q(0), source_offset=Q(0)), Q(0))
        with self.assertRaises(RadiusRejected):
            certify(slab(residual=Q(1, 2), radius=Q(2), domain_margin=Q(3)), Q(0))

    def test_zero_radius_and_zero_decay(self):
        exact = slab(lambda_carrier=Q(-5), residual=Q(0), radius=Q(0))
        self.assertEqual(certify(exact, Q(0)).whole, 0)
        s = slab(lambda_carrier=Q(0), gamma=Q(0), residual=Q(0))
        self.assertEqual(certify(s, Q(1, 8)).endpoint, Q(1, 8))
        tangent = slab(lambda_carrier=Q(1), gamma=Q(1), residual=Q(0),
                       radius=Q(1), domain_margin=Q(2))
        self.assertEqual(certify(tangent, Q(1)).endpoint, Q(1))

    def test_invalid_inputs_domain_and_float_rejected(self):
        for changes in (
            {"gamma": Q(-1)}, {"source_offset": Q(-1)},
            {"t1": Q(0)}, {"radius": Q(-1)}, {"gamma": 0.0},
            {"gamma": True}, {"domain_margin": Q(1, 4)},
        ):
            with self.assertRaises(ContractError):
                slab(**changes)
        for incoming in (Q(-1), Q(1, 2), 0.0, True):
            with self.assertRaises(ContractError):
                certify(slab(), incoming)
        with self.assertRaises(ContractError):
            Binding("", "v1", "norm", "t", "test")

    def test_decay_envelope_decimal_diagnostic(self):
        # Decimal.exp is only a high-precision diagnostic, not interval proof.
        with localcontext() as ctx:
            ctx.prec = 80
            for gamma in (Q(0), Q(1)):
                for initial in (Q(0), Q(1, 16), Q(1, 4)):
                    for h in (Q(1, 1024), Q(1, 2), Q(2), Q(8)):
                        b = certify(slab(gamma=gamma, t1=h), initial)
                        q = (-decimal(b.beta * h)).exp()
                        self.assertLessEqual(decimal(b.decay_lower), q)
                        self.assertLessEqual(q, decimal(b.decay_upper))
                        equilibrium = b.delta / b.beta
                        if initial == equilibrium:
                            # This comparison IVP is exactly stationary. A pair
                            # of rounded Decimal products is not its authority.
                            self.assertEqual(b.endpoint, initial)
                            self.assertEqual(b.whole, initial)
                            ref = decimal(initial)
                        else:
                            # Form the cancellation-prone amplitude exactly.
                            ref = decimal(equilibrium) + q * decimal(initial - equilibrium)
                        self.assertLessEqual(ref, decimal(b.endpoint))
                        self.assertLessEqual(max(decimal(initial), ref), decimal(b.whole))

    def test_stationary_reference_rejects_subdecimal_corruption(self):
        # Independently known constant solutions of E'=-beta E+delta.
        # Exact Fraction checks must reject defects hidden at Decimal80.
        with localcontext() as ctx:
            ctx.prec = 80
            for gamma, constant in ((Q(0), Q(1, 16)), (Q(1), Q(1, 14))):
                for h in (Q(1, 1024), Q(1, 2), Q(2), Q(8)):
                    b = certify(slab(gamma=gamma, t1=h), constant)
                    self.assertEqual(-b.beta * constant + b.delta, 0)
                    self.assertTrue(constant <= b.endpoint and constant <= b.whole)
                    for field in ("endpoint", "whole"):
                        corrupted = replace(b, **{field: getattr(b, field) - Q(1, 2**300)})
                        self.assertEqual(decimal(constant), decimal(getattr(corrupted, field)))
                        self.assertFalse(constant <= corrupted.endpoint and constant <= corrupted.whole)

    def test_bound_monotone_in_incoming(self):
        bounds = [certify(slab(), Q(i, 128)).endpoint for i in range(33)]
        self.assertEqual(bounds, sorted(bounds))

    def test_composition_carries_epoch_norm_and_jump(self):
        a = slab()
        next_binding = replace(BINDING, source_epoch="exact-v2", norm="scaled")
        b = slab(binding=next_binding, t0=a.t1, t1=Q(1))
        transfer = Transfer(a.binding, b.binding, Q(2), Q(1, 100), "D4-test")
        records = compose((a, b), (transfer,), Q(1, 16))
        self.assertEqual(records[1].incoming, 2 * records[0].endpoint + Q(1, 100))
        with self.assertRaises(RadiusRejected):
            compose((a, b), (replace(transfer, eta=Q(1)),), Q(1, 16))
        with self.assertRaises(ContractError):
            compose((a, b), (), Q(0))
        with self.assertRaises(ContractError):
            compose((a, object()), (transfer,), Q(0))
        with self.assertRaises(ContractError):
            compose((a, replace(b, t0=Q(3, 4))), (transfer,), Q(0))
        with self.assertRaises(ContractError):
            compose((a, b), (replace(transfer, new=a.binding),), Q(0))
        with self.assertRaises(ContractError):
            Transfer(BINDING, replace(BINDING, clock="ln(a)"), Q(1), Q(0), "bad")

    def test_norm_ratio_and_noncommutation(self):
        k = norm_transport((Q(2), Q(3)), (Q(1), Q(6)))
        self.assertEqual(k, Q(2))
        v = (Q(2), Q(0))
        old = max(abs(v[0]) / 2, abs(v[1]) / 3)
        new = max(abs(v[0]), abs(v[1]) / 6)
        self.assertEqual(new, k * old)
        q, e, d, eta = Q(1, 2), Q(1, 10), Q(1, 20), Q(1, 8)
        flow_transfer = q * (k * e + eta) + d
        transfer_flow = k * (q * e + d) + eta
        self.assertEqual(flow_transfer-transfer_flow, (q-1)*eta+(1-k)*d)
        self.assertNotEqual(flow_transfer, transfer_flow)

    def test_event_strictness_and_transversality(self):
        self.assertEqual(event_sign(Q(1, 4), Q(1, 2), Q(1, 8), Q(1)), "POS")
        self.assertEqual(event_sign(Q(-1, 2), Q(-1, 4), Q(1, 8), Q(1)), "NEG")
        self.assertEqual(event_sign(Q(1, 8), Q(1, 8), Q(1, 8), Q(1)), "UNRESOLVED")
        self.assertEqual(crossing("NEG", "POS", Q(1), Q(2)), "UNIQUE_CROSSING")
        self.assertEqual(crossing("NEG", "POS", Q(0), Q(2)), "AT_LEAST_ONE_CROSSING")
        self.assertEqual(crossing("POS", "POS", Q(1), Q(2)), "UNRESOLVED")
        with self.assertRaises(ContractError):
            crossing("NEG", "POS", Q(-2), Q(-1))


if __name__ == "__main__":
    unittest.main()
