# GitHub–Atlassian Synchronization Contract

## Frozen identifiers

- GitHub repository: `cosmosapjw-quantum/vigilode`
- Control-package branch: `docs/k0-codex-execution-package-20260827`
- Control-package PR: `#21`
- Source parent PR: `#20` at `e1124586a4029f86669e7489278c61ef676d61aa`
- Prepared implementation branch: `research/k0-stage-telemetry-integration-20260827`
- Preparation topology: two-parent merge, first parent PR #20 head, second parent exact package tip
- Jira owner: `PM-7`
- Confluence page: `15499267`
- Parent control page: `9732097`
- Confluence space: `SD` / `163844`

## Authority ownership

| Surface | Authority |
|---|---|
| GitHub | source bytes, package bytes, prepared merge topology, commits, trees, PR reviews, evidence manifests |
| Jira PM | work status, bounded ownership, dependency and blocker state |
| Confluence SD | navigation, DAG, claim boundary, synchronized current pointers |
| Durable evidence bundle | raw/prototype evidence custody; not canonical integration |

## Required synchronized identities

Every transition after package preparation records:

```text
source parent SHA/tree
package parent SHA/tree
prepared implementation start SHA/tree
implementation final SHA/tree
draft implementation PR
evidence manifest digest
execution state and blockers
```

## Write order

1. Create a durable GitHub package, prepared-branch, implementation-commit, or PR receipt.
2. Update Jira with the exact identities, execution state, and blocker summary.
3. Update Confluence with the same identities and claim boundary.
4. Read both back.
5. Only then report cross-plane completion.

## Outage and drift

- Atlassian unavailable after source publication: `ATLAS_SYNC_PENDING`.
- Mismatched issue/page/source/package/prepared-branch/PR identity: `BLOCKED_BY_ATLASSIAN_AUTHORITY_DRIFT`.
- Never create a duplicate PM issue or Confluence control page.
- Never mark Jira Done before frozen replay, fresh review, final differential audit, and readback pass.
