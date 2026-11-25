# M2-DAE-CONSISTENT-INIT-01
Parent ae8539a909f3ef7155d4d6a5d6b72e0858eaeef0, tree in BASE.txt.
Source-owned declared semi-explicit polynomial DAE initialization opt-in
under polynomial-events. Default mass solver and old M0/M1 artifacts unchanged.
Use existing linked worktree, no production release or M3.

## Mathematical admission
State ordering [x,z], explicit nx,nz>=1, total<=32. f,g each immutable sum
of up to 16 weighted products of at most four affine factors in (t,x,z),
as in the existing declared polynomial module. Exact represented binary64
source and t,x; declared bounded closed branch box contains initial guess.
Strict preserves every x,z bit. It is numerical tolerance consistency,
not exact equality unless outward residual is exactly zero. ProjectAlgebraic
explicitly authorizes changing z only; damped Newton with original residual
decrease produces proposals. No silent projection, branch choice or
automatic inference of DAE index from singular mass matrix.

For matrix C from numerical unit-RHS solves, interval defect E=I-C*J(B),
q=||E||inf<1 establishes invertibility (C,J square), contraction and
uniqueness on B. Explicit componentwise T(Blocal) subset Blocal subset B
is checked using T(z)=z-Cg(z) and interval mean value. Existence proof
is separate from convergence of Newton. Local radius derived from
2||Cg(center)||/(1-q), outward construction, self-map verified after
rounding. ScalarRoot freezes t,x exactly as rationals, maps algebraic
coordinate to theta, uses original retro_event_poly DictionaryCompanion.
Both boundary event intervals must exclude zero; exactly one simple
root in declared branch. Safeguarded Newton polish inside already
certified root enclosure; root certificate unchanged. This can bypass
conservative broad-box derivative dependence. General branch contraction
or exact scalar count required, no guessing multiple branches.
At exact root interval, recompute J and q; inverse norm upper
||C||/(1-q), condition upper ||J||*||C||/(1-q), condition cap 1e10.
Compute xdot=f, zt=-gz^-1 gt, zx=-gz^-1 gx, zdot=-gz^-1(gt+gx*f)
by actual solves; C only certifier, not a numerical inverse multiplication
solver. For each candidate v against true root interval J,b use
||v-v*||inf <= ||C(b-Jv)||inf/(1-q). Return componentwise maximum
derivative/Schur error, not matrix operator norm error for Schur.
Reduced Jacobian fx+fz*zx, original tangent residual at represented center
checked separately; correction bound, original/current constraint residuals,
root box, exact-center flag, rank and condition bound. The preconditioner
matrix itself is not serialized; independent audit checks exact source,
root, condition/derivative and residual claims but is not a full
independent interval self-map replay. Certificate proof is direct,
conditional on directed arithmetic contract. No evolution proof.
Scalar branch boundary root/unknown conservatively refused by root route.

## Candidate matrix and measured results
MathHarness original kernel protected verification (53 files),64 catalog
proposals,7 mapped ideas,8 executable routes. Direct, Reuse, FaerBatch,
PreparedSerial/Batch/Rayon, ScalarRoot, Adaptive. PreparedRhs requires
strict row dominance and uses stored exact structural support when admitted;
no pruning of small nonzero values. Factor/scratch reuse within batch;
separate prepared owners rebuilt per new linear batch, no across-time
factor reuse. ScalarRoot actually uses retro_event_poly::frozen_scalar,
dictionary basis and conditioned Horner when admitted. Adaptive actual
retro_portfolio::choose with fresh independent certificate each attempt;
attempt time/reason retained but failed attempts' internal work counters
not returned, so work totals are successful selected reports only.
No WARP generic nonlinear root claim; future compatible affine pencils
may reuse TensorPencil/warp_cubic. No arbitrary opaque callback path.

9 training sources x2 tolerances x8=144,118 qualified and26 refusals.
State tol loose1e-9/tight1e-12, original residual1e-9/1e-10,
derivative1e-7/1e-9. Each tolerance: Direct/Reuse/FaerBatch8/9,
PreparedSerial/Batch/Rayon7/9, ScalarRoot5/9, Adaptive9/9.
12 contraction refusals (wide cubic z^3+z-2 [-2,2]),8 scalar route
dimension refusals,6 prepared row dominance refusals. Existing scalar
root route proves unique simple cubic root; no claim general nonlinear
failure. Newton/interval route conservatively fails wide box because
interval dependencies obscure positivity. NonDD invertible matrix
[[1,2],[3,1]] admitted direct/reuse/batch; prepared refusal retained.
Strict incorrect input, approximate strict without bit changes,
ambiguous ±1 branch, rank loss, condition cap (scaled diagonal 2^-40
vs 1) all checked. Scalar polish fixed initial failing exact-center
test, raw failure retained. RED API stub returns intended refusal
exit101; same focused projection test passes after implementation.

Independent SymPy exact source reconstruction, symbolic/85-digit root
and derivatives, original constraint and tangent residual exact rational
checks, inverse norm/condition and actual state/derivative errors checked.
Max training actual state error 1.2537167179050217e-16,
max state bound 2.220446049250313e-15; max actual derivative error
1.7763568394002506e-16, max derivative bound 1.776356839400257e-15;
max residual bound 4.440892098500626e-16, condition upper3.200000000000002.
No observed underbounds in accepted finite cases.
Training final successful-report totals: 1050 linear RHS,412 batches,
162 PreparedRhs owners,54 exact-support batches,5400 prepared residual
terms,114 faer batches,54 Rayon batches,20 root isolations/dictionary uses,
36 conditioned Horner calls,168 Newton steps,1130 evaluations.
Counts not including failed/unchosen adaptive routes or warmup runs.
MathHarness GP-UCB on already measured discrete lookup and exhaustive
selection agree all8 groups/tolerances. No reduction of solver runs claim.
Frozen holdout 4 sources x2=8/8 certified. Timings owner-host unpinned,
median three warm invocations including certificate/receipt serialization;
not global DAE solver wall time. At tight sparse nx=nz=8: FaerBatch163.234us,
Reuse168.094us, PreparedSerial177.241us, PreparedBatch185.488us,
PreparedRayon377.216us. Retain slower paths for larger M3 scope.
Selection scalar Reuse/FaerBatch, branch Adaptive/Adaptive, multi
Adaptive/FaerBatch, nonDD Reuse/Reuse. No universal optimality.
Exact MathHarness 8 finite algebraic identities fresh verified;
Wolfram tangent, sqrt derivative and adjugate identities independently
verified. Exa/SciSpace/Parallel and official IDA docs grounded literature.
No full novelty claim.

## Existing mass driver integration smoke
initialize_mass_problem constructs Problem from same owned source f,g,
analytic Jacobian and partial time derivative, M=diag(I,0), corrected
initial state returned separately; no replacement callback input.
Existing sequential native mass driver actual runs at rtol1e-7,atol1e-10,
dense enabled, t=0..1. Analytic g=z-x^2-t:19 accepted output nodes,
max absolute state error1.2621681477753555e-11, exact original constraint
1.2885914725668242e-11,162 RHS. Robertson conservation DAE:27 nodes,
max absolute error3.3132452337270024e-10 vs independent SciPy reduced
Radau rtol2.5e-14,atol1e-16; exact constraint1.8568995082887674e-16,
250 RHS. These are empirical accepted-node smoke, not a dense or
global rigorous state certificate; no arbitrary callback DAE consistency.
No new Diffsol/Julia/IDA matched-error campaign (M6).

## Next bounded node M2-DAE-CONSTRAINED-EVOLUTION-02
Start from this source/initialization report and actual mass solver.
For declared index-1 source, compare native mass and Schur reduced ODE,
reuse factorization when exact Jacobian source identity permits,
compute algebraic projection at accepted nodes and dense queries with
explicit branch continuity and rank/conditioning checks. Derive
constraint residual plus state-error propagation, distinguish local
branch certificate from global integration error. Use exact/source-bound
DAE fixtures: analytic nonautonomous, Robertson, ill-conditioned mass,
nonunique/rank loss. Use compatible previous modules (PreparedRhs,
directed residual gain, retro_event_poly for scalar branch,
conditioned_polynomial, WARP quotient only actual affine pencils).
Include candidate measured full-contract comparisons and external
independent references; avoid general index inference. Keep reset
consistent reinit, nonlinear/high-index reduction and M3 separate.

## Final verification and review
94 targeted tests passed (59 facade,32 integrators,3 core),0 failed,
12 ignored; two new campaign tests separately executed and replayed.
Remaining ten ignored not executed this turn. Clippy -D warnings,
feature-on/off/retro-only checks, rustfmt, diff check passed. Initial
style lint corrected without changing numerical results. Final binary
replay 152 cells,131 fixed exact non-timing reports,21 adaptive fresh
independent checks; both mass trajectories exact replay. MathHarness
manifest 1058 inputs,10 outputs artifact-byte verification. The replay
script actually executed commands; validator itself checks receipts.
No separate native subagent execution exposed; direct source review and
independent numerical/algebraic oracle, Superpowers verification skill
used. Review caution: C matrix not persisted, no global trajectory proof,
prepared routes only row-dominant and rebuild each batch, no arbitrary
DAE/rank inference, no production promotion. Publish only this research
branch, not main. Preserve previous unique worktrees/backups.
