from fractions import Fraction as F
import unittest, math
from symbolic_oracle import bernstein_on_slab, exact_residual_coefficients, certify, example

class TestSymbolicOracle(unittest.TestCase):
    def test_bernstein_identity_and_residual(self):
        p=(F(1),F(-1),F(1,2)); h=F(1,8)
        self.assertEqual(bernstein_on_slab(p,h),(F(1),F(15,16),F(113,128)))
        self.assertEqual(exact_residual_coefficients(p,F(1),F(0)),(F(0),F(0),F(1,2)))
        self.assertEqual(bernstein_on_slab((F(0),F(0),F(1,2)),h),(F(0),F(0),F(1,128)))

    def test_exact_event_crossing(self):
        c=example()
        self.assertEqual(c.defect_upper,F(1,128))
        self.assertEqual(c.endpoint_error_upper,F(1,1088))
        self.assertEqual(c.endpoint_symbols,("POS","NEG"))
        self.assertEqual(c.flow_direction,"STRICTLY_DECREASING")
        self.assertEqual(c.event_status,"UNIQUE_CROSSING")
        self.assertEqual(c.symbol_at(F(1,16)),"POS")
        self.assertEqual(c.symbol_at(F(1,15)),"NEG")
        self.assertEqual(c.symbol_at(F(33,512)),"UNRESOLVED")
        # Diagnostic only: floating comparison is not part of the exact certificate.
        root=-math.log(15/16)
        self.assertTrue(1/16 < root < 1/15)

    def test_unsafe_one_envelope_and_endpoint_only(self):
        # At E=0, lam=h=delta=1, U1=1/2 but exact forcing is ~0.632.
        self.assertLess(float(F(1,2)),1-math.exp(-1))
        c=certify((F(1),F(-1),F(1,2)),F(1,8),F(1),F(0),F(15,16),F(1,10))
        self.assertEqual(c.endpoint_symbols[0],"UNRESOLVED")
        self.assertEqual(c.event_status,"UNRESOLVED")
        self.assertGreaterEqual(c.whole_slab_error_upper,c.initial_error)

    def test_all_slab_symbol_must_use_interior(self):
        # Positive endpoints of a path do not certify an interior symbol.
        c=certify((F(1),F(-4),F(4)),F(1),F(1),F(0),F(1,2))
        self.assertEqual(c.slab_symbol,"UNRESOLVED")
        self.assertNotEqual(c.event_status,"NO_CROSSING")

    def test_out_of_scope_guard(self):
        with self.assertRaises(TypeError):
            certify((0.1,1),F(1),F(1),F(0),F(0))
        with self.assertRaises(TypeError):
            certify((F(1),),F(1),F(1),F(0),0.5)
        for args in [((F(1),),F(1),F(0),F(0),F(0)),
                     ((F(1),),F(-1),F(1),F(0),F(0)),
                     ((F(1),),F(1),F(-1),F(0),F(0))]:
            with self.assertRaises(ValueError): certify(*args)
        with self.assertRaises(ValueError): example().symbol_at(F(1))

    def test_exact_event_bracket_mutations(self):
        c=example()
        self.assertEqual(c.symbol_at(F(1,16)),"POS")
        self.assertEqual(c.symbol_at(F(1,15)),"NEG")
        # Event at t=0 or t=h cannot be certified as a strict symbol when exactly at threshold.
        edge=certify((F(1),),F(1,8),F(1),F(0),F(1),F(0))
        self.assertEqual(edge.endpoint_symbols[0],"UNRESOLVED")
        # Swapped signed slope must not inherit the monotone-decreasing certificate.
        flipped=certify((F(0),F(1)),F(1,8),F(1),F(0),F(1,16))
        self.assertNotEqual(flipped.flow_direction,"STRICTLY_DECREASING")

    def test_sampled_truth_diagnostics(self):
        from decimal import Decimal, localcontext
        from symbolic_oracle import eval_poly
        def dec(z):
            return Decimal(z.numerator)/Decimal(z.denominator)
        probes=0
        with localcontext() as ctx:
            ctx.prec=70
            for lam in (F(1,2),F(1),F(2)):
                for b in (F(-1,3),F(0),F(1,5)):
                    y0=F(3,5)
                    for shift in (F(0),F(1,100)):
                        slope=-lam*y0+b
                        power=(y0+shift,slope,-lam*slope/2)
                        h=F(1,4)
                        for threshold in (F(1,5),F(3,5),F(4,5)):
                            c=certify(power,h,lam,b,threshold,abs(shift))
                            for j in range(17):
                                t=h*F(j,16)
                                exact=dec(b/lam)+(dec(y0-b/lam)*(-dec(lam*t)).exp())
                                s=c.symbol_at(t)
                                if s=="POS": self.assertGreater(exact,dec(threshold))
                                if s=="NEG": self.assertLess(exact,dec(threshold))
                                err=abs(dec(eval_poly(power,t))-exact)
                                self.assertLessEqual(err,dec(c.whole_slab_error_upper)+Decimal('1e-60'))
                                probes+=1
        self.assertEqual(probes,918)

    def test_source_offset_and_clock(self):
        # y'= -2y+3, constant exact solution 3/2, zero defect.
        c=certify((F(3,2),),F(3,7),F(2),F(3),F(1))
        self.assertEqual(c.defect_upper,F(0))
        self.assertEqual(c.endpoint_error_upper,F(0))
        self.assertEqual(c.slab_symbol,"POS")
        self.assertEqual(c.event_status,"NO_CROSSING")

if __name__=="__main__":unittest.main()

# The extra sampled-truth check below is diagnostic rather than an exact proof;
# only the Fraction certificates establish the admitted bound.
