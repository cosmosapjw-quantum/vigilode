# Executed evidence

`final_affected.log` is the actual restored-source final Cargo output:40 tests across5 suites. `python_tests.log` is the actual8-test Python output. `mutation_excerpts.md` quotes the three actual failing mutation runs and binds the full logs by SHA-256; full logs, all54-row diagnostics, refinement data and plots are in the downloadable research bundle. The full data are reproducible with the directly tracked `tools/audit2_output_policy_research.py` and original repository evidence, without any external bundle prerequisite.

The `source_sha256` values in verification.json identify the locally tested source bytes. The published tree uses those seven Git source objects. These are source-provenance checks, not equality criteria for numerical backends. JSON report formatting and archive compression need not be bit-identical.

No full-workspace completion or independent fresh-context source review is claimed. No old K0 bootstrap is required.
