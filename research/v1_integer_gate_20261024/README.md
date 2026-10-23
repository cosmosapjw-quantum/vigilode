# V1q integer-only full33 physical-error admission

Start with [REPORT_KO.md](REPORT_KO.md), [THEORY.md](THEORY.md), [S2_RESULT.json](S2_RESULT.json), [DAG.json](DAG.json), and [HANDOFF_KO.md](HANDOFF_KO.md).

The standalone Rust kernel consumes binary64 encodings as u64, keeps exact signed row sums, and compares certified fixed-point squared-ratio bounds using integer operations only. Native hook and actual BASS client remain separate. This is not a general solver/global-error certificate and does not repair upstream scales.

New evidence:14unique tests;681exact cases343Accept/337Reject/1Indeterminate; prior6ambiguous resolved3/3; exactboundary admitted and boundary+tiny safely unresolved;16x681MXCSR identical certificate comparisons. Initial long-divider and optimization produce identical681outputs. Separate arbitrary-integer readback passed; same author, not independent personnel.

Exact quotient optimization is14–20x shorter than the first integer method in the diagnostic same-binary experiment, but integer remains4–5x slower than the older FP predicate. No default promotion. Single-process/shared-host timing and a noisy baseline cell are retained.

Full66payload source/raw/failure/budget archive: [Drive](https://drive.google.com/file/d/17HqZVwKQm-hF0RhcctoKUYzXN32GOpuy/view?usp=drivesdk); Dropbox ID/path and all stage backups in [FINAL_DELIVERY.json](FINAL_DELIVERY.json). Restore before running full oracle/review scripts; Git intentionally contains selected reviewable source, not every raw record. See [REPRODUCE.md](REPRODUCE.md).

Sourcepublication c9712a50e87dbaeec95da01608ef5ab22b74dc45; executed core blob3cba0e3c8bff1ce104379e69a1233807ccc0af99. Parent/source/otherowners and NCP preserved. Draft only, no merge.
