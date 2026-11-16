# SDD ledger — plan: /home/cosmosapjw/.claude/plans/sleepy-drifting-sphinx.md (copy: research/production_loop_20261116/PLAN_KO.md)

Loop L1, branch research/production-loop-20261116, base 074a5699. One line per event; rulings carry what they cost if wrong.

- W0: worktree ~/worktrees/vigilode-prod-loop-20261116 created from 074a5699; validate_program.py and the 2026-11-15 check_checkpoint.py pass.
- W0: baseline (1.94.1, release): facade+core+krylov 789 passed / 2 failed / 21 ignored; peer integrators 218 passed / 0 failed / 3 ignored. The 2 failures are sp02_workspace_charge assertions on a process-global live-byte counter: 3/3 runs pass with --test-threads=1, 1 of 3 default-thread runs fails.
- Ruling: worktree made with `git worktree add`, session cwd stays in the main checkout — the native worktree tool places trees inside the main checkout where the gate hook blocks edits and cannot base on 074a5699 — cost if wrong: the harness does not track this worktree for cleanup.
- Ruling: execution uses multi-agent workflows per the approved plan, not the inline default of the executing-plans skill — the owner enabled orchestration and approved waves — cost if wrong: more tokens than inline.
- Ruling: baseline accepted as starting point although not green with default test threads — the failing assertions race on a global counter and are unrelated to loop edits; recorded as audit finding — cost if wrong: a real allocation regression hides behind the flake.
- Ruling: ledger ids L-0300+ for this loop — ids L-0205..L-0214 exist on the unmerged switcher branch and other L-0205.. rows on the rei branch — cost if wrong: gaps in the id sequence.
- Ruling: V02 gate T4 (owner signs the protected-blob delta) is covered by owner decision OD-L1-1 when T1-T3 and T5 pass; the delta list is reported — cost if wrong: the owner sees the delta after adoption on the research branch (revertible, never on main).
- Ruling: design-review refinements adopted where stricter or structural: stage levers are StageLinearSystem clients in new files; V07 threaded stage gated on (J+LU)/chain in [0.15,0.92]; faer threading not enabled; lineage corrections go to an addendum; LOOP_EXECUTION_RECORD.json untouched — cost if wrong: a lever that needs a protected edit stays unbuilt this loop.
- W0: bootstrap written: PREREGISTRATION.md, OWNER_DECISIONS_20261116.json, V00..V15 contracts, RELEASE_GATE_MATRIX.json, loop CHECKPOINT.json, stage_clients/ and mol/ stubs.
- W0: committed 2dc9a2c5 and pushed research/production-loop-20261116 (remote readback equal). Builder worktrees -v03g, -v15, -trial created from it.
- Ruling: builder cargo target directories live on the internal drive (/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/{v03g,v15,trial}) instead of /mnt/extdrive — copying the cache to the NTFS-fuse external drive failed with ENOSPC although space and inodes were free — cost if wrong: about 1 GB per builder on the system disk.
- W1: workflow run wf_7cef339f-7cb launched (audit x3 + verification + synthesis, V05 theory, V14 screen, V03G golden -> V02 trial merge, V15 client kit). Script: ~/.claude/projects/-home-cosmosapjw-vigilode/d057eb25-bf20-49ac-a1df-6dd2adc8a0a4/workflows/scripts/vigilode-loop-l1-wave1-wf_7cef339f-7cb.js
- W1: V12 draft written inline (port_plan/port_contracts.json, PORT_PLAN_KO.md; 20 items); final after the V02 verdict.
