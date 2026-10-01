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

---

## Results (appended after the run at `95cb66e`)

Output: `STUDY.json`. **Verdict: FAIL.**

At n = 8 and n = 16, neither the full doubling certificate nor the blocked one closed its state radius within six
attempts. The radius started at 1e-3 and grew fourfold per attempt. The serial certificate does certify these
candidates.

For n = 1, 2 and 4, both doubling certificates closed with bit-identical stage bounds. Their output bounds are within
1.00008, 1.0006 and 1.004 of the serial bound.

| n | Serial operations | Full doubling (formal dense) | Blocked (counted) |
|---|---|---|---|
| 1 | 425 | 2633 | 3689 |
| 2 | 1010 | 20626 | 9186 |
| 4 | 2660 | 164132 | 21988 |
| 8 | 7880 | not closed | not closed |
| 16 | 26000 | not closed | not closed |

Lane rates of the certified integrations:

| n | Accepted steps | p1 | pf |
|---|---|---|---|
| 1 | 6 | 0.33 | 0.17 |
| 2 | 8 | 0.25 | 0 |
| 4 | 10 | 0.10 | 0.10 |
| 8 | 14 | 0.07 | 0 |
| 16 | 21 | 0 | 0.048 |

Every integration succeeded. The serial certificate is the cheapest in operations at every n. Under the idealized
budget, P = 8 workers give `(7 - p1) + 8 pf` batch units against 8 sequential solves, for example 7.0 + 0.38 = 7.38 at
n = 16. The margin left for certificate, RHS and pool costs is under one solve.

`SPEEDUP_UNPROVEN` is retained. The doubling radius schedule does not close at n >= 8 on this family; the R3 radius
prediction from past steps is not used here. That is the next work item.
