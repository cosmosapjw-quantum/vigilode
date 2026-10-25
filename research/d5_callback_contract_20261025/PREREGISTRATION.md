# D5 preregistration — 2026-10-25 KST

## Candidate selection

| Candidate | Existing evidence | Decision |
|---|---|---|
| Strict callback/Jacobian scratch/domain boundary | Direct native problem.rs read shows direct-storage low-level APIs; D2 NaN scratch already supplies the pattern | SELECTED: actual opt-in gap, reused pattern, connects model/derivative consumers |
| Durable delivery and solver crash restore | D4 journal/pending/ACK exists; full Session/driver authority state is private and not restored | DEFER: separate persistent cut/source identity/budget contract required |
| General dense provider | generic_verified already has immutable provider and unbound callback refusal | DEFER: do not duplicate or weaken existing provider binding |

## Frozen discrimination

The implementation suite will cover valid full writes; callback partial writes
followed by recoverable/fatal errors; Ok with missing entries or NaN/Inf;
invalid t/state with zero callback entry; pre/post epoch/environment failure;
handled callback/probe panic and sticky terminal faults; exact attempted counters;
previous-matrix sparse compatibility; valid and refused centered time stencils.
New Session tests exercise existing abort/snapshot/cap functions with D5 callback
failure, plus retained external side effects. These are contract tests, not a
new accuracy or trajectory campaign.

The polynomial grid uses t,x,y=i/8, i=0..8 (729 points), including all box faces.
Every point records represented inputs, RHS, all four J entries, J*(1,-1),
and attempted counters. Exact rational truth is computed in Python from each
binary64 input, not by copying the Rust callback expression evaluation order.
Expected direct polynomial values on this dyadic grid must match exactly.
Time-difference inputs t=i/8 (i=1..7), x=1/4,y=3/4, eps=2^-p (p=3..9)
give 49 symmetric represented stencils. Expected values are checked against the
exact secant and derivative; a tight 64*epsilon*max(1,abs(reference)) gate is frozen.
Additional boundary and sub-ulp refusals are recorded separately.

Mutations are independent oracle interpretations of the saved outputs: transpose
J, negate off-diagonal coupling, and call finite J+I a true derivative. They are
not compiled Rust source mutations. Each must fail at one or more frozen points;
the number and first witness are retained. Interface gate success alone is
explicitly compatible with a wrong but finite full-write J; this is a scope
counterexample, not a novel strong no-go theorem.

## One production pass and one complementary review pass

S0 freezes scope/design; S1 publishes exact source before new contracts run;
S2 contains actual logs/output and saved-data oracle/plot; S3 closes review and
delivery. The same reviewers may close a concrete finding or inspect the final
saved output; this does not authorize repeating completed experiments.
Any failure preserves original source/log, identifies the changed input/code,
and limits any continuation to the unresolved obligation.

## Resources

New D5 local child + reported Wolfram kernel wall budget: 120 seconds.
Toolchain extraction from the supplied exact archive is necessary after observed
workspace pruning and is separately categorized inside that scoped budget.
Scientific children are serial; timeout<=45 seconds and remaining budget.
Compile and tests use Rust1.94.1. Grid size is finite and frozen. No Cargo network,
NCP, native host, Desktop Commander, old solver or benchmark execution.
Historical remaining budget=null; reset=false. No historical ledger is opened.
