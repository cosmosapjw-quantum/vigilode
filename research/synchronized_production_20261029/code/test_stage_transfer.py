from fractions import Fraction as Q
import unittest
from stage_transfer import stage_bound, reconstruction_bound
from defect_composition import Binding, Slab, certify


class StageTransferTests(unittest.TestCase):
    def test_exact_signed_triangular_system(self):
        # Independently solve the signed reference recurrence, then form actual residuals.
        a = [Q(2), Q(3), Q(4)]
        coupling = [[Q(0)]*3, [Q(-2),Q(0),Q(0)], [Q(1),Q(-3),Q(0)]]
        forcing = [Q(1),Q(-2),Q(3)]
        k = []
        for i in range(3):
            k.append((forcing[i]+sum(coupling[i][j]*k[j] for j in range(i)))/a[i])
        for perturb in ([Q(1,100),Q(-1,90),Q(1,80)], [Q(0),Q(0),Q(0)], [Q(-1,10),Q(1,9),Q(-1,8)]):
            kh = [x+e for x,e in zip(k,perturb)]
            d = [abs(forcing[i]+sum(coupling[i][j]*kh[j] for j in range(i))-a[i]*kh[i]) for i in range(3)]
            bound = stage_bound(a,d,[[abs(x) for x in row] for row in coupling])
            self.assertTrue(all(abs(e)<=b for e,b in zip(perturb,bound)))

    def test_sharp_positive_recurrence_and_mutation(self):
        b = stage_bound([2,2],[Q(1,10),0],[[0,0],[4,0]])
        self.assertEqual(b,(Q(1,20),Q(1,10)))
        # Omitting the inherited stage contribution would falsely return zero for stage2.
        self.assertGreater(b[1],Q(0))

    def test_join_to_nonlinear_tube(self):
        eta = stage_bound([2,2],[Q(1,100),Q(1,100)],[[0,0],[1,0]])
        residual = reconstruction_bound(Q(1,100),1,2,2,eta)
        self.assertEqual(residual,Q(1,25))
        binding = Binding('manufactured', 'source1', 'fixed-infinity', 'physical-time', 'S1–S3 exact test')
        slab = Slab(binding,Q(0),Q(1,2),Q(2),Q(0),Q(1),residual,Q(0),Q(1,4),Q(1))
        bound = certify(slab,Q(1,16))
        self.assertLessEqual(bound.whole,Q(1,4))
        # Independently evaluate the invariant-radius polynomial.
        self.assertLessEqual(Q(1,4)**2-2*Q(1,4)+residual,0)

    def test_invalid_or_cyclic_inputs_refused(self):
        for a,d,l in [([0],[0],[[0]]),([1],[-1],[[0]]),([1],[0],[[1]]),([1,1],[0,0],[[0,1],[0,0]])]:
            with self.assertRaises(ValueError): stage_bound(a,d,l)
        with self.assertRaises(TypeError): stage_bound([1.0],[0],[[0]])
        with self.assertRaises(ValueError): reconstruction_bound(0,1,1,1,[])


if __name__ == '__main__':
    unittest.main()
