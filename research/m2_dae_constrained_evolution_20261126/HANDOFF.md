# M2-DAE-CONSTRAINED-EVOLUTION-02
Parent 2655ac2004554f77856ce148e642860a41a94229, tree in BASE.txt.
Opt-in polynomial-events feature. New source-owned facade dae_evolution
and core retro_dae_evolution. Existing native solver defaults untouched.
Fixed t0=0, declared polynomial index-one source, explicit initial projection
authority from InitPolicy, nx,nz>=1, total<=32. Scalar driver atol only,
dense output required. No DAE event/reset or M3 execution.

## Source and mathematical contracts
Mass path existing sequential driver M=diag(I,0). Reduced path projects
z for every RHS, Jacobian and partial time derivative callback using
same immutable source, fixed seed (never mutate from rejected stages).
Jacobian fx+fz*zx, ft+fz*zt. Numerical trajectory may fail if projection
refuses. No claim of an efficient optimized generic reduced ODE driver.
All accepted nodes and declared query grid receive fresh algebraic
projection at represented x; raw trajectory is immutable in returned
owner (read-only access). Arbitrary query() reprojects, refuses outside
driver. Raw mass algebraic residual recorded separately. Point projection
does not alter integrated x or provide a global IVP error by itself.

Native packet authority: sequential uses dense_d; fast uses dense_h.
Initial implementation incorrectly used H for both, diagnosed by native
polynomial leaving branch despite correct rounded driver output.
Initial failing contracts and 80 exploratory rows preserved, excluded
from final selection. Now explicit DenseBasis and actual stage/endpoint/
coefficient receipt; independently reconstruct exact represented
polynomials. Reduced z candidate uses rounded monomial coefficient
centers from actual 3/5-node dictionary Lagrange basis and original
projection at nodes. It need not be globally continuous in z;
physical branch uniqueness and residual enforce closeness. Mass z is
actual native raw dense polynomial. No claim of a global z polynomial
projection at all t. Fresh query evaluates raw x and projects z.

Uniform parameter box per slab: t interval, Bernstein hull of U_x
inflated by tube radius, declared algebraic branch B. Preconditioner C
from LU at midpoint. q=||I-C*g_z||inf<1 on entire B proves uniqueness
and regularity. Explicit parameter-uniform self-map verifies existence;
if whole B map not contained, try contained smaller local box and
its own Jacobian interval. Persist C, map center, local box and image.
All actual V range in B. Let epsilon_z=||C*g(t,U,V)||sup/(1-q),
use original polynomial coefficient composition and Bernstein bounds.
Differential residual r_x=U'-h f(t,U,V) normalized. R=||r_x||sup+
h||f_z||sup*epsilon_z. Reduced Jacobian via interval solve
w=C*(-gx), error <= q*||w||/(1-q); mu_inf upper from row-wise log norm.
Transfer E'=mu E+R/h. For negative mu use actual RETRO Green dictionary,
retro_causal::gain and phi1; positive mu use exp(mu*h)*(E+R) upper.
For nonpositive mu uniform coarse path E+R. Whole-slab bound max
of uniform comparison endpoint and initial radius; strict E<tube
bootstrap keeps actual IVP inside certified parameter domain. Next slab
incoming bound inherited only after previous global proof. Algebraic
error <= epsilon_z+||Z_x|| E. Query roundoff charged via original U
and outward theta; projected algebraic state bound adds actual local
projection error. Physical global qualification only if every slab and
all returned queries have proof and max full state bound <= global_tol.
Core helper certificate conditional on supplied polynomial/initial error;
facade binds original accepted packets and exact initial x. This is not
a general solver for arbitrary DAE, mass matrix or high-index inference.
No automatic physical rollback of old mass/reduced driver; posthoc proof.

## Actual candidate generation and research results
MathHarness protected kernel verification,64 catalog proposals mapped
to eight executable candidates. Mass, Reduced3, Reduced5, Prepared5,
WarpDirect, WarpFolded, Scalar5 and Adaptive. PreparedRhs used at each
actual projection batch; no persistent across-time prepared owner reuse.
WARP time-affine g_z structurally inspected: z-containing term one
linear factor and at most one other time-only factor. Exact represented
pencil coefficients from 0,end. Existing TensorPencil with nilpotent
xi^6, inverse action coefficients by original solver, direct and folded.
Conditioned Horner evaluated at actual query t, predictor only; original
projection still required. For g_z=1+t/8 actual jet [1,-.125,.015625,
-.001953125,.000244140625,-.000030517578125], quotient error zero,
folded6 iterations. Infinite-time inverse residual -(t/8)^6. Refuse
constant/nonlinear pencil, no generic nonlinear WARP claim.
All routes use same hard pointwise/branch authority; global proof
tracked separately. Training 5 sources x2 nominal tolerances (1e-5,
1e-7) x8=80:62 qualify empirical abs error 2e-5/2e-7 and pointwise/
uniform branch,47 global physical (tube and global budget 1e-3).
18 admission refusals:12 constant pencil,4 nonlinear,2 scalar dimension.
15 numerical/branch successes without global certificate. Reduced3
lower degree reconstruction often wider algebraic bound. Max accepted
training empirical error2.165978005526313e-6, projected residual
2.220446049250313e-16, global bound8.844946320356486e-4.
Robertson all admitted routes: numerical/branch pass but no global
cert. Mass tight first bootstrap failure [1.4588513e-4,3.0918865e-4],
mu_upper62173.1, E_whole .03227>1e-3. No bound lowering or success
claim by finite samples. Two-level independent SciPy Radau reduced
reference spread<=1.672e-13, empirical only. Other refs 70-digit analytic.
GP-UCB finite measured lookup and exhaustive min cost agree all10
groups/tolerances. No claim fewer executions. Nominal tolerances not
matched-error solver speed; measurements include serialization, proof,
queries and setup, median three warm runs on unpinned owner host.
Native mass typically cheaper, all-five tight aggregate15.112ms vs
Reduced5 54.575ms, Prepared5 56.254ms. On pencil tight WarpFolded
5.492ms vs Reduced5 5.638ms, Mass2.544ms; no robust speedup.
Analytic tight Reduced3 bound5.7963e-4 vs Reduced5 3.2388e-6 with
same differential error2.20e-8. Dictionary degree matters.
Training final selected-report work:8418 projection calls,56278
linear RHS,34148 linear batches,8814 prepared owners,0 exact-support
batches (these small systems not admitted for exact sparse support),
1514 scalar root calls/dictionary uses,777 conditioned Horner calls
from root isolation,584 WARP predictions,376 Green slabs. Counts
do not include failed/unchosen adaptive full runs, initial mass double
init or slab preconditioner work; timings include all.
Initial frozen holdout10 cells:9 qualified,7 global. Quad-holdout
tight Mass refused local initialization self-map at dense query.
Preserve original selection and negative. Separate recovery protocol
runs Adaptive on same five held-out sources x2,10/10 branch+accuracy,
8 global. Failed cell recovers via Scalar5 with physical bound3.4106e-6.
No independent new holdout; selection not retuned.
RED actual WFilteredEnforce raw dense residual3.951845e-6; exact
same options and initial data in new client: projected2.220446e-16,
101 points, global bound9.0980203e-6. Off campaign different,
raw3.4358279e-4, projected1.110223e-16; do not conflate.
Rank loss, ambiguous branch, too-tight bootstrap, unsupported WARP
checked. Initial compile error, failed basis mapping, failed self-map,
audit variable shadow, initial lint retained. Existing solver unchanged.

## Independent checks
Rational Fraction intervals independently reconstruct actual native
power coefficients, original g and x residual Bernstein bounds, C*J
defect, local self-map, condition and Schur mu, state/derivative transfer.
Exact rational source residual at represented queries; independent
analytic branch roots and physical refs. 70 digits for analytic state,
matrix exponential and pencil; two-level SciPy Radau for Robertson.
Training598 slab checks,2706 queries,1880 ideal trajectory samples.
Holdout87 slab checks,388 queries,275 ideal samples; recovery84 slab
checks,424 queries,260 ideal samples. No observed bound violation.
MathHarness exact9 finite identities fresh verifier; Wolfram derivative,
geometric remainder, tangent independently checked. Exa/SciSpace/
Parallel source discovery in LITERATURE.md. Not a formal global
proof by finite samples; analytic derivation and interval authority
are separate. No full external matched-error campaign, no default
promotion, no native subagent execution exposed. Source review and
independent arithmetic audit used.

## Next M2-DAE-EVENT-RESET-CONSISTENCY-03
Implement actual DAE event/reset transaction with declared differential
and algebraic partition. On event, reset differential variables only
unless explicit algebraic projection authority; recompute consistent
z and derivatives using M2-01, validate branch and regularity in new
source epoch, re-enter native mass or reduced client. Do not conflate
represented root with physical event; propagate bracket clock
uncertainty and reset/state tube where verified. Use M1 exact polynomial,
accepted-step stop, physical enclosure/reset, M2 constraint proof,
PreparedRhs and compatible WARP time pencils. Existing sequential mass
path may need accepted control hook; preserve charged work and rollback.
Test nonlinear constraint reset, Robertson conservation, incompatible
branch, rank loss, simultaneous/initial guards and anti-Zeno caps.
General coupled stiff global certificate remains open: future
componentwise Schur/Green majorants or weighted norm/scaling and
subdivision, no relabeling current failure. High-index explicit reduction
and full M3 remain separate. No arbitrary DAE index inference.

## Final verification and review
104 targeted tests passed (64 facade,40 integrators),0 failed,
15 ignored; three new campaign tests separately executed and replayed,
remaining twelve not executed in this turn. Clippy -D warnings,
feature-on/off/retro-only checks, rustfmt and diff check passed.
Final binary replay100 cells,75 fixed exact non-timing receipts,25
adaptive independently reaudited, original holdout failure unchanged.
Manifest1063 inputs and10 outputs byte checked. Review Caution:
Robertson physical global proof open; point branch verification is not
a global trajectory certificate. Initial adapter used wrong sequential
basis and was repaired before final experiments. Self-map and audit
failure receipts preserved. Full workspace, external matched-error and
native independent subagent not run; Superpowers inline/verification
and direct source review used. Existing switcher governance ledger
unchanged. Publish only research branch, preserve prior worktrees and
unique backups; no release promotion.
