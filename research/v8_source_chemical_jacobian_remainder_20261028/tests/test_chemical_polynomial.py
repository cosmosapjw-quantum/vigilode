"""One new bounded suite; independent nine-state flux oracle, no old tests."""
import copy
from fractions import Fraction as F
import itertools
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'code'))
import chemical_polynomial as C

PROVENANCE = {'kind':'DIAGNOSTIC_SYNTHETIC_WEIGHTS','evidence':'Prescribed exact synthetic coefficients for independent controls only'}
WEIGHTS = [F(1),F(2),F(3)]
RATES = [F(i+1,13) for i in range(12)]


def binding():
    return {'bass_commit':C.BASS,'binding_id':C.BINDING,
            'clock':dict(C.prior().prior().source().CLOCK),
            'evidence':'Pinned cc602 transition topology; diagnostic supplied effective rates and frozen weights'}


def model(rates=RATES,weights=WEIGHTS):
    return C.polynomial(rates,weights,PROVENANCE,binding())


def oracle9(z,rates,weights):
    """Independently differentiate conservative nine-state edge fluxes.

    No production P, Q, J, e, endpoint-sign routine or state conversion is used.
    """
    th = F(2**53-1,2**53)
    populations = [th-z[0],z[0],1-z[1]-z[2],z[1],z[2],
                   1-z[3]-z[4]-z[5],z[3],z[4],z[5]]
    electron9 = [F(0),weights[0],F(0),weights[1],2*weights[1],
                 F(0),weights[2],2*weights[2],3*weights[2]]
    electron = sum((x*y for x,y in zip(populations,electron9)),F(0))
    rhs = [F(0)]*9
    derivative = [[F(0)]*9 for _ in range(9)]
    upward = ((0,1),(2,3),(3,4),(5,6),(6,7),(7,8))
    downward = ((1,0),(3,2),(4,3),(6,5),(7,6),(8,7))
    for k,(a,b) in enumerate(upward+downward):
        if k<6:
            flux = rates[k]*populations[a]
            gradient = [rates[k]*int(j==a) for j in range(9)]
        else:
            flux = rates[k]*electron*populations[a]
            gradient = [rates[k]*(electron9[j]*populations[a]+electron*int(j==a)) for j in range(9)]
        rhs[a] -= flux
        rhs[b] += flux
        for j,v in enumerate(gradient):
            derivative[a][j] -= v
            derivative[b][j] += v
    charged, neutral = (1,3,4,6,7,8),(0,2,2,5,5,5)
    restricted_F = [rhs[i] for i in charged]
    restricted_J = [[derivative[i][j]-derivative[i][n] for j,n in zip(charged,neutral)] for i in charged]
    return restricted_F,restricted_J,rhs,derivative,populations


def oracle_chemical_quadratic(v,u):
    # Nine-term exact low-dimensional test oracle, not a general native norm/lift API.
    h,a,b,c,d,e = v
    components = [-h,h,-a-b,a,b,-c-d-e,c,d,e]
    return sum(((x/u[j])**2 for j,x in zip(range(23,32),components)),F(0))/33


def zero_rates():
    return [F(0)]*12


class Controls(unittest.TestCase):
    saved = {}

    def test_01_source_independent_flux_F_J(self):
        m = model()
        states = [[F(1,5),F(1,7),F(2,7),F(1,11),F(2,11),F(3,11)],
                  [F(-1,9),F(1,4),F(-1,8),F(2,5),F(-1,10),F(1,20)],
                  [F(0)]*6] + C.state_vertices()
        for z in states:
            f,j,*_ = oracle9(z,RATES,WEIGHTS)
            self.assertEqual(C.evaluate(m,z),f)
            self.assertEqual(C.jacobian(m,z),j)
        self.assertEqual(m['coverage']['rows_reduced30'],[23,24,25,26,27,28])
        self.assertEqual(m['coverage']['full_Jacobian_missing_blocks'],'OPEN_NOT_ZERO')
        self.assertFalse(m['full30_model_provider'])
        self.saved['source_F_J_comparison'] = {'states':len(states),'independent_representation':'Nine-state conservative edge fluxes and full derivative, followed by neutral elimination','coverage':m['coverage']}

    def test_02_hydrogen_feedback_negative_control(self):
        rates = zero_rates()
        rates[0],rates[6] = F(2),F(3)
        m = model(rates,weights=[F(5),F(1),F(1)])
        z = [F(1,7)]+[F(0)]*5
        expected_F = 2*(C.TOTALS[0]-z[0])-15*z[0]**2
        expected_J = -2-30*z[0]
        self.assertEqual(C.evaluate(m,z)[0],expected_F)
        self.assertEqual(C.jacobian(m,z)[0][0],expected_J)
        omitted = -2-15*z[0]
        self.assertNotEqual(omitted,expected_J)
        self.assertEqual(expected_J-omitted,F(-15,7))
        self.saved['feedback_negative_control'] = {'correct':expected_J,'omitted_feedback':omitted,'exact_discrepancy':expected_J-omitted,'caught':True}
        curve=[]
        rates[0],rates[6] = F(1),F(1)
        m = model(rates,weights=[F(1)]*3)
        for fraction in (F(0),F(1,8),F(1,4),F(1,2),F(3,4),F(1)):
            x=C.TOTALS[0]*fraction
            curve.append({'HII':x,'correct_J_HH':-1-2*x,'omitted_feedback_J_HH':-1-x,
                          'mismatch':-x,'production_J_HH':C.jacobian(m,[x]+[F(0)]*5)[0][0]})
        self.saved['hydrogen_feedback_curve']=curve

    def test_03_conservation_boundaries_and_nonunit_total(self):
        m=model()
        for z in C.state_vertices():
            _,_,rhs,_,pops=oracle9(z,RATES,WEIGHTS)
            for group in ((0,1),(2,3,4),(5,6,7,8)):
                self.assertEqual(sum((rhs[i] for i in group),F(0)),0)
            for i,p in enumerate(pops):
                if not p:self.assertGreaterEqual(rhs[i],0)
            self.assertTrue(C.state_membership(z)['in_product_simplex'])
        self.assertEqual(C.TOTALS[0],1-F(1,2**53))
        self.assertNotEqual(C.TOTALS[0],1)
        f=C.evaluate(m,[F(0)]*6)
        self.assertEqual(f[0],RATES[0]*C.TOTALS[0])
        self.assertEqual(RATES[0]-f[0],RATES[0]/2**53)
        self.saved['totals']={'H':C.TOTALS[0],'H_hex':'0x1.fffffffffffffp-1','He':F(1),'Li':F(1),'source_zero_state_H_photo_offset':f[0]}

    def test_04_all_36_extrema_72_independent_witnesses(self):
        box=[[F(i+1,17),F(2*i+5,13)] for i in range(12)]
        cert=C.jacobian_extrema(box,WEIGHTS,PROVENANCE,binding())
        count=0
        for row in cert['entries']:
            for entry in row:
                self.assertLessEqual(entry['lower'],entry['upper'])
                for side,name in (('min','lower'),('max','upper')):
                    witness=entry[side+'_witness']
                    self.assertTrue(C.state_membership(witness['state'])['in_product_simplex'])
                    for rate,(lo,hi) in zip(witness['rates'],box):
                        self.assertIn(rate,(lo,hi))
                    independent=oracle9(witness['state'],witness['rates'],WEIGHTS)[1][entry['row']][entry['column']]
                    self.assertEqual(independent,entry[name])
                    self.assertEqual(independent,witness['value'])
                    count+=1
        self.assertEqual(count,72)
        self.assertEqual(cert['state_vertex_count'],24)
        self.saved['exact_entry_extrema']=cert
        self.saved['independent_witnesses_attained']=count

    def test_05_small_exhaustive_corners_and_zero_width(self):
        box=[[r,r] for r in RATES]
        for k in (0,7,11):box[k]=[F(0),F(k+2,7)]
        cert=C.jacobian_extrema(box,WEIGHTS,PROVENANCE,binding())
        lows=[[None]*6 for _ in range(6)];highs=[[None]*6 for _ in range(6)]
        count=0
        # Only three independent rates vary:8corners×24vertices; no endpoint-sign optimization.
        for choices in itertools.product((0,1),repeat=3):
            rates=RATES[:]
            for k,side in zip((0,7,11),choices):rates[k]=box[k][side]
            for z in C.state_vertices():
                j=oracle9(z,rates,WEIGHTS)[1]
                for i in range(6):
                    for k in range(6):
                        lows[i][k]=j[i][k] if lows[i][k] is None else min(lows[i][k],j[i][k])
                        highs[i][k]=j[i][k] if highs[i][k] is None else max(highs[i][k],j[i][k])
                count+=1
        self.assertEqual(lows,[[e['lower'] for e in row] for row in cert['entries']])
        self.assertEqual(highs,[[e['upper'] for e in row] for row in cert['entries']])
        fixed=C.jacobian_extrema([[r,r] for r in RATES],WEIGHTS,PROVENANCE,binding())
        for i in range(6):
            for j in range(6):
                vals=[oracle9(z,RATES,WEIGHTS)[1][i][j] for z in C.state_vertices()]
                self.assertEqual(fixed['entries'][i][j]['lower'],min(vals))
                self.assertEqual(fixed['entries'][i][j]['upper'],max(vals))
        self.saved['small_corner_oracle']={'rate_variables':[0,7,11],'corners':8,'state_vertices':24,'independent_evaluations':count,'all36_entry_intervals_equal':True,'zero_width_specialization_equal':True}

    def test_06_interior_points_and_family_scope(self):
        box=[[F(0),2*r] for r in RATES]
        cert=C.jacobian_extrema(box,WEIGHTS,PROVENANCE,binding())
        for n in (2,3,5,7):
            z=[C.TOTALS[0]/n,F(1,2*n),F(1,3*n),F(1,4*n),F(1,5*n),F(1,6*n)]
            rates=[lo+(hi-lo)*F((i%n)+1,n+1) for i,(lo,hi) in enumerate(box)]
            independent=oracle9(z,rates,WEIGHTS)[1]
            self.assertTrue(C.state_membership(z)['in_product_simplex'])
            for i in range(6):
                for j in range(6):
                    self.assertLessEqual(cert['entries'][i][j]['lower'],independent[i][j])
                    self.assertLessEqual(independent[i][j],cert['entries'][i][j]['upper'])
        self.assertFalse(cert['actual_model_enclosure_verified'])
        self.assertEqual(cert['coverage']['scope'],'CHEMICAL_PARTIAL_6_BY_6_ONLY')
        self.saved['interior_controls']={'points':4,'conclusion':'Exact point controls support implementation; reviewed affine vertex proof supplies continuum result','shared_parameter_correlation_preserved':False}

    def test_07_remainder_hessian_polarization_and_scale(self):
        m=model()
        z=[F(i+1,19) for i in range(6)]
        d=[F((-1)**i*(i+1),31) for i in range(6)]
        h=[F((-1)**(i+1)*(i+2),37) for i in range(6)]
        f0=oracle9(z,RATES,WEIGHTS)[0]
        f1=oracle9([x+y for x,y in zip(z,d)],RATES,WEIGHTS)[0]
        jd=C.matvec(oracle9(z,RATES,WEIGHTS)[1],d)
        independent=[y-x-a for x,y,a in zip(f0,f1,jd)]
        rem=C.quadratic_remainder(m,d)
        self.assertEqual(rem,independent)
        self.assertEqual(C.quadratic_remainder(m,[2*x for x in d]),[4*x for x in rem])
        pol=[x-y-z for x,y,z in zip(C.quadratic_remainder(m,[x+y for x,y in zip(d,h)]),rem,C.quadratic_remainder(m,h))]
        self.assertEqual(C.hessian_action(m,d,h),pol)
        self.assertEqual(C.hessian_action(m,d,d),[2*x for x in rem])
        self.assertEqual(C.hessian_action(m,d,h),C.hessian_action(m,h,d))
        hydrogen=zero_rates();hydrogen[6]=F(1)
        hm=model(hydrogen,weights=[F(1)]*3)
        delta=C.TOTALS[0]/16
        curve=[]
        for scale in (1,2,3,4,6,8):
            signed=C.quadratic_remainder(hm,[scale*delta]+[F(0)]*5)[0]
            self.assertEqual(signed,-(scale*delta)**2)
            curve.append({'increment_scale':scale,'delta_H':scale*delta,'R_H':signed,'abs_R_H':abs(signed),'reference_abs_R_H':delta**2*scale**2})
        self.saved['remainder_identity']={'d':d,'h':h,'R':rem,'D2':pol,'signed_polynomial_trials':True,'physical_membership_claimed_for_signed_trials':False}
        self.saved['quadratic_scaling_curve']=curve

    def test_08_fixed_metric_coefficient_bound_and_rescaling(self):
        rates=zero_rates();rates[6]=F(3,2)
        m=model(rates,weights=[F(1)]*3)
        u=[F(1)]*33
        cert=C.remainder_bound(m,u)
        self.assertEqual(cert['dual_e_sigma_squared'],F(495,2))
        self.assertEqual(cert['operator_squared_Qbar']['upper'],F(9,4))
        self.assertEqual(cert['Gamma_squared_sufficient'],F(4455,8))
        self.assertGreaterEqual(cert['Gamma']**2,cert['Gamma_squared_sufficient'])
        other=C.remainder_bound(m,[F(2)]*33)
        self.assertEqual(other['Gamma'],2*cert['Gamma'])
        self.assertEqual(other['dual_e_sigma_squared'],4*cert['dual_e_sigma_squared'])
        self.assertEqual(other['operator_squared_Qbar']['upper'],F(9,4))
        d=[F(1,7)]+[F(0)]*5
        self.assertEqual(oracle_chemical_quadratic(d,u),F(2,33)*d[0]**2)
        sharp_squared=F(33,2)*F(9,4)
        self.assertLess(sharp_squared,cert['Gamma_squared_sufficient'])
        general=model()
        gc=C.remainder_bound(general,u)
        d=[F((-1)**i*(i+1),23) for i in range(6)]
        h=[F(i+1,29) for i in range(6)]
        nd,nh=oracle_chemical_quadratic(d,u),oracle_chemical_quadratic(h,u)
        nr=oracle_chemical_quadratic(C.quadratic_remainder(general,d),u)
        nH=oracle_chemical_quadratic(C.hessian_action(general,d,h),u)
        self.assertLessEqual(nr,gc['Gamma']**2*nd**2)
        self.assertLessEqual(nH,4*gc['Gamma']**2*nd*nh)
        self.assertEqual(oracle_chemical_quadratic(d,[F(2)]*33),nd/4)
        self.assertEqual(gc['Qbar_role'],'NAMED_POLYNOMIAL_COEFFICIENT_OPERATOR_NOT_FULL_JACOBIAN')
        zeroQ=C.remainder_bound(model([F(1)]*6+[F(0)]*6),u)
        self.assertEqual(zeroQ['Gamma'],0)
        self.assertEqual(zeroQ['Hessian_bilinear_factor'],0)
        self.saved['metric_bound']={'H_only_Q_full_positive_e':cert,'uniform_u_times2':other,'general_fixed_Q':gc,'one_dimensional_H_sharp_Gamma_squared':sharp_squared,'one_dimensional_sharp_is_not_full_positive_e_Gamma':True,'nine_term_oracle_nd_squared':nd,'nine_term_oracle_remainder_squared':nr,'nine_term_oracle_Hessian_squared':nH}

    def test_09_source_RN_inventory_and_rational_serialization(self):
        record=C.reconstruct_inventory_RN()
        self.assertEqual(record['status'],'SOURCE_RECONSTRUCTED_UNDER_RN')
        self.assertFalse(record['original_owner_scale_array_observed'])
        self.assertFalse(record['original_source_manifest_body_recovered'])
        self.assertEqual(record['source_initialization_calls'],0)
        inv=record['represented_inventory_exact']
        self.assertEqual(record['weights_exact_real_sums'][0],inv['H1']+inv['D'])
        self.assertEqual(record['weights_exact_real_sums'][1],inv['He3']+inv['He4'])
        self.assertEqual(record['weights_exact_real_sums'][2],inv['Li6']+inv['Li7'])
        for name,hexvalue in record['inventory_hex'].items():
            self.assertEqual(F.from_float(float.fromhex(hexvalue)),inv[name])
        rnmodel=C.polynomial(RATES,record['weights_exact_real_sums'],record['provenance'],binding())
        z=[F(1,8)]*6
        self.assertEqual(C.evaluate(rnmodel,z),oracle9(z,RATES,record['weights_exact_real_sums'])[0])
        encoded=json.loads(json.dumps(C.jsonable([F(7,13),F(-11,17),F(1,2**53)])))
        self.assertEqual([C.exact(v) for v in encoded],[F(7,13),F(-11,17),F(1,2**53)])
        self.saved['source_RN_inventory_witness']=record

    def test_10_input_scope_and_serialization_refusals(self):
        m=model()
        for rates in ([F(1)]*11,[-1]+[0]*11,[True]+[0]*11,[float('nan')]+[0]*11,[float('inf')]+[0]*11):
            with self.assertRaises(ValueError):model(rates)
        for weights in ([1,2],[0,1,1],[-1,1,1],[True,1,1],[float('nan'),1,1]):
            with self.assertRaises(ValueError):model(weights=weights)
        for totals in ([1,1,1],[C.TOTALS[0],1],[-1,1,1]):
            with self.assertRaises(ValueError):C.polynomial(RATES,WEIGHTS,PROVENANCE,binding(),totals=totals)
        for key,value in (('bass_commit','WRONG'),('binding_id','WRONG'),('clock',{}),('evidence','')):
            b=binding();b[key]=value
            with self.assertRaises(ValueError):C.polynomial(RATES,WEIGHTS,PROVENANCE,b)
        for provenance in ({},{'kind':'OBSERVED_OWNER_SCALE_ARRAY','evidence':'unsupported'},{'kind':'DECLARED_FROZEN_INPUT','evidence':''}):
            with self.assertRaises(ValueError):C.polynomial(RATES,WEIGHTS,provenance,binding())
        for state in ([0]*5,[True]+[0]*5,[float('nan')]+[0]*5):
            with self.assertRaises(ValueError):C.jacobian(m,state)
        for box in ([[0,1]]*11,[[1,0]]*12,[[-1,1]]*12,[[False,1]]*12):
            with self.assertRaises(ValueError):C.jacobian_extrema(box,WEIGHTS,PROVENANCE,binding())
        with self.assertRaisesRegex(ValueError,'PARTIAL_BLOCK_CANNOT_PROMOTE'):
            C.jacobian_extrema([[0,1]]*12,WEIGHTS,PROVENANCE,binding(),requested_scope='FULL_MODEL_PROVIDER')
        for value in ({'numerator':'1','denominator':'0'}, {'numerator':True,'denominator':'1'}, '1/2', .5):
            with self.assertRaises(ValueError):C.exact(value)
        for kw in ({'refinement':True},{'refinement':13},{'precision':True},{'precision':129}):
            with self.assertRaises(ValueError):C.remainder_bound(m,[1]*33,**kw)
        with self.assertRaises(ValueError):C.remainder_bound(m,[1]*32)
        self.assertFalse(C.state_membership([C.TOTALS[0]+1]+[0]*5)['in_product_simplex'])
        self.saved['refusals']={'result':'All prescribed shape/rate/total/weight/binding/scope/serialization/resource refusals exercised','ceilings':C.ceilings()}


if __name__=='__main__':
    result=unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(Controls))
    data={'schema':'vigilode.source-chemical.new-controls.v1','unit_id':C.UNIT,'tests_run':result.testsRun,'errors':len(result.errors),'failures':len(result.failures),'diagnostic_inputs':True,'diagnostics':Controls.saved,**C.ceilings()}
    p=ROOT/'results/first_suite_data.json';p.parent.mkdir(exist_ok=True)
    with p.open('x') as f:json.dump(C.jsonable(data),f,indent=2,sort_keys=True,allow_nan=False);f.write('\n')
    sys.exit(0 if result.wasSuccessful() else 1)
