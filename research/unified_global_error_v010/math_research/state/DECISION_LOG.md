# Decision log

| Decision | Disposition | Reason |
|---|---|---|
| external max common-grid L2 as P0 primary error | PROMOTE | exact, method independent, detects interior error |
| endpoint and RMS metrics | PROMOTE DIAGNOSTIC | reveal metric dependence |
| scalar weighted work score | REJECT | hardware/problem dependent weights |
| no interpolation in P0 | PROMOTE | preserves high-order method meaning |
| T4 per-candidate latency ranking | REJECT | contention confounds latency |
| fixed anchors as production ranking | REJECT | authority validation only |
| authority implementation | PROMOTE | exact references, zero failures, deterministic scientific checksum |
| current fixed-anchor wall ranking | HOLD | small dense corpus and internal-step storage |
| suite/campaign wall clock split | PROMOTE | removes spurious cross-thread speedup |
| common-output observer before adaptive candidates | REQUIRED | no-interpolation authority otherwise cannot consume arbitrary adaptive steps |
