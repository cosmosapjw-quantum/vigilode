# M2-DAE-CONSISTENT-INIT-01
One bounded node. Source-owned semi-explicit polynomial f,g represented as
sum of products of affine factors from existing DeclaredPolynomial.
nx,nz>=1, total<=32, explicit fixed t,x and caller branch box. Exact
represented binary64 source and fixed x. Strict retains z bits; numerical
acceptance requires original residual and certified distance to unique
root below explicit tolerance; not a claim of exact equality. Project
changes only z with explicit authority. No automatic DAE index inference.
Projection damped Newton with original interval residual. Verify existence
inside branch using preconditioned contraction self-map on contained local
box; uniqueness on entire declared branch via I-C*J interval norm<1.
Scalar alternate exact rational frozen constraint root isolation (both
boundary signs nonzero, exactly one simple root) can prove uniqueness
even when whole-box contraction fails; use same derivative certificate.
C from numerical unit-RHS LU solves solely for certification; solve actual
RHS rather than multiply by inverse. Bound condition and derivative/
Schur errors against actual root interval using C residual and inverse
defect. xdot=f, zdot=-gz^{-1}(gt+gx*f); batch zx=-gz^{-1}gx, zt,
zdot; reduced Jacobian fx+fz*zx. Original residual and rank/branch
must pass; no silent branch selection if ambiguous. Error report for
represented numerical centers relative to exact root derivatives; no
global trajectory certificate. Core direct LU, reused LU, faer batch,
PreparedRhs serial/batch/rayon exact support when admitted, scalar
exact root/dictionary screening plus direct solve, actual RETRO adaptive.
WARP quotient solves linear pencils, not arbitrary nonlinear roots;
exclude unless actual admission, do not fake. Study finite train/holdout
with analytic quadratic, Robertson, stiff row scales, non-DD invertible,
multi-algebraic sparse and nonlinear, nonunique/rank loss/strict refusal.
Fixed numerical state/derivative tolerances 1e-9 / 1e-7 (also tighter
1e-12 / 1e-9); cond upper<=1e10. Independent rational/MP root, g, derivative
and Schur checks; cost selection hard certificate first. New mass facade
client smoke uses exact same source, existing driver; not M2 evolution.
No high-index, reset, full benchmark or default promotion. Publish scoped
branch and compact verified bundle.
