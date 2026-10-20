# VigilODE 연구·벤치마크 누적 인벤토리 (2026-10-20 KST)

본 인덱스는 **원격에 실제로 저장된 커밋/로그와, 현재 대화 컨테이너에만 있는 독립 연구 번들**을 구분한다. 원본 측정의 source SHA와 소스 변경 후의 출판 HEAD를 혼동하지 않는다. 다른 브랜치의 기록을 재측정했다고 표시하지 않는다.

## GitHub에 게시된 연구 브랜치 및 측정

| 성격 | PR | 브랜치 및 누적 상태 | 실제 증거 |
|---|---|---|---|
| VODE/Diffsol/VigilODE 엄격 3자 비교, 달성 오차 정합 | [#71](https://github.com/cosmosapjw-quantum/vigilode/pull/71) | `research/benchmark-results-20261019-dual` / `64b9852124b67eedbb8575b42c01127e4b86283b` | `research/strict_solver_comparison_20261019/runs/39257390707`, `refined_runs/39261895983`, `readback_runs/39263860973` |
| Banded 수치오류 수정·정확 LU cache | [#73](https://github.com/cosmosapjw-quantum/vigilode/pull/73) | `research/banded-exact-reuse-20261020` / `3c0157910076ce552283c831c2be000c66671fab` | `research/banded_exact_reuse_20261020` |
| JVP-only 전체 stage 병렬 homotopy의 Rust 구현 | [#74](https://github.com/cosmosapjw-quantum/vigilode/pull/74) | `research/coupled-matrix-free-rust-20261020` / `15c43ce76734d0a560fcec5909ac49f1e6f2d347` | `research/coupled_native_20261020/runs/39349031422`, `closeout_runs/39349959195` |
| Residual-first·후보 JVP budget 실험 | [#75](https://github.com/cosmosapjw-quantum/vigilode/pull/75) | `research/coupled-economy-20261020` / `6c945b6c6e0e024d8f733c21da8d35fbcc3c5f2a` | `research/coupled_economy_20261020/runs/39355449893`, `reference_recovery/39357596507` |
| 동일 호스트 Diffsol/DVODE/coupled 중간점검 | [#76](https://github.com/cosmosapjw-quantum/vigilode/pull/76) | `research/midcheck-three-solver-20261020` / `6972f28aa4bcbe4c68dbba92e44ff58884545bca` | `research/midcheck_20261020/runs/39360536475` |
| Algebraic packet, 국소 AD, 구조화된 반응·확산, 연속 trajectory 인증 | [#77](https://github.com/cosmosapjw-quantum/vigilode/pull/77) | `research/hybrid-structure-ad-20261020` / `9bb05bed4cd7df720f2b429c2b47e97ed76712d5` | measured source `49ca3150c68d527a275c855ecf814122ff9c533a`, `research/hybrid_20261020/runs/39378576576`, 새 `recovery_20261020/` 9개 파일 |
 
위 PR들은 조회 당시 모두 **Draft·미병합** 상태였다. 기존 실행 로그·보존된 실패·해시·분할 전송 한계는 각각 원래 브랜치의 기록이 권위 있는 원전이다.

## 본 누적 브랜치에 새로 들어온 독립 연구

- [공유 Krylov·8-stage 동시 homotopy 원본 분석](integration/REPORT_KO.md): 원래 Rust production을 바꾸기 전, 물리/수학 구성과 별도 Python probe 연구.
- [Jacobian/JVP 없이 가능한 FVM·packet·causal 인증 연구](derivative_free/REPORT_KO.md): 반사/주기 FVM 전파, 대수적 국소 반응 소거, Rubinstein/Bernstein 연속 defect, AMR/ALE 반례와 설계.
- PR #77의 [복구 분석](../hybrid_20261020/RECOVERY_REVIEW_KO.md)과 [재현 문서](../hybrid_20261020/REPRODUCE_RECOVERY.md): 독립 증거와 이미 원격에 있던 수치 결과를 구분한다.

## 원본 자료 GitHub 게시 + Google Drive 이중 백업 (검증 완료)

이전 `NOT_PUSHED` 장부는 **복구 이전의 기록**이다. 최신 GitHub PR #78의 원본 아카이브 4개, ZIP 내부 72개 파일, Google Drive 백업 4개를 다시 검사해 아래처럼 갱신한다.

| 원본 ZIP | 원본 크기 | ZIP 내부 파일 | GitHub | Google Drive |
|---|---:|---:|---|---|
| `VigilODE_Integration_Research_20261020.zip` | 47,810 B | 22 | [원본 ZIP](original_archives/VigilODE_Integration_Research_20261020.zip) | [원본 백업](https://drive.google.com/file/d/1KFeUnjwSxuVF8NXujyLx9QOr69hbVM6-/view) |
| `VigilODE_DerivativeFree_FVM_Research_20261020.zip` | 121,653 B | 18 | [원본 ZIP](original_archives/VigilODE_DerivativeFree_FVM_Research_20261020.zip) | [원본 백업](https://drive.google.com/file/d/1kiZpLR8RAIMmF5x_j_siQ-vJdJMtvcDh/view) |
| `VigilODE_VODE_Diffsol_Comparison_20261019.zip` | 18,869 B | 11 | [원본 ZIP](original_archives/VigilODE_VODE_Diffsol_Comparison_20261019.zip) | [원본 백업](https://drive.google.com/file/d/1fPfXkgTjKYzelxufhz56XehfLZ4xKlZF/view) |
| `VigilODE_Hybrid_AD_Recovery_20261020.zip` | 66,374 B | 21 | [원본 ZIP](original_archives/VigilODE_Hybrid_AD_Recovery_20261020.zip) | [원본 백업](https://drive.google.com/file/d/1FOucG_ROrsBiVL3XSLxVQSsSEzUJH0UK/view) |

**실측이 아닌 보존 검증**: [전체 원본 장부](ORIGINALS_MANIFEST.json)의 원본 ZIP 4개 및 [원본 소스 트리](original_sources/)의 72개 파일을 GitHub의 재귀 Git tree와 비교했다. **76/76개 Git blob SHA-1 및 바이트 크기 일치**. 네 Google Drive 원본 ZIP은 새로 완전 다운로드한 바이트의 **SHA-256 4/4 일치**. 네 ZIP의 로컬 사본도 SHA-256 및 내부 파일 수를 다시 확인했다. [검증 결과 JSON](VERIFIED_REMOTE_CLOSEOUT_20261020.json).

이 자료는 원본 ZIP 바이트와 내부 수치 실험 스크립트/원자료까지 저장된 **source-complete 독립 실험 아카이브**다. 단, 기존 PR의 다른 측정 호스트와 시간 표본을 합쳐 새 성능 결과라고 주장하지 않으며, 현재 브랜치에서 새 수치 계산을 수행했다는 뜻도 아니다. 각 PR의 본래 실행 로그·실패 기록·source SHA가 해당 실험의 원전이다.

## 검증과 재생성

- [ZIP 원본과 72개 내부 파일을 바이트 단위로 검증하는 코드](verify_original_archives.py)
- [GitHub checkout 또는 Drive ZIP 다운로드에서 복원하는 명령](REPRODUCE_ORIGINAL_ARCHIVES.md)
- [184 MiB Rust 오프라인 배포본 및 33 MiB RUSTCORE의 Drive 백업과 분할 재결합법](LARGE_DRIVE_BACKUPS_KO.md)

Google Drive 백업은 **기존 비공개 접근 설정을 변경하지 않았다**. 소유 계정의 Google Drive 접근 권한이 있어야 링크를 열 수 있다. GitHub 파일은 PR #78의 현재 브랜치에 올라 있지만 아직 기본 브랜치에 merge되지 않았다.

## 게시·승급 상태

1. 원본 ZIP 4개와 내부 독립 파일 72개: **원격 게시/검증 완료**.
2. 동일 바이트 Google Drive 백업 4개: **원본 SHA-256 검증 완료**.
3. 기존 PR #71, #73–#77의 수치 결과·기록: 각 원격 원전에서 유지; 해당 브랜치를 덮어쓰지 않음.
4. PR #78: **Draft·미병합 유지**. 원본 출판 검증으로 archive completeness를 확인했을 뿐 solver의 일반적인 정확도/성능 승급은 선언하지 않음.

이전 `import_remaining_archives.sh`는 오프라인 재현/새 환경 이식용으로 보존한다. 현재 저장소에 이미 들어온 원본에 다시 강제 적용하거나 덮어쓸 필요는 없다.
