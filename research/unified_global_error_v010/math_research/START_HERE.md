# Start Here

## 새 연구 저장소에서

```bash
python3 tools/init_workspace.py --project "Project Name"
make validate
```

## ChatGPT Project / Work에서

1. `PROJECT_INSTRUCTIONS.md`를 Project Instructions에 붙인다.
2. `state/`의 여섯 파일을 Project sources에 추가한다.
3. 핵심 자료와 현재 연구 메모를 sources에 추가한다.
4. 짧은 세션은 `prompts/quick/ultralight.md`를 사용한다.
5. 중요한 프로젝트는 `prompts/phases/01_research_contract.md`부터 순차 실행한다.

## Local desktop skill 사용

저장소에는 `.agents/skills/`가 포함되어 있다. Codex/ChatGPT desktop에서 repo-scoped skill이 인식되면 `$skill-name`으로 명시 호출하거나 설명에 맞는 작업에서 자동 선택하게 둘 수 있다.

## 운영 원칙

- 한 phase에서 다른 phase로 조용히 넘어가지 않는다.
- candidate producer가 최종 promote 결정을 혼자 내리지 않는다.
- citation 존재와 실제 claim support를 구분한다.
- evidence가 서사보다 먼저다.
- formalization은 promote된 후보에만 허용한다.
- 중요한 milestone 뒤에만 context를 compact한다.
