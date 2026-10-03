# Native follow-up audit scope

Date: 2026-10-03 (Asia/Seoul)
Requested by the repository owner: inspect current changes and PRs, fix reproducible basic issues, validate with Rust 1.94.1, continue incremental RVJ-related research, and publish a separate PR. The review and future DAG will contain only remaining, unperformed work; completed fixes and actual validation belong in a separate execution record.

Baseline commit: `5a8d7fe9ffc681bca98a98a2f9889a2a05505783`.
Base branch: `claude/jolly-wozniak-7wl15h-wu25-stiff-benchmark`.
Review branch: `audit/rvj-native-followup-20261003`.

The prior thread-transfer DAG has already been executed in L-0034 through L-0043. Do not propose its completed nodes as new work. L-0038 remains FAIL and timing authority remains HOLD. Existing ledger entries, holdouts, scientific verdicts and coefficient identities are preserved.

## Planned bounded audit

Inspect the changed raw matrix-free driver, signed Laguerre adjoint/cache, chart contracts, homotopy radius/action code and GCRO-DR small-pencil repair. Trace actual call sites and distinguish intended domain restrictions from defects. For every code fix, add a failing regression first, preserve its failure output, apply a minimal correction, and rerun relevant native tests. No numerical probe or timing campaign is admitted by this scope file.

Investigate the next genuinely open RVJ transfer contracts: physical error transport in regular ratio charts, immutable proof/cache data binding, strict-lower radius action, and inexact/current-operator semantics. Preserve the known RVJ counterexamples; do not replace RODAS5P with generic RVJ5.

## Reproduction environment

The user supplied a Rust 1.94.1 standalone distribution and a separate project's vendor bundle. The local container has no DNS route to GitHub or crates.io. A read-only, branch-scoped CI export may create a pinned source bundle plus Cargo.lock-compatible vendor artifact for local native reproduction. This changes no dependency versions, does not export credentials, and grants the job only contents:read. Native tests, archive identity, signatures and CI status must each be reported only after observation.

## Publication

Keep product edits confined to verified issues and narrow opt-in research interfaces. Commit and push on the review branch without force, open a PR against the active base, and preserve concurrent upstream changes. Record content SHA/tree, exact commands, failures, unexecuted tests and the remaining-only development DAG. Do not merge the PR or relax timing/holdout authority.
