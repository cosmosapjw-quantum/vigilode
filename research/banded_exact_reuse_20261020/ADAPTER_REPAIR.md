# Benchmark adapter repair; numerical core unchanged

Run39301232025 passed9public regression tests and17internal tests, including exact factor/pivot/solve bits and cache invalidation. The workspace command hit its660second ceiling (exit124), which remains NOT_PASS. These are separate outcomes.

All432attempted benchmark rows were process_failure before integration with `invalid numerical input: provide an explicit Jacobian or JVP`. The benchmark had supplied an external band provider but omitted the derivative capability required by OdeProblem::new. Therefore the prior SUMMARY's BLOCKED_NUMERICAL_PARITY is reclassified here as ADAPTER_CONSTRUCTION_FAILURE, not evidence of a changed numerical trajectory. No timings were admitted, and the failed rows remain immutable.

The repair supplies the analytical JVP for both model families. The banded integration path still uses the same band matrix callback, with unchanged equations, tolerance, initial/output conditions and numerical library. Both parent and candidate binaries use the same repaired model adapter. A preflight smoke now executes both model families and all four comparison arms before reference calculation or a full campaign; early adapter failures no longer fan out into432identical failed rows.

Production source remains bit-identical to b51c051b045e977c038e5eec468de0004840872b. Scoped tests are rerun; the unchanged workspace timeout receipt is preserved with a reuse binding instead of spending another660seconds on the same uncompleted whole-suite path. This does not convert timeout to PASS. This is one diagnosed adapter repair, not a tolerance relaxation or silent benchmark reselection.
