# V14 screen: spatial Schwarz iteration on the whole eight-stage chain (Python)

Loop L1 node V14, executed 2026-11-16. Files: `RESULTS.json` (completed run 1, written by the script
revision saved as `logs/final_run1_screen.py.txt`, hash in `logs/final_run1_screen_py.sha256`), `screen.py`
(a later revision: the agent edited it after run 2 and the run of that revision was interrupted), `logs/final_run1_table.md` (table generated from that run),
`logs/determinism_check_run1_vs_run2.txt` (a second full run reproduced step histories, sweep counts and
verdict tables), `logs/final_run1.log`, `logs/final_run2.log`.

The agent that ran the screen was stopped by a usage limit while writing this file; a third run it had
started left a partial `RESULTS.json` (status `running`), which was replaced by the completed run-1 output.
This README was written by the orchestrator from the result files; it contains no number that is not in them.

## What was run

RODAS5P in U-form with the exact tableau, 2D heat, Brusselator and Gray-Scott at N = 64 (and N = 128 for
heat and Gray-Scott), accepted steps of an embedded-error controller at tolerance 1e-4 and 1e-6 (a stand-in
for, not a copy of, the protected Rust controller), 20 sampled steps per case. Restricted additive Schwarz on
the full eight-stage system with P x P subdomains (P = 2, 4), overlap m*ell cells (m = 1, 2, 4,
ell = sqrt(gamma*h*D)/dx), Jacobi update, halo guesses g0 (zero increments) and g1 (previous step's stage
increments scaled by the step ratio).

## Verdict against the frozen criteria

PASS needs median sweeps <= 2 at overlap <= 4*ell with g1, endpoint difference <= 0.1 tolerance units and
median halo <= H/8; KILL is median sweeps > 4. Over the 60 g1 configurations: PASS 17, FAIL 31, KILL 12
(strict integer-halo reading: PASS 11, FAIL 37, KILL 12).

- Gray-Scott (diffusion number h*D/dx^2 about 2 to 5, ell about 0.6 to 1 cell): passes with a 2-cell halo in
  2 sweeps for most configurations.
- Heat and Brusselator (diffusion number about 50 to 900, ell about 3 to 14 cells): no configuration passes.
  One-ell overlap is killed (5 to 9 sweeps); two-ell overlap needs 3 to 4 sweeps; four-ell overlap reaches 2
  to 3 sweeps only with a halo of 0.4 to 1.8 subdomain widths.
- The predictor g1 changes the sweep count by at most one.

Reading: the iteration is admissible only where the diffusion length per step is about one cell. In that
regime the per-stage systems are also easy for a per-stage parallel solve, which is the honest baseline. No
native prototype is started in this loop; the result is kept as a scoped screen.

## Limits

Python model, not the Rust drivers; no cost or wall-time measurement; no advection; subdomain counts up to
4 x 4; the controller is a stand-in. Wall times in the files are diagnostic only.
