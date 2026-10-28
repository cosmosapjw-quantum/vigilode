# Divergent-branch integration: missing RSS telemetry

The imported component harness had three fake-process contract failures because it assumed every host could return positive peak RSS. This environment has no `/usr/bin/time`, and a short child may terminate before the `/proc` sampling thread observes it. The old code represented this missing observation as `0` bytes. This is an environment/telemetry distinction, not evidence of zero memory use or a solver failure.

The bridge now emits `peak_rss_bytes: null`, `peak_live_rss_bytes: null` where no positive corresponding observation exists, and `rss_status: UNAVAILABLE` with an explicit source explanation. Positive `/proc` or GNU-time observations remain positive and retain their measured values. The receipt says an observed maximum is not a complete process-tree peak; no sampled observation is promoted to a rigorous upper bound.

The run-owned timing output is cleared before launch, so a missing time utility or failed launcher cannot read an earlier run's RSS report. The runner and component-contract receipts only attach a GNU-time report hash when that report belongs to the current launch. Metadata is passed through the screening, component-contract and confirmation paths.

The confirmation rule still returns `HOLD_UNKNOWN` with `UNKNOWN_PEAK_RSS` when either arm lacks a positive RSS observation. Unknown memory cannot pass the confirmation memory requirement or be substituted for a screening memory ceiling. The fake-process tests now verify either a positive, explicitly observed value or a null, explicitly unavailable value. Dedicated hostile tests force missing instrumentation, an entirely missed child sample and a stale timing file; they also verify that actual positive values remain unchanged and that missing RSS prevents paired inference.

Actual focused execution:

```sh
PYTHONPATH=/workspace/scratch/0dc1e721cf82/runtime/python-audit:tools python3 -m pytest -q tools/tests/test_component_benchmark_hpc.py tools/tests/test_component_benchmark_confirmation.py tools/tests/test_component_benchmark_component_contracts.py
```

**28 passed in 1.52 seconds.** Logs are `evidence/branch_integration/hpc_portability_focused.log` and `.exit`. Earlier failed logs remain in `component_contracts.log`. This execution uses fake adapters and resource-contract fixtures; it reruns no scientific benchmark or published numerical result. The full component-suite execution is owned and recorded separately by the branch-integration workstream.
