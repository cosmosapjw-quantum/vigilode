"""Constructive arithmetic-complexity theorem for a fixed admissible window.
No candidate data or numerical oracle is used to choose sufficient resources.
Model family: T<=1/2, |Delta|<=1, |chi|<=1/8, A0<=9/32,
max_j |a_j0|<=11/32. R=3/2, analytic coefficient ball B=3/4.
"""
from fractions import Fraction as F
from math import factorial

def exp_upper(x,n=12):
    x=F(x)
    if x<0 or n+2<=x:raise ValueError('nonnegative argument, geometric tail required')
    total=sum((x**j/factorial(j) for j in range(n+1)),F(0))
    first=x**(n+1)/factorial(n+1)
    return total+first/(1-x/F(n+2))

def theoretical_bound(order,degree,sweeps,bits=112):
    T=F(1,2);chi=F(1,8);r=F(1,2);R=F(3,2);B=F(3,4)
    A0=F(9,32);amax=F(11,32);S=F(17,8);S1=F(5,3)
    qR=3*chi*T*R*S*B**2;q1=3*chi*T*S1*B**2
    mapping=A0*S+chi*T*R*S*B**3
    # A strict smaller analytic ball plus this rounding allowance stays in B.
    # Finite-degree rounding ||e||_R <= 2^-bits sum R^k (two real components).
    eps1=F(degree+1,1<<bits)
    epsR=sum((R**k for k in range(degree+1)),F(0))/(1<<bits)
    if mapping+epsR>B:raise ValueError('rounding can exhaust analytic ball margin')
    approximation=2*B*q1**sweeps+B*R**(-degree-1)/(1-q1)+eps1/(1-q1)
    residual=(1+q1)*approximation
    tau=r**(order+1)/factorial(order+1)
    model=tau*(A0+T*chi/F(order+2));L=3*chi
    mean=(residual+model)/(1-L*T)
    mode=tau*(amax+T*chi/F(order+2))+L*T*mean
    return {'q_R':qR,'q_1':q1,'mapping_bound':mapping,'rounding_R':epsR,
            'projected_iteration_error':approximation,'volterra_defect_bound':residual,
            'mean_bound':mean,'physical_max_bound':mode}

def sufficient_parameters(tolerance,bits=112):
    tol=F(tolerance)
    if tol<=0:raise ValueError('positive tolerance required')
    # Conservative deterministic joint refinement; not a cost-optimal selector.
    order=0;degree=0;sweeps=1
    for level in range(1,256):
        order=level;degree=2*level;sweeps=level
        b=theoretical_bound(order,degree,sweeps,bits)
        if b['physical_max_bound']<=tol:
            return {'order':order,'degree':degree,'sweeps':sweeps,'bits':bits,**b}
    raise ValueError('resource limit')
