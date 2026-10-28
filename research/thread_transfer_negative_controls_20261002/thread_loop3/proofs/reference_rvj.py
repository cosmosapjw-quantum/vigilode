"""Small high-precision analytic reference for research, NOT a production solver.

All jets are differentiated symbolically from the supplied polynomial/analytic RHS.
No Jacobian approximation, Krylov iteration, AD backend, or parallel performance
claim is made. This intentionally expensive reference separates mathematics from
linear-solver and floating-point errors.
"""
from __future__ import annotations
from dataclasses import dataclass
from typing import Callable, Sequence
import sympy as sp
import mpmath as mp

z = sp.Symbol('z')
P5 = 1+sp.Rational(2,5)*z+z**2/20
Q5 = 1-sp.Rational(3,5)*z+sp.Rational(3,20)*z**2-z**3/60
P4 = 1+z/4
Q4 = 1-sp.Rational(3,4)*z+z**2/4-z**3/24

def rational_phi(order: int) -> tuple[sp.Expr, list[sp.Expr]]:
    if order not in (4,5):
        raise ValueError('This research reference supports only orders 4 and 5.')
    P,Q = (P5,Q5) if order==5 else (P4,Q4)
    ns=[sp.cancel((P-Q*sum(z**k/sp.factorial(k) for k in range(j)))/z**j)
        for j in range(1,order+1)]
    return Q,ns

def mprat(x: sp.Expr) -> mp.mpf:
    x=sp.Rational(x)
    return mp.mpf(int(x.p))/int(x.q)

def matrix_poly(coeff: Sequence[sp.Expr], powers: Sequence[mp.matrix]) -> mp.matrix:
    out=mp.zeros(powers[0].rows)
    for c,power in zip(coeff,powers):
        if c: out += mprat(c)*power
    return out

@dataclass
class AnalyticRVJ:
    variables: tuple[sp.Symbol,...]
    parameters: tuple[sp.Symbol,...]
    rhs: sp.Matrix
    order: int = 5

    def __post_init__(self) -> None:
        if self.order not in (4,5): raise ValueError('order must be 4 or 5')
        self.rhs=sp.Matrix(self.rhs)
        if self.rhs.shape!=(len(self.variables),1): raise ValueError('RHS shape mismatch')
        self.jac=self.rhs.jacobian(self.variables)
        self.jets=[self.rhs]
        for _ in range(1,self.order):
            self.jets.append((self.jets[-1].jacobian(self.variables)*self.rhs).applyfunc(sp.expand))
        self.sources=[self.rhs]+[(self.jets[j]-self.jac*self.jets[j-1]).applyfunc(sp.expand)
                                      for j in range(1,self.order)]
        args=self.variables+self.parameters
        self.jfun=sp.lambdify(args,self.jac,'mpmath')
        self.sfun=[sp.lambdify(args,v,'mpmath') for v in self.sources]
        Q,ns=rational_phi(self.order)
        self.qcoeff=[sp.expand(Q).coeff(z,j) for j in range(4)]
        self.ncoeff=[[sp.expand(n).coeff(z,j) for j in range(3)] for n in ns]

    def step(self, state: Sequence[mp.mpf] | mp.matrix, h: mp.mpf,
             parameter_values: Sequence[mp.mpf] = ()) -> mp.matrix:
        h=mp.mpf(h); state=mp.matrix(state)
        if h<=0 or not mp.isfinite(h): raise ValueError('h must be positive and finite')
        if len(state)!=len(self.variables) or len(parameter_values)!=len(self.parameters):
            raise ValueError('state/parameter length mismatch')
        args=tuple(state)+tuple(parameter_values)
        J=mp.matrix(self.jfun(*args)); Z=h*J
        powers=[mp.eye(J.rows),Z,Z*Z,Z*Z*Z]
        QZ=matrix_poly(self.qcoeff,powers)
        b=mp.zeros(J.rows,1)
        for j,(coeff,sfun) in enumerate(zip(self.ncoeff,self.sfun),start=1):
            b += h**j * matrix_poly(coeff,powers) * mp.matrix(sfun(*args))
        out=state+mp.lu_solve(QZ,b)
        if not all(mp.isfinite(v) for v in out): raise ArithmeticError('nonfinite reference step')
        return out

def R5(zv: mp.mpf) -> mp.mpf:
    return (1+mp.mpf(2)/5*zv+zv*zv/20)/(1-mp.mpf(3)/5*zv+mp.mpf(3)/20*zv*zv-zv**3/60)

def phi5(zv: mp.mpf) -> mp.mpf:
    return (zv*zv-5*zv+12)/(1440*(1-mp.mpf(3)/5*zv+mp.mpf(3)/20*zv*zv-zv**3/60))

def slow_step(x: mp.mpf,h: mp.mpf) -> mp.mpf:
    return x+sum(h**j*x**(j+1) for j in range(1,5))+120*h**5*phi5(2*h*x)*x**6
