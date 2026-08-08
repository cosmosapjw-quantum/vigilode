# Decision log

- Use analytic references only in v0.10 fixed-step replay.
- Use exact-landing output grids; no interpolation in this cycle.
- Use reference-only WRMS weights.
- Keep endpoint, max-grid, and RMS-grid metrics.
- Keep one Pareto front per cost metric; no composite score.
- Do not alter solver algorithms in this cycle.
