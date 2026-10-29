# REVTeX report

`rvj_production.tex` is the main REVTeX 4.2 source and includes
`theory_sections.tex`. Figures are generated only from the frozen analysis
JSON by `plot_results.py`; this is posthoc presentation, not a new solver run.

From this directory, in an environment with Python/matplotlib and the TeX
Live `revtex4-2` package:

```bash
python3 plot_results.py
pdflatex -interaction=nonstopmode -halt-on-error rvj_production.tex
pdflatex -interaction=nonstopmode -halt-on-error rvj_production.tex
```

The delivered PDF was built with an available local REVTeX 4.2 support
directory via `TEXINPUTS`; the third-party class is not vendored into this
node. The build receipt records the actual class-file hashes, commands and
render checks. Installing the normal REVTeX package is the portable option.

`REPORT_KO.md` in the parent directory provides the Korean companion report.
`LOCAL_CODEX_HANDOFF_KO.md` supplies the complete independent local validation
prompt. Rebuilding the PDF does not require native Rust tests or benchmarks.
