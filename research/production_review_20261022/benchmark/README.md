# Benchmark audit and figure reproduction

Run `python3 research/production_review_20261022/benchmark/build_figures.py` from the repository root. This reads saved records and produces five PNG/SVG figure pairs and CSV tables; it runs no solver. Python 3, NumPy and Matplotlib are required. The earlier heat campaign record is reused with its separate source identity.

`AGGREGATE_AUDIT.json` records 44 imported-file hash checks, 233 matched-rung selections and 51 paired-time arithmetic checks across attempts 2 and 3. All agree with the published records. These checks do not re-evaluate trajectory errors: the latest imported subset omits 1,098 / 1,684 referenced stdout paths for those attempts. Its locally retained tarball hashes identify the missing source data but cannot substitute for those data in an external review.

`benchmark_section.md` is the formal Korean external-review section; `work_precision.csv`, `matched_costs.csv`, `paired_ratios.csv` and `accuracy_at_1e-8.csv` expose the plot data. All latest accuracy numbers are labelled recorded observable errors, not independently recomputed state errors. Paired ranges are descriptive minimum-maximum ranges, not confidence intervals.

The historical stencil figure selects 480 confirmation blocks from 16 parallel single-core streams. Nine further single-core confirmation blocks are recorded in a separate phase, making 489 in total. They are not pooled into that figure. The 1,046 trajectory count includes both warm-up and confirmation runs of both arms.

The evaluator repair is documented in `METRIC_DOMAIN_AUDIT.json`. Run `python3 research/production_review_20261022/benchmark/test_metric_reference_domain.py` for the three focused regressions. The parent harness also needs its existing SciPy, NumPy and mpmath reference dependencies. `metric_before.log` preserves the two failing cases; `metric_after.log` records all three passing after repair. Saved campaign conclusions are unchanged.
