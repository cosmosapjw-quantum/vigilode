# Phys–Math Coding Harness for GPT-5.6 / Codex

물리학·수학 연구 코드를 위한 **리포지터리 하네스**다. 긴 프롬프트보다 `AGENTS.md`, scientific contract, validation matrix, durable logs, repo-scoped skills를 사용한다.

## 기존 repo에 설치

repo 루트에서 이 ZIP을 풀었다는 전제:

```bash
python3 tools/init_harness.py
python3 tools/validate_harness.py
```

이미 `AGENTS.md`가 있다면 덮어쓰기 전에 내용을 병합해라. 이 archive는 Codex가 즉시 읽을 수 있는 root `AGENTS.md`를 포함한다.

## 시작 경로

- 작은 버그/기능: `prompts/00_ultralight.md`
- 복잡한 단일 실행: `prompts/01_deep_once.md`
- 장기 작업: `prompts/phases/00_task_contract.md`부터 순차 실행
- Work → Codex handoff: `prompts/handoff/work_to_codex.md`
- Codex → Work 결과 전달: `prompts/handoff/codex_to_work.md`

## 핵심 루프

```text
Scientific/software contract
→ Reproduction or acceptance baseline
→ Repository localization
→ Bounded solution design
→ Execution plan
→ Isolated implementation
→ Software validation
→ Scientific validation
→ Numerical/reproducibility validation
→ Independent diff review
→ Promote / hold / rework / revert
```

## 주의

- 코드가 실행되는 것과 과학적으로 맞는 것은 별도 gate다.
- 버그는 먼저 재현하고, 기능은 observable acceptance criterion을 정하며, 리팩터링은 characterization test로 기존 동작을 잠근다.
- 독립 작업만 subagent/worktree로 병렬화한다.
- 모델·기능 가용성은 플랜과 버전에 따라 다를 수 있다. `docs/RESEARCH_BASIS.md` 참고.
