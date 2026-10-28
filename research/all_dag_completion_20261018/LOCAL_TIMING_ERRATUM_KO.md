# LOCAL-04 timing interpretation erratum — 2026-10-18

This note corrects the direction sentence in `LOCAL_REPORT_KO.md`; the raw campaign, `local_timing_01/REPORT.md`, `VALIDATED.json`, and all measurements are unchanged.

The table reports **dense / structured** paired median timing ratio. Therefore a value below 1 means dense is faster; a value above 1 means structured is faster. At `rtol=1e-8`, the observed arm medians were:

- 8 modes: dense 84,901 ns; structured 191,383 ns; ratio 0.43693. Dense was faster.
- 32 modes: dense 949,563 ns; structured 600,991 ns; ratio 1.55831. Structured was faster.
- 128 modes: use the same direction convention; paired ratio 8.91756 indicates structured was faster in this campaign.

Thus the original report's prose reversed the direction at all sizes. The size-dependent diagnostic crossover remains, and matched-accuracy speed promotion remains **HOLD**. Timings are instrumented; allocator counts in full arms are nonzero. This erratum changes no source, tolerance, model, output, or measurement.

Source for these statements: `local_timing_01/REPORT.md` and `local_timing_01/VALIDATED.json`, retained unchanged. This is a reporting correction, not a new benchmark run.
