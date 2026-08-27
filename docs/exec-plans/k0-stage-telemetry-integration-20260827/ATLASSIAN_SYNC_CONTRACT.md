# GitHub–Atlassian Synchronization Contract

## Frozen identifiers

- GitHub repository: `cosmosapjw-quantum/vigilode`
- Control-package branch: `docs/k0-codex-execution-package-20260827`
- Control-package PR: `#21`
- Implementation base PR: `#20` at `e1124586a4029f86669e7489278c61ef676d61aa`
- Jira owner: `PM-7`
- Confluence page: `15499267`
- Parent control page: `9732097`
- Confluence space: `SD` / `163844`

## Authority ownership

| Surface | Authority |
|---|---|
| GitHub | source bytes, contracts, commits, trees, PR reviews, evidence manifests |
| Jira PM | work status, bounded ownership, dependency and blocker state |
| Confluence SD | navigation, DAG, claim boundary, synchronized current pointers |
| Durable evidence bundle | raw/prototype evidence custody; not canonical integration |

## Write order

1. Create a durable GitHub commit/tree or PR receipt.
2. Update Jira with branch, commit, tree, PR, execution state, and blocker summary.
3. Update Confluence with the same identifiers and claim boundary.
4. Read both back.
5. Only then report cross-plane completion.

## Outage and drift

- Atlassian unavailable after source publication: `ATLAS_SYNC_PENDING`.
- Mismatched issue/page/branch/PR identity: `BLOCKED_BY_ATLASSIAN_AUTHORITY_DRIFT`.
- Never create a duplicate PM issue or Confluence control page.
- Never mark Jira Done before fresh review, final differential audit, and readback pass.
