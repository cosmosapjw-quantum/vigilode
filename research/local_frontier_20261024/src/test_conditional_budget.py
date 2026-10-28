import itertools
from fractions import Fraction as F
import unittest
from conditional_budget import weights, allocate

class ConditionalBudgetTests(unittest.TestCase):
    def test_causal_residual_propagation(self):
        k=[[0,0,0],[F(1,2),0,0],[F(1,3),F(1,4),0]];b=[1,-2,3];m=[2,1,4];r=[F(1,100),F(1,20),F(1,7)]
        d=[]
        for i in range(3): d.append(m[i]*r[i]+sum(k[i][j]*d[j] for j in range(i)))
        self.assertEqual(sum(weights(k,b,m)[i]*r[i] for i in range(3)),sum(abs(b[i])*d[i] for i in range(3)))
    def test_exact_optimum_against_exhaustive(self):
        for target in [F(1,2),F(1,5),F(1,10),F(1,30)]:
            w=[F(1,10),1,2];q=[F(1,2),F(1,3),F(1,2)];cost=[1,3,2];caps=[5,5,5]
            result=allocate(w,[1]*3,[0]*3,q,cost,caps,target)
            exact=[(sum(cost[i]*ks[i]for i in range(3)),ks)for ks in itertools.product(range(6),repeat=3)if sum(w[i]*q[i]**ks[i]for i in range(3))<=target]
            self.assertEqual(None if result is None else result['declared_work_units'],min(exact)[0]if exact else None)
    def test_uncertainty_floor(self):
        self.assertIsNone(allocate([1],[1],[F(1,10)],[F(1,2)],[1],[128],F(1,100)))
    def test_work_cap(self):
        self.assertIsNone(allocate([1],[1],[0],[F(1,2)],[3],[12],F(1,100),max_work=3))
    def test_zero_weight_needs_no_work(self):
        self.assertEqual(allocate([0],[1],[0],[F(1,2)],[3],[12],F(1,100))['declared_work_units'],0)
    def test_invalid_structure(self):
        with self.assertRaises(ValueError):weights([[1]],[1],[1])
        with self.assertRaises(ValueError):allocate([1],[1],[0],[1],[1],[12],F(1,100))
    def test_weighted_improvement_over_uniform(self):
        w=[F(1,100),1,10];q=[F(1,2)]*3;target=F(1,100)
        result=allocate(w,[1]*3,[0]*3,q,[1]*3,[16]*3,target)
        uniform=next(k for k in range(17)if sum(w)*F(1,2)**k<=target)
        self.assertLess(result['declared_work_units'],3*uniform)

if __name__=='__main__':unittest.main()
