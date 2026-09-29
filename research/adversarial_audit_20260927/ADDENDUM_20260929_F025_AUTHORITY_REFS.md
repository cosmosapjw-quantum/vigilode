# Addendum, 2026-09-29: F-025 publication check and authority-reference gate

This addendum is a new file. The sealed audit ledger and report are unchanged.

## Stage-certificate bytes are published

F-025 said `crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs` and
the associated proofs existed only in a local dirty worktree. On 2026-09-29 the claim
no longer holds. These commands were run in a clone with all `origin/*` branches
fetched:

```text
$ git cat-file -e e1c3a5f:crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs && echo EXISTS
EXISTS
$ git rev-parse e1c3a5f
e1c3a5f14df7973ede74d77b9efc95be008f95de
$ git branch -r --contains e1c3a5f
  origin/audit/adversarial-audit-20260927
  origin/claude/jolly-wozniak-7wl15h-integration
  origin/claude/jolly-wozniak-7wl15h-wu1-gcrodr-state ... wu9-stage-certificate   (12 branches in total)
$ git show e1c3a5f:crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs | sha256sum
4b3e6c991bfb955189649e017485ce49618f4870976786d164adf645bfea65e2  -
```

The module is 22,022 bytes. It was first added in commit
`5ca1365f84f402b3dcb8fae1951946b4311fdb88` ("research: publish stage-certificate
worktree bytes"). That commit is contained in
`origin/research/audit2-stage-certificate-worktree-20260831`, and its blob has the
same SHA-256. The formal sources are also present at e1c3a5f:

| Path at `e1c3a5f14df7973ede74d77b9efc95be008f95de` | SHA-256 |
|---|---|
| `crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs` | `4b3e6c991bfb955189649e017485ce49618f4870976786d164adf645bfea65e2` |
| `research/audit2_stage_certificate_telemetry_20260831/formal/lean/StageCertificate.lean` | `a0e1d45b44ab0a9ca92c4a38e44afa32526efacd9d17460d74f608ed6d97fd66` |
| `research/audit2_stage_certificate_telemetry_20260831/formal/rocq/StageCertificate.v` | `e0dd47d26d71e030e1fa6794c64380f9aa58657373316f83d783fae77189cea6` |

These checks establish publication and byte identity only. They do not show whether
the predecessor P1/P2 defects that F-025 lists are still present. That question stays
with the stage-certificate repair node and is not decided here.

## Gate for new citations

`tools/check-authority-refs.py` enforces the second half of F-025's proposed fix. It
extracts every commit SHA that `research/` and `docs/` Markdown, JSON, JSONL or TOML cite
as authority. It fails when the object is missing or when no remote branch contains it.
The extraction rule is documented in the script and in `docs/RESEARCH_LEDGER.md`.

On the tree at e1c3a5f the gate finds 83 distinct cited SHAs:

- 34 are commits reachable from a remote branch.
- 23 are tree objects, which the gate skips.
- 26 are absent from the object database, so none of them can be pushed from this
  clone:
  - 16 are pre-repository import history.
  - 3 are local-only or CI merge-ref commits.
  - 7 are third-party upstream revisions.

Each of the 26 is listed with its reason in `tools/authority_refs_allowlist.txt`, so the
gate passes today and fails on any new unreachable citation. This change adds
`research/LEDGER.jsonl` and this addendum, which cite five more commits. All five are
reachable, so the total becomes 88 distinct SHAs: 39 reachable, 23 trees and 26
allowlisted.
