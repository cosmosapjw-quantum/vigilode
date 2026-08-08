# Start Here

## 1. 설치 확인

```bash
python3 tools/init_harness.py
make harness-check
```

## 2. 반드시 채울 파일

- `SCIENTIFIC_CONTRACT.md`: 물리·수학적 정답의 정의
- `VALIDATION_MATRIX.md`: software/scientific/numerical/reproducibility checks

## 3. Codex에서

1. repo 루트에서 Codex를 시작한다.
2. 작은 작업은 `prompts/00_ultralight.md`에 TASK/SCOPE/NON_GOALS를 채운다.
3. 복잡한 작업은 `prompts/01_deep_once.md` 또는 phase prompts를 사용한다.
4. 중요한 변경은 별도 reviewer 또는 `$independent-diff-review` skill을 실행한다.
5. parallel implementation은 반드시 별도 Git worktree를 사용한다.

## 4. 완료 정의

코드 작성이 아니라 acceptance, regression, scientific, numerical, reproducibility validation과 independent review를 충족한 상태다.
