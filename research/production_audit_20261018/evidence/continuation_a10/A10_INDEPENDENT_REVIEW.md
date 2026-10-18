# A10 independent review

## Review scope

Read-only review of the Bernstein interval transform, dyadic de Casteljau subdivision, caller gate, repair receipt, and retained focused logs. Source identity reviewed:

- `crates/rodas5p-integrators/src/packet_window.rs`: `cca2b7c008f062e38d784de3127789d05dba0d07e322349a81ac7744a9004084`
- `crates/rodas5p-integrators/src/packet_work.rs`: `846fafd24f163a0c1ab7f89a094a9552f2c92c2a0ff93beef3703c8ed1efc9ad`
- `crates/rodas5p-integrators/tests/production_audit_packet.rs`: `ac0b7244e752dcb5ee6c60646f5e14b3e6cb88f3794227ed9fa226e2521e74b2`
- Repair receipt: `REVIEW_REPAIR_RECEIPT.json`, SHA-256 `f70996a6171845abc9251726ca3f9b6dc8519b70a46f72d808c3d2be052c8f8a`

## Findings and decision

The two prior MAJOR findings are closed:

1. Public real and imaginary interval endpoints are checked for finite ordered bounds before the cheap norm gate. NaN, infinity, and reversed intervals refuse with a typed error.
2. Work proxies are charged before fallible cheap-bound, basis-transform, control-norm, and subdivision arithmetic. The overflowing `2^1023` subdivision regression confirms attempted refinement work is retained.

The outward Bernstein transform and dyadic subdivision give a scoped upper bound for represented interval polynomials. The helper is called only from diagnostic `compute_certificate` after the cheap coefficient bound fails. It is not used by the A01 outward residual/output path. The tail model remains unchanged and packet `roundoff_enclosed` remains false.

Receipt-bound evidence reports the focused tests **8/8**, packet target **37/37**, Clippy with `-D warnings`, and formatting checks passed. The earlier failures are preserved. No new blocker was found. The review did not rerun commands.

Decision: **SCOPED PASS for the interval-polynomial helper; A10 full node and production authority remain HOLD** because the required A01 candidate/residual interval path is not connected. This review does not establish zero allocation or complete total cost.
