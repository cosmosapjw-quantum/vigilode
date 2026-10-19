# VigilODE: 2026-10-19 two benchmark campaigns

**Status:** historical benchmark publication / scoped evidence. This directory does **not** promote a solver to production, certify continuous-time error, or establish generally admissible wall-clock speedups.

These results are published separately from the numerical implementation on the working branch \`claude/nice-dijkstra-208xq0\`. The publication branch began from \`a21d573980f66fe02891bd216153443b5352f921\`; no numerical source changes are included.

## What is archived

| Campaign | Binary/source under measurement | Verifiable primary result | SHA-256 (CSV) |
|---|---|---|---|
| Run 1, five-problem endpoint / work-precision sweep | \`ccaeaa50beb4580505a36d25ccb919ec3ca38c7f\` | [154 original CSV rows](run1/endpoint_results.csv) | \`8a78b5a9c9b9e37dcc599efc7a348457437f606399d2bf236759b7e02d14909c\` |
| Run 2, new triage accuracy campaign | \`8a8bd409b1d96c7b7421f496d229e119fd3d284e\` | [2,700 original CSV rows](run2/ct_results_2700.csv) | \`03eda7909f39c467d1ddce7990dd2707b94a4dbb9bde5e036e21f1cf9435e2fa\` |

Git blob identities of these two files were independently matched after upload against the original local files: \`01f8b90c0c72eeffe3a568d36f2bae6bc45c0349\` and \`2800aea255360901e8da92ff14c013c1cd772fdf\`, respectively. CSV files preserve CRLF source bytes.

The latest branch head checked during the second campaign was \`a21d573980f66fe02891bd216153443b5352f921\`. It contained follow-up Python harness changes. **Run 2 timing and accuracy numbers are from the \`8a8bd40\` Rust binary**; a fresh \`a21d573\` binary was not independently benchmarked. Later GitHub updates do not retroactively change these identities.

## Key independently inspectable results

- **Run 1:** 154/154 rows \`completed\` and \`deterministic=True\`; maximum endpoint WRMS plus empirical reference spread \`0.9917799399710141\`. The \`brusselator-1d-200\` test has 400 state variables. At eligible endpoint target \`error+spread <= 1e-5\`, its reported medians are base \`409.833040 ms\`, fast \`25.659520 ms\`, and banded-slices \`7.164704 ms\`. These are run-1 single-environment measurements, not cross-commit improvements.
- **Run 2:** 2,700 attempted cells, \`2639 ok / 54 refused / 7 solver_failure\`. Among the 2,639 successful cells, \`2329\` have observed requested-grid \`R_up <= 1\`, \`2580\` have \`R_up <= 10\`, and \`2626\` have \`R_up <= 100\`. The \`fast-ctl-pi\` arm has \`144/144\` successful cells within \`R_up <= 1\` **only on its tested applicable subset**.
- **Run 2 dense-output finding:** on \`prothero-robinson-nl\`, \`uniform101\`, \`rtol=1e-8\`, \`h0_scale=1\`, the requested-grid \`R_up\` values were direct-grid \`6.63271000448e-7\`, dense-off \`1751.990412875\`, dense-enforce \`1.093303066\`. All three completed, demonstrating that solver success alone is not the requested-grid accuracy gate.

Read the separate method/limitation documents: [run1/REPORT_KO.md](run1/REPORT_KO.md) and [run2/REPORT_KO.md](run2/REPORT_KO.md).

## Evidence classification / missing artifacts

- The two CSVs above are **original, byte-preserved observed results**. Aggregates directly calculated from them are independently checkable.
- A prior chat reported 120-process paired old/new timing, 84 allocation cells, 14 Callgrind cells, and Python harness defects for Run 2. **Their complete raw receipts and original detailed reports were unavailable in the current workspace.** These previous findings are summarized with the explicit \`PRIOR_REPORT_ONLY\` qualifier in the Run 2 document, not presented as newly verified data.
- Run-1 and Run-2 PNG plots, original instrument logs, host receipts, independent reference files, the historical detailed Markdown reports, and the approximately 184 MB evidence archive were **not** included in this push. A CSV permits data-table regeneration but not exact reconstruction of missing host, raw profiler, cross-commit timing or reference receipts.
- Wall-clock results were collected in a shared VM and are diagnostic. A reproducible production speed gate requires an admitted host/campaign and same-achieved-error comparison.
- The source \`source_file\` column in Run 1 names original raw JSON filenames, **not files contained in this directory**.

## Quick checks

From the repository root:

\`\`\`bash
sha256sum research/benchmark_results_20261019/run{1/endpoint_results,2/ct_results_2700}.csv
python3 research/benchmark_results_20261019/verify_csv.py
\`\`\`

The verification script uses Python stdlib only, checks original bytes and record counts, and recomputes the scoped published accuracy totals. It does not re-run the numerical solver or claim missing instrument evidence is present.

**Publication discipline:** keep the original raw CSVs and historical failure statuses unchanged. Any later improved benchmarks should use a new dated directory and distinct measurement commit.
