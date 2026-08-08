# Codex setup

- repo 루트에서 Codex를 시작해 `AGENTS.md`가 자동 로드되게 한다.
- repo-specific reusable workflows는 `.agents/skills/`에 둔다.
- 동일 파일을 병렬 수정할 때는 별도 worktree를 사용한다.
- 개인 기본값은 사용자 config, repo behavior는 repo files에 둔다.
- 기존 `AGENTS.md`가 있다면 이 harness의 핵심 섹션을 병합하고 중복·충돌을 제거한다.
