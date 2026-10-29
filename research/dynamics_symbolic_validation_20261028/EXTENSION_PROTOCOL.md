# Prospective cumulative integration and saved-data audit

The first registered run finished all eight commands successfully at source
58ec9b053163da6b6f0906010e0422c104db72fd. Its coupled/facade commands selected
library tests only (39 and31), so they do not cover the packages' integration
contracts. Before claiming cumulative compatibility, execute all integration
test targets for both affected packages. This extends coverage without
repeating their completed library/benchmark runs. Keep the same Rust1.94.1,
locked/offline graph, measurement profile, target and shared native lock.

Publish this protocol and analysis script before these commands. Record the
actual new HEAD; executable solver source must be byte-identical to the
first run. Use the existing record_command.py, new output directories and a
900s per-command resource cap:

    cargo test --profile measurement --locked --offline -p vigilode-coupled -p vigilode --test '*'

The quoted star is one literal argv argument to Cargo, not shell expansion.
Failing assertions remain failures, including obsolete telemetry expectations;
diagnose before modifying either implementation or tests. No full-workspace,
optional-feature or real-client qualification is inferred.

Run analyze.py over the saved first-run JSON once. It independently constructs
the original quartic multiplication matrix with Fraction arithmetic, checks
every saved query and returned interval, and summarizes preplanned paired
timings. It does not import native moment formulas. Generate plots from all
seven paired values, including regressions and no-copy controls. This is
saved-data scientific analysis, not a second native benchmark. Preserve
script/input hashes, actual exit and numeric outputs.

After results publication, run the unchanged research-node and authority
checkers in a full materialized checkout of the actual delivery tree. Existing
global authority failures remain open; do not weaken the checker/allowlist.
