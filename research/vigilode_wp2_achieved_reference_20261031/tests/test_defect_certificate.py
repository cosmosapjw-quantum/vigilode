"""Independent negative/identity tests for the WP2 exact-rational defect code.

Standalone: python3 -m unittest discover -s tests -v
"""
from fractions import Fraction as F
import math
import os
import sys
import tempfile
from pathlib import Path
import unittest

sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import defect_certifier as d


class DefectTests(unittest.TestCase):
    def source(self):
        return {'n':2,'k':[F(4),F(7)],'g':[F(20),F(40)],
               'c':[[F(0),F(12)],[F(8),F(0)]],
               'y0':[F.from_float(.2),F.from_float(.12)],
               'h':F.from_float(.0015625)}

    def test_polynomial_degree9_bernstein_identity(self):
        for r in range(1,21):
            poly=[F((j+1)*(r-j),j+3) for j in range(10)]
            b=d.coeffs_bernstein9(poly)
            for s in (F(0),F(1,7),F(1,2),F(4,5),F(1)):
                self.assertEqual(d.polynomial_eval(poly,s),d.bernstein9_eval(b,s))
                self.assertLessEqual(abs(d.polynomial_eval(poly,s)),max(map(abs,b)))

    def test_original_cubic_defect_independently_reassembled(self):
        src=self.source(); a=src['y0']; b=[F.from_float(.19),F.from_float(.11)]
        coeffs,p=d.original_defect_coeffs(src,a,b,src['h'])
        R=d.defect_panel(src,a,b,src['h'],audit=True)
        self.assertGreater(R,0)
        for s in (F(0),F(1,3),F(2,3),F(1)):
            xs=[d.polynomial_eval(y,s) for y in p]
            f=d.rhs_rational(xs,src)
            deriv=[d.polynomial_eval([j*y[j]/src['h'] for j in range(1,4)],s) for y in p]
            for i in range(2):
                self.assertEqual(deriv[i]-f[i],d.polynomial_eval(coeffs[i],s))
                self.assertLessEqual(abs(deriv[i]-f[i]),R)

    def test_hermite_endpoint_defect_is_zero_but_interior_nonzero(self):
        src=self.source();a=src['y0'];b=[F.from_float(.19),F.from_float(.11)]
        c,_=d.original_defect_coeffs(src,a,b,src['h'])
        for panel in c:
            self.assertEqual(d.polynomial_eval(panel,F(0)),0)
            self.assertEqual(d.polynomial_eval(panel,F(1)),0)
        self.assertTrue(any(d.polynomial_eval(panel,F(1,2)) for panel in c))

    def test_upper_binary64_exact(self):
        for q in (F(1,10),F(1,3),F(1,2**1020),F(7,10**20),F(1),F(3,2)):
            u=d.upper_as_float(q)
            self.assertGreaterEqual(F.from_float(u),q)
            if u>0:
                lower=math.nextafter(u,-math.inf)
                self.assertLess(F.from_float(lower),q)

    def test_tampered_reference_forces_larger_defect(self):
        src=self.source()
        original,_=d.generate_reference(src,2,2)
        baseline=d.certify_reference(src,2,2,original)
        fake=list(original)
        fake[1]=(d.f_to_bits(0.95),d.f_to_bits(0.12))
        bad=d.certify_reference(src,2,2,fake)
        self.assertGreater(bad['certified_delta_grid_exact'],baseline['certified_delta_grid_exact'])
        self.assertGreater(bad['certified_delta_grid'],0.01)

    def test_zero_reference_lower_fails_relative_parity(self):
        a={'lower':F(0),'upper':F(1)}
        b={'lower':F(0),'upper':F(1)}
        self.assertEqual(d.pair_certificate(a,b),(False,None))

    def test_malicious_parity_ratio_fails(self):
        a={'lower':F(1,100),'upper':F(101,10000)}
        b={'lower':F(2,100),'upper':F(201,10000)}
        self.assertFalse(d.pair_certificate(a,b)[0])

    def test_exact_interarm_reverse_triangle(self):
        A=[{'state':[d.f_to_bits(.1),d.f_to_bits(.2)]}]
        B=[{'state':[d.f_to_bits(.15),d.f_to_bits(.2)]}]
        delta=d.exact_crossarm_delta({'points':A},{'points':B})
        self.assertEqual(delta,abs(F.from_float(.1)-F.from_float(.15)))

    def test_source_log_hash_rejection(self):
        p=Path(os.environ.get('WP1_SWEEP_LOG','/mnt/data/vigilode_wp2_reference/SWEEP.log'))
        if not p.exists():self.skipTest('standalone source artifact not present; workflow checks SHA')
        b=p.read_bytes()
        self.assertEqual(d.hashlib.sha256(b).hexdigest(),d.EXPECTED_SWEEP_SHA256)
        with tempfile.TemporaryDirectory() as tmp:
            fake=Path(tmp)/'SWEEP.log';fake.write_bytes(b+b'!')
            with self.assertRaisesRegex(AssertionError,'WP1_IMMUTABLE_SOURCE_LOG_CHANGED'):
                d.run_analysis(fake,tmp,os.environ.get('WP1_SUMMARY','/mnt/data/vigilode_wp2_reference/WP_SUMMARY.json'),micros=(32,))

if __name__=='__main__':unittest.main()
