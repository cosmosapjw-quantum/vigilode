# K0 research-only public bridge contract

## Decision

WU-05 established an irreducible Rust visibility boundary: telemetry is produced in `rodas5p-krylov`, assembled in `rodas5p-integrators`, and serialized by the separate `rodas5p-cli` crate. Rust has no workspace-private or friend visibility. The selected repair is therefore a **narrow, documentation-hidden public bridge**. Cargo feature/new-crate isolation is rejected for this node because it would change the build graph and force an unnecessary campaign rebuild/rebind.

## Authorized modules

Exactly these bridge modules may be newly public:

```text
rodas5p_krylov::k0_research_bridge
rodas5p_integrators::k0_research_bridge
```

The modules and every directly exported item must use `#[doc(hidden)]`. New names must start with `K0` or contain `_k0_` / `_for_k0`.

## Allowed purpose

The bridge may carry only:

- initial and final unpreconditioned residual observations;
- actual initial-guess provenance;
- named linear and diagnostic apply counters;
- K0 stage receipt types and validators;
- K0 receipt execution and serialization helpers.

It is public only to satisfy Rust crate visibility. It is not a stable user API, production authority, or activation mechanism.

## Allowed call sites

```text
crates/rodas5p-integrators/src/k0_stage_telemetry.rs
crates/rodas5p-integrators/src/sequential.rs
crates/rodas5p-integrators/src/a1_two_arm_receipt.rs
crates/rodas5p-integrators/tests/k0_stage_telemetry_contracts.rs
crates/rodas5p-integrators/tests/a1_two_arm_receipt_contracts.rs
crates/rodas5p-cli/src/bin/a1_post_a2a3_kernel_cell.rs
```

No ordinary production CLI or integrator path may import or call either bridge.

## Hard prohibitions

- no `Cargo.toml`, `Cargo.lock`, feature, crate, or dependency change;
- no change to an existing production function signature;
- no dispatch, tolerance, convergence-authority, acceptance, output, recycle-transaction, or counter-semantic change;
- no undocumented public symbol;
- no re-export outside the two bridge modules;
- no call site outside the allowlist.

## Mechanical receipt

The repair must write:

```text
research/k0_stage_telemetry_20260827/review/public_bridge_surface.json
```

It records every bridge module, symbol, defining file, `#[doc(hidden)]` status, and call site. The repair validator rejects any non-hidden symbol, forbidden call site, manifest change, or production-path import.
