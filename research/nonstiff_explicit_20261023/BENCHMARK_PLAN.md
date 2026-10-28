# Local nonstiff solver comparison

The source is pinned before execution. NCP, SSH, package installation and resource creation are not part of this campaign.

The bounded campaign contains ten analytic families: diagonal decay, driven nonautonomous decay, logistic growth, harmonic oscillation, rapid oscillation, damped oscillation, mixed state scales, nonnormal transient growth, a 4096-component vector and a stiff negative control. Each runs at rtol 1e-4, 1e-6 and 1e-8, with atol=1e-3 rtol. The state, initial data, five output times and actual output error are identical comparison targets. The new public `Dopri5::solve_observed` is compared with the protected dense-fast RODAS5P driver for small systems and the existing matrix-free JVP driver for the large vector. This avoids a dense n-cubed baseline for the large system. All cases have an analytic reference; complete states and reference arrays are published.

The main comparison requires the worst observed component error divided by `max(1, |initial component|, |exact component at that time|)` to be <=1e-6 or <=1e-8. These scales do not depend on solver tolerances. Absolute error is also reported. Only successful configurations meeting this requirement enter cost ratios. A successful integrator return alone does not establish the requested global error.

The runner performs one warmup and three timed rounds. Each round randomizes case/tolerance group order and solver order within each group with seed20261023. Each solve constructs its own problem/workspace inside the timer; reference calculation and JSON serialization occur after the timer. Native compilation and process launch are excluded from solver seconds and recorded separately through process receipts. No run is silently dropped. The default campaign budget is900seconds and each subprocess has a45second timeout. Budget-exhausted configurations remain `NOT_RUN_BUDGET`.

With `--external`, installed SciPy's wrapper of Fortran VODE/DVODE is an allowed comparator: Adams for nonstiff families and BDF for stiff controls. Python/NumPy callback overhead is included, and VODE target-time output may use internal interpolation. Neither aspect is presented as an identical arithmetic implementation. No scipy RK45/Radau/LSODA method substitutes for the requested comparator families. Diffsol and Julia are run only when the caller supplies an executable adapter; otherwise their explicit status is `NOT_RUN`.

## Re-run

```sh
cargo build --locked --offline --profile measurement -p vigilode --example nonstiff_bench
python research/nonstiff_explicit_20261023/run_bench.py \
  --binary target/measurement/examples/nonstiff_bench \
  --out /tmp/nonstiff-benchmark-new --source-commit "$(git rev-parse HEAD)" \
  --external --budget-seconds 900
python research/nonstiff_explicit_20261023/analyze_bench.py /tmp/nonstiff-benchmark-new
```

Set `CARGO_TARGET_DIR` and `--binary` consistently if using a separate build directory. The output directory must not exist. Use `--full --budget-seconds 7200` for the expanded local campaign: four tolerance levels, a moderate-stiff family and an additional16384-component vector. `--cpu-affinity 0,1` is an optional explicit host setting; choose actual allowed CPUs from the host. Without it the runner records the inherited affinity and makes no claim of isolated timing.

External diffsol/Julia adapters receive one JSON object on stdin with `case, dimension, rtol, atol, source_commit, max_steps`. They return the `vigilode.nonstiff-benchmark-run.v1` JSON fields demonstrated by `vode_adapter.py`, including actual times, all states, analytic references, elapsed solver seconds, method/version identity and honest work counters. The analysis independently re-evaluates references and errors before admitting a row. Missing fields or inconsistent reference arrays invalidate that record. Adapter source/version/build/JIT receipts must accompany any subsequent external performance claim.

`RAW_OUTPUTS.tar.gz` preserves all numbered stdout/stderr bytes; `RAW_MANIFEST.json` hashes every entry. Extract it in the campaign directory before re-running analysis. The analysis does not rerun integration. Generated min–max bars and ratio envelopes are observed ranges, not confidence intervals.

## Independent-trajectory supplement

`nonstiff_ensemble_bench` runs16independent4096-component decays with a reused local pool of1or2workers, at rtol1e-6or1e-8. `run_ensemble_bench.py` performs one warmup and three shuffled timed rounds for these four settings, with a180second default total budget and45seconds per process. Pool construction is separately timed before the integration timer. Every member's setup/workspace and solve are included. The harness independently re-scores both saved endpoints, verifies state identity across worker counts, and archives all raw outputs. This measures ensemble throughput, not parallel Runge–Kutta stages.

```sh
cargo build --locked --offline --profile measurement -p vigilode --example nonstiff_ensemble_bench
python research/nonstiff_explicit_20261023/run_ensemble_bench.py \
  --binary target/measurement/examples/nonstiff_ensemble_bench \
  --out /tmp/nonstiff-ensemble-new --source-commit "$(git rev-parse HEAD)" \
  --budget-seconds 180
```
