# Pilot material for the 2026-10-08 algorithmic-directions cycle

This directory holds the exploratory evidence behind
[`../ALGORITHMIC_DIRECTIONS.md`](../ALGORITHMIC_DIRECTIONS.md).

It is **not** a research node:
- there is no preregistration and no ledger row;
- no number here is authority for a claim;
- the material exists so that future preregistered nodes can take their predictions, gates and replica designs from it.

## Contents

| Directory | What it is |
|---|---|
| `phase1_maps/` | Eight phase-1 reports. Five read-only code and evidence maps (`rodas-core`, `krylov`, `expo-cert`, `evidence`, `harness`) and three literature reviews (`lit-rok-w`, `lit-stab-ctrl`, `lit-expo-precond`). The literature reviewers could read only GitHub-hosted primary sources in full; journal sites were blocked, so most literature claims are tagged as abstract or snippet level in the reports themselves. |
| `phase2_candidates/` | `merged_candidates.json`: the 16 merged hypotheses with mechanism, predictions, probes and evidence. `adversarial_verification.json`: the five adversarial reviews and the judge's dispositions, combined-stack estimate and ranked program. |
| `phase2_code/` | The phase-2 matrix-free U-form replica (`mfrep.py`, `common.py`, `probs.py`) and the transfer-constant table (`tau_table.py`). |
| `phase3_reports/` | One report per probe: `target` (A1, coupled stage target), `corpus` (A2, corpus v2 transcription), `ctrl` (B5, controllers), `pclag` (B4, 2-D preconditioning), `stack` (B1, integrated stack), `reg` (B2, ROCK4 hand-off), `expo` (B3, exponential at its own step), `geamp` (B6, global-error amplification detector), `critic` (completeness and stress tests). |
| `phase3_code/<probe>/` | Each probe's code and summary tables (`.py`, `.txt`, `.md`). Raw run files (`.jsonl`), references (`.npy`, `.npz`) and logs are not included. |

## Reproducing

- The scripts were run from a session scratch layout, so many contain absolute `/tmp/claude-0/...` paths and
  import siblings from that layout. Adjust the paths before rerunning.
- They need numpy, scipy (with SuperLU) and, for some checks, sympy and mpmath.
- `phase3_code/corpus/corpus_v2.py` is self-contained: `python3 -I corpus_v2.py` runs its self-test.
- `phase3_code/expo/kiops_unit_checks.py` was named `test_kiops.py` in the scratch layout. It was renamed so that no test
  runner collects it.
- The holdout families in `corpus_v2.py` are transcribed as definitions only and refuse to build without
  `allow_holdout=True`. No probe integrated a holdout (`docs/HOLDOUT_HYGIENE.md`).
