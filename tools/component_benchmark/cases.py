"""Frozen, JSON-serializable benchmark cases. No holdout fixtures are imported."""
from __future__ import annotations
import math

TOLS = ((1e-4, 1e-7), (1e-6, 1e-9), (1e-8, 1e-11))


def _grid(end, count=101, start=0.0):
    out = [start + (end-start)*i/(count-1) for i in range(count)]
    out[0], out[-1] = start, end
    return out


def _log_grid(end, count=101, first=1e-8):
    # Resolve the initial layer down to first before following the long tail.
    out=[0.0]+[math.exp(math.log(first)+(math.log(end)-math.log(first))*i/(count-2)) for i in range(count-1)]
    out[-1]=end
    return out


def _osc_grid(omega,end):
    # Nine samples per period plus fixed noncommensurate observation times.
    count=math.ceil(end*omega*9/(2*math.pi))+1
    times=_grid(end,count)
    times.extend(end*q for q in (0.137137,0.411411,0.773773))
    return sorted(set(times))


def _case(cid, family, model, p, y0, end, eq, *, rtol=1e-8, atol=1e-11,
          grid=None, mass=None, reference='analytic', bc=None):
    d = dict(case_id=cid, family=family, model=model, parameters=p,
             initial_state=[float(x) for x in y0], t_span=[0.0,float(end)],
             output_times=grid if grid is not None else _grid(end), rtol=rtol,
             atol=[float(atol)]*len(y0), equation_form=eq,
             mass_matrix=mass, reference_policy=reference, boundary_conditions=bc)
    return d


def build_cases():
    c=[]
    add=c.append
    add(_case('ode-scalar-decay','nonstiff','scalar_decay',{'rate':-1.0},[1],10,'y′ = rate*y'))
    add(_case('ode-logistic','nonstiff','logistic',{'rate':2.0,'capacity':1.0},[0.1],5,'y′=rate*y*(1-y/capacity)'))
    add(_case('ode-kepler','nonstiff','kepler',{'mu':1.0,'eccentricity':0.1},[0.9,0,0,math.sqrt(1.1/0.9)],2*math.pi,'r′=v; v′=-mu*r/|r|³',reference='analytic'))
    add(_case('ode-robertson','stiff_kinetics','robertson',{'k1':0.04,'k2':1e4,'k3':3e7},[1,0,0],40,'A′=-k1*A+k2*B*C; B′=k1*A-k2*B*C-k3*B²; C′=k3*B²',grid=_grid(40),reference='numerical_refinement'))
    add(_case('ode-vdp-mu1000','stiff_kinetics','vanderpol',{'mu':1000.0},[2,0],2000,'x′=v; v′=mu*(1-x²)*v-x',grid=_log_grid(2000),reference='numerical_refinement'))
    add(_case('ode-hires','stiff_kinetics','hires',{},[1,0,0,0,0,0,0,0.0057],321.8122,'HIRES eight-species kinetics, native stiff_benchmark.rs',grid=_log_grid(321.8122),reference='numerical_refinement'))
    for lam, name in [(-100.0,'ode-pr-linear-1e2'),(-10000.0,'ode-pr-linear-1e4')]:
        add(_case(name,'prothero_robinson','pr_linear',{'lambda':lam,'beta':0.0},[0],1,'y′=lambda*(y-sin(t))+cos(t)'))
    add(_case('ode-pr101','prothero_robinson','pr_nonlinear',{'lambda':-1e6,'beta':10.0},[0],1,'y′=-1e6*(y-sin(t))+cos(t)+10*(y-sin(t))²',grid=_grid(1,101)))
    for omega, tag in [(1,'1'),(100,'100'),(10000,'1e4')]:
        end=20*math.pi if omega<10000 else 1.0
        grid=_osc_grid(omega,end)
        add(_case('ode-rotation-'+tag,'oscillation','rotation',{'omega':float(omega)},[1,0],end,'x′=-omega*y; y′=omega*x',grid=grid))
    for zeta,tag in [(0.001,'underdamped'),(1,'critical'),(10,'overdamped')]:
        add(_case('ode-damping-'+tag,'damping','damping',{'omega':100.0,'zeta':zeta},[1,0],1,'x′=v; v′=-2*zeta*omega*v-omega²*x',grid=_grid(1,157)))
    add(_case('ode-diagonal-stiff','linear_structure','diagonal_stiff',{'rates':[-1.0,-100.0,-10000.0]},[1,1,1],1,'y_i′=rates_i*y_i',grid=_log_grid(1)))
    add(_case('ode-jordan','linear_structure','jordan',{'lambda':-10.0,'offdiag':100.0},[0,1],1,'x′=lambda*x+offdiag*y; y′=lambda*y'))
    add(_case('ode-nonnormal','linear_structure','nonnormal',{'a':-1.0,'b':100.0,'d':-2.0},[0,1],5,'x′=a*x+b*y; y′=d*y'))
    n=64
    add(_case('ode-periodic-heat','spatial','periodic_heat',{'cells':n,'diffusion':0.01},[1+0.1*math.sin(2*math.pi*i/n) for i in range(n)],1,'u′_i=D*N²*(u_(i-1)-2u_i+u_(i+1))',bc='periodic'))
    n=256
    add(_case('ode-advection-diffusion','spatial','advection_diffusion',{'cells':n,'diffusion':0.01,'velocity':1.0},[1+0.1*math.sin(2*math.pi*i/n) for i in range(n)],1,'u′_i=D*N²*(u_(i-1)-2u_i+u_(i+1))-v*N*(u_i-u_(i-1))',bc='periodic'))
    b0=[]
    for i in range(50): b0.extend([1+math.sin(2*math.pi*(i+1)/51),3])
    add(_case('ode-brusselator-1d-50','spatial','brusselator_1d',{'cells':50,'boundary_u':1.0,'boundary_v':3.0,'diffusion_factor':(51**2)/50},b0,10,'native stiff_benchmark.rs Brusselator 1D',bc='Dirichlet u=1,v=3',reference='numerical_refinement'))
    n=4; gs=[]
    for j in range(n):
        for i in range(n):
            dist=((i+0.5)/n-0.5)**2+((j+0.5)/n-0.5)**2
            gs.append(.25*math.exp(-100*dist))
    add(_case('ode-gray-scott-2d-4','specialized','gray_scott_2d',{'grid':n,'du':.002,'dv':.001,'feed':.04,'kill':.06},[1-v for v in gs]+gs,20,'native gray_scott.rs two-species periodic grid',bc='periodic',reference='numerical_refinement'))
    # SABR here is the original stage algorithm applied to its scalar-linear fixture.
    sabr=_case('ode-sabr-scalar-fixture','specialized','sabr',{'lambda':-100.0,'fixture':'homotopy_experiments::Smoke/scalar-l-100-h5.000e-3','h':0.005},[1],0.005,'y′=lambda*y; SABR algorithm fixture',grid=[0.0,0.005],rtol=1e-6,atol=1e-7)
    sabr['algorithm_fixture']='SABR nilpotent 3 iterations; source homotopy_experiments.rs'
    sabr['algorithm_config']={'predictor':'Zero','max_iterations':3,'block_method':'Nilpotent','defect_budget_fraction':0.1,'fallback':'Direct','force_accept':True}
    sabr['output_semantics']='single forced source step endpoints; OK is not tolerance-controlled acceptance'
    add(sabr)
    # PP15 client_arm(40) uses the two complex amplitudes (1/4, 1/2).
    fourier=_case('ode-fourier-path-client','specialized','fourier_path',{'omega':40.0,'g':.125,'eps':.125,'sigma':1,'fixture':'pp15_fourier_comparator::client_arm(40)','step_cap':.125},[.25,0,.5,0],.5,'A′=i*q*(g+eps*Re(exp(i*omega*t)*A))*conj(A)*B; B′=i*q/2*(g+eps*Re(exp(i*omega*t)*A))*A²; state=[Re A,Im A,Re B,Im B]',reference='numerical_refinement')
    fourier['output_semantics']='complex amplitudes sampled as four real components, not certificate path state'
    add(fourier)
    add(_case('dae-mass-invertible','mass_dae','mass_invertible',{'lambda':-1.0},[1,2],1,'M*y′=f(t,y), M=[[2,1],[0,3]], f=M*(-y)',mass=[[2,1],[0,3]]))
    add(_case('dae-mass-ill-conditioned','mass_dae','mass_ill_conditioned',{'epsilon':1e-8},[1,2],1,'M*y′=f(t,y), M=diag(1,epsilon), f=M*(-y)',mass=[[1,0],[0,1e-8]]))
    add(_case('dae-index1','mass_dae','dae_index1',{},[0,1],1,'y′=-y+z; 0=y+z-(2sin(t)+cos(t))',mass=[[1,0],[0,0]]))
    add(_case('dae-index1-inconsistent','mass_dae','dae_index1_inconsistent',{},[0,0],1,'same index-1 DAE with inconsistent y(0)+z(0)=0',mass=[[1,0],[0,0]],reference='expected_failure'))
    add(_case('dae-robertson-constraint','mass_dae','robertson_constraint',{'k1':.04,'k2':1e4,'k3':3e7},[1,0,0],40,'Robertson A′ and B′; algebraic A+B+C=1',mass=[[1,0,0],[0,1,0],[0,0,0]],reference='numerical_refinement'))
    add(_case('dae-pendulum-index3','mass_dae','pendulum_index3',{'g':1.0,'length':1.0},[1,0,0,0,0],1,'x′=vx,y′=vy,vx′=-lambda*x,vy′=-g-lambda*y,0=x²+y²-1',mass=[[1,0,0,0,0],[0,1,0,0,0],[0,0,1,0,0],[0,0,0,1,0],[0,0,0,0,0]],reference='capability_gap'))
    initial_derivatives={
        'mass_invertible':[-1.0,-2.0],
        'mass_ill_conditioned':[-1.0,-2.0],
        'dae_index1':[1.0,1.0],
        'dae_index1_inconsistent':[0.0,0.0],
        'robertson_constraint':[-0.04,0.04,0.0],
        'pendulum_index3':[0.0,0.0,0.0,-1.0,0.0],
    }
    for case in c:
        if case['mass_matrix'] is not None:
            case['initial_derivative']=initial_derivatives[case['model']]
    # Stress cases are explicit; they do not replace the 24+6 anchors.
    for model,n in [('periodic_heat',256),('periodic_heat',4096),('advection_diffusion',4096)]:
        p={'cells':n,'diffusion':0.01}
        if model=='advection_diffusion': p['velocity']=1.0
        add(_case(f'stress-{model}-{n}','stress',model,p,
                  [1+0.1*math.sin(2*math.pi*i/n) for i in range(n)],1,
                  'periodic diffusion' if model=='periodic_heat' else 'periodic upwind advection-diffusion',
                  grid=_grid(1,51),bc='periodic',reference='analytic'))
    n=200; y=[]
    for i in range(n): y.extend([1+math.sin(2*math.pi*(i+1)/(n+1)),3])
    add(_case('stress-brusselator-1d-200','stress','brusselator_1d',
              {'cells':n,'boundary_u':1.0,'boundary_v':3.0,'diffusion_factor':((n+1)**2)/50},
              y,10,'native stiff_benchmark.rs Brusselator 1D',grid=_grid(10,51),
              bc='Dirichlet u=1,v=3',reference='numerical_refinement'))
    assert len(c)==34 and len({v['case_id'] for v in c})==34
    return c
