# One focused timing confirmation

Registered after the first 27.57-second timing pass and before any confirmation sample.

The first pass passed all physical accuracy and equality checks, but every active cohort's five paired wall-time ranges crossed unity. Adapter n=1 did not activate the new stop and received no timing. The initially attractive dense n=192 median (1.252) therefore cannot be claimed as a reliable 25% speedup.

One complementary measurement pass is permitted to resolve this concrete shared-host noise risk, within the original 300-second scientific timing budget. Selected cells are LU n=32 (allocation-dominated end), dense n=65 (first Faer range), dense n=192 (check the largest initial apparent gain), and adapter n=16 (largest preregistered work reduction). No new physical model, tolerance, success cap or production policy is selected after observing results.

Each cell uses five alternating ABBA/BAAB blocks and fixed repetitions calibrated to approximately one second per baseline batch. CPU2, single process, fixed one-thread environment, no concurrent compilation. The report keeps both passes, including regressions. Alongside the internal timed wall interval, Python RUSAGE_CHILDREN records the complete process's user+system CPU consumption. That CPU value includes command startup/setup, diagnostic warmup and JSON output. In the adapter fixed-arm harness it includes two initial diagnostic solves. It is an amortized command CPU metric and must not be relabeled as the exact internal integration-only CPU time.

Only the adapter benchmark example gained an optional fixed-arm/repetition CLI; all production code is unchanged. Its bounded rebuild succeeded in 2.03 seconds. The confirmation is capped at 210 seconds of wall time; combined with the first pass this remains below 300 seconds. No third timing pass is scheduled.
