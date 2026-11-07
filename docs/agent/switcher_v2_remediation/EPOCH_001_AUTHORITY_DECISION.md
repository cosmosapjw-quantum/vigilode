# Authority Decision Record: Window Epoch Identity Specification (WP-02 Supersedes F7)

- **Decision ID**: `EPOCH-001-DECISION-20261106`
- **Date**: 2026-11-06
- **Status**: `APPROVED_BY_OWNER`
- **Scope**: VigilODE SwitchingSolver Window Epoch Identification (`crates/vigilode/src/registry.rs`)
- **Historical Reference**: `F7` (FNV-1a window epoch, checkpoint-index independent) — **SUPERSEDED**
- **Active Authoritative Standard**: `WP-02` (Fixed-layout SHA-256 window epoch, checkpoint-index sensitive)

---

## 1. Rationale & Problem Statement

In the historical switching architecture (Audit `F7`), the window epoch was computed using an FNV-1a hash over only the start and target time endpoints, omitting the checkpoint index:
$$\text{Epoch}_{\text{F7}} = \text{FNV-1a}(t_{\text{start}}, t_{\text{target}})$$

This formulation exhibited two critical defects:
1. **Checkpoint Collision**: When a solver re-enters an identical time interval $[t_{\text{start}}, t_{\text{target}}]$ across different checkpoints (e.g., during sub-stepping, restart, or multi-interval continuation), identical epoch digests were produced, causing cross-checkpoint cache pollution and invalid continuation claims.
2. **Cryptographic & Bit Stability**: FNV-1a has poor diffusion on floating-point bit patterns and lacks standard outward bit guarantees.

To establish deterministic, collision-resistant namespace isolation across continuation checkpoints, the **WP-02 specification** was derived and adopted. This document serves as the authoritative, immutable specification for WP-02.

---

## 2. Mathematical Characterization & Trust Boundary

1. **Identity Namespace Discriminator**:
   - A window epoch is an **identity discriminator** for cache tagging, session state continuation, and route history isolation.
   - It is **NOT** a numerical error bound, Lyapunov certificate, contraction factor, or trajectory enclosure.
2. **Deterministic Injectivity Requirement**:
   - Two window intervals differing in checkpoint index, start time bit pattern, or target time bit pattern MUST produce distinct epoch digests with cryptographic probability ($\le 2^{-64}$).

---

## 3. Exact 24-Byte Serialization Layout

The window epoch digest is computed over an exact, packed 24-byte little-endian byte array:

$$\text{Payload} = \text{Bytes}[0..24]$$

| Byte Range | Field | Data Type | Encoding / Byte Order | Bit Pattern |
| :--- | :--- | :--- | :--- | :--- |
| `0..8` (8 bytes) | `checkpoint_index` | `usize as u64` | Little-endian (`to_le_bytes()`) | On 32-bit platforms: zero-extended to `u64`. On 64-bit: exact cast. |
| `8..16` (8 bytes) | `start` | `f64` | `start.to_bits().to_le_bytes()` | IEEE-754 binary64 bit pattern in little-endian |
| `16..24` (8 bytes) | `target` | `f64` | `target.to_bits().to_le_bytes()` | IEEE-754 binary64 bit pattern in little-endian |

### Digest Derivation
1. Compute the 256-bit SHA-256 hash of the 24-byte payload:
   $$\text{Digest}_{256} = \text{SHA-256}(\text{Payload}) \in \mathbb{B}^{32}$$
2. Take the first 8 bytes (`Digest[0..8]`) and parse as a little-endian unsigned 64-bit integer:
   $$\text{Epoch} = \sum_{k=0}^{7} \text{Digest}[k] \cdot 2^{8k}$$

In Rust:
```rust
pub fn compute_window_epoch(checkpoint_index: usize, start: f64, target: f64) -> u64 {
    let mut bytes = [0u8; 24];
    bytes[0..8].copy_from_slice(&(checkpoint_index as u64).to_le_bytes());
    bytes[8..16].copy_from_slice(&start.to_bits().to_le_bytes());
    bytes[16..24].copy_from_slice(&target.to_bits().to_le_bytes());
    let digest = rodas5p_core::sha256_bytes(&bytes);
    u64::from_le_bytes(digest[0..8].try_into().expect("slice has 8 bytes"))
}
```

---

## 4. IEEE-754 Special Value & Boundary Semantics

1. **Signed Zeros (`-0.0` vs `+0.0`)**:
   - In IEEE-754, `+0.0` has bit pattern `0x0000_0000_0000_0000` while `-0.0` has `0x8000_0000_0000_0000`.
   - Because `to_bits()` preserves the sign bit, `compute_window_epoch(0, 0.0, 1.0)` and `compute_window_epoch(0, -0.0, 1.0)` are strictly non-equal.
2. **Subnormals & Non-finites**:
   - Subnormal numbers, infinities ($\pm\infty$), and quiet/signaling NaNs preserve their bit representations into the payload.
3. **Index Domain**:
   - Indices above $2^{32} - 1$ on 64-bit architectures are fully preserved in the 64-bit payload slice without truncation.

---

## 5. Persistence, Invalidation & Migration Policy

1. **Cold Invalidation**:
   - Persisted session state, cached spectral projections, or route traces created under the historical F7 FNV-1a standard are incompatible with WP-02.
   - Any persisted state bearing an unverified or FNV-1a epoch MUST be treated as invalidated upon intake (`CACHE_INVALIDATED`).
2. **Recomputation**:
   - In-memory solver registries and checkpoint controllers MUST recompute the window epoch dynamically using `compute_window_epoch`.
3. **No Dual Dispatch**:
   - No fallback or backward-compatibility mode to FNV-1a is maintained in production code paths.

---

## 6. Golden Test Vectors

The authoritative test vectors for WP-02 are:

| `checkpoint_index` | `start` | `target` | Expected 64-bit Epoch Digest |
| :---: | :---: | :---: | :---: |
| `0` | `0.0` | `1.0` | `0x5718_834c_1304_ae04` |
| `3` | `0.25` | `0.5` | `0x27a1_3285_42cc_99d0` |
| `17` | `-1.5` | `2e-300` | `0x8b1b_1407_ca5e_33aa` |
| `7` | `0.0` | `1.0` | `0xe9b6_ee9b_f31a_9fa7` |

