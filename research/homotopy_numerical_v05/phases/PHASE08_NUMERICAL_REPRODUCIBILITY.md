# Phase 8 — Numerical and reproducibility validation

- canonical one-step screen: 57 cases, 8,208 candidates, 114 controls;
- exact repeated canonical run after the output-budget schema clarification: byte-identical;
- canonical SHA-256: `a665562e3088950e3a7d64e985f220708cac6cad7356f096baf378e4ee27026e`;
- non-finite/failure rows: zero;
- fixed-step global order screen uses h = 0.04, 0.02, 0.01, 0.005 to T=0.2;
- stiffness grid reaches 1e6 and nonnormality grid reaches 0.9;
- exact target certificate and sequential oracle use the same Rust/faer backend.

A six-level order-screen attempt was terminated by the execution timeout because the q=7 exact
certificate path scales poorly with the number of timesteps. Its partial output was not used. The
four-level screen was rerun completely and deterministically.

No wall-time performance claim is made: the current path uses an exact dense certificate and the
batch interface does not prove concurrent execution.
