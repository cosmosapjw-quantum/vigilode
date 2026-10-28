# Registered short comparisons

The experiment asks whether the integrated adaptive selector improves on the
native evidence policy, and how much it costs relative to an appropriate direct
solver. It is a single-thread experiment. No cloud benchmark or method-by-thread
Cartesian sweep is needed.

## Fixed comparisons and accuracy

| Fixture | Initial step | Relative / absolute tolerance | Full-state endpoint cap | Direct control |
|---|---:|---:|---:|---|
| Rotation, decay, stiff forced cosine | 1e-7 | 1e-7 / 1e-10 | 1e-5 | DOPRI for rotation/decay; RODAS for forced cosine |
| Relaxation, transition at 0.39 and 0.79 | 1e-4 | 1e-7 / 1e-10 | 1e-5 | RODAS with the same provider |
| Case5 persistent transverse stiffness | 1e-4 | 1e-6 / 1e-9 | 1e-4 | Both matrix-free and dense RODAS |
| Rapid gyromotion, tight | 1e-4 | 1e-9 / 1e-12 | 1e-4 | DOPRI |
| Rapid gyromotion, default | 1e-4 | 1e-6 / 1e-9 | 1e-4; report only | DOPRI |

Every solve has an attempt budget of 100,000. All comparisons retain the same
physical variables, initial conditions, endpoint and error cap. No tolerance is
tuned after measurements. Exact solutions cover all fixtures except case5,
whose existing precommitted external endpoint reference and empirical
uncertainty of 1e-12 are reused. Error is the maximum absolute error over every
state component, with reference uncertainty added before testing the cap.
The gyromotion reference contains all six coordinates and momenta.

The 0.39 and 0.79 transitions are new parameter positions within the previously
studied smooth tanh-transition family. They are not evidence of generalization
to an independent model family. The default gyromotion tolerance is retained
as an accuracy diagnostic even when it fails; it cannot support a speed claim.

The primary pair in each fixture is `evidence` versus `adaptive`. Both run in
the same binary with the same implicit provider, tolerance and maximum work.
The requested differences are adaptive evidence reuse, its same-checkpoint
return comparison, and reuse of a winning entry pilot's mean accepted step as
the initial proposal of the next implicit tenure. Neither branch changes the
implicit backend to dense LU. The extra case5 matrix-free/dense direct pair
therefore separates backend cost from selection overhead.

An additional ablation compares `adaptive-unseeded` with `adaptive` on case5,
stiff forced cosine, relaxation and the two transition cases. It changes only
`reuse_entry_pilot_step` from false to true. A winning pilot's accepted state is
still not substituted for the explicit comparator state; only its successful
step scale seeds the next implicit run, whose usual error test remains active.
This measures the new proposal-reuse optimization separately from the cost of
adaptive evidence renewal. The ablation was registered before running this
experiment and before seeing its timing results.

The selector returns eight uniform landing windows and additional accepted
batch endpoints. The direct controls request span endpoints. Thus the direct
comparison measures the cost of the complete selection service, including its
landing requirements; it is not an equal-output-contract kernel comparison.
The separate `rhs-only-availability` diagnostic does use identical eight-window
observations on both sides and compares all native work counters and states.

## Timing protocol

Each arm is calibrated by doubling its repetition count until one batch lasts
at least 25 ms. Calibration durations are retained. Five measured blocks follow;
their orders alternate ABBA and BAAB. The reported time ratio is the median of
the five ratios of within-block means. The plotted uncertainty is the observed
minimum–maximum range of those five paired ratios, not a confidence interval.

Solver construction, workspace allocation, callbacks, rejected steps, every
pilot, transitions and native statistics construction occur inside the clock.
Fixed model/reference construction and extraction or serialization of telemetry
occur outside it. Destruction of prior solve results is timed; destruction of
the last result in each batch is outside the clock for every arm. Each arm is
checked before timing and the last solve of every batch is checked afterward.
The record explicitly identifies this sampled validation coverage.

Timing eligibility requires at least five blocks, every batch at least 20 ms,
early/late median drift no greater than 15%, maximum/minimum block mean no
greater than 1.30 for either arm, deterministic sampled state/work, and the
common endpoint cap. To report a clear measured improvement, every paired
ratio must be below 0.95; a median alone does not suffice. These conservative
rules only govern this short measurement, not a theorem about runtime.

Each cell has a 60-second external timeout; the executable stops starting new
timed batches after 55 seconds. The runner executes one cell at a time. It saves
timeouts and failed outputs, rather than silently dropping them. No timed run
should overlap compilation, tests or another benchmark.

## Work accounting and interpretation

Winning implicit entry pilots are normally not committed as physical states:
the explicit comparator already advanced the trajectory. They still justified
entry and must not be counted as failed selection evidence. The output separates
these winning pilots, losing/incomplete entry pilots, committed integration,
explicit return probes and implicit same-window return comparators. A successful
implicit comparator committed after a losing explicit probe is productive work.
A successful but uncommitted comparator following an explicit win is charged
comparison cost, not a failed implicit entry trial.

Work-counter improvements and timing improvements are different observations.
An unqualified timing result remains visible. Candidate choices are narrowed by
accuracy, measured cost and applicable problem structure; absence of improvement
in these small fixtures does not by itself prove a method universally useless.

## Native evidence reuse and parity

The latest native packet was measured at source
`596dcdfae8b95d24f27b57975d9f4626267b8149` and published on tip
`e8572eac8f6b1e019d8bf3dcb6d8b957fc1d1824`. Its latest comparison already
reduced rotation attempts from 1046 to 892, case5 from 232 to 115 and forced
cosine from 137 to 66. Its timing records showed substantial drift and are not
used as the denominator of this experiment.

The new `evidence` arm is the latest-native policy factory inside the integrated
binary. This wording does not assert cross-commit behavioral identity. Such a
claim requires a separate check of fixture callbacks, configuration, all native
work counters and state, including source identities. In particular, the old
case5/gyromotion suite used 16 windows, 32 pilot attempts and four lifetime
pilots, whereas this new comparison uses the evidence factory's eight windows,
64 pilot attempts and eight lifetime pilots. The case5 dense provider now uses
stack scratch for its tiny directional columns; the older suite allocated its
column buffers. Historical wall times must not be mixed with this measurement.

## Execution

Build the example in the same measurement profile as the audited code. Then,
with all compilation and other timed work stopped:

```sh
python3 research/adapter_integration_20261027/benchmark/run_pairs.py \
  /absolute/path/to/adapter_integration_pilot \
  research/adapter_integration_20261027/benchmark/raw
/absolute/path/to/adapter_integration_pilot rhs-only-availability \
  > research/adapter_integration_20261027/benchmark/AVAILABILITY.json
python3 research/adapter_integration_20261027/benchmark/analyze_pairs.py \
  research/adapter_integration_20261027/benchmark/raw \
  research/adapter_integration_20261027/benchmark/analysis
```

The JSON analysis retains full initial work records, raw per-block timings,
accuracy failures, qualification reasons and source identities. The five plots
are exported as PNG, SVG and PDF for independent review.
