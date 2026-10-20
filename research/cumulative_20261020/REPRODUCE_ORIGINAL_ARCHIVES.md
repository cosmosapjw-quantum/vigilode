# VigilODE 누적 원본 아카이브 복원 및 재생성

이 문서는 **실제로 보존된 네 독립 연구 ZIP**과 내부 72개 파일의 복원 절차다. 기록된 벤치마크 wall-time 값을 새 실행에서 비트 단위로 재현할 수 있다는 뜻이 아니다.

## A. GitHub에서 원본 복원

```bash
git fetch origin research/cumulative-evidence-20261020
git switch research/cumulative-evidence-20261020
python3 research/cumulative_20261020/verify_original_archives.py
```

기대 결과: `PASS_EXACT_ARCHIVE_MEMBERS_AND_SHA256` / `zip_files=4` / `source_members=72`.

- `research/cumulative_20261020/original_archives/`: 네 ZIP의 **원래 바이트**
- `research/cumulative_20261020/original_sources/`: 네 ZIP에서 나온 **72개 원래 파일**
- `research/cumulative_20261020/ORIGINALS_MANIFEST.json`: 원본 ZIP/각 파일의 SHA-256, Git blob SHA-1, 크기 및 Drive ID

Python 표준 라이브러리만 사용한다. 파일을 빌드·재실행하거나 기존 결과를 덮어쓰지 않는다.

## B. Google Drive에서 원본 복원

원본 ZIP을 다음 링크에서 다운로드해, 각 파일명을 변경하지 말고 별도의 새 폴더에 저장한다.

- [Integration research](https://drive.google.com/file/d/1KFeUnjwSxuVF8NXujyLx9QOr69hbVM6-/view)
- [Derivative-free FVM](https://drive.google.com/file/d/1kiZpLR8RAIMmF5x_j_siQ-vJdJMtvcDh/view)
- [VODE/Diffsol comparison](https://drive.google.com/file/d/1fPfXkgTjKYzelxufhz56XehfLZ4xKlZF/view)
- [Hybrid AD recovery](https://drive.google.com/file/d/1FOucG_ROrsBiVL3XSLxVQSsSEzUJH0UK/view)

```bash
sha256sum -c - <<'SUMS'
54f4f4a99d552388fc28d01f5f367176e301c2af15c3cd8a9c7a30b4e5dfd4f1  VigilODE_Integration_Research_20261020.zip
a17864dd8fa1951c7ddcd236740c564c10b323248b1bb30c92eea32bc1e745e7  VigilODE_DerivativeFree_FVM_Research_20261020.zip
790b4621e09dff21e196cc548b7851ca751d09f7b0287c52e0bc857fca2ad9ae  VigilODE_VODE_Diffsol_Comparison_20261019.zip
aea357e42f53e027b18ab012a5cb9fb554bd3d0f2ab2a641a7eaa27b6632c656  VigilODE_Hybrid_AD_Recovery_20261020.zip
SUMS
```

검사 통과 후 `unzip`으로 각 ZIP을 **서로 다른 임시 디렉터리**에 풀어 원본 스크립트·로그·JSON을 읽는다. Drive 링크는 기존 비공개 설정 그대로이며 공유 권한을 확대하지 않았다.

## C. 연구 실행의 진정한 재현 범위

각 원본 스크립트는 모델, 환경, 입력/출력 및 자체 검증 절차를 별도로 명시한다. 먼저 `README.md` / `RESEARCH_STATE.md` / `SOURCE_MAP.json` 등 각 ZIP에 **실제로 들어 있는 원전**을 읽고 필요한 종속성·소스 해시를 확인한다. 단일 스크립트를 실행하면 다른 브랜치에서 생성된 모든 측정 원자료까지 재생성된다고 가정하지 않는다.

벤치마크 원자료는 해당 PR #71, #73~#77의 보존된 `runs/`, `refined_runs/`, `readback_runs/` 등에서 각각 확인한다. 새 컴파일·실행은 별도 호스트·실행 소스·새 run ID를 남겨야 하며 과거 기록을 덮어쓰지 않는다.

대형 Rust 환경의 Drive 백업과 재결합 절차는 [LARGE_DRIVE_BACKUPS_KO.md](LARGE_DRIVE_BACKUPS_KO.md)를 참고한다.
