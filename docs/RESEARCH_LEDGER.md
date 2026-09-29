# Research ledger and research-node rule

This rule implements adversarial-audit finding F-024
(`research/adversarial_audit_20260927/`). It replaces per-node receipt families with a
single append-only ledger.

## A research node

A research node is a new top-level directory `research/<node>/`. It holds exactly these
things:

1. **`PREREGISTRATION.md`**, written and committed before the run. It states the
   question, the command, the inputs, the pass/kill gate and the holdout. Results are
   appended below the pre-registration after the run. They are never written into it
   beforehand.
2. **Raw outputs.** At least one results file (`.json`, `.jsonl`, `.csv`, `.tsv` or
   `.txt`) must carry numeric rows.
3. **Ledger rows.** At least one row in `research/LEDGER.jsonl` must have an
   `outputs_sha256` that covers a numeric results file in the node.

Contracts, receipts, claim ledgers and audit reports are written only for a result that is
being **admitted**. Formal-proof and computer-algebra obligations, such as
Lean/Rocq/Wolfram/Sage/Singular, are deferred until a numerical claim has survived a
holdout.

## `research/LEDGER.jsonl`

The ledger holds one JSON object per line, and every row uses the same schema:

| Field | Meaning |
|---|---|
| `id` | `L-NNNN`, unique, increasing |
| `date` | `YYYY-MM-DD` date of the result. Seed rows use the date of `commit` |
| `commit` | Full 40-hex SHA of the source commit at which `command` reproduces the outputs. It must be reachable from a remote branch |
| `command` | The exact command. Placeholders such as `MEASUREMENT_BINARY` name build products. `UNRECORDED: ...` is allowed only with verdict `INCONCLUSIVE` or `INVALID` |
| `profile` | Run, corpus or build profile |
| `inputs_sha256` | `{repo-relative path: sha256}` of the inputs, as they are in the tree of `commit` |
| `outputs_sha256` | `{repo-relative path: sha256}` of the outputs, at least one file. The files must still match in the current tree |
| `claim` | One sentence on one line, at most 600 characters |
| `verdict` | One of the four verdicts described below |
| `supersedes` | `null`, or the `id` of an earlier row that this row replaces |

The four verdicts are:

- **`PASS`**: the pre-registered gate was met.
- **`FAIL`**: the gate was not met. The row is a preserved negative result.
- **`INCONCLUSIVE`**: the row cannot establish pass or fail. For example, the command or
  commit cannot be cited, or the run is noise-dominated.
- **`INVALID`**: the run or claim is void, for example because of a protocol breach or
  contaminated inputs.

Rows are never edited or deleted. A correction, retraction or tombstone is a new row whose
`supersedes` names the old one. Failed and invalid rows stay visible without needing a new
schema.

The existing 83 schema and receipt families are **frozen as they are and are not
migrated**. Directories that already existed when `research/LEDGER.jsonl` was introduced are
exempt from the node rule. The seed rows L-0001 to L-0004 cover v3.5, v3.6, v3.7 and
scientific-validity v2. Each row's claim says how it was sourced. L-0002 and L-0003 were
re-run on 2026-09-29. L-0001 is `INCONCLUSIVE` because its run commit and command are not
in this repository.

## Checks (CI: `.github/workflows/research-process.yml`)

### `tools/check-research-node.py [--base REF]`

- **Schema:** every ledger row validates. Every `outputs_sha256` entry must match the file
  bytes.
- **Append-only** (with `--base`): the ledger lines present at `merge-base(REF, HEAD)` must
  be an unchanged prefix of the current ledger. Removing, editing or reordering a row fails.
- **New nodes** (with `--base`): each new top-level `research/<dir>` needs
  `PREREGISTRATION.md`, a numeric results file and a ledger row that covers one.
- **Stall rule:** nodes are ordered by their first ledger row, using `date` and then line
  order. A node has a numeric result when a row that no later row supersedes gives it
  `PASS` or `FAIL` on a numeric results file. After two consecutive nodes without a numeric
  result, a further node without one fails the check. The stall ends when a new or earlier
  node gets a `PASS`/`FAIL` numeric row, so the fix is to measure, not to open another
  process node.
