# [복사·붙여넣기] VigilODE 2026-10-29 → 다음 연구 스레드 handoff

$physmath-research-loop $scispace $exa $wolfram

PROJECT=cosmosapjw-quantum/vigilode
BASE_PR=98
BASE_SHA=bf308589e21b524d0ddc6e571b260f5e94a853f0
NEW_DRAFT_PR=https://github.com/cosmosapjw-quantum/vigilode/pull/100
NEW_BRANCH=research/physical-symbolic-defect-oracle-20261029
NEW_UNIT=VIGILODE-SYM-01
CORE_DAG=7/23 scoped, delta0, edges34
AUXILIARY_DAG=1/1 PASS_SCOPED_TOY
REI_BIANCHI_SEPARATE_DAG=28/88
PRODUCTION=HOLD

## 현 상태 / 지켜야 할 구분

1. PR90–94→PR96→PR97→PR98은 누적 ancestry. 새 PR100은 PR98 head를 base로 하는 *별도 draft*이며 `main` 또는 PR98을 변경하거나 force-push하지 않았다. PR95→PR99의 actual-source H–Li 계열은 별도 조건부 입력이지 이미 통합된 native production 기능이 아니다.
2. PR98 evidence: 8 suites/73 paths/10 interactions 원형 보존, scoped 375 integration and 70 library tests, Givens exact residual, sparse trace query 개선; production HOLD, coupled all-targets strict Clippy 26 diagnostics, full typed authority missing56, 40 main-base registration omissions, Q-API/Q-GIVENS/Q-NEGATIVE, real-client and release gates open.
3. PR100 adds only exact-rational affine scalar ODE event-sign oracle and theory; no Rust production code path changed. Theorem: same-clock original-equation defect + fixed norm error + event Lipschitz/strict transversality give certified signs and unique crossing. Controller symbolic words ≠ physical-ODE approximate bisimulation.
4. `THEORY_KO.md`, `RESULTS.json`, `STAGE_LEDGER.json`, `CORRECTION_LEDGER.json`, `symbolic_oracle.py`, `test_symbolic_oracle.py`, `REPORT_KO.md`, `PLOT_DATA.json` and `validation_plot.py` are the first reads in PR100. Python 8/8 tests PASS and 918 Decimal70 diagnostic samples; Wolfram envelope-difference identity 0. Event example defect 1/128, state error 1/1088, strict unique crossing at 15/16 threshold between t=1/16 and t=1/15. Diagnostic plots are not formal certificates.
5. First Python indentation failure, mixed residual/error unit-violating plot, and initial theory last-newline byte mismatch were **repaired but retained in CORRECTION_LEDGER**. Do not hide or treat earlier production task budgets as reset.

## 다음 DAG-admissible 작업을 정확히 하나 선택

`VIGILODE-SYM-02` = native source/FP/clock-bound event-provider contract. 먼저 latest repo/ref/AGENTS, unpublished local worktree와 owner-client authoritative source를 읽고 허용 범위를 고정하라. PR100의 일반 theory 2B 충분조건을 target으로 삼아 원래 solver의 dense polynomial, RHS evaluation, phase/time, unit scaling, floating rounding, event callback identity, error carry, step refusal, adaptive interpolation 및 epoch transaction을 연결할 최소 adapter specification과 **negative controls**를 작성하라. Signed lognorm tube, event function Lipschitz/transversality bound와 no-crossing/unique-crossing gate를 제출하기 전에는 new routing/default optimization 적용 금지.

구체적 구현 순서: physical equation/source pin→exact real contract→outward interval provider→candidate dense residual whole slab→signed error tube→event g sign/derivative interval→negative controls (grazing, reset, endpoint-only, float rounding, source switch)→independent native/release tests→matched output/achieved accuracy comparison. 원래 Q-LEDGER 및 Q-GIVENS source/profile discrepancies를 병행 추적하되 해당 gate가 PASS 되었다고 하지 마라. Original `main`/nested PR ancestry 중복 cherry-pick 금지.

## 로컬 Codex 첫 명령

```bash
set -euo pipefail
cd /absolute/path/to/vigilode
git status --short
git fetch origin --prune
git cat-file -e bf308589e21b524d0ddc6e571b260f5e94a853f0^{commit}
git cat-file -e origin/research/physical-symbolic-defect-oracle-20261029^{commit}
STAMP=$(date -u +%Y%m%dT%H%M%SZ)
BR="validation/physical-symbolic-native-${STAMP}"
WT="$(dirname "$PWD")/vigilode-symbolic-${STAMP}"
git worktree add -b "$BR" "$WT" origin/research/physical-symbolic-defect-oracle-20261029
cd "$WT"
python3 -m unittest -v research/physical_symbolic_defect_20261029/test_symbolic_oracle.py
# If import fails outside the package path, run in source directory instead:
(cd research/physical_symbolic_defect_20261029 && python3 -m unittest -v test_symbolic_oracle.py)
```

주의: 첫 `python3 -m unittest -v path/to/test_symbolic_oracle.py` 형식은 Python unittest loader가 해당 경로를 module로 읽지 못하면 실패할 수 있으므로 source directory에서 실행하는 **두 번째 명령이 기준**이다. 새로운 Rust job은 공통 lock에서만 실행, source HEAD를 remote-publish/readback한 뒤 검증한다. 현재 컨테이너에는 실제 Rust source native build의 결과가 없고 기존 PR98 측정치를 새 코드의 실행 증거로 사용하지 않는다.

## 최종 결과 반환 형식

- current head/tree/source blob, client original model binding·unit/clock identity
- R1–R10 이론과 C1–C10 구현 진척, 각 단계마다 `7/23 scoped`와 보조 노드 수를 병기
- PHYS-MATH → PHYS-MATH-CODE dual ledger, corrected/failed/retained plots
- actual commands, compiler versions, exact output tolerances, work counts, refusal rates, `NOT_RUN`/`FAIL` 분리
- GitHub non-force Draft PR + Git file SHA readback; Dropbox/Drive 두 provider 각각의 실제 ACK와 raw byte hash가 없다면 이중백업으로 부르지 않기
- core production `HOLD` 유지. 새 science node completion은 원본 23 DAG 분모에 무단 합산 금지.

NO_FORCE_PUSH=true
NO_MAIN_MERGE=true
NO_FAKE_NATIVE_EVIDENCE=true
NO_RETROACTIVE_PRODUCTION_PROMOTION=true
