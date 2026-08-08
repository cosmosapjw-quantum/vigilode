# Parallelization and worktrees

## Parallelize

- 서로 다른 module exploration
- 독립 test audit
- software/scientific reviewer
- 별도 worktree의 서로 다른 solution candidate
- 독립 benchmark analysis

## Sequential

- reproduction 결과가 다음 탐색을 결정
- 한 알고리즘의 단계별 구현
- patch → retest loop
- baseline failure root cause
- final promote decision

같은 checkout의 같은 파일을 여러 agent가 동시에 수정하지 않는다.
