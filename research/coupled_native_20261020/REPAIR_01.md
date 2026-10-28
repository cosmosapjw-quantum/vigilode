# CI publication repair, before any benchmark ran

Run39348756394 at source109d73e failed before testing or numerical benchmarking. The pre-build formatting helper also edited a typo in the workflow. The Actions token is contents-write only; GitHub rejected both the source push and receipt push because their ancestry included a workflow edit without workflows permission. The failed job log118387643990 was read through the GitHub connector. No attempt is made to elevate that token's privileges.

Repair: the preparation helper changes only the new Rust companion, and the source commit stages only that crate. The workflow remains unchanged; source build, test, numerical tolerances and benchmark protocol remain the same. The original run is a publication-permission failure, not a failed numerical experiment. Its original Actions log remains the source for this finding; no missing raw solver output is invented.
