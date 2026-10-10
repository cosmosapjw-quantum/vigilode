# Accuracy/speed delta re-audit — 2026-10-10

## Frozen before new executions

Base source: `cfed140bf127f5aa8fcf4fe4c16689d7de062b87`, branch
`audit/rvj-algorithmic-directions-20261008`. New publication branch:
`audit/rvj-accuracy-speed-reaudit-20261010`. The latest implementation already
contains the October 5 speed work and the October 8 imported offline pilot.
The separate `research/rvj-full-history-20261004` archive remains a provenance
source, not a competing latest implementation. Published runs are reused after
one delta evidence audit; no ALG01–06, SPD01–09 or old polynomial/time campaign
is rerun. Scientific novelty here is a bounded admission/porting experiment,
not a claim to a new matrix-analysis theorem.

Question: which new correctness barriers remain after ALG04–06, and can a
finite, current-operator residual gain witness supply one missing native seam?
Production algorithms, defaults, dependencies, lockfile and historical records
are protected. New research example/prototype, review documents and numeric
evidence are in scope. No wall-time, global-order, nonlinear-accuracy or default
promotion claim is permitted.

## A. Focused implementation adversaries

Use a new example `crates/rodas5p-integrators/examples/reaudit_20261010.rs`
through existing public APIs. Record rather than silently repair observed
behavior. Fixed adversaries from static inspection, not tuned trajectory runs:

1. Predictive controller: accepted history h=1,error=0.5 followed by h=1e-100,
   error=1e-200, estimator order 5; independent logarithmic/exact-exponent
   calculation must distinguish min-factor from max-factor underflow.
2. Post-rejection informative output clipping: requested h=1, trial h=0.75,
   error=0.001. Record next h, pending flag and stated cap semantics. A finding
   requires public behavior plus source trace; an ambiguous restoration policy
   is not reported as a proved integration failure.
3. Staged GMRES true residual accepted at floor/stall above the requested
   threshold: small declared systems, identity and 2x2/4x4 triangular/cyclic
   controls. Maximum 12 configurations. Record status, true residual, target,
   counters, and whether the driver charges that acceptance category.
4. Finite components with overflowing unscaled L2: scaled staged solve on
   four components, budget=restart=1 and a cyclic operator, b components
   1.5e308, scale 1e308. Record the driver's fallback predicate including
   nonfinite values. Distinguish direct driver reproduction from a reproduction
   of its public solver callback seam.

Commands (from repository root, supplied offline Rust 1.94.1 environment):
`cargo run --offline --locked --release -p rodas5p-integrators --example reaudit_20261010 -- research/reaudit_accuracy_speed_20261010/NATIVE.json`
then
`python3 research/reaudit_accuracy_speed_20261010/check.py`.
Compilation/debugging may repair the research harness; preserve first failures.
No unchanged retries of published scientific runs.

## B. Native local linear-solve certificate prototype

Implement `gain_witness.rs` within this research node, included by the example.
Contract: real represented 2x2 finite W, finite b and candidate x, positive finite
error scales s; W is the exact matrix of its supplied binary64 entries. Compute
an outward interval inverse, recompute b-Wx outward, enclose each correction
component divided by s. Its infinity upper bound also bounds weighted RMS.
Singular or determinant-zero-overlapping intervals, invalid inputs and overflow
must reject. The witness owns the current W/scales and exposes immutable results;
it is neither a deserializable authority object nor a cached JVP estimate.
W formation/JVP uncertainty, off-block couplings, nonlinear stage transfer,
time accumulation and global endpoint accuracy are explicitly outside this
prototype; a future caller must add those bounds before claiming them.

Discovery grid: W=[[1,K],[0,1]], K in {0,1,1024,1048576}; scales in
{(1,1),(2^-20,2^20),(2^20,2^-20)}; b=(1,1); candidate from the analytic inverse
with a 2^-40 perturbation in coordinate 1 or 2: 24 cells. Controls: W=[[3,1],[1,2]]
and W=[[1,1],[1,1+2^-40]]. Negative controls cover singular W, nonfinite W/b/x,
nonpositive or nonfinite scales and arithmetic overflow (at least 8 cases).
Fixed disjoint holdout (no tuning): K in {7,65536}, scales (2^-7,2^9),
b=(3,-2), candidate perturbation 2^-30 in either coordinate: 4 cells.

Independent Python Fraction oracle interprets every exported float as its exact
binary64 rational, solves 2x2 systems without the interval implementation, and
compares squared WRMS error with the squared exported upper bound. Gate: every
admitted grid/control/holdout enclosure contains that exact error; every invalid
control rejects; no false claim for unidentified operators. Record bound
usefulness separately (bound/error), without an ex post threshold. If a gate
fails, prototype stays REWORK/HOLD; local fixes may be made and affected cases
rerun, preserving the original failing evidence. PASS promotes only this bounded
research seam to further porting, never production or generic certificates.

## C. Evidence/checker and integration audit

One read-only review of current preregistrations/checkers/raw artifacts and
imported pilot claims. Tiny malformed-input probes may use copies: empty or
truncated component arrays, NaN/nonfinite values, wrong twin identity. They do
not replace/rewrite historical RUNS or re-score their verdicts. Record exact
observations and whether a validation gate truly depends on each identity.

Deliver Korean review, claim/source reuse inventory, findings and executable
next-development DAG with prerequisites, file/API targets, error/cost units,
pass/kill gates and non-promotion ceilings. Include Laguerre/Chebyshev/Leja and
homotopy parallel work, updated from PP01–PP13/SPD/ALG evidence. Require a final
independent reviewer who did not design B before a bounded prototype decision.
Methodology: GPT-6 Astra v4.0.0 research/coding harness packages; actual runtime
model identity UNKNOWN, harness-model performance NOT_EVALUATED.
