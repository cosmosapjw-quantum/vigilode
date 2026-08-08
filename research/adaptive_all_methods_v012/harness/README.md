# Phys–Math Research Harness for GPT-5.6 / ChatGPT Work

전문 물리학·수학 연구를 위한 **프롬프트 + 컨텍스트 + 하네스 + 루프** 저장소다.
거대한 만능 프롬프트 대신, 연구 상태를 파일로 유지하고 각 phase에 필요한 계약과 gate만 실행한다.

## 30초 시작

```bash
python3 tools/init_workspace.py --project "내 연구 프로젝트"
python3 tools/validate_workspace.py
```

그다음:

1. `PROJECT_INSTRUCTIONS.md`를 ChatGPT Project의 instructions에 붙인다.
2. 핵심 논문·메모와 `state/` 파일을 Project sources에 넣는다.
3. 빠른 작업은 `prompts/quick/ultralight.md`, 긴 Work run은 `prompts/00_integrated_work_run.md`를 사용한다.
4. 정식 연구 cycle은 `prompts/phases/01_...`부터 순서대로 실행한다.
5. 각 phase 종료 후 `state/` 파일을 갱신하고 Project source로 저장한다.

## 핵심 루프

```text
Research contract
→ Evidence acquisition
→ Claim–source audit
→ Hypothesis space
→ Independent adversarial review
→ Physics/mathematics validation
→ Minimal decisive verification
→ External decision gate
→ Survivor-only formalization
→ Closeout/compaction
```

## 세 가지 실행 강도

- **초경량**: `prompts/quick/ultralight.md`
- **한 번에 깊게**: `prompts/quick/deep_once.md`
- **다단계 정식형**: `prompts/phases/`

## 제품별 역할

- **Chat**: 질문 좁히기, 국소 토론, 빠른 검토
- **Deep Research**: 여러 웹·파일·앱을 이용한 출처 기반 문헌 조사
- **Work**: 장시간 다단계 연구와 완성된 보고서/문서
- **Projects**: instructions, files, chats, saved responses를 보관하는 외부 기억

공식 기능은 계정·플랜·워크스페이스에 따라 다를 수 있다. `docs/RESEARCH_BASIS.md`의 확인 날짜와 공식 문서를 참고해라.
