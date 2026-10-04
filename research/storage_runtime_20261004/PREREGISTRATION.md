# STORAGE-RUNTIME — measured allocator bytes of the shared shift jet (prospective registration)

RVJ DAG node `STORAGE-RUNTIME` (depends on PP02). Base commit: the commit
that adds this file. No timing.

## Measurement

A counting global allocator (test binary, one thread) records live and
peak heap bytes. For `shared_shift_jet` on a sweep n in {4, 16, 64, 128},
r in {1, 4}, degree p in {0, 8, 24}, targets m in {1, 17, 65} (72
configurations, seeded dissipative J) it records: peak bytes above the
live bytes at entry (`P_total`); the same for `LuFactorization::new` of the
n x n center plus one `solve_rows` of r columns alone (`P_lu`); and
`8 * explicit_storage_upper_scalars_excluding_lu` (`S_explicit`).

## Gate

G1 Explicit accounting covers the non-LU part: `P_total - P_lu <=
   S_explicit + 64 * (vector headers allocated)` in every configuration,
   with the header count taken from the allocation count. A failure means
   the explicit slot formula misses a buffer and is a FAIL.
G2 The budget refusal allocates nothing: with `max_stored_scalars` or
   `max_work_units` one below the requirement, the call errors with zero
   allocations during the call.
G3 Reported, not gated: `P_lu / (8 n^2)` and `P_total / S_explicit`; the
   documentation states that the explicit cap is not an RSS cap.

## Results (append only after the recorded run)
