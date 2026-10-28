import math
import unittest
import numpy as np
from spherical_mesh import EntropyGrid,power_bregman,profiles_and_kernels,joint_mb_check


class ContractedEntropyTests(unittest.TestCase):
    def test_constant_has_zero_loss(self):
        grid=EntropyGrid(np.ones((8,16))*1.25)
        self.assertEqual(grid.root.entropy,0)
        self.assertEqual(len(grid.refine_to_budgets([0])[0][1]),1)

    def test_zero_values_and_zero_state(self):
        np.testing.assert_allclose(power_bregman(np.array([0.,1.]),1),[.25,0])
        grid=EntropyGrid(np.zeros((4,4)))
        self.assertEqual(grid.root.entropy,0)

    def test_small_variation_resolves_fisher_limit(self):
        x=np.linspace(-1,1,256).reshape(16,16)
        eps=1e-8;rho=1+eps*x;grid=EntropyGrid(rho)
        expected=(3/32)*grid.weight*float(np.sum((rho-rho.mean())**2))
        self.assertGreater(grid.root.entropy,0)
        self.assertLess(abs(grid.root.entropy/expected-1),1e-7)

    def test_nested_loss_telescope(self):
        rho=np.array([[.2,.7,2.,3.],[.3,.5,1.7,4.],[.4,.9,5.,8.],[.8,1.3,6.,9.]])
        grid=EntropyGrid(rho);children=grid.children(grid.root)
        increment=sum(c.area*float(power_bregman(np.array(c.mean),grid.root.mean)) for c in children)
        self.assertAlmostEqual(grid.root.entropy,sum(c.entropy for c in children)+increment,places=12)

    def test_energy_conserved_number_drift_recorded(self):
        rho=np.array([[.3,.6],[2.,4.]])
        grid=EntropyGrid(rho);out=grid.evaluate([grid.root],{'x':np.array([[-1.,1.],[-1.,1.]])})
        self.assertLess(out['energy_relative_drift'],1e-15)
        self.assertGreater(out['number_relative_drift'],0)
        self.assertLess(abs(out['number_drift_entropy_identity_residual']),1e-13)

    def test_budget_and_observable_certificate(self):
        profiles,kernels=profiles_and_kernels(16,32)
        for rho in profiles.values():
            grid=EntropyGrid(rho);budget=grid.root.entropy*.03
            _,cells=grid.refine_to_budgets([budget])[0]
            out=grid.evaluate(cells,kernels)
            self.assertLessEqual(out['entropy_loss'],budget)
            self.assertTrue(all(o['certified'] for o in out['observables'].values()))
            self.assertLess(out['energy_relative_drift'],2e-14)

    def test_joint_number_energy(self):
        r=joint_mb_check()
        self.assertGreater(r['minimum_pointwise_divergence'],0)
        self.assertGreater(min(r['hessian_eigenvalues']),0)
        self.assertAlmostEqual(r['jensen_loss'],r['bregman_loss'],places=13)
        self.assertLess(abs(r['number_conservation_error']),1e-15)
        self.assertLess(abs(r['energy_conservation_error']),1e-15)

    def test_invalid_input_rejected(self):
        for values in ([[1,-1]],[[float('nan')]],[]):
            with self.assertRaises(ValueError):EntropyGrid(values)
        with self.assertRaises(ValueError):power_bregman([1],0)


if __name__=='__main__':unittest.main(verbosity=2)
