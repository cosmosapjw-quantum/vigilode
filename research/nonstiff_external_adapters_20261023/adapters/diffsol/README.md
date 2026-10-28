# Diffsol standalone adapter

This isolated workspace pins Diffsol 0.16.1 with faer. Its own Cargo.lock is independent of VigilODE's frozen dependency graph. Rust 1.94.1; measurement profile opt-level 3, LTO off, 16 codegen units. No native solver code is linked.

One JSON request on stdin returns one benchmark record on stdout. The immutable mathematical problem source is 45037fc24e8f8101605f309e1fdb3a9a4e3df56d. Build with `VIGILODE_ADAPTER_SOURCE_COMMIT` set to the committed adapter source identity; development builds disclose UNCOMMITTED_DEVELOPMENT. The controller records lock, source and executable hashes independently.

Tsit45 serves all nine nonstiff families, including 4096/16384 vectors. It receives RHS only; stage storage is O(dimension times stages), with no dense Jacobian. BDF serves only the two dimension-12 stiff controls, with analytic JVP and faer dense LU. Methods never switch automatically. Scalar rtol/atol and initial step 0.001 are passed directly to Diffsol.

A single adaptive history uses Diffsol's built-in interpolation at five fixed requested times and a final stop-time. `max_steps` caps public step calls globally; internal error-test rejections appear in Diffsol statistics and actual RHS callback counts. Failure records preserve collected target observations and the current accepted state/time, with no reference substitution. Unknown/malformed requests exit 2 with stderr and no benchmark record.

The timer includes fresh initial data, problem/workspace construction, solve callbacks, interpolation and saved-state collection. References, error replay, JSON encoding and process launch are excluded. Timing authority is diagnostic_only. Exact linear solve and heap-allocation counts are unavailable and explicitly null; observed setup/Newton/rejection statistics retain their original Diffsol meanings. These counts do not assert a native WorkCounters equivalence or zero-allocation certificate.

`test_protocol.py --binary PATH --analyzer FROZEN_ANALYZE_BENCH --out NEW_FOLDER` checks all families, the 16384 vector, frozen independent analytic replay, actual initial states, method identity, counters, malformed requests and a truthful step-limit failure. Rust unit tests compare all-family RHS with independently differenced analytic trajectories.
