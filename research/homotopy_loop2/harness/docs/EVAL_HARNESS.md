# Private evaluation harness

공개 benchmark만 믿지 말고 private task suite를 유지한다.

권장 구성:

- 작은 버그 5
- multi-file 버그 5
- numerical instability 3
- scientific invariant violation 3
- 기능 추가 3
- behavior-preserving refactor 3
- 불충분한 요구사항 3

기록:

MODEL, REASONING, HARNESS_VERSION, AGENTS_VERSION, REPO_COMMIT, TASK, TOOLS, WORKTREE, SUCCESS, TESTS, SCIENTIFIC_CHECKS, TOKENS/CREDITS, WALL_TIME, HUMAN_INTERVENTIONS, REGRESSIONS, NOTES.

최소 baseline은 `localize → patch → validate`. 그 위에 reviewer, skills, worktrees, subagents를 하나씩 추가해 ablation한다.
