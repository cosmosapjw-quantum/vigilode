# F1: overlapping primitive charts with certified branch identity

All variables and time are dimensionless. This package imports the read-only
`../baseline/code/algebraic_client.py` exact residual and Bernstein helpers.
The fixed main system and its `1e-8` whole-path absolute error contract are
unchanged. `atlas.py` actually evolves only three states and alternates the
primitive coordinate; it performs no online nonlinear solve.

## 1. Exact atlas and its coverage

Write `G1=u^2+v-a`, `G2=v^2+u-b`, `d=4*u*v-1`.
On the regular carrier `G=0,d!=0`, the two parameterized primitive charts are

- `Cu`: `(a,b,z=u)`, `v=a-u^2`, `Pu=u^4-2*a*u^2+u+a^2-b=0`;
- `Cv`: `(a,b,z=v)`, `u=b-v^2`, `Pv=v^4-2*b*v^2+v+b^2-a=0`.

After reconstruction, `dPu/du=-d` and `dPv/dv=-d`. Each simple root therefore
admits an implicit local graph over `(a,b)`. The polynomial transitions are
`Cu -> Cv: z_new=a-z_old^2` and `Cv -> Cu: z_new=b-z_old^2`.
Their compositions equal the identity modulo `(G1,G2)`. Both charts cover the
same regular carrier; neither enlarges the determinant domain. In particular,
selecting a different primitive does **not** cure a selected root with `d=0`.

For `a'=fa,b'=fb`, define `Nu=2*v*fa-fb`, `Nv=-fa+2*u*fb`.
The exact polynomial identities
`2*u*Nu+Nv=d*fa`, `Nu+2*v*Nv=d*fb` give `u'=Nu/d,v'=Nv/d`.
The runtime uses these scalar rational circuits in either chart. There are no
Jacobian, JVP, VJP, finite difference, or root-solver callbacks. Derivative
information is explicitly compiled into these coefficients.

## 2. Main branch identity and floating-point transition

The main parameter box is `X=[-1/8,1/8]^2`, root box `Q=[-1/4,1/4]^2`.
`T(u,v)=(b-v^2,a-u^2)` maps `Q` into itself and is `1/2` Lipschitz.
Thus each `x in X` has one root `q(x) in Q`, with
`||q(x)-q(x_tilde)||_inf<=2||x-x_tilde||_inf`,
`||qhat-q(x)||_inf<=2||G(x,qhat)||_inf`, and `d<=-3/4` on `Q`.
This uniqueness, together with checked box membership, is the branch token;
proximity or a root index from an eliminant is not used as branch identity.
A different exact root `(-1,-1)` at `x=(0,0)` has zero original residual and is
still rejected because it lies outside `Q`.

For an actual binary64 `Cu -> Cv` switch, copy the old full `v` into the new
primitive without rounding and compute `u_plus=fl(b-v^2)`.
Let `epsilon=u_plus-(b-v^2)` evaluated as an exact rational from the stored
floats. Then, even for a nonzero-residual input,

`u_plus-u_minus=-G2(x,q_minus)+epsilon`, `v_plus=v_minus`.

The reverse switch has
`v_plus-v_minus=-G1(x,q_minus)+epsilon`, `u_plus=u_minus`.
Each identity is checked exactly. Both one-sided candidates must lie in the
same `X,Q`; both original residual radii must pass the budget. With
`r_minus=2||G_minus||`, `r_plus=2||G_plus||`, uniqueness gives
`||q_plus-q_minus||<=r_minus+r_plus`. This includes reconstruction rounding.
The code also verifies this inequality, each branch token, and the complete
alignment of transitions with the adjacent slabs. Tampered records fail.

The slow coordinate `(a,b)` is bitwise unchanged at a switch. Consequently its
propagated error radius has zero jump. The algebraic candidate can jump, so
both one-sided values are retained and certified; no continuity of the
algebraic dense path is assumed. If slow state were modified by `Delta x`, the
radius would require an additional `||Delta x||`; the present implementation
rejects such unaccounted modifications rather than silently omitting them.

## 3. Whole-slab error with switches

The main coupled field remains
`fa=-a+(u+v)/8+1/16`, `fb=-b+(u-v)/8+1/32` with exact initial state zero.
The baseline proof supplies forward invariance of `X` and reduced-field
one-sided contraction rate `1/2`. On each slab the stored endpoint floats are
converted to exact rationals, and four physical cubic Hermite paths are made
using exact compiled endpoint slopes. The Hermite interior is the certified
physical output contract, not an assumption that reconstructed elimination
holds identically inside the slab. Both original polynomials are bounded.

Exact Bernstein hulls certify all of the dense `X,Q` domain and provide
`Dg>=sup||G||`, `Df>=sup||xhat'-f(xhat,qhat)||`.
Then `delta=Df+Dg/2`, and for incoming slow radius `E0`,

`Ewhole=E0+h*delta`,
`Eend=ceil_dyadic96(E0/(1+h/2)+h*delta)`,
`Eqwhole=2*Ewhole+2*Dg`.

The same `Eend` passes through the representation switch since `xhat` has no
jump. The next slab checks its new algebraic endpoint afresh. Hence a chain
of switches adds no hidden algebraic jump sum to the slow error: original
residuals on every slab already bound their effect on the slow RHS. The
one-sided root errors at every switch are included by the closed slabs.
All certificate arithmetic is rational, including rounding inflation. No
sampled supremum or observed error is substituted for these bounds.

## 4. Separate analytic control crossing a global discriminant

This control is explicitly outside the main small box and has a prescribed
parameter path; it does not replace the coupled main experiment. Define

`s(t)=-49/32+t/16`, `c(t)=s(t)^2+s(t)`, `0<=t<=1`,
`a(t)=b(t)=c(t)`, `u(t)=v(t)=s(t)`.

Specify the ODE as `a'=b'=(2*s(t)+1)/16`, with closure velocities given by the
same compiled tangent circuit. Since `fa=fb=(2*s+1)/16` and
`d=(2*s-1)*(2*s+1)`, the circuit gives `u'=v'=1/16` exactly.
The initial root `s0=-49/32`, initial parameters `c0=833/1024`, and all their
constraint products are dyadic and exactly represented in binary64. Thus
`G(0)=0` holds exactly before the first numerical step. The analytic solution
is the unique exact closure path of this regular initial root.

For `a=b=c`, the global eliminant discriminant factors as
`D(c,c)=(4*c-3)^3*(4*c+1)`.
At `t=1/2`, `c=3/4`, and
`Pu=(u-1/2)^3*(u+3/2)`.
The selected root is `u=v=-3/2`, with local `d=8`; the colliding triple root
is `u=v=1/2`, a different branch. Since `c(t)` decreases through `3/4`, `D`
changes from positive to negative. On the complete selected path,
`d>=1953/256>0`. This is a transverse crossing of the discriminant set
`c=3/4` in parameter space; the discriminant polynomial itself has a third
order zero along this slice.

For branch certification use
`Xc=[13/20,7/8]^2`, `Qc=[-8/5,-7/5]^2`.
The proof-only map
`Tc(u,v)=(-sqrt(a-v),-sqrt(b-u))`
expresses the two original equations on their negative roots.
Every radicand lies in `[41/20,99/40]`, contained strictly in
`[(7/5)^2,(8/5)^2]`. Therefore `Tc(Qc)` lies inside `Qc` and its infinity-norm
Lipschitz constant is at most `5/14<1`. For a candidate in the box,
`||qhat-Tc(qhat)|| <=(5/14)||G||`, since each difference of squares has
absolute denominator at least `14/5`. Contraction yields the original
residual root radius `5||G||/9`. No square root is computed at runtime.
This identifies the same unique negative root at every switch, independently
of all roots outside `Qc`.

The control whole-slab certificate compares cubic candidate `A,B` to the
known quadratic `c` using exact Bernstein hulls, giving `Ex` for slow error.
Writing errors relative to `s`, the original residual equations give the
secant system

`(U+s)*eu+ev=G1+(A-c)`,
`eu+(V+s)*ev=G2+(B-c)`.

On the certified box and analytic path, `|U+s|,|V+s|>=57/20`; strict diagonal
dominance thus gives
`Eqwhole<=(20/37)*(Dg+Ex)`.
This is a whole-slab original-residual proof, independently checked against
an exact polynomial Bernstein error bound for `U-s,V-s`. The four recorded
runs pass the common `1e-8` physical error budget.

## 5. Execution, limitations, and reproducibility

Run from this directory:

```sh
python atlas.py
python test_atlas.py
```

The production campaign contains four runs, 288 certified slabs, 150 actual
switches and 150 certification-time transition rechecks, 1152 RK RHS
calls, and zero online root solves. The certificate evaluates two exact
physical endpoint slope vectors per slab (576 total). These checks are
additional certificate work and are not counted as online RK RHS calls.
Tests are reported separately: 14 test cases, including 11 distinct exact
polynomial identities and adversarial wrong-branch/domain/evidence tests.
`execution.log` records final executions; `results.json`, `test_results.json`
and `formulas.json` contain the machine-readable outputs.

The main N=64 u-first and v-first switched paths have root upper errors
approximately `3.77922e-9` and `3.77957e-9`; the every-step-switch N=128 path
has `4.67123e-10`. This is scoped correctness evidence, not a speed claim.
The control is a regular selected branch while unrelated roots collide.
No claim is made for selected `d=0`, generic global atlases, automatic chart
conditioning improvements, or a general inverse-free compiler. The supplied
u/v charts have identical determinant coverage. Independent review remains
required before final acceptance.
