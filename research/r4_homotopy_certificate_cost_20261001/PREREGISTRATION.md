# Preregistration: serial, doubling and blocked certificate cost (R4-HOM-DEV-07)

Written and committed before the run. Results are appended below the line at the end after the run.

## Question

On diagonal quadratic models of dimension n in {1, 2, 4, 8, 16}, the study compares the directed-operation cost of
three certificates for the same candidate and target: serial, full doubling and component-blocked doubling. It also
measures the q1/q2/fallback rates that enter the budget `(7 - p1) ceil(8/P) + 8 pf`.

## Source and command

- Source commit: `96639dbf9b6be7c288028534570674d4decfcfcb`, rustc 1.94.1.
- Command:
  `target/release/rodas5p r4-study --study homotopy-cost --output research/r4_homotopy_certificate_cost_20261001/STUDY.json`

## Design

- Model: `A = diag(-1 - i)`, `q_i = -0.05 (1 + i mod 3)`, `y_i = 1 + 0.1 i`, h = 0.05.
- Target: the native sequential stage target. Candidate: `K_i = h f(y)`.
- Doubling starts at radius 1e-3 with at most 6 attempts.
- Lane rates come from a certified adaptive integration on (0, 0.5) with one thread.

## Gate

**PASS** if, for every n, both the full doubling and the blocked doubling certificate close and give bit-identical
stage bounds. Otherwise **FAIL**: a non-closing path counts as a failure, not as an identity.

The cost columns are reported and not gated. The full path charges formal dense products, `(2L-1)(8n)^3` per radius
attempt; the blocked path counts its own operations. Their ratio is about n^2 by construction, so it is not evidence
of anything measured. The idealized batch budget is disclosed with its assumptions: equal solve cost, with
certificate, RHS, pool and dispatch costs excluded. `SPEEDUP_UNPROVEN` is retained. The matched paired-timing
campaign is not run while timing authority is on hold.

## Prior information

The contract tests showed bit identity on the 12 diagonal R3 fixtures and on one n = 6 problem. This grid has not
been run before.
