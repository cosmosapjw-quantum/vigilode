# v2.7 Actual Level-1 Prefix Implementation Plan

1. TDD RED: add a contract test for a missing level-1 prefix API and resume-from-prefix exact parity.
2. Minimal GREEN: factor U2/D2 construction out of `pexprb54s4_fused_step_with_telemetry_config` into a retained prefix object plus serializable report.
3. Preserve the old one-shot API by implementing it as begin-prefix + resume.
4. Re-run existing pexprb/telemetry tests and Clippy before any campaign output.
5. TDD RED/GREEN: add an R-JF attempt runner that executes prefix-only work at causal first-next proposals for exact frozen k1 decisions and k3 development events.
6. Run family-sharded N=128/N=512 development campaigns only; full E continuation remains zero.
7. Analyze real prefix work, rho geometry, rejected-target behavior, group concentration, and compare actual U2/D2 prefix cost with the historical G4-S4 acquisition prefix without conflating them.
8. Dual audit + plot CRAG. N=2048 remains sealed until this node closes.
