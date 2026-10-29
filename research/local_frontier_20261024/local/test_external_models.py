import copy
import unittest
from external_models import rhs,exact,verify

class ExternalModelTests(unittest.TestCase):
    def contract(self):
        return {'case_id':15,'parameters':{'omega0':1.,'beta':1.2,'gamma':.8},'initial_state':[1.,-.8],'t_start':0.,'t_end':.1,'rhs_probes':[{'t':0.,'state':[1.,0.],'rhs':[0.,-.68]},{'t':0.,'state':[0.,1.],'rhs':[1.,-.4]}]}
    def test_independent_linear_basis_probes_match_chirp(self):
        self.assertEqual(verify(self.contract())['status'],'MATCHED_AT_DECLARED_PROBES')
    def test_changed_rhs_probe_is_rejected(self):
        c=self.contract();c['rhs_probes'][0]['rhs'][1]+=.01
        with self.assertRaises(ValueError):verify(c)
    def test_initial_condition_change_is_rejected(self):
        c=self.contract();c['initial_state'][0]=2
        with self.assertRaises(ValueError):verify(c)
    def test_chirp_exact_derivative_matches_vector_field(self):
        c=self.contract();t=.03;h=1e-6
        a,b=exact(c,t+h),exact(c,t-h)
        f=rhs(c,t,exact(c,t))
        for x,y,z in zip(a,b,f):self.assertAlmostEqual((x-y)/(2*h),z,places=8)
    def test_unsupported_model_cannot_be_substituted(self):
        c=self.contract();c['case_id']=24
        with self.assertRaises(ValueError):verify(c)

if __name__=='__main__':unittest.main()
