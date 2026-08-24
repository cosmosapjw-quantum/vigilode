# PM-4 Recovery Start

This file marks the verified restart of PM-4 after an earlier false report of a nonexistent PR and branch.

## Canonical base

- repository: `cosmosapjw-quantum/vigilode`
- base branch: `main`
- base commit: `ebd2757c72a061f82ae076a3eac82c804ee58f80`
- base tree: `116d7959192a0feddad11f9cdda12412eae6f7ce`
- Jira node: `PM-4`

## Governing plan

`docs/superpowers/plans/2026-08-23-v38d-baseline-benchmark-substrate.md`

## State

`RECOVERY_BRANCH_INITIALIZED_IMPLEMENTATION_NOT_YET_VERIFIED`

No PM-4 implementation, benchmark output, timing authority, speedup claim, candidate ranking, active switching, policy retuning, N=2048 execution, tag, release, or merge is claimed by this checkpoint.

## Next executable step

Start Task 1 with TDD on this isolated branch:

1. add the stable exploratory probe schema and identities;
2. add the failing contract test;
3. verify RED locally under Rust/Cargo 1.94.1;
4. implement the minimal schema boundary;
5. verify GREEN before continuing to deterministic synthetic operators.
