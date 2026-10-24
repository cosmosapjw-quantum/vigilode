# N5: genuine nonlinear callback cost and stage parallelism

The completed experiment demonstrates a useful speedup from parallel stage
batches on one controlled expensive callback. It does not make Picard or
residual-first the fastest complete solver. The existing forward causal and
dense routes remain stronger comparators, so automatic parallel selection is
not enabled by these measurements.

## Model, exact trajectory and counted work

There are 64 phase coordinates plus an autonomous clock t=y_0, y_0'=1.
Define omega_i=0.5+0.025(i mod 11), nu_i=2+(i mod 5), and

    q_i(t)=0.15(i mod 7)+omega_i t+0.05 sin(nu_i t),
    k_ij=(10/64)/(1+(|i-j|/8)^2).

The physical RHS is

    f_i=q_i'(t)+sum_{j!=i} k_ij [sin(y_j-y_i)-sin(q_j(t)-q_i(t))].

This is a nonseparable weighted all-pair phase interaction with a manufactured
forcing. Both trigonometric sums are evaluated within every timed RHS call;
there is no synthetic burn loop. This is not a claim about an unforced
physical Kuramoto simulation. The forcing deliberately constructs a known,
curved trajectory for an independent output error check.

Substitution y_i=q_i(t) cancels both interaction terms, proving y'=q' with
the prescribed initial value. Smoothness gives local uniqueness. The supplied
analytical JVP is

    (Jv)_i=q_i'' v_0+sum_{j!=i} k_ij [cos(y_j-y_i)(v_j-v_i)
                          -cos(q_j-q_i)(q_j'-q_i')v_0].

The test checks the RHS on the exact path, the JVP against centered finite
differences off the path, and the dense Jacobian against that same JVP at four
clock values. The exact reference is evaluated outside each solve timer;
the forcing needed by the actual vector field stays inside the callback.
This distinguishes a legitimate verification oracle from hidden computational
work supplied to the solver.

## Timing and comparison

One and two worker configurations used identical physical output landings,
tolerances, callback providers, solver settings and the 1e-5 absolute error cap.
Each method had one warmup and three cyclic timed repetitions in its process.
All 48 timed trajectories succeeded, with maximum absolute error
1.6723766816e-10. Every route had identical work counters between one and two
workers. Pool construction was reported outside solve timing; creation of
the two-worker pool took 88.453 microseconds in this process. The full timer
includes setup, callback work, attempts and solver receipt construction.

| Route | One worker (ms) | Two workers (ms) | Same-route speedup | RHS / JVP calls |
|---|---:|---:|---:|---:|
| Fixed forward causal | 117.575 | 89.769 | 1.310 | 543 / 402 |
| Fixed Picard | 177.385 | 122.383 | 1.449 | 1026 / 402 |
| Fixed residual-first | 357.029 | 204.974 | 1.742 | 858 / 1786 |
| Protected raw-U | 1668.230 | 1655.415 | 1.008 | 288 / 9711 |
| Protected dense | 21.659 | 21.354 | 1.014 | 144 / 0 |

The Picard parallel gain does not offset its extra RHS evaluations relative
to forward causal: even its two-worker time is 4.1% above single-worker
forward causal. The dense comparison is more decisive: serial dense is about
5.65 times faster than two-worker Picard on this 65-state fixture. Residual-first
reduces same-route time but spends many more JVP calls. These are useful
bottleneck observations, not grounds to silently prune the faster comparator.

The callback has actual O(n^2) weighted pair work, while its dense solve has
O(n^3) factorization work. These asymptotic terms alone do not locate the
crossover: setup, number of calls, dimension and parallel overhead matter.
There is no guaranteed parallel speedup theorem without assumptions about
work/span, scheduling overhead and algorithmic iteration counts. The present
measurements show a favorable point for parallel stage batches and an
unfavorable point for overall replacement of dense/forward routes.

## Use in the local bottleneck campaign

The existing `reconsideration_bench` now accepts
`--case meanfield-manufactured --root-variants --threads T`, with
T in 1,2,4,8,12. The local campaign should retain identical error semantics,
repeat in separate processes, report pool creation and total solve time,
and compare both the same route at one worker and the strongest legal fixed
route. A faster stage batch alone is insufficient for an adaptive dispatch
decision. No nested model-level parallelism is used here.

The remaining local sweep is prepared, not run here. This experiment exercises
the already-implemented stage-batch execution and root schedules; it does not
establish that homotopy subdivisions reduce total work or that accepted time
steps can generally be vectorized. All native callback derivatives are
analytical JVPs, so this is also not a fully derivative-free speed result.

`N5_PARALLEL_CALLBACKS.json` preserves every repeat, source/binary identity,
pool startup, callback counts and comparator ratios. Raw receipts and streams
are `evidence/runs/n5-final-meanfield-{1,2}.*`. The common final binary hash is
`549583201854c7db78c7fb12a85bcbefb7438412b7cd6be8b0eac8f6d1bb8710`.
Three within-process samples are descriptive; process order and host scheduling
remain possible influences. The decision is to retain specialized stage
parallelism for measured expensive callbacks, without changing production
defaults or claiming universal elapsed-time gains.

![Local cost guard and nonlinear callback timing](n3_n5_local_discrimination.png)
