# FAILURE_LOG.md

## F-001 — Baseline build source path

The first offline Cargo invocation failed because the vendored-source relative path resolved to a missing repository-local symlink. Root cause was environment reconstruction, not source code. A repository-local symlink to the verified offline bundle restored the exact configured path. Subsequent full release tests passed.

## F-002 — Initial release compilation timeouts

Foreground container calls terminated during the first clean `faer` release build. A background build completed successfully; after caching, the exact baseline test command completed in under one second. Partial timed-out runs are not used as validation evidence.

## F-003 — Validation launcher quoting error

The first attempt to launch the composite background validation script had unmatched shell quoting and did not execute any validation command. It was replaced by a standalone script file. No partial result was used.

## F-004 — Polling wrapper timeouts

Two foreground polling calls timed out while the detached validation process continued. The underlying process completed with exit code zero; only its complete final log is used as evidence.
