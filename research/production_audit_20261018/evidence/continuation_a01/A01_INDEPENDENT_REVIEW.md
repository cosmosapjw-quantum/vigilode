# A01 independent review

## Reviewed source identity

- `crates/rodas5p-integrators/src/packet_window.rs`: `0e527d45aa4676f95f7dc781d4805df837b3c3072fff016602bd36b0c8f22b90`
- `crates/rodas5p-integrators/tests/production_audit_packet.rs`: `91bd2bd77593bb46b5ff752a65693d06ba7aaea605cd6b042e3b986d784a6286`

## Scoped outcome

Independent review accepted the generated-packet outward coefficient, Volterra residual, tail/reconstruction and returned-center enclosure repair under the directed-primitive, `q < 1`, and geometric-tail premises. It found no new mathematical counterexample in that local scope. The exact one-mode cubic regression, multi-mode high-precision diagnostic reference, intermediate and endpoint outputs, true partial `[0.005, NaN]` request, and actual post-precharge refusal are retained.

The checked-size follow-up was also reviewed: `compile_packet(order=usize::MAX)` and `generate_candidate(degree=usize::MAX)` reject with typed `PacketError::SizeOverflow` before the overflowed allocation/resize path. This does not certify arbitrary allocation failures or all size-taking APIs.

Focused evidence: packet contract target 29/29, clippy exit 0, owned-file rustfmt check exit 0, and counter receipt `output_restriction_multiplies=54`, all bound to the source hashes above. Whole-workspace formatting once failed only on a concurrently edited A02 file; that failure is retained, and A02 has a separate passing format log.

Limits: no allocator-instrumented complete cost proof; high-precision multi-mode reference is an independent diagnostic, not a rigorous reference enclosure; `PacketCertificate.roundoff_enclosed` and `OutwardPacketOutput.roundoff_enclosed` remain false. The accepted result is a bounded local enclosure scope, not whole A01 pipeline completion, A02 trajectory admission, or production promotion.

Decision: `PASS_WITH_SCOPE` for reviewed local outward reconstruction; A01 whole-node authority `HOLD` pending broader arithmetic/cost/refusal obligations, and global trajectory remains separately held.
