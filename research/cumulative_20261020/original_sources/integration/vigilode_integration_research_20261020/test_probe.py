"""Bounded mathematical/implementation checks for the exploratory prototype."""
import unittest,json
import numpy as np
from probe import A,C,G,BOUT,Toy,arnoldi_affine,nilpotent_demo,determinant_test,newton_path
from projected_parallel import ParallelProjected

class Tests(unittest.TestCase):
    def toy(self,eps=10.):return Toy(np.array([-1.,-10.,-100.,-1000.]),eps,np.full(4,.1),.01)
    def test_raw_coefficients_strict_lower(self):
        self.assertTrue(np.array_equal(np.triu(A),np.zeros((8,8))))
        self.assertTrue(np.array_equal(np.triu(C),np.zeros((8,8))))
    def test_homotopy_endpoint_target_independent_of_theta(self):
        t=self.toy();u=np.arange(32).reshape(8,4)*.0001
        for theta in [0.,.3,1.]:np.testing.assert_allclose(t.residual(u,theta,1.),t.residual(u,1.,1.),atol=0,rtol=0)
    def test_lambda_zero_affine_seed(self):
        t=self.toy()
        for theta in [0.,.5,1.]:self.assertLess(np.linalg.norm(t.residual(t.linear_seed(theta),theta,0.)),1e-12)
    def test_root_reference_is_actual_raw_stage_target(self):
        t=self.toy();u=t.exact_target();self.assertLess(np.linalg.norm(t.residual(u)),1e-12)
    def test_jacobian_action_centered_difference(self):
        t=self.toy();rng=np.random.default_rng(18);u=rng.normal(size=(8,4))*.001;v=rng.normal(size=(8,4))
        eps=1e-7
        numerical=(t.residual(u+eps*v,.4,.7)-t.residual(u-eps*v,.4,.7))/(2*eps)
        analytic=t.d*v-t.lower_derivative(u,v,.4,.7)
        np.testing.assert_allclose(numerical,analytic,atol=2e-9,rtol=2e-9)
    def test_determinant_constant_for_fixed_invertible_W(self):self.assertLess(determinant_test()['max_logdet_error'],1e-10)
    def test_nilpotence_does_not_imply_monotone_contraction(self):
        for r in nilpotent_demo():
            self.assertEqual(r['norm_T_powers'][7],0.)
            self.assertGreater(max(r['error_per_round']),r['error_per_round'][0])
    def test_few_mode_arnoldi_constructs_target(self):
        r=arnoldi_affine(np.tile([-1.,-20.,-100.],32),np.ones(96),.02,8)[-1]
        self.assertEqual(r['m'],3);self.assertLess(r['output_WRMS_vs_stage_root'],1e-5)
    def test_rank_one_is_not_sufficient_for_small_krylov(self):
        r=arnoldi_affine(-np.logspace(0,4,64),np.ones(64),.02,32)[-1]
        self.assertEqual(r['raw_rhs_rank'],1);self.assertGreater(r['output_WRMS_vs_stage_root'],1.)
    def test_q1_reduces_iteration_but_not_total_inverse_work(self):
        a=newton_path(self.toy(),1.,[1.],0);b=newton_path(self.toy(),1.,[1.],1)
        self.assertLess(b['counts']['Krylov_iterations'],a['counts']['Krylov_iterations'])
        self.assertGreater(b['counts']['W_inverse_batches'],a['counts']['W_inverse_batches'])
    def test_complete_projected_parallel_candidate_without_affine_oracle(self):
        r=ParallelProjected(n=384).run([1.])
        self.assertEqual(r['subspace_dimension'],3);self.assertFalse(r['full_n_by_n_matrix_constructed'])
        self.assertLess(r['full_space_relative_residual'],1e-10);self.assertLess(r['output_WRMS_vs_sequential_stage_root'],.01)
    def test_nonnormal_projection_can_be_singular_even_for_invertible_full_operator(self):
        W=np.array([[0.,1.],[-1.,0.]])
        V=np.array([[1.],[0.]])
        self.assertEqual(np.linalg.det(W),1.)
        self.assertEqual(float((V.T@W@V)[0,0]),0.)
    def test_final_error_check_not_merely_small_projected_residual(self):
        r=arnoldi_affine(-np.logspace(0,4,64),np.ones(64),.02,32)[-1]
        self.assertLess(r['full_relative_stage_residual'],1e-8)
        self.assertGreater(r['output_WRMS_vs_stage_root'],1.)

if __name__=='__main__':unittest.main(verbosity=2)
