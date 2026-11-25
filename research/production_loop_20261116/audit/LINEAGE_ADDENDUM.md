# Lineage addendum (loop L1, 2026-11-16)

Addendum to `research/production_unification_20261115/lineage.json`. That file is hash-pinned
(sha256 `8c5cf94f...9828b`, `research/production_unification_20261115/CHECKPOINT.json:83`) and is
not edited. Sources: `audit_findings.json` (finding ids below) and the read-only git checks in
`synthesis_spot_checks.log` section C14. Items marked [spot] rest on that log only and were not
reproduced by an independent verifier.

## 1. Two lineages are missing from the lineage record

| | switcher lineage | rei lineage |
|---|---|---|
| head | `933922495045796c99c530b0f1f1efd8fec83ed0` (`origin/research/switcher-v2-20261103`) | `6844c8fe18361f3c2243ee2db95bbf3d68a6ce7b` (local `research/vigilode-rei-peer-native-20261115`; contained in `origin/research/vigilode-rei-igm-photon-fusion-20261101`) |
| merge-base with loop head `2dc9a2c5` | `569ce026` (2026-11-02) | `2d430a7a` (2026-10-29) |
| commits not on the loop head | 52 | 287 |
| ancestor of the loop head | no | no |

- `lineage.json`, `LINEAGE.md` and `HISTORY_RECOVERY.json` contain 0 occurrences of `93392249`,
  `612063db`, `6844c8fe`, `switcher` or `rei`. [spot]
- `git_inventory.json` of the same node does list `origin/research/switcher-v2-20261103`,
  `origin/research/vig-authority-followup-20261105` and the `vigilode-rei-*` refs, each with
  `ancestor_of_current_research_base: false`. The refs were inventoried; no track covers them.
  The 14 tracks cite 2 distinct source commits (`bee5563c`, `3336d682`). [spot]
- `research/vig-authority-followup-20261105` @ `612063db` has 7 commits that are not on the
  switcher head. [spot]
- No reason for the omission is recorded (plan 1.1; not checked further).

## 2. Corrections the findings support

Switcher lineage, to be recorded as a track with state "implemented, unmerged, evidence partly invalid":

1. Code overlap. Only `crates/rodas5p-integrators/src/lib.rs` is modified on both sides of the
   merge-base (CODE plan item, confirmed). The switcher toolchain channel is `stable`; the loop pins `1.94.1`.
2. Protected files. Eight files of the loop's protected list differ from the merge-base on the
   switcher line: `Cargo.toml`, `rust-toolchain.toml`, `crates/vigilode/src/lib.rs`,
   `crates/vigilode/src/switching.rs`, and in `crates/rodas5p-integrators/src/`: `sequential.rs`,
   `adaptive.rs`, `rodas5p_fast.rs`, `rodas5p_matrix_free_fast.rs`. The plan lists seven and omits
   `rodas5p_matrix_free_fast.rs`. Only the file list and line counts were checked, not the content. [spot]
3. Ledger. Rows `L-0205..L-0214` exist only on this line. `L-0205`/`L-0208` execute the solver (raw
   directory untracked). `L-0206` is INVALID through `L-0207`. `L-0209`, `L-0210`, `L-0212`,
   `L-0213`, `L-0214` are typed-in numbers and `L-0211` measures unrelated unit tests: six rows need
   superseding INVALID rows, not four (EVD-01 plan stage and auditor; EVD-02, EVD-03, EVD-04 verifier-confirmed).
   The loop uses ids `L-0300` and above.
4. Measured status of the added modules. `shared_basis.rs` and `coloring.rs` have contract tests
   only; no performance run exists (EVD-01, plan item confirmed). The example programs
   `k1_shared_basis_bench.rs` and `b7_first_batch_bench.rs` appear in no ref (EVD-04).
5. Unified frontend. `unified/router.rs` sends Tier2, Tier3, Tier4 and Protected to the same
   `Rodas5p::solve`; the executed engines are DOPRI5, `Rodas5p::solve` and `SwitchingSolver`
   (COD-05, plan item and auditor; read only, not built). Leads from one auditor: replaced failure
   reasons and dropped counters (COD-06), contractivity used as non-stiffness and an n*n
   allocation per solve (COD-07).
6. Benchmark crate. `crates/vigilode-bench` executes external solvers and keeps raw per-repetition
   data. Its unified report is matched-tolerance on grid-clipped VigilODE arms (DSG-03,
   verifier-confirmed). PDE battery v3 has matched rows for 2 of 9 problems (DSG-05,
   verifier-confirmed). Four of five raw files were built from a dirty tree and one v3 receipt
   misstates the row counts (EVD-09, lead).
7. Process. The switcher gate tool was edited inside gated commits (EVD-05, lead), so its "gate
   passed" record is informational.

Rei lineage, to be recorded as a track with state "unmerged, not audited":

8. Head, merge-base and commit count as in the table. 283 of the 287 commits are dated before
   2026-11-15, the date of `lineage.json`. [spot]
9. The content claims of the plan (N02 triggered FGMRES, N03 stage interval certificate, the
   46,080-cell Peer-W experiment) and the `L-0205..` rows said to exist on that branch were not
   examined by any audit dimension.

Unification lineage (the base of the loop):

10. `research/LEDGER.jsonl` ends at `L-0204` plus the loop baseline `L-0300`; U01-U11 have no row and
    `tools/check-research-node.py` exits 1 for the unification node (EVD-07).

## 3. Plan-stage statements about lineage documents that were not checked

The misattribution of PR135-147 in track L10, tracks H14/H15 pointing at files absent from the head,
and the meaning of NOT_PORTED in the DICTIONARY (plan 1.2) were not examined by any dimension. They
stay open and are not corrections supported by this audit.
