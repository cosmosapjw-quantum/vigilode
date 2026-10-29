"""Independent lightweight checks of noncyclic Fourier products and chart phase.

The DFT is a small mathematical oracle, not a new production FFT implementation.
"""
import cmath
import json
import math
from pathlib import Path
import unittest


def product(a, b):
    out = {}
    for k, x in a.items():
        for j, y in b.items():
            out[k+j] = out.get(k+j, 0) + x*y
    return out


def involution(a):
    return {-k: v.conjugate() for k, v in a.items()}


def cyclic_product(a, b, n):
    # Evaluate on roots of unity, multiply, and invert: explicit DFT oracle.
    values = []
    for j in range(n):
        root = cmath.exp(2j*math.pi*j/n)
        values.append(sum(v*root**k for k,v in a.items()) * sum(v*root**k for k,v in b.items()))
    return {k: sum(v*cmath.exp(-2j*math.pi*j*k/n) for j,v in enumerate(values))/n for k in range(n)}


class AlgebraChecks(unittest.TestCase):
    def test_involution_preserves_full_convolution(self):
        a={-2:1+2j,1:-0.3j}; b={0:0.4,3:1-2j}
        self.assertEqual(involution(product(a,b)),product(involution(a),involution(b)))

    def test_padded_transform_matches_noncyclic_coefficients(self):
        a={-1:0.4+0.2j,0:0.1,1:1-0.1j}
        square=product(a,a); direct=product(square,a)
        fft=cyclic_product(square,a,8)
        self.assertLess(max(abs(v-fft[k%8]) for k,v in direct.items()),1e-14)

    def test_harmful_alias_negative_control(self):
        # Genuine high mode folds to retained -1; not an ineffective tiny-tail control.
        a={1:1+0j}; direct=product(product(a,a),a)
        fft=cyclic_product(product(a,a),a,4)
        self.assertEqual(direct.get(-1,0),0)
        self.assertGreater(abs(fft[3]-direct.get(-1,0)),0.99)

    def test_equivariance_charge_rule(self):
        z=0.3+0.7j; phi=0.41; e=cmath.exp(1j*phi)
        for p,q in [(1,0),(2,1),(0,2),(3,0)]:
            transformed=e*(e.conjugate()*z)**p*(e*z.conjugate())**q
            expected=cmath.exp(1j*(1-p+q)*phi)*z**p*z.conjugate()**q
            self.assertLess(abs(transformed-expected),1e-14)

    def test_exact_binary_parameter_phase_against_80_digits(self):
        from decimal import Decimal, localcontext
        beta=1.2; t=8.0
        double=math.expm1(beta*t)/beta
        with localcontext() as ctx:
            ctx.prec=80
            high=((Decimal(beta)*Decimal(t)).exp()-1)/Decimal(beta)
            self.assertLess(abs(Decimal(double)-high),Decimal('2e-11'))


if __name__ == '__main__':
    suite=unittest.defaultTestLoader.loadTestsFromTestCase(AlgebraChecks)
    result=unittest.TextTestRunner(verbosity=2).run(suite)
    Path(__file__).with_name('ALGEBRA_CHECKS.json').write_text(json.dumps({
        'tests_run':result.testsRun,'failures':len(result.failures),'errors':len(result.errors),
        'scope':'new chart phase, charge decomposition, noncyclic product and harmful alias control',
        'status':'PASS' if result.wasSuccessful() else 'FAIL'},indent=2)+'\n')
    raise SystemExit(0 if result.wasSuccessful() else 1)
