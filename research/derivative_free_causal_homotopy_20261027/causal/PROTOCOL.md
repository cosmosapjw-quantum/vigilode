# Causal reuse experiment, registered before execution

Parent: `68a28357d960c4c2743c299770f0a33ef63245b2`. The legacy
`BatchedPicard` arm remains callable in the same binary. The new opt-in arm is
`BatchedPicardReuse`. `Forward` is an additional work comparator.

The manufactured autonomous identity-mass problem is
`f_i(y)=a_i y_i+epsilon*y_i^2`, `a_i in {-1,-20,-100}` repeated by component,
with `y_i(0)=0.1`. Nonlinear callbacks are deterministic. Frozen JVPs remain
in the projected implementation. This experiment does not label that path fully
derivative-free; the independent structural consumer is a separate lane.

Before timing, execute the work/accuracy grid: dimensions 6, 96, 6144;
epsilon 0, 1, 10; step sizes .01 and .1; one and four workers; three schedules.
Every result retains the native residual and embedded decisions. At each grid
cell compare all represented candidate stages, output and error vector between
legacy and reuse. A separate scalar raw-U forward-elimination oracle supplies
the target error, required at most 2e-10 absolute for candidate qualification.
This is a one-step target check, distinct from physical truncation error.

Also integrate the Riccati problem at dimensions 6 and 96, epsilon .5,
`[0,.2]`, 33 common physical observation times, `rtol=1e-7`, `atol=1e-11`,
initial/max step .05. Require successful completion, every observed state
identical in bits between old/new, identical accepted/rejected step sequence,
and maximum physical error at most 3e-9 versus
`0.1*exp(a*t)/(1+(.05/a)*(1-exp(a*t)))`.

Only after successful qualification, time legacy/reuse at n=96 and 6144,
epsilon=10, h=.1, with 1, 2 and 4 workers. Use five alternating ABBA/BAAB
blocks, five steps per timed segment. Exclude thread-pool creation, oracle,
serialization and first diagnostic solves. Retain every segment and ratio;
report paired medians and ranges. Each step still includes basis creation,
factorization, root solve, full-space validation and candidate construction.
This short timing is diagnostic; no claim of general wall speed follows from
work counters or the finite-causality theorem. Stop optional timing after this
bounded pass unless a concrete implementation defect requires repair.

Actual RHS/JVP/block solves, cache hits, projected rounds, scheduled/completed
tasks and observed peak concurrency are reported separately. Cache hits must
equal the old-minus-new RHS and solve count on same qualified paths. Reuse can
qualify under a tighter callback cap where the old arm exhausts the cap; the
unlimited-budget parity claim is separate from that behavior.
