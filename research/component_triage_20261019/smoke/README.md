# Smoke run (`ct-20261019-smoke`): pipeline check, not a decision

This directory holds the smoke run of the triage harness. It exists to show
that every instrument works end to end and to catch harness defects before the
main campaign. Its labels are not component decisions.

- Evidence is partial:
  - G tier only, rtol 1e-4/1e-6/1e-8, 101 outputs.
  - Callgrind on four problems (robertson, hires, van der Pol, nonlinear Prothero-Robinson).
  - References on the 101-point grid only, no P tier, no external arms.
  - Most comparisons are therefore excluded, and most labels are provisional or fall back to the NO_EVIDENCE rule.
- Instrument controls C1-C4 (`controls/`, `PROFILE.jsonl`) reproduced their prior results.
- Defects found and fixed before the main campaign are listed as amendments 1-7 in `../PREREGISTRATION.md`.

| File | Content |
|---|---|
| `RUNS.jsonl.gz` | 6,201 rows (gzip -n; the 156 MB original exceeds the push limit) |
| `PROBES.jsonl` | 664 fail-closed probe rows, all passing on the fixed binary |
| `PROFILE.jsonl` | 702 callgrind cells |
| `ALLOCATIONS.jsonl` | raw allocation windows of the profiled cells |
| `REFERENCES.json` | 13 independent references |
| `RESULTS.json.gz`, `CLASSIFICATION.json`, `REPORT_KO.md` | outputs of `triage_check.py` and `triage_report.py` (report verified with `--check`) |
