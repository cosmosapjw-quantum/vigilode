import math
from tools.component_benchmark.cases import build_cases
from tools.component_benchmark.models import rhs,jac,jvp,dae_residual
from tools.component_benchmark.protocol import validate_case


def test_catalog_and_fixed_pr101():
    cases=build_cases()
    assert len(cases)==34
    assert sum(c['mass_matrix'] is None and c['family']!='stress' for c in cases)==24
    assert sum(c['mass_matrix'] is not None for c in cases)==6
    assert sum(c['family']=='stress' for c in cases)==4
    assert all(not validate_case(c) for c in cases)
    pr=next(c for c in cases if c['case_id']=='ode-pr101')
    assert pr['parameters']=={'lambda':-1e6,'beta':10.0}
    assert pr['initial_state']==[0.0] and pr['t_span']==[0.0,1.0]
    assert pr['rtol']==1e-8 and pr['atol']==[1e-11] and len(pr['output_times'])==101
    assert pr['output_times']==[i/100 for i in range(101)]


def test_jvp_jacobian_and_dae_constraints():
    cases=build_cases()
    for c in cases:
        if c['family']=='stress': continue
        y=c['initial_state']; v=[1.0]*len(y)
        assert len(rhs(c,0,y))==len(y)
        got=jvp(c,0,y,v)
        expect=[sum(row) for row in jac(c,0,y)]
        assert all(math.isclose(a,b,rel_tol=1e-12,abs_tol=1e-10) for a,b in zip(got,expect))
    d=next(c for c in cases if c['model']=='dae_index1')
    assert dae_residual(d,0,[0,1],[1,0])==[0,0]
    bad=next(c for c in cases if c['model']=='dae_index1_inconsistent')
    assert dae_residual(bad,0,[0,0],[0,0])[1]==1


def test_directional_derivative_against_independent_difference():
    cases=build_cases()
    for cid in ('ode-logistic','ode-kepler','ode-robertson','ode-vdp-mu1000',
                'ode-hires','ode-pr101','ode-rotation-100','ode-damping-underdamped',
                'ode-brusselator-1d-50','ode-gray-scott-2d-4'):
        c=next(v for v in cases if v['case_id']==cid)
        y=[v+0.01 for v in c['initial_state']]; v=[0.5+0.01*i for i in range(len(y))]
        h=1e-6/max(v)
        yp=[a+h*b for a,b in zip(y,v)]; ym=[a-h*b for a,b in zip(y,v)]
        fd=[(a-b)/(2*h) for a,b in zip(rhs(c,0.11,yp),rhs(c,0.11,ym))]
        observed=jvp(c,0.11,y,v)
        assert all(math.isclose(a,b,rel_tol=2e-5,abs_tol=2e-5) for a,b in zip(fd,observed)),cid


def test_spatial_boundary_and_source_bound_specialized_identity():
    cases=build_cases()
    heat=next(c for c in cases if c['case_id']=='ode-periodic-heat')
    assert heat['parameters']['cells']==64 and heat['boundary_conditions']=='periodic'
    adv=next(c for c in cases if c['case_id']=='ode-advection-diffusion')
    assert adv['parameters']['cells']==256 and adv['parameters']['velocity']==1.0
    br=next(c for c in cases if c['case_id']=='ode-brusselator-1d-50')
    assert br['parameters']['cells']==50 and br['parameters']['diffusion_factor']==51**2/50
    sabr=next(c for c in cases if c['model']=='sabr')
    assert sabr['parameters']['lambda']==-100 and sabr['t_span']==[0.0,.005]
    assert 'homotopy_experiments::Smoke' in sabr['parameters']['fixture']
    four=next(c for c in cases if c['model']=='fourier_path')
    assert four['initial_state']==[.25,0,.5,0] and four['parameters']['omega']==40
    assert four['t_span']==[0.0,.5]
    for c in (heat,adv):
        n=c['parameters']['cells']; y=[0.0]*n; y[0]=1.0
        out=rhs(c,0,y)
        assert out[-1]>0, 'left periodic neighbor must wrap to last cell'
    for c in cases:
        if c['model']=='rotation':
            omega=c['parameters']['omega'];T=c['t_span'][1];grid=c['output_times']
            assert len(grid)>=math.ceil(T*omega*9/(2*math.pi))
            assert any(abs(t-.137137*T)<1e-12 for t in grid)


def test_initial_layer_grid_resolves_fast_scale():
    c=next(c for c in build_cases() if c['model']=='diagonal_stiff')
    assert c['output_times'][1]<=1e-6


def test_frozen_dae_initial_derivatives_are_residual_consistent():
    for c in build_cases():
        if c['mass_matrix'] is None: continue
        residual=dae_residual(c,c['t_span'][0],c['initial_state'],c['initial_derivative'])
        if c['model']=='dae_index1_inconsistent':
            assert abs(residual[-1])==1
        else:
            assert max(abs(v) for v in residual)<=max(c['atol'])
