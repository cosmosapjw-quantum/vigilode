# A02-TXN-01 implementation execution preregistration

This implementation consumes the independently READY frozen V2 contract,
DC1 accessor, and unchanged equations D1–D5. No new physical tolerance, oracle
order, work cap or packet/global production authority is introduced.

Execution inputs: supplement_02/FROZEN_ACCEPTANCE_INPUTS_V2.json,
AMENDMENT_KO.md, INDEPENDENT_DELTA_REVIEW.json, current packet_trajectory.rs,
production_audit_verified_trajectory.rs, this directory's fraction_oracle.py,
and the unchanged A01 exact-reference functions imported by the oracle.
The controller must bind the posted implementation EXEC_SHA and these hashes
before running. This file contains no numerical outcomes.

Implementation uses private immutable Arc<WindowStartSeed>, private prepared
owner/token, caller-owned query/advance workspace and fixed output/retained
capacity. Successful commit moves its owner and shares the immutable seed with
no allocation. A consumed token retains its old seed so replay is explicitly
stale. Historical query reads that seed, not the live accepted state. Both
accepted weighted error and marginal radii are retained separately. Global
energy is frozen from the initial tube. Physical span and polynomial H remain
distinct, with outward offset and temporal lift for every requested label.

The non-packet adapter counter tally is frozen in SOURCE_COUNTER_TALLY.json.
PacketWorkCounters remains separate and preserves all delegated kernel work.
Preallocation/setup has allocations; only warmed successful query and staged
advance have a zero-allocation gate, after a same-allocator CollectiveModel
clone positive control detects its three Vecs. Failed calibration is
ALLOCATION_UNAVAILABLE. Single-process timing remains DIAGNOSTIC_ONLY.

Commands (must preserve raw stdout/stderr, first failure, source/config identity,
wall time and exit code; timeout limits are those in frozen V2):

```sh
VIGILODE_A02_OUTPUT=<native.json> VIGILODE_A02_ALLOCATION_OUTPUT=<alloc.json> VIGILODE_A02_INVALID_OUTPUT=<invalid.json> cargo test --locked --offline -p rodas5p-integrators --test production_audit_verified_trajectory -- --nocapture
cargo test --locked --offline -p rodas5p-integrators --lib verified_transaction_private_tests -- --nocapture
python3 research/production_audit_20261018/evidence/continuation_a02_research/txn_01/fraction_oracle.py <native.json> <oracle.json>
cargo test --locked --offline -p rodas5p-integrators --doc packet_trajectory
```

Before admission independently review the actual diff and all raw outcomes.
No blind rerun or numerical threshold change is authorized. Implementation
errors require preserved first failure and bounded repair approval.
