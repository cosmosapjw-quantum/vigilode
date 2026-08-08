# FAILURE_LOG.md

- F-001: `init_harness.py --project` failed because the tool accepts only `--force`; reran with documented CLI. No source change.
- F-002: adding BDF/Radau execution variants made old RODAS-stage matches non-exhaustive. Root cause was a missing mathematical-domain boundary; fixed by explicit stage/native classification and regression tests.
- F-003: a foreground full integrator test command exceeded the command wrapper timeout. The exact full release command was rerun in the background and exited 0; the partial foreground run was not used as evidence.
- F-004: an attempted test command named nonexistent targets `unified_gates_contracts`/`unified_screen_contracts`; corrected to the repository's actual `unified_candidate_contracts` target. No source change.
- F-005: `cargo run` initially omitted `--bin` in a package with two binaries; reran with `--bin rodas5p`. No source change.
