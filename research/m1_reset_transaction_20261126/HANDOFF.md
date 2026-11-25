# M1-RESET-TRANSACTION-04

Parent 440cba96102f1de64037d20dbe527b12a6e83c5a; exact tree in BASE.txt.
Scoped opt-in scalar source-owned affine hybrid driver, feature polynomial-events.
No default solver change, no M2 implementation. Original M0 reference,
sampled, polynomial and physical root evidence unchanged. Prior
physical-path full-horizon behavior reproduced as RED symptom (101);
new accepted-stop API passes equivalent contract. RED fixture and raw
output preserved; not a claim that the old a posteriori API was erroneous.

## Mathematical and implementation contract
Immutable scalar mode y'=-a*y+b, a>0, one deterministic threshold and
affine reset R(t,y)=m*y+c+d*t per guarded mode, declared next mode.
Initial state exact binary64. Original source, threshold, clock and
epoch bound in affine certificate. Each hypothetical continuation in
one mode is monotone or constant. All members of the incoming state
enclosure start on the same strict side of the threshold; equal strict
sign at a later point excludes any prior crossing, opposite strict
sign proves a unique first crossing for each member. Uncertain
intermediate endpoints may be traversed only to obtain a later
opposite strict sign, while retaining the last common safe time.
Do not certify a physical terminal stop at the earliest possible
actual root: native accepted proposals through the first proven sign
bracket are charged, then stop before the next proposal. A shared
physical interval-Newton contraction from retro_physical_event narrows
the all-members root set. Original source flow and affine carry, not LTE,
authorize signs. Represented root can only seed contraction, except
in separately labeled nonphysical control. Returned represented reset
uses actual rounded native state at upper bracket endpoint.

At true root tau, physical pre-reset state equals exact threshold q.
At synchronization time s=upper endpoint, actual next state is
Phi_new(s-tau,R(tau,q)). Rectangle enclosure takes hull of reset box
and flow of its endpoints at maximal elapsed time (scalar monotonicity
and positive initial-state gain). Gap must strictly exclude next guard.
Optional correlated interval mean-value bound around tau=s:
d/dtau Phi_new(s-tau,R(tau,q)) = exp(-a_new*(s-tau)) *
(d+a_new*R(tau,q)-b_new). Intersect with rectangle. This is a finite
interval bound, not only a first-order saltation approximation. New
mode restarts at s, center and enclosing radius constructed internally.
Require state radius and root width, minimum certified advance and reset
cap before commit. Rollback preserves preceding committed prefix and
attempted epoch counters. Driver calls yielding normal partial receipts
retain all counters; hard CoreError before a driver receipt does not
expose unknown internal work. No arbitrary callback side effects.
No full dense hybrid trajectory exposed, especially no single-valued
state guarantee inside event-time uncertainty gap. At most 32 modes,
64 resets, 100000 attempts per epoch, rate*horizon <= 16384.
Initial guard touching zero refuses. A near-next-guard in reset gap,
cap, too-small state budget, insufficient advance and driver attempt
budget tested. General simultaneous guards, directional filtering,
grazing, multidimensional/nonlinear hybrid dynamics, general DAE resets,
continuous physical hybrid dense output and high-index consistency remain
open; no generic anti-Zeno theorem (bounded contract only).

## Candidate combination guidance and experiment
MathHarness original protected kernel verified; 64 exploration proposals,
six mapped to eight actual routes. Represented is nonphysical control.
Carry uses affine_event_certificate::certify/carry/query, LegacyFlow
uses retro_green::RetroTable and retro_causal::flow/gain; DictionaryFlow
uses actual full Hybrid dictionary mode. Hinted adds retro_event_poly
DictionaryCompanion, Transport adds correlated synchronization, Combined
adds affine carry and root hint. Shared physical interval Newton reused.
Adaptive uses existing retro_portfolio::choose, all returned candidate
attempts, counters and work retained; no result cache. Prepared RHS/WARP
not used because scalar fresh-epoch contract does not require them.
Causal budget planner not used; actual gain and flow called.
Training 7 sources x2 time tolerances (1e-6,1e-9) x8 routes =112 cells,
98 physical,87 qualified,11 unqualified Carry due bracket width,
14 nonphysical represented controls. State tol 1e-4. LegacyFlow,
DictionaryFlow, Hinted, Transport, Combined, Adaptive all 14/14.
Selection excludes represented, hard physical contract before measured
cost. GP-UCB on already measured lookup and exhaustive choice agree in
all eight group/tolerance selections. No online run-reduction claim.
Frozen holdout 4 sources x2 =8/8 qualified. No-event Hinted/Transport
selection not evidence of useful root hint or transport without an event.
Timings median of 3 warm runs, unpinned owner host, include native
solve, verification, accounting assertions and receipt construction.
Not solver-only or matched-error external ranking. DictionaryFlow
aggregate at 1e-9 2.360ms vs LegacyFlow4.083ms for same 7 fixtures.
Hinted/Combined more expensive but narrower brackets in some cases.
Correlated-clock same bracket rectangle width 2.8577917809968767e-11
vs transport 1.9451107391432743e-13, 146.9218x narrower, not speedup.
Cycle 3 physical events all enclosed; alternating 4 transitions.
Actual training final-receipt totals including adaptive attempts:
553 affine carries,7923 flow,2952 gain,314 represented isolations,
44 hints,605 contractions,133 sync rectangles,55 sync transports.
No universal selection or complete generic hybrid qualification.

## Independent validation
85-digit mpmath analytic scalar oracle, exact represented source,
2741 endpoint checks,145 physical event records (includes adaptive
attempts) over train+holdout; no underbound. Independently 9x9 points
per reset candidate-time/gap,11745 checks; max observed radius ratio
1.0, no exceedance. MathHarness 6 finite exact identities fresh verified.
Wolfram independent differentiation returned 0. Combined initial
Exa/SciSpace/Parallel/Wolfram context invocation timed out; use actual
primary saltation article discovery and own derivation, see LITERATURE.md.
SciPy DOP853/Radau/BDF diagnostic 42/42 completed at two nominal rtols;
not matched-error timing, no new Diffsol/Julia campaign this node.
All original data preserved. Final scoped facade 41, integrators 47,
88 passed,0 failed,9 ignored (2 new campaign tests separately executed
and replayed; remaining 7 not run this turn). Clippy library -D warnings,
feature off, retro-only, polynomial-events, rustfmt and diff check PASS.
No full workspace or native subagent/Superpowers execution.
Final binary actual replay of 120 cells:106 fixed reports exact,
14 adaptive reaudited; 1056 manifest inputs,9 outputs byte checked.
See FINAL_RESULTS.json, REPLAY_RESULT.json, raw logs and proof scripts.

## Next bounded node: M2-DAE-CONSISTENT-INIT-01
Parent original RESEARCH_PLAN.md M2. Keep M1 residual capability gaps
explicitly open; advance sequentially to M2 initialization, not a claim
that general M1 is closed. First inspect actual constant-mass and
sequential solver contracts. Source-declared semi-explicit index-1
x'=f(t,x,z), g(t,x,z)=0. Strict checks original IC without mutation;
ProjectAlgebraic only on explicit caller authority, fixed x0; solve g
and calculate xdot=f and zdot=-g_z^{-1}(g_t+g_x f) with checked rank,
conditioning and residual. No automatic high-index inference from mass
matrix singularity. Candidate generation: direct dense LU, existing
PreparedRhs/cache and exact-support residual, faer/Rayon for meaningful
batch, Schur/conditioned polynomial if actual compatible source,
retro_portfolio measured selection. Use MathHarness exact Schur and
branch counterexamples, numerical oracle, rank-loss/ambiguous branch,
inconsistent strict vs projected IC, existing analytic index-1 and
Robertson mass examples. Distinguish residual, state error, derivative
consistency and correct source epoch. Event reset consistency extends
this only after initialization is verified. No M3 in same node.
Full matched-error external comparator campaign remains deferred M6.
