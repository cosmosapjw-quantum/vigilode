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
| Algebraic packet, 국소 AD, 구조화된 반응·확산, 연속 trajectory 인증 | [#77](https://github.com/cosmosapjw-quantum/vigilode/pull/77) | `research/hybrid-structure-ad-20261020` / `5186e021dd8bc9767337b3ae86cbf1b62356f66e` | measured source `49ca3150c68d527a275c855ecf814122ff9c533a`, `research/hybrid_20261020/runs/39378576576`, 새 `recovery_20261020/` 9개 파일 |
 
위 PR들은 조회 당시 모두 **Draft·미병합** 상태였다. 기존 실행 로그·보존된 실패·해시·분할 전송 한계는 각각 원래 브랜치의 기록이 권위 있는 원전이다.

## 본 누적 브랜치에 새로 들어온 독립 연구

- [공유 Krylov·8-stage 동시 homotopy 원본 분석](integration/REPORT_KO.md): 원래 Rust production을 바꾸기 전, 물리/수학 구성과 별도 Python probe 연구.
- [Jacobian/JVP 없이 가능한 FVM·packet·causal 인증 연구](derivative_free/REPORT_KO.md): 반사/주기 FVM 전파, 대수적 국소 반응 소거, Rubinstein/Bernstein 연속 defect, AMR/ALE 반례와 설계.
- PR #77의 [복구 분석](../hybrid_20261020/RECOVERY_REVIEW_KO.md)과 [재현 문서](../hybrid_20261020/REPRODUCE_RECOVERY.md): 독립 증거와 이미 원격에 있던 수치 결과를 구분한다.

## 컨테이너에만 남아 있고 원격에 원본 ZIP 바이트를 아직 게시하지 않은 번들

이 섹션은 업로드 성공을 주장하지 않기 위한 정확한 **미게시 항목 장부**이다. 원본은 대화 산출물에서 받거나, 아래 SHA로 검증해야 한다.

| 원본 ZIP | SHA-256 | 포함 자료 | 상태 |
|---|---|---:|---|
| `VigilODE_Integration_Research_20261020.zip` | `54f4f4a99d552388fc28d01f5f367176e301c2af15c3cd8a9c7a30b4e5dfd4f1` | 22개 파일, 수치 probe/테스트/원자료 | ZIP 및 다수 코드·raw 미게시 |
| `VigilODE_DerivativeFree_FVM_Research_20261020.zip` | `a17864dd8fa1951c7ddcd236740c564c10b323248b1bb30c92eea32bc1e745e7` | 18개 파일, Fraction/Bernstein 실험 및 약 249 kB adaptive JSON | ZIP 및 다수 코드·raw 미게시 |
| `VigilODE_VODE_Diffsol_Comparison_20261019.zip` | `790b4621e09dff21e196cc548b7851ca751d09f7b0287c52e0bc857fca2ad9ae` | 11개 파일, 별도 SciPy VODE 대조 스크립트/원자료 | 원본 ZIP 미게시; GitHub PR #71의 후속 독립 3자 캠페인이 있음 |
| `VigilODE_Hybrid_AD_Recovery_20261020.zip` | `aea357e42f53e027b18ab012a5cb9fb554bd3d0f2ab2a641a7eaa27b6632c656` | 21개 파일, recovery patch/검산, 출처 원본 | 핵심 복구 패치 9개는 PR #77에 정확히 게시; 원본 ZIP 바이트는 미게시 |

**중요:** user-supplied report/Python 프로토타입의 새로 게시된 텍스트는 장기 분석의 의미론적 보존이다. 원본 ZIP의 raw measurement bytes나 전체 replay payload가 모두 원격에 있다는 주장은 하지 않는다. 큰 재생성 가능 데이터를 제외할 때도 생성 코드와 source binding이 없으면 그 자료가 완전히 재생성 가능하다고 주장하지 않는다.

## 상태와 남은 최소 조치

1. PR #71–#77, 본 누적 브랜치의 원격 source/artifact 게시: **확인된 범위에서 완료**.
2. 하이브리드 복구 추가 9개 파일의 SHA 보존: **완료**.
3. 위 미게시 독립 Python 프로토타입·숫자 원자료 전체와 ZIP 원본: **아직 업로드 미완료**. 파일 바이트를 첨부 경로에서 GitHub Blob으로 전송 가능한 환경에서 원본 해시와 함께 추가해야 한다.
4. 독립 PR들의 merge, solver 기본값 전환: **수행하지 않음**. 서로 다른 measured binary/case/timing을 단일 수치 승급으로 합치지 않는다.

## 원본 전체 자료를 마저 GitHub에 올리는 재현 가능한 명령

대화에서 다운로드한 네 ZIP을 한 폴더에 모으고, 이 누적 브랜치의 **깨끗한 worktree**에서 다음 명령을 실행한다.

```bash
bash research/cumulative_20261020/import_remaining_archives.sh /path/to/downloaded-zips
```

스크립트는 4개 ZIP의 원본 SHA-256, 내부 72개 파일의 개별 digest를 확인하고 바이너리 원본과 압축해제된 모든 원본을 별도 `original_archives/` 위치에 적재한다. 사용자가 실행하기 전까지 이 원본 ZIP들은 **NOT_PUSHED**다. 이번 대화 환경의 GitHub 커넥터는 Git 텍스트/blob 편집을 지원하지만 작업 컨테이너의 binary ZIP 경로를 그대로 인자로 받지는 않는다. 작업 컨테이너 Git HTTPS는 github.com DNS 해석에 실패했다. 따라서 위 명령을 실행했다고 주장하지 않는다.

Derivative-free 보고서는 원본 `REPORT_KO.md`의 Git blob identity를 확인했으며, Integration 보고서에는 대화 업로드된 표기형 버전이 게시돼 있어 원본 ZIP 안의 byte identity를 주장하지 않는다. 최종 전체 ZIP/코드 원본 업로드가 완료되면 해당 상태를 갱신해야 한다.
