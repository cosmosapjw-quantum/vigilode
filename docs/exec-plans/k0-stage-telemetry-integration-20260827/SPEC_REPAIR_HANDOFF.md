# WU-03 Specification Repair Handoff

## Preserved implementation state

```text
branch  research/k0-stage-telemetry-integration-20260827
WU-00   183e24feb39fd7581450ae4380bd8afe09249451
WU-01   faa759de5c54848bb60d4cb8af4b06b6bcbbe514
WU-02   2badcec35b51d23fcd2938d1e15c9e0875a0f9df
tree    5df56a846908972ed0159d8fd59aa47934550a3b
status  clean
remote  three local commits not yet pushed
```

The WU-03 stop is accepted as `BLOCKED_BY_UNRESOLVED_SPEC`. No WU-03 mutation is inherited.

## Resolved semantic boundary

For

```text
f_i      = f(t_i, Y_i)
f_0      = f(t_n, y_n)
j_delta  = J_n (Y_i - y_n)
t_delta  = c_i h f_t,n
N_i      = f_i - f_0 - j_delta - t_delta
```

record exactly

```text
scaled_nonlinear_remainder =
    l2(N_i)
    / max(l2(f_i), l2(f_0), l2(j_delta), l2(t_delta), f64::MIN_POSITIVE)
```

The norm is Euclidean L2. The scalar is not tolerance-weighted and is not capped at one.

## Orchestrator-only repair

After the updated package branch is published, the orchestrator merges its exact fetched tip into the preserved clean WU-02 branch:

```bash
set -euo pipefail
cd <implementation-worktree>

test "$(git branch --show-current)" = \
  "research/k0-stage-telemetry-integration-20260827"
test "$(git rev-parse HEAD)" = \
  "2badcec35b51d23fcd2938d1e15c9e0875a0f9df"
test "$(git rev-parse HEAD^{tree})" = \
  "5df56a846908972ed0159d8fd59aa47934550a3b"
test -z "$(git status --porcelain=v1)"

git fetch --prune origin docs/k0-codex-execution-package-20260827
PACKAGE_TIP="$(git rev-parse origin/docs/k0-codex-execution-package-20260827)"
git merge --no-ff --no-edit "$PACKAGE_TIP"

test "$(git rev-parse HEAD^1)" = \
  "2badcec35b51d23fcd2938d1e15c9e0875a0f9df"
test "$(git rev-parse HEAD^2)" = "$PACKAGE_TIP"
test -z "$(git status --porcelain=v1)"

python tools/verify-k0-stage-telemetry-plan.py \
  --repo-root . \
  --check-package \
  --check-spec-repair-authority
```

Codex does not perform the merge. It resumes WU-03 only after both markers appear:

```text
PACKAGE_CONTRACT_PASS
SPEC_REPAIR_AUTHORITY_PASS
```
