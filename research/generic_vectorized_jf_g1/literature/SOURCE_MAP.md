# G1/G2 primary source map

- van der Houwen & Sommeijer, *Iterated Runge–Kutta Methods on Parallel Computers*,
  SIAM J. Sci. Comput., DOI 10.1137/0912054.  Parallel stage depth can reduce to the iteration count,
  provided iteration parameters recover corrector accuracy/stability rapidly.
- Luan & Ostermann, *Parallel exponential Rosenbrock methods*,
  Comput. Math. Appl. 71 (2016), DOI 10.1016/j.camwa.2016.01.020.  Order-5 and order-6 parallel
  exponential Rosenbrock methods can be implemented at the cost of a three-stage method.
- Tokman, Loffeld & Tranquilli, *New Adaptive EPIRK Methods*,
  SIAM J. Sci. Comput., DOI 10.1137/110849961.  EPIRK structure is designed to exploit adaptive Krylov
  evaluation of phi-function actions.
- Hochbruck, Ostermann & Schweitzer, *Exponential Rosenbrock-Type Methods*,
  SIAM J. Numer. Anal., DOI 10.1137/080717717.  Exponential Rosenbrock methods are fully explicit and
  avoid conventional linear-system solves while treating stiff linearization through matrix functions.
- Tranquilli & Sandu, *Rosenbrock–Krylov Methods for Large Systems of Differential Equations*,
  SIAM J. Sci. Comput., DOI 10.1137/130923336.  Time discretization and Krylov projection form one
  order-condition theory; this is a distinct complete-integrator comparator.
