# M1-POLYNOMIAL-ROOT-ISOLATION-02
Parent ad3fbd15b03bbc285c0db78f0fc786654d1953ce, tree in BASE.txt.
Scope: opt-in feature polynomial-events, one declared polynomial event on
forward identity-mass fast dense/banded path. No change of default route.
Original M0 reference and M1 sampled event evidence untouched.
Baseline C06 failed both original hidden-pair cells on actual re-execution.

## Exact target and implementation
Source = sum of at most 16 weighted products of at most 4 affine factors
of physical time and state, at most 16 state weights/factor. Input binary64
values interpreted exactly as rationals (num-rational 0.4.2 / num-bigint
0.4.8 / num-traits 0.2.19, optional feature). Exact physical clock is
represented endpoint difference. Native H and stage dot products are exact
rational sums; U(theta)=y0+(y1-y0+d0)theta+(d1-d0)theta^2+
(d2-d1)theta^3-d2 theta^4. Compose source exactly, preserving factor
multiplicities. Direct fixture polynomial up to degree16. Degree/normalized
rational-coefficient bit caps, isolation depth/node caps; not a hard total
allocation/time limit. Zero polynomial unresolved, no false no-root result.
Sturm of square-free part on (0,1], gcd layers for multiplicity; odd direction
from sign after root, even Touch. Root brackets exact rational (lo,hi] or
singleton plus outward binary64 theta and physical time projection. Width
target normalized theta, not physical global IVP error. No physical root
enclosure implied. Fast transaction retains full proposal work, clips
physical output to upper bracket of earliest admitted terminal root.
Distinct roots with overlapping represented time brackets refused
conservatively, no merging. Initial zero excluded. One declared event,
no opaque mixing, mass/Enforce/endpoint routes refused before RHS. New
typed failure and distinct Stats schema, returned trace with source coefficients
when capture_all or roots present. Trace retains all successful slab reports;
refusal reason recorded, failed slab report not always available. Memory
scales with accepted slabs. Existing opaque Touch request explicitly refused.

## Actual existing VigilODE modules used
Native affine_event_certificate::bernstein outward quartic conversion and
directed arithmetic; actual FastDenseSegments::represented_component retains
same native packet as affine certificate. retro_dictionary::basis(3/5/7)
Lagrange/Bernstein intervals consumed in degree<=6 screen, using
conditioned_polynomial::eval_poly_interval when exact coefficient projection
permits, else enclosure Horner. Same strict sign exclusion, de Casteljau
half-split, exact Sturm fallback. Degree>6 falls back to generic Bernstein
conversion (not dictionary use). faer eigenvalue hints on square-free
companion never omit root intervals; exact counting remains authoritative.
Five explicit combinations plus actual retro_portfolio::choose adapter
ranks fresh certificates, no cached root answers. Adaptive may be slower.
WARP/Green/Picard/Fourier and other prior modules were not misused as generic
root count proof: they solve different mathematical contracts. Future
physical root enclosure may combine actual affine carry, RETRO Green and
causal error budgets under their real admission conditions.

## Actual experiments
Seven initial contract tests and added Robertson packet test. Exact rational
SymPy checks: 36 kernel reports and 46 native polynomials from first contracts;
20 independent MathHarness Sturm checks (its endpoint-root refusal respected;
SymPy checks endpoint conventions). Fraction independently reconstructs all
coefficients of three actual Robertson native packets, raw H and stage rows
included. Training: 72/72 (9 kernel fixtures +3 native, 6 routes), 186
polynomial reports, 84 roots across reports. Held-out 6/6,16 reports,8 roots.
Hidden pair .431/.432 and repeated .331 now detected in real driver.
Degree16 multiplicity, near-cluster, endpoint exclusion, nonfinite/zero
refusal and binary64 large clock overlapping roots checked. Native maximum
analytic event time error 6.424225595935695e-10 on training.
MathHarness generated 64 route proposals (see actual HARNESS_ROUTES.json),
mapped six catalog concepts to executable policy combinations. GP-UCB on
already measured discrete lookup and exhaustive constrained selection agree
for four fixture groups. No claim fewer solver evaluations. Frozen selection
before holdout. Original dictionary unchanged. Training actual final receipts:
47 dictionary uses,20 native Bernstein uses,45 conditioned Horner calls,
38 companion attempts,76 proposed hints,114 strict sign excluded intervals.
Counts refer to final reports of measured cells, not all warmup repetitions
or unsuccessful adaptive attempts. Timings median of three warm calls,
rotated ordering, unpinned owner-host diagnostic only. Native group aggregate
65.192436ms Sturm vs19.791206ms DictionaryCompanion (3.29x), not universal
IVP speedup. Root-heavy group Companion preferred. High-degree hybrid falls
back to generic Bernstein. Root-free low degree dictionary screen useful.
Final frozen campaign replay exact static receipts 66/66, adaptive cells
12/12 recertified (timing can change route choice), MathHarness manifest
1046 inputs,3 outputs checked. No full solver matched-error rerun or release
promotion. Native subagent and separate Superpowers tools unavailable.

## Next bounded node: M1-PHYSICAL-ROOT-ENCLOSURE-03
Use source-declared polynomial and original affine-event physical state tube
carry; derive bound on |g(t,y)-g(t,U)| using interval gradient over tube,
strict sign exclusion and monotone/transversal root intervals. Combine
existing affine_event_certificate with retro_dictionary Green/moment
transport and retro_causal gain budgets where their premises apply, actual
source-bound residual/initial error only; do not equate LTE with global
state error. Test affine decay and coupled source-bound linear cases first,
then factored Touch via underlying transversal factor and unstable grazing
counterexamples. Candidate search: cheap sign certificate, derivative tube,
Green-weighted residual, exact represented root proposal plus independent
physical verification; adaptive choose by measured cost after admissibility.
If physical certificate cannot close, report unresolved. Arbitrary opaque
root isolation and reset/restart (M1-RESET-TRANSACTION-04), DAE consistency
and M2 remain separate. Do not start next node in this turn.

Final targeted regression: facade 42, integrators 37, total 79 passed,
0 failed, 1 ignored (existing owner timing case not executed). Final
Clippy -D warnings, polynomial-events / retro-only / feature-off release
checks, rustfmt and locked standalone benchmark metadata passed. Full
workspace and external competition campaign not run. See FINAL_RESULTS.json.
