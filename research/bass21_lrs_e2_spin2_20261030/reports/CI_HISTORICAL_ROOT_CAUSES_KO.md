# Global CI inherited failure diagnosis, not an exemption receipt

CI source: VigilODE PR133 'Research ledger and authority references' GitHub Actions run `41030332372`, job `123978280700`, full logs. `check-research-node` stage passed after ledger `L-0195`. Next stage `check-authority-refs.py` **failed 330 historical citations**:

- `missing` (257): pre-repository/local-only source SHA/trees not found on current remote branch and historical `production_reaudit_20261021` absent source tree. Some preimport raw history resides only in named source bundles; cannot regenerate authentic commit/tree identity from a citation string.
- `repository-unavailable` (54): references to SciML/NumPy/SciPy/upstream require exact external local checkout bound by `--repository owner/name=PATH`. Without it, checker correctly refuses to equate remote SHA with current vigilode repository.
- `invalid-repository` (19): data contain full SHA with absent/invalid owner/name, e.g. `research/bassrei_p02_fixed_grid_20261021/results.json:7` and `prefix02_preconditioned_20261029/SOURCE_SURVEY.json`.

**Root classification:** historical provenance defect, not BASS21 current calculation failure. Existing strict all-source gate is deliberately preserved and retains red. Fix requires per-file source-owner metadata restoration with provenance from actual original files and upstream checkouts, and source/published object reconstruction where possible. It is not authorized to fabricate, general-whitelist or downscope this global gate. Separate `bass21-e2-spin2-scoped.yml` checks current prereg/ledger, math/CSV plus manifest without claiming global green.
