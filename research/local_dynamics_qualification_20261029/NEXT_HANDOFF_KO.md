# 다음 작업 인계 — 누적 qualification

## 현 상태

- Claim ceiling: **HOLD**, broad production 미승인.
- 검증 소스: 현재 실행 `0255297e4f865bb78753cad9fe51d6f27536d188` (tree `4b1b7e7da6fa5e1fdcd064c0de1d19671c31e5b3`).
- 8 suite receipt source: `a1a459f2be186d9f67ff86bf38c19ae24b769768`.
- 역사 replay `58ec9b053163da6b6f0906010e0422c104db72fd`, F6 native `299310290c2e254149ee30f4a9834c1123066815`, delivery SHA `bf308589e21b524d0ddc6e571b260f5e94a853f0`는 서로 다른 identity다.
- 새 장시간 benchmark는 시작하지 않는다. 성공/실패가 분명한 완료 evidence는 `SUMMARY.json`의 raw manifest를 보고 재사용한다.

## 먼저 재확인할 identity

```bash
git status --short
git rev-parse HEAD
git rev-parse 'HEAD^{tree}'
git rev-parse origin/validation/local-dynamics-exec-20261028T170155Z
```

문서 수정 후 commit이 새로 생겼더라도 소스 실행 identity는 위 고정 SHA로 별도 기록한다. Cargo.lock은 수정하지 않는다. suite receipt의 source `a1a459f2`를 `0255297e`의 실행 결과라고 부르지 않는다.

## READY / 차단된 다음 일

1. **Q-LEDGER — 우선 blocked:** scoped repair는 있으나 latest full authority exit 1이다. 실패 receipt와 정확한 repository/object identity를 사용하여 실제 누락 authority 입력을 확인한다. 일괄 allowlist 또는 historical evidence 수정으로 통과를 만들지 않는다. 필요 authority repository/path가 없으면 `HOLD` 및 누락 input을 반환한다.
2. **Q-API — 판정/수정 필요:** baseline/current consumer 결과는 `BREAKING_CHANGE_REPRODUCED`이다. 공공 API compatibility policy, semver classification 및 explicit migration/version decision을 검토한다. 이번 fixture를 green으로 뒤집거나 API break를 무시하지 않는다.
3. **Q-BENCH / SUNDIALS:** 7 adapter tests exit 0만 확인됐다. 독립 vendor graph, target system headers/compiler/link/runtime evidence가 새로 준비될 때만 이어서 qualification하고, 그전까지 SUNDIALS는 `PENDING`.
4. **Q-GIVENS:** 세 exact residual fixtures의 current scoped PASS를 그대로 유지. 다른 tolerance/shape/regime로 일반화하지 않는다.
5. **Q-LINT:** source 0255297e retry exit 0를 scoped pass로 사용. 누락 target environment였던 first attempt는 불완전 attempt로 보존한다.
6. **Release / suites:** suites-a1 8개 current receipt는 완료. 동일 source에서 안심 목적 재실행 금지. 실제 deployment host/CPU/libm가 달라질 때만 L-RELEASE / L-HARDWARE 검증을 연다.
7. **23-node DAG와 10 interactions:** interaction은 현재 전부 미실행. dependencies/correction counters를 유지하며 READY frontier만 진행한다. 이번 software pass나 F6 manufactured result로 R-QUALIFY 또는 전체 DAG를 닫지 않는다.

## 누적 correction budget

```json
{
  "I-PROD": {
    "maximum": 3,
    "source": "LOCAL_CODEX_HANDOFF_KO.md",
    "used": 1
  },
  "PR-N01": {
    "maximum": 3,
    "source": "cumulative_validation_20261028/SUMMARY.json",
    "used": 0
  },
  "PR-N02": {
    "maximum": 3,
    "source": "cumulative_validation_20261028/SUMMARY.json",
    "used": 0
  },
  "PR-N03": {
    "maximum": 3,
    "source": "LOCAL_CODEX_HANDOFF_KO.md",
    "used": 2
  },
  "PR-N04": {
    "maximum": 3,
    "source": "LOCAL_CODEX_HANDOFF_KO.md",
    "used": 1
  },
  "PR-N05": {
    "frozen_snapshot_used": 2,
    "maximum": 3,
    "source": "LOCAL_CODEX_HANDOFF_KO.md",
    "used": 3
  },
  "PR-N06": {
    "maximum": 3,
    "source": "cumulative_validation_20261028/SUMMARY.json",
    "used": 0
  },
  "Q-API": {
    "frozen_snapshot_used": 0,
    "maximum": 3,
    "source": "cumulative_validation_20261028/SUMMARY.json",
    "used": 1
  },
  "Q-BENCH": {
    "maximum": 3,
    "source": "cumulative_validation_20261028/SUMMARY.json",
    "used": 0
  },
  "Q-FAIR-AB": {
    "maximum": 3,
    "source": "cumulative_validation_20261028/SUMMARY.json",
    "used": 0
  },
  "Q-GIVENS": {
    "maximum": 3,
    "source": "cumulative_validation_20261028/SUMMARY.json",
    "used": 0
  },
  "Q-LEDGER": {
    "frozen_snapshot_used": 1,
    "maximum": 3,
    "source": "LOCAL_CODEX_HANDOFF_KO.md",
    "used": 3
  },
  "Q-LINT": {
    "frozen_snapshot_used": 0,
    "maximum": 3,
    "source": "cumulative_validation_20261028/SUMMARY.json",
    "used": 1
  },
  "Q-NEGATIVE": {
    "maximum": 3,
    "source": "cumulative_validation_20261028/SUMMARY.json",
    "used": 0
  },
  "R-ADAPT": {
    "maximum": 3,
    "source": "LOCAL_CODEX_HANDOFF_KO.md",
    "used": 3
  },
  "R-HOMO": {
    "maximum": 3,
    "source": "LOCAL_CODEX_HANDOFF_KO.md",
    "used": 1
  },
  "R-STRUCT": {
    "maximum": 3,
    "source": "LOCAL_CODEX_HANDOFF_KO.md",
    "used": 3
  }
}
```

## 저장된 증거와 실행 재개 지점

- Summary: `research/local_dynamics_qualification_20261029/SUMMARY.json`
- Human report: `research/local_dynamics_qualification_20261029/REPORT_KO.md`
- Large local receipts/raw: `/home/cosmosapjw/evidence-20261028T170155Z/`; do not overwrite or move.
- Final current native command: `python3 research/dynamics_symbolic_validation_20261028/code/run_scoped.py --source-sha 0255297e4f865bb78753cad9fe51d6f27536d188 --remote-ref origin/validation/local-dynamics-exec-20261028T170155Z --output <new external dir> --cargo cargo` (only run for a changed relevant source/contract; prior same-identity run is already represented).
- 8 suite receipts: `/home/cosmosapjw/evidence-20261028T170155Z/suites-a1/`; exact argv/result receipts are per command. Never combine the suite source with current native-source claims.
- Q-API completed outputs: `/home/cosmosapjw/evidence-20261028T170155Z/qapi-final3/SUMMARY.json`.
- Q-LEDGER last authority output: `/home/cosmosapjw/evidence-20261028T170155Z/qledger-attribution-final/authority-stdout.log` and corresponding exit; must remain FAIL until the authority graph is actually complete.

Continue within the owner-authorized scope and existing correction budgets; Q-LEDGER is exhausted at 3/3. Continue with identified missing authority inputs and scope-bound migration decision; retain `HOLD` unless all required independent gates close.

재현 전 전용 고정 실행 ref를 fetch하고 별도 detached worktree에서 확인한다: `git fetch origin validation/local-dynamics-exec-20261028T170155Z`; `git worktree add --detach <NEW_WORKTREE> 0255297e4f865bb78753cad9fe51d6f27536d188`. 새 worktree에서 `HEAD`와 `origin/validation/local-dynamics-exec-20261028T170155Z`가 둘 다 해당 SHA인지 확인한 뒤 위 명령을 실행한다. 결과 branch는 이 ref와 별개다.

최종 checker 실행 source는 `c9c734529cf5bbbc33683987006bf71fca3f122d` (tree `32243dae409795171a63210e38a2709b605a4bf6`)다. 025 이후 native source는 변경되지 않았다. Q-LEDGER 3/3은 소진되었고 full authority는 exit 1이다.
