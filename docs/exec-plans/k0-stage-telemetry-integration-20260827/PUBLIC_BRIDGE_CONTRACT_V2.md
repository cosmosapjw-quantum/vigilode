# K0 research-only public bridge contract v2 — WU-05 controlling supplement

## Decision

Rust has no workspace-private or friend visibility. Telemetry originates in `rodas5p-krylov`, is assembled in `rodas5p-integrators`, and is serialized by the separate CLI crate. The only authorized repair is a **narrow public-at-the-language-level, documentation-hidden K0 bridge**.

Cargo feature or new-crate isolation is rejected for this node because it changes the build graph and would require an unnecessary campaign rebuild and evidence rebind.

## Authorized modules and files

```text
rodas5p_krylov::k0_research_bridge
  crates/rodas5p-krylov/src/k0_research_bridge.rs

rodas5p_integrators::k0_research_bridge
  crates/rodas5p-integrators/src/k0_research_bridge.rs
```

Each declaration in the corresponding `lib.rs` must be:

```rust
#[doc(hidden)]
pub mod k0_research_bridge;
```

Every directly exported function, type, constant, static, trait, or module in either bridge must also be `#[doc(hidden)]`. New names start with `K0` or contain `_k0_` / `_for_k0`. `pub use` inside or outside the bridges is forbidden.

## Allowed payload

The bridge may carry only:

- initial and final unpreconditioned residual observations;
- actual initial-guess provenance;
- named linear and diagnostic operator-apply counters;
- K0 stage/cell receipt types and validators;
- K0 receipt execution and serialization helpers.

It is not a stable API, a production authority, or an activation mechanism.

## Allowed call sites

```text
crates/rodas5p-integrators/src/k0_stage_telemetry.rs
crates/rodas5p-integrators/src/sequential.rs
crates/rodas5p-integrators/src/a1_two_arm_receipt.rs
crates/rodas5p-integrators/tests/k0_stage_telemetry_contracts.rs
crates/rodas5p-integrators/tests/a1_two_arm_receipt_contracts.rs
crates/rodas5p-cli/src/bin/a1_post_a2a3_kernel_cell.rs
```

No ordinary production CLI or non-K0 integrator path may import or call either bridge.

## Mechanical source audit

`public_bridge_surface.json` is not authority by itself. The validator must:

1. parse the two `lib.rs` module declarations;
2. derive exported bridge symbols from the two bridge source files;
3. verify `#[doc(hidden)]` and K0 naming on every export;
4. reject any `pub use`;
5. scan repository call sites and reject paths outside the allowlist;
6. compare the source-derived inventory exactly with `public_bridge_surface.json`;
7. verify all Cargo manifests and `Cargo.lock` are unchanged from the source parent.

Only then may it emit:

```text
PUBLIC_BRIDGE_SOURCE_PASS
```
