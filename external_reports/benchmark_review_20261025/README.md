# VigilODE independent benchmark report

The Korean report is intended for external human readers. It reanalyses published
measurements through 25 October 2026. Source baseline for this report package:
`2a4bf90d1445b67542bdac5030b4c5d10da20445`.

- `VigilODE_Benchmark_Report_KO.pdf`: printable report, embedded Korean font.
- `REPORT_KO.md`: editable report source; eight scientific figures.
- `figures/`: PNG and SVG figures; the whole-solver figures also have PDF versions.
- `BENCHMARK_FINDINGS.json`: machine-readable results and applicability.
- `analysis/`: row selection, source hashes, reanalysis and historical correction.
- `reviews/`: independent mathematical, measurement, code and figure reviews.
- `LOCAL_CODEX_HANDOFF_KO.md`: instructions that preserve the running local campaign.

The report adds no new ODE timing samples. It includes a read-only export tool and
a minimal native comparison validation fix. Historical source data remain intact.

## Reproduce the analysis

Run from the repository root, in a separate checkout, with NumPy and Matplotlib:

```bash
python3 -B external_reports/benchmark_review_20261025/analysis/whole_solver_analysis.py
python3 -B external_reports/benchmark_review_20261025/analysis/reproduce_historical_correction.py
python3 -B external_reports/benchmark_review_20261025/analysis/conditional_analysis.py
```

These commands read the repository's existing raw data and regenerate aggregates
and figures. They do not launch any solver. The compact latest index contains
statuses and hashes; missing local timing values are not inferred from it.

To regenerate the PDF, supply a locally installed `NanumGothic-Regular.ttf`:

```bash
python3 -B external_reports/benchmark_review_20261025/tools/render_report.py \
  --font /absolute/path/to/NanumGothic-Regular.ttf
```

Font provenance and digest are in `analysis/RENDER_IDENTITY.json`. The render tool
needs ReportLab, Matplotlib and Pillow and never downloads dependencies. The PDF
embeds the subset needed for viewing. Full-height figures 5-8 form consecutive plates after the discussion in the PDF;
the Markdown keeps them at the original insertion point.

## Verify code changes

```bash
python3 -B -m unittest discover -s external_reports/benchmark_review_20261025/tools -p 'test_export_local_measurements.py' -v
python3 -B -m unittest discover -s external_reports/benchmark_review_20261025/reviews -p 'test_exporter_adversarial.py' -v
python3 -B -m unittest discover -s external_reports/benchmark_review_20261025/analysis -p 'test_diagnostics.py' -v
python3 -B -m unittest discover -s research/local_frontier_native_diffsol_20261024 -p 'test_compare.py' -v
```

The exporter preserves failures and incomplete attempts; its integrity result is
separate from scientific accuracy. See `tools/README_KO.md`. The comparison fix
rejects nonfinite or malformed final times and state values. Finite valid cases
retain their existing endpoint tolerance and accuracy classification. Apply this
fix only in a newly identified comparison run, after the current run is preserved.

The historical rotation correction is **7 of 140**, not the prose's former zero.
The original numeric summary already retained the seven passing rows. The same
analysis distinguishes native solve time from process wall time. See
`analysis/HISTORICAL_CORRECTION.json` for raw rows, hashes and exact selectors.
