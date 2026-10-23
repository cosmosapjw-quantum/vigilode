import copy, math, unittest
from fractions import Fraction as F
from readiness import norm_squared, native_gate

class Geometry(unittest.TestCase):
    def test_correlated_helium_has_opposite_verdict_at_same_magnitudes(self):
        e=[0.0]*30; e[24]=e[25]=2.5
        self.assertEqual(norm_squared(e,[1.0]*33),F(25,22))
        e[25]=-2.5
        self.assertEqual(norm_squared(e,[1.0]*33),F(25,66))
    def test_full_dimension_not_reduced_dimension(self):
        e=[0.0]*30; e[0]=5.5
        self.assertEqual(norm_squared(e,[1.0]*33),F(121,132))
        self.assertGreater(F(121,120),1)
    def test_finite_normalized_norm_with_unrepresentable_lift(self):
        e=[0.0]*30; e[24]=e[25]=1e308
        self.assertEqual(norm_squared(e,[1e308]*33),F(2,11))
    def test_zero_error(self):
        self.assertEqual(norm_squared([0.0]*30,[1.0]*33),0)
    def test_invalid_domains(self):
        for e,s in [([0.]*29,[1.]*33),([0.]*30,[1.]*32),([0.]*30,[0.]+[1.]*32),([math.nan]+[0.]*29,[1.]*33),([0.]*30,[math.inf]+[1.]*32)]:
            with self.assertRaises(ValueError): norm_squared(e,s)

class Receipt(unittest.TestCase):
    """Synthetic schema tests, never an actual native-Rust result."""
    def fixture(self):
        return {'driver':'native-rodas5p','source':'a'*40,'reduced_dimension':30,'physical_dimension':33,'denominator':33,
                'scale_owner':'client-full-physical','error_lift':'L*error30-no-offset','default_norm_digest_equal':True,
                'correlated_oracle_pass':True,'mixed_exponent_contract_pass':True,'typed_refusal_no_commit':True,
                'hot_path_allocations':0,'owner_clearance_current':True,'local_changes_preserved':True,
                'source_bound_native_execution':True}
    def test_complete_synthetic_schema(self): self.assertEqual(native_gate(self.fixture(),'a'*40),[])
    def test_wrong_affine_meaning_does_not_close_v1(self):
        r=self.fixture(); r['driver']='affine-harmonic-flow'
        self.assertIn('driver',native_gate(r,'a'*40))
    def test_source_lease_mismatch(self): self.assertIn('source',native_gate(self.fixture(),'b'*40))
    def test_unknown_owner_blocks_even_passed_numerics(self):
        r=self.fixture(); r['owner_clearance_current']=None
        self.assertIn('owner_clearance_current',native_gate(r,'a'*40))
    def test_missing_contracts_fail_closed(self):
        r=self.fixture(); del r['typed_refusal_no_commit']; r['denominator']=30
        self.assertEqual(set(native_gate(r,'a'*40)),{'typed_refusal_no_commit','denominator'})
    def test_string_pass_and_bool_allocation_are_not_evidence(self):
        r=self.fixture(); r['correlated_oracle_pass']='PASS';r['hot_path_allocations']=False
        self.assertEqual(set(native_gate(r,'a'*40)),{'correlated_oracle_pass','hot_path_allocations'})
    def test_non_mapping_fails_closed(self): self.assertEqual(native_gate(None,'a'*40),['receipt'])

if __name__=='__main__': unittest.main(verbosity=2)
