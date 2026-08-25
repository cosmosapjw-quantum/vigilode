# CODEX LAUNCHER

Resume VigilODE PR #18 from the durable two-arm receipt handoff.

Repository:

```text
~/vigilode
```

Fetch:

```text
main
research/a1-inner-tolerance-parity
handoff/a1-two-arm-authority-receipt-20260825
```

Create a detached read-only worktree for the handoff branch. Read its `AGENTS.md`, follow the mandatory read order, run `acceptance/test_handoff_contract.py`, and then execute `IMPLEMENTER_PROMPT.md` completely.

Important:

- The compile/trace baseline is already GREEN at `7952bf96bfd9fb604e87bce41bd9b918cc9b93f4`.
- Reuse PR #18. Do not open another PR.
- Do not merge the handoff branch.
- Implement only `A1-TWO-ARM-AUTHORITY-RECEIPT`.
- Leave PR #18 draft and unmerged.
